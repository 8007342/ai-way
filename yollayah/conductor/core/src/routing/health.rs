//! Model Health Tracking and Circuit Breaker
//!
//! Provides comprehensive health tracking for models including:
//! - Success/failure rate monitoring with exponential moving averages
//! - Circuit breaker pattern to prevent hammering unhealthy backends
//! - Automatic recovery detection
//! - Thread-safe state using atomic types
//!
//! # Circuit Breaker Pattern
//!
//! The circuit breaker has three states:
//!
//! ```text
//! +--------+     3 failures     +-------+     recovery_timeout     +------------+
//! | Closed | -----------------> | Open  | -----------------------> | Half-Open  |
//! +--------+                    +-------+                          +------------+
//!     ^                             ^                                    |
//!     |                             |                                    |
//!     |      3 successes            |         1 failure                  |
//!     +-----------------------------+------------------------------------+
//! ```
//!
//! - **Closed**: Normal operation, requests allowed
//! - **Open**: Circuit tripped, requests rejected immediately
//! - **Half-Open**: Testing if backend recovered, limited requests allowed
//!
//! # Thread Safety
//!
//! All state is managed using atomic types and lock-free operations where possible.
//! The `HealthTracker` uses `DashMap` for concurrent access to per-model health state.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use parking_lot::RwLock;

// ============================================================================
// Health Status
// ============================================================================

/// Health status of a model
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HealthStatus {
    /// Model is healthy and accepting requests
    Healthy,

    /// Model is degraded (high error rate but still operational)
    Degraded,

    /// Model is unhealthy (circuit breaker open)
    Unhealthy,

    /// Model is recovering (half-open circuit breaker)
    Recovering,

    /// Model status is unknown (no recent data)
    #[default]
    Unknown,
}

impl HealthStatus {
    /// Check if requests should be allowed
    #[must_use]
    pub fn allows_requests(&self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded | Self::Recovering)
    }

    /// Check if the model is considered operational
    #[must_use]
    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded)
    }
}

// ============================================================================
// Circuit Breaker State
// ============================================================================

/// Circuit breaker state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CircuitState {
    /// Circuit is closed (normal operation)
    #[default]
    Closed,

    /// Circuit is open (rejecting requests)
    Open,

    /// Circuit is half-open (testing recovery)
    HalfOpen,
}

// ============================================================================
// Health Configuration
// ============================================================================

/// Configuration for health tracking behavior
#[derive(Clone, Debug)]
pub struct HealthConfig {
    /// Number of consecutive failures before marking unhealthy
    pub failure_threshold: u32,

    /// Number of consecutive successes to recover from unhealthy
    pub success_threshold: u32,

    /// Time to wait before attempting recovery (circuit breaker open duration)
    pub recovery_timeout: Duration,

    /// Error rate threshold for degraded status (0.0 - 1.0)
    pub degraded_error_rate: f64,

    /// Error rate threshold for unhealthy status (0.0 - 1.0)
    pub unhealthy_error_rate: f64,

    /// Alpha value for exponential moving average (0.0 - 1.0)
    /// Higher values weight recent samples more heavily
    pub ema_alpha: f64,

    /// Window duration for calculating rates
    pub rate_window: Duration,

    /// Maximum requests allowed during half-open state
    pub half_open_max_requests: u32,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 3,
            success_threshold: 3,
            recovery_timeout: Duration::from_secs(30),
            degraded_error_rate: 0.1,  // 10% error rate = degraded
            unhealthy_error_rate: 0.5, // 50% error rate = unhealthy
            ema_alpha: 0.3,
            rate_window: Duration::from_secs(60),
            half_open_max_requests: 3,
        }
    }
}

// ============================================================================
// Model Health
// ============================================================================

/// Health state for a single model
///
/// Uses atomic types for thread-safe, lock-free updates where possible.
pub struct ModelHealth {
    /// Model identifier
    pub model_id: String,

    /// Health configuration
    config: HealthConfig,

    /// Current circuit breaker state (stored as u8 for atomic access)
    /// 0 = Closed, 1 = Open, 2 = `HalfOpen`
    circuit_state: AtomicU32,

    /// Consecutive failure count
    consecutive_failures: AtomicU32,

    /// Consecutive success count (for recovery)
    consecutive_successes: AtomicU32,

    /// Total request count (wraps at `u64::MAX`)
    total_requests: AtomicU64,

    /// Total success count
    total_successes: AtomicU64,

    /// Total failure count
    total_failures: AtomicU64,

    /// Error rate (stored as fixed-point: value * 10000)
    error_rate_fp: AtomicU32,

    /// Average response time in milliseconds
    avg_response_time_ms: AtomicU64,

    /// Whether model is marked as healthy
    is_healthy: AtomicBool,

