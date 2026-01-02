//! Daemon Server Implementation
//!
//! This module provides the core server loop for the Conductor daemon:
//! - Accepts connections on a Unix socket
//! - Spawns handler tasks per connection
//! - Tracks active connections via SurfaceRegistry
//! - Supports graceful shutdown
//! - Handles config reload signals
//!
//! # Multi-Surface Architecture
//!
//! The daemon supports multiple simultaneous surface connections:
//!
//! ```text
//!                     DaemonServer
//!                          │
//!          ┌───────────────┼───────────────┐
//!          │               │               │
//!     TUI Client      Web Client     Mobile Client
//!     (conn-1)        (conn-2)        (conn-3)
//!          │               │               │
//!          └───────────────┴───────────────┘
//!                          │
//!                    Conductor
//!               (with SurfaceRegistry)
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use dashmap::DashMap;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, warn, Instrument};

use conductor_core::{
    transport::{FrameDecoder, FrameEncoder},
    Conductor, ConductorConfig, ConductorMessage, ConnectionId, OllamaBackend, SurfaceCapabilities,
    SurfaceEvent, SurfaceHandle, SurfaceRegistry, SurfaceType,
};

/// Connection state tracking (internal to server, separate from SurfaceHandle)
struct ConnectionState {
    /// When the connection was established
    connected_at: std::time::Instant,
    /// Remote peer UID (from SO_PEERCRED)
    peer_uid: Option<u32>,
    /// Handle to abort the connection task
    abort_handle: tokio::task::AbortHandle,
}

/// Configuration for the daemon server
pub struct ServerConfig {
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Per-connection channel capacity
    pub connection_channel_capacity: usize,
    /// Event channel capacity (from surfaces to conductor)
    pub event_capacity: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            max_connections: 100,
            connection_channel_capacity: 256,
            event_capacity: 256,
        }
    }
}

/// The main daemon server
pub struct DaemonServer {
    /// Path to the Unix socket
    socket_path: PathBuf,
    /// Optional configuration file path
    config_path: Option<PathBuf>,
    /// Server configuration
    server_config: ServerConfig,
    /// Active connection state (task handles, peer info)
    connection_states: Arc<DashMap<ConnectionId, ConnectionState>>,
}

impl DaemonServer {
    /// Create a new daemon server
    pub fn new(socket_path: PathBuf, config_path: Option<PathBuf>) -> Result<Self> {
        Ok(Self {
            socket_path,
            config_path,
            server_config: ServerConfig::default(),
            connection_states: Arc::new(DashMap::new()),
        })
    }

