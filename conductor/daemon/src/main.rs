//! Conductor Daemon - Multi-Surface Orchestration Server
//!
//! This is the main entry point for the Conductor daemon, which provides
//! the headless orchestration layer for ai-way. Multiple surfaces (TUI, WebUI,
//! mobile apps) can connect to this daemon via Unix socket.
//!
//! # Usage
//!
//! ```bash
//! # Start with defaults
//! conductor-daemon
//!
//! # Custom socket path
//! conductor-daemon --socket-path /tmp/my-conductor.sock
//!
//! # With config file
//! conductor-daemon --config /etc/ai-way/conductor.toml
//!
//! # Daemonize (run in background)
//! conductor-daemon --daemonize
//!
//! # Verbose logging
//! RUST_LOG=debug conductor-daemon
//! ```
//!
//! # Signals
//!
//! - `SIGTERM` / `SIGINT`: Graceful shutdown
//! - `SIGHUP`: Reload configuration (hot reload)

mod server;

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use tokio::signal::unix::{signal, SignalKind};
use tracing::{error, info, warn};

use server::DaemonServer;

/// Conductor Daemon - Multi-surface orchestration server for ai-way
#[derive(Parser, Debug)]
#[command(name = "conductor-daemon")]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Unix socket path for surface connections
    #[arg(short = 's', long, env = "CONDUCTOR_SOCKET", value_name = "PATH")]
    socket_path: Option<PathBuf>,

    /// Configuration file path
    #[arg(short = 'c', long, env = "CONDUCTOR_CONFIG", value_name = "FILE")]
    config: Option<PathBuf>,

    /// Run as daemon (fork to background)
    #[arg(short = 'd', long)]
    daemonize: bool,

    /// PID file path (for daemon mode)
    #[arg(long, env = "CONDUCTOR_PID_FILE", value_name = "PATH")]
    pid_file: Option<PathBuf>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short = 'l', long, env = "CONDUCTOR_LOG_LEVEL", default_value = "info")]
    log_level: String,
}

/// Get the default socket path
///
/// Uses XDG_RUNTIME_DIR if available, otherwise /tmp/ai-way-$UID/
fn default_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir)
            .join("ai-way")
            .join("conductor.sock")
    } else {
        let uid = unsafe { libc::getuid() };
        PathBuf::from(format!("/tmp/ai-way-{uid}/conductor.sock"))
    }
}

/// Get the default PID file path
fn default_pid_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir)
            .join("ai-way")
            .join("conductor.pid")
    } else {
        let uid = unsafe { libc::getuid() };
        PathBuf::from(format!("/tmp/ai-way-{uid}/conductor.pid"))
    }
}

/// Write PID file
fn write_pid_file(path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create PID directory: {parent:?}"))?;
    }

    let pid = std::process::id();
    let mut file =
        fs::File::create(path).with_context(|| format!("Failed to create PID file: {path:?}"))?;
    writeln!(file, "{pid}")?;

    info!(pid = pid, path = ?path, "PID file created");
    Ok(())
}

/// Remove PID file
fn remove_pid_file(path: &PathBuf) {
    if path.exists() {
        if let Err(e) = fs::remove_file(path) {
            warn!(error = %e, path = ?path, "Failed to remove PID file");
        } else {
            info!(path = ?path, "PID file removed");
        }
    }
}

/// Check if another daemon is running by checking PID file
fn check_existing_daemon(pid_path: &PathBuf) -> Result<()> {
    if !pid_path.exists() {
        return Ok(());
    }

    let pid_str = fs::read_to_string(pid_path)
        .with_context(|| format!("Failed to read PID file: {pid_path:?}"))?;

    let pid: i32 = pid_str
        .trim()
        .parse()
        .with_context(|| "Invalid PID in file")?;

    // Check if process is running (signal 0 just checks existence)
    let result = unsafe { libc::kill(pid, 0) };
    if result == 0 {
        anyhow::bail!(
            "Another conductor-daemon is already running (PID: {pid}). \
             Stop it first or remove {pid_path:?} if it's stale."
        );
    }

    // Process not running, PID file is stale
    warn!(pid = pid, "Removing stale PID file");
    fs::remove_file(pid_path)?;
    Ok(())
}

