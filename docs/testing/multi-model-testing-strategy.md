# Multi-Model Testing Strategy

Testing strategy for multi-model support in ai-way, addressing the challenges of testing model routing without actual LLM inference in CI.

## Overview

We're adding support for routing requests to different models:
- **Specialized models** (code, math) - always use specific model
- **General models** (quick, deep, creative) - meta-agent selects based on query

This document provides a comprehensive testing strategy that builds on existing patterns from `tui/tests/integration_test.rs`.

---

## 1. Mock Backend Design for Multiple Models

### 1.1 MultiModelMockBackend Architecture

Extend the existing `IntegrationMockBackend` pattern to support multiple model personalities:

```rust
/// Mock backend supporting multiple model configurations
pub struct MultiModelMockBackend {
    /// Available models with their configurations
    models: HashMap<String, MockModelConfig>,
    /// Request history for verification
    request_history: Arc<Mutex<Vec<ModelRequest>>>,
    /// Simulated latencies per model
    latencies: HashMap<String, Duration>,
    /// Models currently "unavailable" for fallback testing
    unavailable_models: Arc<Mutex<HashSet<String>>>,
    /// Request counter per model
    request_counts: Arc<Mutex<HashMap<String, usize>>>,
}

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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ModelCategory {
    /// Specialized: always routes to specific model
    Code,
    Math,
    /// General: meta-agent selects
    Quick,
    Deep,
    Creative,
    /// Default fallback
    General,
}

/// Captured request for test verification
#[derive(Clone, Debug)]
pub struct ModelRequest {
    pub model: String,
    pub prompt: String,
    pub timestamp: Instant,
    pub category_hint: Option<ModelCategory>,
}
```

### 1.2 Response Generators per Model Type

```rust
impl MultiModelMockBackend {
    /// Create with standard model configurations
    pub fn with_standard_models() -> Self {
        let mut backend = Self::new();

        // Code model - returns code-like responses
        backend.add_model(MockModelConfig {
            name: "code-llama".to_string(),
            category: ModelCategory::Code,
            response_fn: Box::new(|req| {
                vec![
                    "[yolla:mood focused]".to_string(),
                    "```rust\n".to_string(),
                    "fn example() -> Result<()> {\n".to_string(),
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
            response_fn: Box::new(|req| {
                vec![
                    "[yolla:mood thinking]".to_string(),
                    "The answer is: ".to_string(),
                    "$x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}$".to_string(),
                ]
            }),
            token_delay_ms: 10,
            loaded: true,
        });

        // Quick model - fast, concise responses
        backend.add_model(MockModelConfig {
            name: "quick-7b".to_string(),
            category: ModelCategory::Quick,
            response_fn: Box::new(|req| {
                vec!["Quick answer: Yes.".to_string()]
            }),
            token_delay_ms: 2,
            loaded: true,
        });

        // Deep model - thorough responses
        backend.add_model(MockModelConfig {
            name: "deep-70b".to_string(),
            category: ModelCategory::Deep,
            response_fn: Box::new(|req| {
                vec![
                    "[yolla:mood thinking]".to_string(),
                    "Let me analyze this thoroughly. ".to_string(),
                    "First, we need to consider... ".to_string(),
                    "Second, the implications are... ".to_string(),
                    "In conclusion...".to_string(),
                ]
            }),
            token_delay_ms: 20,
            loaded: true,
        });

        // Creative model
        backend.add_model(MockModelConfig {
            name: "creative-writer".to_string(),
            category: ModelCategory::Creative,
            response_fn: Box::new(|req| {
                vec![
                    "[yolla:mood happy]".to_string(),
                    "Once upon a time, ".to_string(),
                    "in a world of endless possibilities...".to_string(),
                ]
            }),
            token_delay_ms: 8,
            loaded: true,
        });

        // General fallback
        backend.add_model(MockModelConfig {
            name: "general-13b".to_string(),
            category: ModelCategory::General,
            response_fn: Box::new(|req| {
                vec!["I can help with that.".to_string()]
            }),
            token_delay_ms: 5,
            loaded: true,
        });

        backend
    }

    /// Mark a model as unavailable (for fallback testing)
    pub fn set_unavailable(&self, model: &str) {
        self.unavailable_models.lock().unwrap().insert(model.to_string());
    }

    /// Mark a model as available again
    pub fn set_available(&self, model: &str) {
        self.unavailable_models.lock().unwrap().remove(model);
    }

    /// Get request count for a specific model
    pub fn request_count(&self, model: &str) -> usize {
        *self.request_counts.lock().unwrap().get(model).unwrap_or(&0)
    }

    /// Get full request history
    pub fn request_history(&self) -> Vec<ModelRequest> {
        self.request_history.lock().unwrap().clone()
    }

    /// Clear request history (between test cases)
    pub fn clear_history(&self) {
        self.request_history.lock().unwrap().clear();
        self.request_counts.lock().unwrap().clear();
    }
}
```

### 1.3 LlmBackend Implementation

```rust
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
        // Record the request
        self.request_history.lock().unwrap().push(ModelRequest {
            model: request.model.clone(),
            prompt: request.prompt.clone(),
            timestamp: Instant::now(),
            category_hint: None,
        });

        // Increment request count
        *self.request_counts
            .lock()
            .unwrap()
            .entry(request.model.clone())
            .or_insert(0) += 1;

        // Check if model is unavailable
        if self.unavailable_models.lock().unwrap().contains(&request.model) {
            anyhow::bail!("Model {} is unavailable", request.model);
        }

        // Get model config
        let config = self.models.get(&request.model)
            .ok_or_else(|| anyhow::anyhow!("Unknown model: {}", request.model))?;

        let tokens = (config.response_fn)(request);
        let delay = config.token_delay_ms;

        let (tx, rx) = mpsc::channel(10);
        tokio::spawn(async move {
            let mut full_message = String::new();
            for token in tokens {
                full_message.push_str(&token);
                let _ = tx.send(StreamingToken::Token(token)).await;
                if delay > 0 {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
            let _ = tx.send(StreamingToken::Complete { message: full_message }).await;
        });

        Ok(rx)
    }

    async fn send(&self, request: &LlmRequest) -> anyhow::Result<LlmResponse> {
        // Similar to send_streaming but returns complete response
        // ...
    }

    async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(self.models.values()
            .filter(|m| !self.unavailable_models.lock().unwrap().contains(&m.name))
            .map(|m| ModelInfo {
                name: m.name.clone(),
                description: Some(format!("{:?} model", m.category)),
                size: None,
                parameters: None,
                loaded: m.loaded,
            })
            .collect())
    }
}
```

---

## 2. Unit Tests for ModelSelector

The `ModelSelector` is the component that decides which model to use for a given query.

### 2.1 ModelSelector Interface

```rust
/// Model selection result
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

