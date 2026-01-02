# TODO-security-findings: Security Audit Tracking

**Created**: 2026-01-02
**Last Audit**: 2026-01-02 (Sprint 6 - H-001, H-002 resolved)
**Auditors**: Architect, Hacker, QA, Lawyer

---

## Overview

This document tracks security findings, vulnerabilities, and hardening requirements discovered during development and audits. Items are triaged by severity and tracked through resolution.

---

## Severity Levels

| Level | Definition | Response Time |
|-------|------------|---------------|
| **CRITICAL** | Actively exploitable, data breach risk | Immediate (block release) |
| **HIGH** | Exploitable with effort, significant impact | Next sprint |
| **MEDIUM** | Limited exploitability or impact | Within 2-3 sprints |
| **LOW** | Theoretical risk, defense-in-depth | Backlog |
| **INFO** | Best practice recommendations | As time permits |

---

## Active Findings

### CRITICAL

_None currently identified._

### HIGH

_None currently identified._

### MEDIUM

#### M-001: auth_token Field Unused
**Location**: `conductor/core/src/events.rs` (SurfaceCapabilities)
**Status**: Open
**Found**: Sprint 5
**Target**: Sprint 8
**Epic**: E-2026Q1-multi-surface
**Description**: The `auth_token` field exists but is never validated. False sense of security.
**Recommendation**: Either implement token validation or remove the field.

#### M-002: No Rate Limiting on Sprite Requests
**Location**: `conductor/core/src/avatar/security.rs`
**Status**: Implemented (Sprint 5 - P1.5)
**Found**: Sprint 5
**Epic**: E-2026Q1-avatar-animation
**Description**: Malicious client could flood sprite requests, exhausting server resources.
**Recommendation**: Implement token bucket rate limiting per connection.
**Resolution**: Pending tracker and rate limiting implemented in avatar/security.rs.

#### M-003: Unicode Character Validation Missing
**Location**: `conductor/core/src/avatar/security.rs`
**Status**: Implemented (Sprint 5 - P1.5)
**Found**: Sprint 5
**Epic**: E-2026Q1-avatar-animation
**Description**: No validation of Unicode characters in sprite blocks. Could allow rendering exploits or terminal escape sequences.
**Recommendation**: Whitelist allowed Unicode ranges per avatar constraints doc.
**Resolution**: Unicode validation implemented with ALLOWED_UNICODE_RANGES whitelist.

### LOW

#### L-001: Mock-Only Integration Tests
**Location**: `conductor/core/tests/`
**Status**: Open
**Found**: Sprint 4
**Description**: Integration tests use mocks, not real HTTP/JSON parsing. Edge cases in serialization could be missed.
**Recommendation**: Add end-to-end tests with real backend in CI.

#### L-002: No TUI Rendering Tests
**Location**: `tui/`
**Status**: Open
**Found**: Sprint 4
**Description**: Tests verify conductor logic but not actual terminal output. Visual regressions possible.
**Recommendation**: Add snapshot tests for key UI states.

### INFO

#### I-001: Socket Permissions
**Location**: `conductor/daemon/src/server.rs`
**Status**: Implemented
**Found**: Sprint 5 (Audit)
**Description**: Unix socket created with 0o600 permissions (owner-only). Good practice.
**Status**: No action needed - already implemented correctly.

#### I-002: Peer Credential Validation
**Location**: `conductor/daemon/src/server.rs`
**Status**: Implemented
**Found**: Sprint 5 (Audit)
**Description**: SO_PEERCRED used to validate connecting user matches daemon user.
**Status**: No action needed - already implemented correctly.

---

## Resolved Findings

### R-003: Connection Pool Reuse Not Implemented (was HIGH - H-002)
**Location**: `conductor/core/src/routing/connection_pool.rs`
**Resolved**: Sprint 6
**Epic**: E-2026Q1-multi-surface
**Description**: `PooledConnection::Drop` didn't return connections to pool. Under high load, this could lead to connection exhaustion (DoS vector).
**Resolution**: Implemented `Arc<Self>` pattern with `new_shared()` constructor, async mpsc unbounded channel for non-blocking connection returns from sync Drop handlers, RAII `PooledConnection` wrapper implementing `Deref` for transparent access, `process_returns()` for processing the return channel, `cleanup_idle()` for stale connection removal, and `start_cleanup_task()` for background maintenance. Test `scenario_7_connection_pool` now passes with >90% connection reuse ratio (achieved 100% reuse - only 1 connection created for 20 requests).

### R-002: Sequential ConnectionId Generation (was HIGH - H-001)
**Location**: `conductor/core/src/surface_registry.rs`
**Resolved**: Sprint 6
**Epic**: E-2026Q1-multi-surface
**Description**: ConnectionId used sequential counter (AtomicU64). Predictable IDs could allow connection hijacking if combined with other vulnerabilities.
**Resolution**: Replaced sequential counter with cryptographically random UUID v4. The `ConnectionId` type now wraps `uuid::Uuid` providing 122 bits of randomness, making ID prediction practically impossible. Tests verify uniqueness and unpredictability. Unix socket peer credential validation continues to provide defense-in-depth.

### R-001: QuickResponse Hard Latency Filter (was HIGH)
**Location**: `conductor/core/src/routing/policy.rs`
**Resolved**: Sprint 4
**Description**: Hard latency filter rejected all models for short messages, causing "NoModelsAvailable" errors.
**Resolution**: Changed to scoring-only approach. Latency influences selection but doesn't hard-filter.

---

## Audit Schedule

| Audit Type | Last Performed | Next Scheduled |
|------------|----------------|----------------|
| Security Review | Sprint 5 | Sprint 8 |
| Architecture Review | Sprint 3 | Sprint 6 |
| Full Audit | Sprint 5 | Sprint 10 |
| Penetration Testing | Never | Before v1.0 |

---

## Compliance Notes

### Data Handling
- Conversations stored in memory only (current implementation)
- No persistent storage of user data yet
- When persistence added, will need data retention policy

### Licensing
- All dependencies use permissive licenses (MIT, Apache-2.0)
- No GPL dependencies in core (would require license review)

### Third-Party Dependencies
- `tokio` - Rust async runtime (MIT)
- `serde` - Serialization (MIT/Apache-2.0)
- `ratatui` - TUI framework (MIT)
- `reqwest` - HTTP client (MIT/Apache-2.0)
- Full audit of dependency tree recommended before v1.0

---

## Adding New Findings

When adding a new finding:

1. Assign appropriate severity level
2. Use next available ID (H-XXX, M-XXX, L-XXX, I-XXX)
3. Include: Location, Status, Found (sprint), Description, Recommendation
4. Update "Last Audit" date in header
5. If resolving, move to "Resolved Findings" with resolution notes

---

**See Also**:
- `TODO-next.md` - Sprint priorities
- `workflows/sprint.md` - Audit schedule
- `docs/yollayah-avatar-constraints.md` - Avatar security requirements
