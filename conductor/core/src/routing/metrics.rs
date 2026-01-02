//! Router Metrics
//!
//! Observability for the routing system including:
//! - Request latencies (TTFT, total time)
//! - Throughput (requests/sec, tokens/sec)
//! - Error rates and types
//! - Queue depths
//! - Model utilization
//! - Resource usage (memory, connections)

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use super::config::TaskClass;

// ============================================================================
// Histogram for Latency Tracking
// ============================================================================

/// A histogram for tracking latency distributions
#[derive(Debug)]
pub struct Histogram {
    /// Bucket boundaries (in the unit being measured)
    buckets: Vec<f64>,
    /// Count per bucket
    counts: Vec<AtomicU64>,
    /// Total count
    total_count: AtomicU64,
    /// Sum of all values
    sum: AtomicU64,
    /// Minimum value seen
    min: AtomicU64,
    /// Maximum value seen
    max: AtomicU64,
}

impl Histogram {
    /// Create a new histogram with the given bucket boundaries
    pub fn new(buckets: Vec<f64>) -> Self {
        let counts = buckets.iter().map(|_| AtomicU64::new(0)).collect();
        Self {
            buckets,
            counts,
            total_count: AtomicU64::new(0),
            sum: AtomicU64::new(0),
            min: AtomicU64::new(u64::MAX),
            max: AtomicU64::new(0),
        }
    }

    /// Create with default latency buckets (in milliseconds)
    pub fn latency_default() -> Self {
        Self::new(vec![
            10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0, 30000.0,
            60000.0, 120000.0,
        ])
    }

    /// Record a value
    pub fn record(&self, value: f64) {
        // Find bucket
        let bucket_idx = self
            .buckets
            .iter()
            .position(|&b| value <= b)
            .unwrap_or(self.buckets.len() - 1);

        self.counts[bucket_idx].fetch_add(1, Ordering::Relaxed);
        self.total_count.fetch_add(1, Ordering::Relaxed);
        self.sum.fetch_add(value as u64, Ordering::Relaxed);

        // Update min/max (not perfectly atomic but good enough for metrics)
        let value_u64 = value as u64;
        let mut current_min = self.min.load(Ordering::Relaxed);
        while value_u64 < current_min {
            match self.min.compare_exchange_weak(
                current_min,
                value_u64,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_min = x,
            }
        }

        let mut current_max = self.max.load(Ordering::Relaxed);
        while value_u64 > current_max {
            match self.max.compare_exchange_weak(
                current_max,
                value_u64,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }
    }

    /// Get histogram snapshot
    pub fn snapshot(&self) -> HistogramSnapshot {
        let counts: Vec<u64> = self
            .counts
            .iter()
            .map(|c| c.load(Ordering::Relaxed))
            .collect();
        let total = self.total_count.load(Ordering::Relaxed);
        let sum = self.sum.load(Ordering::Relaxed);
        let min = self.min.load(Ordering::Relaxed);
        let max = self.max.load(Ordering::Relaxed);

        HistogramSnapshot {
            buckets: self.buckets.clone(),
            counts,
            total,
            sum,
            min: if min == u64::MAX { 0 } else { min },
            max,
            mean: if total > 0 {
                sum as f64 / total as f64
            } else {
                0.0
            },
        }
    }
}

/// Snapshot of histogram data
#[derive(Clone, Debug)]
pub struct HistogramSnapshot {
    pub buckets: Vec<f64>,
    pub counts: Vec<u64>,
    pub total: u64,
    pub sum: u64,
    pub min: u64,
    pub max: u64,
    pub mean: f64,
}

impl HistogramSnapshot {
    /// Get percentile value
    pub fn percentile(&self, p: f64) -> f64 {
        if self.total == 0 {
            return 0.0;
        }

        let target = (self.total as f64 * p) as u64;
        let mut cumulative = 0u64;

        for (i, &count) in self.counts.iter().enumerate() {
            cumulative += count;
            if cumulative >= target {
                return self.buckets[i];
            }
        }

        *self.buckets.last().unwrap_or(&0.0)
    }

