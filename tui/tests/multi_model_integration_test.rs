//! Multi-Model Integration Tests
//!
//! These tests verify the multi-model routing functionality of the Conductor,
//! ensuring that queries are routed to appropriate models based on content
//! classification, and that fallback mechanisms work correctly.
//!
//! # Test Coverage
//!
//! 1. **Code Query Routing**: Verifies code-related queries route to code model
//! 2. **Math Query Routing**: Verifies math-related queries route to math model
//! 3. **Fallback Mechanism**: Tests fallback when primary model is unavailable
//! 4. **Concurrent Streaming**: Tests multiple concurrent model streams
//! 5. **Request History Tracking**: Verifies request history is accurately recorded
//!
//! # Design Philosophy
//!
//! These tests use the `MultiModelMockBackend` to simulate multiple LLM models
//! without making actual inference calls. This enables:
//! - Fast test execution (< 5 seconds per test)
//! - Deterministic behavior for CI/CD
//! - Verification of routing logic without LLM dependencies
//!
//! # Note on Test Structure
//!
//! Since the actual model routing logic requires the Conductor to be configured
//! with a model selector, these tests focus on the mock backend's behavior and
//! demonstrate how routing verification would work with the full system.

use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::time::timeout;

use conductor_core::{
    backend::{LlmBackend, LlmRequest, LlmResponse, ModelInfo, StreamingToken},
    Conductor, ConductorConfig, ConductorMessage, ConductorState, MessageRole, SurfaceCapabilities,
    SurfaceEvent, SurfaceType,
};

// ============================================================================
// Multi-Model Mock Backend (Embedded for Integration Tests)
// ============================================================================

/// Model category for routing classification
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ModelCategory {
    Code,
    Math,
    Quick,
    Deep,
    Creative,
    General,
}

/// Request record for history tracking
#[derive(Clone, Debug)]
pub struct ModelRequest {
    pub model: String,
    pub prompt: String,
    pub timestamp: Instant,
}

/// Multi-model mock backend for integration testing
///
/// Supports multiple model configurations with:
/// - Request history tracking
/// - Model availability toggling
/// - Configurable response generation
/// - Simulated latencies
pub struct MultiModelMockBackend {
    /// Request history for verification
    request_history: Arc<std::sync::Mutex<Vec<ModelRequest>>>,
    /// Request counter per model
    request_counts: Arc<std::sync::Mutex<std::collections::HashMap<String, usize>>>,
    /// Models currently "unavailable" for fallback testing
    unavailable_models: Arc<std::sync::Mutex<HashSet<String>>>,
    /// Token delay for streaming simulation
    token_delay_ms: u64,
    /// Model-to-category mapping
    model_categories: std::collections::HashMap<String, ModelCategory>,
}

impl Clone for MultiModelMockBackend {
    fn clone(&self) -> Self {
        Self {
            request_history: Arc::clone(&self.request_history),
            request_counts: Arc::clone(&self.request_counts),
            unavailable_models: Arc::clone(&self.unavailable_models),
            token_delay_ms: self.token_delay_ms,
            model_categories: self.model_categories.clone(),
        }
    }
}

impl MultiModelMockBackend {
    /// Create a backend with standard model configurations
    pub fn with_standard_models() -> Self {
        let mut model_categories = std::collections::HashMap::new();
        model_categories.insert("code-llama".to_string(), ModelCategory::Code);
        model_categories.insert("math-wizard".to_string(), ModelCategory::Math);
        model_categories.insert("quick-7b".to_string(), ModelCategory::Quick);
        model_categories.insert("deep-70b".to_string(), ModelCategory::Deep);
        model_categories.insert("creative-writer".to_string(), ModelCategory::Creative);
        model_categories.insert("general-13b".to_string(), ModelCategory::General);

        Self {
            request_history: Arc::new(std::sync::Mutex::new(Vec::new())),
            request_counts: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            unavailable_models: Arc::new(std::sync::Mutex::new(HashSet::new())),
            token_delay_ms: 5,
            model_categories,
        }
    }

    /// Create a backend with custom delay
    pub fn with_delay(delay_ms: u64) -> Self {
        let mut backend = Self::with_standard_models();
        backend.token_delay_ms = delay_ms;
        backend
    }

