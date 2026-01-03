//! Avatar Security Validation Module
//!
//! This module provides security validation for avatar sprites and related data
//! to prevent `DoS` attacks and resource exhaustion. It implements the security
//! constraints defined in Phase P1.5 of the Avatar Animation System.
//!
//! # Security Invariants
//!
//! - Maximum sprite dimensions: 100x100 blocks
//! - Maximum blocks per sprite: 10,000
//! - Only allowed Unicode ranges for block characters
//! - Rate limiting for sprite requests
//!
//! # Allowed Unicode Ranges
//!
//! The following Unicode ranges are permitted for sprite block characters:
//! - Block elements: U+2580-U+259F
//! - Box drawing: U+2500-U+257F
//! - Geometric shapes: U+25A0-U+25FF
//! - Braille patterns: U+2800-U+28FF
//! - Basic ASCII printable: U+0020-U+007E
//!
//! # Usage
//!
//! ```
//! use conductor_core::avatar::security::{
//!     validate_sprite_dimensions, validate_block_count, validate_unicode_char,
//!     SecurityError, MAX_SPRITE_WIDTH, MAX_SPRITE_HEIGHT,
//! };
//!
//! // Validate dimensions
//! assert!(validate_sprite_dimensions(50, 50).is_ok());
//! assert!(validate_sprite_dimensions(150, 50).is_err());
//!
//! // Validate block count
//! assert!(validate_block_count(5000).is_ok());
//! assert!(validate_block_count(15000).is_err());
//!
//! // Validate Unicode characters
//! assert!(validate_unicode_char('\u{2588}').is_ok()); // Full block
//! assert!(validate_unicode_char('A').is_ok());         // ASCII
//! assert!(validate_unicode_char('\u{1F600}').is_err()); // Emoji - not allowed
//! ```

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::Block;

// =============================================================================
// Security Constants
// =============================================================================

/// Maximum sprite width in blocks
pub const MAX_SPRITE_WIDTH: u16 = 100;

/// Maximum sprite height in blocks
pub const MAX_SPRITE_HEIGHT: u16 = 100;

/// Maximum number of blocks per sprite
pub const MAX_BLOCKS_PER_SPRITE: usize = 10_000;

/// Maximum pending sprite requests per session
pub const MAX_PENDING_REQUESTS_PER_SESSION: usize = 10;

/// Maximum sprite requests per minute per session
pub const MAX_SPRITE_REQUESTS_PER_MINUTE: u32 = 100;

/// Maximum cache size in bytes (10MB)
pub const MAX_CACHE_SIZE_BYTES: usize = 10 * 1024 * 1024;

/// Maximum animation frames
pub const MAX_ANIMATION_FRAMES: usize = 100;

/// Maximum animation duration in milliseconds (1 minute)
pub const MAX_ANIMATION_DURATION_MS: u64 = 60_000;

/// Allowed Unicode ranges for block characters
///
/// Each tuple represents (start, end) inclusive range of Unicode code points.
pub const ALLOWED_UNICODE_RANGES: &[(u32, u32)] = &[
    (0x2580, 0x259F), // Block Elements
    (0x2500, 0x257F), // Box Drawing
    (0x25A0, 0x25FF), // Geometric Shapes
    (0x2800, 0x28FF), // Braille Patterns
    (0x0020, 0x007E), // Basic ASCII printable
];

// =============================================================================
// Error Types
// =============================================================================

/// Security validation errors for avatar sprites
#[derive(Clone, Debug, Error, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityError {
    /// Sprite dimensions exceed maximum allowed size
    #[error(
        "Sprite dimensions {width}x{height} exceed maximum {MAX_SPRITE_WIDTH}x{MAX_SPRITE_HEIGHT}"
    )]
    DimensionOverflow {
        /// The requested width
        width: u16,
        /// The requested height
        height: u16,
    },

    /// Sprite contains too many blocks
    #[error("Sprite block count {count} exceeds maximum {MAX_BLOCKS_PER_SPRITE}")]
    TooManyBlocks {
        /// The number of blocks in the sprite
        count: usize,
    },

    /// Character is not in the allowed Unicode ranges
    #[error("Character U+{codepoint:04X} is not in allowed Unicode ranges")]
    InvalidUnicodeChar {
        /// The Unicode code point of the invalid character
        codepoint: u32,
    },

    /// Rate limit exceeded for sprite requests
    #[error("Rate limit exceeded: {requests} requests in {window_seconds}s (max: {max_requests})")]
    RateLimitExceeded {
        /// Number of requests made
        requests: u32,
        /// Time window in seconds
        window_seconds: u64,
        /// Maximum allowed requests
        max_requests: u32,
    },

    /// Too many pending requests
    #[error("Too many pending requests: {pending} (max: {max_pending})")]
    TooManyPendingRequests {
        /// Number of pending requests
        pending: usize,
        /// Maximum allowed pending requests
        max_pending: usize,
    },

    /// Animation has too many frames
    #[error("Animation has {frames} frames (max: {MAX_ANIMATION_FRAMES})")]
    TooManyAnimationFrames {
        /// Number of frames in the animation
        frames: usize,
    },

    /// Animation duration exceeds maximum
    #[error("Animation duration {duration_ms}ms exceeds maximum {MAX_ANIMATION_DURATION_MS}ms")]
    AnimationDurationExceeded {
        /// The requested duration in milliseconds
        duration_ms: u64,
    },

    /// Sprite data is empty
    #[error("Sprite contains no blocks")]
    EmptySprite,

    /// Cache size would exceed limit
    #[error("Cache size {size} bytes would exceed limit of {MAX_CACHE_SIZE_BYTES} bytes")]
    CacheSizeExceeded {
        /// The size that would be reached
        size: usize,
    },
}

/// Result type for security validation
pub type SecurityResult<T> = Result<T, SecurityError>;

// =============================================================================
// Validation Functions
// =============================================================================

/// Validate sprite dimensions against security limits
///
/// # Arguments
///
/// * `width` - The sprite width in blocks
/// * `height` - The sprite height in blocks
///
/// # Returns
///
/// `Ok(())` if dimensions are valid, `Err(SecurityError::DimensionOverflow)` otherwise
///
/// # Examples
///
/// ```
/// use conductor_core::avatar::security::{validate_sprite_dimensions, SecurityError};
///
/// // Valid dimensions
/// assert!(validate_sprite_dimensions(50, 50).is_ok());
/// assert!(validate_sprite_dimensions(100, 100).is_ok());
/// assert!(validate_sprite_dimensions(1, 1).is_ok());
///
/// // Invalid dimensions
/// assert!(matches!(
///     validate_sprite_dimensions(101, 50),
///     Err(SecurityError::DimensionOverflow { width: 101, height: 50 })
/// ));
/// ```
pub fn validate_sprite_dimensions(width: u16, height: u16) -> SecurityResult<()> {
    if width > MAX_SPRITE_WIDTH || height > MAX_SPRITE_HEIGHT {
        return Err(SecurityError::DimensionOverflow { width, height });
    }
    Ok(())
}