/// Initialize logging with the specified level
fn init_logging(level: &str) -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new(format!(
            "conductor_daemon={level},conductor_core={level}"
        ))
    });

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    Ok(())
}

/// Daemonize the process (fork to background)
fn daemonize() -> Result<()> {
    use nix::unistd::{fork, setsid, ForkResult};

    // First fork
    match unsafe { fork() } {
        Ok(ForkResult::Parent { .. }) => {
            // Parent exits
            std::process::exit(0);
        }
        Ok(ForkResult::Child) => {
            // Child continues
        }
        Err(e) => {
            anyhow::bail!("First fork failed: {e}");
        }
    }

    // Create new session
    setsid().context("setsid failed")?;

    // Second fork (prevent acquiring controlling terminal)
    match unsafe { fork() } {
        Ok(ForkResult::Parent { .. }) => {
            std::process::exit(0);
        }
        Ok(ForkResult::Child) => {
            // Grandchild continues as daemon
        }
        Err(e) => {
            anyhow::bail!("Second fork failed: {e}");
        }
    }

    // Close stdin, stdout, stderr (or redirect to /dev/null)
    // For now we keep them open for logging purposes in early stages

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging first
    init_logging(&args.log_level)?;

    info!("Conductor Daemon starting");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("PID: {}", std::process::id());

    // Resolve paths
    let socket_path = args.socket_path.unwrap_or_else(default_socket_path);
    let pid_path = args.pid_file.unwrap_or_else(default_pid_path);

    info!(socket_path = ?socket_path, "Socket path");
    info!(pid_path = ?pid_path, "PID file path");

    if let Some(ref config_path) = args.config {
        info!(config_path = ?config_path, "Config file");
    }

    // Check for existing daemon
    check_existing_daemon(&pid_path)?;

    // Daemonize if requested
    if args.daemonize {
        info!("Daemonizing...");
        daemonize()?;
        // After daemonizing, PID changes
        info!("Daemonized, new PID: {}", std::process::id());
    }

    // Write PID file
    write_pid_file(&pid_path)?;

    // Setup signal handlers
    let shutdown = Arc::new(AtomicBool::new(false));
    let reload_config = Arc::new(AtomicBool::new(false));

    // Spawn signal handler task
    let shutdown_clone = Arc::clone(&shutdown);
    let reload_clone = Arc::clone(&reload_config);
    tokio::spawn(async move {
        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to install SIGINT handler");
        let mut sighup = signal(SignalKind::hangup()).expect("Failed to install SIGHUP handler");

        loop {
            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, initiating shutdown");
                    shutdown_clone.store(true, Ordering::SeqCst);
                    break;
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT, initiating shutdown");
                    shutdown_clone.store(true, Ordering::SeqCst);
                    break;
                }
                _ = sighup.recv() => {
                    info!("Received SIGHUP, marking config for reload");
                    reload_clone.store(true, Ordering::SeqCst);
                }
            }
        }
    });

    // Create and run daemon server
    let mut server = DaemonServer::new(socket_path.clone(), args.config)?;

    // Run the server
    let result = server.run(shutdown, reload_config).await;

    // Cleanup
    info!("Shutting down...");
    remove_pid_file(&pid_path);

    // Remove socket file if it still exists
    if socket_path.exists() {
        if let Err(e) = fs::remove_file(&socket_path) {
            warn!(error = %e, "Failed to remove socket file");
        }
    }

    match result {
        Ok(()) => {
            info!("Conductor daemon stopped cleanly");
            Ok(())
        }
        Err(e) => {
            error!(error = %e, "Daemon stopped with error");
            Err(e)
        }
    }
}