    /// Get p50
    pub fn p50(&self) -> f64 {
        self.percentile(0.5)
    }

    /// Get p90
    pub fn p90(&self) -> f64 {
        self.percentile(0.9)
    }

    /// Get p99
    pub fn p99(&self) -> f64 {
        self.percentile(0.99)
    }
}

// ============================================================================
// Counter
// ============================================================================

/// A simple atomic counter
#[derive(Debug, Default)]
pub struct Counter {
    value: AtomicU64,
}

impl Counter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add(&self, n: u64) {
        self.value.fetch_add(n, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}

// ============================================================================
// Gauge
// ============================================================================

/// A gauge (can go up or down)
#[derive(Debug, Default)]
pub struct Gauge {
    value: AtomicU64,
}

impl Gauge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, value: u64) {
        self.value.store(value, Ordering::Relaxed);
    }

    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec(&self) {
        self.value.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}

// ============================================================================
// Per-Model Metrics
// ============================================================================

/// Metrics for a single model
pub struct ModelMetrics {
    /// Model ID
    pub model_id: String,
    /// Total requests
    pub requests: Counter,
    /// Successful requests
    pub successes: Counter,
    /// Failed requests
    pub failures: Counter,
    /// Timeouts
    pub timeouts: Counter,
    /// Time to first token histogram
    pub ttft: Histogram,
    /// Total response time histogram
    pub response_time: Histogram,
    /// Tokens generated
    pub tokens_generated: Counter,
    /// Current active requests
    pub active_requests: Gauge,
    /// Current queue depth
    pub queue_depth: Gauge,
}

impl ModelMetrics {
    pub fn new(model_id: String) -> Self {
        Self {
            model_id,
            requests: Counter::new(),
            successes: Counter::new(),
            failures: Counter::new(),
            timeouts: Counter::new(),
            ttft: Histogram::latency_default(),
            response_time: Histogram::latency_default(),
            tokens_generated: Counter::new(),
            active_requests: Gauge::new(),
            queue_depth: Gauge::new(),
        }
    }

    /// Get error rate
    pub fn error_rate(&self) -> f64 {
        let total = self.requests.get();
        if total == 0 {
            return 0.0;
        }
        self.failures.get() as f64 / total as f64
    }

    /// Get average tokens per second
    pub fn avg_tokens_per_sec(&self) -> f64 {
        let snapshot = self.response_time.snapshot();
        if snapshot.total == 0 || snapshot.sum == 0 {
            return 0.0;
        }
        let tokens = self.tokens_generated.get() as f64;
        let seconds = snapshot.sum as f64 / 1000.0;
        tokens / seconds
    }

    /// Get summary
    pub fn summary(&self) -> ModelMetricsSummary {
        let ttft = self.ttft.snapshot();
        let response_time = self.response_time.snapshot();

        ModelMetricsSummary {
            model_id: self.model_id.clone(),
            total_requests: self.requests.get(),
            successful_requests: self.successes.get(),
            failed_requests: self.failures.get(),
            timeout_requests: self.timeouts.get(),
            error_rate: self.error_rate(),
            ttft_p50_ms: ttft.p50(),
            ttft_p90_ms: ttft.p90(),
            ttft_p99_ms: ttft.p99(),
            response_time_p50_ms: response_time.p50(),
            response_time_p90_ms: response_time.p90(),
            response_time_p99_ms: response_time.p99(),
            tokens_generated: self.tokens_generated.get(),
            tokens_per_second: self.avg_tokens_per_sec(),
            active_requests: self.active_requests.get(),
            queue_depth: self.queue_depth.get(),
        }
    }
}

/// Summary of model metrics
#[derive(Clone, Debug)]
pub struct ModelMetricsSummary {
    pub model_id: String,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub timeout_requests: u64,
    pub error_rate: f64,
    pub ttft_p50_ms: f64,
    pub ttft_p90_ms: f64,
    pub ttft_p99_ms: f64,
    pub response_time_p50_ms: f64,
    pub response_time_p90_ms: f64,
    pub response_time_p99_ms: f64,
    pub tokens_generated: u64,
    pub tokens_per_second: f64,
    pub active_requests: u64,
    pub queue_depth: u64,
}

// ============================================================================
// Router Metrics
// ============================================================================

/// Centralized metrics for the router
pub struct RouterMetrics {
    /// Per-model metrics
    models: RwLock<HashMap<String, Arc<ModelMetrics>>>,

