# TODO: TUI Initialization & Loading Experience

**Created**: 2026-01-03
**Last Updated**: 2026-01-03 (Initial creation - blank screen investigation)
**Owner**: UX + Backend team
**Priority**: HIGH (UX issue visible to all users)

---

## Problem Statement

**User Report** (2026-01-03):
> "All this time the greeting still seems to be breaking stuff. Even in --test mode. The TUI always launches blank for a moment, only then I can see Yollayah dancing and the You:_ prompt."

**Impact**: Poor first impression, appears broken/frozen during startup

**Root Cause Hypothesis**: TUI waiting for Conductor connection before showing UI

---

## Goal

Transform TUI startup from "blank screen → sudden appearance" to smooth, delightful loading experience with:
- Immediate visual feedback (no blank screen)
- Yollayah-themed loading animation
- Graceful degradation if Conductor/Ollama not responsive
- Integration tests to prevent regressions

---

## Progress

### Phase 1: Investigation & Root Cause ⏳

**STORY 1: Analyze TUI Launch Sequence**
**Status**: COMPLETED ✅
**Owner**: Backend Specialist
**File**: `tui/src/app.rs`, `tui/src/main.rs`
**Completed**: 2026-01-03

**Tasks**:
- [x] Trace execution from `main()` to first render
- [x] Identify blocking operations (Conductor connect, Ollama ping, etc)
- [x] Measure time to first pixel vs time to interactive
- [x] Document current startup phases

**Findings**:

**Execution Flow**:
1. `main()` (main.rs:27-74)
   - Lines 28-32: Initialize tracing/logging
   - Lines 35-41: Set up panic hook
   - **Lines 44-49: Initialize terminal + enter alternate screen** ✅
   - **Line 52: Call `run_app()` which creates App**

2. `run_app()` (main.rs:63-74)
   - **Line 64: `App::new().await?` - FIRST POTENTIAL DELAY** ⚠️
   - Line 65: `app.run(terminal).await?` - Main loop

3. `App::new()` (app.rs:129-223) - **THIS IS SYNCHRONOUS AND FAST** ✅
   - Lines 130-131: Get terminal size (instant)
   - Lines 133-189: Create compositor and layers (instant, all UI setup)
   - Line 191: Create Avatar (instant)
   - **Line 198: `ConductorClient::new()` - CREATES CLIENT BUT DOESN'T CONNECT** ✅
   - Returns immediately with disconnected client

4. `App::run()` (app.rs:226-340) - Main event loop
   - Line 234: Create event stream (async, non-blocking)
   - Line 242: Initialize `StartupPhase::NeedStart`
   - **Line 245: `self.render(terminal)?` - FIRST RENDER CALL** ✅ **IMMEDIATE!**
   - Lines 247-337: Main loop with `tokio::select!` (non-blocking)
     - Lines 274-288: `StartupPhase::NeedStart` → calls `conductor.start()` with 50ms timeout
     - Lines 290-303: `StartupPhase::NeedConnect` → calls `conductor.connect()` with 50ms timeout
     - **Both are NON-BLOCKING with short timeouts** ✅

**Time to First Render**: ~10-50ms (terminal setup + initial render)
- Terminal initialization (enable raw mode, alternate screen): ~5-10ms
- App::new() (UI layout, layers): ~5-10ms
- First render() call: ~10-30ms (depends on terminal speed)
- **TOTAL: 20-50ms to first pixels on screen** ✅

**Time to Conductor Connection**:
- In embedded mode: 50-200ms (first `conductor.start()` timeout + retry)
- In daemon mode: Depends on whether daemon is already running
  - If daemon running: 50-100ms (socket connection)
  - If daemon not running: **INDEFINITE - NEVER CONNECTS** ❌

**Blocking Operations Identified**:
1. **NONE in the critical path to first render** ✅
2. Conductor startup happens AFTER first render in event loop
3. All startup operations use 50ms timeouts with retry

**Root Cause of Blank Screen**:

**PRIMARY CAUSE**: The "blank screen" is actually the **terminal clearing and entering alternate screen** (line 46 in main.rs) BEFORE the first render at line 245 in app.rs. There's a **195-line execution gap** between entering alternate screen and first render.

**TIMING BREAKDOWN**:
```
T+0ms:    Terminal cleared, alternate screen entered (main.rs:46)
          ↓ User sees: BLANK SCREEN
T+0-10ms: App::new() runs (fast, but terminal is blank during this)
          ↓ User sees: STILL BLANK
T+10-50ms: First render() completes (app.rs:245)
          ↓ User sees: UI APPEARS!
```

**Why it feels broken**:
- 10-50ms blank screen is noticeable (humans detect 16ms delays)
- On slow terminals or under load, could be 100-200ms
- No visual feedback during App::new() initialization

**SECONDARY ISSUE (Daemon Mode)**:
In yollayah.sh, if daemon mode is NOT used (default is embedded mode), there's no issue. However, if daemon mode IS used and daemon isn't started first, the TUI will show loading UI but never connect.

**Technical Details**:
- App::new() is NOT async blocking (returns instantly)
- ConductorClient::new() creates client but doesn't connect (instant)
- Conductor.start() and .connect() happen in the event loop AFTER first render
- The blank screen is purely the gap between terminal setup and first render

---

**STORY 2: Fix yollayah.sh Launch Order**
**Status**: COMPLETED ✅ (ALREADY CORRECT)
**Owner**: Backend Specialist
**File**: `yollayah.sh`
**Completed**: 2026-01-03

**Tasks**:
- [x] Check if Conductor daemon is started before TUI
- [x] Verify launch order is correct
- [x] Confirm health check exists before TUI launch
- [x] Document findings

**Current Behavior (ALREADY CORRECT)**:

**Analysis of yollayah.sh** (lines 542-562):

The script uses the **correct architecture** - TUI runs in embedded mode by default (NOT daemon mode):

1. **Line 560: `ux_start_interface()`** calls `lib/ux/terminal.sh:ux_start_interface()`
2. **terminal.sh lines 543-558**:
   - Line 548: `ux_tui_ensure_ready()` - Builds TUI if needed
   - Line 550-553: `ux_launch_tui()` - Launches TUI
3. **terminal.sh lines 508-516**:
   - Checks if daemon mode is enabled via `conductor_needs_daemon()`
   - Default transport is "inprocess" (embedded mode)
   - Only starts daemon if `CONDUCTOR_TRANSPORT=unix` or `=socket`

**Key Insight**: The blank screen issue is NOT related to daemon launch order!

**Why?**
- Default mode: **Embedded Conductor** (no daemon needed)
- TUI creates embedded Conductor via `ConductorClient::new()` (conductor_client.rs:85-122)
- Embedded Conductor starts AFTER first render (in event loop)
- Daemon mode is OPTIONAL, controlled by `CONDUCTOR_TRANSPORT` env var

**Daemon Mode Launch Order (IF ENABLED)**:

When `CONDUCTOR_TRANSPORT=unix`, terminal.sh does the RIGHT thing:

```bash
# terminal.sh lines 508-516 (inside ux_launch_tui)
if conductor_needs_daemon; then
    # Start daemon BEFORE launching TUI
    conductor_ensure_running || return 1
fi

# Launch TUI (connects to running daemon)
"$tui_bin"
```

**Health Check**: `conductor_ensure_running()` (terminal.sh lines 285-344)
- Line 293: Checks if daemon already running
- Lines 316-318: Starts daemon in background
- Lines 321-332: **Waits for socket to appear (max 10s)**
- Line 334: Fails if socket not created

**CONCLUSION**: Launch order is ALREADY CORRECT. The blank screen is NOT caused by daemon/TUI ordering issues.

**Recommendations**:
1. ✅ No changes needed to yollayah.sh
2. ✅ No changes needed to daemon launch order
3. ⚠️ Focus on STORY 4 (non-blocking startup) to fix blank screen
4. ⚠️ The issue is the 10-50ms gap between terminal clear and first render

---