#[derive(Clone, Debug, PartialEq)]
pub enum SelectionReason {
    /// Specialized model required (code, math)
    Specialized(ModelCategory),
    /// Meta-agent classification result
    MetaAgentDecision { classification: String },
    /// User explicitly requested model
    UserRequested,
    /// Fallback due to unavailability
    Fallback { original: String, reason: String },
    /// Default model (no specific routing)
    Default,
}

pub trait ModelSelector: Send + Sync {
    /// Select model for a given query
    fn select(&self, query: &str, context: &SelectionContext) -> ModelSelection;

    /// Check if a model is available
    fn is_available(&self, model: &str) -> bool;

    /// Get fallback chain for a model
    fn fallback_chain(&self, model: &str) -> Vec<String>;
}
```

### 2.2 Unit Test Cases

```rust
#[cfg(test)]
mod model_selector_tests {
    use super::*;

    // =========================================================================
    // Specialized Model Selection (Always Use Specific Model)
    // =========================================================================

    #[test]
    fn test_code_query_routes_to_code_model() {
        let selector = DefaultModelSelector::new();
        let selection = selector.select(
            "Write a Rust function to parse JSON",
            &SelectionContext::default()
        );

        assert_eq!(selection.model, "code-llama");
        assert!(matches!(selection.reason, SelectionReason::Specialized(ModelCategory::Code)));
    }

    #[test]
    fn test_programming_keywords_trigger_code_model() {
        let selector = DefaultModelSelector::new();

        let code_queries = [
            "debug this Python script",
            "explain this function def",
            "fix the syntax error in class Foo",
            "implement a binary search algorithm",
            "refactor this code to be more efficient",
        ];

        for query in code_queries {
            let selection = selector.select(query, &SelectionContext::default());
            assert_eq!(
                selection.model, "code-llama",
                "Query '{}' should route to code model", query
            );
        }
    }

    #[test]
    fn test_math_query_routes_to_math_model() {
        let selector = DefaultModelSelector::new();
        let selection = selector.select(
            "Solve the quadratic equation x^2 + 5x + 6 = 0",
            &SelectionContext::default()
        );

        assert_eq!(selection.model, "math-wizard");
        assert!(matches!(selection.reason, SelectionReason::Specialized(ModelCategory::Math)));
    }

