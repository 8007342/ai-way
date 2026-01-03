# DESIGN: Palette Rotation System (Replacing Per-Element Breathing)

**Created**: 2026-01-03
**Author**: UX Expert (responding to user suggestion)
**Status**: PROPOSED

---

## Executive Summary

**User Insight**: "Why don't you merely rotate the palette slowly with a smooth gradient, and let the render do its thing normally."

This is **BRILLIANT**. Instead of calculating sin() for every visible element every frame, we pre-compute a rotating palette and let elements reference palette indices.

---

## Current Approach (EXPENSIVE)

### Per-Element Breathing Calculation

**Every frame, for EVERY visible element**:

```rust
// Called 2,100+ times per second at 60 FPS (35 elements × 60 frames)
pub fn breathing_color(base: Color, bright: Color, cycle_ms: u64, elapsed: Duration) -> Color {
    let progress = (elapsed.as_millis() % cycle_ms as u128) as f32 / cycle_ms as f32;  // Modulo + division
    let wave = (progress * 2.0 * PI).sin() * 0.5 + 0.5;  // ← EXPENSIVE sin()
    interpolate_color(base, bright, wave)  // ← 3× RGB lerp (9 float ops)
}
```

**Cost Breakdown** (60 FPS, 35 breathing elements):
- **2,100 sin() calls/second** (35 elements × 60 frames)
- **6,300 float interpolations/second** (3 RGB channels × 2,100)
- **CPU-bound**: Trigonometry dominates render time

---

## Proposed Approach (EFFICIENT)

### Global Palette Rotation

**Once per update cycle (1-5 seconds)**:

```rust
// Called 1-5 times per second (not per element, not per frame!)
pub fn rotate_palette(&mut self, delta: Duration) {
    self.rotation_time += delta;
    if self.rotation_time >= ROTATION_INTERVAL {
        self.rotation_time = Duration::ZERO;

        // Compute ENTIRE palette once
        let t = calculate_smooth_wave(self.phase);

        // Pre-compute ALL breathing colors in one batch
        self.user_prefix_color = interpolate_color(USER_BASE, USER_BRIGHT, t);
        self.assistant_prefix_color = interpolate_color(ASSISTANT_BASE, ASSISTANT_BRIGHT, t);
        self.input_color = interpolate_color(INPUT_BASE, INPUT_BRIGHT, t);
        self.status_color = interpolate_color(STATUS_BASE, STATUS_BRIGHT, t);
        // ... etc

        self.phase += PHASE_STEP;
    }
}
```

**Cost Breakdown** (1 rotation/second, 8 palette colors):
- **1 sin() call/second** (calculate wave once)
- **24 float interpolations/second** (8 colors × 3 RGB channels)
- **99.95% reduction** in calculations (2,100 → 1 sin/sec)

---

## Performance Comparison

### OLD: Per-Element Breathing

| Metric | Operations/Second | CPU Impact |
|--------|------------------|------------|
| sin() calls | 2,100 | CPU-bound math |
| RGB interpolations | 6,300 | High float overhead |
| **Total per-frame cost** | **35 calculations** | **Every 16ms** |

**Render loop**:
```
Frame 1: 35 sin() + 105 lerp
Frame 2: 35 sin() + 105 lerp
Frame 3: 35 sin() + 105 lerp
... every 16ms (60 FPS)
```

---

### NEW: Palette Rotation

| Metric | Operations/Second | CPU Impact |
|--------|------------------|------------|
| sin() calls | 1 | Negligible |
| RGB interpolations | 24 | Negligible |
| **Total update cost** | **8 calculations** | **Once per second** |

**Rotation cycle**:
```
Second 0: Compute palette (8 colors) → use for next 1000ms
Second 1: Compute palette (8 colors) → use for next 1000ms
Second 2: Compute palette (8 colors) → use for next 1000ms
... once per second
```

**Render loop**:
```
Frame 1-60: Reference palette[USER_PREFIX], palette[ASSISTANT_PREFIX], ... (zero calculation)
Frame 61-120: Reference palette[USER_PREFIX], palette[ASSISTANT_PREFIX], ... (zero calculation)
... every 16ms, but NO per-frame calculations
```

