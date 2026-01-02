//! Routing Policy
//!
//! Implements intelligent routing decisions based on:
//! - Task classification
//! - Model availability and health
//! - Resource constraints
//! - Historical performance
//!
//! # Routing Decision Flow
//!
//! ```text
//! 1. Classify incoming request (task type, urgency, size)
//! 2. Filter available models (healthy, can meet latency, has capacity)
//! 3. Score candidates (affinity, performance, cost)
//! 4. Select best model
//! 5. Get fallback chain if primary fails
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use super::config::{CostTier, ModelProfile, TaskClass};
use super::metrics::RouterMetrics;

// ============================================================================
// Routing Request
// ============================================================================

/// A routing request with classification hints
#[derive(Clone, Debug)]
pub struct RoutingRequest {
    /// Unique request ID
    pub request_id: String,

    /// The prompt/message
    pub prompt: String,

    /// Explicitly requested model (optional)
    pub requested_model: Option<String>,

    /// Task classification hint
    pub task_class: Option<TaskClass>,

    /// Urgency level (1-10, 10 = most urgent)
    pub urgency: u8,

    /// Estimated input tokens
    pub estimated_input_tokens: Option<u32>,

    /// Estimated output tokens
    pub estimated_output_tokens: Option<u32>,

    /// Whether streaming is required
    pub requires_streaming: bool,

    /// Whether tool use is required
    pub requires_tools: bool,

    /// Maximum cost tier allowed
    pub max_cost_tier: Option<CostTier>,

    /// Timeout override
    pub timeout: Option<Duration>,

    /// Conversation ID (for session affinity)
    pub conversation_id: Option<String>,

    /// Priority override
    pub priority: Option<u8>,
}

impl Default for RoutingRequest {
    fn default() -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            prompt: String::new(),
            requested_model: None,
            task_class: None,
            urgency: 5,
            estimated_input_tokens: None,
            estimated_output_tokens: None,
            requires_streaming: true,
            requires_tools: false,
            max_cost_tier: None,
            timeout: None,
            conversation_id: None,
            priority: None,
        }
    }
}

impl RoutingRequest {
    /// Create a new routing request
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            ..Default::default()
        }
    }

    /// Set task classification
    #[must_use]
    pub fn with_task_class(mut self, class: TaskClass) -> Self {
        self.task_class = Some(class);
        self
    }

    /// Set urgency
    #[must_use]
    pub fn with_urgency(mut self, urgency: u8) -> Self {
        self.urgency = urgency.clamp(1, 10);
        self
    }

    /// Request a specific model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.requested_model = Some(model.into());
        self
    }

    /// Set conversation for session affinity
    pub fn with_conversation(mut self, conversation_id: impl Into<String>) -> Self {
        self.conversation_id = Some(conversation_id.into());
        self
    }

    /// Classify the request if not already classified
    #[must_use]
    pub fn classify(&self) -> TaskClass {
        self.task_class.unwrap_or_else(|| self.auto_classify())
    }

    /// Auto-classify based on prompt content
    fn auto_classify(&self) -> TaskClass {
        let prompt_lower = self.prompt.to_lowercase();
        let word_count = self.prompt.split_whitespace().count();

        // Quick response patterns
        if word_count < 10
            && (prompt_lower.contains("hi")
                || prompt_lower.contains("hello")
                || prompt_lower.contains("hey")
                || prompt_lower.contains("thanks"))
        {
            return TaskClass::QuickResponse;
        }

        // Code patterns
        if prompt_lower.contains("code")
            || prompt_lower.contains("function")
            || prompt_lower.contains("implement")
            || prompt_lower.contains("```")
            || prompt_lower.contains("bug")
            || prompt_lower.contains("fix")
        {
            return TaskClass::CodeGeneration;
        }

        // Math patterns
        if prompt_lower.contains("calculate")
            || prompt_lower.contains("solve")
            || prompt_lower.contains("prove")
            || prompt_lower.contains("equation")
            || prompt_lower.contains("math")
        {
            return TaskClass::Mathematical;
        }

        // Deep thinking patterns
        if prompt_lower.contains("analyze")
            || prompt_lower.contains("explain in detail")
            || prompt_lower.contains("compare and contrast")
            || prompt_lower.contains("step by step")
            || word_count > 100
        {
            return TaskClass::DeepThinking;
        }

        // Creative patterns
        if prompt_lower.contains("story")
            || prompt_lower.contains("write")
            || prompt_lower.contains("creative")
            || prompt_lower.contains("imagine")
        {
            return TaskClass::Creative;
        }

        // Tool use patterns
        if self.requires_tools
            || prompt_lower.contains("search")
            || prompt_lower.contains("look up")
            || prompt_lower.contains("find")
        {
            return TaskClass::ToolUse;
        }

        TaskClass::General
    }

    /// Get effective priority (considers urgency and task class)
    #[must_use]
    pub fn effective_priority(&self) -> u8 {
        if let Some(p) = self.priority {
            return p;
        }

        let class = self.classify();
        let base_priority = class.priority();

        // Adjust by urgency (1-10 maps to -9 to +0)
        let urgency_adjust = (i16::from(self.urgency) - 10) as i8;

        (i16::from(base_priority) + i16::from(urgency_adjust)).clamp(0, 100) as u8
    }

    /// Get effective timeout
    #[must_use]
    pub fn effective_timeout(&self) -> Duration {
        self.timeout
            .unwrap_or_else(|| self.classify().default_timeout())
    }
}

