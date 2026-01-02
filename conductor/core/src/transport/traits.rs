//! Transport Traits
//!
//! Core trait definitions for Conductor-Surface communication.
//!
//! Two traits define the two sides of the connection:
//! - `SurfaceTransport`: Client side (TUI, `WebUI`, etc.)
//! - `ConductorTransport`: Server side (Conductor daemon)

use std::fmt;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::events::SurfaceEvent;
use crate::messages::ConductorMessage;

/// Unique identifier for a connected surface
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionId(pub String);

impl ConnectionId {
    /// Generate a new unique connection ID using cryptographically random 128-bit value
    #[must_use]
    pub fn new() -> Self {
        use rand::Rng;
        let bytes: [u8; 16] = rand::thread_rng().gen();
        Self(format!("conn_{}", hex::encode(bytes)))
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Errors that can occur during transport operations
#[derive(Debug)]
pub enum TransportError {
    /// Connection to peer failed
    ConnectionFailed(String),
    /// Connection was closed
    ConnectionClosed,
    /// Failed to send message
    SendFailed(String),
    /// Failed to receive message
    ReceiveFailed(String),
    /// Message serialization/deserialization error
    SerializationError(String),
    /// Authentication or authorization failed
    AuthenticationFailed(String),
    /// IO error from underlying transport
    IoError(std::io::Error),
    /// Transport not in expected state
    InvalidState(String),
    /// Frame checksum mismatch - data corruption detected
    ChecksumMismatch {
        /// Expected checksum value
        expected: u32,
        /// Actual checksum value received
        actual: u32,
    },
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionFailed(msg) => write!(f, "Connection failed: {msg}"),
            Self::ConnectionClosed => write!(f, "Connection closed"),
            Self::SendFailed(msg) => write!(f, "Send failed: {msg}"),
            Self::ReceiveFailed(msg) => write!(f, "Receive failed: {msg}"),
            Self::SerializationError(msg) => write!(f, "Serialization error: {msg}"),
            Self::AuthenticationFailed(msg) => write!(f, "Authentication failed: {msg}"),
            Self::IoError(e) => write!(f, "IO error: {e}"),
            Self::InvalidState(msg) => write!(f, "Invalid state: {msg}"),
            Self::ChecksumMismatch { expected, actual } => write!(
                f,
                "Checksum mismatch: expected {expected:#010x}, got {actual:#010x}"
            ),
        }
    }
}

impl std::error::Error for TransportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for TransportError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

/// Transport trait for Surface (client) side
///
/// Surfaces use this trait to communicate with the Conductor.
/// Implementations handle the specific transport mechanism.
#[async_trait]
pub trait SurfaceTransport: Send + Sync {
    /// Connect to the Conductor
    ///
    /// Establishes a connection. For embedded mode, this is a no-op.
    /// For socket transports, this connects to the socket.
    async fn connect(&mut self) -> Result<(), TransportError>;

    /// Disconnect from the Conductor
    ///
    /// Gracefully closes the connection.
    async fn disconnect(&mut self) -> Result<(), TransportError>;

    /// Send a `SurfaceEvent` to the Conductor
    async fn send(&self, event: SurfaceEvent) -> Result<(), TransportError>;

    /// Receive a `ConductorMessage` (blocks until message available)
    async fn recv(&mut self) -> Result<ConductorMessage, TransportError>;

    /// Try to receive a `ConductorMessage` (non-blocking)
    fn try_recv(&mut self) -> Option<ConductorMessage>;

    /// Check if currently connected
    fn is_connected(&self) -> bool;
}

/// Transport trait for Conductor (server) side
///
/// The Conductor uses this to accept and manage surface connections.
/// Supports multiple simultaneous connections.
#[async_trait]
pub trait ConductorTransport: Send + Sync {
    /// Start listening for connections
    ///
    /// For socket transports, this binds and listens on the socket.
    async fn listen(&mut self) -> Result<(), TransportError>;

    /// Accept a new connection
    ///
    /// Returns the connection ID and a receiver for that connection's events.
    async fn accept(
        &mut self,
    ) -> Result<(ConnectionId, mpsc::Receiver<SurfaceEvent>), TransportError>;

    /// Send a message to a specific surface
    async fn send_to(
        &self,
        conn_id: &ConnectionId,
        msg: ConductorMessage,
    ) -> Result<(), TransportError>;

    /// Broadcast a message to all connected surfaces
    async fn broadcast(&self, msg: ConductorMessage) -> Result<(), TransportError>;

    /// Disconnect a specific surface
    async fn disconnect(&self, conn_id: &ConnectionId) -> Result<(), TransportError>;

    /// Get all connected surface IDs
    fn connections(&self) -> Vec<ConnectionId>;

    /// Stop accepting new connections and close all existing ones
    async fn shutdown(&mut self) -> Result<(), TransportError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_id_unique() {
        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_connection_id_display() {
        let id = ConnectionId("test_conn".to_string());
        assert_eq!(format!("{}", id), "test_conn");
    }

    #[test]
    fn test_transport_error_display() {
        let err = TransportError::ConnectionFailed("test".to_string());
        assert!(err.to_string().contains("Connection failed"));

        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let err = TransportError::IoError(io_err);
        assert!(err.to_string().contains("IO error"));
    }
}
