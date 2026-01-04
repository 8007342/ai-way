# ANALYSIS: Blocking Await in Event Loops - Codebase Triage

**Date**: 2026-01-03
**Team**: Architect + Hacker
**Scope**: Complete ai-way codebase scan for PRINCIPLE-efficiency.md violations
**Focus**: "Blocking Await in Event Loops" anti-pattern

---

## Executive Summary

**Comprehensive codebase audit completed.** Found **ONE critical violation** that causes TUI to freeze during streaming.

**Root Cause**: `conductor.rs:1034` uses `rx.recv().await` in a polling function called from event loops.

**Impact**: TUI freezes for 2-5 seconds while GPU loads model, then dumps all text at once (defeats streaming).

**Fix**: Change ONE line (`rx.recv().await` → `rx.try_recv()`) fixes 4 locations automatically.

**Risk**: LOW - Well-understood async pattern, minimal code change.

---

## What We Searched For

### Search Patterns

1. **Async polling functions**
   - Functions named `poll_*`, `check_*` that are `async fn`
   - Contains `.recv().await`, `.send().await`, or other blocking await operations

2. **Event loop call sites**
   - `while self.running` patterns
   - `loop { ... }` with event processing
   - `tokio::select!` branches

3. **Blocking await patterns**
   - Any `.await` calls inside functions called repeatedly from tight loops
   - `rx.recv().await` outside of `tokio::select!` or dedicated tasks

### Files Checked

- **Conductor**: `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/conductor.rs`
- **TUI**: `/var/home/machiyotl/src/ai-way/yollayah/core/surfaces/tui/src/app.rs`
- **TUI Client**: `/var/home/machiyotl/src/ai-way/yollayah/core/surfaces/tui/src/conductor_client.rs`
- **Daemon Server**: `/var/home/machiyotl/src/ai-way/yollayah/conductor/daemon/src/server.rs`
- **Daemon Binary**: `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/bin/conductor-daemon.rs`
- **Transports**: `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/transport/unix_socket/`
- **All Rust files** with `.recv().await`, `.send().await`, `tokio::spawn`, etc.

---

## Violations Found

### CRITICAL: Event Loop Blockers (P0)

#### 1. `conductor.rs:1034` - PRIMARY VIOLATION

**Location**: `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/conductor.rs:1022-1065`

**Function**:
```rust
pub async fn poll_streaming(&mut self) -> bool
```

**Violation**:
```rust
// Line 1034: BLOCKS until token arrives (can be 2-5 seconds!)
match rx.recv().await {
    Some(token) => { /* ... */ }
    None => return false,
}
```

**Called From**:
- **TUI event loop**: `app.rs:351` - `self.conductor.poll_streaming().await;`
- **Daemon loop**: `server.rs:214` - `c.poll_streaming().await;`
- **Daemon binary**: `conductor-daemon.rs:230` - `conductor.poll_streaming().await;`

**Impact**:
1. TUI event loop calls this every frame (100ms)
2. `rx.recv().await` blocks until first token arrives
3. GPU loads model (2-5 seconds) before first token
4. **TUI FROZEN** - no rendering, no updates, appears hung
5. First token arrives → ALL buffered tokens drain at once
6. CPU spike from batch processing instead of gradual streaming
7. User sees: freeze → sudden text dump (broken streaming)

**Fix**:
```rust
// Change to non-blocking
match rx.try_recv() {
    Ok(token) => { /* ... */ }
    Err(_) => return false,  // No tokens yet, keep UI responsive
}
```

**Priority**: **P0 - CRITICAL**

---

#### 2. `conductor_client.rs:380` - WRAPPER PROPAGATION

**Location**: `/var/home/machiyotl/src/ai-way/yollayah/core/surfaces/tui/src/conductor_client.rs:380-389`

**Function**:
```rust
pub async fn poll_streaming(&mut self) -> bool
```

**Violation**:
```rust
// Line 382: Forwards blocking call from conductor.rs:1034
ClientMode::InProcess { conductor, .. } => conductor.poll_streaming().await,
```

**Impact**:
- Propagates blocking behavior from `conductor.rs:1034`
- TUI calls this wrapper, which calls the blocking function

**Fix**:
- Automatically fixed when `conductor.rs:1034` is fixed (just forwards call)

