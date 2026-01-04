# Conductor Blocking Call Audit

**Date**: 2026-01-03
**Auditors**: Rust Specialist, Async Expert, Hacker
**Scope**: yollayah/conductor/ codebase
**Policy**: ZERO TOLERANCE for blocking calls in production code

---

## Summary

**FINDINGS**: Limited blocking violations found, but critical investigation needed

---

## Detailed Findings

### 1. block_on() Calls

**Location**: `core/src/transport/unix_socket/server.rs:330`
```rust
h.block_on(async { self.connections.read().await.keys().cloned().collect() })
```

**Severity**: 🟡 MEDIUM
**Context**: Server connection listing
**Analysis**: Needs review - blocking call in server context
**Action Required**: YES - Replace with async method

### 2. std::thread::sleep() Calls

**All occurrences**: Test code only (11 instances)
- `core/src/avatar/cache.rs` (3 tests)
- `core/src/routing/health.rs` (3 tests)
- `core/src/transport/heartbeat.rs` (3 tests)
- `core/src/transport/rate_limit.rs` (2 tests)

**Severity**: ✅ ACCEPTABLE
**Context**: Test synchronization
**Action Required**: NO - Test code acceptable

### 3. Warmup Function Blocking

**Location**: `core/src/conductor.rs:410`
```rust
while let Some(token) = rx.recv().await {
    // Process warmup tokens
}
```

**Severity**: 🟡 MEDIUM (Potentially)
**Context**: Warmup function during initialization
**Analysis**: Blocking recv in warmup - could delay startup
**Action Required**: INVESTIGATE - Check where warmup is called

---

## Anti-Patterns NOT Found (Good!)

✅ No `reqwest::blocking::*` usage
✅ No `std::fs::*` usage (would be `tokio::fs`)
✅ No `.wait()` on futures (except barrier in tests)
✅ No `.get()` blocking calls

---

## Areas Requiring Investigation

### 1. Ollama Backend HTTP Calls

**File**: `core/src/backend/ollama.rs`
**Status**: ✅ LOOKS GOOD
- Using async reqwest properly
- Streaming via tokio spawn
- Non-blocking tokio channels

**BUT**: Need to verify timeout settings (line 43: 120 second timeout!)

### 2. Conductor Initialization

**File**: `core/src/conductor.rs`
**Concerns**:
1. When is `warmup()` called? (line 401-431)
2. Is it blocking startup?
3. Does greeting generation block? (line 438-469)

**Action Required**: Trace startup path

### 3. Router Integration

**File**: Referenced in conductor.rs:914-980
**Status**: NEEDS REVIEW
- QueryRouter health checks (line 915)
- Router.route() calls (line 939)

**Questions**:
- Is router.is_healthy() blocking?
- Is router.route() truly async?

---

## Critical Questions for Team

### For Rust Specialist:
1. Is the block_on() at unix_socket/server.rs:330 acceptable?
2. Should warmup() be redesigned as non-blocking?

### For Async Expert:
1. Review HTTP timeout (120s) - is this causing delays?
2. Verify reqwest async usage is optimal
3. Check if router integration is truly async

### For Hacker:
1. Security implications of 120s HTTP timeout?
2. Any DoS vectors in warmup/greeting?

### For Architect:
1. Should warmup be eliminated entirely?
2. Should conductor startup be lazy/on-demand?
3. Is the router integration design sound?

---

## Hypothesis: Slowness Not From Blocking

**Key Observation**: Very few blocking violations found

**Alternative Causes to Investigate**:
1. **HTTP Timeout Too Long**: 120s timeout might cause long waits on errors
2. **Warmup Overhead**: Warmup sends actual LLM request during init
3. **Greeting Generation**: Another LLM call during startup
4. **Router Health Checks**: Might be slow/failing
5. **Configuration Issues**: Wrong Ollama host/port?

---

## Recommended Next Steps

1. **TRACE STARTUP PATH**: Find where warmup() is called from
2. **TEST WITHOUT WARMUP**: Disable warmup, measure performance
3. **CHECK OLLAMA CONNECTION**: Verify host:port configuration
4. **PROFILE**: Use tokio-console or similar to find actual bottleneck
5. **REVIEW ROUTER**: Deep dive into QueryRouter implementation

---

## Verdict

**Blocking violations**: MINIMAL (1 production issue)
**Async usage**: MOSTLY CORRECT
**Real problem**: LIKELY ELSEWHERE

**Recommendation**: Continue investigation focusing on:
- Initialization/warmup overhead
- HTTP configuration
- Router integration
- Actual runtime profiling

---

**This audit does NOT explain the reported slowness. Deeper investigation required.**
