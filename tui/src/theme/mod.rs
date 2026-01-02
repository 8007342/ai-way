//! Theme and Colors
//!
//! Yollayah's signature color palette - designed for blocky pixel art.
//!
//! The axolotl palette uses soft pinks for the body, coral for gills,
//! and subtle highlights/shadows for depth.
//!
//! # Breathing Effects
//!
//! The UI includes subtle "breathing" color animations to make the
//! interface feel alive. These are gentle sine-wave color transitions
//! that pulse at different speeds for different UI elements.

use ratatui::style::Color;
use std::time::Duration;

// ============================================================================
// Yollayah Axolotl Palette
// ============================================================================

/// Body - soft pink (main color)
pub const AXOLOTL_BODY: Color = Color::Rgb(255, 182, 193); // Light pink

/// Body shadow - slightly darker pink for depth
pub const AXOLOTL_BODY_SHADOW: Color = Color::Rgb(219, 148, 160);

/// Body highlight - lighter for shine
pub const AXOLOTL_BODY_HIGHLIGHT: Color = Color::Rgb(255, 218, 224);

/// Gills - coral/salmon red (the feathery bits!)
pub const AXOLOTL_GILLS: Color = Color::Rgb(255, 127, 127);

/// Gills highlight
pub const AXOLOTL_GILLS_HIGHLIGHT: Color = Color::Rgb(255, 160, 160);

/// Eyes - dark, expressive
pub const AXOLOTL_EYES: Color = Color::Rgb(40, 40, 40);

/// Eye shine - white dot
pub const AXOLOTL_EYE_SHINE: Color = Color::Rgb(255, 255, 255);

/// Smile/mouth
pub const AXOLOTL_MOUTH: Color = Color::Rgb(180, 100, 120);

/// Belly - slightly lighter/warmer
pub const AXOLOTL_BELLY: Color = Color::Rgb(255, 220, 210);

// ============================================================================
// Mood Colors (for expressions/effects)
// ============================================================================

/// Happy glow - warm yellow
pub const MOOD_HAPPY: Color = Color::Rgb(255, 223, 128);

/// Thinking - soft blue
pub const MOOD_THINKING: Color = Color::Rgb(150, 180, 255);

/// Error/confused - muted red
pub const MOOD_ERROR: Color = Color::Rgb(255, 100, 100);

/// Excited - bright coral
pub const MOOD_EXCITED: Color = Color::Rgb(255, 150, 120);

// ============================================================================
// UI Colors
// ============================================================================

/// Yollayah's signature magenta (for text/accents)
pub const YOLLAYAH_MAGENTA: Color = Color::Magenta;

/// User input green
pub const USER_GREEN: Color = Color::Rgb(130, 220, 130);

/// System/dim text
pub const DIM_GRAY: Color = Color::Rgb(100, 100, 100);

/// Error red
pub const ERROR_RED: Color = Color::Rgb(255, 80, 80);

/// Success green
pub const SUCCESS_GREEN: Color = Color::Rgb(120, 230, 120);

/// Water/swimming effect
pub const WATER_BLUE: Color = Color::Rgb(100, 180, 255);

/// Bubble color
pub const BUBBLE: Color = Color::Rgb(200, 230, 255);

// ============================================================================
// Breathing Effect Configuration
// ============================================================================

/// Breathing effect: status bar "Ready" indicator
pub const BREATHING_STATUS_BASE: Color = Color::Rgb(100, 100, 100);
pub const BREATHING_STATUS_BRIGHT: Color = Color::Magenta;
pub const BREATHING_STATUS_CYCLE_MS: u64 = 3000;

/// Breathing effect: input "You: " prefix
pub const BREATHING_INPUT_BASE: Color = Color::Rgb(130, 220, 130);
pub const BREATHING_INPUT_BRIGHT: Color = Color::Rgb(170, 255, 170);
pub const BREATHING_INPUT_CYCLE_MS: u64 = 3000;

/// Breathing effect: Yollayah magenta accents
pub const BREATHING_ACCENT_BASE: Color = Color::Magenta;
pub const BREATHING_ACCENT_BRIGHT: Color = Color::Rgb(255, 150, 255);
pub const BREATHING_ACCENT_CYCLE_MS: u64 = 2500;

/// Breathing effect: active task progress
pub const BREATHING_TASK_BASE: Color = Color::Rgb(100, 100, 100);
pub const BREATHING_TASK_BRIGHT: Color = Color::Rgb(255, 223, 128);
pub const BREATHING_TASK_CYCLE_MS: u64 = 1500;

// ============================================================================
// Scroll Gradient Colors
// ============================================================================

/// Scroll indicator: content above (arrow)
pub const SCROLL_INDICATOR_ABOVE: Color = Color::Rgb(200, 100, 255);

/// Scroll indicator: content below (arrow)
pub const SCROLL_INDICATOR_BELOW: Color = Color::Rgb(200, 100, 255);

/// Scroll fade: darkest (at edge)
pub const SCROLL_FADE_DARK: Color = Color::Rgb(60, 60, 60);

/// Scroll fade: medium
pub const SCROLL_FADE_MEDIUM: Color = Color::Rgb(100, 100, 100);