// ============================================================================
// Routing Decision
// ============================================================================

/// The result of a routing decision
#[derive(Clone, Debug)]
pub struct RoutingDecision {
    /// Selected model
    pub model_id: String,

    /// Backend to use
    pub backend_id: String,

    /// Effective timeout for this request
    pub timeout: Duration,

    /// Priority for queue ordering
    pub priority: u8,

    /// Fallback models if primary fails
    pub fallbacks: Vec<String>,

    /// Why this model was selected
    pub reason: RoutingReason,

    /// Estimated time to first token
    pub estimated_ttft_ms: u64,

    /// Request classification
    pub task_class: TaskClass,
}

/// Reason for routing decision
#[derive(Clone, Debug)]
pub enum RoutingReason {
    /// User explicitly requested this model
    UserRequested,
    /// Best match for task class
    BestForTaskClass { score: f32 },
    /// Session affinity (same model as previous requests)
    SessionAffinity,
    /// Only available option
    OnlyAvailable,
    /// Default model for this task class
    DefaultModel,
    /// Fallback from unavailable primary
    Fallback { from_model: String },
}

// ============================================================================
// Model State Tracking
// ============================================================================

/// Runtime state of a model
#[derive(Clone, Debug)]
pub struct ModelState {
    /// Model ID
    pub model_id: String,
    /// Whether the model is healthy
    pub healthy: bool,
    /// Whether the model is loaded (for local models)
    pub loaded: bool,
    /// Current queue depth
    pub queue_depth: usize,
    /// Active request count
    pub active_requests: usize,
    /// Recent average TTFT (exponential moving average)
    pub avg_ttft_ms: f64,
    /// Recent average tokens per second
    pub avg_tokens_per_sec: f64,
    /// Recent error rate (0.0 - 1.0)
    pub error_rate: f64,
    /// Last successful request time
    pub last_success: Option<Instant>,
    /// Last error time
    pub last_error: Option<Instant>,
    /// Consecutive failures
    pub consecutive_failures: u32,
}

impl ModelState {
    /// Create new state for a model
    #[must_use]
    pub fn new(model_id: String) -> Self {
        Self {
            model_id,
            healthy: true,
            loaded: false,
            queue_depth: 0,
            active_requests: 0,
            avg_ttft_ms: 1000.0,
            avg_tokens_per_sec: 20.0,
            error_rate: 0.0,
            last_success: None,
            last_error: None,
            consecutive_failures: 0,
        }
    }

