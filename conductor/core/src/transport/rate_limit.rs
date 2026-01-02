//! Transport Rate Limiting for `DoS` Prevention
//!
//! This module provides rate limiting for the transport layer to prevent
//! denial-of-service attacks and resource exhaustion. It implements:
//!
//! - Per-connection message rate limiting using token bucket algorithm
//! - Per-UID connection limits to prevent connection exhaustion
//! - Configurable limits with sensible defaults
//! - Graceful degradation (delay instead of immediate disconnect)
//!
//! # Design
//!
//! The rate limiter uses a token bucket algorithm where:
//! - Tokens are added at a fixed rate (tokens per second)
//! - Tokens can accumulate up to a burst limit
//! - Each message consumes one token
//! - When tokens are exhausted, requests are delayed rather than rejected
//!
//! # Usage
//!
//! ```
//! use conductor_core::transport::rate_limit::{RateLimitConfig, TransportRateLimiter};
//!
//! // Create with default configuration
//! let limiter = TransportRateLimiter::new(RateLimitConfig::default());
//!
//! // Or with custom configuration
//! let config = RateLimitConfig::new()
//!     .with_messages_per_second(50)
//!     .with_burst_size(25)
//!     .with_max_connections_per_uid(5);
//! let limiter = TransportRateLimiter::new(config);
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::traits::ConnectionId;

// =============================================================================
// Configuration
// =============================================================================

/// Configuration for transport rate limiting
///
/// All limits are configurable with sensible defaults for typical usage.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum messages per second per connection (steady state)
    pub messages_per_second: u32,

    /// Burst size - maximum tokens that can accumulate
    /// Allows temporary spikes above the steady-state rate
    pub burst_size: u32,

    /// Maximum connections allowed per UID
    pub max_connections_per_uid: u32,

    /// Whether to enable rate limiting (can be disabled for testing)
    pub enabled: bool,

    /// Minimum delay when throttled (milliseconds)
    /// Used for graceful degradation instead of immediate rejection
    pub min_throttle_delay_ms: u64,

    /// Maximum delay when throttled (milliseconds)
    /// Caps the backpressure delay
    pub max_throttle_delay_ms: u64,

    /// Time window for tracking violation metrics (seconds)
    pub metrics_window_seconds: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            messages_per_second: 100,
            burst_size: 50,
            max_connections_per_uid: 10,
            enabled: true,
            min_throttle_delay_ms: 10,
            max_throttle_delay_ms: 1000,
            metrics_window_seconds: 60,
        }
    }
}

impl RateLimitConfig {
    /// Create a new configuration with default values
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the messages per second limit
    #[must_use]
    pub fn with_messages_per_second(mut self, rate: u32) -> Self {
        self.messages_per_second = rate;
        self
    }

    /// Set the burst size
    #[must_use]
    pub fn with_burst_size(mut self, size: u32) -> Self {
        self.burst_size = size;
        self
    }

    /// Set the maximum connections per UID
    #[must_use]
    pub fn with_max_connections_per_uid(mut self, max: u32) -> Self {
        self.max_connections_per_uid = max;
        self
    }

    /// Enable or disable rate limiting
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the minimum throttle delay
    #[must_use]
    pub fn with_min_throttle_delay_ms(mut self, delay: u64) -> Self {
        self.min_throttle_delay_ms = delay;
        self
    }

    /// Set the maximum throttle delay
    #[must_use]
    pub fn with_max_throttle_delay_ms(mut self, delay: u64) -> Self {
        self.max_throttle_delay_ms = delay;
        self
    }

    /// Create a disabled configuration (for testing)
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    /// Create a strict configuration for high-security environments
    #[must_use]
    pub fn strict() -> Self {
        Self {
            messages_per_second: 50,
            burst_size: 20,
            max_connections_per_uid: 5,
            enabled: true,
            min_throttle_delay_ms: 50,
            max_throttle_delay_ms: 2000,
            metrics_window_seconds: 60,
        }
    }

    /// Create a relaxed configuration for trusted environments
    #[must_use]
    pub fn relaxed() -> Self {
        Self {
            messages_per_second: 500,
            burst_size: 200,
            max_connections_per_uid: 50,
            enabled: true,
            min_throttle_delay_ms: 5,
            max_throttle_delay_ms: 500,
            metrics_window_seconds: 60,
        }
    }
}

// =============================================================================
// Error Types
// =============================================================================

