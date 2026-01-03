# TUI Initialization Tests: Quick Reference

**TL;DR**: 5 integration tests prevent blank screen regressions and ensure graceful error handling.

---

## Test Summary Table

| # | Name | Scenario | Duration | Pre-Commit? | Purpose |
|---|------|----------|----------|------------|---------|
| 1 | No blank screen | Normal startup | 100ms | ✅ YES | Verify first frame renders <50ms |
| 2 | Missing Conductor | No daemon running | 150ms | ✅ YES | Error handling, graceful shutdown |
| 3 | Slow Conductor | 5s connection delay | 5.2s | ❌ NO | Animation responsiveness baseline |
| 4 | Offline Ollama | Backend unavailable | 200ms | ✅ YES | Service degradation, user interaction |
| 5 | Ctrl+C handling | SIGINT during load | 400ms | ✅ YES | Signal safety, terminal cleanup |

**Performance**:
- Pre-commit (4 fast tests): 1-2 seconds
- CI (all 5 tests): 6-7 seconds

---

## Running Tests

### Run All Tests
```bash
cd tui
cargo test --test initialization_test
```

### Run Single Test
```bash
cargo test --test initialization_test test_tui_no_blank_screen
```

### Run with Debug Output
```bash
cargo test --test initialization_test -- --nocapture
```

### Run Only Pre-Commit Tests (Skip Slow Test 3)
```bash
cargo test --test initialization_test \
  --skip test_conductor_slow_response
```

### Run in Release Mode (Faster)
```bash
cargo test --test initialization_test --release
```

---

## Mock Scenarios Quick Start

### Test 1: No Blank Screen
```rust
#[tokio::test]
async fn test_tui_no_blank_screen() {
    let (app, capture) = TestAppBuilder::new()
        .with_conductor(MockConductorClient::instant())
        .build()
        .await?;

    assert!(capture.time_to_first_frame().as_millis() < 50);
    assert!(!capture.first_frame().unwrap().is_empty());
}
```

### Test 2: Missing Conductor
```rust
#[tokio::test]
async fn test_conductor_not_running() {
    let (app, _capture) = TestAppBuilder::new()
        .with_conductor(MockConductorClient::fails("Connection refused"))
        .build()
        .await?;

    // App should exit gracefully with error message shown
    assert_eq!(app.exit_code, Some(1));
}
```

### Test 3: Slow Conductor
```rust
#[tokio::test]
#[ignore]  // Skip by default (too slow for pre-commit)
async fn test_conductor_slow_response() {
    let (app, capture) = TestAppBuilder::new()
        .with_conductor(MockConductorClient::with_delay(5000))
        .build()
        .await?;

    assert!(capture.frame_count() > 5);  // Multiple frames = animation
}
```

### Test 4: Offline Ollama
```rust
#[tokio::test]
async fn test_ollama_offline() {
    let (app, capture) = TestAppBuilder::new()
        .with_conductor(MockConductorClient::offline_ollama())
        .build()
        .await?;

    assert_contains(capture.last_frame(), "offline");
    // User should still be able to type
}
```

### Test 5: Ctrl+C During Loading
```rust
#[tokio::test]
async fn test_ctrl_c_during_loading() {
    let (mut app, _capture) = TestAppBuilder::new()
        .with_conductor(MockConductorClient::with_delay(1000))
        .build()
        .await?;

    // Simulate Ctrl+C after 300ms
    app.handle_key(KeyEvent::new(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
        KeyEventKind::Press,
    )).await;

    assert_eq!(app.exit_code, Some(0) | Some(130));  // Clean exit
}
```

---

## Troubleshooting

### Test Fails: "First frame is blank"
**Cause**: Loading screen not rendering immediately
**Fix**: Check that `App::run()` calls `terminal.draw()` before any blocking operations
**See**: `tui/src/app.rs` line ~245

### Test Fails: "Panic during conductor connection"
**Cause**: Code using `.unwrap()` or `.expect()` on Conductor connection
**Fix**: Use error handling instead: `match`, `?`, or `.ok()`
**See**: `tui/src/conductor_client.rs` line ~150

### Test Hangs (Timeout)
**Cause**: UI blocked waiting for Conductor response
**Fix**: Verify `tokio::select!` is being used in event loop
**See**: `tui/src/app.rs` line ~250-310

### Test Fails: "Terminal not restored"
**Cause**: Panic or early exit without cleanup
**Fix**: Check panic hook and terminal cleanup in main
**See**: `tui/src/main.rs` line ~34-56

### Can't Find Test File
**File Location**: `/var/home/machiyotl/src/ai-way/tui/tests/initialization_test.rs`
**Not Yet Created**: See STORY 6 in `TODO-tui-initialization.md`

