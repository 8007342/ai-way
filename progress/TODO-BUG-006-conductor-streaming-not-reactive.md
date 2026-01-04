# TODO-BUG-006-Conductor-Streaming-Not-Reactive

**Status**: 🔴 CRITICAL - Active Investigation
**Created**: 2026-01-03
**Priority**: P0 - BLOCKING UX
**Severity**: CRITICAL - Defeats purpose of streaming
**Team**: Rust Specialist, Async Expert, Architect (REQUIRED)

---

## Navigation

**Parent**: [TODO-EPIC-conductor-reactive-overhaul.md](TODO-EPIC-conductor-reactive-overhaul.md)
**Siblings**:
- [TODO-BUG-conductor-slow-ollama-calls.md](../yollayah/conductor/TODO-BUG-conductor-slow-ollama-calls.md)
**Related**:
- [TODO-BUG-001-tui-waits-for-full-stream.md](TODO-BUG-001-tui-waits-for-full-stream.md)

---

## Problem Statement

**Startup is now fast (warmup eliminated), but streaming is still broken.**

### Observed Behavior

User report (2026-01-03):
> "parsing responses is still waiting for the complete response and then 'streaming slowly', looks like a synchronous poll of a queue, rather than an observable stream"

### Symptoms

1. ✅ **Startup**: Near-instant (warmup fix worked!)
2. ❌ **Response streaming**: Waits for COMPLETE response
3. ❌ **Display**: Then "streams slowly" (chunked/batched)
4. ❌ **Pattern**: Synchronous polling, NOT reactive streams

### Expected vs Actual

**Expected (Reactive)**:
```
User sends message
    ↓
Backend starts generating (async spawn)
    ↓
Tokens arrive → Immediately displayed (observable stream)
    ↓
True streaming (token-by-token as generated)
```

**Actual (Polling)**:
```
User sends message
    ↓
Backend starts generating
    ↓
Wait for ALL tokens (blocking/polling?)
    ↓
Display buffered response "slowly" (chunked batches)
```

---

## Root Cause Hypothesis

**Conductor is using POLLING instead of REACTIVE STREAMS**

### Suspected Anti-Patterns

1. **Polling loop with try_recv()**:
   - `poll_streaming()` called from TUI event loop
   - Uses `try_recv()` which is POLLING, not reactive
   - TUI must poll every frame (10 FPS) to check for tokens

2. **No true observable stream**:
   - Backend spawns task correctly (tokio::spawn)
   - Sends to mpsc channel correctly
   - But consumer uses POLLING instead of SELECT/OBSERVE

3. **Frame-limited consumption**:
   - TUI polls at 10 FPS (100ms intervals)
   - Can only drain tokens 10 times per second
   - 200 tok/sec ÷ 10 FPS = 20 tokens per frame
   - Causes "chunky" streaming

---

## Principle Violations

From `knowledge/principles/PRINCIPLE-efficiency.md`:

### ❌ VIOLATED: "No Polling, Use Async Observers"

**Current Implementation** (conductor.rs:1022):
```rust
pub async fn poll_streaming(&mut self) -> bool {  // ← POLLING!
    match rx.try_recv() {  // ← NON-BLOCKING POLL
        Ok(token) => { /* process */ }
        Err(_) => return false,  // No token, come back later
    }
}
```

**Should Be** (Reactive):
```rust
// In TUI event loop:
tokio::select! {
    Some(token) = conductor.recv_token() => {  // ← REACTIVE!
        // Process immediately when token arrives
    }
    event = event_stream.next() => {
        // Handle UI events
    }
}
```

### ❌ VIOLATED: "Event-Driven, Not Polled"

The conductor forces TUI to poll for tokens instead of being notified.

---

## Investigation Required

### Phase 1: Streaming Architecture Audit

**Questions**:
1. Why does conductor use `poll_streaming()` instead of reactive API?
2. Is there a way to make conductor emit observable events?
3. Can we integrate streaming channel into tokio::select!?
4. Is the mpsc channel being used reactively?

### Phase 2: TUI Integration Review

