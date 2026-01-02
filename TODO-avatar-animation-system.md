# TODO: Avatar Animation System Implementation

> Implementation plan for the Yollayah Avatar Constraints specification.
>
> **Document Version**: 1.0
> **Created**: 2026-01-02
> **Reference**: `docs/yollayah-avatar-constraints.md`

---

## Team Analysis

### Architect Analysis

**Protocol Integration Assessment:**

The current architecture shows a clean separation between Conductor (state owner) and Surface (renderer):

1. **Existing State Machine** (`conductor/core/src/avatar.rs`):
   - `AvatarState` tracks position, mood, size, visibility, wandering, and active gestures/reactions
   - `CommandParser` handles `[yolla:command]` extraction from LLM responses
   - Moods and gestures map to suggested animations via `suggested_animation()`

2. **TUI Implementation** (`tui/src/avatar/`):
   - `AnimationEngine` manages sprite playback with per-size sprite sheets
   - `ActivityManager` handles activity overlays (thinking, construction, etc.)
   - Sprites are hardcoded in `sizes.rs` using character-based patterns

3. **Conductor-Surface Protocol** (`conductor/core/src/messages.rs`, `events.rs`):
   - `ConductorMessage` includes avatar directives (MoveTo, Mood, Size, Gesture, React, etc.)
   - `SurfaceCapabilities` declares avatar and animation support
   - No sprite data transmission currently - surfaces are self-contained

**Gap Analysis (Constraints Doc vs. Current Implementation):**

| Feature | Current Status | Gap |
|---------|----------------|-----|
| Block-based rendering | Implemented (TUI) | Need protocol-level Block struct |
| Partial rendering | Not implemented | Need clipping/viewport support |
| On-the-fly sprite generation | Not implemented | Need GenerateSpriteRequest protocol |
| Animation evolution | Not implemented | Need evolution context tracking |
| Sprite caching | TUI-local only | Need shared cache with eviction policies |
| Cross-session memory | Not implemented | Need persistence layer |
| Dynamic accessories | CustomSprite exists but unused | Need accessory composition system |

**Recommended Protocol Extensions:**

```rust
// New message types for ConductorMessage
AvatarSpriteRequest {
    request_id: RequestId,
    base: SpriteName,
    mood: Option<Mood>,
    size: Option<SizeHint>,
    context: Option<Context>,
    evolution: Option<u8>,
}

AvatarSpriteResponse {
    request_id: RequestId,
    blocks: Vec<Block>,
    dimensions: (u16, u16),
    anchor: AnchorPoint,
    cache_key: Option<String>,
    ttl: Option<Duration>,
}

AvatarAnimationRequest {
    request_id: RequestId,
    name: String,
    duration: Option<Duration>,
    loop_behavior: LoopBehavior,
    interruptible: bool,
    evolution_context: Option<EvolutionContext>,
}
```

**Caching Architecture Recommendation:**

```
┌─────────────────────────────────────────────────────────────────┐
│                        Conductor                                │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                   SpriteCache                            │  │
│  │  ┌─────────────┐  ┌──────────────┐  ┌────────────────┐  │  │
│  │  │ Base Sprites│  │Derived Cache │  │ Evolution Cache│  │  │
│  │  │ (Never evict)│  │ (LRU + TTL)  │  │ (Session scope)│  │  │
│  │  └─────────────┘  └──────────────┘  └────────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                  │
│                    ConductorMessage                             │
│                              │                                  │
└──────────────────────────────┼──────────────────────────────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
   ┌────▼────┐           ┌─────▼─────┐         ┌─────▼─────┐
   │   TUI   │           │   WebUI   │         │  Mobile   │
   │ Local   │           │ Local     │         │ Local     │
   │ Cache   │           │ Cache     │         │ Cache     │
   └─────────┘           └───────────┘         └───────────┘
```

---

### Hacker Analysis (Security Focus)

**Threat Model:**

1. **On-the-fly Sprite Generation Risks:**
   - **Prompt Injection**: Malicious LLM output could embed commands in `GenerateSpriteRequest.description`
   - **Resource Exhaustion**: Unbounded generation requests could DoS the system
   - **Unsafe Content**: Generated sprites could contain offensive/harmful imagery

