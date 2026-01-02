//! Transport Factory
//!
//! Factory functions for creating transport instances based on configuration.
//! This abstracts transport creation from the ConductorClient.

use super::{
    config::{TransportConfig, TransportType},
    traits::{SurfaceTransport, TransportError},
};

#[cfg(unix)]
use super::unix_socket::UnixSocketClient;

/// Create a surface transport based on configuration
///
/// This factory function creates the appropriate transport implementation
/// based on the provided configuration. It abstracts the transport creation
/// logic from the client code.
///
/// # Arguments
///
/// * `config` - The transport configuration specifying which transport to use
///
/// # Returns
///
/// A boxed trait object implementing `SurfaceTransport`, or an error if the
/// transport cannot be created.
///
/// # Errors
///
/// Returns `TransportError::InvalidState` if:
/// - `InProcess` transport is requested (requires Conductor instance, use `InProcessTransport::new_pair()` directly)
/// - `WebSocket` transport is requested (not yet implemented)
///
/// # Example
///
/// ```ignore
/// use conductor_core::transport::{TransportConfig, create_surface_transport};
///
/// let config = TransportConfig::local();
/// let mut transport = create_surface_transport(&config)?;
/// transport.connect().await?;
/// ```
pub fn create_surface_transport(
    config: &TransportConfig,
) -> Result<Box<dyn SurfaceTransport>, TransportError> {
    match &config.transport {
        TransportType::InProcess => Err(TransportError::InvalidState(
            "InProcess transport requires Conductor instance; use InProcessTransport::new_pair() directly".into(),
        )),

        #[cfg(unix)]
        TransportType::UnixSocket { path } => {
            let client = match path {
                Some(socket_path) => UnixSocketClient::new(socket_path.clone()),
                None => UnixSocketClient::with_default_path(),
            };
            Ok(Box::new(client))
        }

        #[cfg(feature = "websocket")]
        TransportType::WebSocket { .. } => Err(TransportError::InvalidState(
            "WebSocket transport not yet implemented".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_inprocess_transport_fails() {
        let config = TransportConfig::embedded();
        let result = create_surface_transport(&config);

        assert!(result.is_err());
        match result {
            Err(TransportError::InvalidState(msg)) => {
                assert!(msg.contains("InProcess"));
            }
            Ok(_) => panic!("Expected error for InProcess transport"),
            Err(other) => panic!("Expected InvalidState error, got: {other}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_create_unix_socket_transport_default_path() {
        let config = TransportConfig::local();
        let result = create_surface_transport(&config);

        assert!(result.is_ok());
        // Transport is created but not connected
        let transport = result.unwrap();
        assert!(!transport.is_connected());
    }

    #[cfg(unix)]
    #[test]
    fn test_create_unix_socket_transport_custom_path() {
        use std::path::PathBuf;

        let config = TransportConfig {
            transport: TransportType::UnixSocket {
                path: Some(PathBuf::from("/tmp/test-conductor.sock")),
            },
            ..Default::default()
        };
        let result = create_surface_transport(&config);

        assert!(result.is_ok());
        let transport = result.unwrap();
        assert!(!transport.is_connected());
    }
}
