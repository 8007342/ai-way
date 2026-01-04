# Sleep Call Audit - Preliminary Findings

**Status**: CRITICAL REVIEW IN PROGRESS
**Created**: 2026-01-04
**Context**: User questioned why `tokio::time::sleep()` exists in production code despite "No Sleep" principle

---

## Executive Summary

**Total sleep() calls found**: 100+ instances
**Production code calls**: 7 critical locations
**Test code calls**: 90+ instances (acceptable)

### Critical Finding

The architectural enforcement policy (integration_test_sleep_prohibition.rs) EXPLICITLY ALLOWS:
- ✅ Frame rate limiting in TUI using `tokio::time::sleep()`
- ✅ Exponential backoff in retry logic
- ✅ Test code

**However**: User is questioning whether this policy is CORRECT.

---

## Production Sleep Calls (Categorized)

### 1. Frame Rate Limiting (TUI) - POLICY SAYS ALLOWED

**app.rs:319** - Main frame tick in event loop:
```rust
_ = tokio::time::sleep(frame_duration) => {
    // Handle startup phases incrementally
}
```

**app.rs:376** - Additional frame rate limiting:
```rust
let elapsed = frame_start.elapsed();
if elapsed < frame_duration {
    tokio::time::sleep(frame_duration - elapsed).await;
}
```

**Policy Status**: EXPLICITLY ALLOWED by architectural enforcement test
**User Concern**: "WHYYYYYYYYY is there a call to tokio::time::sleep ?? Isn't that super ultra the fuck mega forbidden?"

**Alternative Pattern**: Research proposals recommend `tokio::time::Interval`:
```rust
let mut render_interval = tokio::time::interval(Duration::from_millis(50));
render_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

// In tokio::select!:
_ = render_interval.tick() => {
    if self.has_dirty_layers() {
        self.render(terminal)?;
    }
}
```

### 2. Exponential Backoff (Retry Logic) - POLICY SAYS ALLOWED

**router.rs:517** - Retry backoff:
```rust
// Backoff before retry
let backoff = retry_config.backoff_for_attempt(attempt);
tokio::time::sleep(backoff).await;
```

**Policy Status**: ALLOWED (exponential backoff pattern)

### 3. Rate Limiting (Backpressure) - POLICY SAYS ALLOWED

**rate_limit.rs:749** - Apply backpressure:
```rust
pub async fn apply_backpressure(result: &RateLimitResult) {
    if let RateLimitResult::Throttled { delay } = result {
        tokio::time::sleep(*delay).await;
    }
}
```

**Policy Status**: ALLOWED (rate limiting pattern)

### 4. Polling Loop (Test Code) - TEST CODE, OK

**conductor.rs:1863** - Polling in test:
```rust
// Wait for streaming to complete
for _ in 0..10 {
    if conductor.poll_streaming().await {
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    } else {
        break;
    }
}
```

**Context**: Inside test function (line 1855 shows "Test message")
**Policy Status**: ALLOWED in test code

---

## CRITICAL BLOCKING VIOLATIONS

### std::thread::sleep() in Production Code (FORBIDDEN!)

**animator.rs:830, 894, 899** - BLOCKING SLEEP IN SYNC CODE:
```rust
thread::sleep(Duration::from_millis(300));  // Line 830
thread::sleep(Duration::from_millis(125));  // Line 894
thread::sleep(Duration::from_millis(150));  // Line 899
```

**Status**: ⚠️ BLOCKING VIOLATION - Agent ab18c52 analyzing
**Impact**: If called from async context, blocks event loop
**Fix Required**: Determine if animator is async or sync context

---

## Test Code Sleep Calls (90+ instances)

All acceptable under policy:
- TUI integration tests (yollayah/core/surfaces/tui/tests/integration_test.rs) - 20+ instances
- TUI stress tests (stress_test.rs) - 15+ instances
- Conductor integration tests - 10+ instances
- Conductor chaos tests - 15+ instances
- Routing performance tests - 15+ instances
- Multi-model integration tests - 5+ instances
- Avatar cache tests - 3 instances
- Heartbeat tests - 5 instances
- Rate limit tests - 3 instances
- Stream manager tests - 12 instances

---

## Example Code (OK)

- reactive_prototype.rs:116
- hello_reactive.rs:42

---