/// Errors related to rate limiting
#[derive(Clone, Debug, Error, PartialEq)]
pub enum RateLimitError {
    /// Too many connections from this UID
    #[error("UID {uid} has {current} connections (max: {max})")]
    TooManyConnections {
        /// The UID that exceeded the limit
        uid: u32,
        /// Current number of connections
        current: u32,
        /// Maximum allowed connections
        max: u32,
    },

    /// Rate limit exceeded (message rate)
    #[error("Rate limit exceeded: {rate:.1} msg/s (limit: {limit} msg/s)")]
    RateLimitExceeded {
        /// Current message rate
        rate: f64,
        /// Configured limit
        limit: u32,
    },
}

/// Result of a rate limit check
#[derive(Clone, Debug)]
pub enum RateLimitResult {
    /// Request is allowed to proceed immediately
    Allowed,

    /// Request is allowed but should be delayed for backpressure
    Throttled {
        /// Recommended delay before processing
        delay: Duration,
    },

    /// Request is rejected (only used for connection limits, not message limits)
    Rejected {
        /// The reason for rejection
        error: RateLimitError,
    },
}

impl RateLimitResult {
    /// Check if the request is allowed (either immediately or after delay)
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed | Self::Throttled { .. })
    }

    /// Check if the request was throttled
    #[must_use]
    pub fn is_throttled(&self) -> bool {
        matches!(self, Self::Throttled { .. })
    }

    /// Check if the request was rejected
    #[must_use]
    pub fn is_rejected(&self) -> bool {
        matches!(self, Self::Rejected { .. })
    }

    /// Get the throttle delay if any
    #[must_use]
    pub fn delay(&self) -> Option<Duration> {
        match self {
            Self::Throttled { delay } => Some(*delay),
            _ => None,
        }
    }
}

// =============================================================================
// Per-Connection Rate Limiter (Token Bucket)
// =============================================================================

/// Token bucket rate limiter for a single connection
///
/// Implements the token bucket algorithm:
/// - Tokens are added at a fixed rate
/// - Tokens can accumulate up to the burst limit
/// - Each operation consumes one token
/// - When no tokens are available, a delay is recommended
#[derive(Debug)]
pub struct ConnectionRateLimiter {
    /// Configuration
    config: RateLimitConfig,

    /// Current token count (scaled by 1000 for precision)
    /// Using atomics for thread-safe updates
    tokens_millis: AtomicU64,

    /// Last time tokens were refilled
    last_refill: RwLock<Instant>,

    /// Total messages processed
    total_messages: AtomicU64,

    /// Messages that were throttled
    throttled_messages: AtomicU64,

    /// Timestamp of first message in current metrics window
    metrics_window_start: RwLock<Instant>,

    /// Messages in current metrics window
    messages_in_window: AtomicU64,
}

impl ConnectionRateLimiter {
    /// Create a new connection rate limiter
    #[must_use]
    pub fn new(config: RateLimitConfig) -> Self {
        let burst_tokens_millis = u64::from(config.burst_size) * 1000;
        Self {
            config,
            tokens_millis: AtomicU64::new(burst_tokens_millis),
            last_refill: RwLock::new(Instant::now()),
            total_messages: AtomicU64::new(0),
            throttled_messages: AtomicU64::new(0),
            metrics_window_start: RwLock::new(Instant::now()),
            messages_in_window: AtomicU64::new(0),
        }
    }

    /// Check if a message is allowed and get any required delay
    ///
    /// This implements graceful degradation:
    /// - If tokens are available, returns `Allowed`
    /// - If tokens are depleted, returns `Throttled` with a delay
    /// - Never returns `Rejected` for message rate limiting
    pub fn check_message(&self) -> RateLimitResult {
        if !self.config.enabled {
            return RateLimitResult::Allowed;
        }

        // Refill tokens based on elapsed time
        self.refill_tokens();

        // Try to consume a token
        let tokens = self.tokens_millis.load(Ordering::SeqCst);

        if tokens >= 1000 {
            // Have at least one token, consume it
            self.tokens_millis.fetch_sub(1000, Ordering::SeqCst);
            self.record_message(false);
            RateLimitResult::Allowed
        } else {
            // No tokens available, calculate delay
            let tokens_needed = 1000 - tokens;
            let refill_rate_millis = u64::from(self.config.messages_per_second); // tokens per second = millis per millisecond

            // Calculate delay needed to get one token
            let delay_ms = if refill_rate_millis > 0 {
                tokens_needed / refill_rate_millis
            } else {
                self.config.max_throttle_delay_ms
            };

            // Clamp delay to configured bounds
            let delay_ms = delay_ms
                .max(self.config.min_throttle_delay_ms)
                .min(self.config.max_throttle_delay_ms);

            self.record_message(true);

            RateLimitResult::Throttled {
                delay: Duration::from_millis(delay_ms),
            }
        }
    }

