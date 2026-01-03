//! Query Router
//!
//! The main entry point for routing requests to models. Coordinates between
//! the routing policy, connection pools, and backends.
//!
//! # Usage
//!
//! ```ignore
//! let router = QueryRouter::new(config);
//! router.start().await?;
//!
//! let response = router.route_request(request).await?;
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, RwLock, Semaphore};

use super::config::{BackendConfig, RetryConfig, RouterConfig};
use super::connection_pool::{ConnectionPool, PoolError, PoolManager};
use super::fallback::{FallbackChainManager, FallbackContext};
use super::health::HealthTracker;
use super::metrics::RouterMetrics;
use super::policy::{RoutingDecision, RoutingError, RoutingPolicy, RoutingRequest};
use super::semaphore::GpuMemoryManager;

use crate::backend::{LlmRequest, LlmResponse, StreamingToken};

// ============================================================================
// Router Request/Response
// ============================================================================

/// A routed request handle
pub struct RouterRequest {
    /// Original routing request
    pub request: RoutingRequest,
    /// Routing decision made
    pub decision: RoutingDecision,
    /// When the request was submitted
    pub submitted_at: Instant,
    /// When routing completed
    pub routed_at: Instant,
}

/// Result of a routed request
#[derive(Debug)]
pub enum RouterResponse {
    /// Streaming response
    Streaming {
        /// Receiver for tokens
        receiver: mpsc::Receiver<StreamingToken>,
        /// Request handle for tracking
        request_id: String,
        /// Selected model
        model_id: String,
    },
    /// Non-streaming response
    Complete {
        /// Full response
        response: LlmResponse,
        /// Request handle for tracking
        request_id: String,
        /// Selected model
        model_id: String,
    },
}

// ============================================================================
// Query Router
// ============================================================================

/// The main query router
pub struct QueryRouter {
    /// Configuration
    config: RouterConfig,
    /// Routing policy
    policy: Arc<RoutingPolicy>,
    /// Connection pool manager
    pools: Arc<PoolManager>,
    /// GPU memory manager (for local models)
    gpu_memory: Option<Arc<GpuMemoryManager>>,
    /// Metrics
    metrics: Arc<RouterMetrics>,
    /// Health tracker for models
    health_tracker: Arc<HealthTracker>,
    /// Fallback chain manager
    fallback_manager: Arc<FallbackChainManager>,
    /// Global concurrency limiter
    global_semaphore: Semaphore,
    /// Request queue (for priority scheduling)
    queue: RwLock<RequestQueue>,
    /// Whether the router is running
    running: RwLock<bool>,
    /// Backend registry
    backends: RwLock<HashMap<String, Arc<BackendHandle>>>,
}

/// Handle to a registered backend
struct BackendHandle {
    config: BackendConfig,
    pool: Arc<ConnectionPool>,
    healthy: RwLock<bool>,
}

/// Priority queue for requests
struct RequestQueue {
    /// Pending requests by priority
    pending: Vec<QueuedRequest>,
    /// Maximum queue size
    max_size: usize,
}

struct QueuedRequest {
    request: RoutingRequest,
    priority: u8,
    queued_at: Instant,
    response_tx: mpsc::Sender<Result<RouterResponse, RouterError>>,
}

impl RequestQueue {
    fn new(max_size: usize) -> Self {
        Self {
            pending: Vec::new(),
            max_size,
        }
    }

    fn push(&mut self, request: QueuedRequest) -> Result<(), RouterError> {
        if self.pending.len() >= self.max_size {
            return Err(RouterError::QueueFull);
        }

        // Insert in priority order (higher priority first)
        let pos = self
            .pending
            .iter()
            .position(|r| r.priority < request.priority)
            .unwrap_or(self.pending.len());

        self.pending.insert(pos, request);
        Ok(())
    }

    fn pop(&mut self) -> Option<QueuedRequest> {
        if self.pending.is_empty() {
            None
        } else {
            Some(self.pending.remove(0))
        }
    }

    fn len(&self) -> usize {
        self.pending.len()
    }
}

