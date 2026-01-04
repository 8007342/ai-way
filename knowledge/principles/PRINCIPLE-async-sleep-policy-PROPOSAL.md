# PRINCIPLE: Async Sleep Policy - When `tokio::time::sleep()` Is Acceptable

**Status**: PROPOSAL - Refining PRINCIPLE-efficiency.md
**Date**: 2026-01-04
**Applies To**: Conductor, TUI, All Surfaces
**Severity**: CRITICAL - Violations are BUGs
**Related**: `knowledge/principles/PRINCIPLE-efficiency.md` (Law 1: No Sleep, Only Wait on Async I/O)

---

## Executive Summary

**The principle "No Sleep, Only Wait on Async I/O" is CORRECT in spirit but needs refinement to distinguish between anti-patterns and legitimate use cases.**

This document clarifies:
1. **Blocking sleep (`std::thread::sleep`)** - ALWAYS FORBIDDEN in async code
2. **Async sleep (`tokio::time::sleep`)** - CONDITIONAL (forbidden in polling loops, acceptable for timing control)
3. **Event-driven timers (`tokio::time::Interval`)** - PREFERRED for recurring events

The current PRINCIPLE-efficiency.md is **90% correct** but conflates:
- ❌ Anti-pattern: Polling loops with sleep (FORBIDDEN)
- ✅ Legitimate: Frame rate limiting, exponential backoff, rate limiting (ACCEPTABLE)

---

## The Confusion: Why This Needs Clarification

### What PRINCIPLE-efficiency.md Says

```rust
// ❌ TERRIBLE - Wastes CPU cycles
tokio::time::sleep(Duration::from_millis(10)).await;

// ❌ TERRIBLE - Polling loop
loop {
    check_something();
    tokio::time::sleep(Duration::from_millis(10)).await;
}
```

**But later it says:**

```rust
// ✅ ACCEPTABLE - Frame rate control for rendering
let frame_duration = Duration::from_millis(100); // 10 FPS
let elapsed = frame_start.elapsed();
if elapsed < frame_duration {
    tokio::time::sleep(frame_duration - elapsed).await; // OK for frame limiting
}

// ✅ ACCEPTABLE - Rate limiting / backoff
async fn retry_with_backoff() {
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

### The Problem

**The principle appears contradictory:**
1. "Never use `tokio::time::sleep`" (lines 33, 38)
2. "Except for frame limiting and backoff" (lines 122-146)

**This confuses developers**: Is `tokio::time::sleep()` forbidden or not?

---

## Research: Industry Best Practices

### Tokio Official Guidance

**From Tokio docs** ([tokio::time::sleep](https://docs.rs/tokio/latest/tokio/time/fn.sleep.html), [tokio::time::interval](https://docs.rs/tokio/latest/tokio/time/fn.interval.html)):

1. **`sleep()` is non-blocking** - It yields the task, allowing other tasks to run
2. **`interval()` is preferred for recurring tasks** - Prevents timing drift
3. **Sleep is legitimate for:**
   - Rate limiting
   - Timeouts (with `tokio::time::timeout`)
   - Exponential backoff
   - Simulating delays (testing)

**Quote from Tokio docs:**
> "To run something regularly on a schedule, see `interval`."

**Key insight:** Sleep is not inherently bad. **Polling with sleep is bad.**

### Ratatui Async Templates

**From Ratatui async-template** ([async-template](https://github.com/ratatui/async-template)):

```rust
// Frame rate limiting pattern (RECOMMENDED)
let tick_interval = Duration::from_millis(1000 / tick_rate);
let frame_interval = Duration::from_millis(1000 / frame_rate);

