# TODO-BUG-001: TUI Waits for Full Stream Before Displaying

**Status**: 🔴 CRITICAL - Active Investigation
**Created**: 2026-01-03
**Priority**: P0 (blocking user experience)
**Team**: Architect, Hacker, Rust Async Experts

## Symptom Summary

The TUI is **not streaming responses in real-time**. Instead:

1. ⏱️ **Wait Phase**: TUI appears frozen, waiting for entire response
2. 🔥 **GPU Load**: GPU activity starts (model inference)
3. 💾 **CPU Spike**: After GPU completes, CPU load spikes dramatically
4. 📤 **Delayed Display**: All text dumps at once, no streaming

**User Impact**: Experience feels broken - defeats purpose of streaming interface.

## Performance Pattern (Observed)

```
User sends prompt
    ↓
TUI freezes (no visual feedback)
    ↓
GPU starts loading (backend working)
    ↓
GPU completes inference
    ↓
CPU spikes (suspicious!)  ← MEMORY LEAK?
    ↓
All text dumps at once
```

## Hypothesis: Async Stack Issues

### Suspected Root Causes

1. **Over-Async Pattern**: Team may have gone "async happy"
   - Unnecessary async/await chains
   - Blocking operations disguised as async
   - Violates PRINCIPLE-efficiency.md

2. **Memory Accumulation**: CPU spike suggests:
   - Buffering entire response in memory
   - Not processing tokens as they arrive
   - Possible Vec growth without streaming

3. **Event Loop Blocking**:
   - Async operations blocking the event loop
   - Message handling not truly async
   - UI updates waiting for completion

## ROOT CAUSE IDENTIFIED 🎯

**Investigation Completed**: 2026-01-03
**Status**: ✅ DIAGNOSED - Ready for Fix

### The Problem: Event Loop Blocking on First Token

The TUI is **NOT actually broken** - it's streaming correctly, but the event loop is **blocking** on the first token arrival.

#### Architecture Flow (How It SHOULD Work)

```
Backend (Ollama)              Conductor                    TUI
    ↓                            ↓                          ↓
Streaming Task            poll_streaming()         Event Loop (10 FPS)
   spawned                  rx.recv().await ← BLOCKS HERE
    ↓
Sends tokens via
mpsc::channel(256)
```

#### The Blocking Issue

**Location**: `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/conductor.rs:1022-1065`

**Problematic Code**:
```rust
pub async fn poll_streaming(&mut self) -> bool {
    // ...
    // ✅ GOOD: Wait asynchronously for first token (no polling, no sleep!)
    // ❌ BAD: This blocks until a token arrives!
    match rx.recv().await {  // ← BLOCKS EVENT LOOP
        Some(token) => {
            // ...process tokens...
        }
        None => return false,
    }
}
```

**Called From**: `/var/home/machiyotl/src/ai-way/yollayah/core/surfaces/tui/src/app.rs:345-354`

```rust
// In main event loop (runs every 100ms)
while self.running {
    tokio::select! {
        // ... handle events ...
        _ = tokio::time::sleep(Duration::from_millis(100)) => {
            // ...
        }
    }

    // ❌ PROBLEM: This blocks if no tokens available!
    self.conductor.poll_streaming().await;  // ← Can block for seconds!

    // These only run AFTER poll_streaming completes!
    self.update();
    self.render(terminal)?;
}
```

#### Why It Blocks

1. **Backend spawns streaming task** (line 181 in `ollama.rs`) - runs in background
2. **TUI event loop calls `poll_streaming()`** every frame
3. **`poll_streaming()` calls `rx.recv().await`** - waits for first token
4. **GPU is loading model** - takes 2-5 seconds before first token
5. **Event loop is frozen** - no rendering, no updates, appears hung
6. **First token arrives** - suddenly ALL tokens drain, CPU spike from batch processing
7. **All text dumps** - because they were buffered while UI was frozen

### Investigation Completed

#### [✅] Phase 1: Message Reception Analysis

**Findings**:
- **Tokens stream correctly** from Ollama backend (line 200-208 in `ollama.rs`)
- **Channel is appropriately sized** (256 buffer, line 132)
- **No `.collect()` accumulation** - tokens processed individually
- **Problem is NOT in message handling** - it's in when they're processed

**Key Discovery**: The backend is sending tokens fine, but the TUI is blocking waiting for them!

#### [✅] Phase 2: Async Pattern Audit

**Findings**:
- **VIOLATION FOUND**: `rx.recv().await` in tight event loop
- **No blocking I/O** - all async is proper tokio
- **No `.wait()` or `block_on()`** - async is correct
- **Problem**: Correct async, WRONG LOCATION

**The Issue**:
```rust
// ❌ BAD: Blocking recv in polling function
pub async fn poll_streaming(&mut self) -> bool {
    match rx.recv().await {  // Blocks until token available!
        // ...
    }
}
```

Should be:
```rust
// ✅ GOOD: Non-blocking check
pub async fn poll_streaming(&mut self) -> bool {
    match rx.try_recv() {  // Returns immediately!
        Ok(token) => { /* process */ }
        Err(_) => return false,  // No tokens yet, keep rendering
    }
}
```