---

## Implementation Design

### 1. Palette Structure

```rust
/// Pre-computed rotating color palette
/// Updated once per rotation interval (default: 1 second)
pub struct BreathingPalette {
    /// Current rotation phase (0.0 to 1.0)
    phase: f32,

    /// Rotation phase step per update
    phase_step: f32,

    /// Time accumulator for rotation timing
    rotation_time: Duration,

    /// Update interval (how often to rotate)
    rotation_interval: Duration,

    // === Pre-computed colors (updated in batch) ===
    pub user_prefix: Color,
    pub assistant_prefix: Color,
    pub streaming_cursor: Color,
    pub input_text: Color,
    pub status_ready: Color,
    pub processing_indicator: Color,
    pub agent_indicator: Color,
    pub latest_message_glow: Color,
}

impl BreathingPalette {
    /// Create new palette with default rotation speed
    pub fn new() -> Self {
        Self::with_rotation_interval(Duration::from_secs(1))
    }

    /// Create palette with custom rotation speed
    pub fn with_rotation_interval(interval: Duration) -> Self {
        let mut palette = Self {
            phase: 0.0,
            phase_step: 0.01,  // Smooth gradient steps
            rotation_time: Duration::ZERO,
            rotation_interval: interval,

            // Initialize with base colors
            user_prefix: USER_PREFIX_BASE,
            assistant_prefix: ASSISTANT_PREFIX_BASE,
            streaming_cursor: STREAMING_BASE,
            input_text: INPUT_BASE,
            status_ready: STATUS_BASE,
            processing_indicator: PROCESSING_BASE,
            agent_indicator: AGENT_BASE,
            latest_message_glow: MSG_GLOW_BASE,
        };

        // Compute initial palette
        palette.rotate(Duration::ZERO);
        palette
    }

    /// Update palette rotation (call once per frame in update())
    pub fn update(&mut self, delta: Duration) {
        self.rotation_time += delta;

        if self.rotation_time >= self.rotation_interval {
            self.rotation_time = Duration::ZERO;

            // Compute smooth wave once
            let wave = smooth_wave(self.phase);

            // Batch-update all palette colors
            self.rotate_all_colors(wave);

            // Advance phase for next rotation
            self.phase = (self.phase + self.phase_step).rem_euclid(1.0);
        }
    }

    /// Pre-compute all breathing colors in one batch
    fn rotate_all_colors(&mut self, wave: f32) {
        self.user_prefix = interpolate_color(USER_PREFIX_BASE, USER_PREFIX_BRIGHT, wave);
        self.assistant_prefix = interpolate_color(ASSISTANT_PREFIX_BASE, ASSISTANT_PREFIX_BRIGHT, wave);
        self.streaming_cursor = interpolate_color(STREAMING_BASE, STREAMING_BRIGHT, wave);
        self.input_text = interpolate_color(INPUT_BASE, INPUT_BRIGHT, wave);
        self.status_ready = interpolate_color(STATUS_BASE, STATUS_BRIGHT, wave);
        self.processing_indicator = interpolate_color(PROCESSING_BASE, PROCESSING_BRIGHT, wave);
        self.agent_indicator = interpolate_color(AGENT_BASE, AGENT_BRIGHT, wave);
        self.latest_message_glow = interpolate_color(MSG_GLOW_BASE, MSG_GLOW_BRIGHT, wave);
    }
}

/// Calculate smooth wave value (0.0 to 1.0) from phase
fn smooth_wave(phase: f32) -> f32 {
    // Sine wave: smooth oscillation
    (phase * 2.0 * std::f32::consts::PI).sin() * 0.5 + 0.5
}
```

---

### 2. Integration with App

#### App Struct Update

```rust
pub struct App {
    // ... existing fields ...

    /// Rotating color palette (updated once per second)
    breathing_palette: BreathingPalette,
}

impl App {
    pub async fn new() -> anyhow::Result<Self> {
        // ... existing init ...

        Ok(Self {
            // ... existing fields ...
            breathing_palette: BreathingPalette::new(),
        })
    }
}
```

