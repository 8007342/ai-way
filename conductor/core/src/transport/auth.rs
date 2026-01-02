//! Session Token Authentication for Conductor-Surface IPC
//!
//! This module provides session-based authentication to prevent unauthorized
//! connections to the Conductor daemon. Each daemon session generates a unique
//! token that surfaces must present during handshake.
//!
//! # Security Model
//!
//! - Token is 32 bytes of cryptographically random data
//! - Stored as base64 in `$XDG_RUNTIME_DIR/ai-way/session.token`
//! - Token file has 0o600 permissions (owner read/write only)
//! - Token is regenerated on each daemon restart
//! - Constant-time comparison prevents timing attacks
//!
//! # Usage
//!
//! ## Daemon (Server) Side
//!
//! ```ignore
//! use conductor_core::transport::auth::{SessionToken, get_token_path};
//!
//! // Generate and write token on daemon start
//! let token = SessionToken::generate();
//! let path = get_token_path()?;
//! token.write_to_file(&path)?;
//!
//! // During handshake validation
//! if !token.validate(provided_token) {
//!     // Reject connection
//! }
//! ```
//!
//! ## Surface (Client) Side
//!
//! ```ignore
//! use conductor_core::transport::auth::{SessionToken, get_token_path};
//!
//! // Read token before connecting
//! let path = get_token_path()?;
//! let token = SessionToken::read_from_file(&path)?;
//!
//! // Include in handshake
//! let handshake = SurfaceEvent::Handshake {
//!     auth_token: Some(token.to_base64()),
//!     // ...
//! };
//! ```

use std::fmt;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use rand::RngCore;
use thiserror::Error;

/// Size of the session token in bytes
pub const TOKEN_SIZE: usize = 32;

/// Token file name within the runtime directory
pub const TOKEN_FILENAME: &str = "session.token";

/// Runtime directory name
pub const RUNTIME_DIR_NAME: &str = "ai-way";

/// Errors related to session token operations
#[derive(Debug, Error)]
pub enum TokenError {
    /// Failed to generate random token
    #[error("failed to generate random token: {0}")]
    GenerationFailed(String),

    /// Failed to write token file
    #[error("failed to write token file: {0}")]
    WriteFailed(#[from] std::io::Error),

    /// Failed to read token file
    #[error("failed to read token file: {0}")]
    ReadFailed(String),

    /// Token file not found
    #[error("token file not found at {0}")]
    NotFound(PathBuf),

    /// Invalid token format
    #[error("invalid token format: {0}")]
    InvalidFormat(String),

    /// Failed to decode base64
    #[error("failed to decode base64: {0}")]
    Base64DecodeFailed(String),

    /// Runtime directory not available
    #[error("runtime directory not available: XDG_RUNTIME_DIR not set")]
    NoRuntimeDir,

    /// Failed to create directory
    #[error("failed to create directory: {0}")]
    DirectoryCreationFailed(String),

    /// Token validation failed
    #[error("token validation failed: {reason}")]
    ValidationFailed {
        /// Reason for validation failure
        reason: String,
    },
}

/// Session token for authenticating surface connections
///
/// A 32-byte cryptographically random token that is:
/// - Generated on daemon startup
/// - Written to a secure file in `$XDG_RUNTIME_DIR/ai-way/`
/// - Read by surfaces before connecting
/// - Validated during handshake using constant-time comparison
#[derive(Clone)]
pub struct SessionToken {
    /// Raw token bytes
    bytes: [u8; TOKEN_SIZE],
}

impl SessionToken {
    /// Generate a new random session token
    ///
    /// Uses the system's cryptographically secure random number generator.
    ///
    /// # Example
    ///
    /// ```
    /// use conductor_core::transport::auth::SessionToken;
    ///
    /// let token = SessionToken::generate();
    /// assert_eq!(token.as_bytes().len(), 32);
    /// ```
    #[must_use]
    pub fn generate() -> Self {
        let mut bytes = [0u8; TOKEN_SIZE];
        rand::thread_rng().fill_bytes(&mut bytes);
        Self { bytes }
    }

