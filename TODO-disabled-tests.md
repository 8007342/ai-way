# Disabled Tests Tracking

**Generated**: 2026-01-02
**Last Updated**: 2026-01-02 (Sprint 5)

This document tracks all `#[ignore]` tests across the codebase. Update this file when tests are added, removed, or fixed.

---

## Summary

| Total Ignored | High Priority | Medium Priority | Low Priority | Intentional |
|---------------|---------------|-----------------|--------------|-------------|
| 2             | 1             | 0               | 0            | 1           |

---

## Ignored Tests by Crate

### conductor-core

| Test Name | File:Line | Reason | Epic/Feature | Target Sprint | Status | Priority |
|-----------|-----------|--------|--------------|---------------|--------|----------|
| `scenario_7_connection_pool` | `conductor/core/tests/routing_performance_tests.rs:461` | Connection pool reuse not implemented | H-002 (Security Finding) | Sprint 6 | Pending fix | HIGH |
| `scenario_10_stress_test` | `conductor/core/tests/routing_performance_tests.rs:653` | Long-running test (10+ min), run manually | N/A | N/A | Intentional | LOW |

### yollayah-tui

*No ignored tests currently.*

### conductor-daemon

*No ignored tests currently.*

---

## Detailed Test Information

### scenario_7_connection_pool

**Location**: `/var/home/calmecacpilli/src/ai-way/conductor/core/tests/routing_performance_tests.rs:461`

**Ignore Annotation**:
```rust
#[ignore = "Pre-existing failure: connection pool reuse not implemented"]
```

**Description**: Tests connection pool behavior including:
- Connection reuse ratio > 90%
- No connection leaks
- Idle connections cleaned up

**Why Disabled**: The `ConnectionPool` implementation creates new connections but does not properly return them to the pool for reuse. This was identified as security finding H-002 (DoS prevention under high load).

**Fix Plan**:
1. Refactor `ConnectionPool` to use `Arc<Self>`
2. Add async channel for connection returns
3. Implement proper connection lifecycle management

**Related Files**:
- `conductor/core/src/routing/connection_pool.rs`
- `TODO-security-findings.md` (H-002)

**Owner**: Backend Team
**Target**: Sprint 6

---

### scenario_10_stress_test

**Location**: `/var/home/calmecacpilli/src/ai-way/conductor/core/tests/routing_performance_tests.rs:653`

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
| *None yet* | | | |

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

**As of Sprint 5**:
- **conductor-core**: 378 tests passing, 2 ignored
- **yollayah-tui**: 26 tests passing, 0 ignored
- **Total**: 506+ tests passing, 2 ignored (11 doc tests setup-dependent)

---

**Next Review**: Sprint 6 (after connection pool refactor)
