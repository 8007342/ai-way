# TODO-next: Sprint Priorities

**Generated**: 2026-01-02
**Updated**: 2026-01-02 (Sprint 4 mid-sprint update)
**Triage Team**: Architect, Hacker, UX Specialist, QA

---

## Sprint 4 Progress

### Just Completed (This Sprint)

| Item | Category | Notes |
|------|----------|-------|
| **⚠️ QuickResponse Routing Bug Fix** | Critical Bug | Removed hard latency filter - now uses scoring instead |
| **Breathing Colors for Messages** | TUI UX | User/Assistant prefixes pulse gently, streaming faster |

### Sprint 3 Recap

| Item | Category | Notes |
|------|----------|-------|
| **Multi-Surface Conductor Refactor** | Architecture | `SurfaceRegistry`, `SurfaceHandle`, `ConnectionId` - Full multi-surface support |
| **Avatar Block Struct (P1.1)** | Avatar System | `Color`, `Block`, `SizeHint`, `AnchorPoint` in `conductor-core` |
| **yollayah.sh Launcher Updates** | DevOps | Commands: start, daemon, connect, stop, restart, status |
| **Q1-Q2 Answered** | Planning | Block uses native Color struct, JSON encoding via serde |

### Previous Sprint (Sprint 2)

| Item | Category | Notes |
|------|----------|-------|
| **Conductor Daemon Binary** | Architecture | Full crate with CLI, signals, daemonization |
| **Frame Integrity (CRC32)** | Security | Replaced XXH3 with crc32fast, all tests pass |
| **Channel Backpressure Test** | QA | Last Priority 1 integration test complete |
| **Yollayah Avatar Constraints Doc** | UX | `docs/yollayah-avatar-constraints.md` - Block-based rendering spec |
| **Avatar Animation System TODO** | Planning | `TODO-avatar-animation-system.md` - Full implementation plan |

### Previous Sprint (Sprint 1)

| Item | Category | Notes |
|------|----------|-------|
| Input history (Up/Down arrows) | TUI UX | High-impact quick win |
| Warmup flow test | Integration Testing | Critical path coverage |
| Error recovery test | Integration Testing | Backend failure handling |
| Timeout handling test | Integration Testing | Streaming timeout verification |
| Router fallback wiring | Multi-Model | FallbackChainManager into QueryRouter |
| Health tracking | Multi-Model | HealthTracker with time_since_last_success fix |

---

## Top Priorities for Next Sprint (Sprint 4)

### 1. Sprite Protocol Messages (Phase P1.2-P1.3) [CRITICAL - Architect]
**File**: `TODO-avatar-animation-system.md` Phase 1

**Why now**: P1.1 (Block struct) is complete - now add protocol messages.

**Tasks**:
- P1.2: Add `SpriteRequest` and `SpriteResponse` to `ConductorMessage` enum
- P1.3: Add `AnimationRequest` and `AnimationResponse` to protocol

**Unblocks**: Dynamic sprite transmission, animation orchestration

---

### 2. Surface Registration Protocol (Phase 4.3) [HIGH - Architect]
**File**: `TODO-conductor-ux-split.md` Phase 4.3

**Why now**: Multi-surface refactor complete - now formalize registration.

**Tasks**:
- Extend handshake with capability declaration
- Assign and validate ConnectionId
- Send current state snapshot on connect
- Implement surface authentication tokens

**Unblocks**: Proper surface lifecycle, state synchronization

---

### 3. Avatar Security Limits (Phase P1.5) [HIGH - Hacker]
**File**: `TODO-avatar-animation-system.md` Phase 1

**Why now**: Security must be built-in before sprite transmission.

**Tasks**:
- Implement MAX_SPRITE_WIDTH/HEIGHT (100x100)
- Enforce MAX_BLOCKS_PER_SPRITE (10,000)
- Validate Unicode characters against allowed ranges
- Add rate limiting for sprite requests

**Unblocks**: Safe sprite generation pipeline

---

### 4. Sprite Cache Implementation (Phase P1.4) [HIGH - Backend]
**File**: `TODO-avatar-animation-system.md` Phase 1

**Why now**: Caching is essential for performance before sprite transmission.

**Tasks**:
- Implement `SpriteCache` in Conductor with LRU eviction
- Respect memory budget (10MB default)
- Session-scoped cache keys
- Base sprites marked as non-evictable

**Unblocks**: Efficient sprite serving, memory-safe operation

---

### 5. TUI Block Refactor (Phase P2.1) [MEDIUM - TUI]
**File**: `TODO-avatar-animation-system.md` Phase 2

**Why now**: Protocol Block struct ready - migrate TUI to use it.

**Tasks**:
- Refactor `ColoredCell` to use protocol `Block` type
- Update sprite rendering to use new types
- Ensure no visual regression

**Unblocks**: Consistent rendering, protocol compliance

---

## Quick Wins (Can Fit Around Major Work)

| Item | File | Effort | Impact |
|------|------|--------|--------|
| ~~**⚠️ Fix QuickResponse routing bug**~~ | ~~routing/policy.rs~~ | ~~1h~~ | ✅ DONE |
| Scroll gradient indicators | TODO-main.md | 1-2h | High - discoverability |
| ~~Breathing color effect~~ | ~~TODO-main.md~~ | ~~1-2h~~ | ✅ DONE |
| Empty response test | TODO-integration-testing.md | 30m | Medium - edge case coverage |
| TOML config file (5.1) | TODO-conductor-ux-split.md | 2-3h | Medium - configuration flexibility |

---

## New Work Streams (from Avatar System)

