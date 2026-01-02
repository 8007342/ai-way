//! WebSocket Connection Traits
//!
//! Trait definitions for WebSocket connection handling. These traits define
//! the interface for WebSocket server and connection implementations.
//!
//! **NOTE**: These are interface definitions only. Actual implementation
//! is blocked pending security review (see P5.2).

use std::net::SocketAddr;

use async_trait::async_trait;

use super::frame_adapter::WebSocketFrame;
use super::security::SecurityError;
use crate::events::SurfaceEvent;
use crate::messages::ConductorMessage;
use crate::transport::ConnectionId;

/// Errors that can occur during WebSocket operations
#[derive(Debug)]
pub enum WebSocketError {
    /// Connection failed to establish
    ConnectionFailed(String),
    /// TLS handshake failed
    TlsError(String),
    /// WebSocket handshake failed
    HandshakeError(String),
    /// Security check failed
    SecurityError(SecurityError),
    /// Connection was closed
    ConnectionClosed,
    /// Send failed
    SendFailed(String),
    /// Receive failed
    ReceiveFailed(String),
    /// Protocol error
    ProtocolError(String),
    /// Invalid frame
    InvalidFrame(String),
    /// IO error
    IoError(std::io::Error),
    /// Timeout
    Timeout(String),
}

impl std::fmt::Display for WebSocketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionFailed(msg) => write!(f, "Connection failed: {msg}"),
            Self::TlsError(msg) => write!(f, "TLS error: {msg}"),
            Self::HandshakeError(msg) => write!(f, "Handshake error: {msg}"),
            Self::SecurityError(err) => write!(f, "Security error: {err}"),
            Self::ConnectionClosed => write!(f, "Connection closed"),
            Self::SendFailed(msg) => write!(f, "Send failed: {msg}"),
            Self::ReceiveFailed(msg) => write!(f, "Receive failed: {msg}"),
            Self::ProtocolError(msg) => write!(f, "Protocol error: {msg}"),
            Self::InvalidFrame(msg) => write!(f, "Invalid frame: {msg}"),
            Self::IoError(err) => write!(f, "IO error: {err}"),
            Self::Timeout(msg) => write!(f, "Timeout: {msg}"),
        }
    }
}

impl std::error::Error for WebSocketError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(err) => Some(err),
            Self::SecurityError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for WebSocketError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<SecurityError> for WebSocketError {
    fn from(err: SecurityError) -> Self {
        Self::SecurityError(err)
    }
}

/// State of a WebSocket connection
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebSocketConnectionState {
    /// Connection is being established
    Connecting,
    /// WebSocket handshake in progress
    Handshaking,
    /// Connection is open and ready
    Open,
    /// Connection is closing
    Closing,
    /// Connection is closed
    Closed,
}

impl WebSocketConnectionState {
    /// Check if the connection is usable for sending/receiving
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Open)
    }

    /// Check if the connection is in a terminal state
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Closed)
    }
}

/// A WebSocket connection to a Surface
///
/// Represents an established WebSocket connection. Implementations handle
/// the underlying socket management, frame parsing, and message conversion.
///
/// **NOTE**: This trait defines the interface. Implementation is pending
/// security review.
#[async_trait]
pub trait WebSocketConnection: Send + Sync {
    /// Get the connection ID
    fn connection_id(&self) -> &ConnectionId;

    /// Get the remote address
    fn remote_addr(&self) -> SocketAddr;

    /// Get the connection state
    fn state(&self) -> WebSocketConnectionState;

    /// Get the origin header (if provided during handshake)
    fn origin(&self) -> Option<&str>;

    /// Check if the connection is open
    fn is_open(&self) -> bool {
        self.state().is_active()
    }

    /// Send a Conductor message to the surface
    ///
    /// The message is serialized to JSON and sent as a text frame.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection is not open or sending fails.
    async fn send(&self, message: ConductorMessage) -> Result<(), WebSocketError>;

    /// Send a raw WebSocket frame
    ///
    /// For low-level control when needed (e.g., binary sprite data).
    ///
    /// # Errors
    ///
    /// Returns an error if the connection is not open or sending fails.
    async fn send_frame(&self, frame: WebSocketFrame) -> Result<(), WebSocketError>;

    /// Receive a Surface event from the connection
    ///
    /// Blocks until a message is received or the connection closes.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection closes or a protocol error occurs.
    async fn recv(&mut self) -> Result<SurfaceEvent, WebSocketError>;

    /// Try to receive without blocking
    ///
    /// Returns `Ok(None)` if no message is immediately available.
    fn try_recv(&mut self) -> Result<Option<SurfaceEvent>, WebSocketError>;

    /// Send a ping frame
    ///
    /// Used for connection health checking at the WebSocket layer.
    ///
    /// # Errors
    ///
    /// Returns an error if sending fails.
    async fn ping(&self, payload: &[u8]) -> Result<(), WebSocketError>;

