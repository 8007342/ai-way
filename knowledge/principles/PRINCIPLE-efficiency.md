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

### Law 1: No Sleep, Only Wait on Async I/O

**All I/O operations MUST return Futures, not block threads.**

The framework (tokio runtime) handles async wiring, thread management, and task scheduling. Your code operates on the **reified object** when the Future resolves (using `.await`).

#### Part A: No Sleep (Polling is Forbidden)

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

#### Part B: No Blocking I/O (All I/O Must Be Async)

**I/O functions MUST return `Future<Output = Result<T>>`, not block the thread.**

**FORBIDDEN - Blocking I/O**:
```rust
// ❌ TERRIBLE - Blocks thread waiting for file system
use std::fs;
let contents = fs::read_to_string("file.txt")?; // Blocks!

// ❌ TERRIBLE - Blocks thread waiting for network
use std::net::TcpStream;
let stream = TcpStream::connect("127.0.0.1:8080")?; // Blocks!

// ❌ TERRIBLE - Blocking read trait
use std::io::Read;
let mut file = std::fs::File::open("data.bin")?; // Blocks!
file.read_to_end(&mut buffer)?; // Blocks!

// ❌ TERRIBLE - Synchronous HTTP client (blocks thread pool)
let response = reqwest::blocking::get("https://api.example.com")?; // Blocks!
```

**REQUIRED - Async I/O**:
```rust
// ✅ GOOD - Async file I/O (returns Future, runtime manages it)
use tokio::fs;
let contents = fs::read_to_string("file.txt").await?; // Non-blocking!

// ✅ GOOD - Async network I/O
use tokio::net::TcpStream;
let stream = TcpStream::connect("127.0.0.1:8080").await?; // Non-blocking!

// ✅ GOOD - Async read trait
use tokio::io::AsyncReadExt;
let mut file = tokio::fs::File::open("data.bin").await?; // Non-blocking!
file.read_to_end(&mut buffer).await?; // Non-blocking!

// ✅ GOOD - Async HTTP client
let response = reqwest::get("https://api.example.com").await?; // Non-blocking!
```

**Why This Matters**:
- **Blocking I/O wastes threads**: If you have 8 threads in tokio runtime and all block on I/O, no other tasks can run
- **Async I/O is efficient**: Thread can handle 1000s of concurrent I/O operations using event loop (epoll/kqueue)
- **Runtime does the wiring**: You write simple `.await` code, tokio handles thread parking, waking, scheduling

**Common Blocking → Async Replacements**:

| Blocking (❌ FORBIDDEN) | Async (✅ REQUIRED) |
|-------------------------|---------------------|
| `std::fs::read()` | `tokio::fs::read().await` |
| `std::fs::write()` | `tokio::fs::write().await` |
| `std::fs::File::open()` | `tokio::fs::File::open().await` |
| `std::net::TcpStream::connect()` | `tokio::net::TcpStream::connect().await` |
| `std::net::TcpListener::accept()` | `tokio::net::TcpListener::accept().await` |
| `std::io::Read` trait | `tokio::io::AsyncRead` trait |
| `std::io::Write` trait | `tokio::io::AsyncWrite` trait |
| `reqwest::blocking::get()` | `reqwest::get().await` |
| `std::process::Command::output()` | `tokio::process::Command::output().await` |

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

## CRITICAL: ALL `.await` CALLS ARE STRICTLY FORBIDDEN

**STATUS**: 🔴 STRICTLY FORBIDDEN - Any `.await` anywhere is a BUG

**This is a fundamental architectural principle. Do NOT use `.await` ANYWHERE in the Rust codebase.**

### The Core Problem

**The current codebase misuses async/await. It does "unnecessary parallel stuff locally" while simultaneously doing operations that require non-blocking in blocking ways. This is the OPPOSITE of correct async architecture.**

### What Is Forbidden

**STRICTLY FORBIDDEN - Zero Tolerance**:
- ✖️ `.await` - ALL forms, ALL locations
- ✖️ `.wait()` - ALL forms of blocking waits
- ✖️ `.get()` - Blocking gets on futures/channels
- ✖️ `.recv().await` - Blocking channel receives
- ✖️ `.send().await` - Blocking channel sends
- ✖️ `sleep()` / `thread::sleep()` - ALL forms of sleep
- ✖️ Manual thread spawning (`thread::spawn`) - Framework handles threading
- ✖️ Manual thread pools - Framework provides thread pooling

**Root Cause**: These patterns treat async as "threading with extra steps". This causes:
1. **Memory leaks** - Improper resource cleanup
2. **Thread pool exhaustion** - Fighting the framework's thread management
3. **Blocking operations** - Defeating the purpose of async
4. **CPU/GPU coordination issues** - Manual parallelism conflicts with framework

### ❌ WRONG: Current Async/Await Anti-Pattern

```rust
// ❌ WRONG - Uses .await (FORBIDDEN!)
pub async fn poll_streaming(&mut self) -> bool {
    let rx = self.streaming_rx.as_mut()?;

    // ❌ FORBIDDEN: .await
    match rx.recv().await {  // ← BLOCKS, CAUSES MEMORY LEAKS
        Some(token) => {
            process(token);  // ← Manual processing, not reactive
            true
        }
        None => false,
    }
}

// ❌ WRONG: Manual event loop with .await
while self.running {
    tokio::select! {  // ← tokio::select! is manual coordination
        event = event_stream.next() => { ... }  // ← .await implicit
        _ = tick.tick() => { ... }  // ← .await implicit
    }

    // ❌ FORBIDDEN: .await
    self.conductor.poll_streaming().await;  // ← Manual await

    self.update();
    self.render(terminal)?;
}
```

