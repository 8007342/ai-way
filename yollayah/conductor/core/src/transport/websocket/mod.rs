//! WebSocket Transport Infrastructure
//!
//! Provides types and configuration for WebSocket-based Conductor-Surface IPC.
//! This enables remote surfaces (web browsers, mobile apps, TV surfaces) to
//! connect to the Conductor over the network.
//!
//! # Design Philosophy
//!
//! The WebSocket transport extends the existing transport layer abstraction
//! to support remote clients while maintaining security through:
//!
//! - TLS encryption for all production connections
//! - Origin validation to prevent cross-site attacks
//! - Authentication token handshake
//! - Rate limiting integration
//! - Message size limits
//!
//! # Security Considerations
//!
//! WebSocket transport introduces network exposure, requiring additional security:
//!
//! - **TLS Required**: Production deployments must use TLS (wss://)
//! - **Origin Validation**: Only configured origins are allowed
//! - **Token Authentication**: Surfaces must present valid auth token
//! - **Rate Limiting**: Per-connection and per-origin limits apply
//! - **Message Size Limits**: Prevents memory exhaustion attacks
//!
//! # Status
//!
//! **NOTE**: This module provides types and configuration only.
//! Actual WebSocket server implementation is blocked pending security review.
//! See P5.2 for implementation timeline.
//!
//! # Example
//!
//! ```rust
//! use conductor_core::transport::websocket::{WebSocketConfig, TlsConfig};
//! use std::path::PathBuf;
//! use std::time::Duration;
//!
//! // Production configuration with TLS
//! let config = WebSocketConfig::builder()
//!     .bind_address("0.0.0.0:8443")
//!     .tls(TlsConfig {
//!         cert_path: PathBuf::from("/etc/ai-way/cert.pem"),
//!         key_path: PathBuf::from("/etc/ai-way/key.pem"),
//!         ca_path: None,
//!     })
//!     .allowed_origins(vec!["https://app.ai-way.local".into()])
//!     .build();
//!
//! // Development configuration (no TLS, localhost only)
//! let dev_config = WebSocketConfig::development();
//! ```

mod config;
mod frame_adapter;
mod security;
mod traits;

