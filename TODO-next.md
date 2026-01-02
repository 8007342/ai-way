# TODO-next: Sprint Priorities

**Generated**: 2026-01-02
**Updated**: 2026-01-02 (Epic planning session complete)
**Triage Team**: Architect, Hacker, UX Specialist, QA

---

## Active Epics (2026-Q1)

| Epic | File | Status | Target Sprint |
|------|------|--------|---------------|
| **Avatar Animation Evolution** | [TODO-epic-2026Q1-avatar-animation.md](TODO-epic-2026Q1-avatar-animation.md) | Execution | Sprint 6-9 |
| **Multi-Surface Architecture** | [TODO-epic-2026Q1-multi-surface.md](TODO-epic-2026Q1-multi-surface.md) | Execution | Sprint 6-8 |

---

## Sprint 5 Progress

### Just Completed (This Sprint)

| Item | Category | Notes |
|------|----------|-------|
| **Sprite Protocol Messages (P1.2-P1.3)** | Architecture | `SpriteRequest`, `SpriteResponse`, `AnimationRequest`, `AnimationResponse`, `Mood`, `LoopBehavior` |
| **Surface Registration Protocol (4.3)** | Architecture | `StateSnapshot`, `SnapshotMessage`, auth_token, handshake_complete, protocol_version |
| **Avatar Security Limits (P1.5)** | Security | `security.rs` - dimensions, block count, Unicode validation, rate limiting, pending tracker |
| **Sprite Cache (P1.4)** | Backend | `cache.rs` - LRU eviction, 10MB budget, session scoping, non-evictable base sprites |
| **TUI Block Refactor (P2.1)** | TUI | Protocol Block <-> ColoredCell conversions, color mapping functions |
| **Sprint Workflow Documentation** | DevOps | `workflows/sprint.md` - Sprint phases, periodic reviews, TODO hierarchy |
| **Security Findings Tracking** | Security | `TODO-security-findings.md` - Severity levels, active findings, audit schedule |
| **Epic Workflow** | DevOps | `workflows/epic.md` - Milestone-based major features, planning team roles, lifecycle |
| **TODO-Driven Development** | DevOps | `workflows/todo-driven-development.md` - Methodology, file hierarchy, agent guidelines |
| **PRINCIPLES.md** | Governance | Ethical, technical, design, legal principles at repo root |
| **Dependency Tracking** | DevOps | `deps.yaml` - Component freshness, external deps, security audit status |
| **Disabled Tests Tracking** | QA | `TODO-disabled-tests.md` - Centralized `#[ignore]` test tracking with annotation convention |

### Sprint 4 Recap

| Item | Category | Notes |
|------|----------|-------|
| **QuickResponse Routing Bug Fix** | Critical Bug | Removed hard latency filter - now uses scoring instead |
| **Breathing Colors for Messages** | TUI UX | User/Assistant prefixes pulse gently, streaming faster |

### Sprint 3 Recap

| Item | Category | Notes |
|------|----------|-------|
| **Multi-Surface Conductor Refactor** | Architecture | `SurfaceRegistry`, `SurfaceHandle`, `ConnectionId` - Full multi-surface support |
| **Avatar Block Struct (P1.1)** | Avatar System | `Color`, `Block`, `SizeHint`, `AnchorPoint` in `conductor-core` |
| **yollayah.sh Launcher Updates** | DevOps | Commands: start, daemon, connect, stop, restart, status |
| **Q1-Q2 Answered** | Planning | Block uses native Color struct, JSON encoding via serde |

---

## Top Priorities for Next Sprint (Sprint 6)

> Sprint 6 focuses on **Avatar Animation** (E-2026Q1-avatar-animation) and **Security Hardening** (E-2026Q1-multi-surface)

### 1. Animation Evolution System (P3.1-P3.2) [HIGH - Backend]
**Epic**: E-2026Q1-avatar-animation (Sprint 6)
**File**: `TODO-epic-2026Q1-avatar-animation.md`

**Why now**: All Phase 1 (protocol) and Phase 2 (TUI) work is complete.

**Tasks**:
- P3.1: Implement `EvolutionContext` struct and storage
- P3.2: Add evolution triggers (interaction count, session time)
- Answer Q4: Define evolution level thresholds
- Answer Q5: Define visual markers

**Unblocks**: Progressive avatar personality

---

### 2. TUI Avatar Animation (P2.4-P2.5) [HIGH - TUI]
**Epic**: E-2026Q1-avatar-animation (Sprint 6)
**File**: `TODO-epic-2026Q1-avatar-animation.md`

**Why now**: P2.1 (Block refactor) is complete.

**Tasks**:
- P2.4: Implement animation tick/update loop
- P2.5: Partial rendering (dirty-rect tracking)
- Frame interpolation and mood transitions

**Unblocks**: Living, breathing avatar in TUI

---

### 3. Connection Pool Reuse (H-002) [HIGH - Security]
**Epic**: E-2026Q1-multi-surface (Sprint 6)
**File**: `TODO-epic-2026Q1-multi-surface.md`

**Why now**: HIGH security finding from Sprint 5 audit.

**Tasks**:
- Refactor `ConnectionPool` to use `Arc<Self>`
- Add async channel for connection returns
- Re-enable scenario_7_connection_pool test

**Security**: DoS prevention under high load

---

### 4. Random ConnectionId (H-001) [HIGH - Security]
**Epic**: E-2026Q1-multi-surface (Sprint 6)
**File**: `TODO-epic-2026Q1-multi-surface.md`

**Why now**: HIGH security finding - predictable IDs.