**Priority**: **P0 - CRITICAL** (auto-fixed by #1)

---

### LOW PRIORITY: Inefficient Tasks (P2)

#### 3. `server.rs:214` - TIGHT LOOP WITH BLOCKING POLL

**Location**: `/var/home/machiyotl/src/ai-way/yollayah/conductor/daemon/src/server.rs:206-219`

**Violation**:
```rust
tokio::spawn(async move {
    loop {
        {
            let mut c = conductor_for_streaming.lock().await;
            // Calls blocking poll_streaming()
            c.poll_streaming().await;  // ← BLOCKS HERE
        }
        tokio::task::yield_now().await;
    }
});
```

**Impact**:
- Dedicated tokio task (NOT event loop), so doesn't freeze UI
- But inefficient: blocks task waiting for tokens instead of event-driven
- Task sits idle burning CPU with yield_now() when no streaming

**Fix**:
- Automatically fixed when `conductor.rs:1034` is fixed
- After fix, will return immediately when no tokens

**Priority**: **P2 - LOW** (separate task, not event loop, auto-fixed by #1)

---

#### 4. `conductor-daemon.rs:230` - SAME AS #3

**Location**: `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/bin/conductor-daemon.rs:222-235`

**Violation**:
```rust
// Same pattern as server.rs:214
tokio::spawn(async move {
    loop {
        let mut conductor = conductor_for_streaming.lock().await;
        conductor.poll_streaming().await;  // ← BLOCKS HERE
        tokio::task::yield_now().await;
    }
});
```

**Impact**: Same as #3

**Fix**: Automatically fixed when `conductor.rs:1034` is fixed

**Priority**: **P2 - LOW** (separate task, not event loop, auto-fixed by #1)

---

### ACCEPTABLE: Not Violations

#### 5. `conductor.rs:410` - WARMUP (INIT PHASE)

**Location**: `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/conductor.rs:400-431`

**Function**:
```rust
async fn warmup(&mut self) -> anyhow::Result<()>
```

**Pattern**:
```rust
// Line 410: Drains warmup response
while let Some(token) = rx.recv().await {
    match token {
        StreamingToken::Complete { .. } => break,
        StreamingToken::Error(e) => { /* ... */ }
        _ => {}
    }
}
```

**Why Acceptable**:
- Runs **once** during initialization, before event loop starts
- NOT called from event loop (happens in startup sequence)
- Blocking is expected and desired (wait for warmup to complete)
- User sees "Warming up..." status during this time

**Priority**: **N/A - Not a violation**

---

#### 6. `server.rs:196` - EVENT PROCESSING TASK

**Location**: `/var/home/machiyotl/src/ai-way/yollayah/conductor/daemon/src/server.rs:193-204`

**Pattern**:
```rust
tokio::spawn(async move {
    while let Some((conn_id, event)) = event_rx.recv().await {
        debug!(conn_id = %conn_id, event = ?event, "Processing event");
        let mut c = conductor_for_events.lock().await;
        if let Err(e) = c.handle_event_from(conn_id, event).await {
            warn!(conn_id = %conn_id, error = %e, "Failed to handle event");
        }
    }
});
```

**Why Acceptable**:
- **Dedicated background task** (separate from event loop)
- **Event-driven pattern** - blocks waiting for events (correct!)
- This IS the correct pattern for message processing tasks
- Not a polling function, not called from event loop

**Priority**: **N/A - Not a violation** (this is the CORRECT pattern)

---

#### 7. `unix_socket/client.rs:138` - WRITE TASK

**Location**: `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/transport/unix_socket/client.rs:135-150`

**Pattern**:
```rust
tokio::spawn(async move {
    while let Some(event) = event_rx.recv().await {
        match encode(&event) {
            Ok(data) => {
                if let Err(e) = write_half.write_all(&data).await {
                    tracing::warn!(error = %e, "Write error");
                    break;
                }
            }
            Err(e) => { /* ... */ }
        }
    }
});
```

**Why Acceptable**:
- **Dedicated transport write task**
- **Event-driven** - waits for events to send (correct!)
- This is the standard pattern for transport layers

**Priority**: **N/A - Not a violation** (correct async pattern)

---

#### 8. `unix_socket/client.rs:188` - TRANSPORT TRAIT

**Location**: `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/transport/unix_socket/client.rs:186-192`

**Function**:
```rust
async fn recv(&mut self) -> Result<ConductorMessage, TransportError>
```

**Pattern**:
```rust
// Line 188
rx.recv().await.ok_or(TransportError::ConnectionClosed)
```

**Why Acceptable**:
- **Transport trait implementation** - designed to block until message
- Callers know this is blocking (use `try_recv()` for polling)
- Separate from event loop usage
- This is the correct trait implementation

**Priority**: **N/A - Not a violation** (correct trait design)

---

#### 9. Test Files - ALL ACCEPTABLE

**Locations**: All files in `tests/` directories

**Pattern**:
- Tests use `rx.recv().await` to wait for expected messages
- Test context is NOT production code
- Blocking in tests is expected and acceptable

**Priority**: **N/A - Not a violation** (test code)

---

## Summary Table

| # | Location | Severity | Type | Fix Required | Auto-Fixed by #1 |
|---|----------|----------|------|--------------|------------------|
| 1 | `conductor.rs:1034` | **CRITICAL** | Event loop blocker | **YES** - Change to try_recv() | N/A (root cause) |
| 2 | `conductor_client.rs:380` | **CRITICAL** | Wrapper propagation | **YES** | ✅ Yes |
| 3 | `server.rs:214` | LOW | Inefficient task | **YES** | ✅ Yes |
| 4 | `conductor-daemon.rs:230` | LOW | Inefficient task | **YES** | ✅ Yes |
| 5 | `conductor.rs:410` (warmup) | N/A | Acceptable (init) | NO | N/A |
| 6 | `server.rs:196` | N/A | Acceptable (task) | NO | N/A |
| 7 | `unix_socket/client.rs:138` | N/A | Acceptable (transport) | NO | N/A |
| 8 | `unix_socket/client.rs:188` | N/A | Acceptable (trait) | NO | N/A |
| 9 | Tests | N/A | Acceptable (tests) | NO | N/A |

---

## Key Insights

### 1. Single Point of Failure

**Only ONE actual violation**: `conductor.rs:1034`

All other issues are either:
- Downstream effects (auto-fixed when root cause is fixed)
- Acceptable patterns (dedicated tasks, init code, transport traits)

### 2. Cascading Fix

Fixing `conductor.rs:1034` automatically improves 3 other locations:
1. `conductor_client.rs:380` - wrapper propagates the fix
2. `server.rs:214` - spawned task becomes efficient
3. `conductor-daemon.rs:230` - spawned task becomes efficient

### 3. Architecture is Mostly Correct

The codebase follows async patterns correctly in most places:
- Dedicated tasks use `rx.recv().await` (correct!)
- Transport traits implement blocking recv (correct!)
- Event loops use `tokio::select!` (correct!)

**The violation is localized to ONE function that's misnamed.**

### 4. Naming is Misleading

`poll_streaming()` suggests polling (non-blocking), but it actually blocks!

**After fix**, the name will match behavior:
- `poll_streaming()` → returns immediately if no tokens
- True polling pattern (check and return)

---

## Recommended Fix

### The ONE Line Change

**File**: `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/conductor.rs`

**Line**: 1034

**Before**:
```rust
match rx.recv().await {
    Some(token) => {
        // ... existing code ...
    }
    None => return false,
}
```

**After**:
```rust
match rx.try_recv() {
    Ok(token) => {
        // ... existing code (no changes!) ...
    }
    Err(_) => return false,
}
```

**Changes Required**:
1. `rx.recv().await` → `rx.try_recv()`
2. `Some(token)` → `Ok(token)`
3. `None` → `Err(_)`

**Also update line 1046** (already correct, but verify):
```rust
// This is already try_recv() - verify it stays that way
while let Ok(token) = rx.try_recv() {
    // ...
}
```

### Why This Works

**Before** (blocking):
1. Event loop calls `poll_streaming()`
2. `rx.recv().await` blocks until token arrives
3. Event loop frozen (no rendering, no updates)
4. Tokens arrive → all drain at once
5. CPU spike from batch processing

**After** (non-blocking):
1. Event loop calls `poll_streaming()`
2. `rx.try_recv()` returns immediately
3. If tokens available → process them
4. If no tokens → return false, continue event loop
5. UI stays responsive, shows "thinking" state
6. Tokens processed gradually as they arrive

### Testing Plan

**Before Fix** (current broken behavior):
1. Send prompt to Yollayah
2. TUI freezes (no cursor, no updates)
3. Wait 2-5 seconds (GPU loading)
4. All text dumps at once
5. CPU spike visible in htop

**After Fix** (expected behavior):
1. Send prompt to Yollayah
2. "Thinking" indicator appears immediately
3. UI remains responsive (cursor blinks, can scroll)
4. Tokens appear one-by-one as generated (~200/sec)
5. Smooth streaming, no freeze, no CPU spike

---

## Priority and Risk Assessment

### Priority

**P0 - CRITICAL**: This is a user-facing bug that makes the TUI appear broken.

- User experience is severely degraded
- Defeats the purpose of streaming interface
- Makes ai-way look unresponsive and buggy

### Risk Level

**LOW RISK**: Well-understood async pattern, minimal code change.

**Why Low Risk**:
1. **Small change**: ONE line in ONE function
2. **Well-tested pattern**: `try_recv()` is standard Rust async pattern
3. **Isolated**: No changes to calling code needed
4. **Backward compatible**: Return type and behavior contract unchanged
5. **Easy to verify**: User-visible improvement, easy to test

**Potential Issues** (mitigations):
1. ~~Tests might expect blocking behavior~~
   - Tests call `poll_streaming()` in loops already, will adapt naturally
   - No test changes needed
2. ~~Callers might rely on blocking~~
   - All callers are in event loops, WANT non-blocking behavior
   - This fix is what they need!

### Estimated Effort

- **Code Change**: 5 minutes
- **Testing**: 10 minutes (manual TUI test)
- **Total**: 15 minutes

---

## Next Steps

### Immediate Actions

1. ✅ **Investigation Complete** - This document
2. **[ ] Implement Fix** - Change `conductor.rs:1034`
3. **[ ] Manual Test** - Verify streaming works smoothly
4. **[ ] Run Test Suite** - Ensure no regressions
5. **[ ] Update TODO-BUG-001** - Mark as fixed

### Follow-Up (Optional)

**Consider refactoring for even better architecture**:

Move token reception into `tokio::select!` in event loop:
```rust
// Instead of calling poll_streaming()
tokio::select! {
    event = event_stream.next() => { /* ... */ }
    token = self.conductor.recv_token() => { /* ... */ }  // Direct channel
    _ = tick.tick() => { /* ... */ }
}
```

**Benefits**:
- Even cleaner architecture
- True event-driven (not polling at all)
- No function call overhead

**Tradeoffs**:
- More invasive change (refactor event loop)
- Current fix is simpler and sufficient

**Recommendation**: Do the ONE line fix now, consider select! refactor later if needed.

---

## References

**Bug Tracking**:
- `progress/TODO-BUG-001-tui-waits-for-full-stream.md` - Primary bug tracker (updated with this analysis)

**Principles**:
- `knowledge/principles/PRINCIPLE-efficiency.md` - Defines the anti-pattern (lines 312-477)

**Code Files**:
- `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/conductor.rs` (line 1034)
- `/var/home/machiyotl/src/ai-way/yollayah/core/surfaces/tui/src/app.rs` (line 351)
- `/var/home/machiyotl/src/ai-way/yollayah/core/surfaces/tui/src/conductor_client.rs` (line 380)
- `/var/home/machiyotl/src/ai-way/yollayah/conductor/daemon/src/server.rs` (line 214)
- `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/bin/conductor-daemon.rs` (line 230)

---

## Conclusion

**Comprehensive triage complete. Ready for fix.**

The codebase is **architecturally sound** with **ONE localized violation** that has cascading effects.

**Fix is simple, low-risk, and high-impact**: ONE line change fixes TUI freezing and improves 3 other locations automatically.

**Recommended action**: Implement the fix immediately (15 minutes total).

**Team confidence**: HIGH - This is exactly the kind of bug we want: well-understood, well-scoped, easy to fix.

---

**Document Status**: ✅ COMPLETE
**Next Action**: Implement fix in `conductor.rs:1034`
**Assigned To**: Rust Backend Team
**Expected Completion**: Today (5-15 minutes)