#### Update Loop Integration

```rust
fn update(&mut self) {
    let now = Instant::now();
    let delta = now - self.last_frame;
    self.last_frame = now;

    // Update breathing palette (once per second, not per frame)
    self.breathing_palette.update(delta);

    // ... rest of update logic ...
}
```

#### Render Loop Usage

```rust
fn render_conversation(&mut self) {
    // ... build lines ...

    for line_meta in visible_lines {
        // Reference pre-computed palette colors (ZERO calculation)
        let prefix_color = match role {
            DisplayRole::User => self.breathing_palette.user_prefix,
            DisplayRole::Assistant => {
                if line_meta.is_streaming {
                    self.breathing_palette.streaming_cursor
                } else {
                    self.breathing_palette.assistant_prefix
                }
            }
            DisplayRole::System => Color::DarkGray,
        };

        buf.set_string(x, y, &prefix_str, Style::default().fg(prefix_color));
    }
}

fn render_input(&mut self) {
    // Reference palette color directly (ZERO calculation)
    let input_style = Style::default().fg(self.breathing_palette.input_text);
    buf.set_string(x, y, line, input_style);
}

fn render_status(&mut self) {
    // Reference palette colors directly (ZERO calculation)
    if is_processing {
        buf.set_string(x, y, "⚡", Style::default().fg(self.breathing_palette.processing_indicator));
    }

    let status_style = Style::default().fg(self.breathing_palette.status_ready);
    buf.set_string(x, y, state_str, status_style);
}
```

---

### 3. Rotation Timing Recommendations

#### Option A: 1 Second (Recommended)

```rust
BreathingPalette::new()  // Default: 1 second
```

**Pros**:
- Smooth, noticeable animation
- Only 24 float ops/second
- Negligible CPU overhead

**Cons**:
- Slightly more frequent updates than needed

---

#### Option B: 5 Seconds (Ultra-Efficient)

```rust
BreathingPalette::with_rotation_interval(Duration::from_secs(5))
```

**Pros**:
- EXTREMELY efficient (only 4.8 float ops/second)
- Still smooth due to smooth_wave interpolation
- Best for low-power systems

**Cons**:
- Animation slower (may feel sluggish)

---

#### Option C: Frame-Locked (Maximum Smoothness)

```rust
BreathingPalette::with_rotation_interval(Duration::from_millis(100))  // Every 10 frames at 10 FPS
```

**Pros**:
- Smoother gradients (updates every 10 frames)
- Still only 80 float ops/second (vs 6,300 with old approach)

**Cons**:
- More frequent updates (though still 98.7% reduction)

---

### 4. Smooth Gradient Generation

```rust
/// Different wave functions for varied aesthetic
pub enum WaveFunction {
    Sine,       // Smooth sine wave (default)
    Cosine,     // Phase-shifted sine
    Triangle,   // Linear interpolation
    EaseInOut,  // Cubic easing
}

impl BreathingPalette {
    /// Set wave function for different breathing styles
    pub fn set_wave_function(&mut self, func: WaveFunction) {
        self.wave_function = func;
    }
}

fn smooth_wave(phase: f32, func: WaveFunction) -> f32 {
    match func {
        WaveFunction::Sine => {
            (phase * 2.0 * PI).sin() * 0.5 + 0.5
        }
        WaveFunction::Cosine => {
            (phase * 2.0 * PI).cos() * 0.5 + 0.5
        }
        WaveFunction::Triangle => {
            // Linear up/down
            if phase < 0.5 {
                phase * 2.0
            } else {
                2.0 - (phase * 2.0)
            }
        }
        WaveFunction::EaseInOut => {
            // Cubic easing
            let t = phase * 2.0;
            if t < 1.0 {
                t * t * t / 2.0
            } else {
                let t = t - 2.0;
                (t * t * t + 2.0) / 2.0
            }
        }
    }
}
```

---

## Visual Quality Comparison

### OLD: Per-Element Breathing

