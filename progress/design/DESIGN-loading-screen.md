# Loading Screen Design for Yollayah TUI

**Created**: 2026-01-03
**Task**: STORY 3 - Design Yollayah-Themed Loading UI (TODO-tui-initialization.md)
**Status**: Complete - Recommendation Ready

---

## Theme Analysis

Based on audit of `tui/src/avatar/sprites.rs` and `tui/src/theme/mod.rs`:

### Yollayah Design Language

**Color Palette**:
- Body: Soft pink (#FFB6C1) - `AXOLOTL_BODY`
- Body shadow: Darker pink (#DB9CA0) - `AXOLOTL_BODY_SHADOW`
- Body highlight: Lighter pink (#FFDAE0) - `AXOLOTL_BODY_HIGHLIGHT`
- Gills: Coral/salmon (#FF7F7F) - `AXOLOTL_GILLS`
- Gills highlight: Bright coral (#FFA0A0) - `AXOLOTL_GILLS_HIGHLIGHT`
- Eyes: Dark (#282828) - `AXOLOTL_EYES`
- Eye shine: White (#FFFFFF) - `AXOLOTL_EYE_SHINE`
- Mouth: Mauve (#B46478) - `AXOLOTL_MOUTH`
- Belly: Warm light (#FFDCD2) - `AXOLOTL_BELLY`

**UI Accent Colors**:
- Happy/excited: Warm yellow (#FFDF80) - `MOOD_HAPPY`
- Thinking: Soft blue (#96B4FF) - `MOOD_THINKING`
- Error: Muted red (#FF6464) - `MOOD_ERROR`
- Magenta accent: `YOLLAYAH_MAGENTA`
- Status ready: Bright magenta (#FF8CFF) - `STATUS_READY_COLOR`
- Status thinking: Warm yellow (#FFDF80) - `STATUS_THINKING_COLOR`

**Animation Style**:
- Blocky Unicode sprites (█▀▄▌▐ block elements)
- Expressive eyes (oo, --, ^^, ..)
- Cute mouth expressions (w, v, n, O)
- Frame-based animation with `build_animation()` helper
- Breathing effects: sine-wave color oscillation using `breathing_color()`

**Avatar Sizes Available**:
- Tiny: 6x2 - peeking head
- Small: 12x4 - quick interactions
- Medium: 18x6 - normal interactions (default)
- Large: 26x10 - celebrations

### Existing Animation Examples

From `tui/src/avatar/sizes.rs`:

```rust
// Tiny sprite (6x2) - peeking head
fn load_tiny() -> SpriteSheet {
    // Idle with blink cycle
    build_animation("idle", &[
        (&[" TLLT ", " BooBL"], 2000),  // Eyes open
        (&[" TLLT ", " B--BL"], 150),   // Eyes closed
    ], &palette, true)

    // Thinking (curious eyes)
    build_animation("thinking", &[
        (&["?TLLT ", " BooBL"], 400),   // Thinking accent + open eyes
        (&[" TLLT?", " Bo.BL"], 400),   // Different accent position
    ], &palette, true)
}
```

---

## Three Design Options

### Option A: Simple Text Spinner (Minimal)

**Visual**:
```
╭─────────────────────────────────────╮
│                                     │
│    Connecting to Yollayah...        │
│         ⠙ Loading                   │
│                                     │
│  Press Ctrl+C to cancel             │
╰─────────────────────────────────────╯
```

**Animation**:
- Spinner rotates: ⠙⠹⠸⠼⠴⠦⠧⠇ (8 frames, 100ms each)
- Text cycles: "Connecting..." → "Warming up..." → "Almost ready!"
- Color: Breathing between `DIM_GRAY` and `YOLLAYAH_MAGENTA`

**Technical**:
- Uses ratatui `Gauge` widget
- Frame rate: 10fps (100ms)
- Spinner: 8-frame Unicode cycle
- Color: `breathing_color()` with 800ms cycle

**Code Size**: ~40 lines
**Implementation Time**: 1.5 hours
**Delight Factor**: 6/10

**Pros**:
- Minimal code, maximum reliability
- Clear status messages
- Instant render (<50ms)
- Minimal CPU usage
- Works on all terminal sizes

**Cons**:
- Generic (no Yollayah personality)
- Spinner feels bland for first impression
- Doesn't leverage cute axolotl character
- Less brand recognition

---

### Option B: Pulsing Axolotl Head (RECOMMENDED)

**Visual** (centered on screen):
```
╭─────────────────────────────────────╮
│                                     │
│        Just starting...             │
│                                     │
│           TLLT                      │
│           BooBL    ← breathing      │
│                                     │
│        Press Ctrl+C to cancel       │
╰─────────────────────────────────────╯
```

**Animation Sequence** (1 second cycle):
```
Frame 1 (0-333ms):
  Axolotl: standard appearance
    Gills: dim pink (AXOLOTL_GILLS)
    Eyes: open (oo)
  Text: dim gray (DIM_GRAY)
  Message: "Just starting..."

Frame 2 (333-666ms):
  Axolotl: awake/alert appearance
    Gills: bright coral (AXOLOTL_GILLS_HIGHLIGHT)
    Eyes: happy (^^)
  Text: bright magenta (YOLLAYAH_MAGENTA)
  Message: "Just starting..." (held)

Frame 3 (666-1000ms):
  Back to Frame 1 (continuous loop)

Message Rotation (every 2 seconds):
  "Just starting..." (0-2s)
  "Warming up..." (2-4s)
  "Almost there..." (4-6s)
  (loops if still loading)
```

**Technical**:
- Reuses `Tiny` avatar sprite (6x2 cells)
- Three frames built with `build_frame(palette)`
- Color breathing: 1000ms cycle using `breathing_color()`
- Status rotation: every 2000ms
- Frame rate: 10fps (100ms per render)
- Implementation: direct sprite system usage

**Code Size**: ~100-150 lines
**Implementation Time**: 2 hours
**Delight Factor**: 8/10

**Color Implementation**:
```rust
// Gills oscillate between dim and bright
let gills_color = breathing_color(
    AXOLOTL_GILLS,              // Dim state
    AXOLOTL_GILLS_HIGHLIGHT,    // Bright state
    1000,                        // 1 second cycle
    self.elapsed
);

// Text color follows same rhythm
let text_color = breathing_color(
    DIM_GRAY,          // Dim
    YOLLAYAH_MAGENTA,  // Bright
    1000,
    self.elapsed
);
```

**Sprite Building**:
```rust
let palette = vec![
    ('T', '▀', AXOLOTL_BODY),
    ('L', '▄', AXOLOTL_BODY),
    ('B', '█', AXOLOTL_BODY),
    ('o', 'o', AXOLOTL_EYES),
    ('^', '^', AXOLOTL_EYES),
];

// Frame 1: Dim state
let frame_dim = build_frame(&[
    " TLLT ",
    " BooBL"
], &palette, 333);

// Frame 2: Bright state (happy)
let frame_bright = build_frame(&[
    " TLLT ",
    " B^^BL"
], &palette, 333);
```

**Pros**:
- Reuses existing sprite infrastructure (no new assets)
- Yollayah "comes alive" during loading (personality)
- Immediate brand recognition (it's the axolotl!)
- Color palette recognizable to users
- "Breathing" metaphor perfect (AI waking up)
- Works on all terminal sizes (6x2 minimum)
- Minimal CPU usage
- Leverages `theme::breathing_color()` function already written
- Sets right tone: "This is a fun, friendly AI"

**Cons**:
- Slightly more code than Option A
- Requires understanding sprite building system
- Small sprite might be hard to see on very small terminals

**Why This Wins**:
- Perfect balance: delightful but not over-engineered
- Not using over-complex approach (Option C: 4 hours)
- Reuses all existing code (lower risk)
- First user interaction is with the actual Yollayah character
- Breathing effect is perfect metaphor for model initialization
- Can be upgraded to Option C later without waste

---

### Option C: Animated Swimming Progress Bar (Fancy)

**Visual**:
```
╭─────────────────────────────────────╮
│                                     │
│  Warming up your AI friend...       │
│                                     │
│  [░░░░ ⚬ ░░░░░]  40%               │
│     (Tiny axolotl character)        │
│                                     │
│  ✓ Connecting to Conductor...       │
│  ⟳ Initializing model...             │
│  ⏳ Press Ctrl+C to cancel             │
│                                     │
╰─────────────────────────────────────╯
```

**Animation**:
- Axolotl character swims left→right through progress bar
- Gills flutter every 200ms while swimming
- Eyes blink occasionally for life
- Character expression changes at milestones

**Three Phases**:
- Phase 1 (0-30%): "Connecting to Conductor"
- Phase 2 (30-70%): "Initializing model"
- Phase 3 (70-100%): "Finalizing startup"

**Technical**:
- Custom progress widget (new type)
- Multiple sub-animations coordinated
- Phase/progress calculation logic
- Character positioning based on progress percentage

**Code Size**: ~180-250 lines
**Implementation Time**: 3-4 hours
**Delight Factor**: 9/10

**Pros**:
- Shows actual progress (not just spinning)
- Highly engaging—keeps user watching
- Scales to terminal width (responsive)
- Shows Yollayah is working (swimming = activity)
- Playful metaphor (swimming to completion)
- Maximum brand expression

**Cons**:
- Most code (new custom widget required)
- Complex phase/progress coordination
- CPU overhead for multiple animations
- Overkill complexity for typical <3 second loads
- Hard to test and maintain
- Might feel condescending if loading is fast

**When to Use This Instead**:
- Only if loading consistently takes >3 seconds
- Only after Option B is proven working
- Can be built as upgrade on top of Option B later
- Progressive enhancement strategy

---

## RECOMMENDATION: Option B (Pulsing Axolotl Head)

### Decision

**Implement Option B** for these reasons:

1. **Goldilocks Sweet Spot**
   - Better than minimal Option A (has personality)
   - Simpler than over-engineered Option C (2 hours vs 4 hours)
   - Right level of effort for 100ms load time typical

2. **Reuses Existing Infrastructure**
   - Sprite system already built (`sprites.rs`)
   - Breathing color function already written (`theme::breathing_color()`)
   - No new widget types needed
   - Lower implementation risk

3. **Delightful UX**
   - Yollayah personality shown from first frame (not blank)
   - Eyes open → happy → curious shows expression
   - Breathing gills metaphor: "AI waking up"
   - Color palette immediately recognizable

4. **Implementation Speed**
   - 2 hours vs 4 hours for Option C
   - 40 lines vs 250 lines
   - Uses existing code patterns
   - Fewer bugs likely

5. **Practical**
   - Works on 80x24 terminal (minimum recommended)
   - Super fast rendering (<50ms)
   - Minimal CPU: just color interpolation
   - Terminal-size agnostic (Tiny sprite fits anywhere)

6. **Future-Proof**
   - If loading proves slow later, upgrade to Option C
   - Option B code becomes reusable component (the swimmer)
   - Can embed Option B sprite in Option C progress bar
   - Zero wasted effort

7. **Brandable**
   - Sets expectation: "This is fun, friendly AI"
   - First interaction is with Yollayah itself
   - Memorable first impression
   - Different from generic spinners everywhere

---

## Implementation Plan

### File Structure

**New File**: `tui/src/loading.rs` (~150-200 lines)

**Struct Definition**:
```rust
/// Loading screen with pulsing Yollayah avatar
pub struct LoadingScreen {
    elapsed: Duration,
    phase: LoadingPhase,
    current_frame: usize,      // 0-2, cycles every 333ms
    message_index: usize,       // cycles every 2 seconds
}

pub enum LoadingPhase {
    Connecting,
    WarmingUp,
    Ready,
}

pub enum LoadingMessage {
    JustStarting,
    WarmingUp,
    AlmostThere,
}
```

### Core Implementation

**Frame Generation** (using existing sprite helpers):
```rust
fn build_frames() -> [Frame; 3] {
    let palette = vec![
        ('T', '▀', AXOLOTL_BODY),
        ('L', '▄', AXOLOTL_BODY),
        ('B', '█', AXOLOTL_BODY),
        ('o', 'o', AXOLOTL_EYES),
        ('^', '^', AXOLOTL_EYES),
        ('.', '.', AXOLOTL_EYES),
    ];

    [
        build_frame(&[" TLLT ", " BooBL"], &palette, 100),  // Dim + open eyes
        build_frame(&[" TLLT ", " B^^BL"], &palette, 100),  // Bright + happy eyes
        build_frame(&[" TLLT ", " Bo.BL"], &palette, 100),  // Curious eyes
    ]
}
```

**Color Breathing**:
```rust
pub fn render(&self, f: &mut Frame) {
    // Calculate color based on elapsed time
    let gills_color = breathing_color(
        AXOLOTL_GILLS,
        AXOLOTL_GILLS_HIGHLIGHT,
        1000,
        self.elapsed
    );

    let text_color = breathing_color(
        DIM_GRAY,
        YOLLAYAH_MAGENTA,
        1000,
        self.elapsed
    );

    // Render sprite...
    // Render text with text_color...
}
```

### Implementation Checklist

**Setup**:
- [ ] Create `tui/src/loading.rs`
- [ ] Import color palette from `theme`
- [ ] Import sprite builders from `avatar::sprites`
- [ ] Import `Frame`, `SpriteSheet` types

**Data Structure**:
- [ ] Define `LoadingScreen` struct
- [ ] Define `LoadingPhase` enum
- [ ] Define `LoadingMessage` enum
- [ ] Store frame cache (3 frames)
- [ ] Track elapsed time (for color breathing)
- [ ] Track animation state (current frame)
- [ ] Track message state (current message)

**Animation Logic**:
- [ ] Implement `new()` constructor
- [ ] Implement `tick(delta: Duration)` method
  - [ ] Update elapsed time
  - [ ] Calculate frame index (0-2) based on 333ms per frame
  - [ ] Calculate message index based on 2000ms per message
- [ ] Implement `set_phase()` for phase changes
- [ ] Implement phase-to-message mapping

**Rendering**:
- [ ] Implement `render(&self, f: &mut Frame)` method
- [ ] Center sprite on screen
- [ ] Calculate breathing colors for current frame
- [ ] Draw sprite with breathing-colored gills
- [ ] Draw eyes from current frame
- [ ] Render status message below sprite
- [ ] Render "Press Ctrl+C to cancel" footer
- [ ] Use ratatui `Paragraph` widget for text
- [ ] Use ratatui `Canvas` or direct cell drawing for sprite

**Integration Points**:
- [ ] Export `LoadingScreen` in `lib.rs`
- [ ] Prepare for use in `app.rs` (don't integrate yet - STORY 4)

**Testing**:
- [ ] Manual visual test: 80x24 terminal
- [ ] Manual visual test: 120x40 terminal
- [ ] Verify color breathing smooth (no flicker)
- [ ] Verify frame transitions smooth
- [ ] Verify message rotation at 2s intervals
- [ ] Verify "Press Ctrl+C" renders clearly

### Code Complexity Estimate

| Component | Lines | Complexity |
|-----------|-------|-----------|
| Imports/Setup | 20 | Low |
| Data structures | 30 | Low |
| `new()` constructor | 15 | Low |
| `tick()` method | 20 | Medium |
| `set_phase()` method | 10 | Low |
| `render()` method | 80 | Medium |
| Tests/docs | 25 | Low |
| **Total** | **~200** | **Medium** |

### Performance Targets

| Metric | Target | Why |
|--------|--------|-----|
| Time to first render | <50ms | Must appear before Conductor connects |
| Frame rate | 10fps (100ms) | Smooth animations without wasting CPU |
| Memory footprint | <500KB | Light on resource usage |
| Color flicker | None | Breathing should be smooth |
| Terminal compatibility | 80x24+ | Standard minimum size |

---

## Next Steps

### Immediate (After This Design Approval)

1. **Code Review**: Review this design with team
   - Any concerns about Option B choice?
   - Any additional requirements?

2. **Implementation**: Implement `tui/src/loading.rs`
   - Follow checklist above
   - Target: 2-3 hours work

3. **Testing**: Visual verification
   - Multiple terminal sizes
   - Color breathing smoothness
   - Frame transitions

### Short Term (STORY 4)

4. **Integration**: Wire into app startup
   - Show LoadingScreen before Conductor connect
   - Update phase as connection progresses
   - Transition to main UI when ready

### Future (Option C - Only If Needed)

5. **Upgrade Path**: If loading slow, implement Option C
   - Embed Option B sprite in Option C progress bar
   - Reuse all Option B code
   - Minimal wasted effort

---

## Related Files

- `tui/src/avatar/sprites.rs` - Frame and animation building
- `tui/src/theme/mod.rs` - Color palette and breathing effects
- `tui/src/avatar/sizes.rs` - Avatar size examples
- `TODO-tui-initialization.md` - Full TUI startup project
- `tui/src/main.rs` - Where LoadingScreen will be used

---

## Success Criteria

After implementation, the loading screen should satisfy:

- ✅ Appears immediately (< 50ms, no blank screen)
- ✅ Uses Yollayah branding (avatar, colors, personality)
- ✅ Shows progress indication (text changes as loading proceeds)
- ✅ Supports Ctrl+C cancellation (handled in STORY 4)
- ✅ Delights users (not generic/boring)
- ✅ Works on all terminal sizes
- ✅ Minimal CPU/memory usage
- ✅ Code is clean and maintainable

---

## Timeline

| Task | Estimate | Notes |
|------|----------|-------|
| **Design** | 2 hours | Complete (this document) |
| **Implementation** | 2 hours | `tui/src/loading.rs` |
| **Testing** | 1 hour | Visual verification |
| **Review** | 1 hour | Code review with team |
| **Integration** | 2 hours | STORY 4 (wire into app) |
| **Total** | **8 hours** | Complete in 1 day |

---

## Conclusion

**Option B (Pulsing Axolotl Head)** is the recommended choice because it:

1. Shows Yollayah's personality immediately (no blank screen)
2. Reuses all existing sprite and animation infrastructure
3. Implements in 2 hours (vs 4 hours for Option C)
4. Scales to any terminal size
5. Delightful without being over-engineered
6. Perfect metaphor: breathing axolotl = AI waking up
7. Future-proof: can upgrade to Option C without waste

**Next action**: Seek team approval, then begin implementation in `tui/src/loading.rs`.
