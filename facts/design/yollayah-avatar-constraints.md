# Yollayah Avatar Constraints

> Reference document for all surfaces implementing the Yollayah (YA) avatar system.

## Core Philosophy

Yollayah is a **blocky** character by design. This aesthetic MUST be preserved across ALL surfaces, including high-definition displays. The "block" is the fundamental rendering unit, not a limitation of low-resolution displays.

## Terminology

| Term | Definition |
|------|------------|
| **YA** | Yollayah - the ai-way avatar |
| **Block** | The atomic rendering unit - one character-sized cell with foreground, background, and optional transparency |
| **Render** | A complete or partial representation of YA at a given moment |
| **Sprite** | A static arrangement of blocks forming a visual |
| **Animation** | A sequence of sprites with timing information |
| **Surface** | Any display target (TUI, WebUI, mobile, TV, etc.) |

## Block Specification

### Block Properties

```
+---------------------------+
|  Block (1 character cell) |
+---------------------------+
| - Foreground color (RGBA) |
| - Background color (RGBA) |
| - Character (Unicode)     |
| - Transparency (0.0-1.0)  |
| - Z-index (layering)      |
+---------------------------+
```

### Block Characters

Preferred Unicode ranges for blocky aesthetics:
- Block elements: U+2580-U+259F (▀ ▄ █ ░ ▒ ▓ etc.)
- Box drawing: U+2500-U+257F
- Geometric shapes: U+25A0-U+25FF
- Braille patterns: U+2800-U+28FF (for fine detail)

### Color Palette

YA has a canonical color palette that surfaces should respect:
- Primary: Axolotl pink/coral tones
- Secondary: Soft purples and teals
- Accent: Bright highlights for expressions
- Neutral: Grays for shadows and depth

## Render Constraints

### Partial Rendering

YA can be rendered **partially**. This is not a fallback - it's a feature.

Valid partial renders include:
- **Peeking**: Only part of YA visible from edge/corner
- **Zoomed**: Close-up showing only portion of full sprite
- **Occluded**: Parts hidden behind UI elements
- **Cropped**: Intentional framing for effect

```
Full Render:        Peeking:           Zoomed:
+----------+       +----------+       +----------+
|  .--.    |       |          |       |   @@     |
| (o  o)   |       |          |       |  (  )    |
|  \__/    |       |     .--.▌|       |   \/     |
|  /||\    |       |    (o  o▌|       |          |
+----------+       +----------+       +----------+
```

### Size Flexibility

Renders have NO fixed size requirements:
- Minimum: 1 block (a tiny presence indicator)
- Maximum: Entire screen (dramatic moments)
- Aspect ratio: Flexible, YA adapts

### Positioning

YA can appear ANYWHERE on the surface:
- Corners, edges, center
- Overlapping content (with z-index)
- Multiple instances (rare, special effects)
- Off-screen with partial visibility

## Animation System

### Animation Types

1. **Idle Animations**: Subtle movements when waiting
2. **Reaction Animations**: Response to events
3. **Transition Animations**: Moving between states
4. **Activity Animations**: Extended sequences for tasks
5. **Personality Animations**: Character-building moments

### Animation Evolution

Animations are NOT static. They EVOLVE based on context:

```
"cleanup yollayah" progression:
  Start:    Cheerful, energetic
  Middle:   Getting focused, efficient
  Extended: Develops attitude, eye-rolls

"working yollayah" progression:
  Start:    Casual, ready
  Middle:   Serious, puts on glasses
  Extended: Coffee appears, intense focus
```

### Animation Caching

| Cache Type | Purpose | Eviction Policy |
|------------|---------|-----------------|
| **Base sprites** | Core poses | Never evict |
| **Derived sprites** | Context-specific variations | LRU with TTL |
| **Animation sequences** | Reusable motion | LRU, frequency-weighted |
| **Evolution states** | Personality progression | Session-scoped |

