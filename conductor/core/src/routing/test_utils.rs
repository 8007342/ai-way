//! Multi-Model Test Utilities
//!
//! Provides mock infrastructure for testing multi-model routing without actual LLM calls.
//! This module implements the `MultiModelMockBackend` which supports multiple model
//! configurations, request history tracking, and simulated failures for testing
//! fallback behavior.
//!
//! # Usage
//!
//! ```ignore
//! use conductor_core::routing::test_utils::{MultiModelMockBackend, ModelCategory};
//!
//! let backend = MultiModelMockBackend::with_standard_models();
//!
//! // Simulate model failure for fallback testing
//! backend.set_unavailable("code-llama");
//!
//! // After test, verify which models were called
//! assert_eq!(backend.request_count("deep-70b"), 1);
//! ```

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::backend::{LlmBackend, LlmRequest, LlmResponse, ModelInfo, StreamingToken};

// ============================================================================
// Model Categories
// ============================================================================

/// Classification of models for routing decisions
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ModelCategory {
    /// Specialized: always routes to specific model for code tasks
    Code,
    /// Specialized: always routes to specific model for math tasks
    Math,
    /// General: meta-agent selects for quick responses
    Quick,
    /// General: meta-agent selects for deep thinking
    Deep,
    /// General: meta-agent selects for creative tasks
    Creative,
    /// Default fallback model
    General,
}

impl std::fmt::Display for ModelCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelCategory::Code => write!(f, "Code"),
            ModelCategory::Math => write!(f, "Math"),
            ModelCategory::Quick => write!(f, "Quick"),
            ModelCategory::Deep => write!(f, "Deep"),
            ModelCategory::Creative => write!(f, "Creative"),
            ModelCategory::General => write!(f, "General"),
        }
    }
}

// ============================================================================
// Mock Model Configuration
// ============================================================================

/// Configuration for a mock model
pub struct MockModelConfig {
    /// Model identifier (e.g., "code-llama", "math-wizard")
    pub name: String,
    /// Category for routing verification
    pub category: ModelCategory,
    /// Response generator function
    pub response_fn: Box<dyn Fn(&LlmRequest) -> Vec<String> + Send + Sync>,
    /// Simulated token delay (ms)
    pub token_delay_ms: u64,
    /// Whether model is "loaded" (for warmup testing)
    pub loaded: bool,
}

impl std::fmt::Debug for MockModelConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockModelConfig")
            .field("name", &self.name)
            .field("category", &self.category)
            .field("token_delay_ms", &self.token_delay_ms)
            .field("loaded", &self.loaded)
            .finish()
    }
}

// ============================================================================
// Request Tracking
// ============================================================================

/// Captured request for test verification
#[derive(Clone, Debug)]
pub struct ModelRequest {
    /// Model that received the request
    pub model: String,
    /// The prompt sent to the model
    pub prompt: String,
    /// Timestamp when request was received
    pub timestamp: Instant,
    /// Category hint from routing (if available)
    pub category_hint: Option<ModelCategory>,
}

// ============================================================================
// Multi-Model Mock Backend
// ============================================================================

/// Mock backend supporting multiple model configurations
///
/// This backend allows testing of multi-model routing scenarios without
/// requiring actual LLM inference. It supports:
/// - Multiple model personalities with different response patterns
/// - Simulated latencies per model
/// - Request history tracking for verification
/// - Model availability toggling for fallback testing
pub struct MultiModelMockBackend {
    /// Available models with their configurations
    models: HashMap<String, MockModelConfig>,
    /// Request history for verification
    request_history: Arc<Mutex<Vec<ModelRequest>>>,
    /// Simulated latencies per model (overrides config if set)
    latencies: HashMap<String, Duration>,
    /// Models currently "unavailable" for fallback testing
    unavailable_models: Arc<Mutex<HashSet<String>>>,
    /// Request counter per model
    request_counts: Arc<Mutex<HashMap<String, usize>>>,
    /// Default model to use if requested model not found
    default_model: String,
}

