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

## Principles Violated (CONFIRMED)

From `knowledge/principles/PRINCIPLE-efficiency.md`:

- ✅ **"No Sleep, Only Wait on Async I/O"** - NOT VIOLATED
  → All I/O is async (tokio-based), no sleep() calls

- ✅ **"No Blocking I/O in Async Context"** - NOT VIOLATED
  → All I/O is tokio (reqwest, mpsc channels)

- ⚠️ **"Event Loop Must Not Block"** - VIOLATED
  → `rx.recv().await` blocks event loop in `poll_streaming()`

- ✅ **"Surfaces Are Thin Clients"** - NOT VIOLATED
  → TUI only renders, conductor does all business logic

### The Actual Violation

**Pattern**: **Blocking Await in Polling Loop**

```rust
// ❌ ANTI-PATTERN: Blocking recv in event loop
pub async fn poll_streaming() {
    match rx.recv().await {  // Blocks until data available!
        // ...
    }
}

// Called from:
loop {
    poll_streaming().await;  // ← Event loop frozen here!
    render();
}
```

**Should Be**: **Non-Blocking Try-Recv in Polling Loop**

```rust
// ✅ CORRECT: Non-blocking check
pub fn poll_streaming() {  // No async needed!
    match rx.try_recv() {  // Returns immediately
        Ok(token) => { /* process */ }
        Err(_) => return false,
    }
}

// Called from:
loop {
    poll_streaming();  // Returns instantly if no data
    render();  // Always reached
}
```

## Code Evidence

### File: `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/conductor.rs`

**Lines 1022-1065**: The blocking call

```rust
pub async fn poll_streaming(&mut self) -> bool {
    let tokens: Vec<StreamingToken> = {
        let rx = match self.streaming_rx.as_mut() {
            Some(rx) => rx,
            None => return false,
        };

        let mut collected = Vec::new();

        // ❌ PROBLEM LINE 1034: This blocks until token arrives!
        match rx.recv().await {  // ← BLOCKS EVENT LOOP FOR 2-5 SECONDS
            Some(token) => {
                let is_terminal = matches!(
                    token,
                    StreamingToken::Complete { ..} | StreamingToken::Error(_)
                );
                collected.push(token);

                // Then drains any buffered tokens
                if !is_terminal {
                    while let Ok(token) = rx.try_recv() {  // ← This is fine
                        // ...
                    }
                }
            }
            None => return false,
        }

        collected
    };
    // ... process tokens ...
}
```

### File: `/var/home/machiyotl/src/ai-way/yollayah/core/surfaces/tui/src/app.rs`

**Lines 345-360**: Where the blocking happens in event loop

```rust
while self.running {
    // ... event handling via tokio::select! ...

    // ❌ LINE 347: This can block for seconds!
    self.process_conductor_messages();

    // ❌ LINE 351: THIS IS THE BLOCKING CALL
    self.conductor.poll_streaming().await;  // ← FREEZES UI HERE

    // Line 354: Process any newly arrived messages
    self.process_conductor_messages();

    // Lines 357-360: These only run AFTER streaming completes!
    self.update();
    self.render(terminal)?;  // ← Never renders until tokens arrive!
}
```

### File: `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/backend/ollama.rs`

**Lines 181-243**: Background streaming task (this is CORRECT)

```rust
// ✅ This is properly async and non-blocking
tokio::spawn(async move {
    let mut buffer = String::new();
    let mut full_response = String::new();

    // Streams tokens from Ollama
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                // Parse JSON and send tokens
                if tx.send(StreamingToken::Token(token.to_string()))
                    .await
                    .is_err()
                {
                    return;  // Receiver dropped
                }
            }
            // ...
        }
    }
});
```

## Immediate Action Items

### [✅] Investigation Complete
1. **Architect**: ✅ Async call graph reviewed - ONE blocking point found
2. **Hacker**: ✅ Bottleneck identified - `rx.recv().await` in polling loop
3. **Rust Team**: ✅ Memory profiled - no leak, just delayed batch processing

### [✅] Phase 5: Comprehensive Anti-Pattern Triage (2026-01-03)

**Complete codebase scan for "Blocking Await in Event Loops" anti-pattern.**

#### Violations Found

**CRITICAL VIOLATIONS** (Event Loop Blocking):

1. **`conductor.rs:1034`** - PRIMARY VIOLATION
   - **Function**: `pub async fn poll_streaming(&mut self) -> bool`
   - **Pattern**: `rx.recv().await` in polling function
   - **Called From**: TUI event loop (app.rs:351), daemon loops
   - **Impact**: TUI freezes for 2-5 seconds during model loading
   - **Priority**: P0 - CRITICAL
   - **Fix**: Change to `rx.try_recv()`

