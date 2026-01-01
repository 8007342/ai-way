//! Conductor Daemon
//!
//! Standalone server process for the Conductor orchestration engine.
//! Surfaces (TUI, WebUI, etc.) connect to this daemon via Unix socket.
//!
//! # Usage
//!
//! ```bash
//! # Start with default socket path ($XDG_RUNTIME_DIR/ai-way/conductor.sock)
//! conductor-daemon
//!
//! # Start with custom socket path
//! CONDUCTOR_SOCKET=/tmp/my-conductor.sock conductor-daemon
//!
//! # With verbose logging
//! RUST_LOG=debug conductor-daemon
//! ```
//!
//! # Environment Variables
//!
//! - `CONDUCTOR_SOCKET`: Custom Unix socket path
//! - `YOLLAYAH_MODEL`: Ollama model name (default: auto-detected)
//! - `OLLAMA_HOST`: Ollama server host (default: localhost)
//! - `OLLAMA_PORT`: Ollama server port (default: 11434)
//! - `RUST_LOG`: Log level (trace, debug, info, warn, error)
//!
//! # Files
//!
//! - Socket: `$XDG_RUNTIME_DIR/ai-way/conductor.sock` (or `/tmp/ai-way-$UID/conductor.sock`)
//! - PID file: `$XDG_RUNTIME_DIR/ai-way/conductor.pid` (or `/tmp/ai-way-$UID/conductor.pid`)
//!
//! # Signals
//!
//! - SIGTERM/SIGINT: Graceful shutdown (removes PID file and socket)

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::signal;
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info, warn};

use conductor_core::{
    transport::{ConductorTransport, TransportConfig, TransportType, UnixSocketServer},
    Conductor, ConductorConfig, ConductorMessage, OllamaBackend, SurfaceEvent,
};

/// Get the default PID file path
///
/// Uses XDG_RUNTIME_DIR if available, otherwise /tmp/ai-way-$UID/
fn default_pid_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir)
            .join("ai-way")
            .join("conductor.pid")
    } else {
        let uid = unsafe { libc::getuid() };
        PathBuf::from(format!("/tmp/ai-way-{}/conductor.pid", uid))
    }
}

/// Write the PID file
fn write_pid_file(path: &PathBuf) -> anyhow::Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let pid = std::process::id();
    let mut file = fs::File::create(path)?;
    writeln!(file, "{}", pid)?;

    info!(pid = pid, path = ?path, "PID file created");
    Ok(())
}