impl Clone for MultiModelMockBackend {
    fn clone(&self) -> Self {
        // Note: models HashMap cannot be cloned due to Box<dyn Fn>
        // For testing purposes, we share the state via Arc
        Self {
            models: HashMap::new(), // Models are not cloned
            request_history: Arc::clone(&self.request_history),
            latencies: self.latencies.clone(),
            unavailable_models: Arc::clone(&self.unavailable_models),
            request_counts: Arc::clone(&self.request_counts),
            default_model: self.default_model.clone(),
        }
    }
}

impl std::fmt::Debug for MultiModelMockBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiModelMockBackend")
            .field("model_count", &self.models.len())
            .field("default_model", &self.default_model)
            .finish()
    }
}

impl MultiModelMockBackend {
    /// Create an empty backend
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            request_history: Arc::new(Mutex::new(Vec::new())),
            latencies: HashMap::new(),
            unavailable_models: Arc::new(Mutex::new(HashSet::new())),
            request_counts: Arc::new(Mutex::new(HashMap::new())),
            default_model: "general-13b".to_string(),
        }
    }

    /// Create with standard model configurations for testing
    ///
    /// Provides the following models:
    /// - `code-llama`: Code-focused responses with Rust examples
    /// - `math-wizard`: Math-focused responses with LaTeX formulas
    /// - `quick-7b`: Fast, concise responses
    /// - `deep-70b`: Thorough, multi-step responses
    /// - `creative-writer`: Creative, narrative responses
    /// - `general-13b`: General fallback responses
    pub fn with_standard_models() -> Self {
        let mut backend = Self::new();

        // Code model - returns code-like responses
        backend.add_model(MockModelConfig {
            name: "code-llama".to_string(),
            category: ModelCategory::Code,
            response_fn: Box::new(|_req| {
                vec![
                    "[yolla:mood focused]".to_string(),
                    "```rust\n".to_string(),
                    "fn example() -> Result<(), Box<dyn std::error::Error>> {\n".to_string(),
                    "    println!(\"Hello, code!\");\n".to_string(),
                    "    Ok(())\n".to_string(),
                    "}\n```".to_string(),
                ]
            }),
            token_delay_ms: 5,
            loaded: true,
        });

        // Math model - returns LaTeX/calculation responses
        backend.add_model(MockModelConfig {
            name: "math-wizard".to_string(),
            category: ModelCategory::Math,
            response_fn: Box::new(|_req| {
                vec![
                    "[yolla:mood thinking]".to_string(),
                    "The answer is: ".to_string(),
                    "$x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}$".to_string(),
                    "\n\nThis is the quadratic formula.".to_string(),
                ]
            }),
            token_delay_ms: 10,
            loaded: true,
        });

        // Quick model - fast, concise responses
        backend.add_model(MockModelConfig {
            name: "quick-7b".to_string(),
            category: ModelCategory::Quick,
            response_fn: Box::new(|_req| vec!["Quick answer: Yes, that's correct.".to_string()]),
            token_delay_ms: 2,
            loaded: true,
        });

        // Deep model - thorough responses
        backend.add_model(MockModelConfig {
            name: "deep-70b".to_string(),
            category: ModelCategory::Deep,
            response_fn: Box::new(|_req| {
                vec![
                    "[yolla:mood thinking]".to_string(),
                    "Let me analyze this thoroughly. ".to_string(),
                    "First, we need to consider the fundamental principles. ".to_string(),
                    "Second, the implications suggest that... ".to_string(),
                    "In conclusion, the answer requires careful consideration.".to_string(),
                ]
            }),
            token_delay_ms: 20,
            loaded: true,
        });

        // Creative model - narrative responses
        backend.add_model(MockModelConfig {
            name: "creative-writer".to_string(),
            category: ModelCategory::Creative,
            response_fn: Box::new(|_req| {
                vec![
                    "[yolla:mood happy]".to_string(),
                    "Once upon a time, ".to_string(),
                    "in a world of endless possibilities, ".to_string(),
                    "there lived a creative spark of imagination...".to_string(),
                ]
            }),
            token_delay_ms: 8,
            loaded: true,
        });

        // General fallback
        backend.add_model(MockModelConfig {
            name: "general-13b".to_string(),
            category: ModelCategory::General,
            response_fn: Box::new(|_req| {
                vec![
                    "I can help with that. ".to_string(),
                    "Here's what I know about your question.".to_string(),
                ]
            }),
            token_delay_ms: 5,
            loaded: true,
        });

        backend
    }

    /// Add a model configuration
    pub fn add_model(&mut self, config: MockModelConfig) {
        self.models.insert(config.name.clone(), config);
    }

    /// Set custom latency for a model (overrides config)
    pub fn set_latency(&mut self, model: &str, latency: Duration) {
        self.latencies.insert(model.to_string(), latency);
    }

    /// Mark a model as unavailable (for fallback testing)
    pub fn set_unavailable(&self, model: &str) {
        self.unavailable_models
            .lock()
            .unwrap()
            .insert(model.to_string());
    }

    /// Mark a model as available again
    pub fn set_available(&self, model: &str) {
        self.unavailable_models.lock().unwrap().remove(model);
    }

    /// Check if a model is marked as unavailable
    pub fn is_unavailable(&self, model: &str) -> bool {
        self.unavailable_models.lock().unwrap().contains(model)
    }

    /// Get request count for a specific model
    pub fn request_count(&self, model: &str) -> usize {
        *self.request_counts.lock().unwrap().get(model).unwrap_or(&0)
    }

    /// Get total request count across all models
    pub fn total_request_count(&self) -> usize {
        self.request_counts.lock().unwrap().values().sum()
    }

    /// Get full request history
    pub fn request_history(&self) -> Vec<ModelRequest> {
        self.request_history.lock().unwrap().clone()
    }

    /// Get the last request sent to a specific model
    pub fn last_request_for(&self, model: &str) -> Option<ModelRequest> {
        self.request_history
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|r| r.model == model)
            .cloned()
    }

    /// Clear request history (between test cases)
    pub fn clear_history(&self) {
        self.request_history.lock().unwrap().clear();
        self.request_counts.lock().unwrap().clear();
    }

    /// Get the category for a model
    pub fn model_category(&self, model: &str) -> Option<ModelCategory> {
        self.models.get(model).map(|c| c.category)
    }

    /// Get list of all configured model names
    pub fn model_names(&self) -> Vec<String> {
        self.models.keys().cloned().collect()
    }

    /// Set the default model to use when requested model is not found
    pub fn set_default_model(&mut self, model: &str) {
        self.default_model = model.to_string();
    }

    /// Record a request (called internally by send methods)
    fn record_request(&self, model: &str, prompt: &str) {
        // Record in history
        self.request_history.lock().unwrap().push(ModelRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            timestamp: Instant::now(),
            category_hint: self.model_category(model),
        });

        // Increment request count
        *self
            .request_counts
            .lock()
            .unwrap()
            .entry(model.to_string())
            .or_insert(0) += 1;
    }

    /// Get effective delay for a model
    fn get_delay(&self, model: &str) -> Duration {
        if let Some(latency) = self.latencies.get(model) {
            *latency
        } else if let Some(config) = self.models.get(model) {
            Duration::from_millis(config.token_delay_ms)
        } else {
            Duration::from_millis(5)
        }
    }

    /// Generate tokens for a request
    fn generate_tokens(&self, request: &LlmRequest) -> Vec<String> {
        let model = &request.model;

        if let Some(config) = self.models.get(model) {
            (config.response_fn)(request)
        } else if let Some(default_config) = self.models.get(&self.default_model) {
            (default_config.response_fn)(request)
        } else {
            vec!["Default response from mock backend.".to_string()]
        }
    }
}

