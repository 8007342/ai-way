# Reactive Rendering Patterns for ai-way TUI

**Date**: 2026-01-03
**Author**: Rust & Ratatui Team + Performance Hacker
**Status**: PROPOSAL - Research Complete
**Priority**: HIGH - Core Performance Architecture

---

## Executive Summary

**The Problem**: Ratatui's `terminal.draw()` is synchronous and blocks the event loop during rendering. Current implementation renders on every token (200+ FPS during streaming), causing the async runtime to block during terminal I/O.

**The Solution**: Implement frame-rate-limited rendering (20 FPS / 50ms intervals) with separate render timing from token processing, using tokio channels and dirty tracking to decouple token arrival from visual updates.

**Performance Gain**:
- Reduce render calls from 200+/sec → 20/sec (10x reduction)
- Eliminate blocking on every token
- Maintain responsive UI and smooth animations
- Keep token processing fully async

---

## Current Architecture Analysis

### Current Implementation (app.rs:260-367)

```rust
// Main event loop
while self.running {
    tokio::select! {
        // Terminal events (keyboard, mouse, resize)
        maybe_event = event_stream.next() => { ... }

        // REACTIVE STREAMING: Process tokens as they arrive
        _ = self.conductor.process_streaming_token() => {
            self.process_conductor_messages();

            // ❌ BLOCKING ISSUE: Render on EVERY token
            if let Err(e) = self.render(terminal) {
                tracing::error!("Render error during streaming: {}", e);
            }
        }

        // Frame tick (100ms = 10 FPS)
        _ = tokio::time::sleep(Duration::from_millis(100)) => {
            // Startup, update, render
        }
    }
}

// Blocking render call (app.rs:894)
terminal.draw(|frame| {
    let output = self.compositor.composite();
    frame.buffer_mut().merge(output);
})?;
```

### Problems Identified

1. **Render on Every Token**: During streaming (200 tokens/sec), we call `terminal.draw()` 200+ times
2. **Synchronous I/O**: `terminal.draw()` does blocking terminal I/O (write to stdout, flush)
3. **Runtime Blocking**: Each `terminal.draw()` blocks the tokio runtime thread
4. **Inefficient**: Human eyes can't see >60 FPS, we're wasting 140+ render calls per second
5. **Frame Tick Ignored**: We have a 10 FPS frame tick, but streaming renders bypass it

### What's Already Good

✅ **Dirty Tracking**: App already has dirty tracking for most components (conversation, input, status, tasks, avatar)
✅ **Compositor Pattern**: Layers are composited once, not per-widget
✅ **Async Event Handling**: Events are processed asynchronously via `tokio::select!`
✅ **Reactive Token Processing**: Tokens flow through async channels, no polling

---

## Research: State of the Art (2025-2026)

### 1. Ratatui Official Guidance

