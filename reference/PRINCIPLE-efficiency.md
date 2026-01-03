# PRINCIPLE: Efficiency - Fully Async, Zero Sleep, Aggressive Caching

**Status**: ✅ Core Architectural Principle
**Applies To**: Conductor, TUI, All Surfaces
**Severity**: CRITICAL - Violations are BUGs

---

## Core Principle

**AI-Way is a fully asynchronous system with ZERO tolerance for inefficiency.**

All I/O operations MUST be non-blocking. Resources MUST be lazy-initialized and aggressively cached. Surfaces MUST be thin clients with negligible performance impact.

---

## The Three Laws of Async Efficiency

### Law 1: No Sleep, Only Wait on I/O

**FORBIDDEN**:
```rust
// ❌ TERRIBLE - Blocks the runtime
std::thread::sleep(Duration::from_millis(10));

// ❌ TERRIBLE - Wastes CPU cycles
tokio::time::sleep(Duration::from_millis(10)).await;

// ❌ TERRIBLE - Polling loop
loop {
    check_something();
    tokio::time::sleep(Duration::from_millis(10)).await;
}
```

**REQUIRED**:
```rust
// ✅ GOOD - Wait on actual I/O
let data = file.read().await?;

// ✅ GOOD - Wait on channel
let msg = rx.recv().await?;

// ✅ GOOD - Wait on notification
notify.notified().await;

// ✅ GOOD - Event-driven, not polling
tokio::select! {
    event = event_stream.next() => { ... }
    msg = rx.recv() => { ... }
}
```

**Exception - Frame Rate Limiting (TUI ONLY)**:
```rust
// ✅ ACCEPTABLE - Frame rate control for rendering
let frame_duration = Duration::from_millis(100); // 10 FPS
let elapsed = frame_start.elapsed();
if elapsed < frame_duration {
    tokio::time::sleep(frame_duration - elapsed).await; // OK for frame limiting
}
```

**Exception - Exponential Backoff (Retry Logic ONLY)**:
```rust
// ✅ ACCEPTABLE - Rate limiting / backoff
async fn retry_with_backoff() {
    let mut backoff = Duration::from_millis(100);
    loop {
        match try_operation().await {
            Ok(result) => return result,
            Err(_) => {
                tokio::time::sleep(backoff).await; // OK for backoff
                backoff = (backoff * 2).min(Duration::from_secs(30));
            }
        }
    }
}
```

---

### Law 2: Lazy Initialization, Aggressive Caching

**Resources MUST be initialized on-demand and cached for reuse.**

**FORBIDDEN**:
```rust
// ❌ TERRIBLE - Recalculates every frame
fn render(&self) {
    for msg in messages {
        let wrapped = textwrap::wrap(&msg.content, width); // ← Recalculates!
        // ...
    }
}

// ❌ TERRIBLE - Reloads every time
async fn get_model(&self) -> Model {
    let model = load_model_from_disk().await; // ← Reloads!
    model
}
```

**REQUIRED**:
```rust
// ✅ GOOD - Cache wrapped text
struct DisplayMessage {
    content: String,
    wrapped_cache: RefCell<Option<Vec<String>>>,
    cache_width: RefCell<Option<usize>>,
}

impl DisplayMessage {
    fn get_wrapped(&self, width: usize) -> &[String] {
        let mut cache = self.wrapped_cache.borrow_mut();
        if self.cache_width.borrow().as_ref() != Some(&width) {
            *cache = Some(textwrap::wrap(&self.content, width));
            *self.cache_width.borrow_mut() = Some(width);
        }
        cache.as_ref().unwrap()
    }
}

// ✅ GOOD - Lazy init with Arc for sharing
struct ModelCache {
    models: DashMap<String, Arc<Model>>,
}

impl ModelCache {
    async fn get_or_load(&self, name: &str) -> Arc<Model> {
        if let Some(model) = self.models.get(name) {
            return model.clone(); // Cheap Arc clone
        }

        let model = Arc::new(load_model_from_disk(name).await);
        self.models.insert(name.to_string(), model.clone());
        model
    }
}
```

