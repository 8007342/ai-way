//! Routing Configuration
//!
//! Configuration types for model routing, resource limits, and backend settings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

// ============================================================================
// Task Classification
// ============================================================================

/// Classification of task types for routing decisions
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskClass {
    /// Quick response needed (<100ms first token)
    /// Examples: autocomplete, quick answers, greetings
    QuickResponse,

    /// Deep thinking allowed (5-30s total time OK)
    /// Examples: complex analysis, multi-step reasoning
    DeepThinking,

    /// Code generation tasks (moderate latency acceptable)
    /// Examples: writing code, refactoring, code review
    CodeGeneration,

    /// Mathematical/logical tasks (precision over speed)
    /// Examples: proofs, calculations, formal verification
    Mathematical,

    /// Creative/generative tasks
    /// Examples: stories, brainstorming, ideation
    Creative,

    /// Tool use and function calling
    /// Examples: API calls, structured output generation
    ToolUse,

    /// Embedding/retrieval tasks
    /// Examples: semantic search, similarity matching
    Embedding,

    /// Default/unclassified tasks
    General,
}

impl Default for TaskClass {
    fn default() -> Self {
        Self::General
    }
}

impl TaskClass {
    /// Get the default latency budget for this task class
    #[must_use]
    pub fn default_timeout(&self) -> Duration {
        match self {
            Self::QuickResponse => Duration::from_millis(5_000),    // 5s max
            Self::DeepThinking => Duration::from_secs(120),         // 2 min
            Self::CodeGeneration => Duration::from_secs(60),        // 1 min
            Self::Mathematical => Duration::from_secs(90),          // 1.5 min
            Self::Creative => Duration::from_secs(60),              // 1 min
            Self::ToolUse => Duration::from_secs(30),               // 30s
            Self::Embedding => Duration::from_secs(10),             // 10s
            Self::General => Duration::from_secs(60),               // 1 min
        }
    }

    /// Target time to first token
    #[must_use]
    pub fn target_ttft(&self) -> Duration {
        match self {
            Self::QuickResponse => Duration::from_millis(100),      // <100ms
            Self::DeepThinking => Duration::from_secs(5),           // Up to 5s
            Self::CodeGeneration => Duration::from_secs(2),         // Up to 2s
            Self::Mathematical => Duration::from_secs(3),           // Up to 3s
            Self::Creative => Duration::from_secs(1),               // Up to 1s
            Self::ToolUse => Duration::from_millis(500),            // <500ms
            Self::Embedding => Duration::from_millis(200),          // <200ms
            Self::General => Duration::from_secs(1),                // Up to 1s
        }
    }

    /// Priority level (higher = more important for scheduling)
    #[must_use]
    pub fn priority(&self) -> u8 {
        match self {
            Self::QuickResponse => 100,  // Highest priority
            Self::ToolUse => 90,
            Self::Embedding => 85,
            Self::CodeGeneration => 70,
            Self::General => 50,
            Self::Creative => 40,
            Self::Mathematical => 30,    // Can wait for precision
            Self::DeepThinking => 20,    // Lowest priority, long-running
        }
    }

    /// Whether this task class allows preemption by higher priority tasks
    #[must_use]
    pub fn preemptible(&self) -> bool {
        match self {
            Self::QuickResponse => false,  // Already short
            Self::DeepThinking => true,    // Long-running, can pause
            Self::CodeGeneration => false,
            Self::Mathematical => false,   // Need consistency
            Self::Creative => true,
            Self::ToolUse => false,
            Self::Embedding => false,
            Self::General => true,
        }
    }
}

// ============================================================================
// Model Profiles
// ============================================================================