impl Default for MultiModelMockBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmBackend for MultiModelMockBackend {
    fn name(&self) -> &str {
        "MultiModelMock"
    }

    async fn health_check(&self) -> bool {
        true
    }

    async fn send_streaming(
        &self,
        request: &LlmRequest,
    ) -> anyhow::Result<mpsc::Receiver<StreamingToken>> {
        let model = request.model.clone();

        // Record the request
        self.record_request(&model, &request.prompt);

        // Check if model is unavailable
        if self.is_unavailable(&model) {
            anyhow::bail!("Model {} is unavailable", model);
        }

        // Generate tokens
        let tokens = self.generate_tokens(request);
        let delay = self.get_delay(&model);

        let (tx, rx) = mpsc::channel(10);

        tokio::spawn(async move {
            let mut full_message = String::new();
            for token in tokens {
                full_message.push_str(&token);
                if tx.send(StreamingToken::Token(token)).await.is_err() {
                    // Receiver dropped, stop streaming
                    return;
                }
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
            }
            let _ = tx
                .send(StreamingToken::Complete {
                    message: full_message,
                })
                .await;
        });

        Ok(rx)
    }

    async fn send(&self, request: &LlmRequest) -> anyhow::Result<LlmResponse> {
        let model = request.model.clone();

        // Record the request
        self.record_request(&model, &request.prompt);

        // Check if model is unavailable
        if self.is_unavailable(&model) {
            anyhow::bail!("Model {} is unavailable", model);
        }

        // Generate response
        let tokens = self.generate_tokens(request);
        let content = tokens.join("");

        // Simulate delay
        let delay = self.get_delay(&model);
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }

        Ok(LlmResponse {
            content,
            model,
            tokens_used: Some(tokens.len() as u32),
            duration_ms: Some(delay.as_millis() as u64 * tokens.len() as u64),
        })
    }

    async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        let unavailable = self.unavailable_models.lock().unwrap();

        Ok(self
            .models
            .values()
            .filter(|m| !unavailable.contains(&m.name))
            .map(|m| ModelInfo {
                name: m.name.clone(),
                description: Some(format!("{} model (mock)", m.category)),
                size: None,
                parameters: None,
                loaded: m.loaded,
            })
            .collect())
    }
}

// ============================================================================
// Test Utility Functions
// ============================================================================

/// Assert that a specific model received exactly N requests
pub fn assert_model_called(backend: &MultiModelMockBackend, model: &str, times: usize) {
    let actual = backend.request_count(model);
    assert_eq!(
        actual, times,
        "Expected '{}' to be called {} times, but was called {} times",
        model, times, actual
    );
}

/// Assert that only the specified model was called (no other models)
pub fn assert_only_model_called(backend: &MultiModelMockBackend, model: &str) {
    let history = backend.request_history();
    for req in &history {
        assert_eq!(
            req.model, model,
            "Expected only '{}' to be called, but '{}' was also called",
            model, req.model
        );
    }
}

/// Assert that no models were called
pub fn assert_no_models_called(backend: &MultiModelMockBackend) {
    let total = backend.total_request_count();
    assert_eq!(
        total, 0,
        "Expected no models to be called, but {} calls were made",
        total
    );
}

/// Assert that models were called in a specific order
pub fn assert_call_order(backend: &MultiModelMockBackend, expected_order: &[&str]) {
    let history = backend.request_history();
    let actual_order: Vec<&str> = history.iter().map(|r| r.model.as_str()).collect();

    assert_eq!(
        actual_order.len(),
        expected_order.len(),
        "Expected {} calls, got {}. Actual order: {:?}",
        expected_order.len(),
        actual_order.len(),
        actual_order
    );

    for (i, (actual, expected)) in actual_order.iter().zip(expected_order.iter()).enumerate() {
        assert_eq!(
            actual, expected,
            "Call {} was to '{}', expected '{}'",
            i, actual, expected
        );
    }
}

// ============================================================================
// Selection Context for Routing Tests
// ============================================================================

/// Context information for model selection decisions
#[derive(Clone, Debug, Default)]
pub struct SelectionContext {
    /// User explicitly requested this model
    pub user_requested_model: Option<String>,
    /// Previous model used in conversation
    pub previous_model: Option<String>,
    /// Detected topic of conversation
    pub conversation_topic: Option<String>,
    /// Whether this is a follow-up message
    pub is_followup: bool,
}

// ============================================================================
// Model Selection Types
// ============================================================================

/// Result of model selection
#[derive(Clone, Debug)]
pub struct ModelSelection {
    /// Selected model name
    pub model: String,
    /// Why this model was selected
    pub reason: SelectionReason,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Fallback if primary fails
    pub fallback: Option<String>,
}

/// Reason for selecting a particular model
#[derive(Clone, Debug, PartialEq)]
pub enum SelectionReason {
    /// Specialized model required (code, math)
    Specialized(ModelCategory),
    /// Meta-agent classification result
    MetaAgentDecision {
        /// Classification label
        classification: String,
    },
    /// User explicitly requested model
    UserRequested,
    /// Fallback due to unavailability
    Fallback {
        /// Original model that was unavailable
        original: String,
        /// Reason for fallback
        reason: String,
    },
    /// Default model (no specific routing)
    Default,
}

