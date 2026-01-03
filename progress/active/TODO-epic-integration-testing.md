# TODO-epic-integration-testing: Comprehensive Integration Testing Framework

**Created**: 2026-01-02
**Status**: PLANNING
**Priority**: HIGH
**Type**: EPIC
**Owner**: Architect + Hacker + Crazy Intern

---

## Epic Goal

Build a comprehensive, multi-tier integration testing framework that:
1. Validates async/non-blocking behavior
2. Catches regressions before merge
3. Runs at multiple points in build lifecycle
4. Scales from fast smoke tests to heavy load tests
5. Works with `--test` mode for rapid iteration

---

## Background

### Current State

**What We Have**:
- Unit tests in Rust workspace (`cargo test`)
- Manual testing via `./yollayah.sh`
- Test mode for fast iteration (`--test`)
- Basic bash syntax validation

**What's Missing**:
- Automated integration tests
- TUI interaction testing
- Async behavior validation
- Performance regression detection
- Pre-merge validation
- CI/CD pipeline integration

### Why This Matters

**Async Requirements**:
- TUI and Conductor must be fully async
- Need to test responsiveness under load
- Catch blocking calls in tests
- Verify concurrency behavior

**Quality Gates**:
- No regressions before merge
- Performance stays within bounds
- User experience validated
- Breaking changes detected

---

## Test Tiers

### Tier 1: Fast Smoke Tests (< 1 minute)

**Purpose**: Quick validation for every commit

**Scope**:
- Cargo unit tests
- TUI launches cleanly
- Basic message send/receive
- Ollama connectivity

**Run On**:
- Pre-commit hook
- Every git commit
- Developer workstation

**Implementation**:
```bash
#!/bin/bash
# tests/smoke/run.sh

set -e

echo "Running Tier 1: Smoke Tests"

# Rust unit tests
cargo test --workspace --quiet

# TUI smoke test (with --test mode)
timeout 60 ./yollayah.sh --test << 'EOF'
/quit
EOF

echo "✅ Tier 1 passed"
```

---

### Tier 2: Integration Tests (< 10 minutes)

**Purpose**: Full feature validation before merge

**Scope**:
- Multi-turn conversations
- Model switching
- Error handling
- TUI responsiveness
- Conductor async behavior

**Run On**:
- Pre-push hook
- Pull request CI
- Before merge to main

**Implementation**:
```bash
#!/bin/bash
# tests/integration/run.sh

set -e

echo "Running Tier 2: Integration Tests"

# TUI interaction tests
./tests/integration/test_tui_responsiveness.sh
./tests/integration/test_message_flow.sh
./tests/integration/test_error_handling.sh

# Conductor tests
./tests/integration/test_conductor_async.sh
./tests/integration/test_model_switching.sh

echo "✅ Tier 2 passed"
```

---

### Tier 3: Heavy Load Tests (< 30 minutes)

**Purpose**: Performance validation and stress testing

**Scope**:
- Concurrent requests
- Large message throughput
- Memory leak detection
- CPU usage profiling
- Response time benchmarks

**Run On**:
- Nightly builds
- Release candidates
- Manual performance testing

**Implementation**:
```bash
#!/bin/bash
# tests/heavy/run.sh

set -e

echo "Running Tier 3: Heavy Load Tests"

# Performance benchmarks
cargo bench --workspace

# Load testing
./tests/heavy/concurrent_requests.sh
./tests/heavy/memory_profiling.sh
./tests/heavy/response_time_benchmark.sh

echo "✅ Tier 3 passed"
```

---

## Test Scenarios

### S1: TUI Responsiveness

**Goal**: Verify TUI never blocks user input

**Test**:
```rust
#[tokio::test]
async fn test_tui_keyboard_responsiveness() {
    // Start TUI
    let mut tui = launch_tui_test_mode().await;

    // Send rapid keystrokes
    for _ in 0..100 {
        tui.send_key('a').await;
    }

    // Verify all keystrokes processed within 1 second
    assert!(tui.wait_for_input_processed(Duration::from_secs(1)).await);
}
```

### S2: Conductor Async Operations

**Goal**: Verify Conductor handles concurrent requests

**Test**:
```rust
#[tokio::test]
async fn test_conductor_concurrent_models() {
    let conductor = Conductor::new_test().await;

    // Send 10 concurrent requests
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let msg = format!("Request {}", i);
            conductor.send_message(msg)
        })
        .collect();

    // All should complete within 5 seconds
    let results = tokio::time::timeout(
        Duration::from_secs(5),
        futures::future::join_all(handles)
    ).await;

    assert!(results.is_ok());
}
```

### S3: Model Download Non-Blocking

**Goal**: Verify TUI usable during model download

**Test**:
```bash
#!/bin/bash
# tests/integration/test_background_download.sh

# Remove test model
ollama rm qwen2:0.5b 2>/dev/null || true

# Launch TUI in background
timeout 60 ./yollayah.sh --test &
TUI_PID=$!

# Wait for TUI to appear
sleep 2

# Send test message while model downloading
# Should get response or clear error, not hang
kill -0 $TUI_PID && echo "✅ TUI still responsive"
```

### S4: Error Handling

**Goal**: Verify graceful degradation on failures

**Test Cases**:
- Ollama not running
- Network disconnected during git sync
- Model not found
- Out of memory
- Corrupted model file

### S5: Memory Leaks

**Goal**: Detect memory leaks in long-running sessions

**Test**:
```rust
#[tokio::test]
async fn test_no_memory_leak() {
    let mut tui = launch_tui_test_mode().await;

    let initial_mem = get_memory_usage();

    // Send 1000 messages
    for i in 0..1000 {
        tui.send_message(format!("Test {}", i)).await;
        tui.wait_for_response().await;
    }

    let final_mem = get_memory_usage();

    // Memory should not grow more than 50MB
    assert!(final_mem - initial_mem < 50_000_000);
}
```