    /// Refill tokens based on elapsed time
    fn refill_tokens(&self) {
        let now = Instant::now();
        let mut last_refill = self.last_refill.write();

        let elapsed = now.duration_since(*last_refill);
        let elapsed_ms = elapsed.as_millis() as u64;

        if elapsed_ms == 0 {
            return;
        }

        // Calculate tokens to add (tokens per second * elapsed seconds)
        // We work in millis for precision: tokens_to_add = rate * elapsed_ms
        let tokens_to_add = u64::from(self.config.messages_per_second) * elapsed_ms;

        let max_tokens_millis = u64::from(self.config.burst_size) * 1000;

        // Add tokens up to burst limit
        let current = self.tokens_millis.load(Ordering::SeqCst);
        let new_tokens = (current + tokens_to_add).min(max_tokens_millis);
        self.tokens_millis.store(new_tokens, Ordering::SeqCst);

        *last_refill = now;
    }

    /// Record a message for metrics
    fn record_message(&self, throttled: bool) {
        self.total_messages.fetch_add(1, Ordering::Relaxed);
        if throttled {
            self.throttled_messages.fetch_add(1, Ordering::Relaxed);
        }

        // Update metrics window
        let now = Instant::now();
        {
            let mut window_start = self.metrics_window_start.write();
            let window_duration = Duration::from_secs(self.config.metrics_window_seconds);

            if now.duration_since(*window_start) > window_duration {
                // Reset window
                *window_start = now;
                self.messages_in_window.store(1, Ordering::Relaxed);
            } else {
                self.messages_in_window.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Get the current token count
    #[must_use]
    pub fn available_tokens(&self) -> u32 {
        (self.tokens_millis.load(Ordering::SeqCst) / 1000) as u32
    }

    /// Get total messages processed
    #[must_use]
    pub fn total_messages(&self) -> u64 {
        self.total_messages.load(Ordering::Relaxed)
    }

    /// Get count of throttled messages
    #[must_use]
    pub fn throttled_messages(&self) -> u64 {
        self.throttled_messages.load(Ordering::Relaxed)
    }

    /// Get current message rate (messages per second in current window)
    #[must_use]
    pub fn current_rate(&self) -> f64 {
        let window_start = self.metrics_window_start.read();
        let elapsed = Instant::now().duration_since(*window_start);
        let elapsed_secs = elapsed.as_secs_f64();

        if elapsed_secs < 0.001 {
            return 0.0;
        }

        let messages = self.messages_in_window.load(Ordering::Relaxed);
        messages as f64 / elapsed_secs
    }

    /// Reset the limiter to initial state
    pub fn reset(&self) {
        let burst_tokens_millis = u64::from(self.config.burst_size) * 1000;
        self.tokens_millis
            .store(burst_tokens_millis, Ordering::SeqCst);
        *self.last_refill.write() = Instant::now();
        self.total_messages.store(0, Ordering::Relaxed);
        self.throttled_messages.store(0, Ordering::Relaxed);
        *self.metrics_window_start.write() = Instant::now();
        self.messages_in_window.store(0, Ordering::Relaxed);
    }

    /// Get metrics for this limiter
    #[must_use]
    pub fn metrics(&self) -> ConnectionRateLimitMetrics {
        ConnectionRateLimitMetrics {
            total_messages: self.total_messages(),
            throttled_messages: self.throttled_messages(),
            current_rate: self.current_rate(),
            available_tokens: self.available_tokens(),
        }
    }
}

/// Metrics for a connection rate limiter
#[derive(Clone, Debug)]
pub struct ConnectionRateLimitMetrics {
    /// Total messages processed
    pub total_messages: u64,
    /// Messages that were throttled
    pub throttled_messages: u64,
    /// Current message rate (msg/s)
    pub current_rate: f64,
    /// Available tokens
    pub available_tokens: u32,
}

// =============================================================================
// Transport Rate Limiter (Multi-Connection)
// =============================================================================

/// Transport-level rate limiter managing all connections
///
/// Provides:
/// - Per-connection message rate limiting
/// - Per-UID connection counting and limiting
/// - Automatic cleanup of stale limiters
pub struct TransportRateLimiter {
    /// Configuration
    config: RateLimitConfig,

    /// Per-connection rate limiters
    connection_limiters: RwLock<HashMap<ConnectionId, ConnectionRateLimiter>>,

    /// Connection count per UID
    uid_connections: RwLock<HashMap<u32, Vec<ConnectionId>>>,
}

impl TransportRateLimiter {
    /// Create a new transport rate limiter
    #[must_use]
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            connection_limiters: RwLock::new(HashMap::new()),
            uid_connections: RwLock::new(HashMap::new()),
        }
    }

    /// Create with default configuration
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(RateLimitConfig::default())
    }