    /// Close the connection gracefully
    ///
    /// Sends a close frame with the given code and reason, then waits
    /// for the close handshake to complete.
    ///
    /// # Arguments
    ///
    /// * `code` - WebSocket close code (e.g., 1000 for normal closure)
    /// * `reason` - Human-readable close reason
    ///
    /// # Errors
    ///
    /// Returns an error if the close handshake fails.
    async fn close(&mut self, code: u16, reason: &str) -> Result<(), WebSocketError>;

    /// Forcefully close the connection
    ///
    /// Immediately closes the underlying socket without the close handshake.
    fn force_close(&mut self);
}

/// WebSocket server listener trait
///
/// Accepts incoming WebSocket connections. Implementations handle
/// binding, TLS termination, and the WebSocket upgrade handshake.
///
/// **NOTE**: This trait defines the interface. Implementation is pending
/// security review.
#[async_trait]
pub trait WebSocketListener: Send + Sync {
    /// The connection type produced by this listener
    type Connection: WebSocketConnection;

    /// Start listening for connections
    ///
    /// Binds to the configured address and starts accepting connections.
    ///
    /// # Errors
    ///
    /// Returns an error if binding fails.
    async fn start(&mut self) -> Result<(), WebSocketError>;

    /// Accept a new connection
    ///
    /// Blocks until a new connection is available. The returned connection
    /// has completed the WebSocket handshake and is ready for use.
    ///
    /// # Errors
    ///
    /// Returns an error if acceptance fails or the listener is not started.
    async fn accept(&mut self) -> Result<Self::Connection, WebSocketError>;

    /// Get the local address the listener is bound to
    fn local_addr(&self) -> Result<SocketAddr, WebSocketError>;

    /// Check if TLS is enabled
    fn is_tls_enabled(&self) -> bool;

    /// Get the number of active connections
    fn active_connections(&self) -> usize;

    /// Stop the listener
    ///
    /// Stops accepting new connections. Existing connections are not affected.
    async fn stop(&mut self) -> Result<(), WebSocketError>;
}

/// Marker trait for WebSocket client connections
///
/// Used for surface-side WebSocket connections that connect to the Conductor.
///
/// **NOTE**: Client implementation is planned for web surface support.
#[async_trait]
pub trait WebSocketClient: WebSocketConnection {
    /// Connect to a WebSocket server
    ///
    /// # Arguments
    ///
    /// * `url` - WebSocket URL (ws:// or wss://)
    ///
    /// # Errors
    ///
    /// Returns an error if connection fails.
    async fn connect(url: &str) -> Result<Self, WebSocketError>
    where
        Self: Sized;

    /// Reconnect to the server
    ///
    /// Uses the original URL from the initial connection.
    ///
    /// # Errors
    ///
    /// Returns an error if reconnection fails.
    async fn reconnect(&mut self) -> Result<(), WebSocketError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_websocket_error_display() {
        let err = WebSocketError::ConnectionFailed("timeout".to_string());
        assert!(err.to_string().contains("timeout"));

        let err = WebSocketError::TlsError("certificate invalid".to_string());
        assert!(err.to_string().contains("certificate"));

        let err = WebSocketError::ConnectionClosed;
        assert!(err.to_string().contains("closed"));
    }

    #[test]
    fn test_websocket_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        let ws_err: WebSocketError = io_err.into();
        assert!(matches!(ws_err, WebSocketError::IoError(_)));
    }

    #[test]
    fn test_websocket_error_from_security() {
        let sec_err = SecurityError::AuthenticationFailed {
            reason: "bad token".to_string(),
        };
        let ws_err: WebSocketError = sec_err.into();
        assert!(matches!(ws_err, WebSocketError::SecurityError(_)));
    }

    #[test]
    fn test_connection_state() {
        assert!(!WebSocketConnectionState::Connecting.is_active());
        assert!(!WebSocketConnectionState::Handshaking.is_active());
        assert!(WebSocketConnectionState::Open.is_active());
        assert!(!WebSocketConnectionState::Closing.is_active());
        assert!(!WebSocketConnectionState::Closed.is_active());

        assert!(!WebSocketConnectionState::Connecting.is_terminal());
        assert!(!WebSocketConnectionState::Open.is_terminal());
        assert!(WebSocketConnectionState::Closed.is_terminal());
    }

    #[test]
    fn test_websocket_error_source() {
        // IO error should have a source
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test");
        let ws_err = WebSocketError::IoError(io_err);
        assert!(ws_err.source().is_some());

        // Security error should have a source
        let sec_err = SecurityError::ConnectionTimeout;
        let ws_err = WebSocketError::SecurityError(sec_err);
        assert!(ws_err.source().is_some());

        // Other errors don't have a source
        let ws_err = WebSocketError::ConnectionClosed;
        assert!(ws_err.source().is_none());
    }
}