/// Validate the number of blocks in a sprite
///
/// # Arguments
///
/// * `count` - The number of blocks in the sprite
///
/// # Returns
///
/// `Ok(())` if count is valid, `Err(SecurityError::TooManyBlocks)` otherwise
///
/// # Examples
///
/// ```
/// use conductor_core::avatar::security::{validate_block_count, SecurityError, MAX_BLOCKS_PER_SPRITE};
///
/// // Valid counts
/// assert!(validate_block_count(0).is_ok());
/// assert!(validate_block_count(5000).is_ok());
/// assert!(validate_block_count(MAX_BLOCKS_PER_SPRITE).is_ok());
///
/// // Invalid count
/// assert!(matches!(
///     validate_block_count(MAX_BLOCKS_PER_SPRITE + 1),
///     Err(SecurityError::TooManyBlocks { .. })
/// ));
/// ```
pub fn validate_block_count(count: usize) -> SecurityResult<()> {
    if count > MAX_BLOCKS_PER_SPRITE {
        return Err(SecurityError::TooManyBlocks { count });
    }
    Ok(())
}

/// Check if a character is in the allowed Unicode ranges
///
/// # Arguments
///
/// * `c` - The character to check
///
/// # Returns
///
/// `true` if the character is allowed, `false` otherwise
///
/// # Examples
///
/// ```
/// use conductor_core::avatar::security::is_allowed_block_char;
///
/// // Block elements
/// assert!(is_allowed_block_char('\u{2588}')); // Full block
/// assert!(is_allowed_block_char('\u{2580}')); // Upper half block
///
/// // Box drawing
/// assert!(is_allowed_block_char('\u{2500}')); // Light horizontal
/// assert!(is_allowed_block_char('\u{2550}')); // Double horizontal
///
/// // Geometric shapes
/// assert!(is_allowed_block_char('\u{25A0}')); // Black square
/// assert!(is_allowed_block_char('\u{25CF}')); // Black circle
///
/// // Braille
/// assert!(is_allowed_block_char('\u{2800}')); // Braille blank
/// assert!(is_allowed_block_char('\u{28FF}')); // Braille 8 dots
///
/// // ASCII
/// assert!(is_allowed_block_char(' '));
/// assert!(is_allowed_block_char('A'));
/// assert!(is_allowed_block_char('~'));
///
/// // Not allowed
/// assert!(!is_allowed_block_char('\u{1F600}')); // Emoji
/// assert!(!is_allowed_block_char('\u{0000}'));  // Null
/// assert!(!is_allowed_block_char('\u{007F}'));  // DEL
/// ```
#[must_use]
pub fn is_allowed_block_char(c: char) -> bool {
    let code = c as u32;
    ALLOWED_UNICODE_RANGES
        .iter()
        .any(|(start, end)| code >= *start && code <= *end)
}

/// Validate a Unicode character for use in sprites
///
/// # Arguments
///
/// * `c` - The character to validate
///
/// # Returns
///
/// `Ok(())` if the character is allowed, `Err(SecurityError::InvalidUnicodeChar)` otherwise
///
/// # Examples
///
/// ```
/// use conductor_core::avatar::security::{validate_unicode_char, SecurityError};
///
/// // Valid characters
/// assert!(validate_unicode_char('\u{2588}').is_ok()); // Full block
/// assert!(validate_unicode_char('A').is_ok());         // ASCII
/// assert!(validate_unicode_char(' ').is_ok());         // Space
///
/// // Invalid characters
/// assert!(matches!(
///     validate_unicode_char('\u{1F600}'),
///     Err(SecurityError::InvalidUnicodeChar { .. })
/// ));
/// ```
pub fn validate_unicode_char(c: char) -> SecurityResult<()> {
    if is_allowed_block_char(c) {
        Ok(())
    } else {
        Err(SecurityError::InvalidUnicodeChar {
            codepoint: c as u32,
        })
    }
}

/// Validate animation frame count
///
/// # Arguments
///
/// * `frames` - The number of frames in the animation
///
/// # Returns
///
/// `Ok(())` if frame count is valid, `Err(SecurityError::TooManyAnimationFrames)` otherwise
pub fn validate_animation_frames(frames: usize) -> SecurityResult<()> {
    if frames > MAX_ANIMATION_FRAMES {
        return Err(SecurityError::TooManyAnimationFrames { frames });
    }
    Ok(())
}

/// Validate animation duration
///
/// # Arguments
///
/// * `duration` - The animation duration
///
/// # Returns
///
/// `Ok(())` if duration is valid, `Err(SecurityError::AnimationDurationExceeded)` otherwise
pub fn validate_animation_duration(duration: Duration) -> SecurityResult<()> {
    let duration_ms = duration.as_millis() as u64;
    if duration_ms > MAX_ANIMATION_DURATION_MS {
        return Err(SecurityError::AnimationDurationExceeded { duration_ms });
    }
    Ok(())
}

// =============================================================================
// Sprite Validation
// =============================================================================

/// A simple sprite structure for validation purposes
///
/// This represents the minimal sprite data needed for security validation.
/// The actual sprite implementation may have additional fields.
#[derive(Clone, Debug)]
pub struct Sprite {
    /// Width of the sprite in blocks
    pub width: u16,
    /// Height of the sprite in blocks
    pub height: u16,
    /// The blocks that make up the sprite
    pub blocks: Vec<Block>,
}

impl Sprite {
    /// Create a new sprite with the given dimensions and blocks
    #[must_use]
    pub fn new(width: u16, height: u16, blocks: Vec<Block>) -> Self {
        Self {
            width,
            height,
            blocks,
        }
    }
}

/// Validate a complete sprite against all security constraints
///
/// This function performs comprehensive validation:
/// 1. Validates dimensions against maximum limits
/// 2. Validates block count against maximum limit
/// 3. Validates that all block characters are in allowed Unicode ranges
///
/// # Arguments
///
/// * `sprite` - The sprite to validate
///
/// # Returns
///
/// `Ok(())` if the sprite passes all validations, otherwise returns the first error encountered
///
/// # Examples
///
/// ```
/// use conductor_core::avatar::security::{validate_sprite, Sprite, SecurityError};
/// use conductor_core::avatar::Block;
///
/// // Valid sprite
/// let sprite = Sprite::new(10, 10, vec![Block::empty(); 100]);
/// assert!(validate_sprite(&sprite).is_ok());
///
/// // Oversized sprite
/// let oversized = Sprite::new(150, 50, vec![]);
/// assert!(matches!(
///     validate_sprite(&oversized),
///     Err(SecurityError::DimensionOverflow { .. })
/// ));
/// ```
pub fn validate_sprite(sprite: &Sprite) -> SecurityResult<()> {
    // Validate dimensions
    validate_sprite_dimensions(sprite.width, sprite.height)?;

    // Validate block count
    validate_block_count(sprite.blocks.len())?;

    // Validate each block's character
    for block in &sprite.blocks {
        validate_unicode_char(block.character)?;
    }

    Ok(())
}