### Phase 2: Loading Animation 🎨

**STORY 3: Design Yollayah-Themed Loading UI**
**Status**: PENDING
**Owner**: UX Specialist
**File**: New file `tui/src/loading.rs`

**Design Requirements**:
- Shows immediately (no blank screen)
- Yollayah-themed (use avatar sprites or palette)
- Progress indication (connecting → warming up → ready)
- Cancellable (Ctrl+C works during loading)

**Animation Ideas**:
- Option A: Spinning axolotl sprite
- Option B: Pulsing "Yollayah" text with breathing colors
- Option C: Progress bar with axolotl swimming across
- Option D: Simple "Loading..." with animated dots

**Tasks**:
- [ ] Design loading screen mockup
- [ ] Choose animation style (get user approval)
- [ ] Implement loading widget
- [ ] Test on slow connections

---

**STORY 4: Implement Non-Blocking Startup**
**Status**: PENDING
**Owner**: Backend Specialist
**File**: `tui/src/app.rs`

**Current Problem**: TUI blocks on Conductor connection before showing anything

**Solution**: Show loading UI immediately, connect in background

**Implementation Pattern**:
```rust
pub async fn run() -> Result<()> {
    let mut terminal = setup_terminal()?;

    // IMMEDIATE: Show loading screen (no blocking)
    let loading = LoadingScreen::new();
    terminal.draw(|f| loading.render(f))?;

    // BACKGROUND: Connect to Conductor with timeout
    let conductor = tokio::spawn(async {
        Conductor::connect_with_retry(max_retries: 3, timeout: 5s).await
    });

    // LOOP: Animate loading while waiting
    loop {
        tokio::select! {
            Ok(c) = &mut conductor => {
                // Connected! Transition to main UI
                break;
            }
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                // Update loading animation
                loading.tick();
                terminal.draw(|f| loading.render(f))?;
            }
            maybe_event = event_stream.next() => {
                // Handle Ctrl+C during loading
                if is_quit_event(maybe_event) {
                    return Ok(());
                }
            }
        }
    }

    // Continue with normal TUI
}
```

**Tasks**:
- [ ] Extract startup logic into separate phases
- [ ] Show loading screen before any blocking operations
- [ ] Make Conductor connection non-blocking with timeout
- [ ] Animate loading screen during connection
- [ ] Graceful error messages if connection fails

---

### Phase 3: Graceful Degradation 🛡️

**STORY 5: Handle Unresponsive Conductor/Ollama**
**Status**: PENDING
**Owner**: Backend Specialist + QA
**File**: `tui/src/app.rs`, `tui/src/conductor_client.rs`

**Scenarios to Handle**:
1. Conductor not running → Show error, offer to retry or exit
2. Conductor slow to respond → Show "still connecting..." with spinner
3. Ollama not running → TUI launches, shows warning, limited functionality
4. Ollama slow → Show "warming up model..." during inference

**Tasks**:
- [ ] Add timeout to Conductor connection (5 seconds)
- [ ] Show user-friendly error messages
- [ ] Offer retry option
- [ ] Add "offline mode" if Ollama unavailable
- [ ] Test all failure scenarios

**Error Messages** (Yollayah-themed):
```
🦎 Oops! I can't find my brain (Conductor not running)

   Let me try to start it for you...
   [Retry] [Exit]

🦎 Still waking up... (Ollama loading model)

   This might take a minute on first launch.
   ⚡ GPU detected - should be fast!
```

---

### Phase 4: Integration Tests 🧪

**STORY 6: Add TUI Responsiveness Tests**
**Status**: IN PROGRESS
**Owner**: QA Specialist
**File**: `tui/tests/initialization_test.rs`

**Test Coverage**: 5 critical scenarios preventing blank screen regressions

### Test 1: TUI shows loading screen immediately (NO BLANK)
**Timeout**: 2s per test

**Setup**:
- Mock terminal backend using `ratatui::backend::TestBackend`
- Create app with slow Conductor (5s delay)
- Track first frame render time

