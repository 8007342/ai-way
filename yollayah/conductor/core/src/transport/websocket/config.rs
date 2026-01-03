//! WebSocket Configuration
//!
//! Configuration structures for WebSocket transport including TLS settings,
//! connection parameters, and message handling options.

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::security::SecurityConfig;

/// TLS configuration for secure WebSocket connections (wss://)
///
/// Used to configure TLS encryption for production deployments.
/// All paths must point to PEM-encoded files.
///
/// # Example
///
/// ```rust
/// use conductor_core::transport::websocket::TlsConfig;
/// use std::path::PathBuf;
///
/// let tls = TlsConfig {
///     cert_path: PathBuf::from("/etc/ai-way/server.crt"),
///     key_path: PathBuf::from("/etc/ai-way/server.key"),
///     ca_path: Some(PathBuf::from("/etc/ai-way/ca.crt")),
/// };
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to the server certificate file (PEM format)
    pub cert_path: PathBuf,

    /// Path to the private key file (PEM format)
    pub key_path: PathBuf,

    /// Optional path to CA certificate for client certificate validation
    ///
    /// When set, enables mutual TLS (mTLS) where clients must present
    /// a certificate signed by this CA.
    pub ca_path: Option<PathBuf>,
}

impl TlsConfig {
    /// Create a new TLS configuration
    #[must_use]
    pub fn new(cert_path: PathBuf, key_path: PathBuf) -> Self {
        Self {
            cert_path,
            key_path,
            ca_path: None,
        }
    }

    /// Create TLS configuration with client certificate validation
    #[must_use]
    pub fn with_client_auth(cert_path: PathBuf, key_path: PathBuf, ca_path: PathBuf) -> Self {
        Self {
            cert_path,
            key_path,
            ca_path: Some(ca_path),
        }
    }

    /// Validate the TLS configuration
    ///
    /// Checks that all specified files exist and are readable.
    ///
    /// # Errors
    ///
    /// Returns an error describing which file is missing or unreadable.
    pub fn validate(&self) -> Result<(), TlsConfigError> {
        // Check certificate file
        if !self.cert_path.exists() {
            return Err(TlsConfigError::CertificateNotFound(self.cert_path.clone()));
        }
        if !self.cert_path.is_file() {
            return Err(TlsConfigError::InvalidCertificate(format!(
                "Not a file: {}",
                self.cert_path.display()
            )));
        }

        // Check key file
        if !self.key_path.exists() {
            return Err(TlsConfigError::KeyNotFound(self.key_path.clone()));
        }
        if !self.key_path.is_file() {
            return Err(TlsConfigError::InvalidKey(format!(
                "Not a file: {}",
                self.key_path.display()
            )));
        }

        // Check CA file if specified
        if let Some(ref ca_path) = self.ca_path {
            if !ca_path.exists() {
                return Err(TlsConfigError::CaNotFound(ca_path.clone()));
            }
            if !ca_path.is_file() {
                return Err(TlsConfigError::InvalidCa(format!(
                    "Not a file: {}",
                    ca_path.display()
                )));
            }
        }

        Ok(())
    }

    /// Check if mutual TLS (client certificate auth) is enabled
    #[must_use]
    pub fn is_mtls_enabled(&self) -> bool {
        self.ca_path.is_some()
    }
}

/// Errors that can occur during TLS configuration validation
#[derive(Clone, Debug)]
pub enum TlsConfigError {
    /// Certificate file not found
    CertificateNotFound(PathBuf),
    /// Certificate file is invalid
    InvalidCertificate(String),
    /// Private key file not found
    KeyNotFound(PathBuf),
    /// Private key file is invalid
    InvalidKey(String),
    /// CA certificate file not found
    CaNotFound(PathBuf),
    /// CA certificate file is invalid
    InvalidCa(String),
}

impl std::fmt::Display for TlsConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CertificateNotFound(path) => {
                write!(f, "Certificate file not found: {}", path.display())
            }
            Self::InvalidCertificate(msg) => write!(f, "Invalid certificate: {msg}"),
            Self::KeyNotFound(path) => write!(f, "Private key file not found: {}", path.display()),
            Self::InvalidKey(msg) => write!(f, "Invalid private key: {msg}"),
            Self::CaNotFound(path) => {
                write!(f, "CA certificate file not found: {}", path.display())
            }
            Self::InvalidCa(msg) => write!(f, "Invalid CA certificate: {msg}"),
        }
    }
}

impl std::error::Error for TlsConfigError {}