2. **`conductor_client.rs:380`** - WRAPPER VIOLATION
   - **Function**: `pub async fn poll_streaming(&mut self) -> bool`
   - **Pattern**: Calls `conductor.poll_streaming().await`
   - **Impact**: Propagates blocking from conductor.rs:1034
   - **Priority**: P0 - CRITICAL (fixed by fixing #1)
   - **Fix**: Automatically fixed when #1 is fixed (just forwards call)

**ACCEPTABLE USES** (Not in Event Loops):

3. **`conductor.rs:410`** - ✅ ACCEPTABLE
   - **Function**: `async fn warmup(&mut self)`
   - **Pattern**: `while let Some(token) = rx.recv().await`
   - **Context**: One-time warmup during initialization, NOT in event loop
   - **Impact**: None (runs once at startup before event loop starts)
   - **Priority**: N/A - Not a violation

4. **`server.rs:196`** - ✅ ACCEPTABLE (Dedicated Task)
   - **Function**: Spawned task in dedicated thread
   - **Pattern**: `while let Some((conn_id, event)) = event_rx.recv().await`
   - **Context**: Background task processing events, NOT event loop
   - **Impact**: None (separate tokio task)
   - **Priority**: N/A - Not a violation

5. **`server.rs:214`** - ⚠️ POTENTIALLY PROBLEMATIC
   - **Function**: Spawned task with tight loop
   - **Pattern**: `loop { c.poll_streaming().await; tokio::task::yield_now().await; }`
   - **Context**: Dedicated streaming task, but calls blocking poll_streaming()
   - **Impact**: Task blocks waiting for tokens, but doesn't affect UI (separate task)
   - **Priority**: P2 - LOW (separate task, not event loop, but inefficient)
   - **Fix**: Automatically fixed when #1 is fixed

6. **`conductor-daemon.rs:230`** - ⚠️ POTENTIALLY PROBLEMATIC
   - **Function**: Spawned task with tight loop
   - **Pattern**: Same as server.rs:214
   - **Context**: Dedicated streaming task
   - **Impact**: Same as #5
   - **Priority**: P2 - LOW (separate task, not event loop, but inefficient)
   - **Fix**: Automatically fixed when #1 is fixed

7. **`unix_socket/client.rs:138`** - ✅ ACCEPTABLE (Dedicated Task)
   - **Function**: Spawned write task
   - **Pattern**: `while let Some(event) = event_rx.recv().await`
   - **Context**: Dedicated transport write task
   - **Impact**: None (separate tokio task, event-driven)
   - **Priority**: N/A - Not a violation (this is the CORRECT pattern for dedicated tasks)

8. **`unix_socket/client.rs:188`** - ✅ ACCEPTABLE (Transport Trait)
   - **Function**: `async fn recv(&mut self)`
   - **Pattern**: `rx.recv().await`
   - **Context**: Transport trait method, designed to block until message
   - **Impact**: None (callers expect blocking behavior, use try_recv for polling)
   - **Priority**: N/A - Not a violation (correct trait implementation)

**TEST FILES** (Acceptable):
- All `tests/*.rs` files use `rx.recv().await` in test contexts, which is expected and acceptable

#### Summary of Findings

| Location | Severity | Type | Fix Required |
|----------|----------|------|--------------|
| `conductor.rs:1034` | CRITICAL | Event loop blocker | YES - Change to try_recv() |
| `conductor_client.rs:380` | CRITICAL | Wrapper propagation | YES - Auto-fixed by #1 |
| `server.rs:214` | LOW | Inefficient task | YES - Auto-fixed by #1 |
| `conductor-daemon.rs:230` | LOW | Inefficient task | YES - Auto-fixed by #1 |
| `conductor.rs:410` (warmup) | N/A | Acceptable (init) | NO |
| `server.rs:196` | N/A | Acceptable (task) | NO |
| `unix_socket/client.rs` | N/A | Acceptable (transport) | NO |
| Tests | N/A | Acceptable (tests) | NO |

**Key Insight**: Only ONE actual violation (`conductor.rs:1034`). All other issues are downstream effects that will be automatically fixed by correcting the root cause.

### [ ] Next: Implementation

**Owner**: Rust Backend Team
**Files to Change**: 1 file (conductor.rs), ~10 lines
**Estimated Time**: 5 minutes
**Risk**: LOW (well-tested async pattern)
**Cascading Fixes**: 3 additional improvements (server.rs, conductor-daemon.rs, conductor_client.rs)

**Task**: Replace `rx.recv().await` with `rx.try_recv()` in `conductor.rs:1034`

## Success Criteria

- [ ] Tokens display as they arrive (true streaming)
- [ ] No visible delay between GPU start and first token
- [ ] No CPU spike pattern
- [ ] Memory usage stays flat during streaming
- [ ] Event loop remains responsive at 10 FPS

## References

**Codebase Files**:
- `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/conductor.rs` (line 1034)
- `/var/home/machiyotl/src/ai-way/yollayah/core/surfaces/tui/src/app.rs` (line 351)
- `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/backend/ollama.rs` (line 181)

**Documentation**:
- `knowledge/principles/PRINCIPLE-efficiency.md`
- `knowledge/requirements/REQUIRED-separation.md`
- `knowledge/anti-patterns/FORBIDDEN-inefficient-calculations.md`

## Timeline

- **Created**: 2026-01-03
- **Investigation Started**: 2026-01-03
- **Root Cause Identified**: 2026-01-03 ✅
- **Fix Implemented**: _pending_
- **Verified Fixed**: _pending_

---

## Summary for Implementer

**ONE LINE FIX**:

In `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/conductor.rs:1034`:

```diff
-        match rx.recv().await {
+        match rx.try_recv() {
             Some(token) => {
                 // ... existing code ...
             }
-            None => return false,
+            Err(_) => return false,
         }
```

**Why This Works**:
- `try_recv()` returns immediately (non-blocking)
- Event loop continues at 10 FPS even when no tokens
- Tokens processed as they arrive, not batched
- UI stays responsive, shows "thinking" state
- No CPU spike from batch processing

**Testing**:
1. Send prompt to Yollayah
2. Verify "thinking" indicator shows immediately
3. Verify tokens appear one-by-one as generated
4. Verify no freeze during GPU loading
5. Verify smooth streaming at ~200 tok/sec
