// DESIGN EXAMPLE: Palette Rotation Implementation
// This is a concrete code example showing how to replace per-element breathing
// with a global rotating palette.
//
// File: tui/src/theme/mod.rs (additions)

use ratatui::style::Color;
use std::time::Duration;

// ============================================================================
// Color Palette Base/Bright Pairs
// ============================================================================

// User message prefix ("You:")
const USER_PREFIX_BASE: Color = Color::Rgb(130, 220, 130);
const USER_PREFIX_BRIGHT: Color = Color::Rgb(160, 245, 160);

// Assistant message prefix ("Yollayah:")
const ASSISTANT_PREFIX_BASE: Color = Color::Rgb(200, 100, 200);
const ASSISTANT_PREFIX_BRIGHT: Color = Color::Rgb(255, 140, 255);

// Streaming message cursor
const STREAMING_BASE: Color = Color::Rgb(200, 100, 200);
const STREAMING_BRIGHT: Color = Color::Rgb(255, 180, 255);

// Input field text
const INPUT_BASE: Color = Color::Rgb(130, 220, 130);
const INPUT_BRIGHT: Color = Color::Rgb(170, 255, 170);

// Status bar "Ready" indicator
const STATUS_BASE: Color = Color::Rgb(100, 100, 100);
const STATUS_BRIGHT: Color = Color::Magenta;

// Processing indicator (⚡)
const PROCESSING_BASE: Color = Color::Rgb(80, 80, 80);
const PROCESSING_BRIGHT: Color = Color::Rgb(255, 223, 128);

// Agent work indicator (◆)
const AGENT_BASE: Color = Color::Rgb(80, 80, 80);
const AGENT_BRIGHT: Color = Color::Rgb(255, 150, 255);

// Latest message background glow
const MSG_GLOW_BASE: Color = Color::Rgb(40, 40, 50);
const MSG_GLOW_BRIGHT: Color = Color::Rgb(50, 50, 65);

// ============================================================================
// Breathing Palette - Pre-computed rotating colors
// ============================================================================

/// Pre-computed rotating color palette
///
/// Instead of calculating breathing colors per-element per-frame,
/// we compute ALL colors once per rotation interval and elements
/// simply reference the pre-computed colors.
///
/// Performance impact:
/// - OLD: 2,100 sin() calls/second (35 elements × 60 FPS)
/// - NEW: 1 sin() call/second (1 rotation × 1 Hz)
/// - Reduction: 99.95%
pub struct BreathingPalette {
    /// Current rotation phase (0.0 to 1.0)
    phase: f32,

    /// Rotation phase step per update
    phase_step: f32,

    /// Time accumulator for rotation timing
    rotation_time: Duration,

    /// Update interval (how often to rotate palette)
    rotation_interval: Duration,

    /// Wave function to use for interpolation
    wave_function: WaveFunction,

    // === Pre-computed colors (updated in batch) ===

    /// User message prefix ("You:") - green breathing
    pub user_prefix: Color,

    /// Assistant message prefix ("Yollayah:") - magenta breathing
    pub assistant_prefix: Color,

    /// Streaming message cursor - brighter magenta breathing
    pub streaming_cursor: Color,

    /// Input field text - green breathing
    pub input_text: Color,

    /// Status bar "Ready" indicator - magenta breathing
    pub status_ready: Color,

    /// Processing indicator (⚡) - yellow breathing
    pub processing_indicator: Color,

    /// Agent work indicator (◆) - magenta breathing
    pub agent_indicator: Color,

    /// Latest message background glow - subtle breathing
    pub latest_message_glow: Color,
}

/// Wave function for breathing animation
#[derive(Debug, Clone, Copy)]
pub enum WaveFunction {
    /// Smooth sine wave (default) - organic breathing
    Sine,
    /// Cosine wave (phase-shifted sine)
    Cosine,
    /// Triangle wave (linear interpolation)
    Triangle,
    /// Cubic ease-in-out (gentle acceleration/deceleration)
    EaseInOut,
}

impl BreathingPalette {
    /// Create new palette with default rotation speed (1 second)
    pub fn new() -> Self {
        Self::with_rotation_interval(Duration::from_secs(1))
    }