/// WebSocket transport configuration
///
/// Configures all aspects of the WebSocket server including binding,
/// TLS, message handling, and security.
///
/// # Example
///
/// ```rust
/// use conductor_core::transport::websocket::{WebSocketConfig, TlsConfig};
/// use std::time::Duration;
///
/// // Production configuration
/// let config = WebSocketConfig::builder()
///     .bind_address("0.0.0.0:8443")
///     .max_message_size(10 * 1024 * 1024)
///     .compression(true)
///     .heartbeat_interval(Duration::from_secs(30))
///     .build();
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// Address to bind the WebSocket server to
    ///
    /// Format: "host:port" (e.g., "127.0.0.1:8765", "0.0.0.0:8443")
    bind_address: String,

    /// TLS configuration (required for production)
    ///
    /// When None, the server runs without TLS (ws://).
    /// Production deployments should always use TLS (wss://).
    #[serde(default)]
    tls: Option<TlsConfig>,

    /// Maximum message size in bytes
    ///
    /// Messages larger than this will be rejected.
    /// Default: 10 MB (matches frame.rs MAX_FRAME_SIZE)
    max_message_size: usize,

    /// Enable per-message compression (permessage-deflate)
    ///
    /// Reduces bandwidth at the cost of CPU. Recommended for
    /// remote connections with limited bandwidth.
    compression: bool,

    /// Heartbeat (ping/pong) interval in milliseconds
    ///
    /// How often to send WebSocket ping frames to detect dead connections.
    /// This is in addition to the application-level heartbeat.
    heartbeat_interval_ms: u64,

    /// Connection timeout for initial handshake in milliseconds
    ///
    /// How long to wait for the WebSocket handshake to complete.
    handshake_timeout_ms: u64,

    /// Maximum pending connections in accept queue
    ///
    /// Limits the number of connections waiting to be accepted.
    max_pending_connections: u32,

    /// Security configuration
    security: SecurityConfig,

    /// Enable debug logging of frame contents
    ///
    /// WARNING: May log sensitive data. Only for development.
    #[serde(default)]
    debug_frames: bool,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8765".to_string(),
            tls: None,
            max_message_size: 10 * 1024 * 1024, // 10 MB
            compression: true,
            heartbeat_interval_ms: 30_000, // 30 seconds
            handshake_timeout_ms: 10_000,  // 10 seconds
            max_pending_connections: 128,
            security: SecurityConfig::default(),
            debug_frames: false,
        }
    }
}

impl WebSocketConfig {
    /// Create a new builder for WebSocket configuration
    #[must_use]
    pub fn builder() -> WebSocketConfigBuilder {
        WebSocketConfigBuilder::new()
    }

    /// Create a development configuration
    ///
    /// - No TLS (ws://)
    /// - Localhost only binding
    /// - Relaxed security (no origin validation)
    /// - Debug frames enabled
    #[must_use]
    pub fn development() -> Self {
        Self {
            bind_address: "127.0.0.1:8765".to_string(),
            tls: None,
            max_message_size: 10 * 1024 * 1024,
            compression: true,
            heartbeat_interval_ms: 30_000,
            handshake_timeout_ms: 30_000,
            max_pending_connections: 32,
            security: SecurityConfig::development(),
            debug_frames: true,
        }
    }

    /// Get the bind address
    #[must_use]
    pub fn bind_address(&self) -> &str {
        &self.bind_address
    }

    /// Get the TLS configuration
    #[must_use]
    pub fn tls(&self) -> Option<&TlsConfig> {
        self.tls.as_ref()
    }

    /// Check if TLS is enabled
    #[must_use]
    pub fn is_tls_enabled(&self) -> bool {
        self.tls.is_some()
    }

    /// Get maximum message size
    #[must_use]
    pub fn max_message_size(&self) -> usize {
        self.max_message_size
    }

    /// Check if compression is enabled
    #[must_use]
    pub fn compression(&self) -> bool {
        self.compression
    }

    /// Get heartbeat interval
    #[must_use]
    pub fn heartbeat_interval(&self) -> Duration {
        Duration::from_millis(self.heartbeat_interval_ms)
    }

    /// Get handshake timeout
    #[must_use]
    pub fn handshake_timeout(&self) -> Duration {
        Duration::from_millis(self.handshake_timeout_ms)
    }

    /// Get maximum pending connections
    #[must_use]
    pub fn max_pending_connections(&self) -> u32 {
        self.max_pending_connections
    }

    /// Get security configuration
    #[must_use]
    pub fn security(&self) -> &SecurityConfig {
        &self.security
    }