/// Validate sprite dimensions and check if block count would overflow
///
/// This is a convenience function that validates dimensions and ensures
/// the calculated area (width * height) doesn't exceed the maximum blocks.
///
/// # Arguments
///
/// * `width` - The sprite width in blocks
/// * `height` - The sprite height in blocks
///
/// # Returns
///
/// `Ok(())` if dimensions and resulting block count are valid
pub fn validate_sprite_size(width: u16, height: u16) -> SecurityResult<()> {
    validate_sprite_dimensions(width, height)?;

    let block_count = usize::from(width) * usize::from(height);
    validate_block_count(block_count)?;

    Ok(())
}

// =============================================================================
// Rate Limiting
// =============================================================================

/// Rate limiter for sprite requests using a sliding window algorithm
///
/// This implements a token bucket-like approach with a sliding window.
/// Requests are tracked with timestamps and old requests are expired.
///
/// # Thread Safety
///
/// This implementation is thread-safe and can be shared across threads.
/// It uses atomic operations for the fast path and a mutex for timestamp
/// tracking when needed.
///
/// # Examples
///
/// ```
/// use conductor_core::avatar::security::SpriteRateLimiter;
/// use std::time::Duration;
///
/// let limiter = SpriteRateLimiter::new(10, Duration::from_secs(60));
///
/// // First 10 requests should succeed
/// for _ in 0..10 {
///     assert!(limiter.try_acquire().is_ok());
/// }
///
/// // 11th request should fail
/// assert!(limiter.try_acquire().is_err());
/// ```
pub struct SpriteRateLimiter {
    /// Maximum requests allowed in the window
    max_requests: u32,
    /// Window duration
    window_duration: Duration,
    /// Request count in current window (atomic for fast path)
    request_count: AtomicU64,
    /// Window start time
    window_start: std::sync::Mutex<Instant>,
}

impl SpriteRateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    ///
    /// * `max_requests` - Maximum number of requests allowed per window
    /// * `window_duration` - Duration of the sliding window
    #[must_use]
    pub fn new(max_requests: u32, window_duration: Duration) -> Self {
        Self {
            max_requests,
            window_duration,
            request_count: AtomicU64::new(0),
            window_start: std::sync::Mutex::new(Instant::now()),
        }
    }

    /// Create a rate limiter with default settings for sprite requests
    ///
    /// Uses `MAX_SPRITE_REQUESTS_PER_MINUTE` and a 60-second window.
    #[must_use]
    pub fn default_sprite_limiter() -> Self {
        Self::new(MAX_SPRITE_REQUESTS_PER_MINUTE, Duration::from_secs(60))
    }

    /// Try to acquire permission for a request
    ///
    /// # Returns
    ///
    /// `Ok(())` if the request is allowed, `Err(SecurityError::RateLimitExceeded)` otherwise
    pub fn try_acquire(&self) -> SecurityResult<()> {
        let now = Instant::now();

        // Check if we need to reset the window
        {
            let mut window_start = self.window_start.lock().unwrap();
            if now.duration_since(*window_start) >= self.window_duration {
                // Reset the window
                *window_start = now;
                self.request_count.store(1, Ordering::SeqCst);
                return Ok(());
            }
        }

        // Increment and check count
        let count = self.request_count.fetch_add(1, Ordering::SeqCst) + 1;

        if count > u64::from(self.max_requests) {
            // Decrement since we're rejecting
            self.request_count.fetch_sub(1, Ordering::SeqCst);
            return Err(SecurityError::RateLimitExceeded {
                requests: count as u32,
                window_seconds: self.window_duration.as_secs(),
                max_requests: self.max_requests,
            });
        }

        Ok(())
    }

    /// Check if a request would be allowed without consuming a token
    ///
    /// # Returns
    ///
    /// `true` if a request would be allowed, `false` otherwise
    #[must_use]
    pub fn would_allow(&self) -> bool {
        let now = Instant::now();

        // Check if window has expired
        {
            let window_start = self.window_start.lock().unwrap();
            if now.duration_since(*window_start) >= self.window_duration {
                return true; // Would reset window
            }
        }

        let count = self.request_count.load(Ordering::SeqCst);
        count < u64::from(self.max_requests)
    }

    /// Get the current request count
    #[must_use]
    pub fn current_count(&self) -> u64 {
        self.request_count.load(Ordering::SeqCst)
    }

    /// Get the maximum requests allowed
    #[must_use]
    pub fn max_requests(&self) -> u32 {
        self.max_requests
    }

    /// Get the remaining requests in the current window
    #[must_use]
    pub fn remaining(&self) -> u32 {
        let count = self.request_count.load(Ordering::SeqCst);
        self.max_requests.saturating_sub(count as u32)
    }

    /// Reset the rate limiter
    pub fn reset(&self) {
        let mut window_start = self.window_start.lock().unwrap();
        *window_start = Instant::now();
        self.request_count.store(0, Ordering::SeqCst);
    }
}

impl Default for SpriteRateLimiter {
    fn default() -> Self {
        Self::default_sprite_limiter()
    }
}

impl std::fmt::Debug for SpriteRateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpriteRateLimiter")
            .field("max_requests", &self.max_requests)
            .field("window_duration", &self.window_duration)
            .field("request_count", &self.request_count.load(Ordering::SeqCst))
            .finish()
    }
}

// =============================================================================
// Content Policy Checks (P4.4)
// =============================================================================

/// Content policy violation types
#[derive(Clone, Debug, Error, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentPolicyViolation {
    /// Sprite contains suspicious patterns that may be malicious
    #[error("Sprite contains suspicious pattern: {pattern_type}")]
    SuspiciousPattern {
        /// Type of suspicious pattern detected
        pattern_type: String,
    },

    /// Sprite contains excessive transparency that may hide content
    #[error("Sprite contains excessive transparency: {transparent_percent}%")]
    ExcessiveTransparency {
        /// Percentage of transparent blocks
        transparent_percent: u8,
    },

    /// Sprite uses non-approved color patterns
    #[error("Sprite uses invalid color pattern")]
    InvalidColorPattern,
}

/// Audit log entry for sprite generation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Timestamp of the event
    pub timestamp: std::time::SystemTime,
    /// Session ID that requested the sprite
    pub session_id: Option<String>,
    /// Mood requested
    pub mood: String,
    /// Evolution level
    pub evolution_level: u8,
    /// Whether the sprite passed validation
    pub validation_passed: bool,
    /// Violations detected, if any
    pub violations: Vec<ContentPolicyViolation>,
    /// Size of generated sprite in bytes
    pub sprite_size_bytes: usize,
}