    /// Record a successful request
    pub fn record_success(&mut self, ttft_ms: u64, tokens_per_sec: f64) {
        // Exponential moving average (alpha = 0.3)
        const ALPHA: f64 = 0.3;
        self.avg_ttft_ms = ALPHA * ttft_ms as f64 + (1.0 - ALPHA) * self.avg_ttft_ms;
        self.avg_tokens_per_sec = ALPHA * tokens_per_sec + (1.0 - ALPHA) * self.avg_tokens_per_sec;
        self.error_rate *= (1.0 - ALPHA); // Decay error rate

        self.last_success = Some(Instant::now());
        self.consecutive_failures = 0;
        self.healthy = true;
    }

    /// Record a failed request
    pub fn record_failure(&mut self) {
        const ALPHA: f64 = 0.3;
        self.error_rate = ALPHA + (1.0 - ALPHA) * self.error_rate;

        self.last_error = Some(Instant::now());
        self.consecutive_failures += 1;

        // Mark unhealthy after 3 consecutive failures
        if self.consecutive_failures >= 3 {
            self.healthy = false;
        }
    }

    /// Check if model can be used
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.healthy && self.error_rate < 0.5
    }

    /// Get estimated wait time based on queue depth
    #[must_use]
    pub fn estimated_wait_ms(&self) -> u64 {
        // Rough estimate: each queued request adds ~avg_ttft_ms
        (self.queue_depth as f64 * self.avg_ttft_ms) as u64
    }
}

// ============================================================================
// Routing Policy
// ============================================================================

/// Routing policy implementation
pub struct RoutingPolicy {
    /// Model profiles
    profiles: RwLock<HashMap<String, ModelProfile>>,
    /// Runtime model states
    states: RwLock<HashMap<String, ModelState>>,
    /// Default models by task class
    defaults: RwLock<HashMap<TaskClass, String>>,
    /// Fallback chains
    fallbacks: RwLock<HashMap<String, Vec<String>>>,
    /// Session affinity: `conversation_id` -> `last_model`
    session_affinity: RwLock<HashMap<String, SessionAffinity>>,
    /// Metrics reference
    metrics: Option<Arc<RouterMetrics>>,
}

/// Session affinity tracking
#[derive(Clone, Debug)]
struct SessionAffinity {
    model_id: String,
    last_used: Instant,
    request_count: u64,
}

impl RoutingPolicy {
    /// Create a new routing policy
    #[must_use]
    pub fn new() -> Self {
        Self {
            profiles: RwLock::new(HashMap::new()),
            states: RwLock::new(HashMap::new()),
            defaults: RwLock::new(HashMap::new()),
            fallbacks: RwLock::new(HashMap::new()),
            session_affinity: RwLock::new(HashMap::new()),
            metrics: None,
        }
    }

    /// Create with metrics
    pub fn with_metrics(metrics: Arc<RouterMetrics>) -> Self {
        Self {
            profiles: RwLock::new(HashMap::new()),
            states: RwLock::new(HashMap::new()),
            defaults: RwLock::new(HashMap::new()),
            fallbacks: RwLock::new(HashMap::new()),
            session_affinity: RwLock::new(HashMap::new()),
            metrics: Some(metrics),
        }
    }

    /// Register a model profile
    pub async fn register_model(&self, profile: ModelProfile) {
        let model_id = profile.model_id.clone();
        {
            let mut profiles = self.profiles.write().await;
            profiles.insert(model_id.clone(), profile);
        }
        {
            let mut states = self.states.write().await;
            states
                .entry(model_id)
                .or_insert_with_key(|id| ModelState::new(id.clone()));
        }
    }

    /// Set default model for a task class
    pub async fn set_default(&self, task_class: TaskClass, model_id: String) {
        let mut defaults = self.defaults.write().await;
        defaults.insert(task_class, model_id);
    }

    /// Set fallback chain for a model
    pub async fn set_fallbacks(&self, model_id: String, fallbacks: Vec<String>) {
        let mut fb = self.fallbacks.write().await;
        fb.insert(model_id, fallbacks);
    }