## Current Architectural Policy (integration_test_sleep_prohibition.rs)

### Principle (Line 3):
```
This test enforces PRINCIPLE-efficiency.md Law 1: No Sleep, Only Wait on I/O
```

### Policy (Lines 5-6):
```
**Policy**: Production code in TUI and Conductor MUST NOT call sleep methods.
**Exceptions**: Frame rate limiting (TUI only), exponential backoff (retry logic only), test code
```

### Acceptable Uses (Lines 27-31):
```
✅ ACCEPTABLE sleep uses:
  - Frame rate limiting in TUI (tokio::time::sleep in frame control)
  - Exponential backoff in retry logic
  - Test code (#[test] or #[tokio::test] functions)
  - Periodic tasks using tokio::time::interval()
```

### Forbidden Uses (Lines 32-35):
```
❌ FORBIDDEN:
  - Sleep in polling loops
  - Sleep as poor man's synchronization
  - Sleep to 'wait' for events (use async I/O!)
```

---

## The Core Question

**User's Challenge**:
> "WHYYYYYYYYY is there a call to tokio::time::sleep ?? Isn't that super ultra the fuck mega forbidden?"

**The Disconnect**:
1. Current policy ALLOWS `tokio::time::sleep()` for frame limiting
2. Implementation uses `tokio::time::sleep()` for frame limiting (app.rs)
3. Research proposals recommend `tokio::time::Interval` instead
4. User is questioning whether the policy itself is WRONG

**Hypotheses**:

### Hypothesis A: Policy is Correct
- `tokio::time::sleep()` is acceptable for frame limiting
- It's async, non-blocking at OS level
- Implementation is fine as-is
- User's concern is misplaced

### Hypothesis B: Policy Needs Refinement
- Principle should distinguish:
  - `tokio::time::sleep()` (async, but not reactive)
  - `tokio::time::Interval` (event-driven, truly reactive)
- Frame limiting SHOULD use Interval, not sleep
- Current policy is too permissive

### Hypothesis C: Sleep is Fundamentally Wrong
- ANY use of sleep (even async) violates reactive principles
- Frame limiting should use Interval exclusively
- Backoff/rate limiting should use timers, not delays
- Policy needs complete rewrite

---

## Expert Team Analysis (IN PROGRESS)

### Agent adc633e: Tokio sleep vs Interval
**Status**: Running
**Task**: Research official Tokio docs, compare sleep() vs Interval semantics
**Output**: knowledge/anti-patterns/ASYNC-PATTERNS-sleep-vs-interval-PROPOSAL.md

### Agent a23c042: Architect Reactive Principles
**Status**: Running
**Task**: Validate "No Sleep, Only Wait on Async I/O" principle
**Output**: knowledge/principles/PRINCIPLE-async-sleep-policy-PROPOSAL.md

### Agent ab18c52: Animator Blocking Bugs
**Status**: Running
**Task**: Analyze std::thread::sleep() in animator.rs
**Output**: facts/BUG-animator-blocking-sleep-PROPOSAL.md

---

## Next Actions (Awaiting Expert Results)

1. ✅ **DONE**: Complete audit of all sleep() calls
2. ✅ **DONE**: Assemble expert team
3. ⏳ **ACTIVE**: Wait for expert findings
4. ⏳ **PENDING**: Triage decision:
   - Option A: Fix in sprints (rewrite app.rs to use Interval)
   - Option B: Rebuild from scratch (full reactive refactor)
5. ⏳ **PENDING**: Update architectural policy
6. ⏳ **PENDING**: Update TODO.md with action plan

---

## References

- `/var/home/machiyotl/src/ai-way/yollayah/tests/architectural-enforcement/tests/integration_test_sleep_prohibition.rs` - Current policy
- `/var/home/machiyotl/src/ai-way/yollayah/core/surfaces/tui/src/app.rs` - Frame limiting implementation
- `/var/home/machiyotl/src/ai-way/facts/REACTIVE-RENDERING-PATTERNS-PROPOSAL.md` - Recommends Interval
- `/var/home/machiyotl/src/ai-way/facts/TUI-RENDER-BLOCKING-ANALYSIS-PROPOSAL.md` - Render analysis
- `/var/home/machiyotl/src/ai-way/TODO.md` - Sprint tracking

---

**Next Update**: After expert agents complete analysis