    /// Global counters
    pub total_requests: Counter,
    pub total_routed: Counter,
    pub total_fallbacks: Counter,
    pub total_rejections: Counter,

    /// Routing decision histogram (time to make routing decision)
    pub routing_decision_time: Histogram,

    /// Queue metrics
    pub queue_depth: Gauge,
    pub queue_wait_time: Histogram,

    /// Connection pool metrics
    pub pool_connections_active: Gauge,
    pub pool_connections_idle: Gauge,
    pub pool_wait_time: Histogram,

    /// Resource metrics
    pub gpu_memory_used: Gauge,
    pub gpu_memory_total: Gauge,
    pub models_loaded: Gauge,

    /// Task class counters
    task_class_counts: RwLock<HashMap<TaskClass, Counter>>,

    /// When metrics collection started
    started_at: Instant,
}

impl RouterMetrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            models: RwLock::new(HashMap::new()),
            total_requests: Counter::new(),
            total_routed: Counter::new(),
            total_fallbacks: Counter::new(),
            total_rejections: Counter::new(),
            routing_decision_time: Histogram::new(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 25.0]),
            queue_depth: Gauge::new(),
            queue_wait_time: Histogram::latency_default(),
            pool_connections_active: Gauge::new(),
            pool_connections_idle: Gauge::new(),
            pool_wait_time: Histogram::latency_default(),
            gpu_memory_used: Gauge::new(),
            gpu_memory_total: Gauge::new(),
            models_loaded: Gauge::new(),
            task_class_counts: RwLock::new(HashMap::new()),
            started_at: Instant::now(),
        }
    }

    /// Get or create metrics for a model
    pub async fn model_metrics(&self, model_id: &str) -> Arc<ModelMetrics> {
        // Try read first
        {
            let models = self.models.read().await;
            if let Some(metrics) = models.get(model_id) {
                return metrics.clone();
            }
        }

        // Create new
        let mut models = self.models.write().await;
        models
            .entry(model_id.to_string())
            .or_insert_with(|| Arc::new(ModelMetrics::new(model_id.to_string())))
            .clone()
    }

    /// Record a request start
    pub async fn record_request_start(&self, model_id: &str, task_class: TaskClass) {
        self.total_requests.inc();

        let metrics = self.model_metrics(model_id).await;
        metrics.requests.inc();
        metrics.active_requests.inc();

        // Record task class
        let mut classes = self.task_class_counts.write().await;
        classes.entry(task_class).or_insert_with(Counter::new).inc();
    }

    /// Record a successful request
    pub async fn record_request_success(
        &self,
        model_id: &str,
        ttft_ms: u64,
        total_time_ms: u64,
        tokens: u64,
    ) {
        self.total_routed.inc();

        let metrics = self.model_metrics(model_id).await;
        metrics.successes.inc();
        metrics.active_requests.dec();
        metrics.ttft.record(ttft_ms as f64);
        metrics.response_time.record(total_time_ms as f64);
        metrics.tokens_generated.add(tokens);
    }

    /// Record a failed request
    pub async fn record_request_failure(&self, model_id: &str, is_timeout: bool) {
        let metrics = self.model_metrics(model_id).await;
        metrics.failures.inc();
        metrics.active_requests.dec();

        if is_timeout {
            metrics.timeouts.inc();
        }
    }

    /// Record a fallback
    pub async fn record_fallback(&self, from_model: &str, to_model: &str) {
        self.total_fallbacks.inc();
        tracing::debug!(
            from = from_model,
            to = to_model,
            "Request fell back to alternate model"
        );
    }

    /// Record a rejection
    pub fn record_rejection(&self) {
        self.total_rejections.inc();
    }

    /// Update queue depth
    pub fn update_queue_depth(&self, depth: u64) {
        self.queue_depth.set(depth);
    }

    /// Record queue wait time
    pub fn record_queue_wait(&self, wait_ms: u64) {
        self.queue_wait_time.record(wait_ms as f64);
    }

    /// Update GPU memory usage
    pub fn update_gpu_memory(&self, used: u64, total: u64) {
        self.gpu_memory_used.set(used);
        self.gpu_memory_total.set(total);
    }

    /// Update loaded model count
    pub fn update_models_loaded(&self, count: u64) {
        self.models_loaded.set(count);
    }

    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Get all model summaries
    pub async fn all_model_summaries(&self) -> Vec<ModelMetricsSummary> {
        let models = self.models.read().await;
        models.values().map(|m| m.summary()).collect()
    }

    /// Get global summary
    pub async fn global_summary(&self) -> GlobalMetricsSummary {
        let routing = self.routing_decision_time.snapshot();
        let queue = self.queue_wait_time.snapshot();
        let pool = self.pool_wait_time.snapshot();

        let model_summaries = self.all_model_summaries().await;

        // Aggregate model metrics
        let total_tokens: u64 = model_summaries.iter().map(|m| m.tokens_generated).sum();
        let avg_error_rate = if model_summaries.is_empty() {
            0.0
        } else {
            model_summaries.iter().map(|m| m.error_rate).sum::<f64>() / model_summaries.len() as f64
        };

        GlobalMetricsSummary {
            uptime_seconds: self.uptime().as_secs(),
            total_requests: self.total_requests.get(),
            total_routed: self.total_routed.get(),
            total_fallbacks: self.total_fallbacks.get(),
            total_rejections: self.total_rejections.get(),
            routing_decision_p50_ms: routing.p50(),
            routing_decision_p99_ms: routing.p99(),
            queue_depth: self.queue_depth.get(),
            queue_wait_p50_ms: queue.p50(),
            queue_wait_p99_ms: queue.p99(),
            pool_connections_active: self.pool_connections_active.get(),
            pool_connections_idle: self.pool_connections_idle.get(),
            pool_wait_p50_ms: pool.p50(),
            gpu_memory_used: self.gpu_memory_used.get(),
            gpu_memory_total: self.gpu_memory_total.get(),
            models_loaded: self.models_loaded.get(),
            total_tokens_generated: total_tokens,
            average_error_rate: avg_error_rate,
            model_count: model_summaries.len() as u64,
        }
    }

    /// Export as Prometheus format
    pub async fn to_prometheus(&self) -> String {
        let mut output = String::new();

        // Global metrics
        output.push_str(&format!(
            "# HELP router_requests_total Total requests received\n\
             # TYPE router_requests_total counter\n\
             router_requests_total {}\n\n",
            self.total_requests.get()
        ));

        output.push_str(&format!(
            "# HELP router_routed_total Successfully routed requests\n\
             # TYPE router_routed_total counter\n\
             router_routed_total {}\n\n",
            self.total_routed.get()
        ));

        output.push_str(&format!(
            "# HELP router_fallbacks_total Requests that used fallback\n\
             # TYPE router_fallbacks_total counter\n\
             router_fallbacks_total {}\n\n",
            self.total_fallbacks.get()
        ));

        output.push_str(&format!(
            "# HELP router_queue_depth Current queue depth\n\
             # TYPE router_queue_depth gauge\n\
             router_queue_depth {}\n\n",
            self.queue_depth.get()
        ));

        output.push_str(&format!(
            "# HELP router_gpu_memory_bytes GPU memory usage\n\
             # TYPE router_gpu_memory_bytes gauge\n\
             router_gpu_memory_bytes{{type=\"used\"}} {}\n\
             router_gpu_memory_bytes{{type=\"total\"}} {}\n\n",
            self.gpu_memory_used.get(),
            self.gpu_memory_total.get()
        ));

        // Per-model metrics
        let models = self.models.read().await;
        for (model_id, metrics) in models.iter() {
            let summary = metrics.summary();
            output.push_str(&format!(
                "# HELP model_requests_total Requests per model\n\
                 # TYPE model_requests_total counter\n\
                 model_requests_total{{model=\"{}\"}} {}\n\n",
                model_id, summary.total_requests
            ));

            output.push_str(&format!(
                "model_errors_total{{model=\"{}\"}} {}\n",
                model_id, summary.failed_requests
            ));

            output.push_str(&format!(
                "model_ttft_p50_ms{{model=\"{}\"}} {}\n",
                model_id, summary.ttft_p50_ms
            ));

            output.push_str(&format!(
                "model_ttft_p99_ms{{model=\"{}\"}} {}\n",
                model_id, summary.ttft_p99_ms
            ));

            output.push_str(&format!(
                "model_tokens_total{{model=\"{}\"}} {}\n\n",
                model_id, summary.tokens_generated
            ));
        }

        output
    }
}