/// Scroll fade: light (near normal text)
pub const SCROLL_FADE_LIGHT: Color = Color::Rgb(140, 140, 140);

// ============================================================================
// Color Interpolation & Breathing Functions
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

/// Calculate breathing color at a given time
///
/// Uses a sine wave to smoothly oscillate between base and bright colors.
///
/// # Arguments
/// * `base` - Base color (minimum brightness)
/// * `bright` - Bright color (maximum brightness)
/// * `cycle_ms` - Full cycle duration in milliseconds
/// * `elapsed` - Time since breathing started
#[must_use]
pub fn breathing_color(base: Color, bright: Color, cycle_ms: u64, elapsed: Duration) -> Color {
    if cycle_ms == 0 {
        return base;
    }

    // Calculate position in cycle (0.0 to 1.0)
    let progress = (elapsed.as_millis() % cycle_ms as u128) as f32 / cycle_ms as f32;

    // Sine wave: 0.0 -> 0.5, 0.5 -> 1.0, 1.0 -> 0.5 (smooth oscillation)
    let wave = (progress * 2.0 * std::f32::consts::PI).sin() * 0.5 + 0.5;

    interpolate_color(base, bright, wave)
}

/// Calculate gradient fade factor for scroll indicators
///
/// Returns a factor from 0.0 (most faded) to 1.0 (normal) based on
/// position in the viewport and proximity to edge.
///
/// # Arguments
/// * `position` - Line position in viewport (0 = top)
/// * `viewport_height` - Total viewport height
/// * `fade_lines` - Number of lines to fade at each edge
/// * `has_content_above` - Whether there's content above viewport
/// * `has_content_below` - Whether there's content below viewport
#[must_use]
pub fn scroll_fade_factor(
    position: usize,
    viewport_height: usize,
    fade_lines: usize,
    has_content_above: bool,
    has_content_below: bool,
) -> f32 {
    // Check if we're at the top edge
    if has_content_above && position < fade_lines {
        return position as f32 / fade_lines as f32;
    }

    // Check if we're at the bottom edge
    let dist_from_bottom = viewport_height.saturating_sub(position + 1);
    if has_content_below && dist_from_bottom < fade_lines {
        return dist_from_bottom as f32 / fade_lines as f32;
    }

    // Not at an edge, normal brightness
    1.0
}

/// Get fade color based on fade factor
#[must_use]
pub fn scroll_fade_color(fade_factor: f32) -> Color {
    if fade_factor <= 0.33 {
        SCROLL_FADE_DARK
    } else if fade_factor <= 0.66 {
        SCROLL_FADE_MEDIUM
    } else if fade_factor < 1.0 {
        SCROLL_FADE_LIGHT
    } else {
        Color::Reset // Normal text color
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_color_endpoints() {
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
    }

    #[test]
    fn test_interpolate_color_midpoint() {
        let black = Color::Rgb(0, 0, 0);
        let white = Color::Rgb(255, 255, 255);

        // At t=0.5, should be gray
        if let Color::Rgb(r, g, b) = interpolate_color(black, white, 0.5) {
            assert!(r >= 127 && r <= 128);
            assert!(g >= 127 && g <= 128);
            assert!(b >= 127 && b <= 128);
        } else {
            panic!("Expected RGB color");
        }
    }

    #[test]
    fn test_breathing_color_cycle() {
        let base = Color::Rgb(100, 100, 100);
        let bright = Color::Rgb(200, 200, 200);
        let cycle_ms = 1000;

        // At start (0ms), should be at midpoint (sin(0) = 0, wave = 0.5)
        let c0 = breathing_color(base, bright, cycle_ms, Duration::from_millis(0));

        // At quarter cycle (250ms), should be brighter (sin(π/2) = 1, wave = 1.0)
        let c250 = breathing_color(base, bright, cycle_ms, Duration::from_millis(250));

        // At half cycle (500ms), back to midpoint (sin(π) = 0, wave = 0.5)
        let c500 = breathing_color(base, bright, cycle_ms, Duration::from_millis(500));

        // c250 should be the brightest
        if let (Color::Rgb(r0, _, _), Color::Rgb(r250, _, _), Color::Rgb(r500, _, _)) =
            (c0, c250, c500)
        {
            assert!(r250 >= r0);
            assert!(r250 >= r500);
        }
    }

    #[test]
    fn test_scroll_fade_factor() {
        // No content above/below: always 1.0
        assert!((scroll_fade_factor(0, 10, 2, false, false) - 1.0).abs() < f32::EPSILON);

        // Content above, at line 0: fade factor 0.0
        assert!((scroll_fade_factor(0, 10, 2, true, false) - 0.0).abs() < f32::EPSILON);

        // Content above, at line 1: fade factor 0.5
        assert!((scroll_fade_factor(1, 10, 2, true, false) - 0.5).abs() < f32::EPSILON);

        // Content above, at line 2: normal (1.0)
        assert!((scroll_fade_factor(2, 10, 2, true, false) - 1.0).abs() < f32::EPSILON);

        // Content below, at bottom line (9): fade factor 0.0
        assert!((scroll_fade_factor(9, 10, 2, false, true) - 0.0).abs() < f32::EPSILON);
    }
}