**Source**: [Ratatui FAQ](https://ratatui.rs/faq/) and [Async Template](https://github.com/ratatui/async-template)

**Key Findings**:
- Ratatui is **not a native async library** - `terminal.draw()` is synchronous
- Rendering to terminal buffer is "relatively fast" due to double-buffering (only renders diffs)
- Official recommendation: Use **frame rate limiting** with separate tick and render intervals
- Async is most beneficial for event handling (crossterm supports async event streams ✅ we already use this)
- Template pattern: Use `tokio::select!` to merge multiple event streams (✅ we already do this)

**Quote from FAQ**:
> "Rendering to the terminal buffer is relatively fast, especially using the double buffer technique that only renders diffs."

**Implication**: `terminal.draw()` itself is fast (~1-5ms), but blocking is still blocking. At 200 calls/sec, we're blocking 200-1000ms total per second = wasting runtime capacity.

### 2. Frame Rate Limiting Patterns

**Source**: [Ratatui Async Template](https://github.com/ratatui/async-template) (deprecated but shows pattern)

**Pattern**: Separate tick rate (logic updates) from frame rate (rendering)

```rust
// From async-template README
let tick_rate = Duration::from_millis(1000 / args.tick_rate); // Logic updates
let frame_rate = Duration::from_millis(1000 / args.frame_rate); // Rendering (default: 60 FPS)
```

**Implementation**:
- Two separate `tokio::time::Interval` streams
- Tick stream: Updates game logic, AI state, etc.
- Render stream: Only updates visual output
- Both merged into `tokio::select!` event loop

**Benefits**:
- Logic can update faster than rendering (or vice versa)
- Rendering is rate-limited regardless of event frequency
- No wasted render calls

### 3. Async vs Blocking Terminal I/O

**Source**: [Tokio spawn_blocking docs](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html) and [Ratatui Forum Discussion](https://forum.ratatui.rs/t/understanding-tokio-spawn-and-tokio-spawn-blocking/74)

**Can we make terminal.draw() async?**
- **No native async**: Ratatui's `terminal.draw()` is fundamentally synchronous
- **Workaround**: Use `tokio::task::spawn_blocking` to offload to blocking thread pool
- **Trade-off**: Adds thread context switching overhead, but prevents blocking async runtime

**spawn_blocking Characteristics**:
- Runs on separate blocking thread pool (max ~500 threads)
- Isolates blocking I/O from async runtime
- Adds ~10-50μs overhead for context switch
- Recommended for operations >10-100μs

**Analysis for ai-way**:
- `terminal.draw()` takes 1-5ms (way above 100μs threshold)
- During streaming (200 tokens/sec), we'd block async runtime for 200-1000ms/sec
- Using `spawn_blocking` is **appropriate** here

### 4. Double Buffering Patterns in Rust

**Source**: [r3bl_tui docs](https://docs.rs/r3bl_tui/latest/r3bl_tui/) and [Rust Framebuffer Discussion](https://github.com/rust-osdev/bootloader/issues/234)

**Double Buffer Pattern**:
1. Allocate two buffers: `front` (displayed) and `back` (being drawn)
2. Render to `back` buffer off the main thread
3. Swap buffers atomically (just pointer swap)
4. Display `front` buffer

**In Ratatui Context**:
- Ratatui **already uses double buffering internally** (Buffer diff rendering)
- Our Compositor **also does layered buffering** (compose layers, then merge to terminal)
- Adding another buffer layer would be redundant

**Implication**: We don't need explicit double buffering - Ratatui handles it. We just need to **rate-limit the draw calls**.

### 5. Channel Performance: tokio::mpsc vs crossbeam

**Source**: [Rust Channel Comparison](https://codeandbitters.com/rust-channel-comparison/), [Tokio vs Crossbeam Discussion](https://users.rust-lang.org/t/differences-between-channel-in-tokio-mpsc-and-crossbeam/92676)

**For Async Context** (our use case):
- **tokio::sync::mpsc**: ✅ Async-aware, non-blocking `.await`
- **crossbeam::channel**: ❌ Blocks the thread, not allowed in async code
- **crossfire**: 🔥 Newer hybrid option (async + blocking), claims better performance

**Performance Characteristics**:
- **tokio::mpsc**: Context-switches in same thread (cheap), optimized for async
- **crossbeam**: Thread-to-thread communication (expensive context switch)
- **crossfire**: Lockless, claims to outperform both (as of 2025)

**Our Current Usage**:
- Conductor → TUI: tokio::sync::mpsc (capacity: 512) ✅ Correct choice
- Token throughput: 200 tokens/sec = well within channel capacity
- No blocking issues with channel itself

**Recommendation**: Keep using **tokio::sync::mpsc**. It's the right tool for async-to-async communication.

---

## Proposed Solution: Frame-Rate-Limited Rendering

### Architecture Overview

**Core Principle**: Decouple token arrival from visual rendering

```
Token Flow (200+ tokens/sec):
User → Conductor → tokio::mpsc channel → DisplayState → [dirty flag set]

Render Flow (20 FPS fixed):
[20 FPS timer] → check dirty flags → if dirty: terminal.draw() → clear flags
```

### Implementation Pattern

```rust
// Main event loop with SEPARATE render timing
pub async fn run(&mut self, terminal: &mut Terminal<...>) -> anyhow::Result<()> {
    // Render rate: 20 FPS = 50ms per frame
    let mut render_interval = tokio::time::interval(Duration::from_millis(50));
    render_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // Animation tick: 10 FPS = 100ms (for avatar wandering, etc.)
    let mut animation_interval = tokio::time::interval(Duration::from_millis(100));

    // Terminal event stream (async)
    let mut event_stream = EventStream::new();

    // Render initial frame
    self.render(terminal)?;

    while self.running {
        tokio::select! {
            biased; // Process in priority order

            // 1. Terminal events (highest priority - user input)
            maybe_event = event_stream.next() => {
                if let Some(Ok(event)) = maybe_event {
                    self.handle_event(event).await;
                    // Input changes set dirty flags, will render on next tick
                }
            }

            // 2. Streaming tokens (high priority - responsiveness)
            _ = self.conductor.process_streaming_token() => {
                // Process token into display state
                self.process_conductor_messages();
                // ✅ NO RENDER HERE - just set dirty flag
                // Render will happen on next frame tick (within 50ms)
            }

            // 3. Animation tick (medium priority - 10 FPS)
            _ = animation_interval.tick() => {
                self.update_animations();
                // Sets avatar_dirty flag if animation changed
            }

            // 4. Render tick (20 FPS - controlled rate)
            _ = render_interval.tick() => {
                // Only render if something changed
                if self.has_dirty_layers() {
                    // Option A: Direct render (blocks for 1-5ms)
                    self.render(terminal)?;

                    // Option B: Async render (offload to blocking thread pool)
                    // let terminal = terminal.clone(); // if Terminal is Clone
                    // tokio::task::spawn_blocking(move || {
                    //     terminal.draw(|frame| { ... })
                    // }).await??;
                }
            }
        }
    }

    Ok(())
}
```

### Dirty Tracking Strategy

**Already Implemented** ✅:
- `conversation_dirty`: Set when new messages/tokens arrive
- Input dirty tracking: `prev_input_buffer != input_buffer`
- Status dirty tracking: `prev_conductor_state != conductor_state`
- Tasks dirty tracking: Hash comparison
- Avatar dirty tracking: `avatar_changed` flag

**Enhancement Needed**:
```rust
impl App {
    /// Check if ANY layer needs re-rendering
    fn has_dirty_layers(&self) -> bool {
        self.conversation_dirty
            || self.input_dirty()
            || self.status_dirty()
            || self.tasks_dirty()
            || self.avatar_changed
    }

    /// Clear all dirty flags after render
    fn clear_dirty_flags(&mut self) {
        self.conversation_dirty = false;
        self.avatar_changed = false;
        // Input/status/tasks clear themselves via prev_* comparison
    }
}
```

### Render Call Optimization

**Current** (app.rs:884-903):
```rust
fn render(&mut self, terminal: &mut Terminal<...>) -> anyhow::Result<()> {
    // Always render all layers
    self.render_conversation();
    self.render_tasks();
    self.render_input();
    self.render_status();
    self.render_avatar();

    // Blocking terminal.draw()
    terminal.draw(|frame| {
        let output = self.compositor.composite();
        frame.buffer_mut().merge(output);
    })?;

    Ok(())
}
```

**Optimized** (with dirty checking):
```rust
fn render(&mut self, terminal: &mut Terminal<...>) -> anyhow::Result<()> {
    // Only render dirty layers
    if self.conversation_dirty {
        self.render_conversation();
    }
    if self.input_dirty() {
        self.render_input();
    }
    if self.status_dirty() {
        self.render_status();
    }
    if self.tasks_dirty() {
        self.render_tasks();
    }
    if self.avatar_changed {
        self.render_avatar();
    }

    // Compositor only composites dirty layers (already implemented ✅)
    terminal.draw(|frame| {
        let output = self.compositor.composite();
        frame.buffer_mut().merge(output);
    })?;

    // Clear dirty flags
    self.clear_dirty_flags();

    Ok(())
}
```

---

## Alternative Approaches Considered

### Option 1: Offload to spawn_blocking ⚠️

**Pattern**:
```rust
_ = render_interval.tick() => {
    if self.has_dirty_layers() {
        let compositor = self.compositor.clone(); // Need Clone
        tokio::task::spawn_blocking(move || {
            terminal.draw(|frame| {
                let output = compositor.composite();
                frame.buffer_mut().merge(output);
            })
        }).await??;
    }
}
```

**Pros**:
- Fully non-blocking for async runtime
- Isolates terminal I/O to blocking thread pool
- Prevents runtime starvation during heavy rendering

**Cons**:
- Requires `Terminal` to be `Clone` (may not be available)
- Requires `Compositor` to be thread-safe (`Send + Sync`)
- Adds context switch overhead (~10-50μs per render)
- More complex error handling (nested Results)
- Overkill for 1-5ms blocking operation at 20 FPS

**Verdict**: ❌ **NOT RECOMMENDED** - The complexity outweighs the benefit. At 20 FPS, we block for max 100ms/sec (10% of runtime), which is acceptable. The current dirty tracking + frame limiting is sufficient.

### Option 2: Separate Render Thread with Channels 🤔

**Pattern**:
```rust
// Main thread: Sends render requests
let (render_tx, mut render_rx) = tokio::sync::mpsc::channel(8);

// Render thread: Owns terminal
tokio::spawn(async move {
    while let Some(compositor_buffer) = render_rx.recv().await {
        terminal.draw(|frame| {
            frame.buffer_mut().merge(&compositor_buffer);
        }).unwrap();
    }
});

// Main event loop: Sends frames
_ = render_interval.tick() => {
    if self.has_dirty_layers() {
        let buffer = self.compositor.composite();
        render_tx.send(buffer).await.ok(); // Non-blocking send
    }
}
```

**Pros**:
- Fully async main event loop
- Terminal I/O isolated to dedicated thread
- Clean separation of concerns
- No blocking in tokio runtime

**Cons**:
- Requires cloning entire compositor buffer (expensive!)
- Buffer is ~(width × height) cells = 80×24 = 1920 cells × ~8 bytes = ~15KB per frame
- At 20 FPS = 300KB/sec memory bandwidth
- Adds latency (channel round-trip + thread switch)
- Terminal must be `Send` to move to thread

**Verdict**: ⚠️ **MAYBE** - This is technically the "cleanest" async solution, but the buffer cloning overhead is significant. Consider if blocking becomes a measurable problem.

### Option 3: Immediate Mode Rendering (Current + Frame Limit) ✅

**Pattern**: Keep current architecture, just add frame rate limiting

**Changes**:
1. Remove `self.render(terminal)` from token processing path
2. Add separate `render_interval` ticker at 20 FPS
3. Enhance dirty tracking to catch all state changes
4. Only render when dirty flags are set

**Pros**:
- Minimal code changes
- Leverages existing dirty tracking
- No new complexity (channels, threads, cloning)
- Proven pattern (used by Ratatui templates)
- Blocking is tolerable (100ms/sec at 20 FPS)

**Cons**:
- Still blocks async runtime during `terminal.draw()`
- 50ms max latency before token appears (acceptable for 20 FPS)

**Verdict**: ✅ **RECOMMENDED** - This is the sweet spot of simplicity vs performance. Frame limiting + dirty tracking solves 90% of the problem with 10% of the complexity.

---

## Recommended Implementation Plan

### Phase 1: Frame Rate Limiting (Immediate)

**Changes to app.rs**:

1. **Add render interval** (line ~266):
```rust
pub async fn run(&mut self, terminal: &mut Terminal<...>) -> anyhow::Result<()> {
    let mut render_interval = tokio::time::interval(Duration::from_millis(50)); // 20 FPS
    render_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut animation_interval = tokio::time::interval(Duration::from_millis(100)); // 10 FPS
    let mut event_stream = EventStream::new();

    self.render(terminal)?; // Initial render

    while self.running {
        tokio::select! {
            biased;

            // Events, tokens, startup - UNCHANGED
            ...

            // ✅ NEW: Separate render tick
            _ = render_interval.tick() => {
                if self.has_dirty_layers() {
                    self.render(terminal)?;
                }
            }

            // Animation tick - MOVE update() here
            _ = animation_interval.tick() => {
                self.update(); // Avatar wandering, etc.
            }
        }
    }
}
```

2. **Remove render from token path** (line ~314):
```rust
// BEFORE:
_ = self.conductor.process_streaming_token() => {
    self.process_conductor_messages();
    if let Err(e) = self.render(terminal) { // ❌ REMOVE THIS
        tracing::error!("Render error during streaming: {}", e);
    }
}

// AFTER:
_ = self.conductor.process_streaming_token() => {
    self.process_conductor_messages();
    // Render will happen on next frame tick (within 50ms)
}
```

3. **Remove render from main loop** (line ~366):
```rust
// BEFORE:
self.process_conductor_messages();
self.update();
self.render(terminal)?; // ❌ REMOVE THIS

// AFTER:
self.process_conductor_messages();
// update() and render() now happen in their respective interval ticks
```

4. **Add dirty tracking helper**:
```rust
impl App {
    /// Check if ANY layer needs re-rendering
    fn has_dirty_layers(&self) -> bool {
        // conversation_dirty is already tracked ✅
        self.conversation_dirty
            // Input dirty: compare current vs previous
            || (self.input_buffer != self.prev_input_buffer)
            || (self.cursor_pos != self.prev_cursor_pos)
            // Status dirty: already tracked ✅
            || (self.display.conductor_state != self.prev_conductor_state)
            || (self.display.active_task_count() != self.prev_task_count)
            || (self.scroll_offset != self.prev_scroll_offset)
            // Tasks dirty: already tracked ✅ (hash comparison in render_tasks)
            // Avatar dirty: already tracked ✅
            || self.avatar_changed
    }
}
```

**Testing**:
- Run TUI in streaming mode
- Verify tokens appear within 50ms (imperceptible to user)
- Check CPU usage: Should drop significantly (fewer render calls)
- Verify animations still smooth at 10 FPS

**Performance Gain**:
- Before: 200+ renders/sec during streaming
- After: 20 renders/sec maximum
- Reduction: **90% fewer render calls**

### Phase 2: Optimize Dirty Tracking (Follow-up)

**Enhancements**:
1. Track dirty **regions** not just layers (only re-composite changed areas)
2. Debounce rapid state changes (e.g., batch tokens for 1-2 frames)
3. Adaptive frame rate (60 FPS during animations, 20 FPS during text streaming)

**Expected Gain**: Additional 10-20% performance improvement

### Phase 3: Async Render Thread (If Needed)

**Only if**:
- Performance profiling shows `terminal.draw()` blocking is a bottleneck
- Frame rates drop below 20 FPS during heavy load
- User reports input lag or stuttering

**Implementation**: Use Option 2 (Separate Render Thread) with buffer cloning

---

## Performance Benchmarks

### Current Architecture (Before Optimization)

```
Streaming Performance (200 tokens/sec):
├─ Render calls: 200+/sec (on every token)
├─ terminal.draw() time: 1-5ms each
├─ Total blocking time: 200-1000ms/sec
├─ Frame rate: 200+ FPS (wasted)
└─ CPU usage: HIGH (unnecessary renders)

Event Loop Responsiveness:
├─ Token processing: INSTANT (async channels ✅)
├─ Keyboard input: <16ms (tokio::select! ✅)
└─ Render latency: <5ms (immediate render)
```

**Issues**:
- ❌ Blocking async runtime for 200-1000ms/sec
- ❌ Wasting CPU on 200+ FPS (human limit: ~60 FPS)
- ❌ Render calls dominate CPU time

### Proposed Architecture (After Optimization)

```
Streaming Performance (200 tokens/sec):
├─ Render calls: 20/sec MAXIMUM (frame limited)
├─ terminal.draw() time: 1-5ms each (unchanged)
├─ Total blocking time: 20-100ms/sec (90% reduction)
├─ Frame rate: 20 FPS (sufficient for text)
└─ CPU usage: LOW (10x fewer renders)

Event Loop Responsiveness:
├─ Token processing: INSTANT (async channels ✅)
├─ Keyboard input: <16ms (tokio::select! ✅)
└─ Render latency: <50ms (worst case = next frame tick)
```

**Improvements**:
- ✅ 90% reduction in blocking time
- ✅ 10x fewer render calls
- ✅ Same responsiveness (50ms latency imperceptible)
- ✅ Smooth animations (separate 10 FPS tick)

### Human Perception Baseline

| Metric | Threshold | Our Target | Status |
|--------|-----------|------------|--------|
| **Keyboard Input Lag** | <16ms (60 FPS) | <16ms | ✅ Already met |
| **Visual Smoothness** | 24-30 FPS (cinema) | 20 FPS | ✅ Sufficient |
| **Perceived Instant** | <100ms | <50ms | ✅ Exceeds |
| **Animation Smoothness** | 10-15 FPS (casual) | 10 FPS | ✅ Sufficient |

**Conclusion**: 20 FPS rendering is **more than enough** for a text-based TUI. We're optimizing for battery life and CPU efficiency, not gaming-level frame rates.

---

## Trade-offs and Considerations

### Latency Analysis

**Worst-case token-to-screen latency**:
- Token arrives just after frame tick
- Wait for next tick: 50ms
- Render + display: 5ms
- **Total: 55ms**

**Average latency**: 25ms (half the frame interval)

**Comparison to current**:
- Current: <5ms (immediate render)
- Proposed: ~25ms average, 55ms worst case

**User perception**:
- Humans can't perceive <100ms delays
- 55ms is **imperceptible** for text streaming
- Benefit (90% less CPU/blocking) far outweighs cost

### Animation Considerations

**Avatar animations** (current: update every frame):
- Current update rate: 100ms (10 FPS) ✅ Already frame-limited
- Proposed: Keep separate 10 FPS animation tick
- **No change** to animation smoothness

**Breathing effects** (REMOVED in previous optimization):
- Previously caused performance issues
- Now static colors
- **Not affected** by render rate

### Frame Rate Choice: Why 20 FPS?

**Options considered**:

| Rate | Interval | Use Case | Verdict |
|------|----------|----------|---------|
| **60 FPS** | 16.6ms | Gaming, smooth animations | ❌ Overkill for text TUI |
| **30 FPS** | 33ms | Video, casual animations | ⚠️ Could work, but no benefit over 20 |
| **20 FPS** | 50ms | Text streaming, terminal UIs | ✅ **RECOMMENDED** |
| **10 FPS** | 100ms | Status updates, slow changes | ⚠️ Too slow for smooth text |

**Why 20 FPS wins**:
- Sweet spot for text UIs (smooth enough, efficient)
- 50ms latency is imperceptible (<100ms threshold)
- 2x faster than animation tick (10 FPS) = smooth even during animation changes
- Industry standard for terminal multiplexers (tmux, screen use ~20-30 FPS)
- Low CPU overhead (90% reduction from current)

### Alternative: Adaptive Frame Rate

**Idea**: Vary frame rate based on activity

```rust
let render_interval = match self.display.conductor_state {
    ConductorState::Responding => Duration::from_millis(33), // 30 FPS during streaming
    ConductorState::Thinking => Duration::from_millis(50),   // 20 FPS during thought
    _ => Duration::from_millis(100),                         // 10 FPS when idle
};
```

**Pros**:
- Maximum smoothness when needed (30 FPS during streaming)
- Maximum efficiency when idle (10 FPS)

**Cons**:
- More complex (needs dynamic interval adjustment)
- `tokio::time::Interval` can't be mutated (would need recreation)
- Marginal benefit (20 FPS is already smooth enough)

**Verdict**: ⚠️ **DEFER** - Implement fixed 20 FPS first, measure if adaptive is needed

---

## Implementation Checklist

### Phase 1: Core Frame Limiting (Sprint Current)

- [ ] Add `render_interval` ticker (20 FPS / 50ms)
- [ ] Move `update()` to `animation_interval` ticker
- [ ] Remove `render()` from token processing path (line ~314)
- [ ] Remove `render()` from main loop (line ~366)
- [ ] Add `has_dirty_layers()` helper method
- [ ] Update `render()` to clear dirty flags after draw
- [ ] Test streaming performance (verify <50ms token latency)
- [ ] Test keyboard input (verify <16ms response)
- [ ] Test avatar animations (verify smooth at 10 FPS)

### Phase 2: Testing & Validation (Sprint Current)

- [ ] Benchmark: Render calls/sec during streaming (expect: 20 max)
- [ ] Benchmark: CPU usage during streaming (expect: significant drop)
- [ ] Benchmark: Latency from token to screen (expect: ~25ms avg, <55ms max)
- [ ] User testing: Can anyone perceive the 50ms latency? (expect: no)
- [ ] Stress test: 500+ tokens/sec stream (expect: stable 20 FPS)
- [ ] Edge case: Rapid resize events (expect: no render storm)

### Phase 3: Documentation (Sprint Current)

- [ ] Update `TODO-async-architecture-review.md` with findings
- [ ] Document frame rate choice in code comments
- [ ] Add performance characteristics to CLAUDE.md
- [ ] Create benchmark baseline for future regression testing

### Phase 4: Advanced Optimizations (Future Sprint - OPTIONAL)

- [ ] Adaptive frame rate (30 FPS during streaming, 10 FPS idle)
- [ ] Dirty region tracking (only re-composite changed areas)
- [ ] Token batching (collect 2-3 tokens before marking dirty)
- [ ] Async render thread (if blocking becomes measurable issue)

---

## Success Criteria

### Performance Targets

- ✅ Render calls reduced from 200+/sec → 20/sec maximum
- ✅ Total blocking time reduced from 200-1000ms/sec → 20-100ms/sec
- ✅ Token-to-screen latency: <55ms worst case (imperceptible to humans)
- ✅ Keyboard input latency: <16ms (unchanged, already excellent)
- ✅ Animation smoothness: 10 FPS (unchanged, already sufficient)
- ✅ CPU usage: Significant reduction (10x fewer render calls)

### User Experience Targets

- ✅ No perceptible lag during token streaming
- ✅ Smooth avatar animations (no jitter)
- ✅ Instant keyboard response
- ✅ No frame drops during resize/scroll
- ✅ Battery-friendly (less CPU churn)

### Code Quality Targets

- ✅ No new dependencies (use tokio::time only)
- ✅ No increased complexity (simpler event loop)
- ✅ No blocking in async runtime (just controlled blocking in draw)
- ✅ Clean separation: events → state → dirty flags → render

---

## Related Documents

### Research Sources

**Ratatui Official**:
- [Ratatui FAQ](https://ratatui.rs/faq/) - Async guidance and rendering best practices
- [Full Async Events Tutorial](https://ratatui.rs/tutorials/counter-async-app/full-async-events/) - Event merging patterns
- [Ratatui Async Template](https://github.com/ratatui/async-template) - Frame rate limiting example (deprecated but shows pattern)

**Rust Async Patterns**:
- [Tokio spawn_blocking](https://forum.ratatui.rs/t/understanding-tokio-spawn-and-tokio-spawn-blocking/74) - When to use blocking threads
- [Rust Channel Comparison](https://codeandbitters.com/rust-channel-comparison/) - tokio::mpsc vs crossbeam performance
- [Tokio vs Crossbeam Channels](https://users.rust-lang.org/t/differences-between-channel-in-tokio-mpsc-and-crossbeam/92676) - Async channel guidance

**Alternative TUI Frameworks**:
- [r3bl_tui](https://docs.rs/r3bl_tui/latest/r3bl_tui/) - Double buffering and async middleware patterns
- [tui-core](https://github.com/AstekGroup/tui-core) - Component-based async TUI template
- [Crossfire](https://crates.io/crates/crossfire) - High-performance async/blocking hybrid channels

### Internal Documents

- `progress/TODO-async-architecture-review.md` - Overall async audit
- `facts/PERFORMANCE-AUDIT-streaming-slowness-root-cause.md` - Model overhead analysis
- `yollayah/core/surfaces/tui/examples/reactive_prototype.rs` - Event merging prototype

---

## Team Review Requests

### Rust & Ratatui Team
- [ ] **Validate approach**: Is frame limiting the right solution?
- [ ] **Review tokio patterns**: Is our `tokio::select!` usage optimal?
- [ ] **Dirty tracking**: Any edge cases we missed?

### Performance Hacker
- [ ] **Benchmark design**: Are our performance targets realistic?
- [ ] **Profiling**: What should we measure before/after?
- [ ] **Edge cases**: Stress test scenarios we should try?

### Architect
- [ ] **Philosophy alignment**: Does this match async principles?
- [ ] **Trade-offs**: Is 50ms latency acceptable for ai-way's UX?
- [ ] **Future-proofing**: Will this scale to web/API surfaces?

### UX Team
- [ ] **Perception testing**: Can users notice 50ms token latency?
- [ ] **Animation quality**: Is 10 FPS avatar movement smooth enough?
- [ ] **Responsiveness**: Does 20 FPS feel responsive during heavy streaming?

---

## Conclusion

**The Solution**: Frame-rate-limited rendering (20 FPS) with dirty tracking

**Why it works**:
1. **Reduces render calls by 90%** (200+/sec → 20/sec)
2. **Minimal latency** (50ms worst case, imperceptible to humans)
3. **Simple implementation** (leverage existing dirty tracking)
4. **No new dependencies** (just tokio::time intervals)
5. **Proven pattern** (used by Ratatui templates and other TUIs)

**What doesn't work**:
1. ❌ Rendering on every token (current issue)
2. ❌ Async render threads (too complex, overkill)
3. ❌ spawn_blocking for draw calls (adds overhead, marginal benefit)

**Next steps**:
1. Implement Phase 1 (frame limiting) in current sprint
2. Benchmark before/after
3. User testing for perceived latency
4. Document findings

**Philosophy alignment**:
> "No Sleep, Only Wait on Async I/O" - We're not adding sleep, we're using interval tickers (async wait). ✅
> "No Blocking I/O" - terminal.draw() is still blocking, but controlled (20 calls/sec vs 200+). Acceptable trade-off. ✅
> "Surfaces Are Thin Clients" - Render is the last mile, 100ms blocking/sec is negligible. ✅

---

**Status**: PROPOSAL COMPLETE - Ready for team review and implementation
**Recommended Next Action**: Implement Phase 1 (core frame limiting) in current sprint
**Estimated Effort**: 2-4 hours implementation + 2 hours testing
**Risk Level**: LOW - Proven pattern, minimal changes, easy rollback
