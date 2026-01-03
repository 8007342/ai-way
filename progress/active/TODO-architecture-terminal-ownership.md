# TODO-architecture-terminal-ownership: Terminal Ownership and Async Design

**Created**: 2026-01-02
**Status**: CRITICAL
**Priority**: CRITICAL (Design Bug - TUI Corrupted)
**Owner**: Architect

---

## Critical Issue Discovered

**Problem**: Background processes in yollayah.sh continue spewing output to terminal after TUI launches, corrupting the TUI display.

**Root Cause**: Violated separation of terminal ownership by running async background tasks (`_background_bootstrap &`) that write to stdout/stderr while TUI has control.

**Impact**:
- TUI display corrupted with random output
- Character alignment broken
- Unusable interface
- Design violation of single terminal owner principle

---

## Architectural Principles (HARD REQUIREMENTS)

### 1. Terminal Ownership

**RULE**: Only ONE process can own the terminal at a time.

- **Before TUI launch**: yollayah.sh owns terminal, can write to stdout/stderr
- **After TUI launch**: TUI owns terminal exclusively, nothing else can write
- **Violation**: Background bash processes writing to terminal while TUI active

### 2. Async Requirements by Component

| Component | Async Required? | Reason |
|-----------|----------------|--------|
| **yollayah.sh** | ❌ NO | Bootstrap script - completes before TUI |
| **TUI** | ✅ YES | Must be responsive, non-blocking UI |
| **Conductor** | ✅ YES | Must handle multiple models, concurrent requests |
| **Surfaces** | ✅ YES | All surfaces must be responsive |

**Key Insight**: Async is for **runtime components**, not **bootstrap scripts**.

### 3. Separation of Concerns

```
Bootstrap (Synchronous)     Runtime (Async)
┌─────────────────┐        ┌──────────────┐
│  yollayah.sh    │───────▶│     TUI      │
│  (sync setup)   │        │  (async UI)  │
│  - integrity    │        │              │
│  - ollama start │        │  Conductor   │
│  - model pull   │        │  (async core)│
│  - agents sync  │        │              │
└─────────────────┘        └──────────────┘
     Completes                  Runs forever
     then exits                 (async/await)
```

---

## What We Did Wrong (Sprint 10)

### Incorrect Approach

We tried to optimize startup by running background tasks:

```bash
main() {
    # Critical path
    integrity_verify
    setup_run
    ollama_ensure_running

    # WRONG: Launch TUI with background tasks still running
    _background_bootstrap &  # ❌ Writes to terminal!
    ux_start_interface       # ✅ TUI takes terminal
    wait                      # Too late, TUI already corrupted
}
```

**Problem**: `_background_bootstrap` runs functions like `model_pull`, `agents_sync` that call `ux_*` functions which write to stdout. This output appears on top of the TUI, corrupting it.

---

## Correct Approach

### Synchronous Bootstrap

yollayah.sh should complete ALL setup before launching TUI:

```bash
main() {
    # All bootstrap synchronously
    integrity_verify
    setup_run
    ollama_ensure_running
    model_select_best
    model_ensure_ready      # May take time, but necessary
    agents_sync
    yollayah_create_model
    routing_init
    user_init

    # Only NOW launch TUI (terminal ownership transfers)
    ux_start_interface      # ✅ TUI has exclusive terminal
}
```

**Benefit**: Clear separation, no corruption, simpler code.

**Cost**: Slower startup (but correct behavior).

---

## Why This is the Right Design

### 1. Terminal is a Shared Resource

Only one process can control the terminal. Trying to share it leads to:
- Output corruption
- Race conditions
- Undefined behavior
- Poor user experience

### 2. Bootstrap vs Runtime

**Bootstrap** (yollayah.sh):
- Runs once at startup
- Completes and exits
- Can be slow (5-60s acceptable)
- Should be synchronous for simplicity

**Runtime** (TUI, Conductor):
- Runs continuously
- Must be responsive
- MUST be async/non-blocking
- Complex state management

### 3. Optimization Should Happen in Runtime

If we want fast startup:
- ✅ Optimize TUI to show UI before models ready
- ✅ Conductor loads models lazily on first use
- ✅ Show loading states in TUI for background operations
- ❌ Don't run bash background tasks that corrupt terminal

---

## Implementation Plan

### Step 1: Revert Background Bootstrap ✅ IMMEDIATE

- Remove `_background_bootstrap &`
- Remove `wait` calls
- Make all bootstrap operations synchronous
- Keep test mode optimizations (skip operations, not background them)

### Step 2: Clean Up Bootstrap Flow

```bash
main() {
    clear
    ux_print_banner
    integrity_verify || exit 1
    setup_run || exit 1
    ollama_check_installed || exit 1
    ollama_record_state
    ollama_register_cleanup

    # All bootstrap synchronously
    ollama_ensure_running || exit 1

    # Test mode: minimal bootstrap
    if [[ -n "${YOLLAYAH_TEST_MODE:-}" ]]; then
        model_select_best
        model_ensure_ready || exit 1
        # Skip agents_sync, routing_init, user_init, etc
    else
        # Normal mode: full bootstrap
        model_select_best
        verify_ollama_gpu_usage || true
        model_ensure_ready || exit 1
        agents_sync
        yollayah_create_model || exit 1
        routing_init
        user_init
        ux_show_all_ready
    fi

    # NOW launch TUI (owns terminal from here)
    ux_start_interface "$YOLLAYAH_MODEL_NAME"
}
```

### Step 3: Future Optimization (Conductor)

If we want responsive startup:
- TUI can show "Loading..." state
- Conductor can load models in background (async Rust)
- TUI receives status updates via protocol
- Display updates asynchronously in TUI

This keeps async in the right layer (Rust/TUI), not bash.

---

## Testing Plan

### Test 1: TUI Corruption ✅ MUST FIX

```bash
./yollayah.sh --test
# Expected: Clean TUI, no output corruption
# Current: Output from background tasks corrupts display
```

### Test 2: Normal Mode

```bash
./yollayah.sh
# Expected: Full bootstrap completes, then clean TUI
# May be slow, but correct
```

### Test 3: Verify No Background Jobs

```bash
# After launching TUI, check for background processes
jobs
# Expected: Empty (no background jobs from yollayah.sh)
```

---

## Architectural Lessons

### ✅ DO

1. **Sync bootstrap, async runtime**
2. **Single terminal owner**
3. **Transfer ownership cleanly (bootstrap → TUI)**
4. **Optimize in the right layer (Rust, not bash)**
5. **Simple is better than clever**

### ❌ DON'T

1. **Mix sync and async in bash**
2. **Share terminal between processes**
3. **Run background bash jobs that write to stdout**
4. **Optimize at the wrong layer**
5. **Sacrifice correctness for speed**

---

## Related Documents

- `TODO-blocking-startup.md` - Original analysis (now outdated)
- `TODO-sprint-10-blocking-fix.md` - Needs revision
- `TODO-test-mode.md` - Test mode still valid (skip ops, don't background)

---

## Status

- ❌ Current state: BROKEN (TUI corrupted by background output)
- ⏳ Fix in progress: Revert to synchronous bootstrap
- ✅ Long-term: Async in Conductor/TUI, not bash

---

**Owner**: Architect
**Last Updated**: 2026-01-02
**Priority**: CRITICAL - Must fix before any commits