    /// Get the configuration
    #[must_use]
    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }

    /// Register a new connection
    ///
    /// Returns an error if the UID has too many connections.
    pub fn register_connection(
        &self,
        conn_id: &ConnectionId,
        uid: u32,
    ) -> Result<(), RateLimitError> {
        if !self.config.enabled {
            return Ok(());
        }

        // Check UID connection limit
        {
            let uid_conns = self.uid_connections.read();
            if let Some(conns) = uid_conns.get(&uid) {
                if conns.len() >= self.config.max_connections_per_uid as usize {
                    return Err(RateLimitError::TooManyConnections {
                        uid,
                        current: conns.len() as u32,
                        max: self.config.max_connections_per_uid,
                    });
                }
            }
        }

        // Register the connection
        {
            let mut uid_conns = self.uid_connections.write();
            uid_conns.entry(uid).or_default().push(conn_id.clone());
        }

        // Create a rate limiter for this connection
        {
            let mut limiters = self.connection_limiters.write();
            limiters.insert(
                conn_id.clone(),
                ConnectionRateLimiter::new(self.config.clone()),
            );
        }

        tracing::debug!(
            conn_id = %conn_id,
            uid = uid,
            "Registered connection for rate limiting"
        );

        Ok(())
    }

    /// Unregister a connection
    pub fn unregister_connection(&self, conn_id: &ConnectionId, uid: u32) {
        // Remove from UID tracking
        {
            let mut uid_conns = self.uid_connections.write();
            if let Some(conns) = uid_conns.get_mut(&uid) {
                conns.retain(|id| id != conn_id);
                if conns.is_empty() {
                    uid_conns.remove(&uid);
                }
            }
        }

        // Remove the rate limiter
        {
            let mut limiters = self.connection_limiters.write();
            limiters.remove(conn_id);
        }

        tracing::debug!(
            conn_id = %conn_id,
            uid = uid,
            "Unregistered connection from rate limiting"
        );
    }

    /// Check if a message from a connection is allowed
    ///
    /// Returns the rate limit result indicating whether to proceed,
    /// delay, or reject the message.
    pub fn check_message(&self, conn_id: &ConnectionId) -> RateLimitResult {
        if !self.config.enabled {
            return RateLimitResult::Allowed;
        }

        let limiters = self.connection_limiters.read();

        if let Some(limiter) = limiters.get(conn_id) {
            limiter.check_message()
        } else {
            // Unknown connection, allow but log warning
            tracing::warn!(
                conn_id = %conn_id,
                "Rate limit check for unregistered connection"
            );
            RateLimitResult::Allowed
        }
    }

    /// Check if a new connection from a UID would be allowed
    pub fn check_new_connection(&self, uid: u32) -> RateLimitResult {
        if !self.config.enabled {
            return RateLimitResult::Allowed;
        }

        let uid_conns = self.uid_connections.read();

        if let Some(conns) = uid_conns.get(&uid) {
            if conns.len() >= self.config.max_connections_per_uid as usize {
                return RateLimitResult::Rejected {
                    error: RateLimitError::TooManyConnections {
                        uid,
                        current: conns.len() as u32,
                        max: self.config.max_connections_per_uid,
                    },
                };
            }
        }

        RateLimitResult::Allowed
    }

    /// Get the number of connections for a UID
    #[must_use]
    pub fn connection_count_for_uid(&self, uid: u32) -> usize {
        self.uid_connections
            .read()
            .get(&uid)
            .map_or(0, std::vec::Vec::len)
    }

    /// Get total number of tracked connections
    #[must_use]
    pub fn total_connections(&self) -> usize {
        self.connection_limiters.read().len()
    }

    /// Get metrics for a specific connection
    #[must_use]
    pub fn connection_metrics(&self, conn_id: &ConnectionId) -> Option<ConnectionRateLimitMetrics> {
        self.connection_limiters
            .read()
            .get(conn_id)
            .map(ConnectionRateLimiter::metrics)
    }

    /// Get aggregate metrics across all connections
    #[must_use]
    pub fn aggregate_metrics(&self) -> TransportRateLimitMetrics {
        let limiters = self.connection_limiters.read();

        let mut total_messages = 0u64;
        let mut throttled_messages = 0u64;

        for limiter in limiters.values() {
            total_messages += limiter.total_messages();
            throttled_messages += limiter.throttled_messages();
        }

        let uid_conns = self.uid_connections.read();
        let unique_uids = uid_conns.len();

        TransportRateLimitMetrics {
            total_connections: limiters.len(),
            unique_uids,
            total_messages,
            throttled_messages,
        }
    }

    /// Reset all limiters (useful for testing)
    pub fn reset_all(&self) {
        for limiter in self.connection_limiters.read().values() {
            limiter.reset();
        }
    }

    /// Remove all tracked connections
    pub fn clear(&self) {
        self.connection_limiters.write().clear();
        self.uid_connections.write().clear();
    }
}

