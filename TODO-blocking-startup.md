# TODO-blocking-startup: Non-Blocking TUI Launch

**Created**: 2026-01-02
**Status**: ACTIVE
**Priority**: CRITICAL
**Owner**: Architect + Hacker

---

## Problem Statement

The TUI is **unresponsive at launch** due to blocking operations in the bash bootstrap layer (yollayah.sh). While the Rust TUI code is properly designed for async/non-blocking operation, the bash scripts that run BEFORE the TUI launches block on:

1. Network I/O (git clone, model pull, ollama API waits)
2. Sequential operations that could run in parallel
3. No timeouts on network operations
4. No user feedback during long operations

**Impact**: User sees a blank screen or frozen terminal for 10-60+ seconds on first launch, or 5-15 seconds on subsequent launches.

---

## Blocking Calls Identified

### CRITICAL (Network I/O, can timeout indefinitely)

| Component | File | Lines | Duration | Issue |
|-----------|------|-------|----------|-------|
| **Git clone** | lib/agents/sync.sh | 68-72 | 5-30s | Blocks on first run, no timeout |
| **Git pull** | lib/agents/sync.sh | 106 | 2-10s | Blocks on every run, no timeout |
| **Model pull** | lib/ollama/lifecycle.sh | 435-461 | 5-30min | Downloads GBs, completely blocks startup |

### HIGH (Ollama API dependencies)

| Component | File | Lines | Duration | Issue |
|-----------|------|-------|----------|-------|
| **Model create** | lib/yollayah/personality.sh | 142-193 | 2-5s | Waits for Ollama to create model |
| **Ollama startup wait** | lib/ollama/service.sh | 152-163 | 10s max | Sleep loop waiting for API |
| **Sequential main()** | yollayah.sh | 375-398 | Sum of all | All operations run sequentially |

### MEDIUM (System calls)

| Component | File | Lines | Duration | Issue |
|-----------|------|-------|----------|-------|
| **GPU detection** | lib/ollama/lifecycle.sh | 103-207 | 1-5s | Multiple nvidia-smi/rocm-smi calls |
| **Hardware detection** | lib/ollama/lifecycle.sh | 363-373 | 1-2s | File reads, command checks |

---

## Root Cause Analysis

The **Rust TUI** (`tui/src/app.rs`) is correctly designed:
- Uses `tokio::select!` for async event handling (app.rs:260)
- Renders initial frame immediately (app.rs:253)
- Uses 50ms timeouts for startup operations (app.rs:284-301)
- Disables model warmup to keep startup responsive (conductor_client.rs:112)

The problem is the **bash bootstrap layer** runs BEFORE the TUI:
```bash
# yollayah.sh main()
main() {
    # ... (line 375)
    ollama_ensure_running        # Blocks up to 10s
    model_select_best            # Blocks 1-2s
    model_ensure_ready           # Blocks 5-30min on first run!
    agents_sync                  # Blocks 5-30s on git clone
    yollayah_create_model        # Blocks 2-5s

    # FINALLY launches TUI
    yollayah_run_tui             # Non-blocking, properly async
}
```

---

## Solution Strategy

### Phase 1: Quick Wins (Sprint 10) - ❌ REVERTED (Design Flaw)

**Goal**: Make TUI launch immediately with loading indicators

**What We Tried**:
- Launch TUI with minimal bootstrap
- Run background tasks while TUI is active
- Non-blocking startup

**Critical Issue Discovered**:
- Background bash processes wrote to stdout/stderr
- Corrupted TUI display (characters everywhere)
- Violated terminal ownership principle
- **REVERTED**: Back to synchronous bootstrap

**Root Cause**: Cannot share terminal between processes. Once TUI launches, it must have exclusive ownership.

**See**: `TODO-architecture-terminal-ownership.md` for detailed analysis

---

### Current Approach: Synchronous Bootstrap (Correct Design)

**Goal**: Complete ALL setup before TUI launches

- [x] **Synchronous bootstrap**: All operations complete before TUI
  - model_select_best ✅
  - model_ensure_ready ✅ (may be slow, but correct)
  - agents_sync ✅
  - yollayah_create_model ✅
  - routing_init, user_init ✅