2. **Cache Poisoning Vectors:**
   - **Cache Key Collision**: Attacker crafts cache_key that overwrites legitimate sprites
   - **TTL Manipulation**: Long TTLs on malicious sprites persist the attack
   - **Cross-Session Contamination**: Shared caches could leak between users

3. **Resource Exhaustion Attacks:**
   - **Sprite Bomb**: Request millions of unique sprites to exhaust memory
   - **Animation Loop**: Request infinitely-looping animations to exhaust CPU
   - **Block Overflow**: Request sprite with enormous dimensions

4. **Protocol Abuse:**
   - **Replay Attacks**: Re-send old sprite responses to cause UI inconsistency
   - **Sequence Breaking**: Send responses out of order to corrupt state
   - **Unicode Exploits**: Malformed Unicode in block characters could crash renderers

**Mitigation Recommendations:**

| Threat | Mitigation | Priority |
|--------|------------|----------|
| Prompt injection in descriptions | Sanitize/validate descriptions, allowlist patterns | CRITICAL |
| Unbounded sprite requests | Rate limiting per session, max pending requests | HIGH |
| Sprite dimensions overflow | Hard cap on dimensions (e.g., 100x100 max) | HIGH |
| Cache key manipulation | Conductor-controlled cache keys, never trust surface | HIGH |
| Memory exhaustion | Global memory budget for sprite cache | HIGH |
| Unicode exploits | Validate characters against Block Elements whitelist | MEDIUM |
| Cross-session contamination | Session-isolated caches, secure cache key namespacing | MEDIUM |
| Malicious generated content | Content policy checks, admin review for custom sprites | MEDIUM |

**Security Invariants:**

```rust
// These MUST be enforced:
const MAX_SPRITE_WIDTH: u16 = 100;
const MAX_SPRITE_HEIGHT: u16 = 100;
const MAX_BLOCKS_PER_SPRITE: usize = 10_000;
const MAX_PENDING_REQUESTS_PER_SESSION: usize = 10;
const MAX_CACHE_SIZE_BYTES: usize = 10 * 1024 * 1024; // 10MB
const MAX_ANIMATION_FRAMES: usize = 100;
const MAX_ANIMATION_DURATION_MS: u64 = 60_000; // 1 minute

// Allowed Unicode ranges for block characters
const ALLOWED_UNICODE_RANGES: &[(u32, u32)] = &[
    (0x2580, 0x259F), // Block Elements
    (0x2500, 0x257F), // Box Drawing
    (0x25A0, 0x25FF), // Geometric Shapes
    (0x2800, 0x28FF), // Braille Patterns
    (0x0020, 0x007E), // Basic ASCII
];
```

---

### UX Specialist Analysis

**Animation Freshness & "Aliveness":**

The constraints document emphasizes that Yollayah should feel "alive" through:
1. Animation variations (not always the same)
2. Occasional "surprise" variants
3. Evolution over time within session
4. Cross-session memory (optional)

**Current TUI Implementation Assessment:**
- Static sprite sets with no variation
- No evolution tracking
- Limited animation variety (7-8 animations per size)

**Recommendations for Perceived Aliveness:**

1. **Micro-variations:**
   - Add subtle timing jitter (5-15%) to frame durations
   - Randomly select between 2-3 variants of the same animation
   - Occasional "idle fidgets" during long waits

2. **Evolution Indicators:**
   - Visual cues when evolution level increases (sparkle, color shift)
   - Accessories that appear over time (glasses after extended thinking)
   - Mood drift based on interaction history

3. **Partial Rendering UX:**
   - Peeking should feel intentional, not broken
   - Smooth entry/exit transitions for partial views
   - Clear visual affordance that avatar continues beyond viewport

**Accessibility Considerations:**