**Every element breathes independently**:
- User prefix: 4-second cycle
- Assistant prefix: 3.5-second cycle
- Input: 3-second cycle
- Status: 3-second cycle
- Result: Chaotic, uncoordinated animation

**Visual Quality**: ★★☆☆☆ (Feels disjointed)

---

### NEW: Palette Rotation

**All elements breathe in sync**:
- User prefix: References palette (synchronized)
- Assistant prefix: References palette (synchronized)
- Input: References palette (synchronized)
- Status: References palette (synchronized)
- Result: Unified, organic "breathing" effect

**Visual Quality**: ★★★★★ (Harmonious, purposeful)

---

## Additional Benefits

### 1. Easier Customization

```rust
// User can tune breathing speed globally
palette.set_rotation_interval(Duration::from_secs(3));

// Or disable entirely
palette.set_rotation_interval(Duration::MAX);  // Effectively static
```

---

### 2. State-Aware Breathing

```rust
impl BreathingPalette {
    /// Speed up breathing when processing
    pub fn set_thinking_mode(&mut self, thinking: bool) {
        self.rotation_interval = if thinking {
            Duration::from_millis(500)  // Faster breathing when working
        } else {
            Duration::from_secs(3)      // Slower when idle
        };
    }
}
```

---

### 3. Theme-Aware Palettes

```rust
pub enum BreathingTheme {
    Calm,       // Gentle, slow breathing
    Energetic,  // Fast, vibrant breathing
    Minimal,    // Barely visible
    Off,        // No breathing (static colors)
}

impl BreathingPalette {
    pub fn from_theme(theme: BreathingTheme) -> Self {
        match theme {
            BreathingTheme::Calm => Self::with_rotation_interval(Duration::from_secs(5)),
            BreathingTheme::Energetic => Self::with_rotation_interval(Duration::from_millis(800)),
            BreathingTheme::Minimal => Self::with_minimal_amplitude(),
            BreathingTheme::Off => Self::static_colors(),
        }
    }
}
```

---

## Migration Path

### Step 1: Add BreathingPalette to theme/mod.rs

```rust
// Add to tui/src/theme/mod.rs
pub struct BreathingPalette { ... }

impl BreathingPalette { ... }
```

---

### Step 2: Add palette field to App

```rust
// In tui/src/app.rs
pub struct App {
    breathing_palette: BreathingPalette,
    // ... existing fields ...
}
```

---

### Step 3: Update palette in update()

```rust
fn update(&mut self) {
    // ... existing update logic ...

    self.breathing_palette.update(delta);
}
```

---

### Step 4: Replace per-element breathing_color() calls

```rust
// OLD:
let color = breathing_color(BASE, BRIGHT, CYCLE_MS, elapsed);

// NEW:
let color = self.breathing_palette.user_prefix;
```

---

### Step 5: Remove old breathing functions

```rust
// DELETE from theme/mod.rs:
// - breathing_color()
// - BREATHING_*_BASE constants
// - BREATHING_*_BRIGHT constants
// - BREATHING_*_CYCLE_MS constants
```

---

## Performance Impact Projection

### CPU Reduction

| Operation | OLD (per-element) | NEW (palette) | Reduction |
|-----------|------------------|---------------|-----------|
| sin() calls/sec | 2,100 | 1 | **99.95%** |
| RGB interpolations/sec | 6,300 | 24 | **99.62%** |
| Float ops/sec | 12,600 | 48 | **99.62%** |

---

### Memory Impact

**OLD**: Zero state (calculate on-demand)
**NEW**: 32 bytes (BreathingPalette struct)

**Trade-off**: 32 bytes RAM for 99.95% CPU reduction = **EXCELLENT**

---

## Code Examples

### Example: Render Conversation

```rust
// OLD (2,100 sin()/sec):
fn render_conversation(&mut self) {
    let elapsed = self.start_time.elapsed();  // ← Track time

    for line in visible_lines {
        let color = breathing_color(  // ← Expensive calculation
            BREATHING_USER_PREFIX_BASE,
            BREATHING_USER_PREFIX_BRIGHT,
            BREATHING_USER_PREFIX_CYCLE_MS,
            elapsed,
        );
        buf.set_string(x, y, line, Style::default().fg(color));
    }
}

// NEW (1 sin()/sec):
fn render_conversation(&mut self) {
    for line in visible_lines {
        let color = self.breathing_palette.user_prefix;  // ← Simple lookup
        buf.set_string(x, y, line, Style::default().fg(color));
    }
}
```