impl QueryRouter {
    /// Create a new query router
    #[must_use]
    pub fn new(config: RouterConfig) -> Self {
        let global_limit = config.global_rate_limits.max_concurrent;

        // Setup GPU memory manager if we have local models
        let gpu_memory = config
            .backends
            .iter()
            .find(|b| matches!(b.backend_type, super::config::BackendType::LocalGgml { .. }))
            .and_then(|b| b.resources.max_gpu_memory_bytes)
            .map(|max_mem| {
                Arc::new(GpuMemoryManager::new(
                    max_mem,
                    f64::from(config.backends[0].resources.memory_pressure_threshold),
                ))
            });

        // Initialize health tracker and fallback manager
        let health_tracker = Arc::new(HealthTracker::new());
        let fallback_manager = Arc::new(FallbackChainManager::new());

        Self {
            policy: Arc::new(RoutingPolicy::new()),
            pools: Arc::new(PoolManager::new(Default::default())),
            gpu_memory,
            metrics: Arc::new(RouterMetrics::new()),
            health_tracker,
            fallback_manager,
            global_semaphore: Semaphore::new(global_limit),
            queue: RwLock::new(RequestQueue::new(config.max_queue_depth)),
            running: RwLock::new(false),
            backends: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Get metrics reference
    pub fn metrics(&self) -> Arc<RouterMetrics> {
        self.metrics.clone()
    }

    /// Get policy reference
    pub fn policy(&self) -> Arc<RoutingPolicy> {
        self.policy.clone()
    }

    /// Get health tracker reference
    pub fn health_tracker(&self) -> Arc<HealthTracker> {
        self.health_tracker.clone()
    }

    /// Get fallback manager reference
    pub fn fallback_manager(&self) -> Arc<FallbackChainManager> {
        self.fallback_manager.clone()
    }

    /// Start the router (register backends, preload models, etc.)
    pub async fn start(&self) -> Result<(), RouterError> {
        {
            let mut running = self.running.write().await;
            if *running {
                return Err(RouterError::AlreadyRunning);
            }
            *running = true;
        }

        // Register backends
        for backend_config in &self.config.backends {
            self.register_backend(backend_config.clone()).await?;
        }

        // Register models with policy, health tracker, and fallback manager
        for model in &self.config.models {
            self.policy.register_model(model.clone()).await;
            // Register with health tracker
            self.health_tracker.register(&model.model_id);
            // Register profile with fallback manager for auto-generation
            self.fallback_manager.register_profile(model.clone());
        }

        // Set defaults
        for (task_class, model_id) in &self.config.default_models {
            self.policy.set_default(*task_class, model_id.clone()).await;
        }

        // Set fallbacks in both policy and fallback manager
        for (model_id, fallbacks) in &self.config.fallback_chains {
            self.policy
                .set_fallbacks(model_id.clone(), fallbacks.clone())
                .await;
            // Also register with fallback chain manager
            self.fallback_manager
                .set_chain(model_id.clone(), fallbacks.clone());
        }

        // Preload models
        for model_id in &self.config.preload_models {
            if let Err(e) = self.preload_model(model_id).await {
                tracing::warn!(model = model_id, error = %e, "Failed to preload model");
            }
        }

        // Start health check loop
        self.spawn_health_checker();

        // Start queue processor
        self.spawn_queue_processor();

        tracing::info!("Query router started");
        Ok(())
    }

    /// Register a backend
    async fn register_backend(&self, config: BackendConfig) -> Result<(), RouterError> {
        let pool = self.pools.create_pool(&config).await;

        let handle = Arc::new(BackendHandle {
            config: config.clone(),
            pool,
            healthy: RwLock::new(true),
        });

        let mut backends = self.backends.write().await;
        backends.insert(config.id.clone(), handle);

        tracing::info!(backend = %config.id, "Registered backend");
        Ok(())
    }

    /// Preload a model (for local models)
    async fn preload_model(&self, model_id: &str) -> Result<(), RouterError> {
        // Mark as loaded in policy
        self.policy.mark_loaded(model_id, true).await;
        self.metrics.models_loaded.inc();

        tracing::info!(model = model_id, "Preloaded model");
        Ok(())
    }

    /// Spawn health check background task
    fn spawn_health_checker(&self) {
        let interval = Duration::from_millis(self.config.health_check_interval_ms);

        // Note: In a full implementation, we'd pass a channel or Arc to communicate
        // with the health checker. For now, this is a placeholder.
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                // Health check logic would go here
                // For now, just tick to avoid blocking
            }
        });
    }

    /// Spawn queue processor background task
    fn spawn_queue_processor(&self) {
        // In a full implementation, this would process queued requests
        // For now, we process requests inline
    }

    /// Route a request to a model
    #[allow(unused_variables)]
    pub async fn route(&self, request: RoutingRequest) -> Result<RouterResponse, RouterError> {
        let request_id = request.request_id.clone();
        let start = Instant::now();

        // Check if we're running
        if !*self.running.read().await {
            return Err(RouterError::NotRunning);
        }

        // Try to acquire global permit (with timeout)
        let timeout = request.effective_timeout();
        let permit = match tokio::time::timeout(
            Duration::from_millis(self.config.global_rate_limits.queue_timeout_ms),
            self.global_semaphore.acquire(),
        )
        .await
        {
            Ok(Ok(p)) => p,
            Ok(Err(_)) => return Err(RouterError::ShuttingDown),
            Err(_) => {
                self.metrics.record_rejection();
                return Err(RouterError::RateLimited);
            }
        };

        // Make routing decision
        let task_class = request.classify();
        let decision = match self.policy.route(&request).await {
            Ok(d) => d,
            Err(e) => {
                drop(permit);
                return Err(RouterError::RoutingFailed(e));
            }
        };

        // Check model health before proceeding
        if !self.health_tracker.is_available(&decision.model_id) {
            tracing::warn!(
                model = %decision.model_id,
                "Primary model unavailable, attempting fallback"
            );
            // Try to get a healthy fallback using the fallback manager
            let is_healthy = |m: &str| self.health_tracker.is_available(m);
            if let Some(fallback) = self.fallback_manager.get_next_healthy_fallback(
                &decision.model_id,
                Some(task_class),
                is_healthy,
            ) {
                tracing::info!(
                    from = %decision.model_id,
                    to = %fallback,
                    "Using healthy fallback model"
                );
                // Continue with modified decision - fallback will be used in execute_with_retry
            }
        }

        let routing_time = start.elapsed();
        self.metrics
            .routing_decision_time
            .record(routing_time.as_millis() as f64);

        // Record request start
        self.metrics
            .record_request_start(&decision.model_id, task_class)
            .await;

        // Execute request with retry/fallback
        let result = self.execute_with_retry(&request, &decision).await;

        // Record result to metrics, policy, and health tracker
        match &result {
            Ok(response) => {
                // Record success to health tracker
                let response_time_ms = start.elapsed().as_millis() as u64;
                let model_id = match response {
                    RouterResponse::Streaming { model_id, .. } => model_id,
                    RouterResponse::Complete { model_id, .. } => model_id,
                };
                self.health_tracker
                    .record_success(model_id, response_time_ms);
                self.policy
                    .record_request_result(model_id, true, Some(response_time_ms), None)
                    .await;
            }
            Err(e) => {
                let is_timeout = matches!(e, RouterError::Timeout);
                self.metrics
                    .record_request_failure(&decision.model_id, is_timeout)
                    .await;
                // Record failure to health tracker
                self.health_tracker.record_failure(&decision.model_id);
                self.policy
                    .record_request_result(&decision.model_id, false, None, None)
                    .await;
            }
        }

        // Record session affinity
        if let Some(ref conv_id) = request.conversation_id {
            self.policy
                .record_session_affinity(conv_id, &decision.model_id)
                .await;
        }

        drop(permit);
        result
    }

    /// Execute a request with retry and fallback logic
    async fn execute_with_retry(
        &self,
        request: &RoutingRequest,
        decision: &RoutingDecision,
    ) -> Result<RouterResponse, RouterError> {
        let retry_config = self.get_retry_config(&decision.backend_id).await;
        let task_class = request.classify();

        // Create fallback context to track what we've tried
        let mut fallback_ctx = FallbackContext::new(&decision.model_id);
        if let Some(tc) = request.task_class {
            fallback_ctx.task_class = Some(tc);
        }

        let mut current_model = decision.model_id.clone();

        for attempt in 0..=retry_config.max_retries {
            // Check if current model is healthy before attempting
            if attempt > 0 && !self.health_tracker.is_available(&current_model) {
                // Skip unhealthy model, find next fallback
                let is_healthy =
                    |m: &str| self.health_tracker.is_available(m) && !fallback_ctx.has_tried(m);
                if let Some(healthy_fallback) = self.fallback_manager.get_next_healthy_fallback(
                    &current_model,
                    Some(task_class),
                    is_healthy,
                ) {
                    self.metrics
                        .record_fallback(&current_model, &healthy_fallback)
                        .await;
                    fallback_ctx.fallback_to(&healthy_fallback);
                    current_model = healthy_fallback;
                }
            }

            match self
                .execute_request(request, &current_model, &decision.backend_id)
                .await
            {
                Ok(response) => {
                    // Record success to health tracker
                    self.health_tracker.record_success(&current_model, 100); // Placeholder timing
                    return Ok(response);
                }
                Err(e) => {
                    // Record failure to health tracker
                    self.health_tracker.record_failure(&current_model);

                    let should_retry = match &e {
                        RouterError::Timeout => retry_config.retry_on_timeout,
                        RouterError::ConnectionError(_) => retry_config.retry_on_connection_error,
                        RouterError::BackendError(status, _) => {
                            retry_config.should_retry_status(*status)
                        }
                        _ => false,
                    };

                    if !should_retry || attempt >= retry_config.max_retries {
                        // Try to get a healthy fallback using the fallback manager
                        if retry_config.fallback_on_failure {
                            let is_healthy = |m: &str| {
                                self.health_tracker.is_available(m) && !fallback_ctx.has_tried(m)
                            };
                            if let Some(fallback_model) =
                                self.fallback_manager.get_next_healthy_fallback(
                                    &current_model,
                                    Some(task_class),
                                    is_healthy,
                                )
                            {
                                self.metrics
                                    .record_fallback(&current_model, &fallback_model)
                                    .await;
                                fallback_ctx.fallback_to(&fallback_model);
                                current_model = fallback_model;
                                continue;
                            }
                        }
                        return Err(e);
                    }

                    // Backoff before retry
                    let backoff = retry_config.backoff_for_attempt(attempt);
                    tokio::time::sleep(backoff).await;
                }
            }
        }

        Err(RouterError::MaxRetriesExceeded)
    }

    /// Execute a single request (no retry logic)
    #[allow(unused_variables)]
    async fn execute_request(
        &self,
        request: &RoutingRequest,
        model_id: &str,
        backend_id: &str,
    ) -> Result<RouterResponse, RouterError> {
        // Get the backend pool
        let backends = self.backends.read().await;
        let backend = backends
            .get(backend_id)
            .ok_or_else(|| RouterError::BackendNotFound(backend_id.to_string()))?;

        // Check backend health
        if !*backend.healthy.read().await {
            return Err(RouterError::BackendUnhealthy(backend_id.to_string()));
        }

        // Acquire connection from pool
        let timeout = request.effective_timeout();
        let conn = backend.pool.acquire(timeout).await.map_err(|e| match e {
            PoolError::Timeout => RouterError::Timeout,
            PoolError::PoolClosed => RouterError::ShuttingDown,
            e => RouterError::ConnectionError(e.to_string()),
        })?;

        // Get HTTP client
        let client = conn.http_client().ok_or_else(|| {
            RouterError::BackendError(500, "No HTTP client available".to_string())
        })?;

        // Build LLM request
        let llm_request =
            LlmRequest::new(&request.prompt, model_id).with_stream(request.requires_streaming);

        // For now, we'll return a placeholder - in a full implementation,
        // this would call the actual backend
        let model_id_owned = model_id.to_string();

        if request.requires_streaming {
            let (tx, rx) = mpsc::channel(100);
            let model_for_task = model_id_owned.clone();

            // In a real implementation, spawn a task to stream from the backend
            tokio::spawn(async move {
                // Simulate streaming
                let _ = tx
                    .send(StreamingToken::Token("Response from ".to_string()))
                    .await;
                let _ = tx.send(StreamingToken::Token(model_for_task.clone())).await;
                let _ = tx
                    .send(StreamingToken::Complete {
                        message: format!("Response from {model_for_task}"),
                    })
                    .await;
            });

            Ok(RouterResponse::Streaming {
                receiver: rx,
                request_id: request.request_id.clone(),
                model_id: model_id_owned,
            })
        } else {
            Ok(RouterResponse::Complete {
                response: LlmResponse {
                    content: format!("Response from {model_id_owned}"),
                    model: model_id_owned.clone(),
                    tokens_used: Some(10),
                    duration_ms: Some(100),
                },
                request_id: request.request_id.clone(),
                model_id: model_id_owned,
            })
        }
    }

    /// Get retry config for a backend
    async fn get_retry_config(&self, backend_id: &str) -> RetryConfig {
        let backends = self.backends.read().await;
        backends
            .get(backend_id)
            .map(|b| b.config.retry.clone())
            .unwrap_or_default()
    }

    /// Shutdown the router
    pub async fn shutdown(&self) {
        let mut running = self.running.write().await;
        *running = false;

        // Drain pools
        let backends = self.backends.read().await;
        for (_, handle) in backends.iter() {
            handle.pool.drain().await;
        }

        tracing::info!("Query router shut down");
    }

    /// Get current queue depth
    pub async fn queue_depth(&self) -> usize {
        let queue = self.queue.read().await;
        queue.len()
    }

    /// Check if router is healthy
    pub async fn is_healthy(&self) -> bool {
        if !*self.running.read().await {
            return false;
        }

        // Check at least one backend is healthy
        let backends = self.backends.read().await;
        for (_, handle) in backends.iter() {
            if *handle.healthy.read().await {
                return true;
            }
        }

        false
    }
}