/// Profile describing a model's characteristics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelProfile {
    /// Model identifier
    pub model_id: String,

    /// Human-readable name
    pub display_name: String,

    /// Backend this model runs on
    pub backend_id: String,

    /// Task classes this model excels at
    pub strengths: Vec<TaskClass>,

    /// Task classes this model struggles with
    pub weaknesses: Vec<TaskClass>,

    /// Average time to first token (observed)
    pub avg_ttft_ms: u64,

    /// Average tokens per second (observed)
    pub avg_tokens_per_sec: f32,

    /// Maximum context window (tokens)
    pub max_context: u32,

    /// Estimated memory usage (bytes, for local models)
    pub memory_bytes: Option<u64>,

    /// Whether this model supports streaming
    pub supports_streaming: bool,

    /// Whether this model supports tool/function calling
    pub supports_tools: bool,

    /// Cost tier (for billing/budgeting)
    pub cost_tier: CostTier,

    /// Whether the model needs to be preloaded
    pub requires_preload: bool,

    /// Preload priority (higher = load earlier)
    pub preload_priority: u8,
}

impl ModelProfile {
    /// Create a new model profile
    pub fn new(model_id: impl Into<String>, backend_id: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            display_name: String::new(),
            backend_id: backend_id.into(),
            strengths: Vec::new(),
            weaknesses: Vec::new(),
            avg_ttft_ms: 1000,
            avg_tokens_per_sec: 20.0,
            max_context: 4096,
            memory_bytes: None,
            supports_streaming: true,
            supports_tools: false,
            cost_tier: CostTier::Free,
            requires_preload: false,
            preload_priority: 50,
        }
    }

    /// Calculate affinity score for a task class (0.0 - 1.0)
    #[must_use]
    pub fn affinity_for(&self, task_class: TaskClass) -> f32 {
        if self.strengths.contains(&task_class) {
            1.0
        } else if self.weaknesses.contains(&task_class) {
            0.2
        } else {
            0.5
        }
    }

    /// Check if model can meet latency requirements for task class
    #[must_use]
    pub fn can_meet_latency(&self, task_class: TaskClass) -> bool {
        let target = task_class.target_ttft();
        Duration::from_millis(self.avg_ttft_ms) <= target
    }
}

/// Cost tier for models (for budget management)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CostTier {
    /// Free (local models, free APIs)
    Free,
    /// Low cost
    Low,
    /// Medium cost
    Medium,
    /// High cost (premium models)
    High,
    /// Enterprise/unlimited
    Enterprise,
}

// ============================================================================
// Backend Configuration
// ============================================================================

/// Configuration for a backend endpoint
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackendConfig {
    /// Unique backend identifier
    pub id: String,

    /// Backend type
    pub backend_type: BackendType,

    /// Connection settings
    pub connection: ConnectionConfig,

    /// Rate limiting settings
    pub rate_limits: RateLimitConfig,

    /// Resource constraints
    pub resources: ResourceConfig,

    /// Retry configuration
    pub retry: RetryConfig,

    /// Whether this backend is enabled
    pub enabled: bool,

    /// Priority for fallback ordering
    pub fallback_priority: u8,
}

/// Types of backend connections
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BackendType {
    /// Local Ollama instance
    Ollama {
        host: String,
        port: u16,
    },
    /// OpenAI-compatible API
    OpenAI {
        base_url: String,
        api_key_env: String,
    },
    /// Anthropic API
    Anthropic {
        api_key_env: String,
    },
    /// Local model via GGML/llama.cpp
    LocalGgml {
        model_path: String,
        gpu_layers: u32,
    },
    /// gRPC-based backend (vLLM, TensorRT-LLM)
    Grpc {
        endpoint: String,
        use_tls: bool,
    },
    /// Custom HTTP backend
    CustomHttp {
        base_url: String,
        auth_header: Option<String>,
    },
}

/// Connection configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Connection timeout
    pub connect_timeout_ms: u64,

    /// Read timeout per chunk
    pub read_timeout_ms: u64,

    /// Write timeout
    pub write_timeout_ms: u64,

    /// Keep-alive interval
    pub keepalive_interval_ms: u64,

    /// Maximum idle connections in pool
    pub max_idle_connections: usize,

    /// Maximum total connections
    pub max_connections: usize,

    /// Enable HTTP/2
    pub use_http2: bool,

    /// Enable connection pooling
    pub enable_pooling: bool,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            connect_timeout_ms: 5_000,        // 5s connect timeout
            read_timeout_ms: 120_000,         // 2min read timeout (for long generations)
            write_timeout_ms: 30_000,         // 30s write timeout
            keepalive_interval_ms: 30_000,    // 30s keepalive
            max_idle_connections: 4,
            max_connections: 16,
            use_http2: true,
            enable_pooling: true,
        }
    }
}