/// Remove the PID file
fn remove_pid_file(path: &PathBuf) {
    if path.exists() {
        if let Err(e) = fs::remove_file(path) {
            warn!(error = %e, path = ?path, "Failed to remove PID file");
        } else {
            info!(path = ?path, "PID file removed");
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("conductor_daemon=info".parse()?)
                .add_directive("conductor_core=info".parse()?),
        )
        .with_target(true)
        .init();

    info!("Starting Conductor Daemon");
    info!("PID: {}", std::process::id());

    // Get PID file path and write it
    let pid_path = default_pid_path();
    if let Err(e) = write_pid_file(&pid_path) {
        error!(error = %e, "Failed to write PID file");
        return Err(anyhow::anyhow!(
            "Failed to write PID file at {:?}: {}. Check directory permissions.",
            pid_path,
            e
        ));
    }

    // Load transport configuration from environment
    let transport_config = TransportConfig::from_env();

    // Get socket path
    let socket_path = match &transport_config.transport {
        TransportType::UnixSocket { path } => path
            .clone()
            .unwrap_or_else(conductor_core::transport::unix_socket::default_socket_path),
        _ => {
            // Default to Unix socket for daemon mode
            conductor_core::transport::unix_socket::default_socket_path()
        }
    };

    info!(path = ?socket_path, "Socket path");

    // Ensure socket directory exists
    if let Some(parent) = socket_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| {
                error!(error = %e, path = ?parent, "Failed to create socket directory");
                remove_pid_file(&pid_path);
                anyhow::anyhow!(
                    "Failed to create socket directory {:?}: {}. Check permissions.",
                    parent,
                    e
                )
            })?;
            info!(path = ?parent, "Created socket directory");
        }
    }

    // Remove stale socket if it exists
    if socket_path.exists() {
        warn!(path = ?socket_path, "Removing stale socket file");
        fs::remove_file(&socket_path).map_err(|e| {
            error!(error = %e, "Failed to remove stale socket");
            remove_pid_file(&pid_path);
            anyhow::anyhow!(
                "Failed to remove stale socket at {:?}: {}. Another daemon may be running.",
                socket_path,
                e
            )
        })?;
    }

    // Create Unix socket server
    let mut server = UnixSocketServer::new(socket_path.clone());

    // Start listening
    server.listen().await.map_err(|e| {
        error!(error = %e, "Failed to start server");
        remove_pid_file(&pid_path);
        anyhow::anyhow!(
            "Failed to listen on {:?}: {}. Check if another daemon is running or if you have permission to create sockets.",
            socket_path,
            e
        )
    })?;

    info!("Listening for connections");

    // Create channel for Conductor -> Surface messages
    // (We'll broadcast these to all connected surfaces)
    let (msg_tx, mut msg_rx) = mpsc::channel::<ConductorMessage>(100);

    // Create channel for Surface -> Conductor events (aggregated from all surfaces)
    let (event_tx, mut event_rx) = mpsc::channel::<SurfaceEvent>(100);

    // Create Conductor
    let backend = OllamaBackend::from_env();
    let config = ConductorConfig::from_env();
    let mut conductor = Conductor::new(backend, config, msg_tx.clone());

    // Start Conductor
    conductor.start().await?;
    info!("Conductor started and ready");

    // Wrap conductor in Arc<Mutex> for shared access
    let conductor = Arc::new(Mutex::new(conductor));

    // Wrap server in Arc for shared access between tasks
    let server = Arc::new(Mutex::new(server));

    // Spawn task to broadcast messages to all surfaces
    let server_for_broadcast = Arc::clone(&server);
    tokio::spawn(async move {
        while let Some(msg) = msg_rx.recv().await {
            let server = server_for_broadcast.lock().await;
            if let Err(e) = server.broadcast(msg).await {
                warn!(error = %e, "Broadcast failed");
            }
        }
    });

    // Spawn task to forward events to conductor
    let conductor_for_events = Arc::clone(&conductor);
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let mut conductor = conductor_for_events.lock().await;
            if let Err(e) = conductor.handle_event(event).await {
                warn!(error = %e, "Failed to handle event");
            }
        }
    });

    // Spawn task for streaming token polling
    let conductor_for_streaming = Arc::clone(&conductor);
    tokio::spawn(async move {
        loop {
            {
                let mut conductor = conductor_for_streaming.lock().await;
                conductor.poll_streaming().await;
            }
            // Small delay to avoid busy-looping
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    });

    // Spawn a task to handle shutdown signals
    let shutdown_notify = Arc::new(tokio::sync::Notify::new());
    let shutdown_clone = Arc::clone(&shutdown_notify);
    tokio::spawn(async move {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                info!("Received Ctrl+C, shutting down");
            }
            _ = terminate => {
                info!("Received SIGTERM, shutting down");
            }
        }

        shutdown_clone.notify_one();
    });

    info!("Ready to accept connections");

    // Main accept loop
    loop {
        let accept_future = async {
            let mut server = server.lock().await;
            server.accept().await
        };

        tokio::select! {
            _ = shutdown_notify.notified() => {
                info!("Shutting down server");
                break;
            }
            accept_result = accept_future => {
                match accept_result {
                    Ok((conn_id, mut surface_event_rx)) => {
                        info!(conn_id = %conn_id, "Surface connected");

                        // Spawn a task to forward events from this surface
                        let event_tx = event_tx.clone();
                        let conn_id_clone = conn_id.clone();
                        tokio::spawn(async move {
                            while let Some(event) = surface_event_rx.recv().await {
                                info!(conn_id = %conn_id_clone, event = ?event, "Received event");
                                if event_tx.send(event).await.is_err() {
                                    warn!(conn_id = %conn_id_clone, "Event channel closed");
                                    break;
                                }
                            }
                            info!(conn_id = %conn_id_clone, "Surface disconnected");
                        });
                    }
                    Err(e) => {
                        error!(error = %e, "Accept failed");
                    }
                }
            }
        }
    }

    // Graceful shutdown
    info!("Performing graceful shutdown...");

    {
        let mut server = server.lock().await;
        server.shutdown().await.map_err(|e| {
            error!(error = %e, "Shutdown error");
            // Still try to clean up PID file
            remove_pid_file(&pid_path);
            anyhow::anyhow!("Shutdown failed: {}", e)
        })?;
    }

    // Clean up PID file
    remove_pid_file(&pid_path);

    // Clean up socket file (server.shutdown() should do this, but just in case)
    if socket_path.exists() {
        if let Err(e) = fs::remove_file(&socket_path) {
            warn!(error = %e, "Failed to remove socket file");
        }
    }

    info!("Conductor daemon stopped cleanly");
    Ok(())
}