    /// Create a token from raw bytes
    ///
    /// # Errors
    ///
    /// Returns an error if the byte slice is not exactly `TOKEN_SIZE` bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, TokenError> {
        if bytes.len() != TOKEN_SIZE {
            return Err(TokenError::InvalidFormat(format!(
                "expected {} bytes, got {}",
                TOKEN_SIZE,
                bytes.len()
            )));
        }
        let mut token_bytes = [0u8; TOKEN_SIZE];
        token_bytes.copy_from_slice(bytes);
        Ok(Self { bytes: token_bytes })
    }

    /// Create a token from a base64-encoded string
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not valid base64 or doesn't decode
    /// to exactly `TOKEN_SIZE` bytes.
    pub fn from_base64(encoded: &str) -> Result<Self, TokenError> {
        let decoded = base64_decode(encoded.trim())?;
        Self::from_bytes(&decoded)
    }

    /// Get the raw token bytes
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; TOKEN_SIZE] {
        &self.bytes
    }

    /// Encode the token as base64
    ///
    /// # Example
    ///
    /// ```
    /// use conductor_core::transport::auth::SessionToken;
    ///
    /// let token = SessionToken::generate();
    /// let encoded = token.to_base64();
    /// assert_eq!(encoded.len(), 44); // 32 bytes -> 44 base64 chars
    /// ```
    #[must_use]
    pub fn to_base64(&self) -> String {
        base64_encode(&self.bytes)
    }

    /// Write the token to a file with secure permissions
    ///
    /// The file is created with 0o600 permissions (owner read/write only).
    /// If the parent directory doesn't exist, it will be created with 0o700 permissions.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created or written.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use conductor_core::transport::auth::SessionToken;
    /// use std::path::Path;
    ///
    /// let token = SessionToken::generate();
    /// token.write_to_file(Path::new("/run/user/1000/ai-way/session.token"))?;
    /// ```
    pub fn write_to_file(&self, path: &Path) -> Result<(), TokenError> {
        // Ensure parent directory exists with secure permissions
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| {
                    TokenError::DirectoryCreationFailed(format!("{}: {}", parent.display(), e))
                })?;
                // Set directory permissions to 0o700
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = fs::Permissions::from_mode(0o700);
                    fs::set_permissions(parent, perms)?;
                }
            }
        }

        // Write token to file
        let encoded = self.to_base64();
        let mut file = File::create(path)?;

        // Set file permissions to 0o600 before writing
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            file.set_permissions(perms)?;
        }

        file.write_all(encoded.as_bytes())?;
        file.write_all(b"\n")?; // Add newline for readability
        file.sync_all()?;

        tracing::debug!(path = %path.display(), "Session token written to file");
        Ok(())
    }

    /// Read a token from a file
    ///
    /// # Errors
    ///
    /// Returns an error if the file doesn't exist, can't be read, or contains invalid data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use conductor_core::transport::auth::SessionToken;
    /// use std::path::Path;
    ///
    /// let token = SessionToken::read_from_file(Path::new("/run/user/1000/ai-way/session.token"))?;
    /// ```
    pub fn read_from_file(path: &Path) -> Result<Self, TokenError> {
        if !path.exists() {
            return Err(TokenError::NotFound(path.to_path_buf()));
        }

        let mut file = File::open(path)
            .map_err(|e| TokenError::ReadFailed(format!("{}: {}", path.display(), e)))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| TokenError::ReadFailed(format!("{}: {}", path.display(), e)))?;

        Self::from_base64(&contents)
    }

    /// Validate a provided token against this token using constant-time comparison
    ///
    /// This prevents timing attacks where an attacker could guess the token
    /// byte-by-byte by measuring response times.
    ///
    /// # Example
    ///
    /// ```
    /// use conductor_core::transport::auth::SessionToken;
    ///
    /// let expected = SessionToken::generate();
    /// let provided = expected.to_base64();
    ///
    /// assert!(expected.validate(&provided));
    /// assert!(!expected.validate("invalid_token"));
    /// ```
    #[must_use]
    pub fn validate(&self, provided: &str) -> bool {
        // Try to decode the provided token
        let provided_bytes = match Self::from_base64(provided) {
            Ok(token) => token,
            Err(_) => return false,
        };

        // Constant-time comparison
        constant_time_compare(&self.bytes, &provided_bytes.bytes)
    }

    /// Validate a provided token and return a detailed result
    ///
    /// Unlike `validate()`, this method returns an error with details about
    /// why validation failed, useful for logging and debugging.
    ///
    /// # Errors
    ///
    /// Returns an error describing why validation failed.
    pub fn validate_detailed(&self, provided: &str) -> Result<(), TokenError> {
        // Try to decode the provided token
        let provided_bytes =
            Self::from_base64(provided).map_err(|e| TokenError::ValidationFailed {
                reason: format!("invalid token format: {e}"),
            })?;

        // Constant-time comparison
        if constant_time_compare(&self.bytes, &provided_bytes.bytes) {
            Ok(())
        } else {
            Err(TokenError::ValidationFailed {
                reason: "token mismatch".to_string(),
            })
        }
    }
}