---

### Example: Render Input

```rust
// OLD:
fn render_input(&mut self) {
    let elapsed = self.start_time.elapsed();
    let input_color = breathing_color(
        BREATHING_INPUT_BASE,
        BREATHING_INPUT_BRIGHT,
        BREATHING_INPUT_CYCLE_MS,
        elapsed,
    );
    buf.set_string(x, y, line, Style::default().fg(input_color));
}

// NEW:
fn render_input(&mut self) {
    let input_color = self.breathing_palette.input_text;
    buf.set_string(x, y, line, Style::default().fg(input_color));
}
```

---

### Example: Render Status

```rust
// OLD:
fn render_status(&mut self) {
    let elapsed = self.start_time.elapsed();

    if is_processing {
        let processing_color = breathing_color(
            BREATHING_PROCESSING_BASE,
            BREATHING_PROCESSING_BRIGHT,
            BREATHING_PROCESSING_CYCLE_MS,
            elapsed,
        );
        buf.set_string(x, y, "⚡", Style::default().fg(processing_color));
    }

    let status_color = breathing_color(
        BREATHING_STATUS_BASE,
        BREATHING_STATUS_BRIGHT,
        BREATHING_STATUS_CYCLE_MS,
        elapsed,
    );
    buf.set_string(x, y, state_str, Style::default().fg(status_color));
}

// NEW:
fn render_status(&mut self) {
    if is_processing {
        buf.set_string(x, y, "⚡", Style::default().fg(self.breathing_palette.processing_indicator));
    }

    buf.set_string(x, y, state_str, Style::default().fg(self.breathing_palette.status_ready));
}
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_rotation() {
        let mut palette = BreathingPalette::new();
        let initial = palette.user_prefix;

        // Update for 500ms (half rotation at 1s interval)
        palette.update(Duration::from_millis(500));

        let halfway = palette.user_prefix;

        // Color should have changed
        assert_ne!(initial, halfway);
    }

    #[test]
    fn test_palette_performance() {
        let mut palette = BreathingPalette::new();

        let start = Instant::now();

        // Simulate 60 seconds at 10 FPS
        for _ in 0..600 {
            palette.update(Duration::from_millis(100));
        }

        let elapsed = start.elapsed();

        // Should complete in < 1ms total
        assert!(elapsed < Duration::from_millis(1));
    }
}
```

---

### Integration Test

```rust
#[tokio::test]
async fn test_breathing_palette_vs_static_cpu() {
    // Measure static colors (baseline)
    let cpu_static = measure_cpu_with_static_colors().await;

    // Measure palette rotation (should be negligible difference)
    let cpu_palette = measure_cpu_with_palette_rotation().await;

    // CPU should be nearly identical (< 0.1% difference)
    assert!((cpu_palette - cpu_static).abs() < 0.1);
}
```

---

## Recommendation

**IMPLEMENT PALETTE ROTATION** as the superior alternative to per-element breathing:

1. ✅ **99.95% reduction** in sin() calculations
2. ✅ **99.62% reduction** in float operations
3. ✅ **Better visual quality** (synchronized breathing)
4. ✅ **More flexible** (global tuning, state-aware speeds)
5. ✅ **Trivial memory cost** (32 bytes)

**User's insight is exactly right**: Rotate the palette, not the elements.

---

## Open Questions

1. **Rotation interval**: 1 second (smooth) or 5 seconds (ultra-efficient)?
2. **Wave function**: Sine (smooth) or Ease-In-Out (organic)?
3. **State-aware breathing**: Should breathing speed up during processing?
4. **Enable by default**: Or make it opt-in with YOLLAYAH_BREATHING_ENABLED=1?

---

**Next Step**: Prototype BreathingPalette struct in theme/mod.rs and integrate with app.rs.