    /// Make a routing decision
    pub async fn route(&self, request: &RoutingRequest) -> Result<RoutingDecision, RoutingError> {
        let task_class = request.classify();
        let priority = request.effective_priority();
        let timeout = request.effective_timeout();

        // 1. Check for explicitly requested model
        if let Some(ref requested) = request.requested_model {
            if let Some(decision) = self
                .try_route_to(
                    requested,
                    task_class,
                    priority,
                    timeout,
                    RoutingReason::UserRequested,
                )
                .await
            {
                return Ok(decision);
            }
            // Requested model not available, fall through to routing
        }

        // 2. Check session affinity
        if let Some(ref conv_id) = request.conversation_id {
            if let Some(decision) = self
                .try_session_affinity(conv_id, task_class, priority, timeout)
                .await
            {
                return Ok(decision);
            }
        }

        // 3. Get all available candidates
        let candidates = self.get_candidates(request).await;
        if candidates.is_empty() {
            return Err(RoutingError::NoModelsAvailable);
        }

        // 4. Score and select best candidate
        let (model_id, score) = self.select_best(&candidates, task_class, request).await;

        // 5. Get fallbacks
        let fallbacks = self.get_fallbacks_for(&model_id).await;

        // 6. Get profile for the selected model
        let profiles = self.profiles.read().await;
        let profile = profiles
            .get(&model_id)
            .ok_or_else(|| RoutingError::ModelNotFound(model_id.clone()))?;

        Ok(RoutingDecision {
            model_id: model_id.clone(),
            backend_id: profile.backend_id.clone(),
            timeout,
            priority,
            fallbacks,
            reason: RoutingReason::BestForTaskClass { score },
            estimated_ttft_ms: profile.avg_ttft_ms,
            task_class,
        })
    }

    /// Try to route to a specific model
    async fn try_route_to(
        &self,
        model_id: &str,
        task_class: TaskClass,
        priority: u8,
        timeout: Duration,
        reason: RoutingReason,
    ) -> Option<RoutingDecision> {
        let states = self.states.read().await;
        let state = states.get(model_id)?;

        if !state.is_available() {
            return None;
        }

        let profiles = self.profiles.read().await;
        let profile = profiles.get(model_id)?;

        let fallbacks = self.get_fallbacks_for(model_id).await;

        Some(RoutingDecision {
            model_id: model_id.to_string(),
            backend_id: profile.backend_id.clone(),
            timeout,
            priority,
            fallbacks,
            reason,
            estimated_ttft_ms: profile.avg_ttft_ms,
            task_class,
        })
    }

    /// Try session affinity routing
    async fn try_session_affinity(
        &self,
        conversation_id: &str,
        task_class: TaskClass,
        priority: u8,
        timeout: Duration,
    ) -> Option<RoutingDecision> {
        let affinities = self.session_affinity.read().await;
        let affinity = affinities.get(conversation_id)?;

        // Only use affinity if used recently (last 5 minutes)
        if affinity.last_used.elapsed() > Duration::from_secs(300) {
            return None;
        }

        self.try_route_to(
            &affinity.model_id,
            task_class,
            priority,
            timeout,
            RoutingReason::SessionAffinity,
        )
        .await
    }

    /// Get candidate models for a request
    async fn get_candidates(&self, request: &RoutingRequest) -> Vec<(String, ModelState)> {
        let states = self.states.read().await;
        let profiles = self.profiles.read().await;
        let task_class = request.classify();

        states
            .iter()
            .filter(|(id, state)| {
                if !state.is_available() {
                    return false;
                }

                // Check profile requirements
                if let Some(profile) = profiles.get(*id) {
                    // Check streaming requirement
                    if request.requires_streaming && !profile.supports_streaming {
                        return false;
                    }

                    // Check tool requirement
                    if request.requires_tools && !profile.supports_tools {
                        return false;
                    }

                    // Check cost tier
                    if let Some(max_tier) = request.max_cost_tier {
                        if (profile.cost_tier as u8) > (max_tier as u8) {
                            return false;
                        }
                    }

                    // Check latency requirement
                    if !profile.can_meet_latency(task_class) {
                        // For quick response, strictly filter; for others, allow with penalty
                        if task_class == TaskClass::QuickResponse {
                            return false;
                        }
                    }
                }

                true
            })
            .map(|(id, state)| (id.clone(), state.clone()))
            .collect()
    }