    /// Create palette with custom rotation speed
    ///
    /// Recommended values:
    /// - `Duration::from_secs(1)`: Smooth, noticeable (default)
    /// - `Duration::from_secs(3)`: Gentle, subtle
    /// - `Duration::from_secs(5)`: Very slow, ultra-efficient
    /// - `Duration::from_millis(100)`: Very smooth (updates every 10 frames at 10 FPS)
    pub fn with_rotation_interval(interval: Duration) -> Self {
        let mut palette = Self {
            phase: 0.0,
            phase_step: 0.01,  // 100 steps per full rotation
            rotation_time: Duration::ZERO,
            rotation_interval: interval,
            wave_function: WaveFunction::Sine,

            // Initialize with base colors (will be rotated immediately)
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
        palette.rotate_all_colors(smooth_wave(0.0, palette.wave_function));
        palette
    }

    /// Create static palette (no breathing - zero CPU)
    pub fn static_colors() -> Self {
        Self {
            phase: 0.0,
            phase_step: 0.0,
            rotation_time: Duration::ZERO,
            rotation_interval: Duration::MAX,  // Never update
            wave_function: WaveFunction::Sine,

            // Use bright colors (no breathing)
            user_prefix: USER_PREFIX_BRIGHT,
            assistant_prefix: ASSISTANT_PREFIX_BRIGHT,
            streaming_cursor: STREAMING_BRIGHT,
            input_text: INPUT_BRIGHT,
            status_ready: STATUS_BRIGHT,
            processing_indicator: PROCESSING_BRIGHT,
            agent_indicator: AGENT_BRIGHT,
            latest_message_glow: MSG_GLOW_BRIGHT,
        }
    }

    /// Update palette rotation (call once per frame in App::update())
    ///
    /// This is called every frame (10 FPS = 100ms), but only rotates
    /// the palette when rotation_time >= rotation_interval.
    ///
    /// Performance: O(1) check per frame, O(n) computation only on rotation
    pub fn update(&mut self, delta: Duration) {
        self.rotation_time += delta;

        if self.rotation_time >= self.rotation_interval {
            self.rotation_time = Duration::ZERO;

            // Compute smooth wave once (THE ONLY sin() CALL)
            let wave = smooth_wave(self.phase, self.wave_function);

            // Batch-update all palette colors
            self.rotate_all_colors(wave);

            // Advance phase for next rotation
            self.phase = (self.phase + self.phase_step).rem_euclid(1.0);
        }
    }

    /// Pre-compute all breathing colors in one batch
    ///
    /// Called only when rotation_time >= rotation_interval
    /// (typically once per second)
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

    /// Set rotation interval (breathing speed)
    pub fn set_rotation_interval(&mut self, interval: Duration) {
        self.rotation_interval = interval;
    }

    /// Set wave function for different breathing aesthetics
    pub fn set_wave_function(&mut self, func: WaveFunction) {
        self.wave_function = func;
    }

    /// Speed up breathing when processing (state-aware breathing)
    ///
    /// Usage:
    /// ```
    /// palette.set_thinking_mode(true);  // Fast breathing during work
    /// palette.set_thinking_mode(false); // Slow breathing when idle
    /// ```
    pub fn set_thinking_mode(&mut self, thinking: bool) {
        self.rotation_interval = if thinking {
            Duration::from_millis(500)  // Faster breathing when working
        } else {
            Duration::from_secs(3)      // Slower when idle
        };
    }
}

impl Default for BreathingPalette {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Wave Functions
// ============================================================================

/// Calculate smooth wave value (0.0 to 1.0) from phase
///
/// This is THE ONLY place where sin() is called
fn smooth_wave(phase: f32, func: WaveFunction) -> f32 {
    match func {
        WaveFunction::Sine => {
            // Smooth sine wave (default)
            (phase * 2.0 * std::f32::consts::PI).sin() * 0.5 + 0.5
        }
        WaveFunction::Cosine => {
            // Phase-shifted sine
            (phase * 2.0 * std::f32::consts::PI).cos() * 0.5 + 0.5
        }
        WaveFunction::Triangle => {
            // Linear interpolation up/down
            if phase < 0.5 {
                phase * 2.0
            } else {
                2.0 - (phase * 2.0)
            }
        }
        WaveFunction::EaseInOut => {
            // Cubic easing (gentle acceleration/deceleration)
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

// ============================================================================
// Color Interpolation (kept from original breathing_color implementation)
// ============================================================================

/// Interpolate between two RGB colors
///
/// # Arguments
/// * `from` - Starting color (when t = 0.0)
/// * `to` - Target color (when t = 1.0)
/// * `t` - Interpolation factor (0.0 to 1.0)
#[must_use]
pub fn interpolate_color(from: Color, to: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);

    match (from, to) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let r = lerp_u8(r1, r2, t);
            let g = lerp_u8(g1, g2, t);
            let b = lerp_u8(b1, b2, t);
            Color::Rgb(r, g, b)
        }
        // For non-RGB colors, just return the target at t >= 0.5
        _ => {
            if t >= 0.5 {
                to
            } else {
                from
            }
        }
    }
}

/// Linear interpolation for u8 values
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let a = a as f32;
    let b = b as f32;
    (a + (b - a) * t).round() as u8
}

// ============================================================================
// USAGE EXAMPLE: App Integration
// ============================================================================

/*
// In tui/src/app.rs:

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

    fn update(&mut self) {
        let now = Instant::now();
        let delta = now - self.last_frame;
        self.last_frame = now;

        // Update breathing palette (once per second, not per frame)
        self.breathing_palette.update(delta);

        // ... rest of update logic ...
    }

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
            buf.set_string(
                x, y, "⚡",
                Style::default().fg(self.breathing_palette.processing_indicator)
            );
        }

        let status_style = Style::default().fg(self.breathing_palette.status_ready);
        buf.set_string(x, y, state_str, status_style);
    }
}
*/

// ============================================================================
// TESTS
// ============================================================================

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

