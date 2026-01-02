# TODO-security-findings: Security Audit Tracking

**Created**: 2026-01-02
**Last Audit**: 2026-01-02 (Sprint 5 - Initial creation)
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

#### H-001: Sequential ConnectionId Generation
**Location**: `conductor/core/src/surface.rs`
**Status**: Open
**Found**: Sprint 5
**Description**: ConnectionId uses sequential counter (AtomicU64). Predictable IDs could allow connection hijacking if combined with other vulnerabilities.
**Recommendation**: Use cryptographically random UUIDs for production.
**Mitigation**: Unix socket peer credential validation provides defense-in-depth.

#### H-002: Connection Pool Reuse Not Implemented
**Location**: `conductor/core/src/routing/pool.rs`
**Status**: Open
**Found**: Sprint 4
**Description**: `PooledConnection::Drop` doesn't return connections to pool. Under high load, this could lead to connection exhaustion (DoS vector).
**Recommendation**: Refactor to use Arc<ConnectionPool> with return channel.
**Related**: `scenario_7_connection_pool` test is ignored pending fix.

### MEDIUM

#### M-001: auth_token Field Unused
**Location**: `conductor/core/src/surface.rs` (SurfaceCapabilities)
**Status**: Open
**Found**: Sprint 5
**Description**: The `auth_token` field exists but is never validated. False sense of security.
**Recommendation**: Either implement token validation or remove the field.

#### M-002: No Rate Limiting on Sprite Requests
**Location**: `conductor/core/src/avatar/`
**Status**: In Progress (Sprint 5 - P1.5)
**Found**: Sprint 5
**Description**: Malicious client could flood sprite requests, exhausting server resources.
**Recommendation**: Implement token bucket rate limiting per connection.

#### M-003: Unicode Character Validation Missing
**Location**: `conductor/core/src/avatar/`
**Status**: In Progress (Sprint 5 - P1.5)
**Found**: Sprint 5
**Description**: No validation of Unicode characters in sprite blocks. Could allow rendering exploits or terminal escape sequences.
**Recommendation**: Whitelist allowed Unicode ranges per avatar constraints doc.

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