---

### Law 3: Surfaces Are Thin Clients

**Surfaces (TUI, Web UI, CLI) MUST have negligible performance impact.**

**REQUIRED Characteristics**:
1. **No business logic** - Surfaces only display state, send events
2. **No expensive calculations** - All computation in Conductor
3. **Dirty tracking** - Only re-render changed regions
4. **Efficient data structures** - Minimize allocations, reuse buffers
5. **Event-driven** - React to state changes, don't poll

**FORBIDDEN**:
```rust
// ❌ TERRIBLE - Surface doing business logic
impl TUI {
    async fn handle_input(&mut self, input: String) {
        let validated = validate_input(&input); // ← Business logic!
        let response = call_llm(&validated).await; // ← LLM call!
        self.display_response(response);
    }
}
```

**REQUIRED**:
```rust
// ✅ GOOD - Surface sends event, Conductor handles logic
impl TUI {
    async fn handle_input(&mut self, input: String) {
        // Send event to Conductor
        self.conductor.send_event(SurfaceEvent::UserMessage {
            content: input,
        }).await;

        // Conductor will send back ConductorMessage with result
        // TUI just waits for state updates
    }
}
```

---

## Async Patterns

### ✅ GOOD: Non-Blocking Channel Operations

```rust
// Non-blocking drain
while let Ok(token) = rx.try_recv() {
    process(token);
}

// Async receive with select
tokio::select! {
    Some(msg) = rx.recv() => process(msg),
    _ = shutdown.notified() => break,
}
```

### ✅ GOOD: Event-Driven Architecture

```rust
// Wait on multiple async sources
tokio::select! {
    biased; // Prioritize user input

    Some(event) = terminal_events.next() => {
        handle_terminal_event(event);
    }

    Some(msg) = conductor_rx.recv() => {
        handle_conductor_message(msg);
    }

    _ = tick_interval.tick() => {
        update_animations();
    }
}
```

### ✅ GOOD: Notification-Based, Not Polling

```rust
// ❌ BAD: Polling loop
loop {
    if has_work() {
        do_work();
    }
    tokio::time::sleep(Duration::from_millis(10)).await; // TERRIBLE!
}

// ✅ GOOD: Notification-driven
loop {
    work_available.notified().await; // Block until work
    while has_work() {
        do_work();
    }
}
```

---

## Buffer and Channel Sizing

**Channels MUST be sized appropriately for message frequency.**

```rust
// ✅ GOOD: Size based on expected load
let (tx, rx) = mpsc::channel(256); // Streaming tokens (200/sec)
let (tx, rx) = mpsc::channel(100); // UI events (10/sec)
let (tx, rx) = mpsc::channel(512); // High-throughput logs
```

**Pre-allocate buffers for known sizes:**

```rust
// ✅ GOOD: Pre-allocate capacity
let mut content = String::with_capacity(4096);
let mut buffer = Vec::with_capacity(100);
```

---

## Performance Budgets

### TUI (Surface)
- **Idle CPU**: < 0.1% (MUST be negligible when not rendering)
- **Active CPU**: < 5% during streaming
- **Frame rate**: 10 FPS (100ms frame time)
- **Memory**: < 50 MB baseline, < 100 MB during streaming
- **Allocations**: < 1000/sec during rendering, < 100/sec idle

### Conductor (Core)
- **Idle CPU**: < 1% (event loop only)
- **Active CPU**: Depends on LLM backend (not our concern)
- **Memory**: < 200 MB baseline
- **Latency**: < 1ms for message passing (InProcess transport)

---

## Initialization and Warmup

**Surfaces MUST indicate initialization state clearly.**

**REQUIRED**:
1. **Never appear frozen** - Always show activity (spinner, pulse, animation)
2. **Progressive initialization** - Load critical components first
3. **Clear status messages** - "Loading models...", "Connecting...", etc.
4. **Cancel on user input** - Allow skipping warmup if possible