    #[test]
    fn test_math_keywords_trigger_math_model() {
        let selector = DefaultModelSelector::new();

        let math_queries = [
            "calculate the integral of x^2",
            "derive the formula for compound interest",
            "prove that sqrt(2) is irrational",
            "what is 1547 * 892?",
            "solve for x: 3x + 7 = 22",
        ];

        for query in math_queries {
            let selection = selector.select(query, &SelectionContext::default());
            assert_eq!(
                selection.model, "math-wizard",
                "Query '{}' should route to math model", query
            );
        }
    }

    // =========================================================================
    // General Model Selection (Meta-Agent Decides)
    // =========================================================================

    #[test]
    fn test_simple_query_routes_to_quick_model() {
        let selector = DefaultModelSelector::new();
        let selection = selector.select(
            "What day is it?",
            &SelectionContext::default()
        );

        assert_eq!(selection.model, "quick-7b");
        assert!(matches!(selection.reason, SelectionReason::MetaAgentDecision { .. }));
    }

    #[test]
    fn test_complex_query_routes_to_deep_model() {
        let selector = DefaultModelSelector::new();
        let selection = selector.select(
            "Explain the philosophical implications of quantum mechanics on free will, \
             considering both deterministic and indeterministic interpretations",
            &SelectionContext::default()
        );

        assert_eq!(selection.model, "deep-70b");
    }

    #[test]
    fn test_creative_query_routes_to_creative_model() {
        let selector = DefaultModelSelector::new();
        let selection = selector.select(
            "Write a poem about the sunset",
            &SelectionContext::default()
        );

        assert_eq!(selection.model, "creative-writer");
    }

    #[test]
    fn test_story_request_routes_to_creative_model() {
        let selector = DefaultModelSelector::new();

        let creative_queries = [
            "tell me a story",
            "write a haiku about rain",
            "compose a song about friendship",
            "create a fictional dialogue between Einstein and Newton",
        ];

        for query in creative_queries {
            let selection = selector.select(query, &SelectionContext::default());
            assert_eq!(
                selection.model, "creative-writer",
                "Query '{}' should route to creative model", query
            );
        }
    }

    // =========================================================================
    // User Override Tests
    // =========================================================================

    #[test]
    fn test_user_model_override() {
        let selector = DefaultModelSelector::new();
        let context = SelectionContext {
            user_requested_model: Some("custom-model".to_string()),
            ..Default::default()
        };

        let selection = selector.select("any query", &context);
        assert_eq!(selection.model, "custom-model");
        assert!(matches!(selection.reason, SelectionReason::UserRequested));
    }

    #[test]
    fn test_user_override_takes_precedence() {
        let selector = DefaultModelSelector::new();
        let context = SelectionContext {
            user_requested_model: Some("general-13b".to_string()),
            ..Default::default()
        };

        // Even a code query should use user-requested model
        let selection = selector.select(
            "Write a Python function",
            &context
        );
        assert_eq!(selection.model, "general-13b");
        assert!(matches!(selection.reason, SelectionReason::UserRequested));
    }

    // =========================================================================
    // Fallback Chain Tests
    // =========================================================================

    #[test]
    fn test_fallback_chain_for_code_model() {
        let selector = DefaultModelSelector::new();
        let chain = selector.fallback_chain("code-llama");

        assert_eq!(chain, vec!["deep-70b", "general-13b"]);
    }

    #[test]
    fn test_fallback_chain_for_math_model() {
        let selector = DefaultModelSelector::new();
        let chain = selector.fallback_chain("math-wizard");

        assert_eq!(chain, vec!["deep-70b", "general-13b"]);
    }

    #[test]
    fn test_fallback_chain_for_quick_model() {
        let selector = DefaultModelSelector::new();
        let chain = selector.fallback_chain("quick-7b");

        assert_eq!(chain, vec!["general-13b"]);
    }

    #[test]
    fn test_general_model_has_no_fallback() {
        let selector = DefaultModelSelector::new();
        let chain = selector.fallback_chain("general-13b");

        assert!(chain.is_empty());
    }

    // =========================================================================
    // Confidence Score Tests
    // =========================================================================

    #[test]
    fn test_high_confidence_for_explicit_code_keywords() {
        let selector = DefaultModelSelector::new();
        let selection = selector.select(
            "```python\ndef foo():\n    pass\n```",
            &SelectionContext::default()
        );

        assert!(selection.confidence > 0.9);
    }

