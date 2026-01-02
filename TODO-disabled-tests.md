# Disabled Tests Tracking

**Generated**: 2026-01-02
**Last Updated**: 2026-01-02 (Sprint 6 - scenario_7_connection_pool fixed)

This document tracks all `#[ignore]` tests across the codebase. Update this file when tests are added, removed, or fixed.

---

## Summary

| Total Ignored | High Priority | Medium Priority | Low Priority | Intentional |
|---------------|---------------|-----------------|--------------|-------------|
| 5             | 0             | 0               | 0            | 5           |

---

## Ignored Tests by Crate

### conductor-core

| Test Name | File:Line | Reason | Epic/Feature | Target Sprint | Status | Priority |
|-----------|-----------|--------|--------------|---------------|--------|----------|
| `scenario_10_stress_test` | `conductor/core/tests/routing_performance_tests.rs:726` | Long-running test (10+ min), run manually | N/A | N/A | Intentional | LOW |
| `chaos_socket_close_mid_frame` | `conductor/core/tests/chaos_tests.rs` | Chaos test - connection dies mid-frame | Transport Resilience | N/A | Intentional (chaos) | LOW |
| `chaos_backend_hang` | `conductor/core/tests/chaos_tests.rs` | Chaos test - backend stops responding | Transport Resilience | N/A | Intentional (chaos) | LOW |
| `chaos_session_memory_pressure` | `conductor/core/tests/chaos_tests.rs` | Chaos test - many sessions, memory pressure | Transport Resilience | N/A | Intentional (chaos) | LOW |
| `chaos_concurrent_pruning` | `conductor/core/tests/chaos_tests.rs` | Chaos test - concurrent operations during cleanup | Transport Resilience | N/A | Intentional (chaos) | LOW |

### yollayah-tui

*No ignored tests currently.*

### conductor-daemon

*No ignored tests currently.*

---

## Detailed Test Information

### scenario_10_stress_test

**Location**: `/var/home/calmecacpilli/src/ai-way/conductor/core/tests/routing_performance_tests.rs:726`

**Ignore Annotation**:
```rust
#[ignore] // Long-running test, run manually
```

**Description**: Extended stress test that runs for 60+ seconds at 50 RPS to verify:
- Error rate < 1%
- P99 latency < 5s
- No memory leaks
- No connection exhaustion

**Why Disabled**: Intentionally ignored for regular CI runs due to long execution time. Should be run manually before releases or during dedicated performance testing.

**How to Run**:
```bash
cargo test --package conductor-core scenario_10_stress_test -- --ignored --nocapture
```

**Owner**: QA Team
**Target**: N/A (Intentional)

---

### Chaos Tests (Sprint 8)

**Location**: `/var/home/calmecacpilli/src/ai-way/conductor/core/tests/chaos_tests.rs`

These tests verify system resilience under adverse conditions. All are marked `#[ignore]` with "Intentional (chaos)" reason.

#### chaos_socket_close_mid_frame

**Description**: Simulates connection dying during message transmission:
- Partial frame detection
- Graceful error handling without panic
- No resource leaks from interrupted transmissions
- Proper cleanup of decoder state

**Pass Criteria**: < 1% unexpected errors, no resource leaks

#### chaos_backend_hang

**Description**: Simulates a backend that stops responding:
- Timeout triggers correctly
- Connection marked as unhealthy
- System continues to function after timeout
- Graceful degradation under timeout conditions

**Pass Criteria**: < 5% unexpected errors, no resource leaks, timeouts detected

#### chaos_session_memory_pressure

**Description**: Creates many concurrent sessions to verify memory stays bounded:
- Memory stays bounded under high session count
- Old sessions can be evicted properly
- No unbounded growth in data structures
- Registry handles high connection counts

**Pass Criteria**: No resource leaks, bounded memory usage

#### chaos_concurrent_pruning

**Description**: Tests concurrent operations during cleanup/pruning:
- No deadlocks during concurrent register/unregister/prune
- Data integrity maintained during concurrent access
- No races leading to corrupted state
- Proper lock ordering and contention handling

**Pass Criteria**: No deadlocks, > 50% success rate, no unexpected errors

**How to Run All Chaos Tests**:
```bash
cargo test --package conductor-core chaos -- --ignored --nocapture
```

**How to Run Individual Chaos Test**:
```bash
cargo test --package conductor-core chaos_socket_close_mid_frame -- --ignored --nocapture
```

**Owner**: QA Team
**Target**: N/A (Intentional - chaos tests for resilience verification)

---

## Ignore Annotation Convention

When adding `#[ignore]` to a test, use the following format to enable automated tracking:

```rust
#[test]
#[ignore = "<Epic-ID>: <brief reason>, Sprint <N>"]
fn test_name() { ... }
```

**Examples**:
```rust
#[ignore = "E-2026Q1-avatar: sprite cache not implemented, Sprint 6"]
fn test_sprite_caching() { ... }

#[ignore = "H-002: connection pool reuse needed, Sprint 6"]
fn test_pool_reuse() { ... }

#[ignore] // Long-running test, run manually
fn stress_test() { ... }
```

**Fields**:
- **Epic-ID**: Reference to TODO file or security finding (e.g., `E-2026Q1-avatar`, `H-002`)
- **Brief reason**: What's blocking the test
- **Sprint N**: Target sprint for fix (omit for intentional ignores)

---

## Process

### Adding a Disabled Test

1. Add `#[ignore = "..."]` annotation following the convention above
2. Update this file with a new entry in the appropriate crate section
3. If HIGH priority, add to `TODO-next.md` disabled tests section
4. If security-related, reference in `TODO-security-findings.md`

### Fixing a Disabled Test

1. Implement the fix
2. Remove the `#[ignore]` annotation
3. Run the test to verify: `cargo test <test_name>`
4. Update this file: move entry to "Recently Fixed" section
5. Update `TODO-next.md` if applicable

### Reviewing Disabled Tests

During sprint planning:
1. Review all HIGH priority ignored tests
2. Assess if blockers are resolved
3. Assign ownership and target sprints
4. Update this document accordingly

---

## Recently Fixed

| Test Name | File | Fixed Sprint | Notes |
|-----------|------|--------------|-------|
| `scenario_7_connection_pool` | `conductor/core/tests/routing_performance_tests.rs` | Sprint 6 | H-002: Connection pool reuse implemented with Arc<Self> pattern, async mpsc channel for returns, RAII PooledConnection with Deref, and idle timeout cleanup. Connection reuse ratio >90% achieved. |

---

## Audit Commands

Find all ignored tests in the codebase:
```bash
grep -rn '#\[ignore' --include='*.rs' conductor/ tui/
```

Run all ignored tests (for manual verification):
```bash
cargo test -- --ignored
```

Run specific ignored test:
```bash
cargo test --package conductor-core <test_name> -- --ignored --nocapture
```

---

## Test Statistics

**As of Sprint 8**:
- **conductor-core**: 419+ tests passing (includes 2 chaos infrastructure sanity tests), 5 ignored
- **yollayah-tui**: 26+ tests passing, 0 ignored
- **Total**: 547+ tests passing, 5 ignored (1 stress test + 4 chaos tests - all intentional)

**Chaos Tests Added in Sprint 8**:
- `chaos_socket_close_mid_frame` - connection dies mid-frame
- `chaos_backend_hang` - backend stops responding
- `chaos_session_memory_pressure` - many sessions, high memory
- `chaos_concurrent_pruning` - concurrent operations during cleanup

---

**Next Review**: Sprint 9
