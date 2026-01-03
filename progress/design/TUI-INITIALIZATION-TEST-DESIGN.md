# TUI Initialization Integration Test Design

**Date**: 2026-01-03
**Owner**: QA Specialist
**Status**: DESIGN COMPLETE (Ready for Implementation)

---

## Executive Summary

This document specifies comprehensive integration tests for TUI initialization to prevent blank screen regressions and ensure graceful degradation when services are unavailable.

**Key Goals**:
- Eliminate blank screen on startup (50ms to first frame)
- Handle missing/slow Conductor without hanging
- Graceful degradation when Ollama is offline
- Clean shutdown on Ctrl+C
- Fast pre-commit tests (< 5s for fast subset)
- Full test suite in CI (< 10s total)

---

## Problem Statement

**Current Issue** (from user report):
> "The TUI always launches blank for a moment, only then I can see Yollayah dancing and the You:_ prompt."

**Root Cause**: TUI waits for Conductor connection before showing UI

**Test Gap**: No integration tests preventing blank screen regressions

---

## Test Specification Matrix

### Test 1: No Blank Screen (CRITICAL)

| Aspect | Details |
|--------|---------|
| **Test Name** | `test_tui_no_blank_screen` |
| **Duration** | 100-200ms |
| **Category** | Regression prevention (blank screen) |
| **Pre-Commit?** | YES - Fast |
| **CI Only?** | NO |

**Scenario**:
- App launches with normal Conductor delay (no artificial delay)
- First frame must be captured and analyzed

**Expected Outcomes**:
- First frame renders within 50ms
- First frame contains non-empty content
- First frame includes "Loading..." or similar text
- No blank terminal buffers

**Assertions**:
```rust
assert!(!first_frame.is_empty(), "Frame buffer must not be blank");
assert!(time_to_first_frame_ms < 50, "Must render within 50ms");
assert_contains(&first_frame, "Loading");
```

**Mock Infrastructure Needed**:
- `TestBackend` for frame capture
- Timestamp tracking on render calls
- Frame buffer inspection

**Why This Test Matters**:
- User sees immediate visual feedback
- No "frozen" appearance during startup
- Prevents regression to blank screen behavior

---

### Test 2: Missing Conductor (ERROR HANDLING)

| Aspect | Details |
|--------|---------|
| **Test Name** | `test_conductor_not_running` |
| **Duration** | 200-300ms |
| **Category** | Graceful degradation |
| **Pre-Commit?** | YES - Fast |
| **CI Only?** | NO |

**Scenario**:
- TUI launches but Conductor connection fails immediately
- Connection timeout: 100ms
- No running Conductor process

**Expected Outcomes**:
- Error message displayed (user-friendly)
- No panic or stack trace
- App exits with error code 1
- Clean terminal restoration

**Assertions**:
```rust
assert!(!panic_occurred, "Should not panic");
assert_contains(&error_frame, "Oops") || assert_contains(&error_frame, "can't find");
assert!(app_exited, "Should exit gracefully");
assert_eq!(exit_code, Some(1), "Error exit code");
assert!(!error_msg.contains("thread"), "No stack traces");
```

**Mock Infrastructure Needed**:
- `MockConductorClient` that fails on connect
- Return immediate connection error
- Panic hook tracking

**Why This Test Matters**:
- Users might run TUI without starting Conductor daemon
- Clear error message instead of cryptic crash
- Prevents bad UX if Conductor crashes

---

### Test 3: Slow Conductor (PERFORMANCE BASELINE)

| Aspect | Details |
|--------|---------|
| **Test Name** | `test_conductor_slow_response` |
| **Duration** | 5.2 seconds |
| **Category** | Performance / responsiveness |
| **Pre-Commit?** | NO - Too slow |
| **CI Only?** | YES |

**Scenario**:
- Conductor connection succeeds but with 5 second delay
- TUI must remain responsive (animated) during wait
- Eventually connects and transitions to main UI

**Expected Outcomes**:
- Multiple frames rendered during waiting (animation)
- Connection succeeds after ~5s
- No UI freezing
- Smooth transition to main screen