| Concern | Recommendation |
|---------|----------------|
| Motion sensitivity | Provide "reduced motion" mode that uses static sprites |
| Screen reader support | Announce avatar state changes as aria-live regions |
| Color blindness | Ensure mood/state info isn't color-only (add shape/pattern) |
| Cognitive load | Avatar shouldn't compete with primary content for attention |
| Photosensitivity | No rapid flashing (< 3 Hz), avoid high-contrast rapid changes |

**Animation Timing Guidelines:**

| Animation Type | Frame Rate | Loop Behavior | User Expectation |
|----------------|------------|---------------|------------------|
| Idle | 2-4 fps | Loop | Subtle, unobtrusive |
| Thinking | 4-6 fps | Loop | Active but patient |
| Responding | 6-8 fps | Loop | Engaged, speaking |
| Celebrating | 8-10 fps | Once or short loop | Brief joy |
| Error | 4-6 fps | Short loop | Concerned but recoverable |

---

### Intern Analysis (Implementation Gotchas)

**Complexity Concerns:**

1. **State Synchronization:**
   - Conductor avatar state vs. Surface avatar state can drift
   - What happens if surface is slow and misses animation updates?
   - Race conditions between sprite requests and animation changes

2. **Cache Coherency:**
   - When does a surface know its cached sprite is stale?
   - How to handle cache misses during animation playback?
   - Memory pressure handling across multiple surfaces

3. **Evolution State Management:**
   - Who owns evolution state - Conductor or Surface?
   - How is evolution persisted across sessions?
   - What triggers evolution increments?

4. **Block Encoding:**
   - Current `ColoredCell` uses ratatui's `Color` enum directly
   - Need surface-agnostic color representation for protocol
   - Alpha channel support varies by surface

**Questions That Need Answering:**

1. **Sprite Generation:**
   - Who generates sprites - Conductor or a dedicated Sprite Service?
   - Is LLM involved in sprite generation, or is it rule-based?
   - How do we prevent LLM hallucinating invalid sprite data?

2. **Protocol Versioning:**
   - How do we handle surfaces that don't support new sprite features?
   - Graceful degradation strategy for older surfaces?

3. **Performance:**
   - What's the latency budget for sprite generation?
   - Should sprites be streamed or sent atomically?
   - How to handle large sprites on slow connections?

4. **Testing:**
   - How to test animation "freshness" objectively?
   - Snapshot testing for sprite rendering?
   - Property-based testing for cache eviction?

5. **Edge Cases:**
   - What if terminal doesn't support required Unicode blocks?
   - What if true color isn't available?
   - What about 80x24 terminals vs. 4K terminals?

---

## Implementation Phases

### Phase 1: Protocol Foundation (Week 1-2)

**Priority: CRITICAL**

**Tasks:**

- [x] **P1.1** Define `Block` struct in `conductor-core` with surface-agnostic color representation ✓ Sprint 3
  - Dependencies: None
  - Owner: Architect
  - Acceptance: Block can be serialized/deserialized, supports RGBA colors
  - **Completed**: Created `conductor/core/src/avatar/block.rs` with:
    - `Color` struct (RGBA, blend_over, lerp, to_hex)
    - `Block` struct (fg, bg, character, transparency, z_index)
    - `SizeHint` enum (Blocks, Relative, FitWithin, Minimal, Fill)
    - `AnchorPoint` enum (9 positions with offset calculation)
    - Comprehensive unit tests for serialization and operations

- [x] **P1.2** Add `SpriteRequest` and `SpriteResponse` to `ConductorMessage` enum ✓ Sprint 5
  - Dependencies: P1.1 ✓
  - Owner: Architect
  - Acceptance: Protocol compiles, can be sent over transport

- [x] **P1.3** Add `AnimationRequest` and `AnimationResponse` to protocol ✓ Sprint 5
  - Dependencies: P1.1 ✓
  - Owner: Architect
  - Acceptance: Animation metadata can be transmitted

- [x] **P1.4** Implement `SpriteCache` in Conductor with LRU eviction ✓ Sprint 5
  - Dependencies: P1.1 ✓
  - Owner: Backend
  - Acceptance: Cache respects memory budget, evicts correctly
  - **Completed**: Created `conductor/core/src/avatar/cache.rs`