    /// Score and select the best model from candidates
    #[allow(unused_variables)]
    async fn select_best(
        &self,
        candidates: &[(String, ModelState)],
        task_class: TaskClass,
        request: &RoutingRequest,
    ) -> (String, f32) {
        let profiles = self.profiles.read().await;
        let defaults = self.defaults.read().await;

        let mut best_model = candidates[0].0.clone();
        let mut best_score = f32::MIN;

        for (model_id, state) in candidates {
            let mut score = 0.0_f32;

            // Base affinity score from profile
            if let Some(profile) = profiles.get(model_id) {
                score += profile.affinity_for(task_class) * 100.0;

                // Latency score (lower is better)
                let target_ttft = task_class.target_ttft().as_millis() as f64;
                let latency_ratio = state.avg_ttft_ms / target_ttft;
                score -= (latency_ratio * 10.0) as f32;

                // Error rate penalty
                score -= (state.error_rate * 50.0) as f32;

                // Queue depth penalty
                score -= (state.queue_depth as f32) * 2.0;

                // Cost tier penalty (for budget-conscious routing)
                score -= f32::from(profile.cost_tier as u8) * 5.0;

                // Bonus for default model
                if defaults.get(&task_class) == Some(model_id) {
                    score += 20.0;
                }

                // Loaded bonus (for local models)
                if state.loaded {
                    score += 15.0;
                }
            }

            if score > best_score {
                best_score = score;
                best_model = model_id.clone();
            }
        }

        (best_model, best_score)
    }

    /// Get fallback models for a given model
    async fn get_fallbacks_for(&self, model_id: &str) -> Vec<String> {
        let fallbacks = self.fallbacks.read().await;
        fallbacks.get(model_id).cloned().unwrap_or_default()
    }

    /// Record session affinity
    pub async fn record_session_affinity(&self, conversation_id: &str, model_id: &str) {
        let mut affinities = self.session_affinity.write().await;
        affinities
            .entry(conversation_id.to_string())
            .and_modify(|a| {
                a.model_id = model_id.to_string();
                a.last_used = Instant::now();
                a.request_count += 1;
            })
            .or_insert(SessionAffinity {
                model_id: model_id.to_string(),
                last_used: Instant::now(),
                request_count: 1,
            });
    }

    /// Update model state after a request
    pub async fn record_request_result(
        &self,
        model_id: &str,
        success: bool,
        ttft_ms: Option<u64>,
        tokens_per_sec: Option<f64>,
    ) {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(model_id) {
            if success {
                state.record_success(ttft_ms.unwrap_or(1000), tokens_per_sec.unwrap_or(20.0));
            } else {
                state.record_failure();
            }
        }
    }

    /// Update queue depth for a model
    pub async fn update_queue_depth(&self, model_id: &str, depth: usize) {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(model_id) {
            state.queue_depth = depth;
        }
    }

    /// Update active request count for a model
    pub async fn update_active_requests(&self, model_id: &str, count: usize) {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(model_id) {
            state.active_requests = count;
        }
    }

    /// Mark model as loaded (for local models)
    pub async fn mark_loaded(&self, model_id: &str, loaded: bool) {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(model_id) {
            state.loaded = loaded;
        }
    }

    /// Get model state
    pub async fn get_state(&self, model_id: &str) -> Option<ModelState> {
        let states = self.states.read().await;
        states.get(model_id).cloned()
    }

    /// Get all model states
    pub async fn all_states(&self) -> Vec<ModelState> {
        let states = self.states.read().await;
        states.values().cloned().collect()
    }

