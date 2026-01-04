# Conductor Streaming Architecture Analysis

**Date**: 2026-01-03
**Analysts**: Rust Specialist, Async Expert, Architect
**Purpose**: Deep analysis of conductor streaming anti-patterns

---

## Executive Summary

**FINDING**: Conductor streaming uses POLLING pattern instead of REACTIVE pattern.

**IMPACT**: Tokens can only be consumed at 10 FPS (100ms intervals), creating visible "chunking" effect.

**ROOT CAUSE**: TUI event loop polls for tokens OUTSIDE tokio::select! instead of reactively awaiting them INSIDE select!.

---

## Current Architecture (BROKEN)

### TUI Event Loop Structure

```rust
while self.running {
    tokio::select! {
        // Terminal events (reactive ✅)
        maybe_event = event_stream.next() => { ... }

        // Frame tick every 100ms (polling ❌)
        _ = tokio::time::sleep(Duration::from_millis(100)) => {
            // Startup phases
        }
    }

    // OUTSIDE select! - POLLING ANTI-PATTERN ❌
    self.process_conductor_messages();
    self.conductor.poll_streaming().await;  // ← POLLS every frame!
    self.process_conductor_messages();
    self.update();
    self.render(terminal)?;

    // Sleep to maintain 10 FPS
    tokio::time::sleep(remaining_frame_time).await;
}
```

**Location**: `yollayah/core/surfaces/tui/src/app.rs:260-375`

### Conductor poll_streaming() Implementation

```rust
pub async fn poll_streaming(&mut self) -> bool {
    let rx = match self.streaming_rx.as_mut() {
        Some(rx) => rx,
        None => return false,
    };

    // NON-BLOCKING POLL ❌
    match rx.try_recv() {  // ← Returns immediately if no token
        Ok(token) => {
            // Process token
            // Drain additional buffered tokens
            while let Ok(token) = rx.try_recv() { ... }
        }
        Err(_) => return false,  // No tokens, come back next frame
    }
}
```

**Location**: `yollayah/conductor/core/src/conductor.rs:1022-1070`

---

## The Problem

### Polling Frequency Bottleneck

1. TUI runs at **10 FPS** (100ms per frame)
2. `poll_streaming()` called once per frame
3. Can only check for tokens **10 times per second**
4. Ollama generates ~**200 tokens/second**
5. **200 tok/s ÷ 10 FPS = 20 tokens/frame**

### Visible Effect

- Tokens arrive continuously from Ollama
- Buffer fills with 20+ tokens every 100ms
- `poll_streaming()` drains buffer once per frame
- User sees **chunks of 20 tokens** appear at once
- Looks like "slow streaming" but it's actually **batched polling**

### Performance Impact

| Metric | Current (Polling) | Expected (Reactive) |
|--------|-------------------|---------------------|
| Token consumption rate | 10 Hz (100ms intervals) | Immediate (< 1ms) |
| Tokens per batch | ~20 | 1 |
| Visible latency | 100ms chunks | Smooth stream |
| CPU usage | High (unnecessary polls) | Low (event-driven) |

---

## Why This Happens

### Historical Context

From `progress/TODO-BUG-001-tui-waits-for-full-stream.md`:

**Original Bug**: TUI used `rx.recv().await` in `poll_streaming()`
- Blocked event loop waiting for first token
- UI froze for 2-5 seconds during GPU model load

**Fix Applied**: Changed `recv().await` → `try_recv()`
- Non-blocking, so UI doesn't freeze ✅
- But introduced POLLING anti-pattern ❌

**Insight**: We "fixed" the symptom, not the root cause!

### The Real Issue

`poll_streaming()` API design is fundamentally flawed:
- Assumes synchronous/polled consumption
- Forces consumers to call it repeatedly
- Defeats async/reactive architecture

---

## Correct Architecture (REACTIVE)

### How It Should Work

```rust
while self.running {
    tokio::select! {
        // Terminal events (reactive)
        maybe_event = event_stream.next() => { ... }

        // Streaming tokens (REACTIVE - NEW!) ✅
        Some(token) = conductor.recv_streaming_token() => {
            // Process token IMMEDIATELY when it arrives
            self.process_streaming_token(token);
            self.render(terminal)?;  // Render immediately
        }

        // Frame tick for animations only
        _ = tokio::time::sleep(Duration::from_millis(100)) => {
            // Just update animations, not streaming
            self.update_animations();
            self.render(terminal)?;
        }
    }
}
```

### Key Changes

1. **Add streaming to select!**: Tokens become another async event source
2. **Remove poll_streaming()**: No longer needed
3. **Immediate rendering**: Render when token arrives, not on frame tick
4. **Frame tick for animations**: Only non-streaming updates

### New Conductor API

```rust
impl Conductor {
    /// Receive next streaming token (reactive)
    /// Returns None when no active stream
    pub async fn recv_streaming_token(&mut self) -> Option<StreamingToken> {
        match self.streaming_rx.as_mut() {
            Some(rx) => rx.recv().await,  // ← BLOCKS until token (reactive!)
            None => None,
        }
    }
}
```