**Assertions**:
```rust
assert!(connected, "Should eventually connect");
assert!(frame_count > 1, "Multiple frames rendered (animation)");
assert!(elapsed_ms >= 5000, "Waited approximately 5s");
assert!(elapsed_ms < 6000, "No excessive delays");
```

**Mock Infrastructure Needed**:
- Configurable delay in mock Conductor
- Frame counter during connection phase
- Timing measurements

**Why This Test Matters**:
- Ensures TUI doesn't hang on slow connections
- Animation proves UI remains responsive
- Performance baseline for startup

**Note**: This test is slow (5s actual delay) so it's only run in CI, not pre-commit.

---

### Test 4: Offline Ollama (GRACEFUL DEGRADATION)

| Aspect | Details |
|--------|---------|
| **Test Name** | `test_ollama_offline` |
| **Duration** | 300-400ms |
| **Category** | Error handling |
| **Pre-Commit?** | YES - Fast |
| **CI Only?** | NO |

**Scenario**:
- Conductor connects successfully
- Ollama backend is unavailable (returns error)
- User tries to send a message
- Should show offline message, not crash

**Expected Outcomes**:
- TUI shows offline/warning mode
- Input still works (user can type)
- Error message is user-friendly
- No panic on failed LLM request

**Assertions**:
```rust
assert!(!panic_occurred, "Should handle gracefully");
assert_contains(&frame, "offline") || assert_contains(&frame, "unavailable");
assert!(can_type_message, "Input should still work");
assert!(!error_contains("panic"), "No stack traces");
```

**Mock Infrastructure Needed**:
- Mock backend that returns errors
- Error injection in Conductor
- User input simulation

**Why This Test Matters**:
- Ollama might not be running (e.g., on slow first startup)
- TUI should remain usable even without LLM
- Clear messaging about service availability

---

### Test 5: Ctrl+C During Loading (SIGNAL HANDLING)

| Aspect | Details |
|--------|---------|
| **Test Name** | `test_ctrl_c_during_loading` |
| **Duration** | 500-700ms |
| **Category** | Shutdown safety |
| **Pre-Commit?** | YES - Fast |
| **CI Only?** | NO |

**Scenario**:
- TUI launching with 1 second Conductor delay
- SIGINT (Ctrl+C) sent after 300ms
- App should shutdown cleanly

**Expected Outcomes**:
- No panic during shutdown
- Terminal properly restored (no artifacts)
- Clean exit within 500ms
- Exit code 130 or 0 (both acceptable)

**Assertions**:
```rust
assert!(!panic_occurred, "No panic on SIGINT");
assert!(shutdown_ms < 500, "Clean shutdown within 500ms");
assert!(terminal_restored, "Terminal state clean");
assert_eq!(exit_code, 130) || assert_eq!(exit_code, 0);
```

**Mock Infrastructure Needed**:
- Simulate SIGINT via `App::handle_key(Ctrl+C)`
- Track panic hook triggers
- Terminal state verification

**Why This Test Matters**:
- Users might press Ctrl+C while TUI is loading
- Ensures graceful shutdown without hangs
- Prevents terminal corruption

---

## Mock Infrastructure Architecture

### Component 1: MockConductorClient

**Location**: `tui/tests/mocks/conductor_client.rs`

**Purpose**: Replace real `ConductorClient` for testing

**API**:
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

impl MockConductorClient {
    pub fn instant() -> Self { /* ... */ }
    pub fn with_delay(ms: u64) -> Self { /* ... */ }
    pub fn fails(reason: &str) -> Self { /* ... */ }
    pub fn offline_ollama() -> Self { /* ... */ }
}
```

**Usage in Tests**:
```rust
#[tokio::test]
async fn test_slow_conductor() {
    let mock = MockConductorClient::with_delay(5000);
    let app = App::with_client(mock).await?;
    // ... test behavior while connecting
}
```

---

### Component 2: TestFrameCapture

**Location**: `tui/tests/mocks/terminal.rs`

**Purpose**: Capture and analyze rendered frames

**API**:
```rust
pub struct TestFrameCapture {
    frames: Vec<TerminalBuffer>,
    timestamps: Vec<Instant>,
}