// ============================================================================
// Simple Query Classifier for Tests
// ============================================================================

/// Simple keyword-based classifier for testing model routing
///
/// This is a simple implementation for testing purposes. Real routing
/// would use more sophisticated classification.
pub struct SimpleQueryClassifier;

impl SimpleQueryClassifier {
    /// Classify a query and return suggested model
    pub fn classify(query: &str, context: &SelectionContext) -> ModelSelection {
        let query_lower = query.to_lowercase();

        // Check user override first
        if let Some(ref user_model) = context.user_requested_model {
            return ModelSelection {
                model: user_model.clone(),
                reason: SelectionReason::UserRequested,
                confidence: 1.0,
                fallback: Some("general-13b".to_string()),
            };
        }

        // Check for code-related keywords
        let code_keywords = [
            "rust",
            "python",
            "javascript",
            "code",
            "function",
            "implement",
            "debug",
            "refactor",
            "compile",
            "syntax",
            "algorithm",
            "class",
            "struct",
            "enum",
            "def ",
            "fn ",
            "async",
            "```",
        ];
        if code_keywords.iter().any(|k| query_lower.contains(k)) {
            return ModelSelection {
                model: "code-llama".to_string(),
                reason: SelectionReason::Specialized(ModelCategory::Code),
                confidence: 0.9,
                fallback: Some("deep-70b".to_string()),
            };
        }

        // Check for math-related keywords
        let math_keywords = [
            "calculate",
            "solve",
            "equation",
            "integral",
            "derivative",
            "formula",
            "math",
            "prove",
            "theorem",
            "sqrt",
            "^2",
            "= 0",
            "quadratic",
            "x =",
            "y =",
        ];
        if math_keywords.iter().any(|k| query_lower.contains(k)) {
            return ModelSelection {
                model: "math-wizard".to_string(),
                reason: SelectionReason::Specialized(ModelCategory::Math),
                confidence: 0.85,
                fallback: Some("deep-70b".to_string()),
            };
        }

        // Check for creative keywords
        let creative_keywords = [
            "story",
            "poem",
            "write",
            "creative",
            "imagine",
            "tale",
            "fiction",
            "narrative",
            "haiku",
            "song",
        ];
        if creative_keywords.iter().any(|k| query_lower.contains(k)) {
            return ModelSelection {
                model: "creative-writer".to_string(),
                reason: SelectionReason::MetaAgentDecision {
                    classification: "creative".to_string(),
                },
                confidence: 0.8,
                fallback: Some("general-13b".to_string()),
            };
        }

        // Check for deep thinking indicators
        if query.len() > 200 || query_lower.contains("explain") || query_lower.contains("analyze") {
            return ModelSelection {
                model: "deep-70b".to_string(),
                reason: SelectionReason::MetaAgentDecision {
                    classification: "complex".to_string(),
                },
                confidence: 0.7,
                fallback: Some("general-13b".to_string()),
            };
        }

        // Check conversation context for follow-ups
        if context.is_followup {
            if let Some(ref prev_model) = context.previous_model {
                return ModelSelection {
                    model: prev_model.clone(),
                    reason: SelectionReason::MetaAgentDecision {
                        classification: "followup".to_string(),
                    },
                    confidence: 0.6,
                    fallback: Some("general-13b".to_string()),
                };
            }
        }

        // Short queries go to quick model
        if query.len() < 50 {
            return ModelSelection {
                model: "quick-7b".to_string(),
                reason: SelectionReason::MetaAgentDecision {
                    classification: "simple".to_string(),
                },
                confidence: 0.65,
                fallback: Some("general-13b".to_string()),
            };
        }

        // Default to general model
        ModelSelection {
            model: "general-13b".to_string(),
            reason: SelectionReason::Default,
            confidence: 0.5,
            fallback: None,
        }
    }