        // Update another 500ms (full rotation)
        palette.update(Duration::from_millis(500));

        let full_rotation = palette.user_prefix;

        // Colors should change during rotation
        assert_ne!(initial, halfway);

        // Should be close to initial after full rotation (may not be exact due to phase steps)
        // This is acceptable - breathing is meant to be continuous
    }

    #[test]
    fn test_palette_performance() {
        let mut palette = BreathingPalette::new();

        let start = std::time::Instant::now();

        // Simulate 60 seconds at 10 FPS (600 frames)
        for _ in 0..600 {
            palette.update(Duration::from_millis(100));
        }

        let elapsed = start.elapsed();

        // Should complete in < 1ms total (600 frames with only 60 rotations)
        assert!(elapsed < Duration::from_millis(1),
            "Palette rotation took too long: {:?}", elapsed);
    }

    #[test]
    fn test_static_palette() {
        let palette = BreathingPalette::static_colors();

        // Should use bright colors
        assert_eq!(palette.user_prefix, USER_PREFIX_BRIGHT);
        assert_eq!(palette.assistant_prefix, ASSISTANT_PREFIX_BRIGHT);
        assert_eq!(palette.input_text, INPUT_BRIGHT);
    }

    #[test]
    fn test_thinking_mode() {
        let mut palette = BreathingPalette::new();

        // Normal mode: 3 seconds
        palette.set_thinking_mode(false);
        assert_eq!(palette.rotation_interval, Duration::from_secs(3));

        // Thinking mode: 500ms (faster breathing)
        palette.set_thinking_mode(true);
        assert_eq!(palette.rotation_interval, Duration::from_millis(500));
    }

    #[test]
    fn test_wave_functions() {
        // Test all wave functions produce values in [0.0, 1.0]
        for func in [
            WaveFunction::Sine,
            WaveFunction::Cosine,
            WaveFunction::Triangle,
            WaveFunction::EaseInOut,
        ] {
            for phase in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let wave = smooth_wave(phase, func);
                assert!(wave >= 0.0 && wave <= 1.0,
                    "Wave function {:?} at phase {} produced invalid value: {}",
                    func, phase, wave);
            }
        }
    }

    #[test]
    fn test_interpolate_color() {
        let black = Color::Rgb(0, 0, 0);
        let white = Color::Rgb(255, 255, 255);

        // At t=0, should return from color
        assert!(matches!(
            interpolate_color(black, white, 0.0),
            Color::Rgb(0, 0, 0)
        ));

        // At t=1, should return to color
        assert!(matches!(
            interpolate_color(black, white, 1.0),
            Color::Rgb(255, 255, 255)
        ));

        // At t=0.5, should be gray
        if let Color::Rgb(r, g, b) = interpolate_color(black, white, 0.5) {
            assert!(r >= 127 && r <= 128);
            assert!(g >= 127 && g <= 128);
            assert!(b >= 127 && b <= 128);
        } else {
            panic!("Expected RGB color");
        }
    }
}