**Expected Behavior**:
- First frame renders within 50ms of launch
- First frame contains "Loading..." or animated text (not blank)
- Frame buffer is not empty

**Assertions**:
```rust
assert!(!first_frame.is_empty(), "First frame should not be blank");
assert!(elapsed_ms < 50, "Must render within 50ms");
assert_contains_text(&first_frame, "Loading");
```

**Mock Infrastructure**:
- Use `TestBackend` for frame capture
- Hook `App::render()` to record timestamp
- Capture terminal output before Conductor connects

---

### Test 2: TUI handles missing Conductor gracefully
**Timeout**: 3s per test

**Setup**:
- Launch TUI with **no Conductor running** (connection timeout: 100ms)
- Use mock transport that fails immediately
- Capture stderr and frame buffer

**Expected Behavior**:
- TUI shows error message (user-friendly, not stack trace)
- TUI doesn't panic or hang
- App exits gracefully after error

**Assertions**:
```rust
assert!(!panic_occurred, "Should not panic");
assert_contains_text(&frame, "Oops");  // or "can't find"
assert!(app_exited, "Should exit cleanly");
assert_eq!(exit_code, Some(1), "Error exit code");
```

**Mock Infrastructure**:
- `MockConductorClient` that always fails on connect
- Return `ConnectionError` immediately
- Track if panic hook was triggered

---

### Test 3: TUI handles slow Conductor (5s delay)
**Timeout**: 8s per test

**Setup**:
- Mock Conductor that delays connection by 5000ms
- Launch TUI
- Verify UI remains responsive while waiting

