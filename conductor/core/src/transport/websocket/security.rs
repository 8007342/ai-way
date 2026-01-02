//! WebSocket Security Configuration
//!
//! Security settings for WebSocket transport including origin validation,
//! authentication, and connection policies.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Security configuration for WebSocket connections
///
/// Controls authentication, origin validation, and connection policies
/// to protect against unauthorized access and cross-site attacks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Require origin header validation
    ///
    /// When enabled, connections must include an Origin header that
    /// matches one of the allowed origins. This prevents cross-site
    /// WebSocket hijacking attacks.
    pub require_origin_validation: bool,

    /// Require authentication token in handshake
    ///
    /// When enabled, the Handshake event must include a valid
    /// authentication token.
    pub require_authentication: bool,

    /// Authentication method
    pub authentication_method: AuthenticationMethod,

    /// Allowed origins for cross-origin requests
    ///
    /// List of allowed origin URLs. Only used when
    /// `require_origin_validation` is true.
    ///
    /// Example: `["https://app.example.com", "https://admin.example.com"]`
    pub allowed_origins: Vec<String>,

    /// Maximum connections per origin
    ///
    /// Limits the number of simultaneous connections from the same origin.
    /// Helps prevent resource exhaustion from a single source.
    pub max_connections_per_origin: u32,

    /// Connection timeout in milliseconds
    ///
    /// Maximum time to wait for a new connection to complete handshake.
    pub connection_timeout_ms: u64,

    /// Idle timeout in milliseconds
    ///
    /// Disconnect connections that have been idle for this long.
    /// Set to 0 to disable idle timeout.
    pub idle_timeout_ms: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_origin_validation: true,
            require_authentication: true,
            authentication_method: AuthenticationMethod::BearerToken,
            allowed_origins: Vec::new(),
            max_connections_per_origin: 10,
            connection_timeout_ms: 30_000, // 30 seconds
            idle_timeout_ms: 300_000,      // 5 minutes
        }
    }
}

impl SecurityConfig {
    /// Create a new security configuration with default values
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a development configuration with relaxed security
    ///
    /// - No origin validation
    /// - No authentication required
    /// - Longer timeouts
    #[must_use]
    pub fn development() -> Self {
        Self {
            require_origin_validation: false,
            require_authentication: false,
            authentication_method: AuthenticationMethod::None,
            allowed_origins: Vec::new(),
            max_connections_per_origin: 100,
            connection_timeout_ms: 60_000, // 60 seconds
            idle_timeout_ms: 3_600_000,    // 1 hour
        }
    }

    /// Create a strict security configuration
    ///
    /// - Origin validation required
    /// - Authentication required
    /// - Low connection limits
    /// - Short timeouts
    #[must_use]
    pub fn strict() -> Self {
        Self {
            require_origin_validation: true,
            require_authentication: true,
            authentication_method: AuthenticationMethod::BearerToken,
            allowed_origins: Vec::new(),
            max_connections_per_origin: 5,
            connection_timeout_ms: 10_000, // 10 seconds
            idle_timeout_ms: 60_000,       // 1 minute
        }
    }

    /// Get connection timeout as Duration
    #[must_use]
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_millis(self.connection_timeout_ms)
    }

    /// Get idle timeout as Duration
    #[must_use]
    pub fn idle_timeout(&self) -> Duration {
        Duration::from_millis(self.idle_timeout_ms)
    }

    /// Create an origin policy from this configuration
    #[must_use]
    pub fn origin_policy(&self) -> OriginPolicy {
        if self.require_origin_validation {
            OriginPolicy::new(self.allowed_origins.clone())
        } else {
            OriginPolicy::allow_all()
        }
    }

    /// Check if this configuration is suitable for production
    #[must_use]
    pub fn is_production_safe(&self) -> bool {
        self.require_origin_validation
            && self.require_authentication
            && !matches!(self.authentication_method, AuthenticationMethod::None)
    }
}

/// Authentication methods for WebSocket connections
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthenticationMethod {
    /// No authentication required
    None,
    /// Bearer token in handshake (auth_token field)
    BearerToken,
    /// HTTP Basic authentication in Sec-WebSocket-Protocol header
    Basic,
    /// Custom authentication via protocol subheader
    Custom(String),
}

impl AuthenticationMethod {
    /// Check if authentication is disabled
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

/// Origin validation policy
///
/// Controls which origins are allowed to connect via WebSocket.
#[derive(Clone, Debug)]
pub struct OriginPolicy {
    /// Mode of operation
    mode: OriginPolicyMode,
    /// Allowed origins (only used in AllowList mode)
    allowed: Vec<String>,
}

#[derive(Clone, Debug)]
enum OriginPolicyMode {
    /// Allow all origins (development only)
    AllowAll,
    /// Only allow listed origins
    AllowList,
    /// Deny all origins
    DenyAll,
}

impl OriginPolicy {
    /// Create a policy that allows specific origins
    #[must_use]
    pub fn new(allowed_origins: Vec<String>) -> Self {
        Self {
            mode: OriginPolicyMode::AllowList,
            allowed: allowed_origins,
        }
    }