impl ConnectionConfig {
    /// Convert to Duration for connect timeout
    pub fn connect_timeout(&self) -> Duration {
        Duration::from_millis(self.connect_timeout_ms)
    }

    /// Convert to Duration for read timeout
    pub fn read_timeout(&self) -> Duration {
        Duration::from_millis(self.read_timeout_ms)
    }
}

/// Rate limiting configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per second
    pub requests_per_second: f32,

    /// Maximum concurrent requests
    pub max_concurrent: usize,

    /// Token bucket capacity (for burst handling)
    pub burst_capacity: usize,

    /// Maximum tokens per minute (for APIs with token limits)
    pub tokens_per_minute: Option<u64>,

    /// Whether to queue requests when rate limited
    pub queue_when_limited: bool,

    /// Maximum queue size
    pub max_queue_size: usize,

    /// Queue timeout
    pub queue_timeout_ms: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 10.0,
            max_concurrent: 8,
            burst_capacity: 20,
            tokens_per_minute: None,
            queue_when_limited: true,
            max_queue_size: 100,
            queue_timeout_ms: 30_000,
        }
    }
}

/// Resource constraints (primarily for local models)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceConfig {
    /// Maximum GPU memory to use (bytes)
    pub max_gpu_memory_bytes: Option<u64>,

    /// Maximum CPU memory to use (bytes)
    pub max_cpu_memory_bytes: Option<u64>,

    /// Maximum concurrent model loads
    pub max_concurrent_loads: usize,

    /// Idle timeout before unloading (for local models)
    pub idle_unload_timeout_ms: u64,

    /// Whether to allow memory oversubscription
    pub allow_oversubscription: bool,

    /// Memory pressure threshold for eviction (0.0 - 1.0)
    pub memory_pressure_threshold: f32,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            max_gpu_memory_bytes: None,
            max_cpu_memory_bytes: None,
            max_concurrent_loads: 2,
            idle_unload_timeout_ms: 300_000,  // 5 minutes
            allow_oversubscription: false,
            memory_pressure_threshold: 0.85,
        }
    }
}

// ============================================================================
// Retry Configuration
// ============================================================================

/// Retry configuration for failed requests
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: u32,

    /// Initial backoff delay
    pub initial_backoff_ms: u64,

    /// Maximum backoff delay
    pub max_backoff_ms: u64,

    /// Backoff multiplier
    pub backoff_multiplier: f32,

    /// Add jitter to backoff
    pub use_jitter: bool,

    /// Retry on these status codes
    pub retry_status_codes: Vec<u16>,

    /// Retry on connection errors
    pub retry_on_connection_error: bool,

    /// Retry on timeout
    pub retry_on_timeout: bool,

    /// Whether to try a different backend on failure
    pub fallback_on_failure: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 10_000,
            backoff_multiplier: 2.0,
            use_jitter: true,
            retry_status_codes: vec![429, 500, 502, 503, 504],
            retry_on_connection_error: true,
            retry_on_timeout: true,
            fallback_on_failure: true,
        }
    }
}

impl RetryConfig {
    /// Calculate backoff duration for attempt N (0-indexed)
    pub fn backoff_for_attempt(&self, attempt: u32) -> Duration {
        let base = self.initial_backoff_ms as f64 * self.backoff_multiplier.powi(attempt as i32) as f64;
        let capped = base.min(self.max_backoff_ms as f64);

        let duration_ms = if self.use_jitter {
            // Add up to 25% jitter
            let jitter = rand::random::<f64>() * 0.25;
            (capped * (1.0 + jitter)) as u64
        } else {
            capped as u64
        };

        Duration::from_millis(duration_ms)
    }

    /// Check if a status code should trigger a retry
    pub fn should_retry_status(&self, status: u16) -> bool {
        self.retry_status_codes.contains(&status)
    }
}

// ============================================================================
// Full Router Configuration
// ============================================================================

