//! Unix Socket Server Transport
//!
//! Server-side (Conductor) implementation of Unix socket transport.
//! Handles multiple concurrent surface connections.

use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, RwLock};

use crate::events::SurfaceEvent;
use crate::messages::ConductorMessage;
use crate::transport::frame::{encode, FrameDecoder};
use crate::transport::traits::{ConductorTransport, ConnectionId, TransportError};

/// Server-side Unix socket transport for the Conductor
///
/// Accepts connections from surface clients and manages bidirectional
/// communication with each connected surface.
pub struct UnixSocketServer {
    /// Path to the socket file
    socket_path: PathBuf,
    /// The bound listener (None until listen() is called)
    listener: Option<UnixListener>,
    /// Active connections: ConnectionId -> ConnectionHandle
    connections: Arc<RwLock<HashMap<ConnectionId, ConnectionHandle>>>,
}

/// Handle to a single connection
struct ConnectionHandle {
    /// Channel to send messages to this surface
    tx: mpsc::Sender<ConductorMessage>,
}

impl UnixSocketServer {
    /// Create a new Unix socket server
    ///
    /// # Arguments
    ///
    /// * `socket_path` - Path where the socket file will be created
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            listener: None,
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a server using the default socket path
    pub fn with_default_path() -> Self {
        Self::new(super::default_socket_path())
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }

    /// Set socket permissions to 0600 (owner-only)
    fn set_socket_permissions(&self) -> Result<(), TransportError> {
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&self.socket_path, perms)?;
        Ok(())
    }

    /// Validate peer credentials
    ///
    /// On Linux, uses SO_PEERCRED to verify the connecting process
    /// runs as the same user as the Conductor.
    #[cfg(target_os = "linux")]
    fn validate_peer(stream: &UnixStream) -> Result<(), TransportError> {
        use std::os::unix::io::AsRawFd;

        let fd = stream.as_raw_fd();

        let cred = unsafe {
            let mut cred: libc::ucred = std::mem::zeroed();
            let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;

            let result = libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                &mut cred as *mut _ as *mut libc::c_void,
                &mut len,
            );

            if result < 0 {
                return Err(TransportError::AuthenticationFailed(
                    "Failed to get peer credentials".to_string(),
                ));
            }
            cred
        };

        let my_uid = unsafe { libc::getuid() };

        if cred.uid != my_uid {
            tracing::warn!(
                peer_uid = cred.uid,
                my_uid = my_uid,
                "Rejecting connection from different user"
            );
            return Err(TransportError::AuthenticationFailed(format!(
                "Peer UID {} does not match server UID {}",
                cred.uid, my_uid
            )));
        }

        tracing::debug!(peer_uid = cred.uid, peer_pid = cred.pid, "Peer validated");
        Ok(())
    }

    /// Validate peer credentials (non-Linux fallback)
    #[cfg(not(target_os = "linux"))]
    fn validate_peer(_stream: &UnixStream) -> Result<(), TransportError> {
        // On macOS and other platforms, we rely on filesystem permissions
        // since SO_PEERCRED is Linux-specific
        tracing::debug!("Peer validation skipped (non-Linux platform)");
        Ok(())
    }
}