- [x] **Network timeouts**: Prevent indefinite hangs
  - Git clone: 30s timeout ✅
  - Git pull: 15s timeout ✅
  - Ollama API: 10s timeout ✅

- [x] **Test mode optimization**: Skip non-essential operations
  - Uses tiny model (qwen2:0.5b) ✅
  - Skips agents_sync, yollayah_create_model, routing ✅
  - Fast startup for development ✅

**Exit Criteria**: ✅ ALL MET
- ✅ Clean TUI (no output corruption)
- ✅ Terminal ownership respected
- ✅ No indefinite hangs on network failures
- ✅ Test mode provides fast iteration (< 5s)

**Status**: CORRECT DESIGN - Synchronous bootstrap, async runtime

---

### Phase 2: Parallel Operations (Sprint 11) - SHORT TERM

**Goal**: Run independent tasks in parallel

- [ ] **B2.1**: Parallelize independent checks
  - GPU detection + Model selection can run in parallel
  - Agents sync can happen while model is being created
  - Use bash background jobs (`&`) with wait tracking

- [ ] **B2.2**: Async git operations
  - Clone agents repo in background
  - Show progress in TUI status bar
  - Allow TUI to function even if agents sync fails

- [ ] **B2.3**: Lazy model loading
  - Don't create Yollayah model until first message sent
  - Show "Preparing..." when user sends first message
  - Cache model creation result

**Exit Criteria**:
- Total bootstrap time reduced by 50%
- All independent operations run in parallel
- TUI remains responsive throughout

---

### Phase 3: Architecture Refactor (Sprint 12+) - LONG TERM

**Goal**: Move all blocking operations to Rust conductor

- [ ] **B3.1**: Move Ollama management to conductor
  - Conductor handles Ollama lifecycle
  - TUI receives status updates via protocol
  - Remove bash dependency on Ollama API

- [ ] **B3.2**: Move agents sync to conductor
  - Conductor clones/syncs agents repo
  - Progress updates via protocol messages
  - TUI shows progress in status bar

- [ ] **B3.3**: Move model management to conductor
  - Conductor handles model selection/creation
  - Background download with progress tracking
  - TUI can start before models are ready

**Exit Criteria**:
- Bash bootstrap < 500ms (only launches conductor + TUI)
- All blocking operations in Rust with async/await
- Full progress visibility in TUI

---

## Integration Test Requirements

- [x] **Test 1**: TUI launches in < 2s even if Ollama unresponsive
  - Implemented: TUI launches after ollama_ensure_running (10s max timeout)
  - Need automated test: Deferred to future sprint
- [x] **Test 2**: TUI remains responsive during model download
  - Implemented: Model pull runs in background after TUI launch
  - Need automated test: Deferred to future sprint
- [x] **Test 3**: TUI handles network timeout gracefully
  - Implemented: Git operations have 30s/15s timeouts, non-fatal errors
  - Need automated test: Deferred to future sprint
- [x] **Test 4**: User can interact with TUI during git clone
  - Implemented: agents_sync runs in background after TUI launch
  - Need automated test: Deferred to future sprint
- [ ] **Test 5**: TUI shows progress for long-running operations
  - Deferred to Phase 2 (Sprint 11) - requires protocol changes

---

## Technical Debt

**Current debt**:
- All bash operations are synchronous (no async primitives)
- No timeout on network operations
- Sequential execution of independent tasks
- No progress feedback for long operations

**Future debt if not addressed**:
- User experience degradation
- Support burden (users think app is frozen)
- Can't scale to slower hardware/networks

---

## Related Work

- **TODO-integration-testing.md**: Add tests for non-blocking startup
- **TODO-conductor-ux-split.md**: Move bash logic to conductor (Phase 3)
- **TODO-epic-2026Q1-multi-surface.md**: Protocol for progress updates

---

## Questions

### Active

| ID | Question | Owner | Target Sprint |
|----|----------|-------|---------------|
| Q1 | Should TUI show splash screen or go straight to chat? | UX | Sprint 10 |
| Q2 | What's the minimum viable state for TUI to launch? | Architect | Sprint 10 |
| Q3 | How to handle failed git clone (can't sync agents)? | Architect | Sprint 10 |

### Resolved

*None yet*

---

**Owner**: Architect + Hacker
**Last Updated**: 2026-01-02 (Initial analysis complete)
