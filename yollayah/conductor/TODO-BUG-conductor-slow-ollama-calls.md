# TODO-BUG-Conductor-Slow-Ollama-Calls

**Status**: 🔴 CRITICAL - Zero Tolerance Policy
**Created**: 2026-01-03
**Priority**: P0 - BLOCKING ALL USAGE
**Severity**: CRITICAL - Core component completely broken
**Team**: Rust Specialist, Async Expert, Hacker, Architect (required approval)

---

## Navigation

**Parent**: [TODO-EPIC-conductor-reactive-overhaul.md](../../progress/TODO-EPIC-conductor-reactive-overhaul.md)
**Siblings**: None yet
**Children**: TBD (Stories to be created)
**QA Verification**: TBD (after fix)

---

## Problem Statement

**The Conductor is extremely slow when calling Ollama, making the entire application unusable.**

### Symptoms

1. **Bash prompt mode**: Extremely slow responses (multi-second delays)
2. **TUI mode**: Also slow (not a TUI issue - it's the conductor)
3. **Direct Ollama CLI**: Near-instant responses, fully utilizing GPU

### Evidence

```bash
# Direct Ollama - FAST (GPU utilized)
ollama run llama3.1:8b "why is the sky blue? explain using dr seus style haikus"
# Response: Near-instant, GPU active

# Via yollayah.sh --interactive - SLOW
# Same prompt takes many seconds
# GPU may not be properly utilized
```

### Root Cause Hypothesis

**BLOCKING CALLS in Conductor** - Violating async/reactive principles

Suspected anti-patterns:
- `.wait()` or `.get()` calls blocking threads
- Synchronous HTTP calls to Ollama
- Thread pool exhaustion
- Improper async/await usage
- Blocking I/O in async context

---

## Zero Tolerance Policy

**FORBIDDEN patterns** (as per PRINCIPLE-efficiency.md):

| ❌ FORBIDDEN | ✅ REQUIRED |
|--------------|------------|
| `std::thread::sleep()` | `tokio::time::sleep()` (only for backoff/frame limiting) |
| `.wait()` on futures | `.await` in async context |
| `.get()` on futures | `.await` with proper error handling |
| `block_on()` in async | Native async all the way |
| Sync HTTP (`reqwest::blocking`) | Async HTTP (`reqwest`) |
| `std::fs::*` | `tokio::fs::*` |
| Thread pool blocking | Proper async runtime usage |

---

## Impact

**User Experience**: BROKEN
- All user-facing modes unusable (TUI, bash prompt)
- Direct Ollama works fine → Problem is 100% in Conductor
- This is a **production blocker**

**Principle Violations**:
- ⚠️ PRINCIPLE-efficiency.md - Multiple violations suspected
- ⚠️ Async best practices - Blocking in async context

---

## Investigation Required

### Phase 1: Blocking Call Audit (URGENT)
**Team**: Rust + Async Expert + Hacker
**Timeline**: Immediate

Scan entire conductor codebase for:
1. `grep -r "\.wait()" yollayah/conductor/`
2. `grep -r "\.get()" yollayah/conductor/` (on futures)
3. `grep -r "block_on" yollayah/conductor/`
4. `grep -r "std::thread::sleep" yollayah/conductor/`
5. `grep -r "reqwest::blocking" yollayah/conductor/`
6. `grep -r "std::fs::" yollayah/conductor/`
7. Review Ollama API call implementation

### Phase 2: Reactive Architecture Review
**Team**: Architect + Rust Specialist
**Timeline**: After Phase 1

Verify conductor uses:
- ✅ Reactive streams (rxrust)
- ✅ Async futures/promises
- ✅ Tokio runtime for thread pooling
- ✅ Non-blocking I/O everywhere

### Phase 3: Ollama Integration Deep Dive
**Team**: Backend + Async Expert
**Timeline**: Concurrent with Phase 1-2

Questions:
1. How does conductor call Ollama?
2. Is it using reqwest async correctly?
3. Are responses streamed reactively?
4. Is there channel blocking?
5. Is GPU being utilized properly?

---

## Success Criteria

- [ ] Conductor responds as fast as direct Ollama CLI
- [ ] GPU utilization matches direct Ollama usage
- [ ] Zero blocking calls in production code paths
- [ ] All async patterns follow Tokio best practices
- [ ] Reactive streams properly implemented
- [ ] Full team approval (Rust, Async, Hacker, Architect)

---

## Related Work

**Will Create**:
- `TODO-EPIC-conductor-reactive-overhaul.md` - Epic tracking full overhaul
- `facts/conductor-architecture.md` - Documented architecture
- Stories for each blocking anti-pattern fix

**References**:
- `knowledge/principles/PRINCIPLE-efficiency.md` - Zero tolerance policy
- `knowledge/anti-patterns/FORBIDDEN-inefficient-calculations.md` - What not to do
- `progress/DONE-BUG-015-sleep-in-polling-loops.md` - Similar issue (resolved)

---

## Next Steps

1. **NOW**: Scan conductor for blocking calls (grep audit)
2. **NOW**: Create EPIC-conductor-reactive-overhaul.md
3. **NOW**: Document current conductor architecture in facts/
4. **THEN**: Create stories for each violation found
5. **THEN**: Get team approval on fix plan
6. **THEN**: Implement fixes (zero tolerance - no shortcuts)
7. **THEN**: QA verification with performance testing

---

**This is a BLOCKING ISSUE. All other work is secondary until conductor performance is fixed.**