impl TestFrameCapture {
    pub fn first_frame(&self) -> Option<&TerminalBuffer> { /* ... */ }
    pub fn time_to_first_frame(&self) -> Duration { /* ... */ }
    pub fn frame_count(&self) -> usize { /* ... */ }
    pub fn contains_text(&self, text: &str) -> bool { /* ... */ }
    pub fn frame_at(&self, index: usize) -> Option<&TerminalBuffer> { /* ... */ }
}
```

**Usage in Tests**:
```rust
#[tokio::test]
async fn test_no_blank_screen() {
    let capture = TestFrameCapture::new();
    app.run_with_capture(&capture).await?;

    assert!(capture.first_frame().is_some());
    assert!(capture.time_to_first_frame().as_millis() < 50);
}
```

---

### Component 3: TestAppBuilder

**Location**: `tui/tests/mocks/app_builder.rs`

**Purpose**: Fluent API for creating test apps

**API**:
```rust
pub struct TestAppBuilder {
    conductor: Option<MockConductorClient>,
    terminal: Option<TestTerminal>,
}

impl TestAppBuilder {
    pub fn new() -> Self { /* ... */ }
    pub fn with_conductor(self, client: MockConductorClient) -> Self { /* ... */ }
    pub fn with_terminal(self, term: TestTerminal) -> Self { /* ... */ }
    pub async fn build(self) -> anyhow::Result<(App, TestFrameCapture)> { /* ... */ }
}
```

**Usage in Tests**:
```rust
#[tokio::test]
async fn test_example() {
    let (app, capture) = TestAppBuilder::new()
        .with_conductor(MockConductorClient::with_delay(1000))
        .with_terminal(TestTerminal::new())
        .build()
        .await?;

    // ... run test
}
```

---

## Test Execution Flow

### Pre-Commit (Fast Tests Only)

```
Developer stages files
         ↓
Git triggers pre-commit hook
         ↓
Hook checks: Are there TUI changes?
    No → Skip tests, exit 0
    Yes ↓
Test 1: No blank screen (100ms)
         ↓
Test 2: Missing Conductor (200ms)
         ↓
Test 4: Offline Ollama (300ms)
         ↓
Test 5: Ctrl+C handling (500ms)
         ↓
(Test 3 skipped - too slow)
         ↓
Total: 1-2 seconds
         ↓
All pass → Allow commit ✅
Any fail → Reject commit ❌
```

### Continuous Integration (All Tests)

```
Push to GitHub
         ↓
GitHub Actions triggers
         ↓
Test 1-5 run in parallel or sequence
         ↓
Test 1: No blank (100ms)
Test 2: Missing Conductor (200ms)
Test 3: Slow Conductor (5.2s) ← Only in CI
Test 4: Offline Ollama (300ms)
Test 5: Ctrl+C handling (500ms)
         ↓
Total: 6-7 seconds
         ↓