**Tasks**:
- Replace sequential AtomicU64 with UUID
- Use uuid crate (v4, serde)
- Update tests

**Security**: Connection hijacking prevention

---

### 5. State Snapshot for Late-Joining Surfaces (4.3) [MEDIUM - Architecture]
**Epic**: E-2026Q1-multi-surface (Sprint 6)
**File**: `TODO-epic-2026Q1-multi-surface.md`

**Why now**: Multi-surface architecture needs state sync.

**Tasks**:
- Implement StateSnapshot message
- Send on surface connect
- Limit snapshot size (last N messages)

**Unblocks**: Multiple surfaces showing same session

---

## Quick Wins (Can Fit Around Major Work)

| Item | File | Effort | Impact |
|------|------|--------|--------|
| Scroll gradient indicators | TODO-main.md | 1-2h | High - discoverability |
| Empty response test | TODO-integration-testing.md | 30m | Medium - edge case coverage |
| TOML config file (5.1) | TODO-conductor-ux-split.md | 2-3h | Medium - configuration flexibility |
| Fix unused import warnings | conductor-core | 15m | Low - code hygiene |

---

## Work Streams Status

| Stream | Epic | Priority | Status | Next Tasks |
|--------|------|----------|--------|------------|
| Avatar Protocol | E-2026Q1-avatar-animation | HIGH | **Phase 1 Complete** | Phase 3 (Evolution) |
| TUI Avatar Migration | E-2026Q1-avatar-animation | HIGH | **P2.1 Complete** | P2.4, P2.5 (Animation loop) |
| Animation Evolution | E-2026Q1-avatar-animation | HIGH | Ready to start | P3.1, P3.2 |
| Dynamic Generation | E-2026Q1-avatar-animation | MEDIUM | Sprint 8 | P4.1, P4.2 |
| Surface Protocol | E-2026Q1-multi-surface | HIGH | **4.3 Partial** | State snapshot, Auth tokens |
| Security Hardening | E-2026Q1-multi-surface | HIGH | In Progress | H-001, H-002 |

---

## Blocked Items

| Item | Blocked By | When Unblocked |
|------|------------|----------------|
| WebSocket transport | Security review + TLS setup | Phase 5+ |
| macOS getpeereid() | Developer with Mac | External dependency |
| Production deployment | Full security audit | Before v1.0 |

## Recently Unblocked (by Sprint 5)

| Item | Was Blocked By | Now Available |
|------|----------------|---------------|
| Animation Evolution (P3.x) | Animation Protocol (P1.3) | YES |
| Dynamic Sprite Generation (P4.x) | Security Limits (P1.5) | YES |
| TUI Animation Loop (P2.4) | Block Refactor (P2.1) | YES |
| State Sync on Reconnect | Surface Registration (4.3) | YES |

---

## Open Questions (Need Answers)

| Question | Impact | Owner | Status |
|----------|--------|-------|--------|
| ~~Q1: Block color type~~ | API stability | Architect | ✓ Native Color struct |
| ~~Q2: Sprite data encoding~~ | Performance | Architect | ✓ JSON via serde |
| ~~Q3: Uncached sprite request~~ | Protocol flow | Architect | ✓ SpriteRequest message |
| Q4: Evolution level triggers? | UX feel | UX + Backend | Before P3.1 |
| Q5: Evolution visual markers? | Art requirements | UX + Art | Before P3.1 |

---

## Disabled Tests

| Test | Priority | Owner | Sprint Target | Status |
|------|----------|-------|---------------|--------|
| scenario_7_connection_pool | HIGH | Backend | Sprint 6 | Pool refactor needed |
| scenario_10_stress_test | LOW | QA | N/A | Intentional (10 min) |

---

## Technical Debt

| Debt | Risk | Notes |
|------|------|-------|
| Mock-only integration tests | Medium | No real HTTP/JSON parsing |
| No TUI rendering tests | Medium | Tests verify conductor, not terminal |
| Sequential ConnectionId (H-001) | Medium | Use UUIDs before production |
| auth_token field unused (M-001) | Low | Implement or remove |
| Connection pool doesn't reuse (H-002) | Medium | Sprint 6 priority |

---

## Recommended Sprint 6 Split

| Team Member | Focus Area |
|-------------|------------|
| Architect | Evolution system design, answer Q4-Q5 |
| Backend | Connection pool refactor + Sprite generation P4.1-P4.2 |
| Hacker | Security review of sprite generation, fuzzing |
| TUI Developer | Animation loop P2.4 + Partial rendering P2.5 |
| UX Specialist | Evolution level design, visual marker concepts |

---

## Test Summary

- **conductor-core**: 378 tests passing
- **yollayah-tui**: 26 tests passing + integration tests
- **Total**: 506+ tests passing
- **Ignored**: 11 (stress tests + setup-dependent doc tests)

---

**Next Review**: End of Sprint 6

---

## Sprint 7+ Preview

### Sprint 7 (E-2026Q1-avatar-animation)
- P3.3: Animation variants (2-3 per type)
- P3.4: Micro-variation timing (jitter)
- P2.5: Reduced-motion accessibility mode

### Sprint 7 (E-2026Q1-multi-surface)
- B4: macOS getpeereid() implementation
- B5: Transport rate limiting
- 5.3: Heartbeat enforcement

### Sprint 8 (E-2026Q1-avatar-animation)
- P4.1-P4.3: Sprite generation pipeline
- Answer Q6: LLM involvement decision

### Sprint 8 (E-2026Q1-multi-surface)
- 5.1: TOML configuration file
- M-001: auth_token resolution
- Chaos tests