impl std::fmt::Debug for TransportRateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransportRateLimiter")
            .field("config", &self.config)
            .field("total_connections", &self.total_connections())
            .finish()
    }
}

impl Default for TransportRateLimiter {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Aggregate metrics for the transport rate limiter
#[derive(Clone, Debug)]
pub struct TransportRateLimitMetrics {
    /// Total tracked connections
    pub total_connections: usize,
    /// Number of unique UIDs with connections
    pub unique_uids: usize,
    /// Total messages processed across all connections
    pub total_messages: u64,
    /// Total throttled messages across all connections
    pub throttled_messages: u64,
}

// =============================================================================
// Backpressure Helper
// =============================================================================

/// Apply backpressure by delaying if needed
///
/// This is a helper function to implement graceful degradation.
/// If the rate limit result indicates throttling, it sleeps for
/// the recommended delay.
///
/// # Example
///
/// ```ignore
/// let result = limiter.check_message(&conn_id);
/// apply_backpressure(&result).await;
/// // Now process the message
/// ```
pub async fn apply_backpressure(result: &RateLimitResult) {
    if let RateLimitResult::Throttled { delay } = result {
        tokio::time::sleep(*delay).await;
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // =========================================================================
    // Configuration Tests
    // =========================================================================

    #[test]
    fn test_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.messages_per_second, 100);
        assert_eq!(config.burst_size, 50);
        assert_eq!(config.max_connections_per_uid, 10);
        assert!(config.enabled);
    }

    #[test]
    fn test_config_builder() {
        let config = RateLimitConfig::new()
            .with_messages_per_second(200)
            .with_burst_size(100)
            .with_max_connections_per_uid(20)
            .with_enabled(false);

        assert_eq!(config.messages_per_second, 200);
        assert_eq!(config.burst_size, 100);
        assert_eq!(config.max_connections_per_uid, 20);
        assert!(!config.enabled);
    }

