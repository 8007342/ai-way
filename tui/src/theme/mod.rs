//! Theme and Colors
//!
//! Yollayah's signature color palette - designed for blocky pixel art.
//!
//! The axolotl palette uses soft pinks for the body, coral for gills,
//! and subtle highlights/shadows for depth.

use ratatui::style::Color;

// ============================================================================
// Yollayah Axolotl Palette
// ============================================================================

/// Body - soft pink (main color)
pub const AXOLOTL_BODY: Color = Color::Rgb(255, 182, 193);       // Light pink

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
