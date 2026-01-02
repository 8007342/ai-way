# Epic: Avatar Animation Evolution

> Transform Yollayah from static sprites to a living, evolving avatar with dynamic expressions, sprite generation, and cross-session memory.

## Status

- **Phase**: Execution
- **Started**: 2026-01-02
- **Target Completion**: 2026-Q1 (Sprint 9)
- **Sprints Completed**: 6 (Evolution system + TUI animation complete)

## Overview

The Avatar Animation Evolution epic transforms Yollayah's visual presence from static, hardcoded sprites into a dynamic, evolving character system. This work builds on the completed protocol foundation (P1.1-P1.5) and TUI block refactor (P2.1) to enable:

1. **Animation Evolution** - Avatar personality develops over time based on interaction patterns
2. **Dynamic Sprite Generation** - Procedural generation of mood-based variations
3. **TUI Animation Loop** - Smooth, living animations in the terminal interface
4. **Partial Rendering** - Efficient dirty-rect updates for battery/CPU optimization

This epic represents a significant step toward Yollayah's goal of feeling "alive" - not just responding to queries, but having a visual personality that grows with the user.

## Planning Team Sign-off

| Role | Name/ID | Date | Notes |
|------|---------|------|-------|
| Architect | - | 2026-01-02 | Protocol design complete (P1.1) |
| UX Specialist | - | 2026-01-02 | Animation timing guidelines defined |
| Lawyer | - | 2026-01-02 | No licensing concerns with procedural generation |
| QA | - | 2026-01-02 | Test strategy in TODO-integration-testing.md |
| Security Specialist | - | 2026-01-02 | P1.5 security limits implemented |
| Backend Developer | - | 2026-01-02 | Cache implementation complete (P1.4) |
| TUI Developer | - | 2026-01-02 | Block refactor complete (P2.1) |
| UX Designer | - | 2026-01-02 | Evolution visual markers pending (Q5) |

## Security Considerations

- [x] Threat model reviewed (see TODO-avatar-animation-system.md Hacker Analysis)
- [x] Input validation defined (MAX_SPRITE_*, Unicode whitelist)
- [x] Resource limits implemented (P1.5)
- [ ] Rate limiting per session (M-002 in progress)
- [ ] Content policy for generated sprites (P4.4)

### Security Invariants (Implemented in P1.5)

```rust
const MAX_SPRITE_WIDTH: u16 = 100;
const MAX_SPRITE_HEIGHT: u16 = 100;
const MAX_BLOCKS_PER_SPRITE: usize = 10_000;
const MAX_PENDING_REQUESTS_PER_SESSION: usize = 10;
const MAX_CACHE_SIZE_BYTES: usize = 10 * 1024 * 1024; // 10MB
const MAX_ANIMATION_FRAMES: usize = 100;
const MAX_ANIMATION_DURATION_MS: u64 = 60_000;
```

### Identified Threats

1. **Sprite Bomb** - Mitigated by MAX_BLOCKS_PER_SPRITE and cache size limits
2. **Unicode Exploits** - Mitigated by whitelist validation (Block Elements, Box Drawing, etc.)
3. **Cache Poisoning** - Mitigated by Conductor-controlled cache keys
4. **Resource Exhaustion** - Mitigated by pending request limits and rate limiting

## Test Strategy

### Unit Tests

- [x] Block struct serialization (P1.1)
- [x] Color operations (blend, lerp, hex) (P1.1)
- [x] EvolutionContext state transitions (P3.1)
- [ ] Sprite generation rules (P4.2)
- [ ] Animation variant selection (P3.3)

### Integration Tests

- [ ] TUI renders dynamic sprites without regression (P2.2)
- [x] Partial rendering on various terminal sizes (P2.4)
- [x] Evolution tracking persists across reconnects (P3.2)
- [x] Sprite cache eviction under memory pressure (P1.4)

### Test Files Created

| File | Status | Notes |
|------|--------|-------|
| conductor/core/src/avatar/block.rs | Passing | Comprehensive unit tests for Block, Color, SizeHint, AnchorPoint |
| conductor/core/src/avatar/evolution.rs | Passing | 39 tests for EvolutionContext, levels, triggers, callbacks |
| conductor/core/src/cache.rs | Passing | LRU eviction, session scoping tests |
| conductor/core/src/security.rs | Passing | Dimension limits, Unicode validation tests |
| tui/src/avatar/animator.rs | Passing | AvatarAnimator, MoodTransition, frame timing tests |
| tui/src/avatar/dirty_tracker.rs | Passing | DirtyRect, dirty cell tracking, rect merging tests |

