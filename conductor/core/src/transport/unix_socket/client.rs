//! Unix Socket Client Transport
//!
//! Client-side (Surface) implementation of Unix socket transport.
//! Connects to a Conductor daemon and handles bidirectional communication.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::mpsc;

use crate::events::SurfaceEvent;
use crate::messages::ConductorMessage;
use crate::transport::frame::{encode, FrameDecoder};
use crate::transport::traits::{SurfaceTransport, TransportError};

/// Client-side Unix socket transport for Surfaces
///
/// Connects to the Conductor's Unix socket and provides
/// bidirectional communication.
pub struct UnixSocketClient {
    /// Path to the Conductor's socket
    socket_path: PathBuf,
    /// Channel to receive messages from Conductor
    msg_rx: Option<mpsc::Receiver<ConductorMessage>>,
    /// Channel to send events to Conductor
    event_tx: Option<mpsc::Sender<SurfaceEvent>>,
    /// Whether we're connected
    connected: Arc<AtomicBool>,
}

impl UnixSocketClient {
    /// Create a new Unix socket client
    ///
    /// # Arguments
    ///
    /// * `socket_path` - Path to the Conductor's socket file
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            msg_rx: None,
            event_tx: None,
            connected: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a client using the default socket path
    pub fn with_default_path() -> Self {
        Self::new(super::default_socket_path())
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }
}