### Freshness Requirement

To keep YA feeling alive:
- Animations should have variations
- Occasional "surprise" variants
- Evolution over time within session
- Memory of past interactions (cross-session, optional)

## Conductor-Avatar Protocol

### Sprite Requests

The meta-agent (via Conductor) can request sprites on-the-fly:

```rust
// Request types
SpriteRequest {
    base: SpriteName,           // "idle", "wave", "thinking"
    mood: Option<Mood>,         // Happy, Sad, Focused, etc.
    size: Option<SizeHint>,     // Blocks or relative
    context: Option<Context>,   // What YA is doing
    evolution: Option<u8>,      // 0-100 progression level
}

// Response
SpriteResponse {
    blocks: Vec<Block>,
    dimensions: (u16, u16),
    anchor: AnchorPoint,        // Where to position relative to
    cache_key: Option<String>,  // For caching
    ttl: Option<Duration>,      // Cache lifetime
}
```

### Animation Requests

```rust
AnimationRequest {
    name: String,               // "celebrate", "work", "rest"
    duration: Option<Duration>, // How long to play
    loop_behavior: LoopBehavior,// Once, Loop, PingPong
    interruptible: bool,        // Can be stopped mid-play
    evolution_context: Option<EvolutionContext>,
}

AnimationResponse {
    frames: Vec<SpriteResponse>,
    timing: Vec<Duration>,      // Per-frame timing
    cache_key: Option<String>,
    allows_evolution: bool,     // Can be built upon
}
```

### Dynamic Generation

The meta-agent may request sprites that don't exist yet:

```rust
GenerateSpriteRequest {
    description: String,        // "yollayah wearing a party hat"
    base_sprite: Option<SpriteName>,
    constraints: SpriteConstraints,
}
```

This enables:
- Contextual accessories (glasses, coffee, tools)
- Emotional states not in base set
- User-specific customizations
- Seasonal/event variations

## Surface Implementation Guidelines

### Required Capabilities

All surfaces MUST support:
1. Block-based rendering at arbitrary positions
2. Partial sprite display (clipping)
3. Z-index layering (avatar over/under content)
4. Animation playback with timing
5. Sprite caching with eviction

### Optional Capabilities

Surfaces MAY support:
1. Transparency/alpha blending
2. Smooth transitions between sprites
3. Custom color palette mapping
4. Animation interpolation
5. Sound synchronization

### TUI-Specific Notes

For terminal surfaces:
- Block = 1 terminal cell
- Colors limited to terminal palette (256 or true color)
- Transparency via background matching
- Animation timing via frame delays
- Character selection based on font support

### HD Surface Notes

For graphical surfaces:
- Block = Scaled character cell (preserve blockiness!)
- DO NOT smooth or anti-alias blocks
- Maintain sharp pixel boundaries
- Scale uniformly (no fractional blocks)
- Optional: Sub-block detail using Braille patterns

## Examples

### Minimal Presence (1-2 blocks)

```
Status bar: [Ready ◆]     <- Single block indicator
Corner:     ▐░            <- Peeking presence
```

### Small Avatar (3x3 blocks)

```
 ▄█▄
█o o█
 ▀▀▀
```

### Medium Avatar (6x8 blocks)

```
  ▄██▄
 █o  o█
 █ ▄▄ █
  █  █
  █  █
 ▄█  █▄
█      █
▀▀▀  ▀▀▀
```

### Full Avatar with Expression

```
    ▄████▄
   █o    o█
   █  ▄▄  █     <- Smiling
    █▀▀▀▀█
     █  █
    ▄█  █▄
   █ ▀▀▀▀ █
   █      █
```

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-01-02 | Initial specification |

---

**See Also**:
- `TODO-avatar-animation-system.md` - Implementation tracking
- `conductor/core/src/avatar.rs` - Current avatar state machine
- `tui/src/avatar/` - TUI sprite implementations