#[async_trait]
impl ConductorTransport for UnixSocketServer {
    async fn listen(&mut self) -> Result<(), TransportError> {
        // Create parent directories if needed
        if let Some(parent) = self.socket_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                TransportError::IoError(std::io::Error::new(
                    e.kind(),
                    format!("Failed to create directory {:?}: {}", parent, e),
                ))
            })?;
        }

        // Remove existing socket file if present
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path).map_err(|e| {
                TransportError::IoError(std::io::Error::new(
                    e.kind(),
                    format!("Failed to remove old socket {:?}: {}", self.socket_path, e),
                ))
            })?;
        }

        // Bind and listen
        let listener = UnixListener::bind(&self.socket_path)?;

        // Set restrictive permissions
        self.set_socket_permissions()?;

        self.listener = Some(listener);

        tracing::info!(path = ?self.socket_path, "Conductor listening on Unix socket");
        Ok(())
    }

    async fn accept(
        &mut self,
    ) -> Result<(ConnectionId, mpsc::Receiver<SurfaceEvent>), TransportError> {
        let listener = self
            .listener
            .as_ref()
            .ok_or_else(|| TransportError::InvalidState("Not listening".to_string()))?;

        let (stream, _addr) = listener.accept().await?;

        // Validate peer credentials (same user)
        Self::validate_peer(&stream)?;

        let conn_id = ConnectionId::new();

        // Channels for this connection
        let (event_tx, event_rx) = mpsc::channel::<SurfaceEvent>(100);
        let (msg_tx, mut msg_rx) = mpsc::channel::<ConductorMessage>(100);

        // Split the stream for concurrent read/write
        let (mut read_half, mut write_half) = stream.into_split();

        // Spawn read task: stream -> event_tx (SurfaceEvents from client)
        let conn_id_read = conn_id.clone();
        let connections_read = Arc::clone(&self.connections);
        tokio::spawn(async move {
            let mut decoder = FrameDecoder::new();
            let mut buf = [0u8; 4096];

            loop {
                match read_half.read(&mut buf).await {
                    Ok(0) => {
                        // EOF - connection closed
                        tracing::debug!(conn_id = %conn_id_read, "Connection closed by peer");
                        break;
                    }
                    Ok(n) => {
                        decoder.push(&buf[..n]);

                        // Decode all available frames
                        loop {
                            match decoder.decode::<SurfaceEvent>() {
                                Ok(Some(event)) => {
                                    if event_tx.send(event).await.is_err() {
                                        tracing::debug!(
                                            conn_id = %conn_id_read,
                                            "Event receiver dropped"
                                        );
                                        break;
                                    }
                                }
                                Ok(None) => break, // Need more data
                                Err(e) => {
                                    tracing::warn!(
                                        conn_id = %conn_id_read,
                                        error = %e,
                                        "Frame decode error"
                                    );
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(conn_id = %conn_id_read, error = %e, "Read error");
                        break;
                    }
                }
            }

            // Clean up connection
            connections_read.write().await.remove(&conn_id_read);
            tracing::info!(conn_id = %conn_id_read, "Connection ended");
        });

        // Spawn write task: msg_rx -> stream (ConductorMessages to client)
        let conn_id_write = conn_id.clone();
        tokio::spawn(async move {
            while let Some(msg) = msg_rx.recv().await {
                match encode(&msg) {
                    Ok(data) => {
                        if let Err(e) = write_half.write_all(&data).await {
                            tracing::warn!(
                                conn_id = %conn_id_write,
                                error = %e,
                                "Write error"
                            );
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::warn!(conn_id = %conn_id_write, error = %e, "Encode error");
                    }
                }
            }
        });

        // Store connection handle
        let handle = ConnectionHandle { tx: msg_tx };
        self.connections
            .write()
            .await
            .insert(conn_id.clone(), handle);

        tracing::info!(conn_id = %conn_id, "Surface connected");

        Ok((conn_id, event_rx))
    }

    async fn send_to(
        &self,
        conn_id: &ConnectionId,
        msg: ConductorMessage,
    ) -> Result<(), TransportError> {
        let connections = self.connections.read().await;

        if let Some(handle) = connections.get(conn_id) {
            handle
                .tx
                .send(msg)
                .await
                .map_err(|_| TransportError::SendFailed("Channel closed".to_string()))
        } else {
            Err(TransportError::SendFailed(format!(
                "Unknown connection: {}",
                conn_id
            )))
        }
    }

    async fn broadcast(&self, msg: ConductorMessage) -> Result<(), TransportError> {
        let connections = self.connections.read().await;

        for (conn_id, handle) in connections.iter() {
            if let Err(e) = handle.tx.send(msg.clone()).await {
                tracing::warn!(conn_id = %conn_id, error = %e, "Broadcast send failed");
            }
        }

        Ok(())
    }

    async fn disconnect(&self, conn_id: &ConnectionId) -> Result<(), TransportError> {
        self.connections.write().await.remove(conn_id);
        tracing::info!(conn_id = %conn_id, "Disconnected");
        Ok(())
    }

    fn connections(&self) -> Vec<ConnectionId> {
        // This is a sync method, so we use blocking approach
        // In practice, this should be called from an async context
        // Consider making this async in the trait if needed
        let handle = tokio::runtime::Handle::try_current();

        match handle {
            Ok(h) => {
                // We're in a tokio context
                std::thread::scope(|_| {
                    h.block_on(async { self.connections.read().await.keys().cloned().collect() })
                })
            }
            Err(_) => {
                // Not in tokio context, return empty
                // Caller should use async version
                Vec::new()
            }
        }
    }

    async fn shutdown(&mut self) -> Result<(), TransportError> {
        // Drop the listener to stop accepting connections
        self.listener = None;

        // Clear all connections
        self.connections.write().await.clear();

        // Remove socket file
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path).ok();
        }

        tracing::info!("Unix socket server shut down");
        Ok(())
    }
}

impl Drop for UnixSocketServer {
    fn drop(&mut self) {
        // Clean up socket file on drop
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path).ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_server_listen() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        let mut server = UnixSocketServer::new(socket_path.clone());
        server.listen().await.unwrap();

        // Socket file should exist
        assert!(socket_path.exists());

        // Permissions should be 0600
        let metadata = std::fs::metadata(&socket_path).unwrap();
        let perms = metadata.permissions();
        assert_eq!(perms.mode() & 0o777, 0o600);

        server.shutdown().await.unwrap();

        // Socket file should be removed
        assert!(!socket_path.exists());
    }

    #[tokio::test]
    async fn test_server_accept_not_listening() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        let mut server = UnixSocketServer::new(socket_path);

        // Should error when not listening
        let result = server.accept().await;
        assert!(matches!(result, Err(TransportError::InvalidState(_))));
    }

    #[tokio::test]
    async fn test_server_accept_connect() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        let mut server = UnixSocketServer::new(socket_path.clone());
        server.listen().await.unwrap();

        // Spawn a task to connect as a client
        let socket_path_clone = socket_path.clone();
        let client_task = tokio::spawn(async move {
            // Small delay to ensure server is ready
            tokio::time::sleep(Duration::from_millis(10)).await;

            let stream = tokio::net::UnixStream::connect(&socket_path_clone)
                .await
                .unwrap();

            // Keep connection alive briefly
            tokio::time::sleep(Duration::from_millis(100)).await;
            drop(stream);
        });

        // Accept the connection
        let result = tokio::time::timeout(Duration::from_secs(1), server.accept()).await;

        assert!(result.is_ok());
        let (conn_id, _event_rx) = result.unwrap().unwrap();
        assert!(!conn_id.0.is_empty());

        client_task.await.unwrap();
        server.shutdown().await.unwrap();
    }
}
