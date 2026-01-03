# CPU Performance Tests

This document describes the CPU performance test suite for detecting TUI performance regressions.

## Overview

The CPU performance tests measure processor usage during various TUI operations to catch regressions like **BUG-003: TUI Performance Regression**. They establish performance baselines and alert when CPU usage or render rates exceed targets.

## Test Files

- **`cpu_performance_test.rs`** - Main test suite with CPU measurement utilities and test cases

## Running Tests

```bash
# Run all CPU performance tests
cargo test --test cpu_performance_test

# Run with output to see measurements
cargo test --test cpu_performance_test -- --nocapture

# Run tests sequentially (recommended for accurate CPU measurement)
cargo test --test cpu_performance_test -- --nocapture --test-threads=1

# Run a specific test
cargo test --test cpu_performance_test test_cpu_load_during_idle -- --nocapture

# Run the manual baseline benchmark (ignored by default)
cargo test --test cpu_performance_test test_manual_baseline -- --nocapture --ignored
```

## Test Cases

### 1. `test_cpu_load_during_idle`

**Purpose**: Measures CPU usage when the Conductor is idle (no queries, no streaming)

**Expected**: CPU load < 2%

**Rationale**: When nothing is happening, the Conductor should sleep in `select!` and consume minimal CPU. High idle CPU suggests:
- Busy loops
- Polling instead of event-driven architecture
- Unnecessary background tasks

**Current Result**: ✅ PASS (0.00%)

---

### 2. `test_cpu_load_during_streaming`

**Purpose**: Measures CPU usage during response streaming

**Expected**: CPU load < 5%

**Rationale**: Streaming is mostly I/O-bound (receiving tokens from LLM). High CPU during streaming suggests:
- Expensive rendering operations per token
- Excessive re-rendering or layout calculations
- Inefficient string operations or memory allocations

**Test Configuration**:
- Token rate: 50 tokens/sec
- Total tokens: 250 (5 seconds of streaming)
- Delay between tokens: 20ms

**Current Result**: ✅ PASS (0.20%)

---

### 3. `test_render_rate_estimation`

**Purpose**: Estimates the message/render rate during streaming

**Expected**: Message rate < 30 msgs/sec (ideally 10-20 with batching)

**Rationale**: At 50 tokens/sec, we should NOT see 50 messages/sec if batching works. High message rates suggest:
- No batching of tokens
- Each token triggers a separate message
- Excessive re-renders

**Current Result**: ✅ PASS (0.8 msgs/sec)

---

### 4. `test_cpu_load_with_long_conversation`

**Purpose**: Measures CPU load when streaming with a long conversation history

**Expected**: CPU load < 5% even with long conversations

**Rationale**: From BUG-003, we know that conversation re-wrapping happens every frame. For long conversations, this can be expensive. High CPU with long conversations suggests:
- Re-wrapping entire conversation every frame
- No caching of wrapped text
- Inefficient text processing

**Test Configuration**:
- Builds 10-message conversation history
- Streams final response while measuring CPU

**Current Result**: ✅ PASS (0.40%)

---

### 5. `test_manual_baseline` (ignored)

**Purpose**: Manual benchmark to establish baseline CPU characteristics across all tests

**Usage**: Run with `--ignored` flag to execute

This is useful for establishing baselines on new hardware or after major architecture changes.

---

## CPU Measurement Implementation

The tests use `/proc/self/stat` on Linux for accurate CPU measurement:

```rust
struct CpuStats {
    utime: u64,      // User mode time (in clock ticks)
    stime: u64,      // System mode time (in clock ticks)
    total: u64,      // Total time (utime + stime)
    timestamp: Instant,
}

impl CpuStats {
    fn cpu_percent_since(&self, previous: &CpuStats) -> f64 {
        // Calculates CPU percentage between two measurements
        // Accounts for clock ticks (typically 100 Hz)
    }
}
```

**Measurement Methodology**:
1. Read CPU stats from `/proc/self/stat`
2. Sleep for measurement duration (5 seconds)
3. Read CPU stats again
4. Calculate percentage: `(cpu_time_delta / wall_time_delta) * 100`
5. Account for clock tick rate (100 Hz on most Linux systems)

**Note**: On non-Linux systems, CPU measurement gracefully falls back to 0.0% (tests pass but don't measure).

---

## Mock Backend

Tests use `CpuTestMockBackend` which simulates realistic streaming:

```rust
pub struct CpuTestMockBackend {
    tokens_per_response: usize,  // Total tokens in response
    token_delay_ms: u64,          // Delay between tokens
    request_count: AtomicUsize,   // Track requests made
}
```

**Presets**:
- `new_fast()` - 100 tokens/sec, 100 total (1 second) - for quick tests
- `new_realistic()` - 50 tokens/sec, 250 total (5 seconds) - matches real LLM behavior

---

## Performance Baselines (2026-01-03)

| Test | Target | Current | Status |
|------|--------|---------|--------|
| Idle CPU | < 2% | 0.00% | ✅ EXCELLENT |
| Streaming CPU | < 5% | 0.20% | ✅ EXCELLENT |
| Long conversation CPU | < 5% | 0.40% | ✅ EXCELLENT |
| Message rate | < 30/sec | 0.8/sec | ✅ EXCELLENT |

**System**: Linux 6.17.12-300.fc43.x86_64 (Fedora Silverblue)

**Notes**:
- Conductor itself has VERY low CPU usage
- Message batching is working well (0.8 msgs/sec vs 50 tokens/sec)
- The TUI performance regression (BUG-003) is NOT in the Conductor
- The issue is likely in TUI rendering (app.rs, compositor, theme animations)

---

## Integration with CI/CD

These tests should run in CI to catch regressions:

```bash
# In CI pipeline
cargo test --test cpu_performance_test --release -- --test-threads=1
```

**Notes**:
- Use `--release` for more realistic CPU measurements
- Use `--test-threads=1` for sequential execution (accurate CPU measurement)
- Tests take ~80 seconds to complete (4 tests × ~20 sec each)

---

## Troubleshooting

### Test fails with "CPU load too high"

1. Check if other processes are using CPU (run `top` or `htop`)
2. Verify CPU governor is not in performance mode
3. Run tests sequentially with `--test-threads=1`
4. Run on a quiet system (no heavy background tasks)

### CPU measurement shows 0.0%

This is expected on non-Linux systems where `/proc/self/stat` doesn't exist. The tests will pass but won't measure actual CPU usage.

### Message rate too high

If message rate exceeds 30/sec, check:
- Token batching implementation in Conductor
- Message channel buffer size (should batch before sending)
- Delay settings in mock backend

---

## Future Enhancements

- [ ] Add GPU usage measurement (if applicable)
- [ ] Add memory allocation tracking
- [ ] Add render frame time measurements
- [ ] Add TUI-specific tests (beyond Conductor)
- [ ] Add cross-platform CPU measurement (macOS, Windows)

---

## Related Documents

- **BUG-003-tui-performance-regression.md** - The bug this test suite was created to prevent
- **tui/tests/integration_test.rs** - General TUI integration tests
- **tui/tests/multi_model_integration_test.rs** - Multi-model routing tests