loop {
    tokio::select! {
        _ = tick_timer.tick() => { /* update logic */ }
        _ = frame_timer.tick() => { /* render */ }
        event = event_stream.next() => { /* handle event */ }
    }
}
```

**Pattern used:** `tokio::time::Interval` (not `sleep` in loop)

**But also shows:**
```rust
// Acceptable for one-off delays
tokio::time::sleep(Duration::from_millis(50)).await;
```

### Real-World Rust Projects

**Common legitimate uses of `tokio::time::sleep`:**
1. **Exponential backoff** (retry logic)
2. **Rate limiting** (API throttling)
3. **Frame time balancing** (ensuring minimum frame duration)
4. **Graceful shutdown delays** (waiting for cleanup)
5. **Testing** (simulating latency, timeouts)

**Never used for:**
1. ❌ Polling loops (use `tokio::select!` + channels/streams)
2. ❌ Waiting for I/O (use `.await` on async I/O)
3. ❌ Periodic tasks (use `tokio::time::Interval`)

---

## Clarified Policy: The Three Types of Sleep

### 1. ALWAYS FORBIDDEN: `std::thread::sleep()` in Async Code

**Rule:** Never use `std::thread::sleep` in async functions or tokio runtime.

**Why:** Blocks the entire runtime thread, preventing other tasks from running.

```rust
// ❌ CRITICAL BUG - Blocks runtime
async fn bad() {
    std::thread::sleep(Duration::from_millis(10)); // BLOCKS ALL TASKS!
}
```

**Exception:** Only acceptable in **non-async** test code outside tokio runtime.

**Enforcement:**
- Linter: `#![deny(std::thread::sleep)]` in async modules
- CI: Grep audit fails if found in `src/` (excluding test files)

---

### 2. CONDITIONAL: `tokio::time::sleep()` - Case-by-Case Basis

**Rule:** `tokio::time::sleep()` is acceptable ONLY for specific timing control patterns.

#### FORBIDDEN Patterns (Anti-patterns)

```rust
// ❌ FORBIDDEN: Polling loop
loop {
    if check_condition() {
        do_work();
    }
    tokio::time::sleep(Duration::from_millis(10)).await; // BAD!
}

// ❌ FORBIDDEN: Waiting for state change
while !ready {
    tokio::time::sleep(Duration::from_millis(1)).await; // BAD!
}

// ❌ FORBIDDEN: Periodic tasks with sleep
loop {
    do_periodic_work();
    tokio::time::sleep(Duration::from_secs(1)).await; // BAD! Use Interval
}
```

**Why forbidden:**
- Wastes CPU cycles checking conditions that won't change
- Adds unnecessary latency (sleep duration is minimum wait)
- Prevents proper async coordination
- No backpressure handling

**Fix:** Use event-driven patterns (channels, `tokio::select!`, `Notify`, `Interval`)

#### ACCEPTABLE Patterns (Legitimate Uses)

##### A. Exponential Backoff (Retry Logic)

```rust
// ✅ ACCEPTABLE - Controlled retry delays
async fn retry_with_backoff<F, Fut, T>(mut operation: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut backoff = Duration::from_millis(100);
    let max_backoff = Duration::from_secs(30);

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if is_retryable(&e) => {
                tokio::time::sleep(backoff).await; // OK - intentional delay
                backoff = (backoff * 2).min(max_backoff);
            }
            Err(e) => return Err(e),
        }
    }
}
```

**Why acceptable:**
- Sleep is intentional (backoff strategy, not polling)
- Prevents overwhelming failing service
- Industry-standard retry pattern

**Our usage:** `yollayah/conductor/core/src/routing/router.rs:517`

##### B. Frame Rate Limiting (Timing Control)

```rust
// ✅ ACCEPTABLE - Frame time balancing
async fn run_event_loop() {
    let frame_duration = Duration::from_millis(50); // 20 FPS

    loop {
        let frame_start = Instant::now();

        // Event handling, updates, rendering
        handle_events().await;
        update_state();
        render();

        // Ensure minimum frame time (prevents busy-waiting)
        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            tokio::time::sleep(frame_duration - elapsed).await; // OK - frame limiting
        }
    }
}
```

**Why acceptable:**
- Prevents busy-waiting when work completes early
- Ensures consistent frame timing
- Common in game loops and TUI rendering

**Our usage:** `yollayah/core/surfaces/tui/src/app.rs:376`

**BETTER ALTERNATIVE:** Use `tokio::time::Interval` (see Section 3 below)

##### C. Rate Limiting (API Throttling)

```rust
// ✅ ACCEPTABLE - Rate limiting API calls
async fn rate_limited_request(client: &Client) -> Result<Response> {
    const RATE_LIMIT_DELAY: Duration = Duration::from_millis(100);

    tokio::time::sleep(RATE_LIMIT_DELAY).await; // OK - throttling
    client.request().await
}
```