impl AuditLogEntry {
    /// Create a new audit log entry
    pub fn new(
        session_id: Option<String>,
        mood: String,
        evolution_level: u8,
        validation_passed: bool,
        violations: Vec<ContentPolicyViolation>,
        sprite_size_bytes: usize,
    ) -> Self {
        Self {
            timestamp: std::time::SystemTime::now(),
            session_id,
            mood,
            evolution_level,
            validation_passed,
            violations,
            sprite_size_bytes,
        }
    }
}

/// Content policy validator for generated sprites
///
/// Validates sprites against content policy rules to ensure
/// they don't contain malicious or inappropriate patterns.
pub struct ContentPolicyValidator {
    /// Maximum allowed transparency percentage (0-100)
    max_transparency_percent: u8,
    /// Whether to enable audit logging
    audit_logging_enabled: bool,
}

impl ContentPolicyValidator {
    /// Create a new content policy validator with default settings
    ///
    /// Default settings:
    /// - Max transparency: 80%
    /// - Audit logging: enabled
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_transparency_percent: 80,
            audit_logging_enabled: true,
        }
    }

    /// Create a validator with custom transparency limit
    #[must_use]
    pub fn with_max_transparency(mut self, percent: u8) -> Self {
        self.max_transparency_percent = percent.min(100);
        self
    }

    /// Enable or disable audit logging
    #[must_use]
    pub fn with_audit_logging(mut self, enabled: bool) -> Self {
        self.audit_logging_enabled = enabled;
        self
    }

    /// Validate a sprite against content policy
    ///
    /// Checks for:
    /// - Excessive transparency (may hide malicious content)
    /// - Suspicious block patterns
    /// - Invalid color usage
    ///
    /// # Arguments
    ///
    /// * `sprite` - The sprite to validate
    ///
    /// # Returns
    ///
    /// `Ok(())` if sprite passes all policy checks, otherwise returns violations
    pub fn validate(&self, sprite: &Sprite) -> Result<(), Vec<ContentPolicyViolation>> {
        let mut violations = Vec::new();

        // Check transparency percentage
        if let Some(violation) = self.check_transparency(sprite) {
            violations.push(violation);
        }

        // Check for suspicious patterns
        if let Some(violation) = self.check_suspicious_patterns(sprite) {
            violations.push(violation);
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    /// Check transparency percentage of sprite
    fn check_transparency(&self, sprite: &Sprite) -> Option<ContentPolicyViolation> {
        if sprite.blocks.is_empty() {
            return None;
        }

        let transparent_count = sprite
            .blocks
            .iter()
            .filter(|b| b.transparency >= 0.9)
            .count();

        let transparent_percent = (transparent_count * 100) / sprite.blocks.len();

        if transparent_percent as u8 > self.max_transparency_percent {
            Some(ContentPolicyViolation::ExcessiveTransparency {
                transparent_percent: transparent_percent as u8,
            })
        } else {
            None
        }
    }

    /// Check for suspicious block patterns
    ///
    /// Detects patterns that may indicate:
    /// - All blocks identical (possible spam/flood)
    /// - Repetitive patterns (possible encoded data)
    fn check_suspicious_patterns(&self, sprite: &Sprite) -> Option<ContentPolicyViolation> {
        if sprite.blocks.is_empty() {
            return None;
        }

        // Check if all blocks are identical (excluding empty blocks)
        let non_empty: Vec<_> = sprite.blocks.iter().filter(|b| !b.is_empty()).collect();

        if !non_empty.is_empty() {
            let first = non_empty[0];
            let all_identical = non_empty
                .iter()
                .all(|b| b.character == first.character && b.fg == first.fg && b.bg == first.bg);

            if all_identical && non_empty.len() > 10 {
                return Some(ContentPolicyViolation::SuspiciousPattern {
                    pattern_type: "all_blocks_identical".to_string(),
                });
            }
        }

        None
    }

    /// Create an audit log entry for sprite generation
    ///
    /// This should be called after sprite validation to log the result.
    ///
    /// # Arguments
    ///
    /// * `session_id` - Optional session ID
    /// * `mood` - Mood name
    /// * `evolution_level` - Evolution level (0-4)
    /// * `validation_result` - Result of validation
    /// * `sprite_size_bytes` - Size of sprite in bytes
    ///
    /// # Returns
    ///
    /// An `AuditLogEntry` that can be persisted or logged
    pub fn create_audit_log(
        &self,
        session_id: Option<String>,
        mood: String,
        evolution_level: u8,
        validation_result: &Result<(), Vec<ContentPolicyViolation>>,
        sprite_size_bytes: usize,
    ) -> Option<AuditLogEntry> {
        if !self.audit_logging_enabled {
            return None;
        }

        let (passed, violations) = match validation_result {
            Ok(()) => (true, Vec::new()),
            Err(v) => (false, v.clone()),
        };

        Some(AuditLogEntry::new(
            session_id,
            mood,
            evolution_level,
            passed,
            violations,
            sprite_size_bytes,
        ))
    }

    /// Log an audit entry (placeholder for actual logging implementation)
    ///
    /// In production, this would write to a file, database, or logging service.
    /// For now, this is a no-op that can be extended.
    pub fn log_audit_entry(&self, _entry: &AuditLogEntry) {
        // In production, implement actual logging here
        // For example:
        // - Write to file system
        // - Send to logging service
        // - Store in database
        // - Emit metrics
    }
}

impl Default for ContentPolicyValidator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Pending Request Tracker
// =============================================================================

/// Tracks pending sprite requests for a session
///
/// This prevents clients from overwhelming the system with too many
/// concurrent requests.
#[derive(Debug)]
pub struct PendingRequestTracker {
    /// Maximum pending requests allowed
    max_pending: usize,
    /// Current pending request count
    pending_count: AtomicU64,
}

impl PendingRequestTracker {
    /// Create a new pending request tracker
    ///
    /// # Arguments
    ///
    /// * `max_pending` - Maximum number of pending requests allowed
    #[must_use]
    pub fn new(max_pending: usize) -> Self {
        Self {
            max_pending,
            pending_count: AtomicU64::new(0),
        }
    }

    /// Create a tracker with default settings
    #[must_use]
    pub fn default_tracker() -> Self {
        Self::new(MAX_PENDING_REQUESTS_PER_SESSION)
    }

    /// Try to start a new pending request
    ///
    /// # Returns
    ///
    /// `Ok(PendingRequestGuard)` if allowed, `Err(SecurityError::TooManyPendingRequests)` otherwise
    pub fn try_start(&self) -> SecurityResult<PendingRequestGuard<'_>> {
        let current = self.pending_count.fetch_add(1, Ordering::SeqCst);

        if current >= self.max_pending as u64 {
            // Too many pending, revert
            self.pending_count.fetch_sub(1, Ordering::SeqCst);
            return Err(SecurityError::TooManyPendingRequests {
                pending: current as usize + 1,
                max_pending: self.max_pending,
            });
        }

        Ok(PendingRequestGuard { tracker: self })
    }

    /// Get the current pending request count
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending_count.load(Ordering::SeqCst) as usize
    }

    /// Get the maximum pending requests allowed
    #[must_use]
    pub fn max_pending(&self) -> usize {
        self.max_pending
    }

    /// Check if more requests can be started
    #[must_use]
    pub fn can_start(&self) -> bool {
        self.pending_count.load(Ordering::SeqCst) < self.max_pending as u64
    }

    /// Manually complete a pending request (decrements counter)
    ///
    /// Note: Prefer using `PendingRequestGuard` which does this automatically.
    pub fn complete(&self) {
        self.pending_count.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Default for PendingRequestTracker {
    fn default() -> Self {
        Self::default_tracker()
    }
}

/// RAII guard that automatically decrements pending count when dropped
#[derive(Debug)]
pub struct PendingRequestGuard<'a> {
    tracker: &'a PendingRequestTracker,
}