All pass → Merge allowed ✅
Any fail → Block merge ❌
```

---

## Performance Budget

### Per-Test Timing

| Test | Baseline | Overhead | Total | Pre-Commit? |
|------|----------|----------|-------|-------------|
| 1 | 10ms | 20ms | ~50ms | YES |
| 2 | 50ms | 50ms | ~150ms | YES |
| 3 | 5000ms | 200ms | ~5.2s | NO |
| 4 | 50ms | 100ms | ~200ms | YES |
| 5 | 300ms | 100ms | ~400ms | YES |
| **Fast Sum** | - | - | **~1.5s** | - |
| **All Sum** | - | - | **~6.5s** | - |

**Budget Compliance**:
- Pre-commit (4 fast tests): 1-2 seconds ✅ (target: < 5s)
- CI (all 5 tests): 6-7 seconds ✅ (target: < 10s)

---

## Regression Prevention Strategy

### Test 1: Blank Screen Prevention

**Regression Path**: Someone removes loading screen, app renders after Conductor connects
- Test 1 catches this immediately (first frame must be non-empty within 50ms)
- Tight SLA (50ms) prevents slow initialization

**Coverage**: 100% (can't add blocking code without failing test)

---

### Test 2: Missing Conductor

**Regression Path**: Someone adds blocking `.unwrap()` on Conductor connection
- Test 2 runs without Conductor, verifies graceful error
- No panic assertions catch unwrap calls

**Coverage**: 100% (can't add unwrap without failing test)

---

### Test 3: Slow Connections

**Regression Path**: Someone adds synchronous operations during connect
- Test 3 verifies animation continues during 5s wait
- Frame count > 1 assertion catches blocking operations

**Coverage**: 100% (can't add sync code without failing test)

---

### Test 4: Ollama Failures

**Regression Path**: Someone adds `.expect()` on LLM requests
- Test 4 verifies graceful handling of errors
- No panic assertions catch expect calls

**Coverage**: 100% (can't add expect without failing test)

---

### Test 5: Signal Handling

**Regression Path**: Someone adds unsafe signal handlers or doesn't restore terminal
- Test 5 verifies Ctrl+C works during loading
- Terminal restoration checks catch cleanup issues

**Coverage**: 100% (can't corrupt terminal without failing test)

---

## Implementation Roadmap

### Phase 1: Mock Infrastructure (4-6 hours)
- Create `tui/tests/mocks/` module structure
- Implement `MockConductorClient`
- Implement `TestFrameCapture`
- Implement `TestAppBuilder`

### Phase 2: Test Implementation (4-6 hours)
- Create `tui/tests/initialization_test.rs`
- Implement all 5 test functions
- Add helper assertions
- Verify all tests pass locally

### Phase 3: Pre-Commit Integration (1-2 hours)
- Create `scripts/pre-commit-hooks.sh`
- Create `scripts/install-precommit-hook.sh`
- Test hook behavior
- Update documentation

### Phase 4: CI/CD Integration (1-2 hours)
- Create `.github/workflows/test.yml`
- Configure GitHub Actions
- Test in actual CI environment
- Update README with instructions

**Total Estimate**: 10-16 hours (2-3 days)

---

## Success Criteria

### Automated Tests
- ✅ All 5 test functions implemented
- ✅ All tests pass locally
- ✅ Pre-commit runs in < 5 seconds
- ✅ CI runs in < 10 seconds
- ✅ Mock infrastructure reusable

### Regression Prevention
- ✅ Blank screen impossible (Test 1 prevents)
- ✅ Missing Conductor handled gracefully (Test 2)
- ✅ Slow connections don't hang (Test 3)
- ✅ Ollama errors don't crash (Test 4)
- ✅ Ctrl+C works cleanly (Test 5)

### Developer Experience
- ✅ Easy to run: `cargo test --test initialization_test`
- ✅ Fast feedback on failures
- ✅ Clear error messages
- ✅ Can bypass with `--no-verify` if needed

### Documentation
- ✅ This design document (complete)
- ✅ Test comments in code
- ✅ README instructions
- ✅ Troubleshooting guide

---

## References

**Related Documents**:
- `TODO-tui-initialization.md` - STORY 6 & 7 (detailed implementation tasks)
- `tui/src/app.rs` - Current app initialization logic
- `tui/src/conductor_client.rs` - Conductor client interface
- `tui/tests/integration_test.rs` - Example integration test structure

**External References**:
- Tokio Testing Guide: https://tokio.rs/tokio/tutorial/select#select-1-using-biased
- Ratatui Testing: Backend trait for custom test backends
- Git Hooks: https://git-scm.com/book/en/v2/Customizing-Git-Git-Hooks

---

## Questions & Decisions

**Q: Why skip Test 3 in pre-commit?**
A: 5 second delay would make commits slow. Test 3 is important but runs in CI.

**Q: Why assert < 50ms for first frame?**
A: 50ms is imperceptible to users (human reaction time ~200ms). Ensures immediate feedback.

**Q: Can tests run in parallel?**
A: Yes, with `--test-threads=N`. Use `--test-threads=1` for serial execution if state isolation is needed.

**Q: What if Conductor fails to start during test?**
A: Test 2 specifically tests this scenario. Use mock that always fails.

**Q: How to debug test failures?**
A: Run with `--nocapture` to see debug output: `cargo test --test initialization_test -- --nocapture`

---

## Changelog

**2026-01-03**: Initial design document
- Completed specification for all 5 tests
- Mock infrastructure design finalized
- Pre-commit integration planned
- Performance budgets defined
