# TODO-next: Sprint Priorities

**Generated**: 2026-01-02
**Updated**: 2026-01-02 (Sprint 6 complete)
**Triage Team**: Architect, Hacker, UX Specialist, QA

---

## Active Epics (2026-Q1)

| Epic | File | Status | Target Sprint |
|------|------|--------|---------------|
| **Avatar Animation Evolution** | [TODO-epic-2026Q1-avatar-animation.md](TODO-epic-2026Q1-avatar-animation.md) | Execution | Sprint 7-9 |
| **Multi-Surface Architecture** | [TODO-epic-2026Q1-multi-surface.md](TODO-epic-2026Q1-multi-surface.md) | Execution | Sprint 7-8 |

---

## Sprint 6 Progress

### Just Completed (This Sprint)

| Item | Category | Notes |
|------|----------|-------|
| **Animation Evolution System (P3.1-P3.2)** | Backend | `evolution.rs` - EvolutionLevel enum (5 levels), EvolutionContext, dual thresholds (interactions + time), callbacks, 39 tests |
| **TUI Avatar Animation (P2.4-P2.5)** | TUI | `animator.rs` - AvatarAnimator with tick/update loop, MoodTransition; `dirty_tracker.rs` - DirtyRect, partial rendering optimization |
| **Connection Pool Reuse (H-002)** | Security | Arc<Self> pattern, mpsc channel for returns, RAII PooledConnection with Deref, idle timeout cleanup, >90% reuse ratio |
| **Random ConnectionId (H-001)** | Security | Replaced sequential AtomicU64 with UUID v4 (122 bits entropy), connection hijacking prevention |
| **State Snapshot (4.3)** | Architecture | StateSnapshot sent on surface connect, configurable message limit (default 20), avatar + session state |
| **Q4 Answered** | Planning | Evolution thresholds: Nascent(0), Developing(50/1h), Mature(200/5h), Evolved(500/20h), Transcendent(1000/50h) |
| **Q5 Answered** | Planning | Visual markers: glow intensity, particle density, color richness, animation complexity per level |

### Sprint 5 Recap

| Item | Category | Notes |
|------|----------|-------|
| **Sprite Protocol Messages (P1.2-P1.3)** | Architecture | `SpriteRequest`, `SpriteResponse`, `AnimationRequest`, `AnimationResponse`, `Mood`, `LoopBehavior` |
| **Surface Registration Protocol (4.3)** | Architecture | `StateSnapshot`, `SnapshotMessage`, auth_token, handshake_complete, protocol_version |
| **Avatar Security Limits (P1.5)** | Security | `security.rs` - dimensions, block count, Unicode validation, rate limiting, pending tracker |
| **Sprite Cache (P1.4)** | Backend | `cache.rs` - LRU eviction, 10MB budget, session scoping, non-evictable base sprites |
| **TUI Block Refactor (P2.1)** | TUI | Protocol Block <-> ColoredCell conversions, color mapping functions |
| **Workflow Documentation** | DevOps | sprint.md, epic.md, todo-driven-development.md, PRINCIPLES.md, deps.yaml, TODO-disabled-tests.md |

### Sprint 4 Recap

| Item | Category | Notes |
|------|----------|-------|
| **QuickResponse Routing Bug Fix** | Critical Bug | Removed hard latency filter - now uses scoring instead |
| **Breathing Colors for Messages** | TUI UX | User/Assistant prefixes pulse gently, streaming faster |

---

## Top Priorities for Next Sprint (Sprint 7)

> Sprint 7 focuses on **Animation Variants** (E-2026Q1-avatar-animation) and **Transport Hardening** (E-2026Q1-multi-surface)

### 1. Animation Variants (P3.3) [HIGH - Backend]
**Epic**: E-2026Q1-avatar-animation (Sprint 7)
**File**: `TODO-epic-2026Q1-avatar-animation.md`

**Why now**: Evolution system complete, need visual variety.

**Tasks**:
- P3.3: Create 2-3 animation variants per animation type
- Connect variants to evolution levels
- Add random variant selection

**Unblocks**: Avatar visual personality

---

### 2. Micro-Variation Timing (P3.4) [HIGH - Backend]
**Epic**: E-2026Q1-avatar-animation (Sprint 7)
**File**: `TODO-epic-2026Q1-avatar-animation.md`

**Why now**: Animation loop complete, need organic feel.

**Tasks**:
- P3.4: Add timing jitter to animations
- Randomize frame delays within bounds
- Prevent robotic feel

**Unblocks**: Natural animation flow

---

### 3. Reduced-Motion Accessibility (P2.6) [MEDIUM - TUI]
**Epic**: E-2026Q1-avatar-animation (Sprint 7)
**File**: `TODO-epic-2026Q1-avatar-animation.md`

**Why now**: Animation system complete, need accessibility.

**Tasks**:
- Detect prefers-reduced-motion
- Static avatar fallback mode
- Slower animation speed option