---

## Build Lifecycle Integration

### Pre-Commit Hook

**Location**: `.git/hooks/pre-commit`

**Purpose**: Catch obvious errors before commit

**Tests**:
- Cargo test (unit tests)
- Cargo clippy (lints)
- Basic smoke test with `--test`

**Time Budget**: < 30 seconds

```bash
#!/bin/bash
# .git/hooks/pre-commit

set -e

echo "🔍 Pre-commit validation..."

# Rust checks
cargo test --workspace --quiet
cargo clippy --workspace --quiet -- -D warnings

# Quick smoke test
timeout 30 ./yollayah.sh --test << 'EOF' || {
    echo "❌ TUI smoke test failed"
    exit 1
}
/quit
EOF

echo "✅ Pre-commit checks passed"
```

### Pre-Push Hook

**Location**: `.git/hooks/pre-push`

**Purpose**: Full validation before pushing to remote

**Tests**:
- All unit tests
- Integration tests
- Async behavior validation

**Time Budget**: < 5 minutes

```bash
#!/bin/bash
# .git/hooks/pre-push

set -e

echo "🔍 Pre-push validation..."

# Full test suite
cargo test --workspace

# Integration tests
./tests/integration/run.sh

# Async audit
./tests/async/check_blocking_calls.sh

echo "✅ Pre-push checks passed"
```

### CI/CD Pipeline

**Location**: `.github/workflows/ci.yml`

**Purpose**: Automated testing on every PR

**Tests**:
- Tier 1 (smoke)
- Tier 2 (integration)
- Tier 3 (heavy - on main branch only)

**Time Budget**: < 15 minutes

```yaml
name: CI

on: [pull_request, push]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Install Ollama
        run: curl -fsSL https://ollama.com/install.sh | sh

      - name: Tier 1 - Smoke Tests
        run: ./tests/smoke/run.sh

      - name: Tier 2 - Integration Tests
        run: ./tests/integration/run.sh

      - name: Tier 3 - Heavy Tests (main only)
        if: github.ref == 'refs/heads/main'
        run: ./tests/heavy/run.sh
```

---

## Test Infrastructure

### Test Utilities

```rust
// tests/common/mod.rs

pub async fn launch_tui_test_mode() -> TestTUI {
    // Launch TUI with test flag
    // Return handle for scripted interaction
}

pub fn get_memory_usage() -> usize {
    // Get current process memory
}

pub async fn wait_for_tui_ready(timeout: Duration) -> Result<()> {
    // Wait for TUI to be interactive
}

pub fn mock_ollama_server() -> MockServer {
    // Mock Ollama for controlled testing
}
```

### Test Mocks

**Mock Ollama**:
- Simulates slow model downloads
- Controlled response times
- Error injection
- Network failure simulation

**Mock TUI Input**:
- Scripted keyboard events
- Mouse events
- Terminal resize events

---

## Tasks Breakdown

### Sprint 1: Foundation (Current)

- [x] Create Epic tracking document
- [ ] **T1.1**: Create test directory structure
- [ ] **T1.2**: Implement test utilities (launch_tui_test_mode, etc)
- [ ] **T1.3**: Write first smoke test
- [ ] **T1.4**: Add pre-commit hook

**Owner**: Crazy Intern + Hacker

### Sprint 2: Integration Tests

- [ ] **T2.1**: TUI responsiveness tests
- [ ] **T2.2**: Conductor async tests
- [ ] **T2.3**: Error handling tests
- [ ] **T2.4**: Add pre-push hook

**Owner**: Hacker + Crazy Intern

### Sprint 3: Heavy Tests

- [ ] **T3.1**: Performance benchmarks
- [ ] **T3.2**: Memory leak detection
- [ ] **T3.3**: Load testing
- [ ] **T3.4**: CI/CD pipeline setup

**Owner**: Architect + Hacker

### Sprint 4: Continuous Improvement

- [ ] **T4.1**: Add test coverage reporting
- [ ] **T4.2**: Async profiling tools
- [ ] **T4.3**: Regression tracking
- [ ] **T4.4**: Performance dashboards

**Owner**: Architect + Crazy Intern

---

## Success Metrics

### Coverage

- ✅ 80%+ code coverage (unit tests)
- ✅ All critical paths tested (integration)
- ✅ Performance baselines established

### Quality Gates

- ✅ Zero regressions in main branch
- ✅ All async code validated
- ✅ Performance within 10% of baseline

### Developer Experience

- ✅ Fast feedback (< 1 min for smoke tests)
- ✅ Clear test failure messages
- ✅ Easy to run locally
- ✅ Automated in CI/CD

---

## Related Documents

- `TODO-async-architecture-review.md` - Async validation requirements
- `CLAUDE.md` - Build commands and testing instructions
- `TODO-sprint-10-blocking-fix.md` - Previous testing approach

---

## Notes

**Using --test Mode**:
- Leverages tiny model for fast tests
- Skips non-essential operations
- Perfect for smoke tests and basic integration
- Can run hundreds of times per day

**Async Validation**:
- Tests must verify non-blocking behavior
- Check responsiveness under load
- Detect blocking calls in async context
- Validate concurrent operations

**Incremental Rollout**:
- Start with smoke tests (Sprint 1)
- Add integration (Sprint 2)
- Heavy tests last (Sprint 3)
- Don't block current development

---

**Owner**: Architect + Hacker + Crazy Intern
**Last Updated**: 2026-01-02
**Status**: PLANNING (Epic created, Sprint 1 tasks defined)