impl fmt::Debug for SessionToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Don't expose the actual token in debug output
        write!(f, "SessionToken([REDACTED])")
    }
}

impl PartialEq for SessionToken {
    fn eq(&self, other: &Self) -> bool {
        // Use constant-time comparison even for PartialEq
        constant_time_compare(&self.bytes, &other.bytes)
    }
}

impl Eq for SessionToken {}

/// Get the default token file path
///
/// Returns `$XDG_RUNTIME_DIR/ai-way/session.token` on Unix systems.
///
/// # Errors
///
/// Returns an error if `XDG_RUNTIME_DIR` is not set.
///
/// # Example
///
/// ```ignore
/// use conductor_core::transport::auth::get_token_path;
///
/// let path = get_token_path()?;
/// // e.g., /run/user/1000/ai-way/session.token
/// ```
pub fn get_token_path() -> Result<PathBuf, TokenError> {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").map_err(|_| TokenError::NoRuntimeDir)?;

    Ok(PathBuf::from(runtime_dir)
        .join(RUNTIME_DIR_NAME)
        .join(TOKEN_FILENAME))
}

/// Get the runtime directory path for ai-way
///
/// Returns `$XDG_RUNTIME_DIR/ai-way`.
///
/// # Errors
///
/// Returns an error if `XDG_RUNTIME_DIR` is not set.
pub fn get_runtime_dir() -> Result<PathBuf, TokenError> {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").map_err(|_| TokenError::NoRuntimeDir)?;

    Ok(PathBuf::from(runtime_dir).join(RUNTIME_DIR_NAME))
}

/// Perform constant-time comparison of two byte slices
///
/// This function takes the same amount of time regardless of where
/// the first difference is found, preventing timing attacks.
fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    // XOR all bytes and accumulate - any difference will show up
    let mut result: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }

    result == 0
}

/// Base64 encoding (using standard alphabet)
fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();
    let chunks = bytes.chunks(3);

    for chunk in chunks {
        let mut n: u32 = 0;
        for (i, &byte) in chunk.iter().enumerate() {
            n |= (byte as u32) << (16 - i * 8);
        }

        let char_count = chunk.len() + 1;
        for i in 0..char_count {
            let idx = ((n >> (18 - i * 6)) & 0x3F) as usize;
            result.push(ALPHABET[idx] as char);
        }

        // Padding
        for _ in char_count..4 {
            result.push('=');
        }
    }

    result
}

