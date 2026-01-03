//! Transport Layer for Conductor-Surface IPC
//!
//! Provides abstraction over different transport mechanisms:
//! - `InProcess`: Direct channel communication (embedded mode)
//! - `UnixSocket`: Local IPC via Unix domain sockets
//! - `WebSocket`: Remote IPC for web/mobile surfaces
//!
//! # Design Philosophy
//!
//! The transport layer separates the communication mechanism from the
//! Conductor and Surface logic. This enables:
//! - Process separation (Conductor as daemon, TUI as client)
//! - Multiple simultaneous surfaces
//! - WebSocket support for remote surfaces (web, mobile, TV)
//!
//! # Security
//!
//! - Unix sockets use `SO_PEERCRED` to validate peer UID
//! - Socket files are created with 0600 permissions
//! - Session token authentication prevents unauthorized connections
//! - No network exposure by default
//! - WebSocket requires TLS and origin validation for production

pub mod auth;
pub mod config;
pub mod factory;
pub mod frame;
pub mod heartbeat;
pub mod in_process;
pub mod rate_limit;
pub mod traits;
#[cfg(unix)]
pub mod unix_socket;
pub mod websocket;

// Re-exports for convenience
pub use auth::{get_runtime_dir, get_token_path, remove_token_file, SessionToken, TokenError};
pub use config::{TransportConfig, TransportType};
pub use factory::create_surface_transport;
pub use frame::{FrameDecoder, FrameEncoder};
pub use heartbeat::{
    ConnectionHealth, HeartbeatConfig, HeartbeatEvent, HeartbeatMonitor, HeartbeatTask,
};
pub use in_process::InProcessTransport;
pub use rate_limit::{
    apply_backpressure, ConnectionRateLimitMetrics, ConnectionRateLimiter, RateLimitConfig,
    RateLimitError, RateLimitResult, TransportRateLimitMetrics, TransportRateLimiter,
};
pub use traits::{ConductorTransport, ConnectionId, SurfaceTransport, TransportError};

#[cfg(unix)]
pub use unix_socket::{UnixSocketClient, UnixSocketServer};

// WebSocket transport types (infrastructure only - implementation pending security review)
pub use websocket::{
    // Security
    AuthenticationMethod,
    // Frame handling
    FrameConversionError,
    OriginPolicy,
    OriginValidationResult,
    SecurityConfig,
    SecurityError,
    // Configuration
    TlsConfig,
    WebSocketConfig,
    WebSocketConfigBuilder,
    // Traits (interface definitions)
    WebSocketConnection,
    WebSocketConnectionState,
    WebSocketError,
    WebSocketFrame,
    WebSocketFrameAdapter,
    WebSocketFrameType,
    WebSocketListener,
};