| Stream | Priority | First Tasks | Dependencies |
|--------|----------|-------------|--------------|
| Avatar Protocol | HIGH | P1.1, P1.2, P1.5 | None |
| TUI Avatar Migration | MEDIUM | P2.1, P2.4 | P1.1 complete |
| Animation Evolution | LOW | P3.1, P3.2 | P1.3 complete |
| Dynamic Generation | LOW | P4.1, P4.2 | Phase 1-2 complete |

---

## Blocked Items

| Item | Blocked By | When Unblocked |
|------|------------|----------------|
| Sprite transmission (P1.2+) | Sprite protocol messages | After P1.2-P1.3 done |
| Dynamic sprite generation (P4.x) | Security limits (P1.5) | After P1.5 done |
| WebSocket transport | Security review + multi-surface | Phase 5+ |
| macOS getpeereid() | Developer with Mac | External dependency |

## Recently Unblocked (by Sprint 3)

| Item | Was Blocked By | Now Available |
|------|----------------|---------------|
| Surface Registration Protocol (4.3) | Multi-surface refactor (4.2) | YES |
| TUI Avatar Migration (P2.x) | Block struct definition (P1.1) | YES |
| Sprite Protocol Messages (P1.2-P1.3) | Block struct (P1.1) | YES |
| Sprite Cache (P1.4) | Block struct (P1.1) | YES |
| Animation Protocol (P1.3) | Block struct (P1.1) | YES |

---

## Open Questions (Need Answers)

From `TODO-avatar-animation-system.md`:

| Question | Impact | Owner | Status |
|----------|--------|-------|--------|
| ~~Q1: Block color type - protocol-specific or `rgb` crate?~~ | API stability | Architect | ✓ ANSWERED: Native Color struct |
| ~~Q2: Sprite data encoding - JSON or binary?~~ | Performance | Architect | ✓ ANSWERED: JSON via serde |
| Q3: How does surface request uncached sprite? | Protocol flow | Architect | Pending P1.2 |
| Q4: Evolution level triggers? | UX feel | UX + Backend | Before P3.1 |
| Q5: Evolution levels and visual markers? | Art requirements | UX + Art | Before P3.1 |

---

## Disabled Tests (Triage: Architect + QA + Hacker)

Tests currently marked `#[ignore]` that need attention:

### 1. scenario_7_connection_pool [HIGH - Architect/Backend]
**File**: `conductor/core/tests/routing_performance_tests.rs:460`
**Reason**: Connection pool reuse not implemented

**Root Cause Analysis**:
- `PooledConnection::Drop` doesn't return connections to pool due to async/lifetime constraints
- Currently just releases semaphore permit but drops the actual connection
- Comment in code acknowledges this: "A better design would use Arc<ConnectionPool> and a return channel"

**Fix Required**:
- Refactor `ConnectionPool` to use `Arc<Self>`
- Add async channel for connection returns
- Background task to receive and reinsert connections
- Estimated scope: Medium (refactor pool ownership model)

**Security Note** (Hacker): Connection exhaustion risk if pool doesn't reuse - potential DoS vector under load.

---

### 2. ~~scenario_9_session_affinity~~ ✅ FIXED
**File**: `conductor/core/tests/routing_performance_tests.rs:587`
**Status**: Test re-enabled and passing

**Fix Applied** (Option C):
- Removed hard latency filter from `get_candidates()` in `routing/policy.rs`
- Latency is now used for scoring/prioritization only, not hard filtering
- Models with better latency are still preferred, but no more `NoModelsAvailable` errors

---

### 3. scenario_10_stress_test [LOW - QA]
**File**: `conductor/core/tests/routing_performance_tests.rs:653`
**Reason**: Long-running test (10 min), run manually

**Status**: Working correctly, intentionally ignored for CI.

**Recommendation**:
- Keep ignored in normal CI
- Add to separate "stress test" CI job (nightly/weekly)
- Document how to run: `cargo test scenario_10 -- --ignored`

---

## Disabled Test Summary

| Test | Priority | Owner | Sprint Target | Blocker? |
|------|----------|-------|---------------|----------|
| ~~scenario_9_session_affinity~~ | ~~HIGH~~ | ~~Architect~~ | ~~Sprint 4~~ | ✅ **FIXED** |
| scenario_7_connection_pool | HIGH | Backend | Sprint 5 | No (perf) |
| scenario_10_stress_test | LOW | QA | N/A | Intentional |

---

## Technical Debt to Watch

| Debt | Risk | Notes |
|------|------|-------|
| Mock-only integration tests | Medium | No real HTTP/JSON parsing verification |
| No TUI rendering tests | Medium | Tests verify conductor, not terminal output |
| Sequential ConnectionId | Medium | Should be cryptographically random before production |
| auth_token field unused | Low | Implement or remove in security cleanup |
| TUI ColoredCell vs protocol Block | Low | Will diverge further until P2.1 migration |
| Connection pool doesn't reuse | Medium | See scenario_7 above - performance issue |

---

## Recommended Sprint 4 Split

| Team Member | Focus Area |
|-------------|------------|
| Architect | Sprite Protocol Messages (P1.2-P1.3) + Surface Registration (4.3) |
| Hacker | Avatar Security Limits (P1.5) + Rate limiting |
| Backend | Sprite Cache Implementation (P1.4) |
| TUI Developer | TUI Block Refactor (P2.1) |
| UX Specialist | Answer evolution questions (Q4-Q5) + Partial rendering design |

---

**Next Review**: After sprite protocol messages work and TUI can request/render sprites