- [x] **P1.5** Add security limits to sprite handling ✓ Sprint 5
  - Dependencies: P1.2
  - Owner: Hacker
  - Acceptance: All `MAX_*` constants enforced, tests for overflow
  - **Completed**: Created `conductor/core/src/avatar/security.rs`

- [ ] **P1.6** Update `SurfaceCapabilities` to include sprite-related capabilities
  - Dependencies: P1.2
  - Owner: Architect
  - Acceptance: Surfaces can declare partial_rendering, sprite_generation support

### Phase 2: TUI Migration (Week 3-4)

**Priority: HIGH**

**Tasks:**

- [x] **P2.1** Refactor `ColoredCell` to use protocol `Block` type ✓ Sprint 5
  - Dependencies: P1.1
  - Owner: TUI
  - Acceptance: TUI renders using Block, no visual regression
  - **Completed**: Protocol Block <-> ColoredCell conversions, color mapping functions

- [ ] **P2.2** Implement sprite request/response handling in TUI
  - Dependencies: P1.2, P2.1
  - Owner: TUI
  - Acceptance: TUI can request and render dynamic sprites

- [ ] **P2.3** Add local sprite cache to TUI with fallback to hardcoded sprites
  - Dependencies: P2.2
  - Owner: TUI
  - Acceptance: TUI uses cache, falls back gracefully

- [ ] **P2.4** Implement partial rendering support (clipping, viewport)
  - Dependencies: P2.1
  - Owner: TUI
  - Acceptance: Avatar can peek from edges correctly

- [ ] **P2.5** Add reduced-motion accessibility mode
  - Dependencies: P2.1
  - Owner: UX
  - Acceptance: Static sprites when accessibility mode enabled

### Phase 3: Animation Evolution (Week 5-6)

**Priority: MEDIUM**

**Tasks:**

- [ ] **P3.1** Define `EvolutionContext` struct and storage
  - Dependencies: P1.3
  - Owner: Architect
  - Acceptance: Evolution state can be tracked per-session

- [ ] **P3.2** Implement evolution tracking in Conductor
  - Dependencies: P3.1
  - Owner: Backend
  - Acceptance: Evolution level increments based on activity

- [ ] **P3.3** Add animation variants (2-3 per animation type)
  - Dependencies: P2.1
  - Owner: TUI/Art
  - Acceptance: Idle has 3 variants, randomly selected

- [ ] **P3.4** Implement micro-variation timing (jitter)
  - Dependencies: P3.3
  - Owner: TUI
  - Acceptance: Frame timing varies by 5-15%

- [ ] **P3.5** Add accessory composition system
  - Dependencies: P3.2, P2.1
  - Owner: TUI
  - Acceptance: Glasses appear after extended thinking

### Phase 4: Dynamic Generation (Week 7-8)

**Priority: MEDIUM**

**Tasks:**

- [ ] **P4.1** Design sprite generation pipeline (rule-based initially)
  - Dependencies: P1.2
  - Owner: Architect
  - Acceptance: Architecture document approved

- [ ] **P4.2** Implement `GenerateSpriteRequest` with input validation
  - Dependencies: P4.1, P1.5
  - Owner: Backend
  - Acceptance: Requests sanitized, rate-limited

- [ ] **P4.3** Create sprite generation rules for accessories
  - Dependencies: P4.2
  - Owner: Backend/Art
  - Acceptance: Can generate "party hat", "glasses", "coffee" variants

- [ ] **P4.4** Implement content policy checks for generated sprites
  - Dependencies: P4.2
  - Owner: Hacker
  - Acceptance: Offensive content blocked, logged

- [ ] **P4.5** Add sprite preview/approval workflow (optional)
  - Dependencies: P4.3
  - Owner: UX
  - Acceptance: Admins can review generated sprites

### Phase 5: Cross-Session Memory (Week 9-10)

**Priority: LOW**

**Tasks:**

- [ ] **P5.1** Design persistence schema for evolution state
  - Dependencies: P3.1
  - Owner: Architect
  - Acceptance: Schema supports versioning, migration