    /// Check if debug frame logging is enabled
    #[must_use]
    pub fn debug_frames(&self) -> bool {
        self.debug_frames
    }

    /// Validate the configuration
    ///
    /// Checks that all settings are valid and consistent.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration is invalid.
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        // Validate bind address format
        if self.bind_address.is_empty() {
            return Err(ConfigValidationError::InvalidBindAddress(
                "Bind address cannot be empty".to_string(),
            ));
        }

        // Basic format check for host:port
        if !self.bind_address.contains(':') {
            return Err(ConfigValidationError::InvalidBindAddress(
                "Bind address must be in host:port format".to_string(),
            ));
        }

        // Validate TLS config if present
        if let Some(ref tls) = self.tls {
            tls.validate().map_err(ConfigValidationError::TlsError)?;
        }

        // Validate message size
        if self.max_message_size == 0 {
            return Err(ConfigValidationError::InvalidMessageSize(
                "Message size must be greater than 0".to_string(),
            ));
        }

        // Validate timeouts
        if self.heartbeat_interval_ms == 0 {
            return Err(ConfigValidationError::InvalidTimeout(
                "Heartbeat interval must be greater than 0".to_string(),
            ));
        }

        if self.handshake_timeout_ms == 0 {
            return Err(ConfigValidationError::InvalidTimeout(
                "Handshake timeout must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }

    /// Check if this is a production-safe configuration
    ///
    /// Production configurations should:
    /// - Have TLS enabled
    /// - Have origin validation enabled
    /// - Have authentication enabled
    /// - Not have debug frames enabled
    #[must_use]
    pub fn is_production_safe(&self) -> bool {
        self.tls.is_some()
            && self.security.require_origin_validation
            && self.security.require_authentication
            && !self.debug_frames
    }
}

/// Builder for WebSocket configuration
#[derive(Debug)]
pub struct WebSocketConfigBuilder {
    config: WebSocketConfig,
}

impl WebSocketConfigBuilder {
    /// Create a new builder with default values
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: WebSocketConfig::default(),
        }
    }

    /// Set the bind address
    #[must_use]
    pub fn bind_address(mut self, addr: impl Into<String>) -> Self {
        self.config.bind_address = addr.into();
        self
    }

    /// Set TLS configuration
    #[must_use]
    pub fn tls(mut self, tls: TlsConfig) -> Self {
        self.config.tls = Some(tls);
        self
    }

    /// Set maximum message size
    #[must_use]
    pub fn max_message_size(mut self, size: usize) -> Self {
        self.config.max_message_size = size;
        self
    }

    /// Enable or disable compression
    #[must_use]
    pub fn compression(mut self, enabled: bool) -> Self {
        self.config.compression = enabled;
        self
    }

    /// Set heartbeat interval
    #[must_use]
    pub fn heartbeat_interval(mut self, interval: Duration) -> Self {
        self.config.heartbeat_interval_ms = interval.as_millis() as u64;
        self
    }

    /// Set handshake timeout
    #[must_use]
    pub fn handshake_timeout(mut self, timeout: Duration) -> Self {
        self.config.handshake_timeout_ms = timeout.as_millis() as u64;
        self
    }

    /// Set maximum pending connections
    #[must_use]
    pub fn max_pending_connections(mut self, max: u32) -> Self {
        self.config.max_pending_connections = max;
        self
    }

    /// Set security configuration
    #[must_use]
    pub fn security(mut self, security: SecurityConfig) -> Self {
        self.config.security = security;
        self
    }

    /// Set allowed origins (convenience method)
    #[must_use]
    pub fn allowed_origins(mut self, origins: Vec<String>) -> Self {
        self.config.security.allowed_origins = origins;
        self
    }

    /// Enable or disable debug frame logging
    #[must_use]
    pub fn debug_frames(mut self, enabled: bool) -> Self {
        self.config.debug_frames = enabled;
        self
    }

    /// Build the configuration
    #[must_use]
    pub fn build(self) -> WebSocketConfig {
        self.config
    }
}

impl Default for WebSocketConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during configuration validation
#[derive(Clone, Debug)]
pub enum ConfigValidationError {
    /// Invalid bind address
    InvalidBindAddress(String),
    /// TLS configuration error
    TlsError(TlsConfigError),
    /// Invalid message size
    InvalidMessageSize(String),
    /// Invalid timeout value
    InvalidTimeout(String),
}

impl std::fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBindAddress(msg) => write!(f, "Invalid bind address: {msg}"),
            Self::TlsError(err) => write!(f, "TLS configuration error: {err}"),
            Self::InvalidMessageSize(msg) => write!(f, "Invalid message size: {msg}"),
            Self::InvalidTimeout(msg) => write!(f, "Invalid timeout: {msg}"),
        }
    }
}