// ============================================================================
// Router Errors
// ============================================================================

/// Router errors
#[derive(Clone, Debug)]
pub enum RouterError {
    /// Router not running
    NotRunning,
    /// Router already running
    AlreadyRunning,
    /// Router is shutting down
    ShuttingDown,
    /// Request rate limited
    RateLimited,
    /// Request queue full
    QueueFull,
    /// Routing failed
    RoutingFailed(RoutingError),
    /// Backend not found
    BackendNotFound(String),
    /// Backend unhealthy
    BackendUnhealthy(String),
    /// Connection error
    ConnectionError(String),
    /// Request timeout
    Timeout,
    /// Backend returned error
    BackendError(u16, String),
    /// Max retries exceeded
    MaxRetriesExceeded,
    /// Model not loaded
    ModelNotLoaded(String),
}

impl std::fmt::Display for RouterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRunning => write!(f, "Router not running"),
            Self::AlreadyRunning => write!(f, "Router already running"),
            Self::ShuttingDown => write!(f, "Router is shutting down"),
            Self::RateLimited => write!(f, "Request rate limited"),
            Self::QueueFull => write!(f, "Request queue full"),
            Self::RoutingFailed(e) => write!(f, "Routing failed: {e}"),
            Self::BackendNotFound(id) => write!(f, "Backend not found: {id}"),
            Self::BackendUnhealthy(id) => write!(f, "Backend unhealthy: {id}"),
            Self::ConnectionError(e) => write!(f, "Connection error: {e}"),
            Self::Timeout => write!(f, "Request timed out"),
            Self::BackendError(status, msg) => write!(f, "Backend error {status}: {msg}"),
            Self::MaxRetriesExceeded => write!(f, "Max retries exceeded"),
            Self::ModelNotLoaded(id) => write!(f, "Model not loaded: {id}"),
        }
    }
}