**Unblocks**: Accessibility compliance

---

### 4. Transport Rate Limiting (B5) [HIGH - Security]
**Epic**: E-2026Q1-multi-surface (Sprint 7)
**File**: `TODO-epic-2026Q1-multi-surface.md`

**Why now**: Connection security complete, need message-level protection.

**Tasks**:
- Per-connection message rate limiting
- Configurable limits
- Graceful degradation

**Security**: DoS prevention at message level

---

### 5. Heartbeat Enforcement (5.3) [MEDIUM - Architecture]
**Epic**: E-2026Q1-multi-surface (Sprint 7)
**File**: `TODO-epic-2026Q1-multi-surface.md`

**Why now**: State snapshot complete, need connection health.

**Tasks**:
- Implement heartbeat protocol
- Configurable timeout (default 30s)
- Automatic disconnection on timeout

**Unblocks**: Connection health monitoring

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
| Avatar Protocol | E-2026Q1-avatar-animation | HIGH | **Phase 1 Complete** | -- |
| TUI Avatar Migration | E-2026Q1-avatar-animation | HIGH | **P2.4-P2.5 Complete** | P2.6 (Accessibility) |
| Animation Evolution | E-2026Q1-avatar-animation | HIGH | **P3.1-P3.2 Complete** | P3.3, P3.4 (Variants) |
| Dynamic Generation | E-2026Q1-avatar-animation | MEDIUM | Sprint 8 | P4.1, P4.2 |
| Surface Protocol | E-2026Q1-multi-surface | HIGH | **4.3 Complete** | 5.3 (Heartbeat) |
| Security Hardening | E-2026Q1-multi-surface | HIGH | **H-001, H-002 Complete** | B5 (Rate limiting) |

---

## Blocked Items

| Item | Blocked By | When Unblocked |
|------|------------|----------------|
| WebSocket transport | Security review + TLS setup | Phase 5+ |
| macOS getpeereid() | Developer with Mac | External dependency |
| Production deployment | Full security audit | Before v1.0 |

## Recently Unblocked (by Sprint 6)

| Item | Was Blocked By | Now Available |
|------|----------------|---------------|
| Animation Variants (P3.3-P3.4) | Evolution System (P3.1-P3.2) | YES |
| Dynamic Sprite Generation (P4.x) | Evolution System + Security | YES |
| Connection Health Monitoring | State Snapshot (4.3) | YES |
| Transport Rate Limiting | Connection Pool Reuse (H-002) | YES |

---

## Open Questions (Need Answers)

| Question | Impact | Owner | Status |
|----------|--------|-------|--------|
| ~~Q1: Block color type~~ | API stability | Architect | ✓ Native Color struct |
| ~~Q2: Sprite data encoding~~ | Performance | Architect | ✓ JSON via serde |
| ~~Q3: Uncached sprite request~~ | Protocol flow | Architect | ✓ SpriteRequest message |
| ~~Q4: Evolution level triggers?~~ | UX feel | UX + Backend | ✓ Dual thresholds (interactions + time) |
| ~~Q5: Evolution visual markers?~~ | Art requirements | UX + Art | ✓ Glow, particles, color, complexity |
| Q6: LLM involvement in sprite gen? | Architecture | Architect | Before P4.1 |

---

## Disabled Tests

| Test | Priority | Owner | Sprint Target | Status |
|------|----------|-------|---------------|--------|
| scenario_10_stress_test | LOW | QA | N/A | Intentional (10 min) |

---

## Technical Debt

| Debt | Risk | Notes |
|------|------|-------|
| Mock-only integration tests | Medium | No real HTTP/JSON parsing |
| No TUI rendering tests | Medium | Tests verify conductor, not terminal |
| auth_token field unused (M-001) | Low | Implement or remove |

---

## Recommended Sprint 7 Split

| Team Member | Focus Area |
|-------------|------------|
| Architect | Heartbeat protocol design, Q6 decision |
| Backend | Animation variants P3.3-P3.4, rate limiting |
| Hacker | Rate limiting fuzzing, connection stability |
| TUI Developer | Reduced-motion accessibility P2.6 |
| UX Specialist | Animation variant design, accessibility UX |

---

## Test Summary

- **conductor-core**: 417+ tests passing (39 new evolution tests)
- **yollayah-tui**: 26 tests passing + integration tests
- **Total**: 545+ tests passing
- **Ignored**: 1 (stress test - intentional)

---

**Next Review**: End of Sprint 7

---

## Sprint 8+ Preview

### Sprint 8 (E-2026Q1-avatar-animation)
- P4.1-P4.3: Sprite generation pipeline
- Answer Q6: LLM involvement decision

### Sprint 8 (E-2026Q1-multi-surface)
- 5.1: TOML configuration file
- M-001: auth_token resolution
- Chaos tests

### Sprint 9 (E-2026Q1-avatar-animation)
- P4.4: Sprite generation API
- P5.1: WebSocket transport preparation