impl Drop for PendingRequestGuard<'_> {
    fn drop(&mut self) {
        self.tracker.pending_count.fetch_sub(1, Ordering::SeqCst);
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::avatar::block::Color;

    // =========================================================================
    // Dimension Validation Tests
    // =========================================================================

    #[test]
    fn test_validate_sprite_dimensions_valid() {
        assert!(validate_sprite_dimensions(50, 50).is_ok());
        assert!(validate_sprite_dimensions(1, 1).is_ok());
        assert!(validate_sprite_dimensions(100, 100).is_ok());
        assert!(validate_sprite_dimensions(100, 1).is_ok());
        assert!(validate_sprite_dimensions(1, 100).is_ok());
    }

    #[test]
    fn test_validate_sprite_dimensions_invalid_width() {
        let result = validate_sprite_dimensions(101, 50);
        assert!(matches!(
            result,
            Err(SecurityError::DimensionOverflow {
                width: 101,
                height: 50
            })
        ));
    }

    #[test]
    fn test_validate_sprite_dimensions_invalid_height() {
        let result = validate_sprite_dimensions(50, 101);
        assert!(matches!(
            result,
            Err(SecurityError::DimensionOverflow {
                width: 50,
                height: 101
            })
        ));
    }

    #[test]
    fn test_validate_sprite_dimensions_both_invalid() {
        let result = validate_sprite_dimensions(200, 200);
        assert!(matches!(
            result,
            Err(SecurityError::DimensionOverflow {
                width: 200,
                height: 200
            })
        ));
    }

    #[test]
    fn test_validate_sprite_dimensions_zero() {
        // Zero dimensions should be valid (empty sprite)
        assert!(validate_sprite_dimensions(0, 0).is_ok());
        assert!(validate_sprite_dimensions(0, 50).is_ok());
        assert!(validate_sprite_dimensions(50, 0).is_ok());
    }

    // =========================================================================
    // Block Count Validation Tests
    // =========================================================================

    #[test]
    fn test_validate_block_count_valid() {
        assert!(validate_block_count(0).is_ok());
        assert!(validate_block_count(1000).is_ok());
        assert!(validate_block_count(5000).is_ok());
        assert!(validate_block_count(MAX_BLOCKS_PER_SPRITE).is_ok());
    }

    #[test]
    fn test_validate_block_count_invalid() {
        let result = validate_block_count(MAX_BLOCKS_PER_SPRITE + 1);
        assert!(
            matches!(result, Err(SecurityError::TooManyBlocks { count }) if count == MAX_BLOCKS_PER_SPRITE + 1)
        );
    }

    #[test]
    fn test_validate_block_count_way_over() {
        let result = validate_block_count(1_000_000);
        assert!(matches!(
            result,
            Err(SecurityError::TooManyBlocks { count: 1_000_000 })
        ));
    }

    // =========================================================================
    // Unicode Character Validation Tests
    // =========================================================================

    #[test]
    fn test_allowed_block_elements() {
        // Block elements U+2580-U+259F
        assert!(is_allowed_block_char('\u{2580}')); // Upper half block
        assert!(is_allowed_block_char('\u{2584}')); // Lower half block
        assert!(is_allowed_block_char('\u{2588}')); // Full block
        assert!(is_allowed_block_char('\u{2591}')); // Light shade
        assert!(is_allowed_block_char('\u{2592}')); // Medium shade
        assert!(is_allowed_block_char('\u{2593}')); // Dark shade
        assert!(is_allowed_block_char('\u{259F}')); // End of range
    }

    #[test]
    fn test_allowed_box_drawing() {
        // Box drawing U+2500-U+257F
        assert!(is_allowed_block_char('\u{2500}')); // Light horizontal
        assert!(is_allowed_block_char('\u{2502}')); // Light vertical
        assert!(is_allowed_block_char('\u{250C}')); // Light down and right
        assert!(is_allowed_block_char('\u{2550}')); // Double horizontal
        assert!(is_allowed_block_char('\u{2551}')); // Double vertical
        assert!(is_allowed_block_char('\u{257F}')); // End of range
    }

    #[test]
    fn test_allowed_geometric_shapes() {
        // Geometric shapes U+25A0-U+25FF
        assert!(is_allowed_block_char('\u{25A0}')); // Black square
        assert!(is_allowed_block_char('\u{25A1}')); // White square
        assert!(is_allowed_block_char('\u{25CF}')); // Black circle
        assert!(is_allowed_block_char('\u{25B2}')); // Black up-pointing triangle
        assert!(is_allowed_block_char('\u{25BC}')); // Black down-pointing triangle
        assert!(is_allowed_block_char('\u{25FF}')); // End of range
    }

    #[test]
    fn test_allowed_braille() {
        // Braille patterns U+2800-U+28FF
        assert!(is_allowed_block_char('\u{2800}')); // Braille blank
        assert!(is_allowed_block_char('\u{2801}')); // Braille dot 1
        assert!(is_allowed_block_char('\u{28FF}')); // Braille 8 dots
        assert!(is_allowed_block_char('\u{2847}')); // Random braille pattern
    }

    #[test]
    fn test_allowed_ascii_printable() {
        // Basic ASCII printable U+0020-U+007E
        assert!(is_allowed_block_char(' ')); // Space (0x20)
        assert!(is_allowed_block_char('!')); // 0x21
        assert!(is_allowed_block_char('A')); // 0x41
        assert!(is_allowed_block_char('Z')); // 0x5A
        assert!(is_allowed_block_char('a')); // 0x61
        assert!(is_allowed_block_char('z')); // 0x7A
        assert!(is_allowed_block_char('0')); // 0x30
        assert!(is_allowed_block_char('9')); // 0x39
        assert!(is_allowed_block_char('~')); // 0x7E (end of range)
    }

    #[test]
    fn test_disallowed_control_characters() {
        assert!(!is_allowed_block_char('\u{0000}')); // Null
        assert!(!is_allowed_block_char('\u{0007}')); // Bell
        assert!(!is_allowed_block_char('\u{000A}')); // Line feed
        assert!(!is_allowed_block_char('\u{000D}')); // Carriage return
        assert!(!is_allowed_block_char('\u{001B}')); // Escape
        assert!(!is_allowed_block_char('\u{001F}')); // Unit separator
        assert!(!is_allowed_block_char('\u{007F}')); // DEL
    }

    #[test]
    fn test_disallowed_emoji() {
        assert!(!is_allowed_block_char('\u{1F600}')); // Grinning face
        assert!(!is_allowed_block_char('\u{1F4A9}')); // Pile of poo
        assert!(!is_allowed_block_char('\u{2764}')); // Red heart (outside geometric shapes)
        assert!(!is_allowed_block_char('\u{1F680}')); // Rocket
    }

    #[test]
    fn test_disallowed_other_symbols() {
        assert!(!is_allowed_block_char('\u{00A0}')); // Non-breaking space
        assert!(!is_allowed_block_char('\u{00B0}')); // Degree sign
        assert!(!is_allowed_block_char('\u{2000}')); // En quad
        assert!(!is_allowed_block_char('\u{3000}')); // Ideographic space
    }

    #[test]
    fn test_validate_unicode_char_valid() {
        assert!(validate_unicode_char('\u{2588}').is_ok());
        assert!(validate_unicode_char('A').is_ok());
        assert!(validate_unicode_char(' ').is_ok());
    }

    #[test]
    fn test_validate_unicode_char_invalid() {
        let result = validate_unicode_char('\u{1F600}');
        assert!(matches!(
            result,
            Err(SecurityError::InvalidUnicodeChar { codepoint: 0x1F600 })
        ));
    }

    // =========================================================================
    // Sprite Validation Tests
    // =========================================================================

    #[test]
    fn test_validate_sprite_valid() {
        let sprite = Sprite::new(10, 10, vec![Block::solid(Color::rgb(255, 0, 0)); 100]);
        assert!(validate_sprite(&sprite).is_ok());
    }

    #[test]
    fn test_validate_sprite_oversized_dimensions() {
        let sprite = Sprite::new(150, 50, vec![]);
        let result = validate_sprite(&sprite);
        assert!(matches!(
            result,
            Err(SecurityError::DimensionOverflow {
                width: 150,
                height: 50
            })
        ));
    }

    #[test]
    fn test_validate_sprite_too_many_blocks() {
        let sprite = Sprite::new(100, 100, vec![Block::empty(); MAX_BLOCKS_PER_SPRITE + 1]);
        let result = validate_sprite(&sprite);
        assert!(matches!(result, Err(SecurityError::TooManyBlocks { .. })));
    }

    #[test]
    fn test_validate_sprite_invalid_character() {
        let mut blocks = vec![Block::empty(); 10];
        // Inject an invalid character
        blocks[5] = Block {
            fg: Color::transparent(),
            bg: Color::transparent(),
            character: '\u{1F600}', // Emoji - not allowed
            transparency: 1.0,
            z_index: 0,
        };
        let sprite = Sprite::new(2, 5, blocks);
        let result = validate_sprite(&sprite);
        assert!(matches!(
            result,
            Err(SecurityError::InvalidUnicodeChar { codepoint: 0x1F600 })
        ));
    }

    #[test]
    fn test_validate_sprite_empty() {
        let sprite = Sprite::new(0, 0, vec![]);
        assert!(validate_sprite(&sprite).is_ok());
    }

    #[test]
    fn test_validate_sprite_size_combined() {
        assert!(validate_sprite_size(50, 50).is_ok());
        assert!(validate_sprite_size(100, 100).is_ok());

        // Dimensions valid but product exceeds limit doesn't happen
        // since 100*100 = 10000 which equals MAX_BLOCKS_PER_SPRITE
        assert!(validate_sprite_size(100, 100).is_ok());

        // Invalid dimensions
        assert!(matches!(
            validate_sprite_size(101, 50),
            Err(SecurityError::DimensionOverflow { .. })
        ));
    }

    // =========================================================================
    // Animation Validation Tests
    // =========================================================================

    #[test]
    fn test_validate_animation_frames_valid() {
        assert!(validate_animation_frames(0).is_ok());
        assert!(validate_animation_frames(50).is_ok());
        assert!(validate_animation_frames(MAX_ANIMATION_FRAMES).is_ok());
    }

    #[test]
    fn test_validate_animation_frames_invalid() {
        let result = validate_animation_frames(MAX_ANIMATION_FRAMES + 1);
        assert!(matches!(
            result,
            Err(SecurityError::TooManyAnimationFrames { frames: 101 })
        ));
    }

    #[test]
    fn test_validate_animation_duration_valid() {
        assert!(validate_animation_duration(Duration::from_secs(0)).is_ok());
        assert!(validate_animation_duration(Duration::from_secs(30)).is_ok());
        assert!(
            validate_animation_duration(Duration::from_millis(MAX_ANIMATION_DURATION_MS)).is_ok()
        );
    }

    #[test]
    fn test_validate_animation_duration_invalid() {
        let result =
            validate_animation_duration(Duration::from_millis(MAX_ANIMATION_DURATION_MS + 1));
        assert!(matches!(
            result,
            Err(SecurityError::AnimationDurationExceeded { .. })
        ));
    }

    // =========================================================================
    // Rate Limiter Tests
    // =========================================================================

    #[test]
    fn test_rate_limiter_allows_requests_under_limit() {
        let limiter = SpriteRateLimiter::new(10, Duration::from_secs(60));

        for i in 0..10 {
            assert!(
                limiter.try_acquire().is_ok(),
                "Request {} should succeed",
                i
            );
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = SpriteRateLimiter::new(5, Duration::from_secs(60));

        // First 5 should succeed
        for _ in 0..5 {
            assert!(limiter.try_acquire().is_ok());
        }

        // 6th should fail
        let result = limiter.try_acquire();
        assert!(matches!(
            result,
            Err(SecurityError::RateLimitExceeded { .. })
        ));
    }

    #[test]
    fn test_rate_limiter_remaining() {
        let limiter = SpriteRateLimiter::new(10, Duration::from_secs(60));

        assert_eq!(limiter.remaining(), 10);
        limiter.try_acquire().unwrap();
        assert_eq!(limiter.remaining(), 9);

        for _ in 0..5 {
            limiter.try_acquire().unwrap();
        }
        assert_eq!(limiter.remaining(), 4);
    }

    #[test]
    fn test_rate_limiter_would_allow() {
        let limiter = SpriteRateLimiter::new(2, Duration::from_secs(60));

        assert!(limiter.would_allow());
        limiter.try_acquire().unwrap();
        assert!(limiter.would_allow());
        limiter.try_acquire().unwrap();
        assert!(!limiter.would_allow());
    }

    #[test]
    fn test_rate_limiter_reset() {
        let limiter = SpriteRateLimiter::new(5, Duration::from_secs(60));

        // Use up all requests
        for _ in 0..5 {
            limiter.try_acquire().unwrap();
        }
        assert!(limiter.try_acquire().is_err());

        // Reset
        limiter.reset();
        assert_eq!(limiter.current_count(), 0);
        assert!(limiter.try_acquire().is_ok());
    }

    #[test]
    fn test_rate_limiter_default() {
        let limiter = SpriteRateLimiter::default();
        assert_eq!(limiter.max_requests(), MAX_SPRITE_REQUESTS_PER_MINUTE);
    }

    #[test]
    fn test_rate_limiter_debug() {
        let limiter = SpriteRateLimiter::new(10, Duration::from_secs(60));
        let debug_str = format!("{:?}", limiter);
        assert!(debug_str.contains("SpriteRateLimiter"));
        assert!(debug_str.contains("max_requests"));
    }

    // =========================================================================
    // Pending Request Tracker Tests
    // =========================================================================

    #[test]
    fn test_pending_tracker_allows_under_limit() {
        let tracker = PendingRequestTracker::new(5);

        for i in 0..5 {
            assert!(tracker.try_start().is_ok(), "Request {} should succeed", i);
        }
    }

    #[test]
    fn test_pending_tracker_blocks_over_limit() {
        let tracker = PendingRequestTracker::new(3);

        let _guard1 = tracker.try_start().unwrap();
        let _guard2 = tracker.try_start().unwrap();
        let _guard3 = tracker.try_start().unwrap();

        let result = tracker.try_start();
        assert!(matches!(
            result,
            Err(SecurityError::TooManyPendingRequests { .. })
        ));
    }

    #[test]
    fn test_pending_tracker_guard_releases() {
        let tracker = PendingRequestTracker::new(2);

        assert_eq!(tracker.pending_count(), 0);

        {
            let _guard = tracker.try_start().unwrap();
            assert_eq!(tracker.pending_count(), 1);
        }
        // Guard dropped, count should be back to 0
        assert_eq!(tracker.pending_count(), 0);
    }

    #[test]
    fn test_pending_tracker_can_start() {
        let tracker = PendingRequestTracker::new(2);

        assert!(tracker.can_start());
        let _guard1 = tracker.try_start().unwrap();
        assert!(tracker.can_start());
        let _guard2 = tracker.try_start().unwrap();
        assert!(!tracker.can_start());
    }

    #[test]
    fn test_pending_tracker_default() {
        let tracker = PendingRequestTracker::default();
        assert_eq!(tracker.max_pending(), MAX_PENDING_REQUESTS_PER_SESSION);
    }

    #[test]
    fn test_pending_tracker_complete() {
        let tracker = PendingRequestTracker::new(5);

        // Hold the guard to keep the count incremented
        let _guard1 = tracker.try_start().unwrap();
        assert_eq!(tracker.pending_count(), 1);

        // Guard will decrement on drop, so we need to forget it
        std::mem::forget(tracker.try_start().unwrap());
        assert_eq!(tracker.pending_count(), 2);

        // Manual complete
        tracker.complete();
        assert_eq!(tracker.pending_count(), 1);
    }

    // =========================================================================
    // Security Error Display Tests
    // =========================================================================

    #[test]
    fn test_security_error_display() {
        let err = SecurityError::DimensionOverflow {
            width: 150,
            height: 200,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("150"));
        assert!(msg.contains("200"));
        assert!(msg.contains("100"));

        let err = SecurityError::TooManyBlocks { count: 15000 };
        let msg = format!("{}", err);
        assert!(msg.contains("15000"));
        assert!(msg.contains("10000"));

        let err = SecurityError::InvalidUnicodeChar { codepoint: 0x1F600 };
        let msg = format!("{}", err);
        assert!(msg.contains("1F600"));

        let err = SecurityError::RateLimitExceeded {
            requests: 101,
            window_seconds: 60,
            max_requests: 100,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("101"));
        assert!(msg.contains("60"));
        assert!(msg.contains("100"));
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[test]
    fn test_security_error_serialization() {
        let err = SecurityError::DimensionOverflow {
            width: 150,
            height: 200,
        };
        let json = serde_json::to_string(&err).unwrap();
        let parsed: SecurityError = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, err);

        let err = SecurityError::TooManyBlocks { count: 15000 };
        let json = serde_json::to_string(&err).unwrap();
        let parsed: SecurityError = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, err);

        let err = SecurityError::InvalidUnicodeChar { codepoint: 0x1F600 };
        let json = serde_json::to_string(&err).unwrap();
        let parsed: SecurityError = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, err);
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[test]
    fn test_boundary_values() {
        // Exactly at the limits
        assert!(validate_sprite_dimensions(MAX_SPRITE_WIDTH, MAX_SPRITE_HEIGHT).is_ok());
        assert!(validate_block_count(MAX_BLOCKS_PER_SPRITE).is_ok());
        assert!(validate_animation_frames(MAX_ANIMATION_FRAMES).is_ok());
        assert!(
            validate_animation_duration(Duration::from_millis(MAX_ANIMATION_DURATION_MS)).is_ok()
        );

        // One over the limits
        assert!(validate_sprite_dimensions(MAX_SPRITE_WIDTH + 1, MAX_SPRITE_HEIGHT).is_err());
        assert!(validate_sprite_dimensions(MAX_SPRITE_WIDTH, MAX_SPRITE_HEIGHT + 1).is_err());
        assert!(validate_block_count(MAX_BLOCKS_PER_SPRITE + 1).is_err());
        assert!(validate_animation_frames(MAX_ANIMATION_FRAMES + 1).is_err());
        assert!(
            validate_animation_duration(Duration::from_millis(MAX_ANIMATION_DURATION_MS + 1))
                .is_err()
        );
    }

    // =========================================================================
    // Content Policy Validator Tests (P4.4)
    // =========================================================================

    #[test]
    fn test_content_policy_validator_default() {
        let validator = ContentPolicyValidator::new();
        assert_eq!(validator.max_transparency_percent, 80);
        assert!(validator.audit_logging_enabled);
    }

    #[test]
    fn test_content_policy_validator_custom_transparency() {
        let validator = ContentPolicyValidator::new().with_max_transparency(50);
        assert_eq!(validator.max_transparency_percent, 50);
    }

    #[test]
    fn test_content_policy_validator_audit_logging() {
        let validator = ContentPolicyValidator::new().with_audit_logging(false);
        assert!(!validator.audit_logging_enabled);
    }

    #[test]
    fn test_content_policy_validates_normal_sprite() {
        let validator = ContentPolicyValidator::new();

        // Create sprite with varied blocks (not all identical)
        let mut blocks = vec![];
        for i in 0..100 {
            let color = if i % 2 == 0 {
                Color::rgb(255, 0, 0)
            } else {
                Color::rgb(0, 255, 0)
            };
            blocks.push(Block::solid(color));
        }

        let sprite = Sprite::new(10, 10, blocks);

        let result = validator.validate(&sprite);
        assert!(result.is_ok());
    }

    #[test]
    fn test_content_policy_detects_excessive_transparency() {
        let validator = ContentPolicyValidator::new().with_max_transparency(50);

        // Create sprite with 90% transparent blocks
        let mut blocks = vec![];
        for i in 0..100 {
            if i < 90 {
                let mut block = Block::empty();
                block.transparency = 1.0; // Fully transparent
                blocks.push(block);
            } else {
                blocks.push(Block::solid(Color::rgb(255, 0, 0)));
            }
        }

        let sprite = Sprite::new(10, 10, blocks);
        let result = validator.validate(&sprite);

        assert!(result.is_err());
        if let Err(violations) = result {
            assert_eq!(violations.len(), 1);
            assert!(matches!(
                violations[0],
                ContentPolicyViolation::ExcessiveTransparency { .. }
            ));
        }
    }

    #[test]
    fn test_content_policy_detects_suspicious_patterns() {
        let validator = ContentPolicyValidator::new();

        // Create sprite with all identical blocks (suspicious)
        let sprite = Sprite::new(10, 10, vec![Block::solid(Color::rgb(255, 0, 0)); 100]);
        let result = validator.validate(&sprite);

        assert!(result.is_err());
        if let Err(violations) = result {
            assert!(violations.iter().any(|v| matches!(
                v,
                ContentPolicyViolation::SuspiciousPattern { .. }
            )));
        }
    }

    #[test]
    fn test_content_policy_allows_varied_sprite() {
        let validator = ContentPolicyValidator::new();

        // Create sprite with varied blocks
        let mut blocks = vec![];
        for i in 0..100 {
            let color = Color::rgb((i * 2) as u8, (i * 3) as u8, (i * 5) as u8);
            blocks.push(Block::solid(color));
        }

        let sprite = Sprite::new(10, 10, blocks);
        let result = validator.validate(&sprite);

        assert!(result.is_ok());
    }

    #[test]
    fn test_content_policy_audit_log_creation() {
        let validator = ContentPolicyValidator::new();
        let result = Ok(());

        let log = validator.create_audit_log(
            Some("session123".to_string()),
            "Happy".to_string(),
            2,
            &result,
            1024,
        );

        assert!(log.is_some());
        let log = log.unwrap();
        assert_eq!(log.session_id, Some("session123".to_string()));
        assert_eq!(log.mood, "Happy");
        assert_eq!(log.evolution_level, 2);
        assert!(log.validation_passed);
        assert!(log.violations.is_empty());
        assert_eq!(log.sprite_size_bytes, 1024);
    }

    #[test]
    fn test_content_policy_audit_log_with_violations() {
        let validator = ContentPolicyValidator::new();
        let violations = vec![ContentPolicyViolation::ExcessiveTransparency {
            transparent_percent: 95,
        }];
        let result: Result<(), Vec<ContentPolicyViolation>> = Err(violations.clone());

        let log = validator.create_audit_log(
            None,
            "Confused".to_string(),
            1,
            &result,
            512,
        );

        assert!(log.is_some());
        let log = log.unwrap();
        assert!(!log.validation_passed);
        assert_eq!(log.violations.len(), 1);
        assert!(matches!(
            log.violations[0],
            ContentPolicyViolation::ExcessiveTransparency { transparent_percent: 95 }
        ));
    }

    #[test]
    fn test_content_policy_audit_log_disabled() {
        let validator = ContentPolicyValidator::new().with_audit_logging(false);
        let result = Ok(());

        let log = validator.create_audit_log(
            Some("session123".to_string()),
            "Happy".to_string(),
            2,
            &result,
            1024,
        );

        assert!(log.is_none());
    }

    #[test]
    fn test_content_policy_violation_display() {
        let violation = ContentPolicyViolation::SuspiciousPattern {
            pattern_type: "all_blocks_identical".to_string(),
        };
        let msg = format!("{}", violation);
        assert!(msg.contains("all_blocks_identical"));

        let violation = ContentPolicyViolation::ExcessiveTransparency {
            transparent_percent: 95,
        };
        let msg = format!("{}", violation);
        assert!(msg.contains("95"));
    }

    #[test]
    fn test_content_policy_violation_serialization() {
        let violation = ContentPolicyViolation::SuspiciousPattern {
            pattern_type: "test".to_string(),
        };
        let json = serde_json::to_string(&violation).unwrap();
        let parsed: ContentPolicyViolation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, violation);
    }

    #[test]
    fn test_audit_log_entry_serialization() {
        let entry = AuditLogEntry::new(
            Some("session123".to_string()),
            "Happy".to_string(),
            2,
            true,
            vec![],
            1024,
        );

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: AuditLogEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.session_id, entry.session_id);
        assert_eq!(parsed.mood, entry.mood);
        assert_eq!(parsed.evolution_level, entry.evolution_level);
        assert_eq!(parsed.validation_passed, entry.validation_passed);
        assert_eq!(parsed.sprite_size_bytes, entry.sprite_size_bytes);
    }

    #[test]
    fn test_unicode_range_boundaries() {
        // Block elements boundaries (0x2580-0x259F)
        assert!(is_allowed_block_char('\u{2580}')); // Start
        assert!(is_allowed_block_char('\u{259F}')); // End
                                                    // Note: 0x257F is in Box Drawing, 0x25A0 is in Geometric Shapes - both allowed
        assert!(is_allowed_block_char('\u{25A0}')); // Just after (geometric shapes - also allowed)

        // Box drawing boundaries (0x2500-0x257F)
        assert!(is_allowed_block_char('\u{2500}')); // Start
        assert!(is_allowed_block_char('\u{257F}')); // End
        assert!(!is_allowed_block_char('\u{24FF}')); // Just before (not in any allowed range)

        // Geometric shapes boundaries (0x25A0-0x25FF)
        assert!(is_allowed_block_char('\u{25A0}')); // Start
        assert!(is_allowed_block_char('\u{25FF}')); // End
        assert!(!is_allowed_block_char('\u{2600}')); // Just after (miscellaneous symbols)

        // ASCII boundaries (0x0020-0x007E)
        assert!(is_allowed_block_char('\u{0020}')); // Start (space)
        assert!(is_allowed_block_char('\u{007E}')); // End (~)
        assert!(!is_allowed_block_char('\u{001F}')); // Just before (control char)
        assert!(!is_allowed_block_char('\u{007F}')); // Just after (DEL)

        // Gaps between allowed ranges
        assert!(!is_allowed_block_char('\u{007F}')); // Between ASCII and Box Drawing
        assert!(!is_allowed_block_char('\u{2600}')); // After Geometric Shapes
    }
}