    /// Last successful request time (Unix timestamp millis)
    last_success_ts: AtomicU64,

    /// Last failure time (Unix timestamp millis)
    last_failure_ts: AtomicU64,

    /// Last state transition time (Unix timestamp millis)
    last_transition_ts: AtomicU64,

    /// Half-open request count
    half_open_requests: AtomicU32,

    /// Startup time for calculating durations
    startup_time: Instant,
}

impl ModelHealth {
    /// Create new health tracker for a model
    pub fn new(model_id: impl Into<String>) -> Self {
        Self::with_config(model_id, HealthConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(model_id: impl Into<String>, config: HealthConfig) -> Self {
        Self {
            model_id: model_id.into(),
            config,
            circuit_state: AtomicU32::new(0), // Closed
            consecutive_failures: AtomicU32::new(0),
            consecutive_successes: AtomicU32::new(0),
            total_requests: AtomicU64::new(0),
            total_successes: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
            error_rate_fp: AtomicU32::new(0),
            avg_response_time_ms: AtomicU64::new(1000), // Default 1s
            is_healthy: AtomicBool::new(true),
            last_success_ts: AtomicU64::new(0),
            last_failure_ts: AtomicU64::new(0),
            last_transition_ts: AtomicU64::new(0),
            half_open_requests: AtomicU32::new(0),
            startup_time: Instant::now(),
        }
    }

    /// Get current circuit breaker state
    pub fn circuit_state(&self) -> CircuitState {
        match self.circuit_state.load(Ordering::Acquire) {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Closed, // Fallback
        }
    }

    /// Get current health status
    pub fn status(&self) -> HealthStatus {
        let circuit = self.circuit_state();

        match circuit {
            CircuitState::Open => HealthStatus::Unhealthy,
            CircuitState::HalfOpen => HealthStatus::Recovering,
            CircuitState::Closed => {
                let error_rate = self.error_rate();

                if !self.is_healthy.load(Ordering::Acquire) {
                    HealthStatus::Unhealthy
                } else if error_rate >= self.config.degraded_error_rate {
                    HealthStatus::Degraded
                } else if self.total_requests.load(Ordering::Relaxed) == 0 {
                    HealthStatus::Unknown
                } else {
                    HealthStatus::Healthy
                }
            }
        }
    }

    /// Check if the model is available for requests
    pub fn is_available(&self) -> bool {
        let circuit = self.circuit_state();

        match circuit {
            CircuitState::Closed => self.is_healthy.load(Ordering::Acquire),
            CircuitState::Open => {
                // Check if we should transition to half-open
                self.maybe_transition_to_half_open()
            }
            CircuitState::HalfOpen => {
                // Allow limited requests
                let current = self.half_open_requests.load(Ordering::Acquire);
                current < self.config.half_open_max_requests
            }
        }
    }

    /// Check if circuit breaker should transition from open to half-open
    fn maybe_transition_to_half_open(&self) -> bool {
        let last_transition = self.last_transition_ts.load(Ordering::Acquire);
        let now = self.now_millis();
        let recovery_ms = self.config.recovery_timeout.as_millis() as u64;

        if now.saturating_sub(last_transition) >= recovery_ms {
            // Try to transition to half-open
            if self
                .circuit_state
                .compare_exchange(
                    1, // Open
                    2, // HalfOpen
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                self.last_transition_ts.store(now, Ordering::Release);
                self.half_open_requests.store(0, Ordering::Release);
                tracing::info!(model = %self.model_id, "Circuit breaker transitioning to half-open");
                return true;
            }
        }
        false
    }

    /// Record a successful request
    pub fn record_success(&self, response_time_ms: u64) {
        let now = self.now_millis();

        // Update counters
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_successes.fetch_add(1, Ordering::Relaxed);
        self.last_success_ts.store(now, Ordering::Release);

        // Update consecutive counts
        self.consecutive_failures.store(0, Ordering::Release);
        let successes = self.consecutive_successes.fetch_add(1, Ordering::AcqRel) + 1;

        // Update error rate using EMA
        self.update_error_rate(false);

        // Update average response time using EMA
        self.update_response_time(response_time_ms);

        // Handle circuit breaker state transitions
        let circuit = self.circuit_state();

        match circuit {
            CircuitState::HalfOpen => {
                self.half_open_requests.fetch_add(1, Ordering::Relaxed);

                // Check if we have enough successes to close the circuit
                if successes >= self.config.success_threshold {
                    self.transition_to_closed();
                }
            }
            CircuitState::Closed => {
                // Might need to recover from degraded state
                if successes >= self.config.success_threshold {
                    self.is_healthy.store(true, Ordering::Release);
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but handle gracefully
                self.maybe_transition_to_half_open();
            }
        }
    }

    /// Record a failed request
    pub fn record_failure(&self) {
        let now = self.now_millis();

        // Update counters
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_failures.fetch_add(1, Ordering::Relaxed);
        self.last_failure_ts.store(now, Ordering::Release);

        // Update consecutive counts
        self.consecutive_successes.store(0, Ordering::Release);
        let failures = self.consecutive_failures.fetch_add(1, Ordering::AcqRel) + 1;

        // Update error rate using EMA
        self.update_error_rate(true);

        // Handle circuit breaker state transitions
        let circuit = self.circuit_state();

        match circuit {
            CircuitState::Closed => {
                if failures >= self.config.failure_threshold {
                    self.transition_to_open();
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open reopens the circuit
                self.transition_to_open();
            }
            CircuitState::Open => {
                // Already open, nothing to do
            }
        }
    }

    /// Transition circuit breaker to closed state
    fn transition_to_closed(&self) {
        let now = self.now_millis();
        self.circuit_state.store(0, Ordering::Release); // Closed
        self.last_transition_ts.store(now, Ordering::Release);
        self.consecutive_failures.store(0, Ordering::Release);
        self.is_healthy.store(true, Ordering::Release);

        tracing::info!(
            model = %self.model_id,
            "Circuit breaker closed - model recovered"
        );
    }

    /// Transition circuit breaker to open state
    fn transition_to_open(&self) {
        let now = self.now_millis();
        self.circuit_state.store(1, Ordering::Release); // Open
        self.last_transition_ts.store(now, Ordering::Release);
        self.is_healthy.store(false, Ordering::Release);

        tracing::warn!(
            model = %self.model_id,
            consecutive_failures = self.consecutive_failures.load(Ordering::Relaxed),
            "Circuit breaker opened - model marked unhealthy"
        );
    }

    /// Update error rate using exponential moving average
    fn update_error_rate(&self, is_failure: bool) {
        let alpha = self.config.ema_alpha;
        let sample = if is_failure { 1.0 } else { 0.0 };

        loop {
            let current_fp = self.error_rate_fp.load(Ordering::Acquire);
            let current = f64::from(current_fp) / 10000.0;
            let new_rate = alpha * sample + (1.0 - alpha) * current;
            let new_fp = (new_rate * 10000.0) as u32;

            if self
                .error_rate_fp
                .compare_exchange(current_fp, new_fp, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                break;
            }
        }
    }

    /// Update average response time using exponential moving average
    fn update_response_time(&self, response_time_ms: u64) {
        let alpha = self.config.ema_alpha;

        loop {
            let current = self.avg_response_time_ms.load(Ordering::Acquire);
            let new_avg = (alpha * response_time_ms as f64 + (1.0 - alpha) * current as f64) as u64;

            if self
                .avg_response_time_ms
                .compare_exchange(current, new_avg, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                break;
            }
        }
    }

    /// Get current error rate (0.0 - 1.0)
    pub fn error_rate(&self) -> f64 {
        f64::from(self.error_rate_fp.load(Ordering::Acquire)) / 10000.0
    }

    /// Get average response time in milliseconds
    pub fn avg_response_time(&self) -> u64 {
        self.avg_response_time_ms.load(Ordering::Acquire)
    }

    /// Get total request count
    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::Relaxed)
    }

    /// Get total success count
    pub fn total_successes(&self) -> u64 {
        self.total_successes.load(Ordering::Relaxed)
    }

    /// Get total failure count
    pub fn total_failures(&self) -> u64 {
        self.total_failures.load(Ordering::Relaxed)
    }

    /// Get consecutive failure count
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures.load(Ordering::Acquire)
    }

    /// Get consecutive success count
    pub fn consecutive_successes(&self) -> u32 {
        self.consecutive_successes.load(Ordering::Acquire)
    }

    /// Get time since last success
    pub fn time_since_last_success(&self) -> Option<Duration> {
        let ts = self.last_success_ts.load(Ordering::Acquire);
        if ts == 0 {
            return None;
        }
        let now = self.now_millis();
        Some(Duration::from_millis(now.saturating_sub(ts)))
    }

    /// Get time since last failure
    pub fn time_since_last_failure(&self) -> Option<Duration> {
        let ts = self.last_failure_ts.load(Ordering::Acquire);
        if ts == 0 {
            return None;
        }
        let now = self.now_millis();
        Some(Duration::from_millis(now.saturating_sub(ts)))
    }

    /// Force the circuit breaker to a specific state (for testing/admin)
    pub fn force_state(&self, state: CircuitState) {
        let state_val = match state {
            CircuitState::Closed => 0,
            CircuitState::Open => 1,
            CircuitState::HalfOpen => 2,
        };
        self.circuit_state.store(state_val, Ordering::Release);
        self.last_transition_ts
            .store(self.now_millis(), Ordering::Release);

        match state {
            CircuitState::Closed => {
                self.is_healthy.store(true, Ordering::Release);
                self.consecutive_failures.store(0, Ordering::Release);
            }
            CircuitState::Open => {
                self.is_healthy.store(false, Ordering::Release);
            }
            CircuitState::HalfOpen => {
                self.half_open_requests.store(0, Ordering::Release);
            }
        }
    }

    /// Reset all health state
    pub fn reset(&self) {
        self.circuit_state.store(0, Ordering::Release);
        self.consecutive_failures.store(0, Ordering::Release);
        self.consecutive_successes.store(0, Ordering::Release);
        self.total_requests.store(0, Ordering::Release);
        self.total_successes.store(0, Ordering::Release);
        self.total_failures.store(0, Ordering::Release);
        self.error_rate_fp.store(0, Ordering::Release);
        self.avg_response_time_ms.store(1000, Ordering::Release);
        self.is_healthy.store(true, Ordering::Release);
        self.last_success_ts.store(0, Ordering::Release);
        self.last_failure_ts.store(0, Ordering::Release);
        self.last_transition_ts.store(0, Ordering::Release);
        self.half_open_requests.store(0, Ordering::Release);
    }

    /// Get current timestamp in milliseconds since startup
    /// Returns at least 1 to ensure 0 can be used as a sentinel for "never set"
    fn now_millis(&self) -> u64 {
        self.startup_time.elapsed().as_millis() as u64 + 1
    }

    /// Get a snapshot of the current health state
    pub fn snapshot(&self) -> HealthSnapshot {
        HealthSnapshot {
            model_id: self.model_id.clone(),
            status: self.status(),
            circuit_state: self.circuit_state(),
            is_available: self.is_available(),
            error_rate: self.error_rate(),
            avg_response_time_ms: self.avg_response_time(),
            total_requests: self.total_requests(),
            total_successes: self.total_successes(),
            total_failures: self.total_failures(),
            consecutive_failures: self.consecutive_failures(),
            consecutive_successes: self.consecutive_successes(),
            time_since_last_success: self.time_since_last_success(),
            time_since_last_failure: self.time_since_last_failure(),
        }
    }
}

impl std::fmt::Debug for ModelHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelHealth")
            .field("model_id", &self.model_id)
            .field("status", &self.status())
            .field("circuit_state", &self.circuit_state())
            .field("error_rate", &self.error_rate())
            .field("consecutive_failures", &self.consecutive_failures())
            .finish()
    }
}