## Sprint Plan

### Sprint 6: Evolution System Foundation [COMPLETE]

**Theme**: Animation evolution infrastructure and TUI animation loop

- [x] **P3.1**: Define `EvolutionContext` struct and storage
  - EvolutionLevel enum (5 levels: Nascent, Developing, Mature, Evolved, Transcendent)
  - EvolutionContext with dual thresholds (interactions + session time)
  - EvolutionEvent for tracking level changes
  - EvolutionProgress for detailed progress tracking
  - EvolutionCallbackManager for event subscription
- [x] **P3.2**: Implement evolution tracking in Conductor
  - Dual triggers: interaction count AND session duration required
  - Level thresholds: 0, 50/1h, 200/5h, 500/20h, 1000/50h
  - 39 comprehensive tests
- [x] **P2.4**: Implement animation tick/update loop in TUI
  - AvatarAnimator with configurable speed multiplier
  - Smooth frame timing with Instant-based tracking
  - MoodTransition struct for animated mood changes
- [x] **P2.5**: Partial rendering support
  - DirtyTracker with cell-level granularity
  - DirtyRect merging for efficient updates
  - Full-dirty mode for complete redraws

**Dependencies**: P1.1 (Block struct), P1.3 (AnimationRequest), P1.4 (Cache)

**Exit Criteria**: ✓ All met
- ✓ Evolution level increments based on defined triggers
- ✓ TUI animation loop runs at target frame rate
- ✓ CPU usage reduced during idle via partial rendering

### Sprint 7: Animation Variants and Visual Markers

**Theme**: Make animations feel alive and fresh

- [ ] **P3.3**: Add animation variants (2-3 per animation type)
  - Idle variants with subtle differences
  - Thinking variants (different pose angles)
  - Random selection with weighted probabilities
- [ ] **P3.4**: Implement micro-variation timing (jitter)
  - 5-15% random variance in frame durations
  - Breathing effect timing variations
- [ ] **Q5 Resolution**: Define evolution visual markers
  - Color shifts per evolution level
  - Accessory appearances (glasses, hats)
  - Size/complexity changes
- [ ] **P2.5 continued**: Reduced-motion accessibility mode
  - Static sprites when REDUCE_MOTION=1
  - State communicated via accessibility announcements

**Dependencies**: Sprint 6 (P3.1, P3.2, P2.4)

**Exit Criteria**:
- Users cannot predict which animation variant plays
- Evolution visual changes are noticeable but not distracting
- Accessibility mode provides full functionality without animation

### Sprint 8: Sprite Generation Pipeline

**Theme**: Dynamic procedural sprite creation

- [ ] **P4.1**: Design sprite generation pipeline
  - SpriteGenerator trait definition
  - Rule-based generation (no LLM initially)
  - Mood-to-sprite mapping rules
- [ ] **P4.2**: Implement procedural mood variations
  - Base sprite + mood overlay composition
  - Color tinting based on mood
  - Expression modifications (eyes, mouth patterns)
- [ ] **P4.3**: Accessory generation rules
  - Party hat, glasses, coffee mug sprites
  - Composition with base sprites
  - Evolution-gated accessory unlocks
- [ ] Answer Q6: LLM involvement decision
  - Evaluate: rule-based vs LLM-described generation
  - Security implications of each approach

**Dependencies**: P1.5 (Security limits), Sprint 7 (Evolution markers)

**Exit Criteria**:
- Generated sprites match style guide
- Generation latency < 100ms for simple requests
- Accessory composition works correctly

### Sprint 9: Stabilization and Polish

**Theme**: Production readiness

- [ ] **P4.4**: Content policy checks for generated sprites
  - Pattern validation
  - Audit logging
  - Admin review workflow (optional)
- [ ] Enable all disabled tests related to avatar system
- [ ] Performance optimization
  - Profile animation loop CPU usage
  - Optimize sprite cache hit rates
  - Reduce memory footprint
- [ ] Documentation completion
  - Update yollayah-avatar-constraints.md
  - Add sprite generation API docs
  - Update TUI developer guide

**Dependencies**: Sprint 8 (Generation pipeline)

