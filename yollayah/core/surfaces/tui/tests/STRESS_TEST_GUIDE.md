# Stress Test Guide - 1M Message Streaming

This document describes the comprehensive stress testing suite for the ai-way TUI and Conductor, designed to expose real bottlenecks under extreme load.

## Overview

The stress tests verify that the system can handle millions of messages without:
- Blocking or deadlocking
- Losing messages
- Leaking memory
- Becoming unresponsive
- Panicking under backpressure

## Test Scenarios

### 1. 1M Short Messages (10 chars each)

**Purpose**: Test maximum throughput with minimal payload.

**Configuration**:
- Messages: 1,000,000
- Size: 10 characters per message
- Delay: 0 (as fast as possible)
- Expected throughput: > 10,000 messages/second

**What it tests**:
- Message channel throughput
- Async task scheduling overhead
- Lock contention
- Message serialization/deserialization

**Success criteria**:
- ✅ All 1M messages received
- ✅ Throughput > 10K msg/s
- ✅ Zero errors
- ✅ Completes in < 60 seconds

**Run**:
```bash
cargo test --test stress_test stress_test_1m_short_messages --release -- --nocapture
```

### 2. 1M Medium Messages (100 chars each)

**Purpose**: Test buffer management and moderate payload handling.

**Configuration**:
- Messages: 1,000,000
- Size: 100 characters per message
- Total data: ~100 MB
- Expected throughput: > 5,000 messages/second

**What it tests**:
- Memory allocation efficiency
- Buffer management
- String handling overhead
- Garbage collection pressure

**Success criteria**:
- ✅ All 1M messages received
- ✅ Memory growth < 500 MB
- ✅ Throughput > 5K msg/s
- ✅ Completes in < 120 seconds

**Run**:
```bash
cargo test --test stress_test stress_test_1m_medium_messages --release -- --nocapture
```

### 3. 1K Long Messages (10K chars each)

**Purpose**: Test memory pressure with large payloads.

**Configuration**:
- Messages: 1,000
- Size: 10,000 characters per message
- Total data: ~10 MB
- Expected throughput: > 100 messages/second

**What it tests**:
- Large allocation handling
- Memory fragmentation
- Buffer overflow handling
- String concatenation efficiency

**Success criteria**:
- ✅ All 1K messages received
- ✅ Memory usage bounded
- ✅ Throughput > 100 msg/s
- ✅ Completes in < 60 seconds

**Run**:
```bash
cargo test --test stress_test stress_test_1k_long_messages --release -- --nocapture
```

### 4. Rapid Token Streaming (1M tokens in 10s)

**Purpose**: Test real-time streaming performance with time pressure.

**Configuration**:
- Messages: 1,000,000
- Size: 5 characters per token
- Delay: 10 microseconds (100K tokens/sec)
- Duration: ~10 seconds

**What it tests**:
- Real-time streaming responsiveness
- UI update throttling
- Quit responsiveness under load
- Time-sensitive event handling

**Success criteria**:
- ✅ All 1M tokens received
- ✅ Stream completes in < 60s
- ✅ Quit handled in < 1s
- ✅ Zero errors

**Run**:
```bash
cargo test --test stress_test stress_test_rapid_token_streaming --release -- --nocapture
```

### 5. Backpressure and Message Loss Detection

**Purpose**: Verify correct backpressure handling without message loss.

**Configuration**:
- Messages: 100,000
- Channel buffer: 100 (small)
- Consumer delay: 100μs per message (slow consumer)

**What it tests**:
- Backpressure application
- Message loss detection
- Slow consumer handling
- Channel saturation recovery

**Success criteria**:
- ✅ All messages accounted for
- ✅ Message loss < 1%
- ✅ No panics or deadlocks
- ✅ System recovers gracefully

**Run**:
```bash
cargo test --test stress_test stress_test_backpressure_handling --release -- --nocapture
```

## Performance Baselines

Expected performance on modern hardware (4+ cores, 16GB RAM):

| Metric | Target | Scenario |
|--------|--------|----------|
| Throughput (short) | > 10K msg/s | 1M short messages |
| Throughput (medium) | > 5K msg/s | 1M medium messages |
| Throughput (long) | > 100 msg/s | 1K long messages |
| Memory growth | < 100 MB | 1M short messages |
| Memory growth | < 500 MB | 1M medium messages |
| Latency (p99) | < 100ms | Message delivery |
| CPU usage | < 20% | Average during test |
| Quit latency | < 1s | Under load |

## Running All Tests

```bash
# Run all stress tests (will take several minutes)
cargo test --test stress_test --release -- --nocapture

# Run with verbose logging
RUST_LOG=stress_test=debug cargo test --test stress_test --release -- --nocapture

# Run single test with timing
time cargo test --test stress_test stress_test_1m_short_messages --release -- --nocapture
```

## Understanding Results

### Example Output

```
=== Stress Test Results: 1M Short Messages ===
Configuration:
  Messages: 1000000
  Message size: 10 chars

Results:
  Total messages received: 1000245
  Total tokens received: 1000000
  Streams completed: 1
  Errors: 0
  Duration: 45.2s
  Throughput: 22124 tokens/sec
  Memory growth: 45.32 MB

✅ Test passed!
```

