# TODO-next: Sprint Priorities

**Generated**: 2026-01-02
**Updated**: 2026-01-02 (Sprint 5 complete)
**Triage Team**: Architect, Hacker, UX Specialist, QA

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

### 1. Animation Evolution System (Phase P3.1-P3.2) [HIGH - Backend]
**File**: `TODO-avatar-animation-system.md` Phase 3

**Why now**: All Phase 1 (protocol) and Phase 2 (TUI) work is complete.

**Tasks**:
- P3.1: Implement `EvolutionState` tracking in Conductor
- P3.2: Add evolution triggers (interaction count, session time)
- Define evolution level thresholds and visual markers

**Unblocks**: Progressive avatar personality

---

### 2. Sprite Generation Pipeline (Phase P4.1-P4.2) [HIGH - Backend]
**File**: `TODO-avatar-animation-system.md` Phase 4

**Why now**: Security limits (P1.5) and cache (P1.4) are complete.

**Tasks**:
- P4.1: Implement `SpriteGenerator` trait
- P4.2: Create procedural mood-based sprite variations
- Add sprite blending/transitions

**Unblocks**: Dynamic avatar expressions

---

### 3. Connection Pool Reuse (H-002) [HIGH - Backend]
**File**: `TODO-security-findings.md` H-002

**Why now**: Security finding from Sprint 5 audit.

**Tasks**:
- Refactor `ConnectionPool` to use `Arc<Self>`
- Add async channel for connection returns
- Re-enable scenario_7_connection_pool test

**Security**: DoS prevention under high load

---

### 4. TUI Avatar Animation (Phase P2.4) [MEDIUM - TUI]
**File**: `TODO-avatar-animation-system.md` Phase 2

**Why now**: P2.1 (Block refactor) is complete.

**Tasks**:
- P2.4: Implement animation tick/update loop
- Add frame interpolation
- Smooth transitions between moods

**Unblocks**: Living, breathing avatar in TUI

---

### 5. Partial Rendering Support (Phase P2.5) [MEDIUM - TUI]
**File**: `TODO-avatar-animation-system.md` Phase 2

**Why now**: Block rendering is aligned with protocol.

**Tasks**:
- Implement dirty-rect tracking
- Only re-render changed cells
- Reduce TUI CPU usage during idle

**Performance**: Better battery life on laptops

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

| Stream | Priority | Status | Next Tasks |
|--------|----------|--------|------------|
| Avatar Protocol | HIGH | **Phase 1 Complete** | Phase 3 (Evolution) |
| TUI Avatar Migration | MEDIUM | **P2.1 Complete** | P2.4 (Animation loop) |
| Animation Evolution | MEDIUM | Ready to start | P3.1, P3.2 |
| Dynamic Generation | MEDIUM | Ready to start | P4.1, P4.2 |
| Surface Protocol | HIGH | **4.3 Complete** | 4.4 (WebSocket prep) |

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

**Next Review**: After animation evolution and sprite generation work