/// Base64 decoding
fn base64_decode(encoded: &str) -> Result<Vec<u8>, TokenError> {
    let encoded = encoded.trim();

    if encoded.is_empty() {
        return Err(TokenError::Base64DecodeFailed("empty input".to_string()));
    }

    // Build reverse lookup table
    fn char_to_value(c: char) -> Result<u8, TokenError> {
        match c {
            'A'..='Z' => Ok(c as u8 - b'A'),
            'a'..='z' => Ok(c as u8 - b'a' + 26),
            '0'..='9' => Ok(c as u8 - b'0' + 52),
            '+' => Ok(62),
            '/' => Ok(63),
            '=' => Ok(0), // Padding
            _ => Err(TokenError::Base64DecodeFailed(format!(
                "invalid character: {c}"
            ))),
        }
    }

    let mut result = Vec::new();
    let chars: Vec<char> = encoded.chars().collect();

    if !chars.len().is_multiple_of(4) {
        return Err(TokenError::Base64DecodeFailed(
            "length not multiple of 4".to_string(),
        ));
    }

    for chunk in chars.chunks(4) {
        let a = char_to_value(chunk[0])?;
        let b = char_to_value(chunk[1])?;
        let c = char_to_value(chunk[2])?;
        let d = char_to_value(chunk[3])?;

        let n = ((a as u32) << 18) | ((b as u32) << 12) | ((c as u32) << 6) | (d as u32);

        result.push((n >> 16) as u8);
        if chunk[2] != '=' {
            result.push((n >> 8) as u8);
        }
        if chunk[3] != '=' {
            result.push(n as u8);
        }
    }

    Ok(result)
}