**Why acceptable:**
- Intentional delay to respect rate limits
- Prevents overwhelming external services
- Industry-standard throttling pattern

##### D. Graceful Shutdown Delays

```rust
// ✅ ACCEPTABLE - Waiting for cleanup
async fn shutdown(server: Server) {
    server.stop().await;

    // Give clients time to disconnect gracefully
    tokio::time::sleep(Duration::from_secs(1)).await; // OK - cleanup delay

    server.close_all_connections().await;
}
```

**Why acceptable:**
- Intentional delay for graceful cleanup
- Not a polling pattern
- Common in shutdown sequences

##### E. Testing (Simulated Delays)

```rust
// ✅ ACCEPTABLE - Testing only
#[tokio::test]
async fn test_timeout_behavior() {
    tokio::time::sleep(Duration::from_millis(100)).await; // OK - test simulation
    assert!(check_timeout_expired());
}
```

**Why acceptable:**
- Test code, not production
- Simulating real-world delays
- Tokio provides `tokio::time::pause()` for better control

---

### 3. PREFERRED: `tokio::time::Interval` - Event-Driven Timers

**Rule:** For recurring tasks, ALWAYS prefer `Interval` over `sleep` in loop.

#### Why Interval is Better

**Key difference** ([Tokio forum discussion](https://users.rust-lang.org/t/tokio-sleep-vs-interval/72385)):

```rust
// ❌ INFERIOR: sleep in loop (causes drift)
loop {
    let start = Instant::now();
    do_work(); // Takes variable time
    tokio::time::sleep(Duration::from_millis(100)).await; // Adds 100ms AFTER work
    // Actual interval: 100ms + work_time (drifts!)
}

// ✅ SUPERIOR: Interval (no drift)
let mut interval = tokio::time::interval(Duration::from_millis(100));
loop {
    interval.tick().await; // Accounts for time spent in previous iteration
    do_work(); // Variable time
    // Actual interval: ~100ms (drift-corrected!)
}
```

**Interval advantages:**
1. **Drift correction** - Accounts for time spent processing
2. **Missed tick behavior** - Configurable (`Skip`, `Burst`, `Delay`)
3. **More efficient** - No manual time tracking
4. **Better semantics** - "Every N ms" vs "Wait N ms"

#### Recommended Patterns

##### A. Frame Rate Control (TUI Rendering)

```rust
// ✅ BEST PRACTICE - Interval for frame timing
pub async fn run(&mut self) -> Result<()> {
    let mut render_interval = tokio::time::interval(Duration::from_millis(50)); // 20 FPS
    render_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut animation_interval = tokio::time::interval(Duration::from_millis(100)); // 10 FPS

    loop {
        tokio::select! {
            _ = render_interval.tick() => {
                if has_dirty_state() {
                    render()?;
                }
            }
            _ = animation_interval.tick() => {
                update_animations();
            }
            event = event_stream.next() => {
                handle_event(event).await;
            }
        }
    }
}
```

**Why this is optimal:**
- Separate intervals for different update rates
- No drift accumulation
- Configurable missed tick behavior
- Clean event-driven architecture

**Source:** [Ratatui async-template](https://ratatui.github.io/async-template/)

##### B. Periodic Cleanup Tasks

```rust
// ✅ BEST PRACTICE - Interval for periodic work
async fn cleanup_worker(cache: Arc<Cache>) {
    let mut cleanup_interval = tokio::time::interval(Duration::from_secs(60));

    loop {
        cleanup_interval.tick().await;
        cache.evict_expired().await;
    }
}
```

**Why better than sleep:**
- Runs every 60 seconds regardless of `evict_expired()` duration
- No drift over time
- More readable intent

##### C. Heartbeat / Keepalive

```rust
// ✅ BEST PRACTICE - Interval for heartbeats
async fn heartbeat_sender(connection: Connection) {
    let mut heartbeat = tokio::time::interval(Duration::from_secs(30));

    loop {
        heartbeat.tick().await;
        if let Err(e) = connection.send_ping().await {
            tracing::warn!("Heartbeat failed: {}", e);
            break;
        }
    }
}
```

**Why better than sleep:**
- Maintains consistent heartbeat interval
- Accounts for variable `send_ping()` latency
- Industry-standard keepalive pattern

---

## Updated Principle Statement

### Original (PRINCIPLE-efficiency.md lines 19-59)

> **Law 1: No Sleep, Only Wait on Async I/O**
>
> **FORBIDDEN:**
> ```rust
> tokio::time::sleep(Duration::from_millis(10)).await; // ❌ TERRIBLE
> ```

**Problem:** Too absolute, contradicts later exceptions.

### Proposed Refinement

> **Law 1: No Sleep, Only Wait on Async I/O**
>
> **Three Categories:**
>
> 1. **ALWAYS FORBIDDEN:** `std::thread::sleep()` in async code
>    - Blocks runtime thread
>    - Zero tolerance policy
>
> 2. **CONTEXT-DEPENDENT:** `tokio::time::sleep()`
>    - ❌ **FORBIDDEN:** Polling loops, waiting for state changes
>    - ✅ **ACCEPTABLE:** Exponential backoff, frame time balancing, rate limiting
>    - Must have comment explaining why
>
> 3. **PREFERRED:** `tokio::time::Interval`
>    - ✅ **REQUIRED:** For all recurring tasks (frame ticks, periodic cleanup, heartbeats)
>    - Prevents drift, better semantics

---

## Enforcement Strategy

### 1. Linter Rules

```toml
# .cargo/config.toml or clippy.toml
[lints.rust]
# Deny blocking sleep in async code
forbidden-lint-groups = ["std::thread::sleep"]

[lints.clippy]
# Warn on tokio sleep (requires justification)
tokio_sleep = "warn"
```

### 2. CI/CD Audit

```bash
# Fail if std::thread::sleep in production async code
if rg "std::thread::sleep" --type rust --glob '!tests/**' yollayah/*/src; then
    echo "ERROR: Found std::thread::sleep in production code"
    exit 1
fi

# Warn if tokio::time::sleep without comment
rg "tokio::time::sleep" --type rust --glob '!tests/**' yollayah/*/src \
    --context 1 \
    | grep -v "// OK" \
    | grep -v "// ACCEPTABLE" \
    && echo "WARNING: tokio::time::sleep without justification comment"
```

### 3. Code Review Checklist

When reviewing code with `tokio::time::sleep`:

- [ ] Is this in a polling loop? → ❌ **REJECT** (use event-driven)
- [ ] Is this for periodic tasks? → ❌ **REJECT** (use `Interval`)
- [ ] Is this for backoff/retry? → ✅ **ACCEPTABLE** (add comment)
- [ ] Is this for frame limiting? → ⚠️ **SUGGEST** `Interval` instead
- [ ] Is this for rate limiting? → ✅ **ACCEPTABLE** (add comment)
- [ ] Does it have a comment explaining why? → Required for acceptance

---

## Migration Guide: Sleep → Interval

### Pattern 1: Periodic Task

**BEFORE:**
```rust
// ❌ Anti-pattern
loop {
    do_periodic_work();
    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

**AFTER:**
```rust
// ✅ Event-driven
let mut interval = tokio::time::interval(Duration::from_secs(1));
loop {
    interval.tick().await;
    do_periodic_work();
}
```

### Pattern 2: Frame Rate Limiting

**BEFORE:**
```rust
// ⚠️ Works but not optimal
loop {
    let start = Instant::now();
    render();
    let elapsed = start.elapsed();
    if elapsed < frame_duration {
        tokio::time::sleep(frame_duration - elapsed).await;
    }
}
```

**AFTER:**
```rust
// ✅ Optimal - no drift, cleaner
let mut render_interval = tokio::time::interval(frame_duration);
render_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

loop {
    render_interval.tick().await;
    render();
}
```

### Pattern 3: Multiple Timers

**BEFORE:**
```rust
// ❌ Complex manual timing
let mut last_render = Instant::now();
let mut last_update = Instant::now();

loop {
    let now = Instant::now();

    if now.duration_since(last_render) >= render_interval {
        render();
        last_render = now;
    }

    if now.duration_since(last_update) >= update_interval {
        update();
        last_update = now;
    }

    tokio::time::sleep(Duration::from_millis(1)).await; // Polling!
}
```

**AFTER:**
```rust
// ✅ Event-driven with separate intervals
let mut render_interval = tokio::time::interval(render_duration);
let mut update_interval = tokio::time::interval(update_duration);

loop {
    tokio::select! {
        _ = render_interval.tick() => render(),
        _ = update_interval.tick() => update(),
    }
}
```

---

## Real-World Examples from Our Codebase

### ✅ GOOD: Exponential Backoff (router.rs:517)

```rust
// yollayah/conductor/core/src/routing/router.rs
async fn execute_with_retry(&self, request: &RoutingRequest) -> Result<...> {
    for attempt in 0..max_retries {
        match self.execute_request(request).await {
            Ok(response) => return Ok(response),
            Err(e) if is_retryable(&e) => {
                let backoff = retry_config.backoff_for_attempt(attempt);
                tokio::time::sleep(backoff).await; // ✅ OK - exponential backoff
            }
            Err(e) => return Err(e),
        }
    }
}
```

**Why acceptable:** Intentional delay for retry strategy, not polling.

### ✅ GOOD: Frame Time Balancing (app.rs:376)

```rust
// yollayah/core/surfaces/tui/src/app.rs
let elapsed = frame_start.elapsed();
if elapsed < frame_duration {
    tokio::time::sleep(frame_duration - elapsed).await; // ✅ OK - frame limiting
}
```

**Why acceptable:** Prevents busy-waiting, ensures minimum frame time.

**Improvement opportunity:** Use `Interval` instead (see migration guide).

### ❌ BAD: Test Polling Loop (FIXED IN DONE-BUG-015)

```rust
// BEFORE (from DONE-BUG-015-sleep-in-polling-loops.md)
loop {
    if check_condition() {
        break;
    }
    tokio::time::sleep(Duration::from_millis(10)).await; // ❌ Polling!
}

// AFTER (fixed)
tokio::select! {
    _ = condition_changed.notified() => { /* ... */ }
    _ = timeout => { /* ... */ }
}
```

**Why bad:** Polling with arbitrary sleep duration, wastes CPU.

---

## Testing Considerations

### Test Code: More Lenient

**In tests, `tokio::time::sleep` is more acceptable:**

```rust
#[tokio::test]
async fn test_timeout() {
    tokio::time::sleep(Duration::from_millis(100)).await; // ✅ OK in tests
    assert!(check_state());
}
```

**Why:** Simulating real-world delays, not production efficiency concern.

**BETTER:** Use `tokio::time::pause()` for instant time control:

```rust
#[tokio::test]
async fn test_timeout_fast() {
    tokio::time::pause(); // Freeze time

    let task = tokio::spawn(async {
        tokio::time::sleep(Duration::from_secs(3600)).await; // "Sleeps" instantly
    });

    tokio::time::advance(Duration::from_secs(3600)).await;
    task.await.unwrap();
}
```

**Source:** [Tokio testing docs](https://tokio.rs/tokio/topics/testing)

---

## Summary: Decision Matrix

| Use Case | `std::thread::sleep` | `tokio::time::sleep` | `tokio::time::Interval` | Recommendation |
|----------|---------------------|---------------------|------------------------|----------------|
| **Polling loop** | ❌ NEVER | ❌ NEVER | ✅ Use events instead | Event-driven (channels, `Notify`) |
| **Periodic tasks** | ❌ NEVER | ❌ NO (causes drift) | ✅ YES | **Interval** |
| **Frame rate limiting** | ❌ NEVER | ⚠️ OK (not optimal) | ✅ YES | **Interval** (preferred) |
| **Exponential backoff** | ❌ NEVER | ✅ YES | ❌ N/A | **sleep** (acceptable) |
| **Rate limiting** | ❌ NEVER | ✅ YES | ⚠️ Depends | **sleep** or rate limiter crate |
| **Graceful shutdown** | ❌ NEVER | ✅ YES | ❌ N/A | **sleep** (acceptable) |
| **Testing delays** | ❌ NEVER | ⚠️ OK | ❌ N/A | `tokio::time::pause()` (better) |
| **Waiting for I/O** | ❌ NEVER | ❌ NEVER | ❌ NEVER | **async I/O** (`.await`) |
| **Waiting for state** | ❌ NEVER | ❌ NEVER | ❌ NEVER | **event** (`Notify`, channel) |

---

## Proposed Changes to PRINCIPLE-efficiency.md

### Section to Update: Lines 19-146

**Current text (contradictory):**
```markdown
### Law 1: No Sleep, Only Wait on Async I/O

**FORBIDDEN**:
tokio::time::sleep(Duration::from_millis(10)).await; // ❌ TERRIBLE

**Exception - Frame Rate Limiting (TUI ONLY)**:
tokio::time::sleep(frame_duration - elapsed).await; // OK for frame limiting
```

**Proposed replacement:**

```markdown
### Law 1: No Sleep, Only Wait on Async I/O

**Three Categories (in order of preference):**

#### 1. ALWAYS FORBIDDEN: Blocking Sleep

**Rule:** Never use `std::thread::sleep()` in async code.

```rust
// ❌ CRITICAL BUG - Blocks runtime
async fn bad() {
    std::thread::sleep(Duration::from_millis(10)); // BLOCKS ALL TASKS!
}
```

**Enforcement:** CI fails if found in `src/` (excluding sync-only test code).

---

#### 2. PREFERRED: Event-Driven Timers (tokio::time::Interval)

**Rule:** For recurring tasks, ALWAYS use `Interval` instead of sleep in loop.

```rust
// ✅ BEST PRACTICE - Interval for frame timing
let mut render_interval = tokio::time::interval(Duration::from_millis(50)); // 20 FPS
render_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

loop {
    render_interval.tick().await;
    render();
}
```

**Why better:**
- No drift accumulation (accounts for processing time)
- Configurable missed tick behavior
- More efficient than manual time tracking
- Industry-standard pattern (Tokio, Ratatui templates)

**Use cases:**
- Frame rate control (TUI rendering)
- Periodic cleanup tasks
- Heartbeats / keepalives
- Any recurring scheduled work

**Source:** [Tokio interval docs](https://docs.rs/tokio/latest/tokio/time/fn.interval.html), [Ratatui async-template](https://ratatui.github.io/async-template/)

---

#### 3. CONDITIONAL: Async Sleep (tokio::time::sleep)

**Rule:** `tokio::time::sleep()` is acceptable ONLY for specific timing control patterns.

##### ❌ FORBIDDEN Patterns (Anti-patterns)

```rust
// ❌ FORBIDDEN: Polling loop
loop {
    if check_condition() {
        do_work();
    }
    tokio::time::sleep(Duration::from_millis(10)).await; // BAD!
}

// ❌ FORBIDDEN: Waiting for state change
while !ready {
    tokio::time::sleep(Duration::from_millis(1)).await; // BAD!
}

// ❌ FORBIDDEN: Periodic tasks
loop {
    do_work();
    tokio::time::sleep(Duration::from_secs(1)).await; // BAD! Use Interval
}
```

**Fix:** Use event-driven patterns (channels, `tokio::select!`, `Notify`, `Interval`).

##### ✅ ACCEPTABLE Patterns (Legitimate Uses)

**A. Exponential Backoff (Retry Logic)**

```rust
// ✅ ACCEPTABLE - Controlled retry delays
async fn retry_with_backoff() {
    for attempt in 0..max_retries {
        match try_operation().await {
            Ok(result) => return Ok(result),
            Err(_) => {
                let backoff = calculate_backoff(attempt);
                tokio::time::sleep(backoff).await; // OK - intentional delay
            }
        }
    }
}
```

**Why acceptable:** Intentional delay for retry strategy, not polling.

**B. Rate Limiting (API Throttling)**

```rust
// ✅ ACCEPTABLE - Rate limiting API calls
async fn rate_limited_request() -> Result<Response> {
    tokio::time::sleep(RATE_LIMIT_DELAY).await; // OK - throttling
    client.request().await
}
```

**Why acceptable:** Intentional delay to respect rate limits.

**C. Graceful Shutdown Delays**

```rust
// ✅ ACCEPTABLE - Waiting for cleanup
async fn shutdown(server: Server) {
    server.stop().await;
    tokio::time::sleep(Duration::from_secs(1)).await; // OK - cleanup delay
    server.close_all_connections().await;
}
```

**Why acceptable:** Intentional delay for graceful cleanup.

**D. Frame Time Balancing (TUI - Use Interval Instead)**

```rust
// ⚠️ ACCEPTABLE but not optimal - Use Interval instead
let elapsed = frame_start.elapsed();
if elapsed < frame_duration {
    tokio::time::sleep(frame_duration - elapsed).await; // OK but Interval is better
}
```

**Why acceptable:** Prevents busy-waiting, ensures minimum frame time.

**Why not optimal:** `Interval` prevents drift, has better semantics.

---

**Enforcement for tokio::time::sleep:**
- All uses MUST have comment explaining why (e.g., `// OK - exponential backoff`)
- Code review MUST verify it's not a polling pattern
- CI warns if found without justification comment
```

---

## Conclusion

### Key Findings

1. **The principle "No Sleep, Only Wait on Async I/O" is fundamentally CORRECT**
   - The spirit is right: avoid polling, use event-driven patterns
   - The implementation needs refinement to distinguish anti-patterns from legitimate uses

2. **Three categories, not a binary rule:**
   - **ALWAYS FORBIDDEN:** `std::thread::sleep()` in async code
   - **PREFERRED:** `tokio::time::Interval` for recurring tasks
   - **CONDITIONAL:** `tokio::time::sleep()` for specific timing control

3. **Industry best practices support this refinement:**
   - Tokio docs recommend `sleep()` for rate limiting, backoff, timeouts
   - Ratatui templates use `Interval` for frame timing (not sleep in loop)
   - Real-world Rust projects use sleep for backoff, not polling

4. **Our codebase is mostly compliant:**
   - ✅ No `std::thread::sleep` in production async code (only in sync tests)
   - ✅ `tokio::time::sleep` used correctly for backoff (router.rs)
   - ⚠️ Frame limiting with sleep could be improved to use `Interval`

### Recommendations

1. **Update PRINCIPLE-efficiency.md** with refined language (see proposed changes above)
2. **Add this document** as supplementary guidance
3. **Migrate TUI frame limiting** from sleep to Interval (optional improvement)
4. **Enforce in CI:** Warn on tokio::time::sleep without justification comment
5. **Code review checklist:** Verify sleep usage against decision matrix

### No Violations Found

**Our production code is sound:**
- Backoff in router.rs: ✅ Legitimate use
- Frame limiting in TUI: ✅ Acceptable (could be improved)
- Test code sleeps: ✅ Acceptable in tests

**The principle doesn't need fixing. The documentation does.**

---

## Sources

**Tokio Official:**
- [tokio::time::sleep](https://docs.rs/tokio/latest/tokio/time/fn.sleep.html)
- [tokio::time::interval](https://docs.rs/tokio/latest/tokio/time/fn.interval.html)
- [Tokio async in depth](https://tokio.rs/tokio/tutorial/async)
- [Tokio testing](https://tokio.rs/tokio/topics/testing)
- [Bridging with sync code](https://tokio.rs/tokio/topics/bridging)

**Ratatui Community:**
- [Ratatui FAQ](https://ratatui.rs/faq/)
- [Full Async Events tutorial](https://ratatui.rs/tutorials/counter-async-app/full-async-events/)
- [async-template repository](https://github.com/ratatui/async-template)
- [Ratatui rendering concepts](https://ratatui.rs/concepts/rendering/)

**Community Discussions:**
- [Tokio sleep() vs interval() - Rust forum](https://users.rust-lang.org/t/tokio-sleep-vs-interval/72385)
- [Understanding tokio spawn and spawn_blocking](https://guillaume.vanderest.org/posts/rust-tokio-sleep/)

**Our Codebase:**
- `knowledge/principles/PRINCIPLE-efficiency.md` (current principle)
- `facts/REACTIVE-RENDERING-PATTERNS-PROPOSAL.md` (frame rate limiting research)
- `facts/TUI-RENDER-BLOCKING-ANALYSIS-PROPOSAL.md` (blocking operations analysis)
- `progress/DONE-BUG-015-sleep-in-polling-loops.md` (previous sleep audit)

---

**Document Version:** 1.0
**Last Updated:** 2026-01-04
**Status:** PROPOSAL - Ready for team review and principle refinement