#[async_trait]
impl SurfaceTransport for UnixSocketClient {
    async fn connect(&mut self) -> Result<(), TransportError> {
        if self.connected.load(Ordering::SeqCst) {
            return Err(TransportError::InvalidState(
                "Already connected".to_string(),
            ));
        }

        // Connect to the socket
        let stream = UnixStream::connect(&self.socket_path).await.map_err(|e| {
            TransportError::ConnectionFailed(format!(
                "Failed to connect to {:?}: {}",
                self.socket_path, e
            ))
        })?;

        // Split for concurrent read/write
        let (mut read_half, mut write_half) = stream.into_split();

        // Channels
        let (msg_tx, msg_rx) = mpsc::channel::<ConductorMessage>(100);
        let (event_tx, mut event_rx) = mpsc::channel::<SurfaceEvent>(100);

        let connected = Arc::clone(&self.connected);
        connected.store(true, Ordering::SeqCst);

        // Spawn read task: stream -> msg_tx (ConductorMessages from server)
        let connected_read = Arc::clone(&connected);
        tokio::spawn(async move {
            let mut decoder = FrameDecoder::new();
            let mut buf = [0u8; 4096];

            loop {
                match read_half.read(&mut buf).await {
                    Ok(0) => {
                        // EOF - server closed connection
                        tracing::debug!("Connection closed by server");
                        break;
                    }
                    Ok(n) => {
                        decoder.push(&buf[..n]);

                        // Decode all available frames
                        loop {
                            match decoder.decode::<ConductorMessage>() {
                                Ok(Some(msg)) => {
                                    if msg_tx.send(msg).await.is_err() {
                                        tracing::debug!("Message receiver dropped");
                                        break;
                                    }
                                }
                                Ok(None) => break, // Need more data
                                Err(e) => {
                                    tracing::warn!(error = %e, "Frame decode error");
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Read error");
                        break;
                    }
                }
            }

            connected_read.store(false, Ordering::SeqCst);
            tracing::info!("Disconnected from Conductor");
        });

        // Spawn write task: event_rx -> stream (SurfaceEvents to server)
        let connected_write = Arc::clone(&connected);
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match encode(&event) {
                    Ok(data) => {
                        if let Err(e) = write_half.write_all(&data).await {
                            tracing::warn!(error = %e, "Write error");
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Encode error");
                    }
                }
            }

            connected_write.store(false, Ordering::SeqCst);
        });

        self.msg_rx = Some(msg_rx);
        self.event_tx = Some(event_tx);

        tracing::info!(path = ?self.socket_path, "Connected to Conductor");

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        self.connected.store(false, Ordering::SeqCst);
        self.msg_rx = None;
        self.event_tx = None;

        tracing::info!("Disconnected");
        Ok(())
    }

    async fn send(&self, event: SurfaceEvent) -> Result<(), TransportError> {
        if !self.connected.load(Ordering::SeqCst) {
            return Err(TransportError::InvalidState(
                "Not connected".to_string(),
            ));
        }

        if let Some(ref tx) = self.event_tx {
            tx.send(event)
                .await
                .map_err(|_| TransportError::SendFailed("Channel closed".to_string()))
        } else {
            Err(TransportError::InvalidState(
                "Not connected".to_string(),
            ))
        }
    }

    async fn recv(&mut self) -> Result<ConductorMessage, TransportError> {
        if let Some(ref mut rx) = self.msg_rx {
            rx.recv().await.ok_or(TransportError::ConnectionClosed)
        } else {
            Err(TransportError::InvalidState(
                "Not connected".to_string(),
            ))
        }
    }

    fn try_recv(&mut self) -> Option<ConductorMessage> {
        self.msg_rx.as_mut()?.try_recv().ok()
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::traits::ConductorTransport;
    use crate::transport::unix_socket::UnixSocketServer;
    use crate::ConductorMessage;
    use std::time::Duration;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_client_connect_no_server() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("nonexistent.sock");

        let mut client = UnixSocketClient::new(socket_path);
        let result = client.connect().await;

        assert!(matches!(result, Err(TransportError::ConnectionFailed(_))));
        assert!(!client.is_connected());
    }

    #[tokio::test]
    async fn test_client_server_roundtrip() {
        use crate::events::{SurfaceCapabilities, SurfaceType};
        use crate::messages::{ConductorState, EventId};

        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        // Start server
        let mut server = UnixSocketServer::new(socket_path.clone());
        server.listen().await.unwrap();

        // Connect client
        let mut client = UnixSocketClient::new(socket_path);

        // Spawn accept task
        let server_accept = tokio::spawn(async move {
            let (conn_id, mut event_rx) = server.accept().await.unwrap();

            // Wait for an event
            let event = tokio::time::timeout(Duration::from_secs(1), event_rx.recv())
                .await
                .unwrap()
                .unwrap();

            // Send a response
            server
                .send_to(
                    &conn_id,
                    ConductorMessage::State {
                        state: ConductorState::Ready,
                    },
                )
                .await
                .unwrap();

            (server, event)
        });

        // Small delay to ensure server is ready
        tokio::time::sleep(Duration::from_millis(10)).await;

        client.connect().await.unwrap();
        assert!(client.is_connected());

        // Send an event
        let event = SurfaceEvent::Connected {
            event_id: EventId("test".to_string()),
            surface_type: SurfaceType::Tui,
            capabilities: SurfaceCapabilities::tui(),
        };
        client.send(event).await.unwrap();

        // Receive response
        let msg = tokio::time::timeout(Duration::from_secs(1), client.recv())
            .await
            .unwrap()
            .unwrap();

        assert!(matches!(
            msg,
            ConductorMessage::State {
                state: ConductorState::Ready
            }
        ));

        // Clean up
        let (mut server, received_event) = server_accept.await.unwrap();
        assert!(matches!(received_event, SurfaceEvent::Connected { .. }));

        client.disconnect().await.unwrap();
        assert!(!client.is_connected());

        server.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_client_send_not_connected() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        let client = UnixSocketClient::new(socket_path);

        let event = SurfaceEvent::QuitRequested {
            event_id: crate::messages::EventId("test".to_string()),
        };

        let result = client.send(event).await;
        assert!(matches!(result, Err(TransportError::InvalidState(_))));
    }

    #[tokio::test]
    async fn test_client_recv_not_connected() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        let mut client = UnixSocketClient::new(socket_path);

        let result = client.recv().await;
        assert!(matches!(result, Err(TransportError::InvalidState(_))));
    }
}