/// Complete router configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouterConfig {
    /// Available backends
    pub backends: Vec<BackendConfig>,

    /// Model profiles
    pub models: Vec<ModelProfile>,

    /// Default model for each task class
    pub default_models: HashMap<TaskClass, String>,

    /// Fallback chains: model -> ordered list of fallbacks
    pub fallback_chains: HashMap<String, Vec<String>>,

    /// Global rate limits
    pub global_rate_limits: RateLimitConfig,

    /// Models to preload on startup
    pub preload_models: Vec<String>,

    /// Whether to enable request queuing
    pub enable_queue: bool,

    /// Maximum queue depth
    pub max_queue_depth: usize,

    /// Queue ordering strategy
    pub queue_strategy: QueueStrategy,

    /// Health check interval
    pub health_check_interval_ms: u64,

    /// Metrics collection settings
    pub metrics: MetricsConfig,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            backends: Vec::new(),
            models: Vec::new(),
            default_models: HashMap::new(),
            fallback_chains: HashMap::new(),
            global_rate_limits: RateLimitConfig::default(),
            preload_models: Vec::new(),
            enable_queue: true,
            max_queue_depth: 1000,
            queue_strategy: QueueStrategy::PriorityFifo,
            health_check_interval_ms: 30_000,
            metrics: MetricsConfig::default(),
        }
    }
}

/// Queue ordering strategy
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueStrategy {
    /// First-in, first-out
    Fifo,
    /// Priority queue (higher priority first, FIFO within priority)
    PriorityFifo,
    /// Shortest job first (based on estimated token count)
    ShortestJobFirst,
    /// Fair scheduling across conversations
    FairShare,
}

impl Default for QueueStrategy {
    fn default() -> Self {
        Self::PriorityFifo
    }
}

/// Metrics collection configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,

    /// Export metrics via Prometheus endpoint
    pub prometheus_enabled: bool,

    /// Prometheus endpoint port
    pub prometheus_port: u16,

    /// Histogram buckets for latency (ms)
    pub latency_buckets: Vec<f64>,

    /// Keep per-model metrics
    pub per_model_metrics: bool,

    /// Keep per-backend metrics
    pub per_backend_metrics: bool,

    /// Sample rate for detailed tracing (0.0 - 1.0)
    pub trace_sample_rate: f32,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prometheus_enabled: false,
            prometheus_port: 9090,
            latency_buckets: vec![
                10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0,
            ],
            per_model_metrics: true,
            per_backend_metrics: true,
            trace_sample_rate: 0.01,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_class_defaults() {
        assert!(TaskClass::QuickResponse.target_ttft() <= Duration::from_millis(100));
        assert!(TaskClass::DeepThinking.target_ttft() >= Duration::from_secs(1));
        assert!(TaskClass::QuickResponse.priority() > TaskClass::DeepThinking.priority());
    }

    #[test]
    fn test_retry_backoff() {
        let config = RetryConfig {
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            max_backoff_ms: 1000,
            use_jitter: false,
            ..Default::default()
        };

        assert_eq!(config.backoff_for_attempt(0), Duration::from_millis(100));
        assert_eq!(config.backoff_for_attempt(1), Duration::from_millis(200));
        assert_eq!(config.backoff_for_attempt(2), Duration::from_millis(400));
        assert_eq!(config.backoff_for_attempt(3), Duration::from_millis(800));
        assert_eq!(config.backoff_for_attempt(4), Duration::from_millis(1000)); // Capped
    }

    #[test]
    fn test_model_affinity() {
        let mut profile = ModelProfile::new("test-model", "ollama");
        profile.strengths = vec![TaskClass::CodeGeneration, TaskClass::Mathematical];
        profile.weaknesses = vec![TaskClass::QuickResponse];

        assert_eq!(profile.affinity_for(TaskClass::CodeGeneration), 1.0);
        assert_eq!(profile.affinity_for(TaskClass::Mathematical), 1.0);
        assert_eq!(profile.affinity_for(TaskClass::QuickResponse), 0.2);
        assert_eq!(profile.affinity_for(TaskClass::Creative), 0.5);
    }
}