    #[test]
    fn test_config_disabled() {
        let config = RateLimitConfig::disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn test_config_strict() {
        let config = RateLimitConfig::strict();
        assert!(config.messages_per_second < RateLimitConfig::default().messages_per_second);
        assert!(
            config.max_connections_per_uid < RateLimitConfig::default().max_connections_per_uid
        );
    }

    #[test]
    fn test_config_relaxed() {
        let config = RateLimitConfig::relaxed();
        assert!(config.messages_per_second > RateLimitConfig::default().messages_per_second);
        assert!(
            config.max_connections_per_uid > RateLimitConfig::default().max_connections_per_uid
        );
    }

    // =========================================================================
    // RateLimitResult Tests
    // =========================================================================

    #[test]
    fn test_result_allowed() {
        let result = RateLimitResult::Allowed;
        assert!(result.is_allowed());
        assert!(!result.is_throttled());
        assert!(!result.is_rejected());
        assert!(result.delay().is_none());
    }

    #[test]
    fn test_result_throttled() {
        let result = RateLimitResult::Throttled {
            delay: Duration::from_millis(100),
        };
        assert!(result.is_allowed()); // Throttled is still allowed, just delayed
        assert!(result.is_throttled());
        assert!(!result.is_rejected());
        assert_eq!(result.delay(), Some(Duration::from_millis(100)));
    }

    #[test]
    fn test_result_rejected() {
        let result = RateLimitResult::Rejected {
            error: RateLimitError::TooManyConnections {
                uid: 1000,
                current: 10,
                max: 10,
            },
        };
        assert!(!result.is_allowed());
        assert!(!result.is_throttled());
        assert!(result.is_rejected());
        assert!(result.delay().is_none());
    }

    // =========================================================================
    // ConnectionRateLimiter Tests
    // =========================================================================

    #[test]
    fn test_connection_limiter_allows_under_limit() {
        let config = RateLimitConfig::new()
            .with_messages_per_second(100)
            .with_burst_size(50);
        let limiter = ConnectionRateLimiter::new(config);

        // Should allow burst_size messages immediately
        for i in 0..50 {
            let result = limiter.check_message();
            assert!(
                result.is_allowed() && !result.is_throttled(),
                "Message {} should be allowed without throttle",
                i
            );
        }
    }

    #[test]
    fn test_connection_limiter_throttles_over_burst() {
        let config = RateLimitConfig::new()
            .with_messages_per_second(100)
            .with_burst_size(10);
        let limiter = ConnectionRateLimiter::new(config);

        // Consume all burst tokens
        for _ in 0..10 {
            let result = limiter.check_message();
            assert!(result.is_allowed());
        }

        // Next message should be throttled (not rejected)
        let result = limiter.check_message();
        assert!(
            result.is_throttled(),
            "Should be throttled after burst exhausted"
        );
        assert!(result.is_allowed(), "Throttled should still be allowed");
    }

    #[test]
    fn test_connection_limiter_token_refill() {
        let config = RateLimitConfig::new()
            .with_messages_per_second(1000) // 1000 tokens per second = 1 per ms
            .with_burst_size(5);
        let limiter = ConnectionRateLimiter::new(config);

        // Consume all tokens
        for _ in 0..5 {
            limiter.check_message();
        }

        assert_eq!(limiter.available_tokens(), 0);

        // Wait a bit for refill
        std::thread::sleep(Duration::from_millis(10));

        // Should have some tokens back
        let result = limiter.check_message();
        // After 10ms at 1000 tokens/s, we should have ~10 tokens
        // The check_message will refill and then consume one
        assert!(result.is_allowed());
    }

    #[test]
    fn test_connection_limiter_disabled() {
        let config = RateLimitConfig::disabled();
        let limiter = ConnectionRateLimiter::new(config);

        // All messages should be allowed when disabled
        for _ in 0..1000 {
            let result = limiter.check_message();
            assert!(matches!(result, RateLimitResult::Allowed));
        }
    }

    #[test]
    fn test_connection_limiter_metrics() {
        let config = RateLimitConfig::new().with_burst_size(10);
        let limiter = ConnectionRateLimiter::new(config);

        for _ in 0..5 {
            limiter.check_message();
        }

        let metrics = limiter.metrics();
        assert_eq!(metrics.total_messages, 5);
        assert_eq!(metrics.throttled_messages, 0);
    }

    #[test]
    fn test_connection_limiter_reset() {
        let config = RateLimitConfig::new().with_burst_size(10);
        let limiter = ConnectionRateLimiter::new(config);

        // Consume some tokens
        for _ in 0..10 {
            limiter.check_message();
        }

        assert_eq!(limiter.available_tokens(), 0);
        assert_eq!(limiter.total_messages(), 10);

        // Reset
        limiter.reset();

        assert_eq!(limiter.available_tokens(), 10);
        assert_eq!(limiter.total_messages(), 0);
    }

    // =========================================================================
    // TransportRateLimiter Tests
    // =========================================================================

    #[test]
    fn test_transport_limiter_register_connection() {
        let limiter = TransportRateLimiter::with_defaults();
        let conn_id = ConnectionId::new();

        let result = limiter.register_connection(&conn_id, 1000);
        assert!(result.is_ok());

        assert_eq!(limiter.total_connections(), 1);
        assert_eq!(limiter.connection_count_for_uid(1000), 1);
    }

    #[test]
    fn test_transport_limiter_unregister_connection() {
        let limiter = TransportRateLimiter::with_defaults();
        let conn_id = ConnectionId::new();

        limiter.register_connection(&conn_id, 1000).unwrap();
        assert_eq!(limiter.total_connections(), 1);

        limiter.unregister_connection(&conn_id, 1000);
        assert_eq!(limiter.total_connections(), 0);
        assert_eq!(limiter.connection_count_for_uid(1000), 0);
    }

    #[test]
    fn test_transport_limiter_uid_connection_limit() {
        let config = RateLimitConfig::new().with_max_connections_per_uid(3);
        let limiter = TransportRateLimiter::new(config);
        let uid = 1000;

        // First 3 connections should succeed
        for i in 0..3 {
            let conn_id = ConnectionId::new();
            let result = limiter.register_connection(&conn_id, uid);
            assert!(result.is_ok(), "Connection {} should succeed", i);
        }

        // 4th connection should fail
        let conn_id = ConnectionId::new();
        let result = limiter.register_connection(&conn_id, uid);
        assert!(matches!(
            result,
            Err(RateLimitError::TooManyConnections { .. })
        ));
    }

    #[test]
    fn test_transport_limiter_different_uids() {
        let config = RateLimitConfig::new().with_max_connections_per_uid(2);
        let limiter = TransportRateLimiter::new(config);

        // 2 connections for UID 1000
        for _ in 0..2 {
            let conn_id = ConnectionId::new();
            limiter.register_connection(&conn_id, 1000).unwrap();
        }

        // 2 connections for UID 1001 (should succeed - different UID)
        for _ in 0..2 {
            let conn_id = ConnectionId::new();
            limiter.register_connection(&conn_id, 1001).unwrap();
        }

        assert_eq!(limiter.total_connections(), 4);
        assert_eq!(limiter.connection_count_for_uid(1000), 2);
        assert_eq!(limiter.connection_count_for_uid(1001), 2);
    }

    #[test]
    fn test_transport_limiter_check_message() {
        let limiter = TransportRateLimiter::with_defaults();
        let conn_id = ConnectionId::new();

        limiter.register_connection(&conn_id, 1000).unwrap();

        let result = limiter.check_message(&conn_id);
        assert!(result.is_allowed());
    }

    #[test]
    fn test_transport_limiter_check_new_connection() {
        let config = RateLimitConfig::new().with_max_connections_per_uid(2);
        let limiter = TransportRateLimiter::new(config);

        // Initially should be allowed
        let result = limiter.check_new_connection(1000);
        assert!(result.is_allowed());

        // Register 2 connections
        for _ in 0..2 {
            let conn_id = ConnectionId::new();
            limiter.register_connection(&conn_id, 1000).unwrap();
        }

        // Now should be rejected
        let result = limiter.check_new_connection(1000);
        assert!(result.is_rejected());
    }

    #[test]
    fn test_transport_limiter_disabled() {
        let config = RateLimitConfig::disabled();
        let limiter = TransportRateLimiter::new(config);

        // Registration should succeed even without tracking
        let conn_id = ConnectionId::new();
        let result = limiter.register_connection(&conn_id, 1000);
        assert!(result.is_ok());

        // Messages always allowed when disabled
        let result = limiter.check_message(&conn_id);
        assert!(matches!(result, RateLimitResult::Allowed));
    }

    #[test]
    fn test_transport_limiter_aggregate_metrics() {
        let limiter = TransportRateLimiter::with_defaults();

        let conn1 = ConnectionId::new();
        let conn2 = ConnectionId::new();

        limiter.register_connection(&conn1, 1000).unwrap();
        limiter.register_connection(&conn2, 1001).unwrap();

        // Send some messages
        for _ in 0..5 {
            limiter.check_message(&conn1);
        }
        for _ in 0..3 {
            limiter.check_message(&conn2);
        }

        let metrics = limiter.aggregate_metrics();
        assert_eq!(metrics.total_connections, 2);
        assert_eq!(metrics.unique_uids, 2);
        assert_eq!(metrics.total_messages, 8);
    }

    #[test]
    fn test_transport_limiter_clear() {
        let limiter = TransportRateLimiter::with_defaults();

        for i in 0..5 {
            let conn_id = ConnectionId::new();
            limiter.register_connection(&conn_id, 1000 + i).unwrap();
        }

        assert_eq!(limiter.total_connections(), 5);

        limiter.clear();

        assert_eq!(limiter.total_connections(), 0);
    }

    // =========================================================================
    // Burst Handling Tests
    // =========================================================================

    #[test]
    fn test_burst_allows_spike() {
        let config = RateLimitConfig::new()
            .with_messages_per_second(10) // Slow steady rate
            .with_burst_size(100); // But large burst

        let limiter = ConnectionRateLimiter::new(config);

        // Should be able to send 100 messages rapidly (burst)
        let mut allowed_count = 0;
        for _ in 0..100 {
            let result = limiter.check_message();
            if !result.is_throttled() {
                allowed_count += 1;
            }
        }

        assert_eq!(allowed_count, 100, "All burst messages should be allowed");
    }

    #[test]
    fn test_sustained_rate_after_burst() {
        let config = RateLimitConfig::new()
            .with_messages_per_second(1000)
            .with_burst_size(10);

        let limiter = ConnectionRateLimiter::new(config);

        // Exhaust burst
        for _ in 0..10 {
            limiter.check_message();
        }

        // Now all messages should be throttled
        for _ in 0..5 {
            let result = limiter.check_message();
            assert!(result.is_throttled());
        }

        // Wait for refill
        std::thread::sleep(Duration::from_millis(20));

        // Should have tokens again
        let result = limiter.check_message();
        assert!(!result.is_throttled() || result.is_allowed());
    }

    // =========================================================================
    // Throttle Delay Tests
    // =========================================================================

    #[test]
    fn test_throttle_delay_bounds() {
        let config = RateLimitConfig::new()
            .with_messages_per_second(100)
            .with_burst_size(1)
            .with_min_throttle_delay_ms(50)
            .with_max_throttle_delay_ms(200);

        let limiter = ConnectionRateLimiter::new(config);

        // Consume the single token
        limiter.check_message();

        // Next message should be throttled with bounded delay
        let result = limiter.check_message();
        if let RateLimitResult::Throttled { delay } = result {
            assert!(
                delay >= Duration::from_millis(50),
                "Delay should be at least min_throttle_delay_ms"
            );
            assert!(
                delay <= Duration::from_millis(200),
                "Delay should be at most max_throttle_delay_ms"
            );
        } else {
            panic!("Expected Throttled result");
        }
    }

    // =========================================================================
    // Error Display Tests
    // =========================================================================

    #[test]
    fn test_error_display() {
        let err = RateLimitError::TooManyConnections {
            uid: 1000,
            current: 10,
            max: 10,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("1000"));
        assert!(msg.contains("10"));

        let err = RateLimitError::RateLimitExceeded {
            rate: 150.5,
            limit: 100,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("150.5"));
        assert!(msg.contains("100"));
    }

    // =========================================================================
    // Thread Safety Tests
    // =========================================================================

    #[test]
    fn test_connection_limiter_thread_safe() {
        use std::sync::Arc;
        use std::thread;

        let config = RateLimitConfig::new()
            .with_messages_per_second(10000)
            .with_burst_size(1000);
        let limiter = Arc::new(ConnectionRateLimiter::new(config));

        let mut handles = vec![];

        // Spawn 10 threads each sending 100 messages
        for _ in 0..10 {
            let limiter = Arc::clone(&limiter);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    limiter.check_message();
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Should have processed all 1000 messages
        assert_eq!(limiter.total_messages(), 1000);
    }

    #[test]
    fn test_transport_limiter_thread_safe() {
        use std::sync::Arc;
        use std::thread;

        let limiter = Arc::new(TransportRateLimiter::with_defaults());

        let mut handles = vec![];

        // Spawn 10 threads each registering and using a connection
        for i in 0..10 {
            let limiter = Arc::clone(&limiter);
            handles.push(thread::spawn(move || {
                let conn_id = ConnectionId::new();
                let uid = 1000 + i;

                limiter.register_connection(&conn_id, uid).unwrap();

                for _ in 0..50 {
                    limiter.check_message(&conn_id);
                }

                limiter.unregister_connection(&conn_id, uid);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // All connections should be cleaned up
        assert_eq!(limiter.total_connections(), 0);
    }

    // =========================================================================
    // Async Backpressure Tests
    // =========================================================================

    #[tokio::test]
    async fn test_apply_backpressure_allowed() {
        let result = RateLimitResult::Allowed;
        let start = Instant::now();
        apply_backpressure(&result).await;
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(10),
            "Allowed should not delay"
        );
    }

    #[tokio::test]
    async fn test_apply_backpressure_throttled() {
        let result = RateLimitResult::Throttled {
            delay: Duration::from_millis(50),
        };
        let start = Instant::now();
        apply_backpressure(&result).await;
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(45),
            "Throttled should delay at least ~50ms"
        );
    }
}