**File**: `yollayah/core/surfaces/tui/src/app.rs`

**Check**:
- How does TUI call `poll_streaming()`?
- Is it in event loop? At what frequency?
- Can we move to tokio::select! pattern?

### Phase 3: Compare with Direct Ollama

Direct Ollama CLI streams perfectly (reactive).
Need to understand what conductor is doing differently.

---

## Team Analysis

### Rust Specialist

**Question**: Is there a better channel type for reactive streaming?
- Current: `mpsc::channel(256)` with `try_recv()` (polling)
- Better: `tokio::sync::broadcast` or `tokio::sync::watch`?
- Or: Expose channel receiver directly to TUI's tokio::select!?

**Concern**: `poll_streaming()` API design forces polling pattern

### Async Expert

**Analysis**: The async is correct, but the USAGE is wrong

**Problem**: Mixing polling (try_recv) with event-driven (tokio::select!)
- Polling is for synchronous/threaded contexts
- Event loops should use select!/await patterns

**Solution**:
- Remove `poll_streaming()` entirely
- Expose `recv_token() -> impl Future<Output = Option<Token>>`
- Let TUI use in tokio::select! (true reactive)

### Architect

**Design Flaw**: Conductor API is poll-based, not reactive

**Root Issue**:
- Conductor doesn't provide reactive API
- Forces consumers to poll
- Defeats async/reactive principles

**Fix Strategy**:
1. Change conductor API to be reactive (not poll-based)
2. Expose token stream as Future/Stream
3. Let TUI use tokio::select! for true reactivity

---

## Proposed Solution

### Option 1: Expose Channel Receiver (Simplest)

**Conductor**:
```rust
// Remove poll_streaming(), add:
pub async fn recv_token(&mut self) -> Option<StreamingToken> {
    match self.streaming_rx.as_mut() {
        Some(rx) => rx.recv().await,
        None => None,
    }
}
```

**TUI**:
```rust
loop {
    tokio::select! {
        maybe_token = conductor.recv_token() => {
            if let Some(token) = maybe_token {
                // Process immediately (reactive!)
            }
        }
        event = event_stream.next() => {
            // Handle events
        }
    }
}
```

### Option 2: Stream-Based API (More Idiomatic)

Use `futures::Stream` trait for truly reactive streaming.

### Option 3: Rx/Observable Pattern (Most Reactive)

Integrate with rxrust (already in Cargo.toml) for full reactive streams.

---

## Success Criteria

- [ ] Tokens display immediately as they arrive (no batching)
- [ ] No polling - pure reactive/event-driven
- [ ] Streaming feels instant (like direct Ollama CLI)
- [ ] No visible chunks/batches
- [ ] CPU usage minimal during streaming
- [ ] No FPS limitations on token consumption

---

## Testing Plan

```bash
# Test streaming reactivity
./yollayah.sh --interactive

# Send long prompt, observe:
# 1. First token appears immediately (no wait)
# 2. Tokens stream smoothly (no chunks)
# 3. No batching visible
# 4. Feels like direct Ollama CLI
```

Compare with direct Ollama:
```bash
ollama run llama3.1:8b "Write a long story about..."
# Should feel identical to conductor streaming
```

---

## Related Work

**Fixed**:
- ✅ TODO-BUG-001: Changed `recv().await` → `try_recv()` (non-blocking)
  - This fixed TUI freezing
  - But introduced polling anti-pattern!

**Current Issue**:
- ❌ `try_recv()` is POLLING (check if token ready)
- ✅ Should use `recv().await` in tokio::select! (reactive)

**Insight**: We "fixed" the wrong thing!
- TUI should NOT call poll_streaming() every frame
- TUI should use tokio::select! with conductor.recv_token().await

---

## Next Steps

1. **Deep dive**: Analyze current poll_streaming() usage
2. **Prototype**: Implement recv_token() API
3. **Test**: Verify reactive streaming works
4. **Migrate**: Update TUI to use tokio::select!
5. **Verify**: Compare streaming with direct Ollama CLI

---

**This is the REAL streaming bug. Startup was a red herring.**