### Interpreting Metrics

**Total messages received**: Should be close to expected count (may include control messages)

**Total tokens received**: Should exactly match sent count (1M for short/medium tests)

**Throughput**:
- Short messages: > 10K is good, > 50K is excellent
- Medium messages: > 5K is good, > 20K is excellent
- Long messages: > 100 is good, > 500 is excellent

**Memory growth**:
- < 100 MB for 1M short messages = excellent
- < 500 MB for 1M medium messages = good
- Unbounded growth = memory leak (FAIL)

**Errors**: Should always be 0

## Common Issues and Solutions

### Test Timeout

**Symptom**: Test exceeds 60s/120s timeout

**Possible causes**:
- Deadlock in message handling
- Blocked channel (backpressure not working)
- CPU starvation (too few tokio workers)

**Solutions**:
- Check for blocking calls in async code
- Verify channel buffer sizes
- Increase tokio worker threads: `#[tokio::test(flavor = "multi_thread", worker_threads = 8)]`

### Message Loss

**Symptom**: Tokens received < tokens sent

**Possible causes**:
- Channel overflow with drop policy
- Receiver task panicked
- Race condition in message counting

**Solutions**:
- Increase channel buffer size
- Check for panics in consumer task
- Verify atomic operations are Relaxed/SeqCst as needed

### High Memory Usage

**Symptom**: Memory growth > 500 MB for medium messages

**Possible causes**:
- Messages not being freed after processing
- String concatenation creating unnecessary copies
- Buffer not being cleared

**Solutions**:
- Check for message retention in Vec/HashMap
- Use `String::with_capacity()` for known sizes
- Clear buffers after processing

### Low Throughput

**Symptom**: Throughput significantly below baseline

**Possible causes**:
- Debug build (use `--release`)
- CPU throttling (thermal/power management)
- Lock contention
- Blocking operations in hot path

**Solutions**:
- Always use `--release` for benchmarks
- Check CPU frequency scaling
- Use async primitives (tokio::Mutex, not std::sync::Mutex)
- Avoid `block_on()` in async contexts

## Implementation Details

### Mock Backend

The `StressTestBackend` is a high-performance mock that can stream millions of messages:

```rust
pub struct StressTestBackend {
    config: StressTestConfig,
    request_count: AtomicUsize,
    total_messages_sent: Arc<AtomicU64>,
}
```

**Features**:
- Configurable message count, size, and delay
- Non-blocking streaming (uses tokio::spawn)
- Batching support for reduced overhead
- Message tracking for loss detection

### Message Tracker

The `MessageTracker` accumulates statistics during tests:

```rust
pub struct MessageTracker {
    total_received: Arc<AtomicU64>,
    tokens_received: Arc<AtomicU64>,
    streams_completed: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
    // ...
}
```

**Features**:
- Lock-free atomic counters
- Throughput calculation
- Optional individual message tracking (expensive)

### Consumer Task

Each test spawns a consumer task that drains the message channel:

```rust
tokio::spawn(async move {
    while let Some(msg) = rx.recv().await {
        tracker.record_message(&msg);
        // Process message...
    }
});
```

This simulates the TUI's event loop processing messages.

## Architecture Validation

These tests validate the core async architecture decisions:

### ✅ Non-blocking Poll

`conductor.poll_streaming().await` must never block, even with millions of pending tokens.

**Validation**: Rapid token test measures poll latency < 100ms

### ✅ Channel Backpressure

Tokio channels apply backpressure when buffer fills, preventing unbounded memory growth.

**Validation**: Backpressure test with slow consumer verifies no message loss

### ✅ Async All The Way

No blocking operations in async contexts.

**Validation**: All tests complete without deadlocks

### ✅ Bounded Memory

Message buffers have limits to prevent memory exhaustion.

**Validation**: Memory growth tests verify bounded growth

## CI/CD Integration

These tests are suitable for CI/CD pipelines:

```yaml
- name: Run stress tests
  run: |
    cargo test --test stress_test --release -- --nocapture
  timeout-minutes: 10
```

**Notes**:
- Always use `--release` (debug is 10-100x slower)
- Set reasonable timeout (10 minutes for all tests)
- Tests are deterministic (no flakiness)

## Future Enhancements

Potential additions to the stress test suite:

1. **Concurrent Conversations**: Multiple parallel streams
2. **Mixed Load**: Combination of short/medium/long messages
3. **Error Injection**: Random failures during streaming
4. **Network Simulation**: Artificial latency/jitter
5. **CPU Profiling**: Flamegraph generation
6. **Memory Profiling**: Heap allocation tracking

## Related Documentation

- `integration_test.rs` - Basic integration tests
- `cpu_performance_test.rs` - CPU usage tests
- `chaos_tests.rs` - Error injection tests (conductor)
- `TODO-async-architecture-review.md` - Async guidelines

## Support

If stress tests fail:

1. Check you're using `--release` build
2. Verify hardware meets baselines (4+ cores, 16GB RAM)
3. Review test output for specific failure
4. Check system resource usage (htop/top)
5. Enable debug logging: `RUST_LOG=stress_test=debug`

For persistent failures, file an issue with:
- Full test output
- System specs (CPU, RAM, OS)
- Rust version (`rustc --version`)
- Release vs debug build