- [ ] **P5.2** Implement secure storage for session history
  - Dependencies: P5.1
  - Owner: Backend
  - Acceptance: Data encrypted at rest, per-user isolation

- [ ] **P5.3** Add "personality drift" based on interaction history
  - Dependencies: P5.2, P3.2
  - Owner: Backend
  - Acceptance: Avatar behavior reflects past interactions

- [ ] **P5.4** Implement memory decay (optional features fade over time)
  - Dependencies: P5.3
  - Owner: Backend
  - Acceptance: Old features decay, fresh ones remain

---

## Dependency Graph

```
P1.1 (Block struct)
 ├── P1.2 (SpriteRequest/Response)
 │    ├── P1.3 (AnimationRequest/Response)
 │    ├── P1.5 (Security limits)
 │    │    └── P4.2 (GenerateSpriteRequest validation)
 │    ├── P1.6 (SurfaceCapabilities update)
 │    ├── P2.2 (TUI sprite handling)
 │    └── P4.1 (Generation pipeline design)
 │         └── P4.3 (Accessory rules)
 │              └── P4.4 (Content policy)
 │                   └── P4.5 (Preview workflow)
 ├── P1.4 (SpriteCache)
 │    └── P2.3 (TUI local cache)
 └── P2.1 (ColoredCell refactor)
      ├── P2.4 (Partial rendering)
      ├── P2.5 (Reduced motion)
      ├── P3.3 (Animation variants)
      │    └── P3.4 (Timing jitter)
      └── P3.5 (Accessory composition)

P3.1 (EvolutionContext)
 ├── P3.2 (Evolution tracking)
 │    └── P3.5 (Accessory composition)
 └── P5.1 (Persistence schema)
      └── P5.2 (Secure storage)
           └── P5.3 (Personality drift)
                └── P5.4 (Memory decay)
```

---

## Security Considerations

### Input Validation Requirements

1. **Sprite Dimensions:**
   ```rust
   fn validate_sprite_dimensions(width: u16, height: u16) -> Result<(), SpriteError> {
       if width > MAX_SPRITE_WIDTH || height > MAX_SPRITE_HEIGHT {
           return Err(SpriteError::DimensionOverflow);
       }
       if (width as usize * height as usize) > MAX_BLOCKS_PER_SPRITE {
           return Err(SpriteError::TooManyBlocks);
       }
       Ok(())
   }
   ```

2. **Unicode Character Validation:**
   ```rust
   fn is_allowed_block_char(c: char) -> bool {
       let code = c as u32;
       ALLOWED_UNICODE_RANGES.iter().any(|(start, end)| {
           code >= *start && code <= *end
       })
   }
   ```

3. **Rate Limiting:**
   - Max 10 pending sprite requests per session
   - Max 100 sprite requests per minute per session
   - Exponential backoff on repeated failures

### Cache Security

1. **Cache Key Generation:**
   - Always generate cache keys server-side
   - Include session_id in cache key namespace
   - Use cryptographic hash for derived cache keys

2. **Eviction Security:**
   - Base sprites cannot be evicted by user requests
   - Session-scoped caches isolated per user
   - Memory limits enforced globally

### Content Policy

1. **Generated Sprite Review:**
   - All generated sprites logged for audit
   - Flagged content requires human review
   - Automatic rejection of known-bad patterns

---

## UX Guidelines

### Animation Timing

| Context | Recommended Behavior |
|---------|---------------------|
| User idle < 10s | Idle animation |
| User idle 10-30s | Transition to waiting with occasional fidgets |
| User idle > 30s | May enter playful state (probability based on playfulness setting) |
| Streaming response | Talking animation, follow response text |
| Error occurred | Error animation for 5s, then return to idle |
| Task completed | Brief celebration (1-2s), then idle |

### Partial Rendering

1. **Peeking Behavior:**
   - Enter: Slide in over 200-300ms
   - Hold: Stay peeking for duration
   - Exit: Slide out over 200-300ms
   - Never abrupt cuts