impl std::error::Error for RouterError {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_router_creation() {
        let config = RouterConfig::default();
        let router = QueryRouter::new(config);

        assert!(!*router.running.read().await);
        assert_eq!(router.queue_depth().await, 0);
    }

    #[tokio::test]
    async fn test_request_queue() {
        let mut queue = RequestQueue::new(10);

        // Create some requests with different priorities
        let (tx1, _rx1) = mpsc::channel(1);
        let (tx2, _rx2) = mpsc::channel(1);
        let (tx3, _rx3) = mpsc::channel(1);

        queue
            .push(QueuedRequest {
                request: RoutingRequest::new("low").with_urgency(3),
                priority: 30,
                queued_at: Instant::now(),
                response_tx: tx1,
            })
            .unwrap();

        queue
            .push(QueuedRequest {
                request: RoutingRequest::new("high").with_urgency(9),
                priority: 90,
                queued_at: Instant::now(),
                response_tx: tx2,
            })
            .unwrap();

        queue
            .push(QueuedRequest {
                request: RoutingRequest::new("medium").with_urgency(5),
                priority: 50,
                queued_at: Instant::now(),
                response_tx: tx3,
            })
            .unwrap();

        // Should pop in priority order
        let first = queue.pop().unwrap();
        assert_eq!(first.priority, 90);

        let second = queue.pop().unwrap();
        assert_eq!(second.priority, 50);

        let third = queue.pop().unwrap();
        assert_eq!(third.priority, 30);
    }
}