// ============================================================================
// Health Snapshot
// ============================================================================

/// Immutable snapshot of health state at a point in time
#[derive(Clone, Debug)]
pub struct HealthSnapshot {
    /// Model identifier
    pub model_id: String,

    /// Current health status
    pub status: HealthStatus,

    /// Circuit breaker state
    pub circuit_state: CircuitState,

    /// Whether model is available for requests
    pub is_available: bool,

    /// Current error rate (0.0 - 1.0)
    pub error_rate: f64,

    /// Average response time in milliseconds
    pub avg_response_time_ms: u64,

    /// Total request count
    pub total_requests: u64,

    /// Total success count
    pub total_successes: u64,

    /// Total failure count
    pub total_failures: u64,

    /// Consecutive failure count
    pub consecutive_failures: u32,

    /// Consecutive success count
    pub consecutive_successes: u32,

    /// Time since last successful request
    pub time_since_last_success: Option<Duration>,

    /// Time since last failed request
    pub time_since_last_failure: Option<Duration>,
}

impl HealthSnapshot {
    /// Calculate success rate (0.0 - 1.0)
    #[must_use]
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            1.0
        } else {
            self.total_successes as f64 / self.total_requests as f64
        }
    }
}

// ============================================================================
// Health Tracker
// ============================================================================