---

## Pre-Commit Hook

### Install Hook
```bash
cd /var/home/machiyotl/src/ai-way
bash scripts/install-precommit-hook.sh
```

### Run Hook Manually
```bash
.git/hooks/pre-commit
```

### Skip Hook (Emergency Only)
```bash
git commit --no-verify -m "WIP: debugging"
```

### Remove Hook
```bash
rm .git/hooks/pre-commit
```

---

## CI/CD Integration

### GitHub Actions Workflow
**File**: `.github/workflows/test.yml`

Runs all 5 tests (including slow Test 3) on:
- Every push to main
- Every pull request

**View Results**: Check GitHub Actions tab in pull request

---

## Files & Locations

| File | Purpose | Status |
|------|---------|--------|
| `tui/tests/initialization_test.rs` | Main test file | ⏳ TODO |
| `tui/tests/mocks/mod.rs` | Mock exports | ⏳ TODO |
| `tui/tests/mocks/conductor_client.rs` | Mock conductor | ⏳ TODO |
| `tui/tests/mocks/terminal.rs` | Frame capture | ⏳ TODO |
| `scripts/pre-commit-hooks.sh` | Pre-commit script | ⏳ TODO |
| `scripts/install-precommit-hook.sh` | Hook installer | ⏳ TODO |
| `TUI-INITIALIZATION-TEST-DESIGN.md` | Full design doc | ✅ DONE |
| `TODO-tui-initialization.md` | Implementation tasks | ✅ UPDATED |

---

## Mock Infrastructure API

### MockConductorClient
```rust
pub enum MockMode {
    Instant,
    Delayed { ms: u64 },
    Fails { reason: String },
    OfflineOllama,
}

// Create instances
MockConductorClient::instant()              // Connects immediately
MockConductorClient::with_delay(5000)       // 5s delay
MockConductorClient::fails("reason")        // Connection fails
MockConductorClient::offline_ollama()       // Ollama errors
```

### TestFrameCapture
```rust
capture.first_frame()           // Get first rendered frame
capture.time_to_first_frame()   // Duration to first frame
capture.frame_count()           // Total frames rendered
capture.contains_text("text")   // Search all frames
capture.frame_at(index)         // Get specific frame
```

### TestAppBuilder
```rust
TestAppBuilder::new()
    .with_conductor(mock_client)
    .with_terminal(mock_terminal)
    .build()
    .await
```

---

## Performance Assertions

### Test 1: First Frame Timing
```rust
// Must render within 50ms of launch
assert!(capture.time_to_first_frame().as_millis() < 50);
```

### Test 3: Animation During Wait
```rust
// Multiple frames = animation is working
assert!(capture.frame_count() > 5, "Animation should update ~10 times");
```

### Test 5: Clean Shutdown
```rust
// Exit should be fast
assert!(shutdown_time.as_millis() < 500, "Should exit within 500ms");
```

---

## Common Assertions

```rust
// Frame content checks
assert!(!frame.is_empty());
assert_contains(&frame, "Loading");
assert!(!error_msg.contains("panic"));

// Behavior checks
assert!(connected);
assert!(!panic_occurred);
assert!(can_type_message);
assert!(terminal_restored);

// Timing checks
assert!(elapsed_ms < 50);
assert!(elapsed_ms >= 5000);
assert!(frame_count > 1);
```

---

## Implementation Status

### STORY 6: Add TUI Responsiveness Tests
- [ ] Create mock infrastructure
- [ ] Implement 5 test functions
- [ ] All tests passing
- [ ] Add to CI pipeline

### STORY 7: Wire Tests to Pre-Commit
- [ ] Create pre-commit hook script
- [ ] Create installation script
- [ ] Test hook behavior
- [ ] Document in README

---

## Next Steps

1. **Implement mocks** (4-6 hours)
   - `tui/tests/mocks/conductor_client.rs`
   - `tui/tests/mocks/terminal.rs`
   - `tui/tests/mocks/app_builder.rs`

2. **Write tests** (4-6 hours)
   - `tui/tests/initialization_test.rs`
   - All 5 test functions

3. **Pre-commit integration** (1-2 hours)
   - `scripts/pre-commit-hooks.sh`
   - Install script

4. **CI/CD setup** (1-2 hours)
   - `.github/workflows/test.yml`

**Total**: ~10-16 hours (2-3 days)

---

## Questions?

See full design: `TUI-INITIALIZATION-TEST-DESIGN.md`
See implementation tasks: `TODO-tui-initialization.md` STORY 6 & 7