#### [✅] Phase 3: Event Loop Analysis

**Findings**:
- **Event loop structure is GOOD** (app.rs:260-375)
- **tokio::select! is correct** (handles terminal events properly)
- **Frame timing is correct** (10 FPS, 100ms frame time)
- **PROBLEM**: Event loop can't render because it's blocked in `poll_streaming()`

**Evidence** (app.rs:345-360):
```rust
// Process messages FIRST (good!)
self.process_conductor_messages();

// ❌ THIS BLOCKS!
self.conductor.poll_streaming().await;

// Process any newly arrived messages
self.process_conductor_messages();

// ❌ These only run AFTER blocking completes!
self.update();
self.render(terminal)?;
```

#### [✅] Phase 4: Memory/CPU Spike Analysis

**Findings**:
- **Not a memory leak** - just delayed batch processing
- **CPU spike is from**: Processing all buffered tokens at once
- **Why**: While `poll_streaming()` blocked:
  1. Backend kept sending tokens to channel
  2. Channel buffered them (256 capacity)
  3. First `recv()` unblocks
  4. `while let Ok(token) = rx.try_recv()` drains ALL buffered tokens (line 1046)
  5. CPU processes ~200 tokens instantly instead of spread over time

## The Fix Strategy

### Solution: Non-Blocking Token Polling

**Change ONE line** in `conductor.rs:1034`:

```rust
// FROM (blocking):
match rx.recv().await {

// TO (non-blocking):
match rx.try_recv() {
    Ok(token) => {
        // ... existing token processing ...
    }
    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
        return false;  // No tokens yet, keep UI responsive
    }
    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
        return false;  // Stream ended
    }
}
```

### Why This Fixes Everything

1. **Immediate return** when no tokens available → UI stays responsive
2. **Tokens process as they arrive** → true streaming display
3. **No blocking** → event loop runs at 10 FPS consistently
4. **No CPU spike** → tokens processed gradually, not batched
5. **No user-visible delay** → UI renders "thinking" state while waiting

### Alternative Approach (More Complex)

Move token reception into `tokio::select!`:

```rust
tokio::select! {
    maybe_event = event_stream.next() => { /* ... */ }

    // Add token reception as event source
    maybe_token = self.conductor.recv_token() => {
        if let Some(token) = maybe_token {
            // Process token immediately
        }
    }

    _ = tokio::time::sleep(frame_duration) => { /* ... */ }
}
```

This is architecturally cleaner but requires refactoring event loop structure.

## Investigation Tasks

### [✅] Phase 1: Message Reception Analysis
- [✅] Traced message flow from conductor to display
- [✅] Identified token reception at `conductor.rs:1034`
- [✅] Confirmed tokens are NOT batched (backend sends individually)
- [✅] Found blocking `.recv().await` in polling loop

### [✅] Phase 2: Async Pattern Audit
- [✅] Identified all async fn in TUI
- [✅] No blocking operations found (sleep, sync I/O)
- [✅] No `block_on()` or `.wait()` anti-patterns
- [✅] **FOUND**: Blocking await in wrong context (polling loop)

### [✅] Phase 3: Event Loop Analysis
- [✅] Event loop structure verified (tokio::select! correct)
- [✅] No CPU-bound work in async
- [✅] All I/O is async (tokio-based)
- [✅] **FOUND**: Event loop blocked by `poll_streaming()`

### [✅] Phase 4: Memory/CPU Spike Hunt
- [✅] CPU spike is delayed batch processing (not leak)
- [✅] Channel buffering verified (256 tokens)
- [✅] Tokens accumulated during blocking, then processed as batch
- [✅] **FOUND**: Spike caused by draining full buffer at once

## Principles Violated (Suspected)

From `knowledge/principles/PRINCIPLE-efficiency.md`:

- ❌ **"No Sleep, Only Wait on Async I/O"**
  → May have blocking operations in async stack

- ❌ **"No Blocking I/O in Async Context"**
  → Need to verify all I/O is tokio-based

- ❌ **"Surfaces Are Thin Clients"**
  → TUI may be doing too much processing

## Immediate Action Items

1. **Architect**: Review async call graph, identify bloat
2. **Hacker**: Instrument with tracing, find bottleneck
3. **Rust Team**: Profile memory, check streaming path

## Success Criteria

- ✅ Tokens display as they arrive (true streaming)
- ✅ No visible delay between GPU start and first token
- ✅ No CPU spike pattern
- ✅ Memory usage stays flat during streaming
- ✅ Event loop remains responsive

## References

- `knowledge/principles/PRINCIPLE-efficiency.md`
- `knowledge/requirements/REQUIRED-separation.md`
- `yollayah/core/surfaces/tui/src/` (TUI codebase)

## Timeline

- **Created**: 2026-01-03
- **Investigation Started**: _pending_
- **Root Cause Identified**: _pending_
- **Fix Implemented**: _pending_
- **Verified Fixed**: _pending_

---

**Next Steps**: Launch investigation agents to analyze message handling and async patterns.