/// Aggregate health tracker for all models
///
/// Provides a centralized view of health state across all registered models.
/// Thread-safe for concurrent access from multiple request handlers.
pub struct HealthTracker {
    /// Per-model health state
    models: DashMap<String, Arc<ModelHealth>>,

    /// Default configuration for new models
    default_config: RwLock<HealthConfig>,

    /// Global health status (any model unhealthy = degraded)
    global_healthy: AtomicBool,
}

impl HealthTracker {
    /// Create a new health tracker
    #[must_use]
    pub fn new() -> Self {
        Self {
            models: DashMap::new(),
            default_config: RwLock::new(HealthConfig::default()),
            global_healthy: AtomicBool::new(true),
        }
    }

    /// Create with custom default configuration
    #[must_use]
    pub fn with_config(config: HealthConfig) -> Self {
        Self {
            models: DashMap::new(),
            default_config: RwLock::new(config),
            global_healthy: AtomicBool::new(true),
        }
    }

    /// Set default configuration for new models
    pub fn set_default_config(&self, config: HealthConfig) {
        let mut default = self.default_config.write();
        *default = config;
    }

    /// Register a new model with default config
    pub fn register(&self, model_id: impl Into<String>) -> Arc<ModelHealth> {
        let model_id = model_id.into();
        let config = self.default_config.read().clone();
        let health = Arc::new(ModelHealth::with_config(model_id.clone(), config));
        self.models.insert(model_id, health.clone());
        health
    }

