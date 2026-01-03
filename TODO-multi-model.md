# TODO: Multi-Model Support

> Comprehensive plan for supporting multiple AI models with intelligent routing, fallback chains, and specialized handling for different task types.

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Implementation Tasks](#implementation-tasks)
4. [Ethics & Privacy Requirements](#ethics--privacy-requirements)
5. [Testing Strategy](#testing-strategy)
6. [Configuration](#configuration)
7. [Files Created](#files-created)
8. [References](#references)

---

## Overview

### Goal

Enable ai-way to route queries to the most appropriate model based on task type:

| Task Type | Model Category | Routing Strategy |
|-----------|---------------|------------------|
| Code generation | Specialized | Always use code model (e.g., DeepSeek-Coder, CodeLlama) |
| Mathematical | Specialized | Always use math model (e.g., Qwen2-Math) |
| Quick responses | General | Meta-agent selects fast model |
| Deep thinking | General | Meta-agent selects large model |
| Creative writing | General | Meta-agent selects creative model |
| Image recognition | Future | Specialized vision model |
| Image generation | Future | Specialized diffusion model |

### Key Principles

1. **Local-first**: All models run locally by default; external APIs require explicit consent
2. **Transparent**: Users see which model handles their request
3. **Graceful fallback**: When primary model fails, fallback chain activates
4. **No dark patterns**: Honest about quotas and capabilities

---

## Architecture

### A. Core Types

```rust
// Task classification for routing
pub enum TaskClass {
    QuickResponse,      // <100ms first token
    DeepThinking,       // 5-30s total time OK
    CodeGeneration,     // moderate latency, high precision
    Mathematical,       // precision over speed
    Creative,           // artistic/writing tasks
    ToolUse,           // function calling
    Embedding,         // vector generation
    General,           // fallback
}

// Model capability declarations
pub enum ModelCapability {
    TextGeneration,
    CodeGeneration,
    MathReasoning,
    LowLatency,
    DeepReasoning,
    CreativeWriting,
    ToolUse,
    VisionInput,
    AudioInput,
    ImageGeneration,
}

// Performance characteristics
pub enum LatencyClass {
    Instant,     // <100ms first token
    Fast,        // 100-500ms
    Standard,    // 500ms-2s
    Deliberate,  // 2s+ (thinking models)
}
```

### B. ModelRouter Component

```rust
pub struct ModelRouter {
    /// Available model profiles
    profiles: HashMap<String, ModelProfile>,
    /// Default model per task class
    defaults: HashMap<TaskClass, String>,
    /// Fallback chains
    fallbacks: HashMap<String, Vec<String>>,
    /// Health tracking
    health: HashMap<String, ModelHealth>,
}

impl ModelRouter {
    /// Select best model for request
    fn select(&self, request: &RoutingRequest) -> RoutingDecision;

    /// Update model health after request
    fn record_outcome(&mut self, model: &str, outcome: RequestOutcome);

    /// Get fallback chain for model
    fn fallback_chain(&self, model: &str) -> Vec<String>;
}
```

### C. ModelProfile

```rust
pub struct ModelProfile {
    pub model_id: String,
    pub backend_id: String,
    pub strengths: Vec<TaskClass>,
    pub weaknesses: Vec<TaskClass>,
    pub avg_ttft_ms: u64,          // time to first token
    pub avg_tokens_per_sec: f32,
    pub max_context: u32,
    pub memory_bytes: Option<u64>,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub cost_tier: CostTier,
}
```

### D. Connection Pooling

```rust
pub struct ConnectionPool {
    backend_id: String,
    config: ConnectionConfig,
    connections: RwLock<Vec<ConnectionSlot>>,
    semaphore: Semaphore,
    stats: PoolStatsAtomic,
}

pub struct PoolManager {
    pools: RwLock<HashMap<String, Arc<ConnectionPool>>>,
    default_config: ConnectionConfig,
}
```

### E. GPU Memory Management

```rust
pub struct WeightedSemaphore {
    capacity: u64,
    available: AtomicU64,
    waiters: Mutex<VecDeque<Waiter>>,
}

pub struct GpuMemoryManager {
    semaphore: WeightedSemaphore,
    loaded_models: Mutex<HashMap<String, ModelAllocation>>,
    pressure_threshold: f64,
}
```

---

## Implementation Tasks

### Phase 1: Foundation

- [ ] **A1. Create ModelSelector trait and types**
  - File: `conductor/core/src/routing/mod.rs`
  - Define `TaskClass`, `ModelCapability`, `SelectionReason` enums
  - Define `ModelSelection`, `SelectionContext` structs
  - Define `ModelSelector` trait

- [ ] **A2. Implement DefaultModelSelector**
  - File: `conductor/core/src/routing/selector.rs`
  - Keyword-based classification for code/math queries
  - Complexity analysis for quick vs deep routing
  - Creative writing detection

- [ ] **A3. Create ModelProfile and configuration**
  - File: `conductor/core/src/routing/config.rs`
  - Define `ModelProfile`, `CostTier`, `LatencyClass` types
  - Configuration loading from TOML
  - Runtime model discovery from Ollama

- [ ] **A4. Add routing to Conductor**
  - File: `conductor/core/src/conductor.rs`
  - Integrate `ModelRouter` into message handling
  - Route requests through selector before LLM call
  - Track which model handled each message

### Phase 2: Fallback & Recovery

- [x] **B1. Implement fallback chains**
  - File: `conductor/core/src/routing/fallback.rs`
  - Define fallback sequences per model
  - Automatic failover on model error
  - User notification on fallback
  - **DONE**: FallbackChainManager wired into QueryRouter

- [x] **B2. Health tracking**
  - File: `conductor/core/src/routing/health.rs`
  - Track success/failure rates per model
  - Exponential backoff on repeated failures
  - Recovery detection after model becomes available
  - **DONE**: HealthTracker wired into QueryRouter with time_since_last_success fix

- [ ] **B3. Model availability manager**
  - Real-time model availability checking
  - Graceful degradation when models unavailable
  - Admin alerts for persistent failures

### Phase 3: Connection Management

- [ ] **C1. Connection pooling**
  - File: `conductor/core/src/routing/connection_pool.rs`
  - Per-backend connection pools
  - Configurable pool sizes
  - Connection recycling and health checks

- [ ] **C2. GPU memory management**
  - File: `conductor/core/src/routing/semaphore.rs`
  - Weighted semaphore for VRAM allocation
  - Model loading/unloading coordination
  - Memory pressure handling

- [ ] **C3. Rate limiting**
  - File: `conductor/core/src/routing/rate_limit.rs`
  - Per-model rate limits
  - Per-user fair queuing
  - Quota tracking (if applicable)

### Phase 4: UI Integration

- [ ] **D1. Model indicator in TUI**
  - File: `tui/src/display.rs`
  - Show current model name during response
  - Visual indicator for specialized vs general model
  - Cloud icon for external API models

- [ ] **D2. Model selection messages**
  - ConductorMessage variants for model info
  - Accessibility announcements for model changes
  - User-friendly model names (via terminology dictionary)

- [ ] **D3. User model override**
  - Allow user to request specific model
  - Persist preference per conversation
  - Clear model lock command

### Phase 5: Testing

- [ ] **E1. Unit tests for ModelSelector**
  - 60+ test cases per testing strategy
  - Keyword detection tests
  - Fallback chain tests
  - Confidence score tests

- [ ] **E2. Integration tests**
  - 10 integration scenarios
  - MultiModelMockBackend implementation
  - Concurrent streaming tests

- [ ] **E3. Pre-commit hook updates**
  - Add model selector tests
  - Add multi-model integration tests
  - Verify no actual LLM calls in CI

---

## Ethics & Privacy Requirements

From `agents/ai-way-docs/multi-model-ethics-guidelines.md`:

### License Hierarchy

| Priority | License Type | Examples |
|----------|--------------|----------|
| 1 (Preferred) | Permissive Open-Weight | Apache 2.0, MIT (Llama 3.x, Mistral, Phi-3) |
| 2 (Acceptable) | Copyleft Open-Weight | GPL, LGPL |
| 3 (Careful) | Restricted Open-Weight | Llama 2 Community License |
| 4 (External Only) | Proprietary API | OpenAI, Anthropic (user brings key) |

### Privacy Requirements

- **Local-first default**: All models run locally
- **Explicit consent**: Required before data leaves device
- **Secret scanning**: Pre-scan code for credentials before external API
- **EXIF stripping**: Remove metadata from images

### Transparency Requirements

- Users see which model handles their request
- Users see whether model is local or external
- Specialist handoffs are visible ("Getting code expertise...")

### Warning Labels

| Domain | Requirement |
|--------|-------------|
| Math | "Verify important calculations" |
| Code | "Review before using - may contain bugs" |
| External API | "Your data will be sent to [provider]" |
| High-stakes | Enhanced warnings for tax/medical/legal |

### Anti-Patterns to AVOID

- Manufactured urgency ("Only 3 questions left!")
- Silent cloud fallback
- Quality degradation without notice
- Hidden limits

---

## Testing Strategy

From `docs/testing/multi-model-testing-strategy.md`:

### Mock Backend

```rust
pub struct MultiModelMockBackend {
    models: HashMap<String, MockModelConfig>,
    request_history: Arc<Mutex<Vec<ModelRequest>>>,
    unavailable_models: Arc<Mutex<HashSet<String>>>,
    request_counts: Arc<Mutex<HashMap<String, usize>>>,
}
```

### Test Categories

| Category | Count | Priority |
|----------|-------|----------|
| Model Selection Unit Tests | 60+ | High |
| Integration Test Scenarios | 10 | High |
| Input Validation Edge Cases | 7 | High |
| Model Selection Edge Cases | 7 | Medium |
| Model Availability Edge Cases | 6 | Critical |
| Concurrency Edge Cases | 5 | High |
| State Management Edge Cases | 4 | Medium |

### Pre-commit Hooks

```bash
# Model selector unit tests
cargo test --manifest-path conductor/core/Cargo.toml model_selector

# Multi-model integration tests
cargo test --manifest-path tui/Cargo.toml --test multi_model_integration_test
```

### CI/CD

- All tests must pass without actual LLM calls
- Mock backends only in CI
- Optional real-backend tests (manual trigger)
- Test timeout: 5 seconds max per test

---

## Configuration

### Default Model Configuration

```toml
# config/models.toml

[defaults]
general = "llama3.1:8b"
code = "deepseek-coder:6.7b"
math = "qwen2-math:7b"
quick = "phi3:mini"
deep = "llama3.1:8b"  # 70b too large for most hardware, 8b is more realistic
creative = "mistral:7b"

[routing]
# Keyword patterns for specialized routing
code_keywords = ["function", "code", "debug", "implement", "refactor"]
math_keywords = ["calculate", "solve", "equation", "integral", "prove"]
creative_keywords = ["write", "poem", "story", "compose", "imagine"]

[fallbacks]
code = ["deep", "general"]
math = ["deep", "general"]
quick = ["general"]
creative = ["general"]

[timeouts]
quick_ms = 100
standard_ms = 2000
deep_ms = 30000

[privacy]
default_location = "local"
allow_external = false  # User must opt-in
secret_scan_before_external = true
```

### Per-Model Profile

```toml
[[models]]
id = "deepseek-coder:6.7b"
backend = "ollama"
strengths = ["CodeGeneration", "ToolUse"]
weaknesses = ["Creative"]
avg_ttft_ms = 150
avg_tokens_per_sec = 40.0
max_context = 16384
memory_bytes = 6_000_000_000
supports_streaming = true
supports_tools = true
cost_tier = "free"
```

---

## Files Created

| File | Purpose | Created By |
|------|---------|------------|
| `agents/ai-way-docs/multi-model-ethics-guidelines.md` | Ethical guidelines for multi-model support | Legal/Ethics Agent |
| `docs/testing/multi-model-testing-strategy.md` | Comprehensive testing strategy | QA/Intern Agent |
| `TODO-multi-model.md` | This planning document | Conductor |

### Files To Create

| File | Purpose | Priority |
|------|---------|----------|
| `conductor/core/src/routing/mod.rs` | Module structure | P0 |
| `conductor/core/src/routing/config.rs` | Configuration types | P0 |
| `conductor/core/src/routing/selector.rs` | ModelSelector implementation | P0 |
| `conductor/core/src/routing/fallback.rs` | Fallback chain logic | P1 |
| `conductor/core/src/routing/health.rs` | Health tracking | P1 |
| `conductor/core/src/routing/connection_pool.rs` | Connection pooling | P2 |
| `conductor/core/src/routing/semaphore.rs` | GPU memory management | P2 |
| `conductor/core/src/routing/rate_limit.rs` | Rate limiting | P3 |
| `tui/tests/multi_model_integration_test.rs` | Integration tests | P1 |

---

## References

- [`CONSTITUTION.md`](agents/CONSTITUTION.md) - Foundational ethical principles
- [`agents/ai-way-docs/multi-model-ethics-guidelines.md`](agents/ai-way-docs/multi-model-ethics-guidelines.md) - Ethics guidelines
- [`docs/testing/multi-model-testing-strategy.md`](docs/testing/multi-model-testing-strategy.md) - Testing strategy
- [`agents/ai-way-docs/meta-agent-architecture.md`](agents/ai-way-docs/meta-agent-architecture.md) - Agent routing design
- [`agents/ai-way-docs/terminology-dictionary.md`](agents/ai-way-docs/terminology-dictionary.md) - User-friendly terminology

---

## Commit History

| Date | Commit | Description |
|------|--------|-------------|
| 2026-01-01 | (pending) | Initial multi-model architecture design |

---

**Status**: In Progress
**Last Updated**: 2026-01-01
**Contributors**: Architect Agent, Legal/Ethics Agent, Hacker Agent, Intern/QA Agent
