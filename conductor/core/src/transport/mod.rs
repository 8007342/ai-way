//! Transport Layer for Conductor-Surface IPC
//!
//! Provides abstraction over different transport mechanisms:
//! - `InProcess`: Direct channel communication (embedded mode)
//! - `UnixSocket`: Local IPC via Unix domain sockets
//! - WebSocket: Remote IPC for web/mobile surfaces (future)
//!
//! # Design Philosophy
//!
//! The transport layer separates the communication mechanism from the
//! Conductor and Surface logic. This enables:
//! - Process separation (Conductor as daemon, TUI as client)
//! - Multiple simultaneous surfaces
//! - Future WebSocket support for remote surfaces
//!
//! # Security
//!
//! - Unix sockets use `SO_PEERCRED` to validate peer UID
//! - Socket files are created with 0600 permissions
//! - No network exposure by default

pub mod config;
pub mod factory;
pub mod frame;
pub mod heartbeat;
pub mod in_process;
pub mod rate_limit;
pub mod traits;
#[cfg(unix)]
pub mod unix_socket;

// Re-exports for convenience
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