    /// Register a model with custom config
    pub fn register_with_config(
        &self,
        model_id: impl Into<String>,
        config: HealthConfig,
    ) -> Arc<ModelHealth> {
        let model_id = model_id.into();
        let health = Arc::new(ModelHealth::with_config(model_id.clone(), config));
        self.models.insert(model_id, health.clone());
        health
    }

    /// Get health state for a model
    pub fn get(&self, model_id: &str) -> Option<Arc<ModelHealth>> {
        self.models.get(model_id).map(|h| h.clone())
    }

    /// Get or create health state for a model
    pub fn get_or_create(&self, model_id: impl Into<String>) -> Arc<ModelHealth> {
        let model_id = model_id.into();
        self.models
            .entry(model_id.clone())
            .or_insert_with(|| {
                let config = self.default_config.read().clone();
                Arc::new(ModelHealth::with_config(model_id, config))
            })
            .clone()
    }

    /// Record a successful request for a model
    pub fn record_success(&self, model_id: &str, response_time_ms: u64) {
        if let Some(health) = self.get(model_id) {
            health.record_success(response_time_ms);
            self.update_global_health();
        }
    }

    /// Record a failed request for a model
    pub fn record_failure(&self, model_id: &str) {
        if let Some(health) = self.get(model_id) {
            health.record_failure();
            self.update_global_health();
        }
    }

    /// Check if a model is available
    pub fn is_available(&self, model_id: &str) -> bool {
        self.get(model_id).is_some_and(|h| h.is_available())
    }

    /// Check if a model is healthy
    pub fn is_healthy(&self, model_id: &str) -> bool {
        self.get(model_id)
            .is_some_and(|h| h.status().is_operational())
    }

    /// Get status for a model
    pub fn status(&self, model_id: &str) -> HealthStatus {
        self.get(model_id)
            .map_or(HealthStatus::Unknown, |h| h.status())
    }

    /// Get all healthy models
    pub fn healthy_models(&self) -> Vec<String> {
        self.models
            .iter()
            .filter(|e| e.value().status().is_operational())
            .map(|e| e.key().clone())
            .collect()
    }

    /// Get all available models (accepts requests)
    pub fn available_models(&self) -> Vec<String> {
        self.models
            .iter()
            .filter(|e| e.value().is_available())
            .map(|e| e.key().clone())
            .collect()
    }

    /// Get all unhealthy models
    pub fn unhealthy_models(&self) -> Vec<String> {
        self.models
            .iter()
            .filter(|e| !e.value().status().is_operational())
            .map(|e| e.key().clone())
            .collect()
    }

    /// Get snapshots for all models
    pub fn all_snapshots(&self) -> Vec<HealthSnapshot> {
        self.models.iter().map(|e| e.value().snapshot()).collect()
    }

    /// Get snapshot for a specific model
    pub fn snapshot(&self, model_id: &str) -> Option<HealthSnapshot> {
        self.get(model_id).map(|h| h.snapshot())
    }

    /// Check if globally healthy (all models operational)
    pub fn is_globally_healthy(&self) -> bool {
        self.global_healthy.load(Ordering::Acquire)
    }

    /// Update global health status
    fn update_global_health(&self) {
        let all_healthy = self
            .models
            .iter()
            .all(|e| e.value().status().is_operational());
        self.global_healthy.store(all_healthy, Ordering::Release);
    }

    /// Remove a model from tracking
    pub fn remove(&self, model_id: &str) {
        self.models.remove(model_id);
        self.update_global_health();
    }

    /// Clear all health state
    pub fn clear(&self) {
        self.models.clear();
        self.global_healthy.store(true, Ordering::Release);
    }

    /// Get the number of tracked models
    pub fn model_count(&self) -> usize {
        self.models.len()
    }

