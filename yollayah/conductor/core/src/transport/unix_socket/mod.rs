//! Unix Socket Transport
//!
//! Default transport for local process separation using Unix domain sockets.
//!
//! # Socket Location
//!
//! Default: `$XDG_RUNTIME_DIR/ai-way/conductor.sock`
//! Fallback: `/tmp/ai-way-$UID/conductor.sock`
//!
//! # Security
//!
//! - Socket created with mode 0600 (owner-only access)
//! - Peer UID validated via `SO_PEERCRED` (Linux)
//! - No network exposure (Unix domain sockets only)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐                    ┌─────────────────┐
//! │  Surface (TUI)  │                    │    Conductor    │
//! │                 │                    │                 │
//! │ UnixSocketClient├───────────────────►│ UnixSocketServer│
//! │                 │    Unix Socket     │                 │
//! │  SurfaceEvent ─►│    conductor.sock  │◄─ SurfaceEvent  │
//! │  ◄─ ConductorMsg│                    │ ConductorMsg ─► │
//! └─────────────────┘                    └─────────────────┘
//! ```

mod client;
mod server;

pub use client::UnixSocketClient;
pub use server::UnixSocketServer;

use std::path::PathBuf;

/// Get the default socket path for the Conductor
///
/// Uses `XDG_RUNTIME_DIR` if available (preferred), otherwise falls back
/// to /tmp/ai-way-$UID/ for compatibility.
#[must_use]
pub fn default_socket_path() -> PathBuf {
    super::config::default_socket_path()
}