    /// Create a policy that allows all origins
    ///
    /// **WARNING**: Only use for development!
    #[must_use]
    pub fn allow_all() -> Self {
        Self {
            mode: OriginPolicyMode::AllowAll,
            allowed: Vec::new(),
        }
    }

    /// Create a policy that denies all origins
    #[must_use]
    pub fn deny_all() -> Self {
        Self {
            mode: OriginPolicyMode::DenyAll,
            allowed: Vec::new(),
        }
    }

    /// Add an allowed origin
    pub fn add_origin(&mut self, origin: String) {
        if !self.allowed.contains(&origin) {
            self.allowed.push(origin);
        }
    }

    /// Remove an allowed origin
    pub fn remove_origin(&mut self, origin: &str) {
        self.allowed.retain(|o| o != origin);
    }

    /// Validate an origin against this policy
    #[must_use]
    pub fn validate(&self, origin: &str) -> OriginValidationResult {
        match self.mode {
            OriginPolicyMode::AllowAll => OriginValidationResult::Allowed,
            OriginPolicyMode::DenyAll => OriginValidationResult::Denied {
                reason: "All origins denied".to_string(),
            },
            OriginPolicyMode::AllowList => {
                // Exact match required
                if self.allowed.iter().any(|allowed| allowed == origin) {
                    OriginValidationResult::Allowed
                } else {
                    OriginValidationResult::Denied {
                        reason: format!("Origin '{origin}' not in allow list"),
                    }
                }
            }
        }
    }

    /// Get the list of allowed origins
    #[must_use]
    pub fn allowed_origins(&self) -> &[String] {
        &self.allowed
    }

    /// Check if this policy allows all origins
    #[must_use]
    pub fn is_allow_all(&self) -> bool {
        matches!(self.mode, OriginPolicyMode::AllowAll)
    }
}

/// Result of origin validation
#[derive(Clone, Debug)]
pub enum OriginValidationResult {
    /// Origin is allowed
    Allowed,
    /// Origin is denied
    Denied {
        /// Reason for denial
        reason: String,
    },
}

impl OriginValidationResult {
    /// Check if the origin is allowed
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }

    /// Check if the origin is denied
    #[must_use]
    pub fn is_denied(&self) -> bool {
        matches!(self, Self::Denied { .. })
    }

    /// Get the denial reason if denied
    #[must_use]
    pub fn denial_reason(&self) -> Option<&str> {
        match self {
            Self::Denied { reason } => Some(reason),
            Self::Allowed => None,
        }
    }
}

/// Security-related errors
#[derive(Clone, Debug)]
pub enum SecurityError {
    /// Origin validation failed
    OriginDenied {
        /// The denied origin
        origin: String,
        /// Reason for denial
        reason: String,
    },
    /// Authentication failed
    AuthenticationFailed {
        /// Reason for failure
        reason: String,
    },
    /// Connection limit exceeded
    ConnectionLimitExceeded {
        /// Current connection count
        current: u32,
        /// Maximum allowed
        max: u32,
    },
    /// Connection timed out
    ConnectionTimeout,
    /// Invalid credentials format
    InvalidCredentials(String),
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OriginDenied { origin, reason } => {
                write!(f, "Origin denied: {origin} - {reason}")
            }
            Self::AuthenticationFailed { reason } => {
                write!(f, "Authentication failed: {reason}")
            }
            Self::ConnectionLimitExceeded { current, max } => {
                write!(f, "Connection limit exceeded: {current}/{max}")
            }
            Self::ConnectionTimeout => {
                write!(f, "Connection timed out during handshake")
            }
            Self::InvalidCredentials(msg) => {
                write!(f, "Invalid credentials: {msg}")
            }
        }
    }
}