**Why This Is Wrong**:
1. **Manual `.await` = Manual thread management** - Fighting the framework
2. **Manual event loop** - Framework should handle event coordination
3. **Blocking operations** - Defeats async architecture
4. **Memory leaks** - Resources not properly managed by framework
5. **Thread pool conflicts** - CPU/GPU coordination breaks down
6. **No backpressure** - Manual polling can't handle flow control

### ✅ CORRECT: Reactive Streams / Observables Pattern

**Use reactive streams with framework-managed coordination:**

```rust
use tokio_stream::{StreamExt, wrappers::ReceiverStream};
use futures::stream::{Stream, select_all};

// ✅ CORRECT - Reactive stream composition (NO .await!)
pub fn create_event_pipeline() -> impl Stream<Item = AppEvent> {
    // Create streams from various sources
    let token_stream = ReceiverStream::new(token_rx)
        .map(AppEvent::Token);

    let terminal_stream = TerminalEventStream::new()
        .map(AppEvent::Terminal);

    let tick_stream = IntervalStream::new(interval(Duration::from_millis(100)))
        .map(|_| AppEvent::Tick);

    // Framework merges streams (NO manual coordination!)
    select_all(vec![
        token_stream.boxed(),
        terminal_stream.boxed(),
        tick_stream.boxed(),
    ])
}

// ✅ CORRECT - Reactive event handler (NO .await, NO manual loop!)
pub fn run_app(mut app: App) {
    let event_pipeline = create_event_pipeline();

    // Framework handles the event loop!
    event_pipeline
        .for_each(|event| {
            app.handle_event(event);
            app.update();
            app.render();
            // Return ready future (framework manages execution)
            futures::future::ready(())
        })
        .run();  // Framework runs the pipeline
}
```

**Why This Is Correct**:
1. **No `.await`** - Framework manages all async coordination
2. **Reactive composition** - Declare WHAT, not HOW
3. **Automatic backpressure** - Framework handles flow control
4. **No manual threading** - Framework manages thread pool
5. **No memory leaks** - Framework manages resource cleanup
6. **Proper CPU/GPU coordination** - Framework schedules work correctly

### ✅ ALTERNATIVE: Observable Pattern (RxRust)

**For even more powerful reactive composition:**

```rust
use rxrust::prelude::*;

// ✅ CORRECT - Observable composition
pub fn create_app_observable() -> impl Observable<Item = AppState> {
    // Combine multiple event sources
    observable::merge((
        token_observable().map(Event::Token),
        terminal_observable().map(Event::Terminal),
        interval(Duration::from_millis(100), |_| Event::Tick),
    ))
    // Transform events into state updates
    .scan(AppState::default(), |state, event| {
        state.handle_event(event);
        Some(state.clone())
    })
    // Side effects (rendering) - framework manages when this runs!
    .tap(|state| {
        render(state);
    })
}

// Framework runs everything
create_app_observable()
    .subscribe(|_state| {
        // State updates automatically trigger renders above
    });
```

**Advantages**:
- **Declarative** - Describe the data flow, not the execution
- **Composable** - Combine streams with operators
- **Testable** - Pure transformations, no hidden state
- **Framework-managed** - All threading/async handled automatically

### Rule Summary

**STRICTLY FORBIDDEN**:

| Pattern | Status | Reason |
|---------|--------|--------|
| `.await` anywhere | ❌ **FORBIDDEN** | Manual async = thread management bugs |
| `.wait()` anywhere | ❌ **FORBIDDEN** | Blocking = defeats async |
| `tokio::select!` | ❌ **FORBIDDEN** | Manual coordination = memory leaks |
| `tokio::spawn` | ❌ **FORBIDDEN** | Manual threading = framework conflicts |
| `async fn` with manual loops | ❌ **FORBIDDEN** | Manual loops = no backpressure |

**REQUIRED**:

| Pattern | Status | Reason |
|---------|--------|--------|
| Reactive streams (tokio-stream) | ✅ **REQUIRED** | Framework manages everything |
| Observables (RxRust) | ✅ **PREFERRED** | Most powerful composition |
| Stream combinators (map, filter, etc) | ✅ **REQUIRED** | Declarative transformations |
| Framework-managed execution | ✅ **REQUIRED** | Let framework handle async |

### Migration Strategy

**ALL EXISTING CODE WITH `.await` MUST BE REWRITTEN**:

1. **Identify async boundaries** - Where do events come from?
2. **Convert to streams** - Wrap sources in ReceiverStream, IntervalStream, etc.
3. **Compose declaratively** - Use map, filter, merge, scan, etc.
4. **Let framework run** - No manual `.await`, no manual loops
5. **Test reactive behavior** - Framework handles timing/coordination

**See EPIC-nnn-TUI-overhaul.md** for complete architectural migration plan

**Related Bugs**: `TODO-BUG-001-tui-waits-for-full-stream.md` (temporary fix, needs full rewrite)

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
| No Blocking I/O | Use `tokio::fs`, `tokio::net`, not `std::fs`, `std::net` | CRITICAL |
| Futures Only | I/O functions return `Future<Output = T>`, not `T` | CRITICAL |
| Lazy Init | Load on-demand, cache aggressively | HIGH |
| Thin Clients | Surfaces are display-only | CRITICAL |
| Dirty Tracking | Only render changed regions | MEDIUM |
| Event-Driven | React, don't poll | CRITICAL |

**Remember**:
- If you're using `sleep()`, you're probably doing it wrong.
- If you're using `std::fs` or `std::net`, use `tokio::fs` or `tokio::net` instead.