**Expected Behavior**:
- Loading screen shown immediately (test #1 ensures this)
- Animation updates while waiting (frame changes every 100ms)
- Connection succeeds after delay
- Seamless transition to main UI

**Assertions**:
```rust
assert!(connected, "Should eventually connect");
assert!(animation_updated, "Should animate during wait");
assert!(frame_count > 1, "Multiple frames rendered");
assert!(elapsed_ms >= 5000, "Actually waited ~5s");
assert!(elapsed_ms < 6000, "No excessive delay");
```

**Mock Infrastructure**:
- `SlowMockConductor` with configurable delay
- Track frame count during connection phase
- Use `tokio::time::sleep()` to simulate delay

---

### Test 4: TUI handles unresponsive Ollama gracefully
**Timeout**: 3s per test

**Setup**:
- Start real Conductor (or mock that says it's ready)
- Ollama backend returns errors (simulated failure)
- Launch TUI
- Try to send a message

**Expected Behavior**:
- TUI shows "offline" mode or warning
- User can still interact (input available)
- Graceful error message instead of crash
- Optional retry mechanism

**Assertions**:
```rust
assert!(!panic_occurred, "Should handle Ollama error gracefully");
assert_contains_text(&frame, "offline") OR assert_contains_text(&frame, "warning");
assert!(can_type_message, "Input should still work");
assert!(!error_msg.contains("panic"), "No stack trace");
```

**Mock Infrastructure**:
- `OfflineBackend` that returns `ServiceUnavailable` error
- Mock Conductor that handles backend errors
- Error injection in mock backend

---

### Test 5: Ctrl+C during loading works cleanly
**Timeout**: 2s per test

**Setup**:
- Launch TUI with 3s Conductor delay
- Send SIGINT after 500ms (during loading)
- Capture exit behavior

**Expected Behavior**:
- App receives SIGINT and starts shutdown
- No panic or hanging
- Clean terminal restoration (no artifacts)
- Exit within 500ms of signal

**Assertions**:
```rust
assert_eq!(exit_code, Some(130), "SIGINT exit code");  // or 0 for graceful
assert!(!panic_occurred, "No panic on SIGINT");
assert!(shutdown_ms < 500, "Clean shutdown within 500ms");
assert!(terminal_restored, "Terminal state clean");
```

**Mock Infrastructure**:
- Simulate SIGINT by calling `App::handle_key(Ctrl+C)`
- Mock `crossterm` event stream for signal injection
- Track panic hook and cleanup

---

## Mock Infrastructure Design

### Core Mock Components

**1. MockConductorClient** (replaces `ConductorClient`):
```rust
pub struct MockConductorClient {
    mode: MockMode,
    connected: Arc<AtomicBool>,
    connect_delay: Duration,
}

pub enum MockMode {
    Instant,           // Connect immediately
    Delayed { ms: u64 },
    Fails { reason: String },
    OfflineOllama,     // Says ready but Ollama errors
}
```

**2. TestFrameCapture** (hooks into render):
```rust
pub struct TestFrameCapture {
    frames: Vec<TerminalFrame>,
    timestamps: Vec<Instant>,
}

impl TestFrameCapture {
    pub fn first_frame_time(&self) -> Duration { ... }
    pub fn contains_text(&self, text: &str) -> bool { ... }
    pub fn frame_count(&self) -> usize { ... }
}
```

**3. TestTerminal** (minimal mock):
```rust
pub struct TestTerminal {
    buffer: String,
    panic_hook_called: Arc<AtomicBool>,
}

impl TestTerminal {
    pub fn render_spy(&self) -> &str { &self.buffer }
    pub fn panic_occurred(&self) -> bool { ... }
}
```

---

## Test Running Performance

**Target**: All 5 tests complete in < 10 seconds (< 5 for pre-commit)

| Test | Duration | Notes |
|------|----------|-------|
| Test 1: No blank | 0.1s | Fast (no real delays) |
| Test 2: Missing Conductor | 0.2s | Timeout: 100ms |
| Test 3: Slow Conductor | 5.2s | Actual 5s wait |
| Test 4: Offline Ollama | 0.3s | Fast (mock error) |
| Test 5: Ctrl+C | 0.5s | Signal injection |
| **Total** | **~6.3s** | **Within budget** |

**Optimization Strategy**:
- Test 3 dominates (5s delay is real)
- Run in parallel for CI (tokio-test supports concurrent tests)
- Skip Test 3 in pre-commit, run in CI only
- Use `--test-threads=1` to prevent flakiness from concurrent Conductor initialization

---

## Pre-Commit Integration

**Pre-commit script** (run fast tests only):
```bash
#!/bin/bash
# Run fast initialization tests (skip slow #3)
set -e

echo "Running TUI initialization tests (fast subset)..."
cd tui

# Tests 1, 2, 4, 5 (skip Test 3 which takes 5s)
cargo test --test initialization_test \
  --lib test_tui_no_blank_screen \
  --lib test_conductor_not_running \
  --lib test_ollama_offline \
  --lib test_ctrl_c_during_loading \
  -- --test-threads=1 --nocapture

echo "✅ Fast tests passed. Skipped slow Conductor test (runs in CI)."
```

**CI Integration** (run all tests):
```bash
#!/bin/bash
# Run all initialization tests in CI
cargo test --test initialization_test -- --test-threads=1
```

**Expected pre-commit time**: 1-2 seconds (fast tests only)
**Expected CI time**: ~7 seconds (all tests including slow one)

---

## Files to Create/Modify

**New Files**:
- `/var/home/machiyotl/src/ai-way/tui/tests/initialization_test.rs` (main test file, ~400-500 LOC)
- `/var/home/machiyotl/src/ai-way/tui/tests/mocks/mod.rs` (mock infrastructure)
- `/var/home/machiyotl/src/ai-way/tui/tests/mocks/conductor_client.rs` (mock client)
- `/var/home/machiyotl/src/ai-way/tui/tests/mocks/terminal.rs` (test terminal backend)

**Modified Files**:
- `tui/Cargo.toml`: Add `[dev-dependencies]` for test utilities
- `.git/hooks/pre-commit`: Add initialization tests to hook
- CI config (GitHub Actions or similar)

---

## Regression Prevention Strategy

**Blank Screen Prevention**:
1. Test 1 catches any regression where loading screen is delayed
2. First frame must contain visual content (not empty buffer)
3. Assert timestamp < 50ms (tight SLA)

**Conductor Connectivity**:
1. Test 2 validates missing Conductor is handled gracefully
2. Test 3 ensures slow connections don't hang the UI
3. Mock infrastructure simulates real conditions

**Graceful Degradation**:
1. Test 4 ensures Ollama failure doesn't crash TUI
2. Error messages are user-friendly (no jargon, no stack traces)
3. UI remains interactive despite backend issues

**Signal Handling**:
1. Test 5 catches unclean shutdown issues
2. Terminal cleanup is verified
3. No resource leaks on SIGINT

---

## Success Criteria

- ✅ All 5 tests implemented and passing
- ✅ Tests run < 10s total (< 5s for fast subset)
- ✅ Mocks are reusable for future TUI tests
- ✅ Pre-commit hook includes tests
- ✅ CI pipeline runs full test suite
- ✅ No blank screen regressions possible (test coverage)
- ✅ Team can run tests locally: `cargo test --test initialization_test`

**Tasks**:
- [ ] Create mock infrastructure in `tui/tests/mocks/`
- [ ] Implement `TestFrameCapture` for frame recording
- [ ] Implement `MockConductorClient` for test scenarios
- [ ] Write all 5 test functions in `initialization_test.rs`
- [ ] Add helper assertions (`assert_contains_text`, etc)
- [ ] Verify all tests pass locally
- [ ] Update pre-commit hook to run fast tests
- [ ] Document test running in README

---

**STORY 7: Wire Tests to Pre-Commit**
**Status**: PENDING
**Owner**: DevOps/QA
**File**: `.git/hooks/pre-commit` or `scripts/pre-commit.sh`

**Implementation Plan**:

### A. Pre-Commit Hook Setup

**File**: `.git/hooks/pre-commit` or `scripts/pre-commit-hooks.sh`

```bash
#!/bin/bash
# Pre-commit hook for ai-way TUI initialization tests
set -e

# Only run if Rust files changed
if ! git diff --cached --name-only | grep -q "^tui/"; then
    echo "⏭️  No TUI changes, skipping initialization tests"
    exit 0
fi

echo "🧪 Running TUI initialization tests (fast subset)..."
cd "$(git rev-parse --show-toplevel)/tui"

# Run fast tests only (skip Test 3 which takes 5s)
# Tests: 1=no blank, 2=missing conductor, 4=offline ollama, 5=ctrl+c
timeout 5 cargo test --test initialization_test \
    --release \
    -- \
    --test-threads=1 \
    --skip test_conductor_slow_response \
    2>&1 | tail -20

if [ $? -eq 0 ]; then
    echo "✅ TUI initialization tests passed!"
    exit 0
else
    echo "❌ TUI tests failed. Run this for details:"
    echo "   cd tui && cargo test --test initialization_test -- --nocapture"
    exit 1
fi
```

### B. Installation Script

**File**: `scripts/install-precommit-hook.sh`

```bash
#!/bin/bash
# Install pre-commit hook for TUI tests

HOOK_SRC="scripts/pre-commit-hooks.sh"
HOOK_DST=".git/hooks/pre-commit"

if [ ! -f "$HOOK_SRC" ]; then
    echo "Error: $HOOK_SRC not found"
    exit 1
fi

cp "$HOOK_SRC" "$HOOK_DST"
chmod +x "$HOOK_DST"
echo "✅ Pre-commit hook installed at $HOOK_DST"
echo ""
echo "To run pre-commit hook manually:"
echo "  .git/hooks/pre-commit"
echo ""
echo "To skip hook during commit:"
echo "  git commit --no-verify"
```

### C. CI/CD Integration

**GitHub Actions** (`.github/workflows/test.yml`):

```yaml
name: TUI Tests

on: [push, pull_request]

jobs:
  initialization-tests:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: tui

      - name: Run initialization tests (all 5)
        run: |
          cd tui
          cargo test --test initialization_test \
            --release \
            -- --test-threads=1
        timeout-minutes: 2

      - name: Report test results
        if: always()
        run: |
          echo "Test suite completed"
          # Optional: upload coverage, artifacts, etc.
```

### D. Test Behavior

| Scenario | Pre-Commit | CI |
|----------|-----------|-----|
| Fast tests (1,2,4,5) | Run | Run |
| Slow test (3) | Skip | Run |
| Total time | ~1.5s | ~7s |
| Blocks commit? | Yes (on failure) | Yes (on failure) |
| Can skip? | `--no-verify` | N/A |

### E. Developer Workflow

**Setup (one-time)**:
```bash
cd /var/home/machiyotl/src/ai-way
bash scripts/install-precommit-hook.sh
```

**Normal workflow**:
```bash
# Make changes, stage files
git add tui/src/app.rs

# Try to commit - hook runs automatically
git commit -m "Fix TUI initialization"

# If tests fail:
# Option 1: Fix code, commit again
# Option 2: Skip hook for debugging (temporary)
git commit --no-verify -m "WIP: debugging"

# Option 3: Run tests manually first
cd tui
cargo test --test initialization_test -- --nocapture
```

**Troubleshooting**:
```bash
# Run hook manually
.git/hooks/pre-commit

# Run full test suite (not just pre-commit subset)
cd tui
cargo test --test initialization_test -- --nocapture

# Check hook status
ls -la .git/hooks/pre-commit

# Remove hook temporarily
rm .git/hooks/pre-commit
```

**Tasks**:
- [ ] Create `scripts/pre-commit-hooks.sh` with hook implementation
- [ ] Create `scripts/install-precommit-hook.sh` for easy installation
- [ ] Set up CI/CD workflow (GitHub Actions)
- [ ] Document in main README.md
- [ ] Test hook with dummy changes
- [ ] Verify hook blocks commits on test failure
- [ ] Verify `--no-verify` allows bypassing hook

---

## Success Criteria

- ✅ TUI shows loading UI within 100ms of launch (no blank screen)
- ✅ Yollayah-themed loading animation is delightful
- ✅ Graceful error messages if Conductor/Ollama unavailable
- ✅ All 5 integration tests pass
- ✅ Tests wired to pre-commit (run on every commit)
- ✅ User reports improved startup experience

---

## Dependencies

**Blocked By**: None (can start immediately)

**Blocks**:
- User onboarding experience (first impressions matter)
- Error handling documentation

---

## Feature Creep Items

_Items discovered during work that should NOT block this feature:_

- [ ] Animated splash screen with full axolotl animation
- [ ] Progress percentage for model loading
- [ ] Sound effects during loading (if terminal supports)
- [ ] Easter eggs in loading messages
- [ ] Keyboard shortcuts during loading screen

---

## Testing Strategy

**Manual Testing**:
1. Launch TUI on fresh system (no Conductor running)
2. Launch TUI with Ollama stopped
3. Launch TUI with slow network (simulate delay)
4. Press Ctrl+C during loading
5. Verify no blank screen in all scenarios

**Automated Testing**:
- Unit tests: Loading screen rendering
- Integration tests: Startup sequence with mocked Conductor
- E2E tests: Full startup flow with real Conductor (CI only)

---

## Timeline Estimate

| Phase | Effort | Duration |
|-------|--------|----------|
| Phase 1: Investigation | 2 hours | Same day |
| Phase 2: Loading Animation | 3 hours | 1 day |
| Phase 3: Graceful Degradation | 2 hours | Same day |
| Phase 4: Integration Tests | 4 hours | 1-2 days |
| **Total** | **11 hours** | **2-3 days** |

---

## Related Work

**See Also**:
- `ODYSSEY-tui-to-framebuffer.md` - TUI performance work
- `TODO-conductor-ux-split.md` - Conductor architecture
- `workflows/todo-driven-development.md` - Process guidelines

---

## Notes

**2026-01-03**: Initial creation based on user report of blank TUI on launch. User suspects Conductor connection is blocking. Investigation needed to confirm root cause before implementing fixes.

**Key Insight**: "Immediate visual feedback" is critical for UX. No blank screens allowed - show *something* immediately, even if it's just a loading spinner.