    /// Get fallback chain for a model
    pub fn fallback_chain(model: &str) -> Vec<String> {
        match model {
            "code-llama" => vec!["deep-70b".to_string(), "general-13b".to_string()],
            "math-wizard" => vec!["deep-70b".to_string(), "general-13b".to_string()],
            "creative-writer" => vec!["general-13b".to_string()],
            "quick-7b" => vec!["general-13b".to_string()],
            "deep-70b" => vec!["general-13b".to_string()],
            _ => vec![],
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_multi_model_backend_creation() {
        let backend = MultiModelMockBackend::with_standard_models();
        assert_eq!(backend.name(), "MultiModelMock");
        assert!(backend.health_check().await);
    }

    #[tokio::test]
    async fn test_model_request_tracking() {
        let backend = MultiModelMockBackend::with_standard_models();

        let request = LlmRequest::new("test prompt", "code-llama");
        let _ = backend.send(&request).await;

        assert_eq!(backend.request_count("code-llama"), 1);
        assert_eq!(backend.total_request_count(), 1);

        let history = backend.request_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].model, "code-llama");
        assert_eq!(history[0].prompt, "test prompt");
    }

    #[tokio::test]
    async fn test_unavailable_model_fails() {
        let backend = MultiModelMockBackend::with_standard_models();
        backend.set_unavailable("code-llama");

        let request = LlmRequest::new("test", "code-llama");
        let result = backend.send(&request).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unavailable"));
    }

    #[tokio::test]
    async fn test_model_availability_toggle() {
        let backend = MultiModelMockBackend::with_standard_models();

        // Initially available
        assert!(!backend.is_unavailable("code-llama"));

        // Mark unavailable
        backend.set_unavailable("code-llama");
        assert!(backend.is_unavailable("code-llama"));

        // Mark available again
        backend.set_available("code-llama");
        assert!(!backend.is_unavailable("code-llama"));
    }

    #[tokio::test]
    async fn test_clear_history() {
        let backend = MultiModelMockBackend::with_standard_models();

        let request = LlmRequest::new("test", "code-llama");
        let _ = backend.send(&request).await;

        assert_eq!(backend.total_request_count(), 1);

        backend.clear_history();

        assert_eq!(backend.total_request_count(), 0);
        assert!(backend.request_history().is_empty());
    }

    #[tokio::test]
    async fn test_streaming_response() {
        let backend = MultiModelMockBackend::with_standard_models();

        let request = LlmRequest::new("test", "code-llama");
        let mut rx = backend.send_streaming(&request).await.unwrap();

        let mut tokens = Vec::new();
        let mut complete = false;

        while let Some(token) = rx.recv().await {
            match token {
                StreamingToken::Token(t) => tokens.push(t),
                StreamingToken::Complete { .. } => {
                    complete = true;
                    break;
                }
                StreamingToken::Error(_) => break,
            }
        }

        assert!(!tokens.is_empty());
        assert!(complete);
    }

    #[test]
    fn test_simple_classifier_code() {
        let selection = SimpleQueryClassifier::classify(
            "Write a Rust function to parse JSON",
            &SelectionContext::default(),
        );

        assert_eq!(selection.model, "code-llama");
        assert!(matches!(
            selection.reason,
            SelectionReason::Specialized(ModelCategory::Code)
        ));
    }

    #[test]
    fn test_simple_classifier_math() {
        let selection = SimpleQueryClassifier::classify(
            "Solve the quadratic equation x^2 + 5x + 6 = 0",
            &SelectionContext::default(),
        );

        assert_eq!(selection.model, "math-wizard");
        assert!(matches!(
            selection.reason,
            SelectionReason::Specialized(ModelCategory::Math)
        ));
    }

    #[test]
    fn test_simple_classifier_user_override() {
        let context = SelectionContext {
            user_requested_model: Some("custom-model".to_string()),
            ..Default::default()
        };

        let selection = SimpleQueryClassifier::classify("any query", &context);

        assert_eq!(selection.model, "custom-model");
        assert!(matches!(selection.reason, SelectionReason::UserRequested));
    }

    #[test]
    fn test_fallback_chain() {
        let chain = SimpleQueryClassifier::fallback_chain("code-llama");
        assert_eq!(chain, vec!["deep-70b", "general-13b"]);

        let chain = SimpleQueryClassifier::fallback_chain("general-13b");
        assert!(chain.is_empty());
    }
}