    /// Get aggregate statistics
    pub fn aggregate_stats(&self) -> AggregateHealthStats {
        let models: Vec<_> = self.models.iter().collect();
        let total = models.len();

        if total == 0 {
            return AggregateHealthStats::default();
        }

        let healthy = models
            .iter()
            .filter(|e| e.value().status() == HealthStatus::Healthy)
            .count();
        let degraded = models
            .iter()
            .filter(|e| e.value().status() == HealthStatus::Degraded)
            .count();
        let unhealthy = models
            .iter()
            .filter(|e| e.value().status() == HealthStatus::Unhealthy)
            .count();
        let recovering = models
            .iter()
            .filter(|e| e.value().status() == HealthStatus::Recovering)
            .count();

        let total_requests: u64 = models.iter().map(|e| e.value().total_requests()).sum();
        let total_failures: u64 = models.iter().map(|e| e.value().total_failures()).sum();
        let avg_error_rate = if total > 0 {
            models.iter().map(|e| e.value().error_rate()).sum::<f64>() / total as f64
        } else {
            0.0
        };

        AggregateHealthStats {
            total_models: total,
            healthy_count: healthy,
            degraded_count: degraded,
            unhealthy_count: unhealthy,
            recovering_count: recovering,
            total_requests,
            total_failures,
            average_error_rate: avg_error_rate,
            global_healthy: self.is_globally_healthy(),
        }
    }
}

impl Default for HealthTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Aggregate Stats
// ============================================================================

/// Aggregate health statistics across all models
#[derive(Clone, Debug, Default)]
pub struct AggregateHealthStats {
    /// Total number of tracked models
    pub total_models: usize,

    /// Number of healthy models
    pub healthy_count: usize,

    /// Number of degraded models
    pub degraded_count: usize,

    /// Number of unhealthy models
    pub unhealthy_count: usize,

    /// Number of recovering models
    pub recovering_count: usize,

    /// Total requests across all models
    pub total_requests: u64,

    /// Total failures across all models
    pub total_failures: u64,

    /// Average error rate across all models
    pub average_error_rate: f64,

    /// Whether all models are healthy
    pub global_healthy: bool,
}

impl AggregateHealthStats {
    /// Calculate overall health percentage
    #[must_use]
    pub fn health_percentage(&self) -> f64 {
        if self.total_models == 0 {
            100.0
        } else {
            (self.healthy_count as f64 / self.total_models as f64) * 100.0
        }
    }