    #[test]
    fn test_lower_confidence_for_ambiguous_query() {
        let selector = DefaultModelSelector::new();
        let selection = selector.select(
            "help me with something",
            &SelectionContext::default()
        );

        // Ambiguous queries should have lower confidence
        assert!(selection.confidence < 0.7);
    }

    // =========================================================================
    // Context-Aware Selection Tests
    // =========================================================================

    #[test]
    fn test_conversation_context_influences_selection() {
        let selector = DefaultModelSelector::new();
        let context = SelectionContext {
            previous_model: Some("code-llama".to_string()),
            conversation_topic: Some("programming".to_string()),
            ..Default::default()
        };

        // Even an ambiguous follow-up should prefer code model
        let selection = selector.select("can you improve it?", &context);
        assert_eq!(selection.model, "code-llama");
    }

    #[test]
    fn test_topic_switch_detection() {
        let selector = DefaultModelSelector::new();
        let context = SelectionContext {
            previous_model: Some("code-llama".to_string()),
            conversation_topic: Some("programming".to_string()),
            ..Default::default()
        };

        // Explicit topic change should override context
        let selection = selector.select(
            "Actually, let's write a poem instead",
            &context
        );
        assert_eq!(selection.model, "creative-writer");
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[test]
    fn test_empty_query_returns_default() {
        let selector = DefaultModelSelector::new();
        let selection = selector.select("", &SelectionContext::default());

        assert_eq!(selection.model, "general-13b");
        assert!(matches!(selection.reason, SelectionReason::Default));
    }

    #[test]
    fn test_mixed_code_and_math() {
        let selector = DefaultModelSelector::new();
        let selection = selector.select(
            "Write Python code to solve the quadratic formula",
            &SelectionContext::default()
        );

        // Code should take precedence (needs implementation)
        assert!(selection.model == "code-llama" || selection.model == "math-wizard");
    }

    #[test]
    fn test_non_english_query() {
        let selector = DefaultModelSelector::new();
        let selection = selector.select(
            "Escribe una funcion en Rust",
            &SelectionContext::default()
        );

        // Should still detect "funcion" and "Rust"
        assert_eq!(selection.model, "code-llama");
    }

    #[test]
    fn test_very_long_query() {
        let selector = DefaultModelSelector::new();
        let long_query = "a".repeat(10_000);

        let selection = selector.select(&long_query, &SelectionContext::default());

        // Should not panic, should return default
        assert!(!selection.model.is_empty());
    }
}
```

---

## 3. Integration Test Scenarios

### 3.1 Test Scenarios List (10 scenarios)

```rust
// File: tui/tests/multi_model_integration_test.rs

/// Test 1: Specialized model routing - code queries
#[tokio::test]
async fn test_code_query_routes_to_code_model() {
    // Setup: MultiModelMockBackend with all models
    // Action: Send code-related query
    // Verify: Request went to code-llama model
}

/// Test 2: Specialized model routing - math queries
#[tokio::test]
async fn test_math_query_routes_to_math_model() {
    // Setup: MultiModelMockBackend with all models
    // Action: Send math-related query
    // Verify: Request went to math-wizard model
}

/// Test 3: Meta-agent selection for general queries
#[tokio::test]
async fn test_general_query_meta_agent_selection() {
    // Setup: MultiModelMockBackend with all models
    // Action: Send ambiguous query ("help me with something")
    // Verify: Meta-agent classification occurred, appropriate model selected
}

/// Test 4: Fallback when primary model unavailable
#[tokio::test]
async fn test_fallback_when_model_unavailable() {
    // Setup: Set code-llama as unavailable
    // Action: Send code query
    // Verify: Falls back to deep-70b, then general-13b if needed
}

/// Test 5: Concurrent multi-model streaming
#[tokio::test]
async fn test_concurrent_multi_model_streaming() {
    // Setup: MultiModelMockBackend
    // Action: Send multiple queries that route to different models
    // Verify: All streams complete correctly, no interleaving issues
}

/// Test 6: Model selection with conversation context
#[tokio::test]
async fn test_model_selection_with_context() {
    // Setup: Establish conversation about code
    // Action: Send follow-up without explicit code keywords
    // Verify: Context maintains code model selection
}

/// Test 7: User model override
#[tokio::test]
async fn test_user_model_override() {
    // Setup: Configure user preference for specific model
    // Action: Send query that would normally route elsewhere
    // Verify: User preference honored
}

/// Test 8: Model availability recovery
#[tokio::test]
async fn test_model_availability_recovery() {
    // Setup: Set model unavailable, send query (triggers fallback)
    // Action: Set model available again, send same query
    // Verify: Now routes to primary model
}

/// Test 9: Request history tracking
#[tokio::test]
async fn test_request_history_tracking() {
    // Setup: MultiModelMockBackend
    // Action: Send series of queries
    // Verify: Request history correctly records model, prompt, timestamp
}

/// Test 10: Graceful degradation when all specialized models fail
#[tokio::test]
async fn test_graceful_degradation_all_fail() {
    // Setup: Mark all specialized models unavailable
    // Action: Send specialized query
    // Verify: Falls back to general model, user notified
}
```

### 3.2 Detailed Integration Test Implementation

```rust
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

use conductor_core::{
    Conductor, ConductorConfig, ConductorMessage, ConductorState,
    SurfaceEvent, SurfaceType, SurfaceCapabilities,
};

mod test_utils {
    use super::*;