impl std::error::Error for ConfigValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TlsError(err) => Some(err),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        let config = WebSocketConfig::default();
        assert_eq!(config.bind_address(), "127.0.0.1:8765");
        assert!(config.tls().is_none());
        assert_eq!(config.max_message_size(), 10 * 1024 * 1024);
        assert!(config.compression());
        assert_eq!(config.heartbeat_interval(), Duration::from_secs(30));
        assert_eq!(config.handshake_timeout(), Duration::from_secs(10));
        assert_eq!(config.max_pending_connections(), 128);
    }

    #[test]
    fn test_builder_all_options() {
        let tls = TlsConfig::new(PathBuf::from("/cert.pem"), PathBuf::from("/key.pem"));

        let security = SecurityConfig {
            require_origin_validation: true,
            require_authentication: true,
            ..Default::default()
        };

        let config = WebSocketConfig::builder()
            .bind_address("0.0.0.0:443")
            .tls(tls)
            .max_message_size(5 * 1024 * 1024)
            .compression(false)
            .heartbeat_interval(Duration::from_secs(60))
            .handshake_timeout(Duration::from_secs(5))
            .max_pending_connections(256)
            .security(security)
            .debug_frames(true)
            .build();

        assert_eq!(config.bind_address(), "0.0.0.0:443");
        assert!(config.is_tls_enabled());
        assert_eq!(config.max_message_size(), 5 * 1024 * 1024);
        assert!(!config.compression());
        assert_eq!(config.heartbeat_interval(), Duration::from_secs(60));
        assert_eq!(config.handshake_timeout(), Duration::from_secs(5));
        assert_eq!(config.max_pending_connections(), 256);
        assert!(config.debug_frames());
    }

    #[test]
    fn test_development_config() {
        let config = WebSocketConfig::development();
        assert!(!config.is_tls_enabled());
        assert!(config.debug_frames());
        assert!(!config.security().require_origin_validation);
    }

    #[test]
    fn test_validation_empty_bind_address() {
        let mut config = WebSocketConfig::default();
        config.bind_address = String::new();

        let result = config.validate();
        assert!(matches!(
            result,
            Err(ConfigValidationError::InvalidBindAddress(_))
        ));
    }

    #[test]
    fn test_validation_invalid_bind_address_format() {
        let mut config = WebSocketConfig::default();
        config.bind_address = "localhost".to_string(); // Missing port

        let result = config.validate();
        assert!(matches!(
            result,
            Err(ConfigValidationError::InvalidBindAddress(_))
        ));
    }

    #[test]
    fn test_validation_zero_message_size() {
        let mut config = WebSocketConfig::default();
        config.max_message_size = 0;

        let result = config.validate();
        assert!(matches!(
            result,
            Err(ConfigValidationError::InvalidMessageSize(_))
        ));
    }

    #[test]
    fn test_is_production_safe() {
        // Development config is not production safe
        let dev_config = WebSocketConfig::development();
        assert!(!dev_config.is_production_safe());

        // Config with TLS and security is production safe
        let tls = TlsConfig::new(PathBuf::from("/cert.pem"), PathBuf::from("/key.pem"));

        let config = WebSocketConfig::builder()
            .tls(tls)
            .security(SecurityConfig::default())
            .debug_frames(false)
            .build();

        assert!(config.is_production_safe());
    }

    #[test]
    fn test_tls_config_mtls() {
        let tls = TlsConfig::with_client_auth(
            PathBuf::from("/cert.pem"),
            PathBuf::from("/key.pem"),
            PathBuf::from("/ca.pem"),
        );

        assert!(tls.is_mtls_enabled());
        assert!(tls.ca_path.is_some());
    }

    #[test]
    fn test_tls_config_no_mtls() {
        let tls = TlsConfig::new(PathBuf::from("/cert.pem"), PathBuf::from("/key.pem"));

        assert!(!tls.is_mtls_enabled());
        assert!(tls.ca_path.is_none());
    }

    #[test]
    fn test_tls_config_error_display() {
        let err = TlsConfigError::CertificateNotFound(PathBuf::from("/missing.pem"));
        assert!(err.to_string().contains("missing.pem"));

        let err = TlsConfigError::InvalidKey("bad format".to_string());
        assert!(err.to_string().contains("bad format"));
    }
}
