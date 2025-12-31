//! Transport Configuration
//!
//! Configuration types for selecting and configuring transport mechanisms.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Transport type selection
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransportType {
    /// Direct in-process channels (embedded mode)
    ///
    /// This is the default when the TUI embeds the Conductor directly.
    /// No IPC overhead, but no process separation.
    InProcess,

    /// Unix domain socket (local process separation)
    ///
    /// Recommended for local development and production use.
    /// Provides process separation with minimal overhead and strong security.
    #[cfg(unix)]
    UnixSocket {
        /// Socket path (None = use default)
        ///
        /// Default: $XDG_RUNTIME_DIR/ai-way/conductor.sock
        /// Fallback: /tmp/ai-way-$UID/conductor.sock
        path: Option<PathBuf>,
    },

    /// WebSocket (remote surfaces)
    ///
    /// For remote surfaces like iPad, TV, or web browser.
    /// Requires explicit opt-in and authentication.
    #[cfg(feature = "websocket")]
    WebSocket {
        /// Listen address (e.g., "127.0.0.1:8765")
        listen_addr: String,
        /// Require authentication token
        require_auth: bool,
    },
}

impl Default for TransportType {
    fn default() -> Self {
        Self::InProcess
    }
}

/// Transport configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Which transport to use
    pub transport: TransportType,

    /// Connection timeout in milliseconds
    ///
    /// How long to wait when connecting to the Conductor.
    pub connect_timeout_ms: u64,

    /// Read timeout in milliseconds (0 = no timeout)
    ///
    /// How long to wait for a message before timing out.
    pub read_timeout_ms: u64,

    /// Whether to enable heartbeat
    ///
    /// If true, the transport will send periodic pings to detect
    /// dead connections.
    pub heartbeat_enabled: bool,

    /// Heartbeat interval in milliseconds
    ///
    /// How often to send heartbeat pings.
    pub heartbeat_interval_ms: u64,

    /// Reconnection attempts (0 = no reconnection)
    ///
    /// How many times to attempt reconnection on disconnect.
    pub reconnect_attempts: u32,

    /// Delay between reconnection attempts in milliseconds
    pub reconnect_delay_ms: u64,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            transport: TransportType::default(),
            connect_timeout_ms: 5000,
            read_timeout_ms: 0,
            heartbeat_enabled: true,
            heartbeat_interval_ms: 30_000,
            reconnect_attempts: 3,
            reconnect_delay_ms: 1000,
        }
    }
}

impl TransportConfig {
    /// Create configuration for embedded (in-process) mode
    pub fn embedded() -> Self {
        Self {
            transport: TransportType::InProcess,
            ..Default::default()
        }
    }

    /// Create configuration for local Unix socket mode
    #[cfg(unix)]
    pub fn local() -> Self {
        Self {
            transport: TransportType::UnixSocket { path: None },
            ..Default::default()
        }
    }

    /// Load configuration from environment variables
    ///
    /// Environment variables:
    /// - `CONDUCTOR_TRANSPORT`: "inprocess", "unix", "socket", "websocket", "ws"
    /// - `CONDUCTOR_SOCKET`: Path to Unix socket
    /// - `CONDUCTOR_WS_ADDR`: WebSocket listen address
    /// - `CONDUCTOR_WS_AUTH`: "1" or "true" to require auth
    /// - `CONDUCTOR_CONNECT_TIMEOUT`: Connection timeout in ms
    /// - `CONDUCTOR_HEARTBEAT`: "0" or "false" to disable
    /// - `CONDUCTOR_HEARTBEAT_INTERVAL`: Heartbeat interval in ms
    /// - `CONDUCTOR_RECONNECT_ATTEMPTS`: Number of reconnection attempts
    pub fn from_env() -> Self {
        let transport =
            match std::env::var("CONDUCTOR_TRANSPORT")
                .as_deref()
                .map(str::to_lowercase)
            {
                Ok(ref s) if s == "inprocess" || s == "embedded" => TransportType::InProcess,

                #[cfg(unix)]
                Ok(ref s) if s == "unix" || s == "socket" => TransportType::UnixSocket {
                    path: std::env::var("CONDUCTOR_SOCKET").ok().map(PathBuf::from),
                },

                #[cfg(feature = "websocket")]
                Ok(ref s) if s == "websocket" || s == "ws" => TransportType::WebSocket {
                    listen_addr: std::env::var("CONDUCTOR_WS_ADDR")
                        .unwrap_or_else(|_| "127.0.0.1:8765".into()),
                    require_auth: std::env::var("CONDUCTOR_WS_AUTH")
                        .map(|v| v == "1" || v.to_lowercase() == "true")
                        .unwrap_or(true),
                },

                _ => TransportType::default(),
            };

        let heartbeat_enabled = std::env::var("CONDUCTOR_HEARTBEAT")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(true);

        Self {
            transport,
            connect_timeout_ms: std::env::var("CONDUCTOR_CONNECT_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5000),
            read_timeout_ms: std::env::var("CONDUCTOR_READ_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            heartbeat_enabled,
            heartbeat_interval_ms: std::env::var("CONDUCTOR_HEARTBEAT_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30_000),
            reconnect_attempts: std::env::var("CONDUCTOR_RECONNECT_ATTEMPTS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            reconnect_delay_ms: std::env::var("CONDUCTOR_RECONNECT_DELAY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
        }
    }

    /// Check if this is an in-process (embedded) configuration
    pub fn is_embedded(&self) -> bool {
        matches!(self.transport, TransportType::InProcess)
    }

    /// Check if this is a Unix socket configuration
    #[cfg(unix)]
    pub fn is_unix_socket(&self) -> bool {
        matches!(self.transport, TransportType::UnixSocket { .. })
    }
}

/// Get the default Unix socket path
///
/// Uses XDG_RUNTIME_DIR if available, otherwise /tmp/ai-way-$UID/
#[cfg(unix)]
pub fn default_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir)
            .join("ai-way")
            .join("conductor.sock")
    } else {
        let uid = unsafe { libc::getuid() };
        PathBuf::from(format!("/tmp/ai-way-{}/conductor.sock", uid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_type_default() {
        let t = TransportType::default();
        assert!(matches!(t, TransportType::InProcess));
    }

    #[test]
    fn test_transport_config_default() {
        let config = TransportConfig::default();
        assert!(config.is_embedded());
        assert_eq!(config.connect_timeout_ms, 5000);
        assert!(config.heartbeat_enabled);
    }

    #[test]
    fn test_transport_config_embedded() {
        let config = TransportConfig::embedded();
        assert!(config.is_embedded());
    }

    #[cfg(unix)]
    #[test]
    fn test_transport_config_local() {
        let config = TransportConfig::local();
        assert!(config.is_unix_socket());
    }

    #[cfg(unix)]
    #[test]
    fn test_default_socket_path() {
        let path = default_socket_path();
        assert!(path.to_string_lossy().contains("conductor.sock"));
    }
}