impl std::error::Error for SecurityError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(config.require_origin_validation);
        assert!(config.require_authentication);
        assert_eq!(config.max_connections_per_origin, 10);
        assert_eq!(config.connection_timeout(), Duration::from_secs(30));
    }

    #[test]
    fn test_security_config_development() {
        let config = SecurityConfig::development();
        assert!(!config.require_origin_validation);
        assert!(!config.require_authentication);
        assert!(matches!(
            config.authentication_method,
            AuthenticationMethod::None
        ));
    }

    #[test]
    fn test_security_config_strict() {
        let config = SecurityConfig::strict();
        assert!(config.require_origin_validation);
        assert!(config.require_authentication);
        assert_eq!(config.max_connections_per_origin, 5);
        assert_eq!(config.connection_timeout(), Duration::from_secs(10));
    }

    #[test]
    fn test_security_config_is_production_safe() {
        assert!(SecurityConfig::default().is_production_safe());
        assert!(SecurityConfig::strict().is_production_safe());
        assert!(!SecurityConfig::development().is_production_safe());
    }

    #[test]
    fn test_origin_policy_allow_list() {
        let policy = OriginPolicy::new(vec![
            "https://app.example.com".to_string(),
            "https://admin.example.com".to_string(),
        ]);

        assert!(policy.validate("https://app.example.com").is_allowed());
        assert!(policy.validate("https://admin.example.com").is_allowed());
        assert!(policy.validate("https://evil.com").is_denied());
        assert!(policy.validate("http://app.example.com").is_denied()); // Different scheme
    }

    #[test]
    fn test_origin_policy_allow_all() {
        let policy = OriginPolicy::allow_all();
        assert!(policy.is_allow_all());
        assert!(policy.validate("https://any.domain.com").is_allowed());
        assert!(policy.validate("http://localhost:3000").is_allowed());
    }

    #[test]
    fn test_origin_policy_deny_all() {
        let policy = OriginPolicy::deny_all();
        assert!(policy.validate("https://app.example.com").is_denied());
        assert!(policy.validate("http://localhost").is_denied());
    }

    #[test]
    fn test_origin_policy_add_remove() {
        let mut policy = OriginPolicy::new(vec!["https://one.com".to_string()]);

        assert!(policy.validate("https://one.com").is_allowed());
        assert!(policy.validate("https://two.com").is_denied());

        policy.add_origin("https://two.com".to_string());
        assert!(policy.validate("https://two.com").is_allowed());

        policy.remove_origin("https://one.com");
        assert!(policy.validate("https://one.com").is_denied());
    }

    #[test]
    fn test_origin_validation_result() {
        let allowed = OriginValidationResult::Allowed;
        assert!(allowed.is_allowed());
        assert!(!allowed.is_denied());
        assert!(allowed.denial_reason().is_none());

        let denied = OriginValidationResult::Denied {
            reason: "test reason".to_string(),
        };
        assert!(!denied.is_allowed());
        assert!(denied.is_denied());
        assert_eq!(denied.denial_reason(), Some("test reason"));
    }

    #[test]
    fn test_authentication_method() {
        assert!(AuthenticationMethod::None.is_none());
        assert!(!AuthenticationMethod::BearerToken.is_none());
        assert!(!AuthenticationMethod::Basic.is_none());
        assert!(!AuthenticationMethod::Custom("jwt".to_string()).is_none());
    }

    #[test]
    fn test_security_error_display() {
        let err = SecurityError::OriginDenied {
            origin: "https://evil.com".to_string(),
            reason: "not in allow list".to_string(),
        };
        assert!(err.to_string().contains("evil.com"));

        let err = SecurityError::AuthenticationFailed {
            reason: "invalid token".to_string(),
        };
        assert!(err.to_string().contains("invalid token"));

        let err = SecurityError::ConnectionLimitExceeded {
            current: 10,
            max: 10,
        };
        assert!(err.to_string().contains("10/10"));

        let err = SecurityError::ConnectionTimeout;
        assert!(err.to_string().contains("timed out"));
    }

    #[test]
    fn test_security_config_origin_policy() {
        let mut config = SecurityConfig::default();
        config.allowed_origins = vec!["https://app.com".to_string()];

        let policy = config.origin_policy();
        assert!(policy.validate("https://app.com").is_allowed());
        assert!(policy.validate("https://other.com").is_denied());

        // Development config should allow all
        let dev_config = SecurityConfig::development();
        let dev_policy = dev_config.origin_policy();
        assert!(dev_policy.is_allow_all());
    }

    #[test]
    fn test_serialization() {
        let config = SecurityConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: SecurityConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(
            config.require_origin_validation,
            deserialized.require_origin_validation
        );
        assert_eq!(
            config.require_authentication,
            deserialized.require_authentication
        );
    }

    #[test]
    fn test_authentication_method_serialization() {
        let methods = vec![
            AuthenticationMethod::None,
            AuthenticationMethod::BearerToken,
            AuthenticationMethod::Basic,
            AuthenticationMethod::Custom("jwt".to_string()),
        ];

        for method in methods {
            let json = serde_json::to_string(&method).unwrap();
            let deserialized: AuthenticationMethod = serde_json::from_str(&json).unwrap();
            assert_eq!(method, deserialized);
        }
    }
}
