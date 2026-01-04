# TODO-EPIC-Conductor-Reactive-Overhaul

**Status**: 🔴 CRITICAL - Active Investigation
**Created**: 2026-01-03
**Priority**: P0 - BLOCKING ALL USAGE
**Owner**: Rust Specialist, Async Expert, Hacker, Architect (REQUIRED APPROVAL)
**Policy**: ZERO TOLERANCE - No shortcuts, no hacky async

---

## Navigation

**Parent**: [TODO-main.md](TODO-main.md)
**Siblings**: None (highest priority)
**Children**:
- [TODO-STORY-conductor-warmup-elimination.md](TODO-STORY-conductor-warmup-elimination.md)
- [TODO-STORY-conductor-timeout-optimization.md](TODO-STORY-conductor-timeout-optimization.md)
- [TODO-STORY-conductor-block-on-fix.md](TODO-STORY-conductor-block-on-fix.md)

**Related**:
- BUG: [yollayah/conductor/TODO-BUG-conductor-slow-ollama-calls.md](../yollayah/conductor/TODO-BUG-conductor-slow-ollama-calls.md)
- Audit: [facts/conductor-blocking-audit.md](../facts/conductor-blocking-audit.md)

---

## Problem Statement

**Conductor is extremely slow when calling Ollama, making entire application unusable.**

### Evidence
- ✅ Direct Ollama CLI: Near-instant, GPU utilized
- ❌ Via conductor (bash/TUI): Multi-second delays
- ❌ Both surfaces affected → Conductor is bottleneck

### Root Causes Identified

1. **Warmup Overhead** (PRIMARY SUSPECT)
   - Sends actual LLM request during initialization
   - Uses blocking `rx.recv().await` at line 410
   - Blocks entire startup until warmup completes

2. **Greeting Generation** (SECONDARY)
   - Another LLM call during startup (line 438-469)
   - Adds more delay before user interaction

3. **HTTP Timeout Too Long**
   - 120-second timeout configured (line 43)
   - Could cause long waits on network issues

4. **One block_on() Call**
   - Location: `unix_socket/server.rs:330`
   - Not main issue but violates principles

---

## Team Analysis

### Rust Specialist

**Assessment**: Code structure is sound, but initialization is problematic

**Findings**:
- Async patterns mostly correct
- Warmup function blocks during init (line 401-431)
- Greeting generation adds more overhead
- `block_on()` at server.rs:330 is technical debt

**Recommendation**: Eliminate warmup, make greeting optional/lazy

### Async Expert

**Assessment**: Not a traditional blocking issue - it's initialization overhead

**Findings**:
- HTTP client uses async reqwest correctly
- Streaming properly spawned with tokio
- Problem: Sequential LLM calls during startup (warmup → greeting)
- Each call waits for completion before proceeding

**Recommendation**:
- Remove warmup entirely (modern Ollama keeps models loaded)
- Make greeting async/background
- Verify timeout configuration

### Hacker (Security)

**Assessment**: Security implications of warmup/timeout

**Findings**:
- 120s timeout creates DoS vector (slow responses lock conductor)
- Warmup sends predictable prompt (fingerprinting risk)
- Greeting reveals system state

**Recommendation**:
- Reduce timeout to 30s with exponential backoff
- Eliminate warmup (security + performance win)
- Defer greeting until after first user message

### Architect

**Assessment**: Architectural design flaw - eager initialization

**Design Issues**:
1. **Eager warmup**: Should be lazy (on first use)
2. **Sequential init**: Should be parallel/background
3. **Unnecessary overhead**: Ollama already keeps models loaded with keep_alive

**Principles Violated**:
- ⚠️ PRINCIPLE-efficiency.md: "Lazy Initialization, Aggressive Caching"
- ⚠️ Warmup defeats the purpose (Ollama has its own keep_alive)

**Recommendation**:
- ELIMINATE warmup entirely
- Make greeting optional (config flag)
- Trust Ollama's keep_alive mechanism

---

## Implementation Plan

### Story 1: Eliminate Warmup (CRITICAL)

**File**: `TODO-STORY-conductor-warmup-elimination.md`

**Changes**:
- Remove `warmup()` function (line 401-431)
- Remove `warmup_complete` field
- Remove warmup state from `ConductorState`
- Remove warmup call from initialization

**Testing**:
- Verify startup time dramatically reduced
- Verify first message works without warmup
- Compare with Ollama CLI performance

**Success Criteria**:
- Conductor startup < 1 second
- First message latency matches Ollama CLI

### Story 2: Optimize HTTP Timeouts

**File**: `TODO-STORY-conductor-timeout-optimization.md`

**Changes**:
- Reduce HTTP timeout from 120s to 30s (line 43)
- Add per-request timeout override
- Add exponential backoff for retries

**Testing**:
- Test with network delays
- Verify timeout handling
- Ensure error messages are clear

### Story 3: Fix block_on() Violation

**File**: `TODO-STORY-conductor-block-on-fix.md`

**Changes**:
- Replace `block_on()` at server.rs:330 with async method
- Verify server methods are fully async

**Testing**:
- Verify server functionality unchanged
- Confirm no blocking in async context

### Story 4: Make Greeting Optional (OPTIONAL)

**File**: `TODO-STORY-conductor-greeting-optional.md`

**Changes**:
- Add config flag `enable_greeting: bool`
- Default to `false` (disabled)
- Generate greeting lazily if enabled

**Rationale**: Let users opt-in to greeting overhead

---

## Success Criteria

**Performance**:
- [ ] Conductor startup < 1 second
- [ ] First message latency matches Ollama CLI (near-instant)
- [ ] GPU utilization same as direct Ollama
- [ ] No multi-second delays

**Architectural**:
- [ ] Zero blocking calls in production code
- [ ] All async patterns follow Tokio best practices
- [ ] Lazy initialization properly implemented
- [ ] No principle violations

**Approval**:
- [ ] Rust Specialist: Code review approved
- [ ] Async Expert: Async patterns verified
- [ ] Hacker: Security review passed
- [ ] Architect: Design approved

---

## Testing Plan

### Performance Testing
```bash
# Measure startup time
time ./yollayah.sh --test-interactive

# Measure first message latency
./yollayah.sh --interactive
# (time first response)

# Compare with direct Ollama
time ollama run llama3.1:8b "test"
```

### Regression Testing
- All existing tests must pass
- Integration tests verify conductor works
- No new warnings introduced

---

## References

**Principles**:
- `knowledge/principles/PRINCIPLE-efficiency.md` - Lazy initialization, no blocking
- `knowledge/anti-patterns/FORBIDDEN-inefficient-calculations.md` - What to avoid

**Facts**:
- `facts/conductor-blocking-audit.md` - Detailed audit findings

**Related Work**:
- `progress/DONE-BUG-015-sleep-in-polling-loops.md` - Similar async fixes

---

## Timeline

**Phase 1**: Story 1 (Warmup Elimination) - IMMEDIATE
**Phase 2**: Story 2 (Timeout Optimization) - IMMEDIATE
**Phase 3**: Story 3 (block_on fix) - HIGH
**Phase 4**: Story 4 (Greeting optional) - MEDIUM

**Target**: All critical issues fixed today (2026-01-03)

---

## Notes

**ZERO TOLERANCE POLICY IN EFFECT**

This is core infrastructure. No shortcuts. No "good enough for now".
Get it right or don't ship it.

All changes require:
1. Full team approval
2. Tests passing
3. Performance verified
4. Documentation updated
5. QA verification before DONE