    /// Calculate overall availability percentage
    #[must_use]
    pub fn availability_percentage(&self) -> f64 {
        if self.total_models == 0 {
            100.0
        } else {
            let available = self.healthy_count + self.degraded_count + self.recovering_count;
            (available as f64 / self.total_models as f64) * 100.0
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_allows_requests() {
        assert!(HealthStatus::Healthy.allows_requests());
        assert!(HealthStatus::Degraded.allows_requests());
        assert!(HealthStatus::Recovering.allows_requests());
        assert!(!HealthStatus::Unhealthy.allows_requests());
        assert!(!HealthStatus::Unknown.allows_requests());
    }

    #[test]
    fn test_model_health_initial_state() {
        let health = ModelHealth::new("test-model");

        assert_eq!(health.circuit_state(), CircuitState::Closed);
        assert!(health.is_available());
        assert_eq!(health.consecutive_failures(), 0);
        assert_eq!(health.total_requests(), 0);
    }

    #[test]
    fn test_model_health_record_success() {
        let health = ModelHealth::new("test-model");

        health.record_success(100);

        assert_eq!(health.total_requests(), 1);
        assert_eq!(health.total_successes(), 1);
        assert_eq!(health.total_failures(), 0);
        assert_eq!(health.consecutive_successes(), 1);
        assert!(health.time_since_last_success().is_some());
    }

    #[test]
    fn test_model_health_record_failure() {
        let health = ModelHealth::new("test-model");

        health.record_failure();

        assert_eq!(health.total_requests(), 1);
        assert_eq!(health.total_failures(), 1);
        assert_eq!(health.consecutive_failures(), 1);
        assert!(health.time_since_last_failure().is_some());
    }

    #[test]
    fn test_circuit_breaker_opens_after_threshold() {
        let config = HealthConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let health = ModelHealth::with_config("test-model", config);

        // First two failures - circuit stays closed
        health.record_failure();
        assert_eq!(health.circuit_state(), CircuitState::Closed);
        health.record_failure();
        assert_eq!(health.circuit_state(), CircuitState::Closed);

        // Third failure - circuit opens
        health.record_failure();
        assert_eq!(health.circuit_state(), CircuitState::Open);
        assert!(!health.is_available());
        assert_eq!(health.status(), HealthStatus::Unhealthy);
    }

    #[test]
    fn test_success_resets_consecutive_failures() {
        let health = ModelHealth::new("test-model");

        health.record_failure();
        health.record_failure();
        assert_eq!(health.consecutive_failures(), 2);

        health.record_success(100);
        assert_eq!(health.consecutive_failures(), 0);
        assert_eq!(health.consecutive_successes(), 1);
    }

    #[test]
    fn test_failure_resets_consecutive_successes() {
        let health = ModelHealth::new("test-model");

        health.record_success(100);
        health.record_success(100);
        assert_eq!(health.consecutive_successes(), 2);

        health.record_failure();
        assert_eq!(health.consecutive_successes(), 0);
        assert_eq!(health.consecutive_failures(), 1);
    }

    #[test]
    fn test_circuit_breaker_recovery() {
        let config = HealthConfig {
            failure_threshold: 3,
            success_threshold: 3,
            recovery_timeout: Duration::from_millis(10), // Short for testing
            ..Default::default()
        };
        let health = ModelHealth::with_config("test-model", config);

        // Open the circuit
        health.record_failure();
        health.record_failure();
        health.record_failure();
        assert_eq!(health.circuit_state(), CircuitState::Open);

        // Wait for recovery timeout
        std::thread::sleep(Duration::from_millis(20));

        // Should transition to half-open on availability check
        assert!(health.is_available());
        assert_eq!(health.circuit_state(), CircuitState::HalfOpen);

        // Successes in half-open state close the circuit
        health.record_success(100);
        health.record_success(100);
        health.record_success(100);
        assert_eq!(health.circuit_state(), CircuitState::Closed);
        assert!(health.is_available());
    }

    #[test]
    fn test_half_open_failure_reopens() {
        let config = HealthConfig {
            failure_threshold: 3,
            recovery_timeout: Duration::from_millis(10),
            ..Default::default()
        };
        let health = ModelHealth::with_config("test-model", config);

        // Open the circuit
        health.record_failure();
        health.record_failure();
        health.record_failure();

        // Wait and transition to half-open
        std::thread::sleep(Duration::from_millis(20));
        assert!(health.is_available());
        assert_eq!(health.circuit_state(), CircuitState::HalfOpen);

        // Failure in half-open reopens immediately
        health.record_failure();
        assert_eq!(health.circuit_state(), CircuitState::Open);
    }

    #[test]
    fn test_error_rate_ema() {
        let config = HealthConfig {
            ema_alpha: 0.5, // 50% weight on new samples
            ..Default::default()
        };
        let health = ModelHealth::with_config("test-model", config);

        // Record some failures
        health.record_failure();
        let rate1 = health.error_rate();
        assert!(rate1 > 0.0);

        // Record a success
        health.record_success(100);
        let rate2 = health.error_rate();
        assert!(rate2 < rate1); // Should decrease

        // Record more successes
        health.record_success(100);
        health.record_success(100);
        let rate3 = health.error_rate();
        assert!(rate3 < rate2); // Should keep decreasing
    }

    #[test]
    fn test_force_state() {
        let health = ModelHealth::new("test-model");

        health.force_state(CircuitState::Open);
        assert_eq!(health.circuit_state(), CircuitState::Open);
        assert!(!health.is_available());

        health.force_state(CircuitState::HalfOpen);
        assert_eq!(health.circuit_state(), CircuitState::HalfOpen);

        health.force_state(CircuitState::Closed);
        assert_eq!(health.circuit_state(), CircuitState::Closed);
        assert!(health.is_available());
    }

    #[test]
    fn test_reset() {
        let health = ModelHealth::new("test-model");

        // Record some activity
        health.record_failure();
        health.record_failure();
        health.record_failure();
        health.record_success(100);

        // Reset
        health.reset();

        assert_eq!(health.circuit_state(), CircuitState::Closed);
        assert_eq!(health.consecutive_failures(), 0);
        assert_eq!(health.total_requests(), 0);
        assert!(health.is_available());
    }

    #[test]
    fn test_snapshot() {
        let health = ModelHealth::new("test-model");
        health.record_success(200);
        health.record_failure();

        let snapshot = health.snapshot();

        assert_eq!(snapshot.model_id, "test-model");
        assert_eq!(snapshot.total_requests, 2);
        assert_eq!(snapshot.total_successes, 1);
        assert_eq!(snapshot.total_failures, 1);
        assert_eq!(snapshot.consecutive_failures, 1);
    }

    #[test]
    fn test_health_tracker_register() {
        let tracker = HealthTracker::new();

        tracker.register("model-a");
        tracker.register("model-b");

        assert_eq!(tracker.model_count(), 2);
        assert!(tracker.get("model-a").is_some());
        assert!(tracker.get("model-b").is_some());
        assert!(tracker.get("model-c").is_none());
    }

    #[test]
    fn test_health_tracker_record() {
        let tracker = HealthTracker::new();
        tracker.register("test-model");

        tracker.record_success("test-model", 100);
        tracker.record_failure("test-model");

        let snapshot = tracker.snapshot("test-model").unwrap();
        assert_eq!(snapshot.total_requests, 2);
    }

    #[test]
    fn test_health_tracker_available_models() {
        let config = HealthConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let tracker = HealthTracker::with_config(config);

        tracker.register("healthy");
        tracker.register("unhealthy");

        // Make one model unhealthy
        tracker.record_failure("unhealthy");
        tracker.record_failure("unhealthy");
        tracker.record_failure("unhealthy");

        let available = tracker.available_models();
        assert!(available.contains(&"healthy".to_string()));
        assert!(!available.contains(&"unhealthy".to_string()));

        let unhealthy = tracker.unhealthy_models();
        assert!(unhealthy.contains(&"unhealthy".to_string()));
    }

    #[test]
    fn test_health_tracker_aggregate_stats() {
        let config = HealthConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let tracker = HealthTracker::with_config(config);

        tracker.register("model-1");
        tracker.register("model-2");
        tracker.register("model-3");

        // Record some activity
        tracker.record_success("model-1", 100);
        tracker.record_success("model-2", 100);
        tracker.record_failure("model-3");
        tracker.record_failure("model-3");
        tracker.record_failure("model-3"); // Makes unhealthy

        let stats = tracker.aggregate_stats();

        assert_eq!(stats.total_models, 3);
        assert_eq!(stats.unhealthy_count, 1);
        assert_eq!(stats.total_requests, 5);
        assert_eq!(stats.total_failures, 3);
        assert!(!stats.global_healthy);
    }

    #[test]
    fn test_health_tracker_get_or_create() {
        let tracker = HealthTracker::new();

        // First call creates
        let health1 = tracker.get_or_create("model");
        assert_eq!(tracker.model_count(), 1);

        // Second call returns same
        let health2 = tracker.get_or_create("model");
        assert_eq!(tracker.model_count(), 1);

        // Both should point to same data
        health1.record_success(100);
        assert_eq!(health2.total_requests(), 1);
    }

    #[test]
    fn test_health_tracker_clear() {
        let tracker = HealthTracker::new();

        tracker.register("model-1");
        tracker.register("model-2");

        tracker.clear();

        assert_eq!(tracker.model_count(), 0);
        assert!(tracker.is_globally_healthy());
    }

    #[test]
    fn test_aggregate_stats_percentages() {
        let mut stats = AggregateHealthStats {
            total_models: 10,
            healthy_count: 7,
            degraded_count: 2,
            unhealthy_count: 1,
            recovering_count: 0,
            ..Default::default()
        };

        assert_eq!(stats.health_percentage(), 70.0);
        assert_eq!(stats.availability_percentage(), 90.0);

        // Edge case: no models
        stats.total_models = 0;
        assert_eq!(stats.health_percentage(), 100.0);
        assert_eq!(stats.availability_percentage(), 100.0);
    }

    #[test]
    fn test_half_open_request_limit() {
        let config = HealthConfig {
            failure_threshold: 3,
            recovery_timeout: Duration::from_millis(10),
            half_open_max_requests: 2,
            ..Default::default()
        };
        let health = ModelHealth::with_config("test-model", config);

        // Open the circuit
        health.record_failure();
        health.record_failure();
        health.record_failure();

        // Wait for recovery
        std::thread::sleep(Duration::from_millis(20));

        // First request should be allowed (transitions to half-open)
        assert!(health.is_available());

        // Record requests to use up the limit
        health.record_success(100);
        health.record_success(100);

        // Third request while still in half-open should be blocked
        // (we need success_threshold = 3 but only got 2)
        // Actually with success_threshold default of 3, circuit stays half-open
        // and we've used 2 of 2 allowed requests
        let half_open_count = health.half_open_requests.load(Ordering::Acquire);
        assert_eq!(half_open_count, 2);
    }

    #[test]
    fn test_degraded_status() {
        let config = HealthConfig {
            degraded_error_rate: 0.1, // 10% triggers degraded
            ema_alpha: 1.0,           // Use exact values (no smoothing)
            ..Default::default()
        };
        let health = ModelHealth::with_config("test-model", config);

        // Record 9 successes and 1 failure = 10% error rate
        for _ in 0..9 {
            health.record_success(100);
        }
        health.record_failure();

        assert_eq!(health.status(), HealthStatus::Degraded);
    }

    #[test]
    fn test_concurrent_updates() {
        use std::sync::Arc;
        use std::thread;

        let health = Arc::new(ModelHealth::new("test-model"));
        let mut handles = vec![];

        // Spawn multiple threads recording successes and failures
        for i in 0..10 {
            let h = health.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    if i % 2 == 0 {
                        h.record_success(100);
                    } else {
                        h.record_failure();
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Should have recorded 1000 total requests
        assert_eq!(health.total_requests(), 1000);
        assert_eq!(health.total_successes() + health.total_failures(), 1000);
    }

    #[test]
    fn test_snapshot_success_rate() {
        let health = ModelHealth::new("test-model");

        // 80% success rate
        for _ in 0..8 {
            health.record_success(100);
        }
        for _ in 0..2 {
            health.record_failure();
        }

        let snapshot = health.snapshot();
        assert!((snapshot.success_rate() - 0.8).abs() < 0.01);
    }
}