impl Default for RouterMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Global metrics summary
#[derive(Clone, Debug)]
pub struct GlobalMetricsSummary {
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub total_routed: u64,
    pub total_fallbacks: u64,
    pub total_rejections: u64,
    pub routing_decision_p50_ms: f64,
    pub routing_decision_p99_ms: f64,
    pub queue_depth: u64,
    pub queue_wait_p50_ms: f64,
    pub queue_wait_p99_ms: f64,
    pub pool_connections_active: u64,
    pub pool_connections_idle: u64,
    pub pool_wait_p50_ms: f64,
    pub gpu_memory_used: u64,
    pub gpu_memory_total: u64,
    pub models_loaded: u64,
    pub total_tokens_generated: u64,
    pub average_error_rate: f64,
    pub model_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_histogram() {
        let hist = Histogram::new(vec![10.0, 25.0, 50.0, 100.0]);

        hist.record(5.0); // Goes into bucket 10
        hist.record(15.0); // Goes into bucket 25
        hist.record(75.0); // Goes into bucket 100
        hist.record(200.0); // Goes into bucket 100 (last bucket)

        let snap = hist.snapshot();
        assert_eq!(snap.total, 4);
        assert_eq!(snap.min, 5);
        assert_eq!(snap.max, 200);
    }

    #[test]
    fn test_percentiles() {
        let hist = Histogram::new(vec![10.0, 50.0, 100.0, 500.0, 1000.0]);

        // Add 100 samples
        for i in 0..100 {
            hist.record((i * 10) as f64);
        }

        let snap = hist.snapshot();
        // P50 should be around 500 (middle bucket)
        let p50 = snap.p50();
        assert!(p50 >= 50.0 && p50 <= 500.0);
    }

    #[tokio::test]
    async fn test_router_metrics() {
        let metrics = RouterMetrics::new();

        // Record some requests
        metrics
            .record_request_start("model-a", TaskClass::CodeGeneration)
            .await;
        metrics
            .record_request_success("model-a", 100, 2000, 500)
            .await;

        metrics
            .record_request_start("model-a", TaskClass::QuickResponse)
            .await;
        metrics.record_request_failure("model-a", false).await;

        let summary = metrics.model_metrics("model-a").await.summary();
        assert_eq!(summary.total_requests, 2);
        assert_eq!(summary.successful_requests, 1);
        assert_eq!(summary.failed_requests, 1);
        assert!((summary.error_rate - 0.5).abs() < 0.01);
    }
}