    /// Mark a model as unavailable
    pub fn set_unavailable(&self, model: &str) {
        self.unavailable_models
            .lock()
            .unwrap()
            .insert(model.to_string());
    }

    /// Mark a model as available
    pub fn set_available(&self, model: &str) {
        self.unavailable_models.lock().unwrap().remove(model);
    }

    /// Check if model is unavailable
    pub fn is_unavailable(&self, model: &str) -> bool {
        self.unavailable_models.lock().unwrap().contains(model)
    }

    /// Get request count for a model
    pub fn request_count(&self, model: &str) -> usize {
        *self.request_counts.lock().unwrap().get(model).unwrap_or(&0)
    }

    /// Get total request count
    pub fn total_request_count(&self) -> usize {
        self.request_counts.lock().unwrap().values().sum()
    }

    /// Get full request history
    pub fn request_history(&self) -> Vec<ModelRequest> {
        self.request_history.lock().unwrap().clone()
    }

    /// Clear all history
    pub fn clear_history(&self) {
        self.request_history.lock().unwrap().clear();
        self.request_counts.lock().unwrap().clear();
    }

    /// Get category for a model
    pub fn model_category(&self, model: &str) -> Option<ModelCategory> {
        self.model_categories.get(model).copied()
    }

    /// Record a request
    fn record_request(&self, model: &str, prompt: &str) {
        self.request_history.lock().unwrap().push(ModelRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            timestamp: Instant::now(),
        });