2. **Zoomed View:**
   - Use for dramatic moments
   - Show eyes/face for emotional connection
   - Return to full view within 3-5 seconds

3. **Occluded:**
   - Avatar behind UI elements shows only visible portion
   - No animation changes needed, just clipping

### Accessibility Modes

1. **Reduced Motion:**
   - Static sprite (no animation)
   - State changes communicated via screen reader
   - All functionality preserved

2. **High Contrast:**
   - Increase color saturation
   - Add outline to avatar
   - Ensure 4.5:1 contrast ratio minimum

3. **Screen Reader:**
   - Announce avatar state: "Yollayah is thinking"
   - Announce mood changes: "Yollayah is happy"
   - No announcements for animation frame changes

---

## Open Questions

### Must Answer Before Phase 1

1. **Q1**: Should `Block` use a protocol-specific color type, or can we depend on a common library like `rgb`?
   - Impact: API stability, serialization format
   - Owner: Architect
   - **ANSWERED (Sprint 3)**: Using native `Color` struct for minimal dependencies and full serialization control

2. **Q2**: What transport encoding for sprite data - JSON or binary (MessagePack/bincode)?
   - Impact: Performance, debugging ease
   - Owner: Architect
   - **ANSWERED (Sprint 3)**: JSON via serde - all Block types derive Serialize/Deserialize

3. **Q3**: How does a surface request a sprite it doesn't have cached?
   - Impact: Protocol flow, latency
   - Owner: Architect
   - **PENDING**: To be defined in P1.2 (SpriteRequest/SpriteResponse)

### Must Answer Before Phase 3

4. **Q4**: What triggers evolution level increases?
   - Options: Time-based, interaction count, task completion, explicit command
   - Impact: UX feel, complexity
   - Owner: UX + Backend

5. **Q5**: How many evolution levels and what are the visual markers?
   - Impact: Art requirements, state size
   - Owner: UX + Art

### Must Answer Before Phase 4

6. **Q6**: Is LLM involved in sprite generation?
   - Options: Pure rule-based, LLM describes then rules execute, LLM generates raw blocks
   - Impact: Latency, safety, creativity
   - Owner: Architect + Hacker

7. **Q7**: Where does sprite generation compute happen?
   - Options: In Conductor, separate service, surface-side
   - Impact: Architecture, latency, resource usage
   - Owner: Architect

### Nice to Answer Eventually

8. **Q8**: Cross-session memory - local storage vs. cloud sync?
   - Impact: Privacy, multi-device experience
   - Owner: Product + Hacker

9. **Q9**: User customization of avatar colors?
   - Impact: Art requirements, preference storage
   - Owner: UX + Product

---

## Metrics & Success Criteria

### Phase 1 Success

- [ ] Protocol messages can be serialized/deserialized without data loss
- [ ] Sprite cache respects memory limits under stress test
- [ ] Security limits prevent all known attack vectors

### Phase 2 Success

- [ ] TUI renders dynamic sprites with no visual regression
- [ ] Partial rendering works on 80x24, 120x40, and 200x50 terminals
- [ ] Accessibility mode provides equivalent functionality

### Phase 3 Success

- [ ] Users cannot predict which animation variant will play
- [ ] Evolution changes are noticeable but not distracting
- [ ] Accessories appear contextually appropriate

### Phase 4 Success

- [ ] Generated sprites match style guide (blocky, correct palette)
- [ ] Generation latency < 100ms for simple requests
- [ ] No offensive content reaches users

### Phase 5 Success

- [ ] Avatar "remembers" user after 24-hour gap
- [ ] Memory decay feels natural, not arbitrary
- [ ] Privacy preferences respected (opt-out possible)

---

## References

- `docs/yollayah-avatar-constraints.md` - Specification document
- `conductor/core/src/avatar.rs` - Current avatar state machine
- `conductor/core/src/messages.rs` - Conductor-Surface protocol
- `conductor/core/src/events.rs` - Surface-Conductor events
- `conductor/core/src/animation/mod.rs` - Animation abstractions
- `tui/src/avatar/` - TUI sprite implementation
