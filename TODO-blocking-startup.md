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

### Phase 1: Quick Wins (Sprint 10) - IMMEDIATE

**Goal**: Make TUI launch immediately with loading indicators

- [ ] **B1.1**: Launch TUI immediately, show splash screen
  - Move TUI launch to start of main()
  - Show "Initializing Yollayah..." loading screen
  - Run bootstrap tasks in background with progress updates

- [ ] **B1.2**: Add timeouts to all network operations
  - Git clone: 30s timeout
  - Git pull: 15s timeout
  - Ollama API waits: 10s timeout
  - Fail gracefully with error messages

- [ ] **B1.3**: Skip model pull if not needed
  - Check if model exists before pulling
  - Show warning if model missing, let user continue
  - Offer to download in background while TUI runs

**Exit Criteria**:
- TUI shows initial screen within 1 second
- User can interact with TUI while background tasks run
- No indefinite hangs on network failures

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

- [ ] **Test 1**: TUI launches in < 1s even if Ollama unresponsive
- [ ] **Test 2**: TUI remains responsive during model download
- [ ] **Test 3**: TUI handles network timeout gracefully
- [ ] **Test 4**: User can interact with TUI during git clone
- [ ] **Test 5**: TUI shows progress for long-running operations

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
