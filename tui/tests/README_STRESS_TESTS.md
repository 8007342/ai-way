# Stress Test Suite - ai-way TUI/Conductor

## Quick Start

```bash
# Run all stress tests (takes ~3-5 minutes)
cargo test --test stress_test --release -- --nocapture

# Run single test
cargo test --test stress_test stress_test_1m_short_messages --release -- --nocapture

# With verbose logging
RUST_LOG=stress_test=debug cargo test --test stress_test --release -- --nocapture
```

## What This Tests

The stress test suite pushes the TUI and Conductor to their absolute limits:

- **1M short messages** - Maximum throughput test
- **1M medium messages** - Buffer management test
- **1K long messages** - Memory pressure test
- **Rapid streaming** - Real-time responsiveness test
- **Backpressure handling** - Message loss detection test

## Success Criteria

✅ **Zero messages lost** - Every message is accounted for
✅ **No blocking** - All operations complete in reasonable time
✅ **Bounded memory** - No memory leaks or unbounded growth
✅ **CPU efficient** - < 20% average CPU usage
✅ **Backpressure works** - Graceful degradation under load
✅ **UI responsive** - Quit requests honored even under load

## Documentation

- **`STRESS_TEST_GUIDE.md`** - Comprehensive testing guide
- **`STRESS_TEST_RESULTS.md`** - Example outputs and analysis
- **`stress_test.rs`** - Test implementation

## Architecture Validated

These tests prove the async architecture is sound:

1. **Non-blocking polls** - `poll_streaming()` never blocks
2. **Channel backpressure** - Tokio channels prevent memory bloat
3. **Async all the way** - No blocking calls in async contexts
4. **Bounded buffers** - Memory usage is predictable

## Quick Example

```bash
$ cargo test --test stress_test stress_test_1m_short_messages --release -- --nocapture

running 1 test

=== Stress Test Results: 1M Short Messages ===
Configuration:
  Messages: 1000000
  Message size: 10 chars

Results:
  Total messages received: 1000245
  Total tokens received: 1000000
  Streams completed: 1
  Errors: 0
  Duration: 45.234s
  Throughput: 22124 tokens/sec
  Memory growth: 45.32 MB

✅ Test passed!

test stress_test_1m_short_messages ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out
```

## Why This Matters

Real-world usage can generate millions of messages:

- Long conversations with context
- Code generation streaming thousands of lines
- Multiple parallel specialist agents
- WebUI with multiple concurrent users

These tests ensure the system won't fall over under real load.

## CI/CD

These tests are designed for CI/CD:

- **Deterministic** - No flaky failures
- **Fast** - All tests complete in < 5 minutes (release build)
- **Informative** - Clear failure messages with diagnostics

## Baselines

Expected performance (4+ cores, 16GB RAM):

| Test | Throughput | Memory | Duration |
|------|------------|--------|----------|
| Short (1M) | > 10K msg/s | < 100 MB | < 60s |
| Medium (1M) | > 5K msg/s | < 500 MB | < 120s |
| Long (1K) | > 100 msg/s | < 50 MB | < 60s |
| Rapid | > 50K tok/s | N/A | < 60s |
| Backpressure | < 1% loss | N/A | < 120s |

## Next Steps

After tests pass:

1. ✅ Document results in `STRESS_TEST_RESULTS.md`
2. ✅ Setup CI/CD integration
3. ✅ Track performance over time
4. 🔄 Add more scenarios (concurrent conversations, error injection)
5. 🔄 Profile hotspots with flamegraph

## Support

For issues:

1. Check you're using `--release` build
2. Verify hardware meets baselines
3. Enable debug logging
4. Review documentation
5. File issue with full output

---

**Built to expose REAL bottlenecks. Make the system bulletproof.**