```rust
// ✅ GOOD: Incremental startup with status
enum StartupPhase {
    InitializingConductor,
    LoadingModels,
    ConnectingBackend,
    Ready,
}

impl TUI {
    async fn run(&mut self) {
        let mut phase = StartupPhase::InitializingConductor;

        loop {
            // Update status UI every frame
            self.render_startup_status(&phase);

            match phase {
                StartupPhase::InitializingConductor => {
                    self.conductor = Conductor::new().await;
                    phase = StartupPhase::LoadingModels;
                }
                StartupPhase::LoadingModels => {
                    // Show "Loading models..." with spinner
                    self.conductor.ensure_models_ready().await;
                    phase = StartupPhase::ConnectingBackend;
                }
                // ...
                StartupPhase::Ready => break,
            }
        }

        // Now run main event loop
        self.run_main_loop().await;
    }
}
```

---

## Violations and Bug Tracking

**Any violation of these principles is a BUG and MUST be tracked.**

### Filing a Bug for Violations

1. Create `BUG-XXX-<issue-name>.md`
2. Reference the violating file(s) and line numbers
3. Link to this principle document
4. Create corresponding `TODO-XXX-<fix-name>.md` for remediation

Example:
```markdown
# BUG-015: Sleep calls in polling loops

**Severity**: CRITICAL
**Principle Violated**: PRINCIPLE-efficiency.md (Law 1: No Sleep)

## Violations

1. `conductor/core/src/bin/conductor-daemon.rs:231`
   - `tokio::time::sleep(10ms)` in main poll loop
   - Should use notification-based wake

2. `tui/src/conductor_client.rs:245`
   - `sleep(delay_ms)` in retry logic
   - Should use exponential backoff with tokio::time::Interval

## Fix Required

See `TODO-015-eliminate-polling-sleeps.md`
```

---

## Examples: Before & After

### Example 1: Polling → Event-Driven

**BEFORE** (❌ TERRIBLE):
```rust
// conductor-daemon.rs
tokio::spawn(async move {
    loop {
        conductor.poll_streaming().await;
        tokio::time::sleep(Duration::from_millis(10)).await; // ← TERRIBLE!
    }
});
```

**AFTER** (✅ GOOD):
```rust
// conductor-daemon.rs
tokio::spawn(async move {
    loop {
        // Block until streaming starts
        conductor.streaming_started.notified().await;

        // Poll only while streaming
        while conductor.has_active_streaming() {
            conductor.poll_streaming_nonblocking();
            tokio::task::yield_now().await; // Yield to other tasks
        }
    }
});
```

### Example 2: Wasteful Calculation → Cached

**BEFORE** (❌ TERRIBLE):
```rust
fn render_conversation(&mut self) {
    for msg in &self.messages {
        let wrapped = textwrap::wrap(&msg.content, width); // ← Every frame!
        // ...
    }
}
```

**AFTER** (✅ GOOD):
```rust
fn render_conversation(&mut self) {
    // Only re-wrap if width changed or content changed
    if self.conversation_dirty || self.width_changed {
        for msg in &mut self.messages {
            msg.invalidate_wrap_cache();
        }
    }

    for msg in &self.messages {
        let wrapped = msg.get_wrapped(width); // ← Cached!
        // ...
    }
}
```

---

## Monitoring and Enforcement

**CI/CD MUST enforce these principles:**

1. **Linter rules** - Deny `std::thread::sleep`, warn on `tokio::time::sleep` (except frame limiting)
2. **Performance tests** - Measure idle CPU, allocations/sec, latency
3. **Code review** - All async code reviewed by Rust expert
4. **Benchmarks** - Regression detection for allocations and CPU

---

## Summary

| Principle | Rule | Violation Severity |
|-----------|------|-------------------|
| No Sleep | Only wait on I/O, never `sleep()` | CRITICAL |
| Lazy Init | Load on-demand, cache aggressively | HIGH |
| Thin Clients | Surfaces are display-only | CRITICAL |
| Dirty Tracking | Only render changed regions | MEDIUM |
| Async I/O | All I/O is non-blocking | CRITICAL |
| Event-Driven | React, don't poll | CRITICAL |

**Remember**: If you're using `sleep()`, you're probably doing it wrong.