    /// Create conductor with multi-model mock backend
    pub fn create_multi_model_conductor(
        backend: MultiModelMockBackend,
    ) -> (Conductor<MultiModelMockBackend>, mpsc::Receiver<ConductorMessage>) {
        let (tx, rx) = mpsc::channel(100);
        let config = ConductorConfig {
            model: "general-13b".to_string(), // Default model
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

    /// Wait for response completion
    pub async fn wait_for_response(
        conductor: &mut Conductor<MultiModelMockBackend>,
        rx: &mut mpsc::Receiver<ConductorMessage>,
        timeout_ms: u64,
    ) -> bool {
        let deadline = Duration::from_millis(timeout_ms);
        let start = std::time::Instant::now();

        while start.elapsed() < deadline {
            conductor.poll_streaming().await;
            tokio::time::sleep(Duration::from_millis(10)).await;

            while let Ok(msg) = rx.try_recv() {
                if matches!(msg, ConductorMessage::StreamEnd { .. }) {
                    return true;
                }
            }
        }
        false
    }
}

/// Test 1: Specialized model routing - code queries
#[tokio::test]
async fn test_code_query_routes_to_code_model() {
    let backend = MultiModelMockBackend::with_standard_models();
    let backend_ref = backend.clone(); // For verification
    let (mut conductor, mut rx) = test_utils::create_multi_model_conductor(backend);

    conductor.start().await.expect("Should start");

    // Connect surface
    conductor.handle_event(SurfaceEvent::Connected {
        event_id: SurfaceEvent::new_event_id(),
        surface_type: SurfaceType::Tui,
        capabilities: SurfaceCapabilities::tui(),
    }).await.expect("Should connect");

    // Clear initial messages
    while rx.try_recv().is_ok() {}

    // Send code query
    conductor.handle_event(SurfaceEvent::UserMessage {
        event_id: SurfaceEvent::new_event_id(),
        content: "Write a Rust function to parse JSON".to_string(),
    }).await.expect("Should send");

    // Wait for response
    assert!(
        test_utils::wait_for_response(&mut conductor, &mut rx, 1000).await,
        "Response should complete"
    );

    // Verify code model was used
    assert_eq!(
        backend_ref.request_count("code-llama"), 1,
        "Code model should have received exactly 1 request"
    );
    assert_eq!(
        backend_ref.request_count("general-13b"), 0,
        "General model should not have received requests"
    );
}

/// Test 4: Fallback when primary model unavailable
#[tokio::test]
async fn test_fallback_when_model_unavailable() {
    let backend = MultiModelMockBackend::with_standard_models();

    // Mark code model as unavailable
    backend.set_unavailable("code-llama");

    let backend_ref = backend.clone();
    let (mut conductor, mut rx) = test_utils::create_multi_model_conductor(backend);

    conductor.start().await.expect("Should start");

    conductor.handle_event(SurfaceEvent::Connected {
        event_id: SurfaceEvent::new_event_id(),
        surface_type: SurfaceType::Tui,
        capabilities: SurfaceCapabilities::tui(),
    }).await.expect("Should connect");

    while rx.try_recv().is_ok() {}

    // Send code query
    conductor.handle_event(SurfaceEvent::UserMessage {
        event_id: SurfaceEvent::new_event_id(),
        content: "Write a Python function".to_string(),
    }).await.expect("Should send");

    assert!(
        test_utils::wait_for_response(&mut conductor, &mut rx, 1000).await,
        "Response should complete via fallback"
    );

    // Verify fallback chain: code-llama (fail) -> deep-70b (success)
    assert_eq!(backend_ref.request_count("deep-70b"), 1);
}

/// Test 5: Concurrent multi-model streaming
#[tokio::test]
async fn test_concurrent_multi_model_streaming() {
    let backend = MultiModelMockBackend::with_standard_models();
    let backend_ref = backend.clone();

    // Create multiple conductors for parallel requests
    let (mut conductor1, mut rx1) = test_utils::create_multi_model_conductor(backend.clone());
    let (mut conductor2, mut rx2) = test_utils::create_multi_model_conductor(backend.clone());
    let (mut conductor3, mut rx3) = test_utils::create_multi_model_conductor(backend.clone());

    // Start all conductors
    conductor1.start().await.expect("Should start 1");
    conductor2.start().await.expect("Should start 2");
    conductor3.start().await.expect("Should start 3");

    // Connect surfaces
    for conductor in [&mut conductor1, &mut conductor2, &mut conductor3] {
        conductor.handle_event(SurfaceEvent::Connected {
            event_id: SurfaceEvent::new_event_id(),
            surface_type: SurfaceType::Tui,
            capabilities: SurfaceCapabilities::tui(),
        }).await.expect("Should connect");
    }

    // Clear initial messages
    while rx1.try_recv().is_ok() {}
    while rx2.try_recv().is_ok() {}
    while rx3.try_recv().is_ok() {}

    // Send queries to different models concurrently
    let queries = [
        ("Write Rust code", &mut conductor1, &mut rx1),
        ("Solve x^2 = 4", &mut conductor2, &mut rx2),
        ("Write a poem", &mut conductor3, &mut rx3),
    ];

    // Send all queries
    for (query, conductor, _) in &queries {
        conductor.handle_event(SurfaceEvent::UserMessage {
            event_id: SurfaceEvent::new_event_id(),
            content: query.to_string(),
        }).await.expect("Should send");
    }

    // Wait for all responses (poll all conductors)
    let mut all_complete = false;
    for _ in 0..100 {
        conductor1.poll_streaming().await;
        conductor2.poll_streaming().await;
        conductor3.poll_streaming().await;

        tokio::time::sleep(Duration::from_millis(20)).await;

        let mut completions = 0;
        for rx in [&mut rx1, &mut rx2, &mut rx3] {
            while let Ok(msg) = rx.try_recv() {
                if matches!(msg, ConductorMessage::StreamEnd { .. }) {
                    completions += 1;
                }
            }
        }

        if completions >= 3 {
            all_complete = true;
            break;
        }
    }

    assert!(all_complete, "All concurrent streams should complete");

    // Verify each model received exactly one request
    assert_eq!(backend_ref.request_count("code-llama"), 1);
    assert_eq!(backend_ref.request_count("math-wizard"), 1);
    assert_eq!(backend_ref.request_count("creative-writer"), 1);
}
```

---

## 4. Edge Case Coverage List

### 4.1 Input Validation Edge Cases

| # | Edge Case | Expected Behavior | Test Priority |
|---|-----------|-------------------|---------------|
| 1 | Empty query | Route to default model | High |
| 2 | Very long query (>100KB) | Truncate, route normally | High |
| 3 | Query with only whitespace | Treat as empty, use default | Medium |
| 4 | Query with Unicode/emoji | Parse correctly, route normally | Medium |
| 5 | Query with control characters | Sanitize, route normally | Medium |
| 6 | Query with SQL injection attempts | Sanitize, route normally | High |
| 7 | Query with null bytes | Sanitize, route normally | High |

### 4.2 Model Selection Edge Cases

| # | Edge Case | Expected Behavior | Test Priority |
|---|-----------|-------------------|---------------|
| 8 | Mixed code + math query | Prioritize based on keywords | Medium |
| 9 | Query in non-English language | Detect domain keywords | Medium |
| 10 | Code blocks with wrong language | Still route to code model | Medium |
| 11 | Math symbols in natural language | Route to math model | Medium |
| 12 | Ambiguous "calculate" (could be code or math) | Meta-agent decides | Medium |
| 13 | Previous context overrides new query | Context preserved | High |
| 14 | Topic switch detection | Recognize explicit switch | High |

### 4.3 Model Availability Edge Cases

| # | Edge Case | Expected Behavior | Test Priority |
|---|-----------|-------------------|---------------|
| 15 | All models unavailable | Return error, notify user | Critical |
| 16 | Only fallback available | Use fallback, warn user | High |
| 17 | Model becomes unavailable mid-stream | Complete with error, offer retry | High |
| 18 | Model returns empty response | Retry or fallback | High |
| 19 | Model times out | Fallback with timeout message | High |
| 20 | Model returns malformed response | Log, fallback | Medium |

### 4.4 Concurrency Edge Cases

| # | Edge Case | Expected Behavior | Test Priority |
|---|-----------|-------------------|---------------|
| 21 | Same model requested concurrently | Queue or parallel | High |
| 22 | Fallback during concurrent request | Each request independent | High |
| 23 | Model availability changes mid-batch | Affected requests fallback | Medium |
| 24 | Request cancellation during stream | Clean abort | High |
| 25 | Channel overflow during streaming | Backpressure or drop | High |

### 4.5 State Management Edge Cases

| # | Edge Case | Expected Behavior | Test Priority |
|---|-----------|-------------------|---------------|
| 26 | Model selection state persists across requests | Context maintained | High |
| 27 | Session clear resets model preference | Reset to default | Medium |
| 28 | User changes model preference mid-conversation | Switch immediately | Medium |
| 29 | Conductor restart preserves model history | Persist or reset gracefully | Low |

---

## 5. Pre-commit Hook Updates

Update `.git/hooks/pre-commit` to include multi-model tests:

```bash
#!/bin/bash
# ... existing checks ...

# ============================================================================
# 6. Multi-model integration tests
# ============================================================================
if git diff --cached --name-only | grep -qE '\.rs$'; then
    echo "Running multi-model tests..."

    # Model selector unit tests
    if [ -d "$REPO_ROOT/conductor/core" ]; then
        if ! cargo test --manifest-path "$REPO_ROOT/conductor/core/Cargo.toml" \
            model_selector 2>/dev/null; then
            echo "ERROR: Model selector tests failed."
            exit 1
        fi
    fi

    # Multi-model integration tests
    if [ -d "$REPO_ROOT/tui" ]; then
        if ! cargo test --manifest-path "$REPO_ROOT/tui/Cargo.toml" \
            --test multi_model_integration_test 2>/dev/null; then
            echo "ERROR: Multi-model integration tests failed."
            echo "See tui/tests/multi_model_integration_test.rs for details."
            exit 1
        fi
    fi

    echo "Multi-model tests passed."
fi

# ============================================================================
# 7. Test that no actual LLM calls are made (CI safety)
# ============================================================================
if git diff --cached --name-only | grep -qE '\.rs$'; then
    echo "Checking for accidental LLM calls in tests..."

    # Ensure tests use mock backends
    if grep -r "OllamaBackend::from_env" --include="*_test.rs" "$REPO_ROOT"; then
        echo "WARNING: Found OllamaBackend::from_env in test files."
        echo "Use mock backends instead to avoid actual LLM calls."
        # Note: This is a warning, not a blocker, for ignored tests
    fi

    echo "LLM call check passed."
fi
```

---

## 6. CI/CD Considerations

### 6.1 GitHub Actions Workflow

```yaml
# .github/workflows/test.yml
name: Tests

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  # Disable any LLM env vars in CI
  OLLAMA_HOST: ""
  YOLLAYAH_MODEL: "mock"

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Run conductor-core unit tests
        run: cargo test --manifest-path conductor/core/Cargo.toml

      - name: Run model selector tests
        run: cargo test --manifest-path conductor/core/Cargo.toml model_selector

  integration-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Run TUI integration tests (mock backend)
        run: cargo test --manifest-path tui/Cargo.toml --test integration_test

      - name: Run multi-model integration tests
        run: cargo test --manifest-path tui/Cargo.toml --test multi_model_integration_test

  concurrency-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Run concurrent streaming tests
        run: cargo test --manifest-path tui/Cargo.toml concurrent -- --test-threads=1
        # Note: --test-threads=1 for deterministic concurrency testing

  # Optional: Real Ollama tests (manual trigger only)
  real-backend-tests:
    runs-on: ubuntu-latest
    if: github.event_name == 'workflow_dispatch'
    services:
      ollama:
        image: ollama/ollama:latest
        ports:
          - 11434:11434
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Wait for Ollama
        run: |
          for i in {1..30}; do
            curl -s http://localhost:11434/api/tags && break
            sleep 2
          done

      - name: Pull test model
        run: |
          curl -X POST http://localhost:11434/api/pull \
            -d '{"name": "tinyllama"}'

      - name: Run real backend tests
        env:
          OLLAMA_HOST: localhost
        run: cargo test --manifest-path tui/Cargo.toml -- --ignored
```

### 6.2 Test Timeouts

```rust
/// All integration tests should have timeouts
#[tokio::test]
#[timeout(5000)] // 5 second max
async fn test_with_timeout() {
    // ...
}

/// Use tokio's timeout for async operations
async fn wait_with_timeout<T>(
    future: impl Future<Output = T>,
    ms: u64,
) -> Result<T, tokio::time::error::Elapsed> {
    tokio::time::timeout(Duration::from_millis(ms), future).await
}
```

### 6.3 Test Categories for CI

```rust
/// Mark tests that require actual LLM
#[ignore] // Run with: cargo test -- --ignored
#[tokio::test]
async fn test_real_ollama_connection() {
    // ...
}

/// Mark slow tests
#[tokio::test]
#[cfg_attr(not(feature = "slow-tests"), ignore)]
async fn test_long_running_stream() {
    // ...
}

/// Mark flaky tests (for investigation)
#[tokio::test]
#[cfg_attr(feature = "ci", ignore)]
async fn test_timing_sensitive() {
    // ...
}
```

---

## 7. Test File Organization

```
ai-way/
├── conductor/core/
│   └── src/
│       ├── model_selector/
│       │   ├── mod.rs              # ModelSelector trait and types
│       │   ├── default.rs          # DefaultModelSelector implementation
│       │   └── tests.rs            # Unit tests (60+ cases)
│       └── backend/
│           ├── mod.rs
│           ├── mock.rs             # MockBackend (existing)
│           └── multi_mock.rs       # MultiModelMockBackend (new)
└── tui/
    └── tests/
        ├── integration_test.rs          # Existing tests
        ├── multi_model_integration_test.rs  # New multi-model tests
        └── test_utils/
            └── mod.rs              # Shared test utilities
```

---

## 8. Implementation Priority

1. **Phase 1: Foundation** (Week 1)
   - Implement `MultiModelMockBackend`
   - Implement `ModelSelector` trait and `DefaultModelSelector`
   - Write unit tests for model selection logic

2. **Phase 2: Integration** (Week 2)
   - Integration tests for single-model routing
   - Fallback behavior tests
   - Update pre-commit hooks

3. **Phase 3: Concurrency** (Week 3)
   - Concurrent streaming tests
   - Channel overflow tests
   - State management tests

4. **Phase 4: Edge Cases** (Week 4)
   - Input validation edge cases
   - Error recovery tests
   - Real backend tests (optional, ignored in CI)

---

## 9. Success Criteria

- [ ] All 60+ unit tests for ModelSelector pass
- [ ] All 10 integration test scenarios pass
- [ ] Pre-commit hooks include multi-model tests
- [ ] CI runs all tests in < 2 minutes
- [ ] No actual LLM calls in CI (verified by env checks)
- [ ] Test coverage > 80% for model selection code
- [ ] Zero flaky tests in main branch

---

## Appendix: Test Utilities Module

```rust
// File: tui/tests/test_utils/mod.rs

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Create a default test configuration
pub fn default_test_config() -> ConductorConfig {
    ConductorConfig {
        model: "general-13b".to_string(),
        warmup_on_start: false,
        greet_on_connect: false,
        max_context_messages: 10,
        system_prompt: None,
        limits: ConductorLimits {
            max_message_length: 10_000,
            max_messages_per_minute: 100,
            ..Default::default()
        },
        additional_agents: vec![],
    }
}

/// Assert that a specific model received a request
pub fn assert_model_called(
    backend: &MultiModelMockBackend,
    model: &str,
    times: usize,
) {
    let actual = backend.request_count(model);
    assert_eq!(
        actual, times,
        "Expected {} to be called {} times, but was called {} times",
        model, times, actual
    );
}

/// Assert that no model except the specified one was called
pub fn assert_only_model_called(
    backend: &MultiModelMockBackend,
    model: &str,
) {
    let history = backend.request_history();
    for req in &history {
        if req.model != model {
            panic!(
                "Expected only {} to be called, but {} was also called",
                model, req.model
            );
        }
    }
}

/// Drain all pending messages from a channel
pub fn drain_messages(rx: &mut mpsc::Receiver<ConductorMessage>) -> Vec<ConductorMessage> {
    let mut messages = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        messages.push(msg);
    }
    messages
}
```