    /// Get peer credentials from Unix socket
    #[cfg(unix)]
    fn get_peer_uid(stream: &UnixStream) -> Option<u32> {
        use std::os::unix::io::AsRawFd;

        let fd = stream.as_raw_fd();
        let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
        let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;

        let result = unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                &mut cred as *mut _ as *mut libc::c_void,
                &mut len,
            )
        };

        if result == 0 {
            Some(cred.uid)
        } else {
            None
        }
    }

    /// Prepare the socket path (create directory, remove stale socket)
    fn prepare_socket(&self) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = self.socket_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create socket directory: {parent:?}"))?;
                info!(path = ?parent, "Created socket directory");
            }
        }

        // Remove stale socket file
        if self.socket_path.exists() {
            warn!(path = ?self.socket_path, "Removing stale socket file");
            fs::remove_file(&self.socket_path).with_context(|| {
                format!("Failed to remove stale socket: {:?}", self.socket_path)
            })?;
        }

        Ok(())
    }

    /// Run the daemon server
    pub async fn run(
        &mut self,
        shutdown: Arc<AtomicBool>,
        reload_config: Arc<AtomicBool>,
    ) -> Result<()> {
        // Prepare socket
        self.prepare_socket()?;

        // Create Unix listener
        let listener = UnixListener::bind(&self.socket_path)
            .with_context(|| format!("Failed to bind to {:?}", self.socket_path))?;

        info!(path = ?self.socket_path, "Listening for connections");

        // Set socket permissions (owner-only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&self.socket_path, perms)?;
        }

        // Create shared SurfaceRegistry for multi-surface support
        let registry = SurfaceRegistry::new();

        // Create event channel for aggregated surface events (with connection ID)
        let (event_tx, mut event_rx) =
            mpsc::channel::<(ConnectionId, SurfaceEvent)>(self.server_config.event_capacity);

        // Create Conductor with SurfaceRegistry (multi-surface mode)
        let conductor_config = ConductorConfig::from_env();
        let backend = OllamaBackend::from_env();
        let conductor = Arc::new(Mutex::new(Conductor::new_with_registry(
            backend,
            conductor_config,
            registry.clone(),
        )));

        // Start the conductor
        {
            let mut c = conductor.lock().await;
            c.start().await?;
        }
        info!("Conductor started with multi-surface support");

        // Spawn task to handle events from surfaces
        let conductor_for_events = Arc::clone(&conductor);
        tokio::spawn(async move {
            while let Some((conn_id, event)) = event_rx.recv().await {
                debug!(conn_id = %conn_id, event = ?event, "Processing event");
                let mut c = conductor_for_events.lock().await;
                // Use handle_event_from for proper per-connection handling
                if let Err(e) = c.handle_event_from(conn_id, event).await {
                    warn!(conn_id = %conn_id, error = %e, "Failed to handle event");
                }
            }
        });

        // Spawn task for streaming token polling
        let conductor_for_streaming = Arc::clone(&conductor);
        tokio::spawn(async move {
            loop {
                {
                    let mut c = conductor_for_streaming.lock().await;
                    c.poll_streaming().await;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        });

        // Spawn task to periodically cleanup disconnected surfaces
        let registry_for_cleanup = registry.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                registry_for_cleanup.cleanup_disconnected();
            }
        });

        // Main accept loop
        loop {
            // Check for shutdown
            if shutdown.load(Ordering::SeqCst) {
                info!("Shutdown requested, stopping accept loop");
                break;
            }

            // Check for config reload
            if reload_config.swap(false, Ordering::SeqCst) {
                info!("Config reload requested");
                self.reload_config().await?;
            }

            // Accept with timeout to allow checking shutdown flag
            let accept_result =
                tokio::time::timeout(tokio::time::Duration::from_millis(100), listener.accept())
                    .await;

            let (stream, _addr) = match accept_result {
                Ok(Ok((stream, addr))) => (stream, addr),
                Ok(Err(e)) => {
                    error!(error = %e, "Accept failed");
                    continue;
                }
                Err(_) => {
                    // Timeout, loop back to check shutdown
                    continue;
                }
            };

            // Check connection limit
            if self.connection_states.len() >= self.server_config.max_connections {
                warn!("Connection limit reached, rejecting new connection");
                drop(stream);
                continue;
            }

            // Get peer credentials
            let peer_uid = Self::get_peer_uid(&stream);

            // Validate peer (same UID as daemon)
            let our_uid = unsafe { libc::getuid() };
            if let Some(uid) = peer_uid {
                if uid != our_uid && uid != 0 {
                    warn!(
                        peer_uid = uid,
                        our_uid = our_uid,
                        "Rejecting connection from different user"
                    );
                    drop(stream);
                    continue;
                }
            }

            // Create per-connection channel for messages TO this surface
            let (surface_tx, surface_rx) =
                mpsc::channel::<ConductorMessage>(self.server_config.connection_channel_capacity);

            // Allocate connection ID and register in the SurfaceRegistry
            let conn_id = ConnectionId::new();
            let handle = SurfaceHandle::new(
                conn_id,
                surface_tx,
                SurfaceType::Headless, // Will be updated when Handshake is received
                SurfaceCapabilities::headless(), // Will be updated when Handshake is received
            );
            registry.register(handle);

            info!(
                conn_id = %conn_id,
                peer_uid = ?peer_uid,
                active_connections = self.connection_states.len() + 1,
                "New connection accepted"
            );

            // Spawn connection handler
            let connection_states = Arc::clone(&self.connection_states);
            let event_tx = event_tx.clone();
            let registry_clone = registry.clone();

            let task_handle = tokio::spawn(
                Self::handle_connection(
                    conn_id,
                    stream,
                    event_tx,
                    surface_rx,
                    connection_states.clone(),
                    registry_clone,
                )
                .instrument(tracing::info_span!("connection", %conn_id)),
            );

            // Track connection state
            self.connection_states.insert(
                conn_id,
                ConnectionState {
                    connected_at: std::time::Instant::now(),
                    peer_uid,
                    abort_handle: task_handle.abort_handle(),
                },
            );
        }

        // Graceful shutdown
        self.shutdown().await
    }

    /// Handle a single client connection
    ///
    /// Each connection has its own dedicated channel for receiving messages.
    /// The connection reads events from the client and sends them to the Conductor,
    /// while also forwarding messages from the Conductor to the client.
    async fn handle_connection(
        conn_id: ConnectionId,
        stream: UnixStream,
        event_tx: mpsc::Sender<(ConnectionId, SurfaceEvent)>,
        mut surface_rx: mpsc::Receiver<ConductorMessage>,
        connection_states: Arc<DashMap<ConnectionId, ConnectionState>>,
        registry: SurfaceRegistry,
    ) {
        info!("Connection handler started");

        let (read_half, write_half) = stream.into_split();
        let read_half = Arc::new(Mutex::new(read_half));
        let write_half = Arc::new(Mutex::new(write_half));

        // Create frame encoder/decoder
        let mut decoder = FrameDecoder::new();
        let encoder = Arc::new(Mutex::new(FrameEncoder::new()));

        // Read buffer
        let mut read_buf = vec![0u8; 8192];

        loop {
            tokio::select! {
                // Read from client
                read_result = async {
                    use tokio::io::AsyncReadExt;
                    let mut reader = read_half.lock().await;
                    reader.read(&mut read_buf).await
                } => {
                    match read_result {
                        Ok(0) => {
                            // Connection closed
                            info!("Client disconnected (EOF)");
                            break;
                        }
                        Ok(n) => {
                            // Push data to decoder
                            decoder.push(&read_buf[..n]);

                            // Process complete frames
                            loop {
                                match decoder.decode::<SurfaceEvent>() {
                                    Ok(Some(event)) => {
                                        debug!(event = ?event, "Received event");
                                        if event_tx.send((conn_id, event)).await.is_err() {
                                            error!("Event channel closed");
                                            break;
                                        }
                                    }
                                    Ok(None) => {
                                        // Need more data
                                        break;
                                    }
                                    Err(e) => {
                                        warn!(error = %e, "Failed to decode event frame");
                                        break;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "Read error");
                            break;
                        }
                    }
                }

                // Write messages to this specific client
                msg = surface_rx.recv() => {
                    match msg {
                        Some(message) => {
                            let encoder = encoder.clone();
                            let write_half = write_half.clone();

                            let enc = encoder.lock().await;
                            match enc.encode(&message) {
                                Ok(frame) => {
                                    use tokio::io::AsyncWriteExt;
                                    drop(enc); // Release lock before async write
                                    let mut writer = write_half.lock().await;
                                    if let Err(e) = writer.write_all(&frame).await {
                                        error!(error = %e, "Write error");
                                        break;
                                    }
                                }
                                Err(e) => {
                                    warn!(error = %e, "Failed to encode message");
                                }
                            }
                        }
                        None => {
                            // Channel closed, connection should terminate
                            info!("Surface channel closed");
                            break;
                        }
                    }
                }
            }
        }

        // Unregister from SurfaceRegistry and remove from connection states
        registry.unregister(&conn_id);
        connection_states.remove(&conn_id);

        info!(
            active_connections = connection_states.len(),
            "Connection handler finished"
        );
    }

    /// Reload configuration from file
    async fn reload_config(&mut self) -> Result<()> {
        if let Some(ref config_path) = self.config_path {
            info!(path = ?config_path, "Reloading configuration");
            // TODO: Implement actual config reload
            // For now, just log that we would reload
            warn!("Config hot-reload not yet implemented");
        } else {
            info!("No config file specified, skipping reload");
        }
        Ok(())
    }

    /// Graceful shutdown
    async fn shutdown(&mut self) -> Result<()> {
        info!("Initiating graceful shutdown");

        // Abort all connection handlers
        let conn_ids: Vec<ConnectionId> = self.connection_states.iter().map(|r| *r.key()).collect();
        for conn_id in conn_ids {
            if let Some((_, conn_state)) = self.connection_states.remove(&conn_id) {
                info!(conn_id = %conn_id, "Aborting connection");
                conn_state.abort_handle.abort();
            }
        }

        // Wait a bit for handlers to finish
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Remove socket file
        if self.socket_path.exists() {
            fs::remove_file(&self.socket_path)
                .with_context(|| format!("Failed to remove socket: {:?}", self.socket_path))?;
            info!(path = ?self.socket_path, "Socket file removed");
        }

        info!("Shutdown complete");
        Ok(())
    }

    /// Get number of active connections
    pub fn connection_count(&self) -> usize {
        self.connection_states.len()
    }

    /// Get connection statistics
    pub fn connection_stats(&self) -> HashMap<ConnectionId, ConnectionStats> {
        self.connection_states
            .iter()
            .map(|r| {
                let conn_id = *r.key();
                let conn_state = r.value();
                (
                    conn_id,
                    ConnectionStats {
                        connected_at: conn_state.connected_at,
                        peer_uid: conn_state.peer_uid,
                        uptime_secs: conn_state.connected_at.elapsed().as_secs(),
                    },
                )
            })
            .collect()
    }
}

/// Statistics for a single connection
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    /// When the connection was established
    pub connected_at: std::time::Instant,
    /// Peer UID
    pub peer_uid: Option<u32>,
    /// Connection uptime in seconds
    pub uptime_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_id_display() {
        let id = ConnectionId::new();
        // Verify it formats with the "conn-" prefix followed by a UUID
        let formatted = format!("{id}");
        assert!(
            formatted.starts_with("conn-"),
            "ConnectionId should start with 'conn-'"
        );
        // UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx (36 chars)
        // Total format: conn- (5 chars) + UUID (36 chars) = 41 chars
        assert_eq!(
            formatted.len(),
            41,
            "ConnectionId should be 41 characters (conn- + UUID)"
        );
    }

    #[test]
    fn test_connection_id_randomness() {
        // Verify that ConnectionIds are cryptographically random
        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();
        let id3 = ConnectionId::new();

        // All should be different
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);

        // Format should be consistent
        for id in [&id1, &id2, &id3] {
            let formatted = format!("{id}");
            assert!(formatted.starts_with("conn-"));
            assert_eq!(formatted.len(), 41);
        }
    }

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.max_connections, 100);
        assert_eq!(config.connection_channel_capacity, 256);
        assert_eq!(config.event_capacity, 256);
    }
}