    /// Clean up old session affinities
    pub async fn cleanup_stale_affinities(&self, max_age: Duration) {
        let mut affinities = self.session_affinity.write().await;
        affinities.retain(|_, a| a.last_used.elapsed() < max_age);
    }
}

impl Default for RoutingPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// Routing errors
#[derive(Clone, Debug)]
pub enum RoutingError {
    /// No models available for this request
    NoModelsAvailable,
    /// Requested model not found
    ModelNotFound(String),
    /// All models are unhealthy
    AllModelsUnhealthy,
    /// Request cannot be satisfied (e.g., requires tools but no tool-capable model)
    CannotSatisfy(String),
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoModelsAvailable => write!(f, "No models available"),
            Self::ModelNotFound(id) => write!(f, "Model not found: {id}"),
            Self::AllModelsUnhealthy => write!(f, "All models are unhealthy"),
            Self::CannotSatisfy(reason) => write!(f, "Cannot satisfy request: {reason}"),
        }
    }
}

impl std::error::Error for RoutingError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_classification() {
        let quick = RoutingRequest::new("Hi there!");
        assert_eq!(quick.classify(), TaskClass::QuickResponse);

        let code = RoutingRequest::new("Write a function to calculate factorial");
        assert_eq!(code.classify(), TaskClass::CodeGeneration);

        let math = RoutingRequest::new("Solve the equation x^2 + 5x - 6 = 0");
        assert_eq!(math.classify(), TaskClass::Mathematical);

        let deep = RoutingRequest::new("Analyze the economic implications of this policy and compare and contrast with historical precedents...");
        assert_eq!(deep.classify(), TaskClass::DeepThinking);
    }

    #[test]
    fn test_priority_calculation() {
        let urgent = RoutingRequest::new("Quick!")
            .with_task_class(TaskClass::QuickResponse)
            .with_urgency(10);
        assert!(urgent.effective_priority() >= 95);

        let leisurely = RoutingRequest::new("Think deeply...")
            .with_task_class(TaskClass::DeepThinking)
            .with_urgency(1);
        assert!(leisurely.effective_priority() <= 20);
    }

    #[tokio::test]
    async fn test_routing_policy() {
        let policy = RoutingPolicy::new();

        // Register a model with streaming support
        let mut profile = ModelProfile::new("test-model", "ollama");
        profile.supports_streaming = true;
        profile.avg_ttft_ms = 500; // Set a reasonable TTFT
        policy.register_model(profile).await;
        policy
            .set_default(TaskClass::General, "test-model".to_string())
            .await;

        // Verify model state is available
        let state = policy.get_state("test-model").await;
        assert!(state.is_some(), "Model state should exist");
        assert!(state.unwrap().is_available(), "Model should be available");

        // Route a general request (explicitly set task class to avoid QuickResponse filtering)
        let request =
            RoutingRequest::new("Explain how computers work").with_task_class(TaskClass::General);
        let decision = policy.route(&request).await;

        assert!(decision.is_ok(), "Routing should succeed: {:?}", decision);
        let decision = decision.unwrap();
        assert_eq!(decision.model_id, "test-model");
        assert_eq!(decision.backend_id, "ollama");
    }

    #[test]
    fn test_model_state_tracking() {
        let mut state = ModelState::new("test".to_string());

        // Initially healthy
        assert!(state.is_available());
        assert!(state.healthy);
        assert!(state.error_rate < 0.01);

        // Record one failure
        state.record_failure();
        assert!(state.healthy); // Still marked healthy after 1 failure
        assert_eq!(state.consecutive_failures, 1);

        // After 3 consecutive failures, model becomes unhealthy
        state.record_failure();
        state.record_failure();
        assert!(!state.healthy); // Unhealthy after 3 consecutive failures
        assert!(!state.is_available());

        // Success resets consecutive failures and health
        state.record_success(100, 50.0);
        assert!(state.healthy);
        assert_eq!(state.consecutive_failures, 0);
    }
}