/// Remove the session token file
///
/// Should be called on daemon shutdown for cleanup.
///
/// # Errors
///
/// Returns an error if the file exists but cannot be removed.
pub fn remove_token_file(path: &Path) -> Result<(), TokenError> {
    if path.exists() {
        fs::remove_file(path)?;
        tracing::debug!(path = %path.display(), "Session token file removed");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_token_generation_is_random() {
        let token1 = SessionToken::generate();
        let token2 = SessionToken::generate();

        // Tokens should be different (with overwhelming probability)
        assert_ne!(token1.as_bytes(), token2.as_bytes());
    }

    #[test]
    fn test_token_size() {
        let token = SessionToken::generate();
        assert_eq!(token.as_bytes().len(), TOKEN_SIZE);
    }

    #[test]
    fn test_base64_roundtrip() {
        let token = SessionToken::generate();
        let encoded = token.to_base64();
        let decoded = SessionToken::from_base64(&encoded).unwrap();

        assert_eq!(token.as_bytes(), decoded.as_bytes());
    }

    #[test]
    fn test_base64_encode_length() {
        let token = SessionToken::generate();
        let encoded = token.to_base64();

        // 32 bytes -> 44 base64 characters (ceiling(32 * 4 / 3))
        assert_eq!(encoded.len(), 44);
    }

    #[test]
    fn test_file_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let token_path = temp_dir.path().join("session.token");

        let original = SessionToken::generate();
        original.write_to_file(&token_path).unwrap();

        let loaded = SessionToken::read_from_file(&token_path).unwrap();

        assert_eq!(original.as_bytes(), loaded.as_bytes());
    }

    #[test]
    #[cfg(unix)]
    fn test_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let token_path = temp_dir.path().join("session.token");

        let token = SessionToken::generate();
        token.write_to_file(&token_path).unwrap();

        let metadata = fs::metadata(&token_path).unwrap();
        let mode = metadata.permissions().mode();

        // Check that file is 0o600 (owner read/write only)
        // Mode includes file type bits, so we mask to get just permission bits
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn test_validate_correct_token() {
        let expected = SessionToken::generate();
        let provided = expected.to_base64();

        assert!(expected.validate(&provided));
    }

    #[test]
    fn test_validate_wrong_token() {
        let expected = SessionToken::generate();
        let wrong = SessionToken::generate();

        assert!(!expected.validate(&wrong.to_base64()));
    }

    #[test]
    fn test_validate_invalid_base64() {
        let expected = SessionToken::generate();

        assert!(!expected.validate("not-valid-base64!!!"));
    }

    #[test]
    fn test_validate_wrong_length() {
        let expected = SessionToken::generate();

        // Valid base64 but wrong length
        assert!(!expected.validate("AAAA")); // 3 bytes
    }

    #[test]
    fn test_validate_detailed_correct() {
        let expected = SessionToken::generate();
        let provided = expected.to_base64();

        assert!(expected.validate_detailed(&provided).is_ok());
    }

    #[test]
    fn test_validate_detailed_wrong() {
        let expected = SessionToken::generate();
        let wrong = SessionToken::generate();

        let result = expected.validate_detailed(&wrong.to_base64());
        assert!(result.is_err());

        if let Err(TokenError::ValidationFailed { reason }) = result {
            assert_eq!(reason, "token mismatch");
        } else {
            panic!("Expected ValidationFailed error");
        }
    }

    #[test]
    fn test_constant_time_compare() {
        let a = [1u8, 2, 3, 4];
        let b = [1u8, 2, 3, 4];
        let c = [1u8, 2, 3, 5];
        let d = [1u8, 2, 3];

        assert!(constant_time_compare(&a, &b));
        assert!(!constant_time_compare(&a, &c));
        assert!(!constant_time_compare(&a, &d)); // Different lengths
    }

    #[test]
    fn test_from_bytes_correct_length() {
        let bytes = [0u8; TOKEN_SIZE];
        let token = SessionToken::from_bytes(&bytes);
        assert!(token.is_ok());
    }

    #[test]
    fn test_from_bytes_wrong_length() {
        let bytes = [0u8; 16];
        let token = SessionToken::from_bytes(&bytes);
        assert!(token.is_err());
    }

    #[test]
    fn test_debug_redacts_token() {
        let token = SessionToken::generate();
        let debug = format!("{:?}", token);

        // Should not contain actual token bytes
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains(&format!("{:?}", token.as_bytes())));
    }

    #[test]
    fn test_read_nonexistent_file() {
        let result = SessionToken::read_from_file(Path::new("/nonexistent/path/token"));
        assert!(matches!(result, Err(TokenError::NotFound(_))));
    }

    #[test]
    fn test_remove_token_file() {
        let temp_dir = TempDir::new().unwrap();
        let token_path = temp_dir.path().join("session.token");

        // Write a token
        let token = SessionToken::generate();
        token.write_to_file(&token_path).unwrap();
        assert!(token_path.exists());

        // Remove it
        remove_token_file(&token_path).unwrap();
        assert!(!token_path.exists());
    }

    #[test]
    fn test_remove_nonexistent_file_ok() {
        // Removing a file that doesn't exist should be OK
        let result = remove_token_file(Path::new("/nonexistent/file"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_creates_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let token_path = temp_dir
            .path()
            .join("nested")
            .join("dir")
            .join("session.token");

        let token = SessionToken::generate();
        token.write_to_file(&token_path).unwrap();

        assert!(token_path.exists());
    }

    #[test]
    #[cfg(unix)]
    fn test_parent_directory_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("ai-way-test");
        let token_path = nested_dir.join("session.token");

        let token = SessionToken::generate();
        token.write_to_file(&token_path).unwrap();

        let metadata = fs::metadata(&nested_dir).unwrap();
        let mode = metadata.permissions().mode();

        // Directory should be 0o700 (owner only)
        assert_eq!(mode & 0o777, 0o700);
    }

    #[test]
    fn test_token_equality() {
        let bytes = [42u8; TOKEN_SIZE];
        let token1 = SessionToken::from_bytes(&bytes).unwrap();
        let token2 = SessionToken::from_bytes(&bytes).unwrap();

        assert_eq!(token1, token2);
    }

    #[test]
    fn test_token_inequality() {
        let token1 = SessionToken::generate();
        let token2 = SessionToken::generate();

        assert_ne!(token1, token2);
    }

    #[test]
    fn test_base64_with_whitespace() {
        let token = SessionToken::generate();
        let encoded = format!("  {} \n", token.to_base64());

        let decoded = SessionToken::from_base64(&encoded).unwrap();
        assert_eq!(token, decoded);
    }
}