pub use config::{TlsConfig, WebSocketConfig, WebSocketConfigBuilder};
pub use frame_adapter::{
    FrameConversionError, WebSocketFrame, WebSocketFrameAdapter, WebSocketFrameType,
};
pub use security::{
    AuthenticationMethod, OriginPolicy, OriginValidationResult, SecurityConfig, SecurityError,
};
pub use traits::{
    WebSocketConnection, WebSocketConnectionState, WebSocketError, WebSocketListener,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::Duration;

    #[test]
    fn test_default_config() {
        let config = WebSocketConfig::default();
        assert_eq!(config.bind_address(), "127.0.0.1:8765");
        assert!(config.tls().is_none());
        assert!(config.compression());
        assert_eq!(config.max_message_size(), 10 * 1024 * 1024);
    }

    #[test]
    fn test_builder_pattern() {
        let config = WebSocketConfig::builder()
            .bind_address("0.0.0.0:9000")
            .max_message_size(5 * 1024 * 1024)
            .compression(false)
            .heartbeat_interval(Duration::from_secs(60))
            .build();

        assert_eq!(config.bind_address(), "0.0.0.0:9000");
        assert_eq!(config.max_message_size(), 5 * 1024 * 1024);
        assert!(!config.compression());
        assert_eq!(config.heartbeat_interval(), Duration::from_secs(60));
    }

    #[test]
    fn test_development_config() {
        let config = WebSocketConfig::development();
        assert!(config.tls().is_none());
        assert!(!config.security().require_origin_validation);
        assert_eq!(config.bind_address(), "127.0.0.1:8765");
    }

    #[test]
    fn test_production_config_requires_tls() {
        let tls = TlsConfig {
            cert_path: PathBuf::from("/tmp/cert.pem"),
            key_path: PathBuf::from("/tmp/key.pem"),
            ca_path: None,
        };

        let config = WebSocketConfig::builder()
            .bind_address("0.0.0.0:8443")
            .tls(tls.clone())
            .build();

        assert!(config.tls().is_some());
        assert_eq!(
            config.tls().unwrap().cert_path,
            PathBuf::from("/tmp/cert.pem")
        );
    }

    #[test]
    fn test_tls_config_validation() {
        let tls = TlsConfig {
            cert_path: PathBuf::from("/etc/certs/server.crt"),
            key_path: PathBuf::from("/etc/certs/server.key"),
            ca_path: Some(PathBuf::from("/etc/certs/ca.crt")),
        };

        // Validation will fail because files don't exist, but structure is correct
        let result = tls.validate();
        assert!(result.is_err()); // Files don't exist in test
    }

    #[test]
    fn test_security_config_defaults() {
        let config = SecurityConfig::default();
        assert!(config.require_origin_validation);
        assert!(config.require_authentication);
        assert_eq!(config.max_connections_per_origin, 10);
    }

    #[test]
    fn test_frame_adapter_text_frame() {
        use crate::messages::{ConductorMessage, ConductorState};

        let adapter = WebSocketFrameAdapter::new();
        let msg = ConductorMessage::State {
            state: ConductorState::Ready,
        };

        let frame = adapter.to_websocket_frame(&msg).unwrap();
        assert!(matches!(frame.frame_type, WebSocketFrameType::Text));
        assert!(!frame.payload.is_empty());
    }

    #[test]
    fn test_frame_adapter_roundtrip() {
        use crate::messages::{ConductorMessage, ConductorState};

        let adapter = WebSocketFrameAdapter::new();
        let original = ConductorMessage::State {
            state: ConductorState::Thinking,
        };

        let frame = adapter.to_websocket_frame(&original).unwrap();
        let decoded: ConductorMessage = adapter.from_websocket_frame(&frame).unwrap();

        // Compare the discriminants (message type)
        assert!(matches!(decoded, ConductorMessage::State { .. }));
    }

    #[test]
    fn test_origin_validation() {
        let policy = OriginPolicy::new(vec![
            "https://app.example.com".into(),
            "https://admin.example.com".into(),
        ]);

        assert!(policy.validate("https://app.example.com").is_allowed());
        assert!(policy.validate("https://admin.example.com").is_allowed());
        assert!(!policy.validate("https://evil.com").is_allowed());
        assert!(!policy.validate("http://app.example.com").is_allowed()); // HTTP not HTTPS
    }

    #[test]
    fn test_origin_policy_allow_all() {
        let policy = OriginPolicy::allow_all();
        assert!(policy.validate("https://any.domain.com").is_allowed());
        assert!(policy.validate("http://localhost:3000").is_allowed());
    }

    #[test]
    fn test_websocket_error_display() {
        let err = WebSocketError::ConnectionFailed("timeout".to_string());
        assert!(err.to_string().contains("timeout"));

        let err = WebSocketError::TlsError("certificate invalid".to_string());
        assert!(err.to_string().contains("certificate"));
    }

    #[test]
    fn test_config_serialization() {
        let config = WebSocketConfig::builder()
            .bind_address("127.0.0.1:8080")
            .max_message_size(1024 * 1024)
            .build();

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: WebSocketConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.bind_address(), deserialized.bind_address());
        assert_eq!(config.max_message_size(), deserialized.max_message_size());
    }

    #[test]
    fn test_tls_config_serialization() {
        let tls = TlsConfig {
            cert_path: PathBuf::from("/path/to/cert.pem"),
            key_path: PathBuf::from("/path/to/key.pem"),
            ca_path: Some(PathBuf::from("/path/to/ca.pem")),
        };

        let json = serde_json::to_string(&tls).unwrap();
        let deserialized: TlsConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(tls.cert_path, deserialized.cert_path);
        assert_eq!(tls.key_path, deserialized.key_path);
        assert_eq!(tls.ca_path, deserialized.ca_path);
    }

    #[test]
    fn test_security_config_serialization() {
        let security = SecurityConfig {
            require_origin_validation: true,
            require_authentication: true,
            authentication_method: AuthenticationMethod::BearerToken,
            allowed_origins: vec!["https://example.com".into()],
            max_connections_per_origin: 5,
            connection_timeout_ms: 30_000,
            idle_timeout_ms: 300_000,
        };

        let json = serde_json::to_string(&security).unwrap();
        let deserialized: SecurityConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(
            security.require_origin_validation,
            deserialized.require_origin_validation
        );
        assert_eq!(
            security.max_connections_per_origin,
            deserialized.max_connections_per_origin
        );
    }
}