**Exit Criteria**:
- All avatar-related tests passing
- No known animation-related bugs
- Performance meets targets (< 5% CPU idle)

## Progress Log

### Sprint 6 (2026-01-02) - Evolution & Animation Complete

**Completed**:
- P3.1: EvolutionContext with 5-level enum, dual threshold system
- P3.2: Evolution tracking with 39 tests, callback manager
- P2.4: AvatarAnimator with tick/update loop, mood transitions
- P2.5: DirtyTracker with rect merging, partial rendering

**Questions Answered**:
- Q4: Evolution thresholds - Nascent(0), Developing(50/1h), Mature(200/5h), Evolved(500/20h), Transcendent(1000/50h)
- Q5: Visual markers - glow intensity, particle density, color richness, animation complexity per level

**New Files**:
- `conductor/core/src/avatar/evolution.rs` (800+ lines, 39 tests)
- `tui/src/avatar/animator.rs`
- `tui/src/avatar/dirty_tracker.rs`

### Sprint 5 (2026-01-02) - Foundation Complete

**Completed**:
- P1.1: Block struct with Color, SizeHint, AnchorPoint
- P1.4: SpriteCache with LRU eviction, session scoping
- P1.5: Security limits (dimensions, Unicode validation, rate limiting)
- P2.1: TUI Block refactor (Protocol Block <-> ColoredCell conversions)

**Discoveries**:
- Evolution level triggers need UX input (Q4 open)
- Visual markers for evolution need design work (Q5 open)

### Sprint 3 (2025-12-31) - Protocol Foundation

**Completed**:
- P1.1: Initial Block struct design

**Blocked**:
- P1.2, P1.3: Deferred to align with Sprint 5 security work

## Completion Criteria

- [ ] All Phase 3 (Evolution) tasks implemented and tested
- [ ] All Phase 4 (Generation) tasks implemented and tested
- [ ] TUI animation loop runs smoothly (target: 60 FPS capability, 4-10 FPS actual)
- [ ] Evolution tracking persists within session
- [ ] Security review passed for sprite generation
- [ ] Accessibility mode (reduced motion) fully functional
- [ ] Performance targets met:
  - Idle CPU: < 5%
  - Animation loop: consistent frame timing
  - Cache hit rate: > 90%
- [ ] Documentation complete

## Open Questions

### Active

| ID | Question | Owner | Target Sprint |
|----|----------|-------|---------------|
| Q6 | Is LLM involved in sprite generation? | Architect + Hacker | Sprint 8 |
| Q7 | Where does sprite generation compute happen? | Architect | Sprint 8 |

### Resolved

| ID | Question | Resolution | Sprint |
|----|----------|------------|--------|
| Q1 | Block color type? | Native Color struct | Sprint 3 |
| Q2 | Sprite data encoding? | JSON via serde | Sprint 3 |
| Q3 | Uncached sprite request? | SpriteRequest message | Sprint 5 |
| Q4 | What triggers evolution level increases? | Dual threshold: interactions + session time (both required) | Sprint 6 |
| Q5 | How many evolution levels and visual markers? | 5 levels (Nascent→Transcendent), markers: glow, particles, color, complexity | Sprint 6 |

## Dependencies

### External Dependencies

- None (all local Rust implementation)

### Internal Dependencies

| Dependency | Status | Blocks |
|------------|--------|--------|
| P1.1 Block struct | Complete | P2.1, P2.2, P3.3 |
| P1.4 SpriteCache | Complete | P2.3, P3.2 |
| P1.5 Security limits | Complete | P4.2, P4.3 |
| Surface Protocol (4.3) | Complete | State sync for evolution |
| Connection Pool (H-002) | Complete | High-load scenarios |
| P3.1-P3.2 Evolution | Complete | P3.3, P3.4 (Variants) |
| P2.4-P2.5 Animation | Complete | P2.6 (Accessibility) |

## Related Documents

- `TODO-avatar-animation-system.md` - Detailed task breakdown and analysis
- `TODO-security-findings.md` - Security findings (M-002, M-003)
- `TODO-accessibility.md` - Reduced motion, accessibility modes
- `docs/yollayah-avatar-constraints.md` - Avatar specification
- `deps.yaml` - Component tracking

---

**Epic Owner**: Architect + TUI Developer
**Last Updated**: 2026-01-02 (Sprint 6 complete)