**This is REACTIVE**:
- Awaits token arrival (doesn't poll)
- Used in tokio::select! (event-driven)
- Processes tokens as they arrive (immediate)

---

## Comparison: Direct Ollama vs Conductor

### Why Direct Ollama Feels Instant

```bash
ollama run llama3.1:8b "test"
```

Ollama CLI uses reactive streaming:
1. Sends HTTP request (async)
2. Reads response stream (async iterator)
3. Prints each token IMMEDIATELY when received
4. No polling, no batching, no FPS limits

### Why Conductor Feels Slow

1. Ollama generates tokens (fast)
2. Conductor backend receives tokens (fast)
3. Sends to mpsc channel (fast)
4. **TUI polls channel at 10 FPS (SLOW!)**
5. Processes batches of ~20 tokens
6. User sees chunked/"slow" streaming

---

## Implementation Plan

### Phase 1: Add Reactive API to Conductor

**File**: `yollayah/conductor/core/src/conductor.rs`

```rust
impl Conductor {
    /// Receive next streaming token reactively
    pub async fn recv_streaming_token(&mut self) -> Option<StreamingToken> {
        match self.streaming_rx.as_mut() {
            Some(rx) => rx.recv().await,
            None => None,
        }
    }

    // Deprecate poll_streaming() - mark with #[deprecated]
}
```

### Phase 2: Update TUI to Use tokio::select!

**File**: `yollayah/core/surfaces/tui/src/app.rs`

Move streaming token reception INTO select! block:

```rust
tokio::select! {
    biased;

    // Terminal events (existing)
    maybe_event = event_stream.next() => { ... }

    // Streaming tokens (NEW - reactive!)
    Some(token) = self.conductor.recv_streaming_token() => {
        self.process_streaming_token(token);
        self.render(terminal)?;  // Render on token arrival
    }

    // Frame tick for animations only
    _ = tokio::time::sleep(Duration::from_millis(100)) => {
        self.update_animations();
        self.render(terminal)?;
    }
}
```

### Phase 3: Remove Polling Code

Delete:
- `poll_streaming()` calls
- Redundant `process_conductor_messages()` calls
- Frame-rate sleep (handled by select!)

---

## Success Criteria

### Performance Metrics

| Metric | Before (Polling) | After (Reactive) | Test Method |
|--------|------------------|------------------|-------------|
| Token latency | ~100ms (batched) | < 1ms (immediate) | Visual observation |
| Batch size | ~20 tokens/frame | 1 token | Code inspection |
| Streaming smoothness | Chunky | Smooth | User experience |
| CPU during streaming | High (polling) | Low (event-driven) | htop |
| Matches Ollama CLI | No | Yes | Side-by-side test |

### Acceptance Tests

```bash
# Test reactive streaming
./yollayah.sh --interactive
# Send: "Write a long story"

# Expected:
# 1. First token appears immediately (< 1ms)
# 2. Tokens stream smoothly (no chunks)
# 3. Feels identical to: ollama run llama3.1:8b "Write a long story"
# 4. No visible 100ms batching
```

---

## Related Bugs

### Fixed (But Introduced Polling)

**TODO-BUG-001**: TUI waits for full stream
- **Symptom**: UI froze for 2-5s during GPU load
- **Cause**: Blocking `rx.recv().await` in poll loop
- **Fix**: Changed to `try_recv()` (non-blocking)
- **Side Effect**: Introduced polling anti-pattern

### Current (This Analysis)

**TODO-BUG-006**: Conductor streaming not reactive
- **Symptom**: Tokens arrive in chunks every 100ms
- **Cause**: Polling at 10 FPS instead of reactive select!
- **Fix**: Move streaming into tokio::select! (this plan)

---

## Principle Violations

From `knowledge/principles/PRINCIPLE-efficiency.md`:

### ❌ "No Polling, Only Wait on I/O"

**Current**:
```rust
// Polls 10 times per second
loop {
    if let Ok(token) = rx.try_recv() { ... }  // Poll
    sleep(100ms);
}
```

**Correct**:
```rust
// Waits reactively for I/O
tokio::select! {
    token = rx.recv() => { ... }  // Wait on I/O
}
```

### ❌ "Event-Driven, Not Polled"

**Current**: TUI polls conductor every frame
**Correct**: TUI reacts to conductor events

---

## Team Consensus

**Rust Specialist**: API design flaw, need reactive method
**Async Expert**: Classic polling anti-pattern, use tokio::select!
**Architect**: Fundamental design issue, must fix for true reactivity

**Approved Fix**: Add `recv_streaming_token()` and migrate TUI to select!

---

## Next Steps

1. ✅ Analysis complete (this document)
2. Create TODO-STORY for implementation
3. Implement recv_streaming_token() in conductor
4. Update TUI to use tokio::select!
5. Test streaming performance
6. Verify matches direct Ollama CLI experience