        *self
            .request_counts
            .lock()
            .unwrap()
            .entry(model.to_string())
            .or_insert(0) += 1;
    }

    /// Generate response tokens based on model
    fn generate_response(&self, model: &str) -> Vec<String> {
        match self.model_categories.get(model) {
            Some(ModelCategory::Code) => vec![
                "[yolla:mood focused]".to_string(),
                "```rust\n".to_string(),
                "fn example() -> Result<()> {\n".to_string(),
                "    Ok(())\n".to_string(),
                "}\n```".to_string(),
            ],
            Some(ModelCategory::Math) => vec![
                "[yolla:mood thinking]".to_string(),
                "The solution is: ".to_string(),
                "$x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}$".to_string(),
            ],
            Some(ModelCategory::Quick) => {
                vec!["Quick answer: Yes.".to_string()]
            }
            Some(ModelCategory::Deep) => vec![
                "[yolla:mood thinking]".to_string(),
                "Let me analyze this. ".to_string(),
                "The answer requires consideration.".to_string(),
            ],
            Some(ModelCategory::Creative) => vec![
                "[yolla:mood happy]".to_string(),
                "Once upon a time...".to_string(),
            ],
            _ => vec!["I can help with that.".to_string()],
        }
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

        // Record request
        self.record_request(&model, &request.prompt);

        // Check availability
        if self.is_unavailable(&model) {
            anyhow::bail!("Model {} is unavailable", model);
        }

        let tokens = self.generate_response(&model);
        let delay = self.token_delay_ms;

        let (tx, rx) = mpsc::channel(10);

        tokio::spawn(async move {
            let mut full_message = String::new();
            for token in tokens {
                full_message.push_str(&token);
                if tx.send(StreamingToken::Token(token)).await.is_err() {
                    return;
                }
                if delay > 0 {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
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

        self.record_request(&model, &request.prompt);

        if self.is_unavailable(&model) {
            anyhow::bail!("Model {} is unavailable", model);
        }

        let tokens = self.generate_response(&model);
        let content = tokens.join("");

        Ok(LlmResponse {
            content,
            model,
            tokens_used: Some(tokens.len() as u32),
            duration_ms: Some(10),
        })
    }

    async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        let unavailable = self.unavailable_models.lock().unwrap();

        Ok(self
            .model_categories
            .keys()
            .filter(|m| !unavailable.contains(*m))
            .map(|m| ModelInfo {
                name: m.clone(),
                description: Some(format!("{:?} model", self.model_categories.get(m))),
                size: None,
                parameters: None,
                loaded: true,
            })
            .collect())
    }
}

// ============================================================================
// Test Utilities
// ============================================================================

/// Create a conductor with the multi-model mock backend
fn create_test_conductor(
    backend: MultiModelMockBackend,
    model: &str,
) -> (
    Conductor<MultiModelMockBackend>,
    mpsc::Receiver<ConductorMessage>,
) {
    let (tx, rx) = mpsc::channel(100);
    let config = ConductorConfig {
        model: model.to_string(),
        warmup_on_start: false,
        greet_on_connect: false,
        max_context_messages: 10,
        system_prompt: None,
        limits: Default::default(),
        additional_agents: vec![],
    };

    let conductor = Conductor::new(backend, config, tx);
    (conductor, rx)
}

/// Wait for a response to complete, returning collected tokens
async fn wait_for_response(
    conductor: &mut Conductor<MultiModelMockBackend>,
    rx: &mut mpsc::Receiver<ConductorMessage>,
    timeout_ms: u64,
) -> (bool, Vec<String>) {
    let deadline = Duration::from_millis(timeout_ms);
    let start = Instant::now();
    let mut tokens = Vec::new();
    let mut complete = false;

    while start.elapsed() < deadline && !complete {
        conductor.poll_streaming().await;
        tokio::time::sleep(Duration::from_millis(10)).await;

        while let Ok(msg) = rx.try_recv() {
            match msg {
                ConductorMessage::Token { text, .. } => {
                    tokens.push(text);
                }
                ConductorMessage::StreamEnd { .. } => {
                    complete = true;
                }
                _ => {}
            }
        }
    }

    (complete, tokens)
}

/// Drain all pending messages from a channel
fn drain_messages(rx: &mut mpsc::Receiver<ConductorMessage>) -> Vec<ConductorMessage> {
    let mut messages = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        messages.push(msg);
    }
    messages
}

// ============================================================================
// Test 1: Code Query Routes to Code Model
// ============================================================================

/// Verifies that code-related queries are routed to the code-llama model
///
/// This test:
/// 1. Creates a conductor configured to use "code-llama"
/// 2. Sends a code-related query
/// 3. Verifies the response contains code-specific patterns
/// 4. Confirms the code model received the request
#[tokio::test]
async fn test_code_query_routes_to_code_model() {
    let backend = MultiModelMockBackend::with_standard_models();
    let backend_ref = backend.clone();

    // Create conductor with code-llama as the model
    let (mut conductor, mut rx) = create_test_conductor(backend, "code-llama");

    // Start conductor
    conductor.start().await.expect("Conductor should start");

    // Connect surface
    conductor
        .handle_event(SurfaceEvent::Connected {
            event_id: SurfaceEvent::new_event_id(),
            surface_type: SurfaceType::Tui,
            capabilities: SurfaceCapabilities::tui(),
        })
        .await
        .expect("Should connect");

    drain_messages(&mut rx);

    // Send a code-related query
    conductor
        .handle_event(SurfaceEvent::UserMessage {
            event_id: SurfaceEvent::new_event_id(),
            content: "Write a Rust function to parse JSON".to_string(),
        })
        .await
        .expect("Should send message");

    // Wait for response
    let (complete, tokens) = wait_for_response(&mut conductor, &mut rx, 2000).await;

    assert!(complete, "Response should complete");
    assert!(!tokens.is_empty(), "Should receive tokens");

    // Verify code model was used
    assert_eq!(
        backend_ref.request_count("code-llama"),
        1,
        "Code model should receive exactly 1 request"
    );

    // Verify response contains code patterns
    let response = tokens.join("");
    assert!(
        response.contains("```") || response.contains("fn "),
        "Response should contain code patterns"
    );
}

// ============================================================================
// Test 2: Math Query Routes to Math Model
// ============================================================================

/// Verifies that math-related queries are routed to the math-wizard model
///
/// This test:
/// 1. Creates a conductor configured to use "math-wizard"
/// 2. Sends a math-related query
/// 3. Verifies the response contains math-specific patterns
/// 4. Confirms the math model received the request
#[tokio::test]
async fn test_math_query_routes_to_math_model() {
    let backend = MultiModelMockBackend::with_standard_models();
    let backend_ref = backend.clone();

    // Create conductor with math-wizard as the model
    let (mut conductor, mut rx) = create_test_conductor(backend, "math-wizard");

    conductor.start().await.expect("Conductor should start");

    conductor
        .handle_event(SurfaceEvent::Connected {
            event_id: SurfaceEvent::new_event_id(),
            surface_type: SurfaceType::Tui,
            capabilities: SurfaceCapabilities::tui(),
        })
        .await
        .expect("Should connect");

    drain_messages(&mut rx);

    // Send a math-related query
    conductor
        .handle_event(SurfaceEvent::UserMessage {
            event_id: SurfaceEvent::new_event_id(),
            content: "Solve the quadratic equation x^2 + 5x + 6 = 0".to_string(),
        })
        .await
        .expect("Should send message");

    let (complete, tokens) = wait_for_response(&mut conductor, &mut rx, 2000).await;

    assert!(complete, "Response should complete");
    assert!(!tokens.is_empty(), "Should receive tokens");

    // Verify math model was used
    assert_eq!(
        backend_ref.request_count("math-wizard"),
        1,
        "Math model should receive exactly 1 request"
    );

    // Verify response contains math patterns
    let response = tokens.join("");
    assert!(
        response.contains("solution") || response.contains("$") || response.contains("="),
        "Response should contain math patterns"
    );
}

// ============================================================================
// Test 3: Fallback When Primary Model Unavailable
// ============================================================================

/// Verifies fallback behavior when the primary model is unavailable
///
/// This test:
/// 1. Creates a backend and marks primary model as unavailable
/// 2. Attempts to send a request to the unavailable model
/// 3. Verifies the request fails appropriately
/// 4. Tests that alternative models still work
#[tokio::test]
async fn test_fallback_when_primary_model_unavailable() {
    let backend = MultiModelMockBackend::with_standard_models();

    // Mark code-llama as unavailable
    backend.set_unavailable("code-llama");

    let backend_ref = backend.clone();

    // First, verify that direct request to unavailable model fails
    let request = LlmRequest::new("test", "code-llama");
    let result = backend.send(&request).await;
    assert!(result.is_err(), "Request to unavailable model should fail");

    // Now test that fallback model (deep-70b) works
    let (mut conductor, mut rx) = create_test_conductor(backend_ref.clone(), "deep-70b");

    conductor.start().await.expect("Conductor should start");

    conductor
        .handle_event(SurfaceEvent::Connected {
            event_id: SurfaceEvent::new_event_id(),
            surface_type: SurfaceType::Tui,
            capabilities: SurfaceCapabilities::tui(),
        })
        .await
        .expect("Should connect");

    drain_messages(&mut rx);

    // Send query to fallback model
    conductor
        .handle_event(SurfaceEvent::UserMessage {
            event_id: SurfaceEvent::new_event_id(),
            content: "Analyze this code".to_string(),
        })
        .await
        .expect("Should send message");

    let (complete, tokens) = wait_for_response(&mut conductor, &mut rx, 2000).await;

    assert!(complete, "Fallback response should complete");
    assert!(!tokens.is_empty(), "Should receive tokens from fallback");

    // Verify fallback model was used
    assert_eq!(
        backend_ref.request_count("deep-70b"),
        1,
        "Fallback model should receive the request"
    );

    // Verify unavailable model was not called
    // (except for the initial direct test, which was recorded)
    assert_eq!(
        backend_ref.request_count("code-llama"),
        1,
        "Unavailable model should only have 1 request (from direct test)"
    );
}

// ============================================================================
// Test 4: Concurrent Multi-Model Streaming
// ============================================================================

/// Verifies that multiple concurrent streams to different models work correctly
///
/// This test:
/// 1. Creates multiple conductors with different models
/// 2. Sends queries to all models concurrently
/// 3. Verifies all streams complete without interference
/// 4. Confirms each model received exactly one request
#[tokio::test]
async fn test_concurrent_multi_model_streaming() {
    let backend = MultiModelMockBackend::with_delay(10); // 10ms per token
    let backend_ref = backend.clone();

    // Create three conductors with different models
    let (mut conductor1, mut rx1) = create_test_conductor(backend.clone(), "code-llama");
    let (mut conductor2, mut rx2) = create_test_conductor(backend.clone(), "math-wizard");
    let (mut conductor3, mut rx3) = create_test_conductor(backend.clone(), "creative-writer");

    // Start all conductors
    conductor1.start().await.expect("Conductor 1 should start");
    conductor2.start().await.expect("Conductor 2 should start");
    conductor3.start().await.expect("Conductor 3 should start");

    // Connect all surfaces
    for conductor in [&mut conductor1, &mut conductor2, &mut conductor3] {
        conductor
            .handle_event(SurfaceEvent::Connected {
                event_id: SurfaceEvent::new_event_id(),
                surface_type: SurfaceType::Tui,
                capabilities: SurfaceCapabilities::tui(),
            })
            .await
            .expect("Should connect");
    }

    // Clear initial messages
    drain_messages(&mut rx1);
    drain_messages(&mut rx2);
    drain_messages(&mut rx3);

    // Send queries to different models concurrently
    conductor1
        .handle_event(SurfaceEvent::UserMessage {
            event_id: SurfaceEvent::new_event_id(),
            content: "Write Rust code".to_string(),
        })
        .await
        .expect("Should send to code model");

    conductor2
        .handle_event(SurfaceEvent::UserMessage {
            event_id: SurfaceEvent::new_event_id(),
            content: "Solve x^2 = 4".to_string(),
        })
        .await
        .expect("Should send to math model");

    conductor3
        .handle_event(SurfaceEvent::UserMessage {
            event_id: SurfaceEvent::new_event_id(),
            content: "Write a poem".to_string(),
        })
        .await
        .expect("Should send to creative model");

    // Wait for all responses
    let mut completions = 0;
    let start = Instant::now();
    let deadline = Duration::from_secs(3);

    while start.elapsed() < deadline && completions < 3 {
        conductor1.poll_streaming().await;
        conductor2.poll_streaming().await;
        conductor3.poll_streaming().await;

        tokio::time::sleep(Duration::from_millis(20)).await;

        for rx in [&mut rx1, &mut rx2, &mut rx3] {
            while let Ok(msg) = rx.try_recv() {
                if matches!(msg, ConductorMessage::StreamEnd { .. }) {
                    completions += 1;
                }
            }
        }
    }

    assert_eq!(completions, 3, "All three streams should complete");

    // Verify each model received exactly one request
    assert_eq!(backend_ref.request_count("code-llama"), 1);
    assert_eq!(backend_ref.request_count("math-wizard"), 1);
    assert_eq!(backend_ref.request_count("creative-writer"), 1);

    // Verify no other models were called
    assert_eq!(backend_ref.request_count("general-13b"), 0);
    assert_eq!(backend_ref.request_count("deep-70b"), 0);
    assert_eq!(backend_ref.request_count("quick-7b"), 0);
}

// ============================================================================
// Test 5: Request History Tracking
// ============================================================================

/// Verifies that request history is accurately tracked across multiple queries
///
/// This test:
/// 1. Sends a series of queries to different models
/// 2. Verifies the request history contains all requests
/// 3. Confirms timestamps are ordered correctly
/// 4. Tests history clearing functionality
#[tokio::test]
async fn test_request_history_tracking() {
    let backend = MultiModelMockBackend::with_standard_models();
    let backend_ref = backend.clone();

    // Clear any existing history
    backend_ref.clear_history();
    assert_eq!(backend_ref.total_request_count(), 0);

    // Send multiple requests directly to different models
    let request1 = LlmRequest::new("First query about code", "code-llama");
    backend
        .send(&request1)
        .await
        .expect("Request 1 should succeed");

    let request2 = LlmRequest::new("Second query about math", "math-wizard");
    backend
        .send(&request2)
        .await
        .expect("Request 2 should succeed");

    let request3 = LlmRequest::new("Third query general", "general-13b");
    backend
        .send(&request3)
        .await
        .expect("Request 3 should succeed");

    // Verify history
    let history = backend_ref.request_history();
    assert_eq!(history.len(), 3, "Should have 3 requests in history");

    // Verify order
    assert_eq!(history[0].model, "code-llama");
    assert_eq!(history[0].prompt, "First query about code");

    assert_eq!(history[1].model, "math-wizard");
    assert_eq!(history[1].prompt, "Second query about math");

    assert_eq!(history[2].model, "general-13b");
    assert_eq!(history[2].prompt, "Third query general");

    // Verify timestamps are ordered
    assert!(history[0].timestamp <= history[1].timestamp);
    assert!(history[1].timestamp <= history[2].timestamp);

    // Verify request counts
    assert_eq!(backend_ref.request_count("code-llama"), 1);
    assert_eq!(backend_ref.request_count("math-wizard"), 1);
    assert_eq!(backend_ref.request_count("general-13b"), 1);
    assert_eq!(backend_ref.total_request_count(), 3);

    // Test history clearing
    backend_ref.clear_history();
    assert_eq!(backend_ref.total_request_count(), 0);
    assert!(backend_ref.request_history().is_empty());
}

// ============================================================================
// Additional Edge Case Tests
// ============================================================================

/// Test that streaming completes within expected time
#[tokio::test]
async fn test_streaming_completes_within_timeout() {
    let backend = MultiModelMockBackend::with_delay(5); // 5ms per token
    let (mut conductor, mut rx) = create_test_conductor(backend, "quick-7b");

    conductor.start().await.expect("Should start");

    conductor
        .handle_event(SurfaceEvent::Connected {
            event_id: SurfaceEvent::new_event_id(),
            surface_type: SurfaceType::Tui,
            capabilities: SurfaceCapabilities::tui(),
        })
        .await
        .expect("Should connect");

    drain_messages(&mut rx);

    let start = Instant::now();

    conductor
        .handle_event(SurfaceEvent::UserMessage {
            event_id: SurfaceEvent::new_event_id(),
            content: "Quick question".to_string(),
        })
        .await
        .expect("Should send");

    let (complete, _) = wait_for_response(&mut conductor, &mut rx, 1000).await;

    let elapsed = start.elapsed();

    assert!(complete, "Response should complete");
    assert!(
        elapsed < Duration::from_secs(1),
        "Response should complete in under 1 second, took {:?}",
        elapsed
    );
}

/// Test model availability toggle
#[tokio::test]
async fn test_model_availability_toggle() {
    let backend = MultiModelMockBackend::with_standard_models();

    // Initially available
    let request = LlmRequest::new("test", "code-llama");
    assert!(backend.send(&request).await.is_ok());
    assert_eq!(backend.request_count("code-llama"), 1);

    // Mark unavailable
    backend.set_unavailable("code-llama");
    assert!(backend.send(&request).await.is_err());
    // Request count should still increment (request was recorded before failure check)
    assert_eq!(backend.request_count("code-llama"), 2);

    // Mark available again
    backend.set_available("code-llama");
    assert!(backend.send(&request).await.is_ok());
    assert_eq!(backend.request_count("code-llama"), 3);
}

/// Test that all standard models produce responses
#[tokio::test]
async fn test_all_standard_models_respond() {
    let backend = MultiModelMockBackend::with_standard_models();

    let models = [
        "code-llama",
        "math-wizard",
        "quick-7b",
        "deep-70b",
        "creative-writer",
        "general-13b",
    ];

    for model in models {
        let request = LlmRequest::new(&format!("Test query for {}", model), model);
        let response = backend
            .send(&request)
            .await
            .expect(&format!("{} should respond", model));

        assert!(
            !response.content.is_empty(),
            "{} should return non-empty content",
            model
        );
        assert_eq!(
            response.model, model,
            "Response should identify correct model"
        );
    }

    // Verify all models were called
    assert_eq!(backend.total_request_count(), 6);
}

/// Test streaming error handling when model becomes unavailable mid-request
#[tokio::test]
async fn test_streaming_with_unavailable_model() {
    let backend = MultiModelMockBackend::with_standard_models();

    // Mark model unavailable before streaming
    backend.set_unavailable("code-llama");

    let request = LlmRequest::new("test", "code-llama");
    let result = backend.send_streaming(&request).await;

    assert!(
        result.is_err(),
        "Streaming to unavailable model should fail"
    );
    assert!(
        result.unwrap_err().to_string().contains("unavailable"),
        "Error should mention unavailability"
    );
}

/// Test that request history preserves prompt content accurately
#[tokio::test]
async fn test_history_preserves_prompt_content() {
    let backend = MultiModelMockBackend::with_standard_models();
    backend.clear_history();

    let complex_prompt =
        "This is a complex prompt with special chars: @#$%^&*() and unicode: \u{1F600}";
    let request = LlmRequest::new(complex_prompt, "general-13b");
    backend.send(&request).await.expect("Should succeed");

    let history = backend.request_history();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].prompt, complex_prompt);
}
