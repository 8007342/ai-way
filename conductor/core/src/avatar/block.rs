//! Block-based avatar rendering primitives
//!
//! This module defines the atomic rendering unit for the Yollayah avatar system.
//! A `Block` represents a single character-cell that can be rendered on any surface
//! (TUI, `WebUI`, mobile, etc.) while maintaining the blocky aesthetic.
//!
//! # Design Philosophy
//!
//! The "block" is the fundamental rendering unit - not a limitation of low-resolution
//! displays, but a deliberate aesthetic choice. High-definition surfaces should scale
//! blocks uniformly and maintain sharp pixel boundaries.
//!
//! # Serialization
//!
//! All types serialize to JSON for protocol transmission between Conductor and surfaces.

use serde::{Deserialize, Serialize};

/// Surface-agnostic RGBA color
///
/// A simple color representation that can be mapped to any rendering target:
/// - Terminal: Nearest 256-color or true color
/// - Web: CSS `rgba()` or hex
/// - Native: Platform color types
///
/// # Examples
///
/// ```
/// use conductor_core::avatar::block::Color;
///
/// // Create an opaque red
/// let red = Color::rgb(255, 0, 0);
/// assert_eq!(red.a, 255);
///
/// // Create a semi-transparent blue
/// let blue = Color::rgba(0, 0, 255, 128);
/// assert_eq!(blue.a, 128);
///
/// // Fully transparent
/// let clear = Color::transparent();
/// assert_eq!(clear.a, 0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Color {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
    /// Alpha component (0=transparent, 255=opaque)
    pub a: u8,
}

impl Color {
    /// Create a fully opaque color from RGB components
    ///
    /// # Examples
    ///
    /// ```
    /// use conductor_core::avatar::block::Color;
    ///
    /// let white = Color::rgb(255, 255, 255);
    /// assert_eq!(white.a, 255);
    /// ```
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Create a color with explicit alpha channel
    ///
    /// # Examples
    ///
    /// ```
    /// use conductor_core::avatar::block::Color;
    ///
    /// let half_black = Color::rgba(0, 0, 0, 128);
    /// assert_eq!(half_black.a, 128);
    /// ```
    #[must_use]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create a fully transparent color
    ///
    /// This is the "nothing here" color - used for empty space in sprites.
    ///
    /// # Examples
    ///
    /// ```
    /// use conductor_core::avatar::block::Color;
    ///
    /// let clear = Color::transparent();
    /// assert_eq!(clear.a, 0);
    /// assert!(clear.is_transparent());
    /// ```
    #[must_use]
    pub const fn transparent() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }

    /// Check if this color is fully transparent
    #[must_use]
    pub const fn is_transparent(&self) -> bool {
        self.a == 0
    }

    /// Check if this color is fully opaque
    #[must_use]
    pub const fn is_opaque(&self) -> bool {
        self.a == 255
    }

    /// Convert to a CSS-style hex string (#RRGGBB or #RRGGBBAA)
    ///
    /// # Examples
    ///
    /// ```
    /// use conductor_core::avatar::block::Color;
    ///
    /// let red = Color::rgb(255, 0, 0);
    /// assert_eq!(red.to_hex(), "#ff0000");
    ///
    /// let half_blue = Color::rgba(0, 0, 255, 128);
    /// assert_eq!(half_blue.to_hex(), "#0000ff80");
    /// ```
    #[must_use]
    pub fn to_hex(&self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }

    /// Blend this color over another (simple alpha compositing)
    ///
    /// Uses the "over" operator: result = src + dst * (1 - `src_alpha`)
    ///
    /// # Examples
    ///
    /// ```
    /// use conductor_core::avatar::block::Color;
    ///
    /// let overlay = Color::rgba(255, 0, 0, 128);  // 50% red
    /// let background = Color::rgb(0, 0, 255);     // solid blue
    /// let blended = overlay.blend_over(background);
    /// // Result is a purple-ish color
    /// ```
    #[must_use]
    pub fn blend_over(&self, background: Color) -> Color {
        if self.a == 255 {
            return *self;
        }
        if self.a == 0 {
            return background;
        }

        let src_a = f32::from(self.a) / 255.0;
        let dst_a = f32::from(background.a) / 255.0;
        let out_a = src_a + dst_a * (1.0 - src_a);

        if out_a == 0.0 {
            return Color::transparent();
        }

        let blend = |src: u8, dst: u8| -> u8 {
            let s = f32::from(src) / 255.0;
            let d = f32::from(dst) / 255.0;
            let result = (s * src_a + d * dst_a * (1.0 - src_a)) / out_a;
            (result * 255.0).round() as u8
        };

        Color {
            r: blend(self.r, background.r),
            g: blend(self.g, background.g),
            b: blend(self.b, background.b),
            a: (out_a * 255.0).round() as u8,
        }
    }

    /// Linearly interpolate between two colors
    ///
    /// # Arguments
    ///
    /// * `other` - The target color
    /// * `t` - Interpolation factor (0.0 = self, 1.0 = other)
    ///
    /// # Examples
    ///
    /// ```
    /// use conductor_core::avatar::block::Color;
    ///
    /// let black = Color::rgb(0, 0, 0);
    /// let white = Color::rgb(255, 255, 255);
    /// let gray = black.lerp(white, 0.5);
    /// assert!(gray.r > 120 && gray.r < 135); // Approximately 127
    /// ```
    #[must_use]
    pub fn lerp(&self, other: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let lerp_u8 = |a: u8, b: u8| -> u8 {
            let a = f32::from(a);
            let b = f32::from(b);
            (a + (b - a) * t).round() as u8
        };

        Color {
            r: lerp_u8(self.r, other.r),
            g: lerp_u8(self.g, other.g),
            b: lerp_u8(self.b, other.b),
            a: lerp_u8(self.a, other.a),
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::transparent()
    }
}

/// A single character-cell block - the atomic unit of avatar rendering
///
/// Each block represents one cell in a sprite grid. Blocks can contain:
/// - A Unicode character (for shape/texture)
/// - Foreground and background colors
/// - Transparency for layering
/// - Z-index for depth ordering
///
/// # Block Characters
///
/// Preferred Unicode ranges for blocky aesthetics:
/// - Block elements: U+2580-U+259F (e.g., "fullblock", "halfblock")
/// - Box drawing: U+2500-U+257F
/// - Geometric shapes: U+25A0-U+25FF
/// - Braille patterns: U+2800-U+28FF (for fine detail)
///
/// # Examples
///
/// ```
/// use conductor_core::avatar::block::{Block, Color};
///
/// // A solid pink block (Yollayah's primary color)
/// let pink = Block::solid(Color::rgb(255, 182, 193));
///
/// // An empty/transparent block
/// let empty = Block::empty();
/// assert!(empty.is_empty());
///
/// // A custom block with character
/// let eye = Block {
///     fg: Color::rgb(0, 0, 0),
///     bg: Color::rgb(255, 255, 255),
///     character: 'o',
///     transparency: 0.0,
///     z_index: 1,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// Foreground color (for the character)
    pub fg: Color,
    /// Background color (fills the cell)
    pub bg: Color,
    /// The character to display (space for solid color blocks)
    pub character: char,
    /// Transparency (0.0 = opaque, 1.0 = fully transparent)
    ///
    /// This is separate from color alpha - it controls overall block visibility
    /// for animation effects like fading in/out.
    pub transparency: f32,
    /// Z-index for layering (-128 to 127)
    ///
    /// Higher values render on top. Use for:
    /// - Background elements: negative values
    /// - Main sprite: 0
    /// - Effects/overlays: positive values
    pub z_index: i8,
}

impl Block {
    /// Full block character (solid rectangle)
    pub const FULL_BLOCK: char = '\u{2588}';
    /// Upper half block
    pub const UPPER_HALF: char = '\u{2580}';
    /// Lower half block
    pub const LOWER_HALF: char = '\u{2584}';
    /// Light shade
    pub const LIGHT_SHADE: char = '\u{2591}';
    /// Medium shade
    pub const MEDIUM_SHADE: char = '\u{2592}';
    /// Dark shade
    pub const DARK_SHADE: char = '\u{2593}';

    /// Create a solid block filled with a single color
    ///
    /// Uses the full block character for maximum coverage.
    ///
    /// # Examples
    ///
    /// ```
    /// use conductor_core::avatar::block::{Block, Color};
    ///
    /// let red_block = Block::solid(Color::rgb(255, 0, 0));
    /// assert_eq!(red_block.character, Block::FULL_BLOCK);
    /// assert_eq!(red_block.transparency, 0.0);
    /// ```
    #[must_use]
    pub const fn solid(color: Color) -> Self {
        Self {
            fg: color,
            bg: color,
            character: Self::FULL_BLOCK,
            transparency: 0.0,
            z_index: 0,
        }
    }

    /// Create an empty (fully transparent) block
    ///
    /// This represents "nothing" - the sprite doesn't occupy this cell.
    ///
    /// # Examples
    ///
    /// ```
    /// use conductor_core::avatar::block::Block;
    ///
    /// let empty = Block::empty();
    /// assert!(empty.is_empty());
    /// assert_eq!(empty.transparency, 1.0);
    /// ```
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            fg: Color::transparent(),
            bg: Color::transparent(),
            character: ' ',
            transparency: 1.0,
            z_index: 0,
        }
    }

    /// Create a block with just a character on transparent background
    ///
    /// Useful for text overlays or sparse effects.
    ///
    /// # Examples
    ///
    /// ```
    /// use conductor_core::avatar::block::{Block, Color};
    ///
    /// let eye = Block::character('o', Color::rgb(0, 0, 0));
    /// assert_eq!(eye.character, 'o');
    /// assert!(eye.bg.is_transparent());
    /// ```
    #[must_use]
    pub const fn character(ch: char, fg: Color) -> Self {
        Self {
            fg,
            bg: Color::transparent(),
            character: ch,
            transparency: 0.0,
            z_index: 0,
        }
    }

    /// Create a block with foreground and background colors
    ///
    /// # Examples
    ///
    /// ```
    /// use conductor_core::avatar::block::{Block, Color};
    ///
    /// let block = Block::colored('X', Color::rgb(255, 0, 0), Color::rgb(0, 0, 255));
    /// assert_eq!(block.fg, Color::rgb(255, 0, 0));
    /// assert_eq!(block.bg, Color::rgb(0, 0, 255));
    /// ```
    #[must_use]
    pub const fn colored(character: char, fg: Color, bg: Color) -> Self {
        Self {
            fg,
            bg,
            character,
            transparency: 0.0,
            z_index: 0,
        }
    }

    /// Check if this block is empty (fully transparent)
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.transparency >= 1.0
            || (self.fg.is_transparent() && self.bg.is_transparent())
            || (self.character == ' ' && self.bg.is_transparent())
    }

    /// Create a copy with modified z-index
    #[must_use]
    pub const fn with_z_index(mut self, z_index: i8) -> Self {
        self.z_index = z_index;
        self
    }

    /// Create a copy with modified transparency
    #[must_use]
    pub fn with_transparency(mut self, transparency: f32) -> Self {
        self.transparency = transparency.clamp(0.0, 1.0);
        self
    }

    /// Blend this block over another (for layering)
    ///
    /// Respects both color alpha and block transparency.
    #[must_use]
    pub fn blend_over(&self, background: &Block) -> Block {
        if self.is_empty() {
            return background.clone();
        }
        if background.is_empty() {
            return self.clone();
        }

        // Apply block transparency to colors
        let self_alpha_mult = 1.0 - self.transparency;
        let effective_fg = Color::rgba(
            self.fg.r,
            self.fg.g,
            self.fg.b,
            (f32::from(self.fg.a) * self_alpha_mult) as u8,
        );
        let effective_bg = Color::rgba(
            self.bg.r,
            self.bg.g,
            self.bg.b,
            (f32::from(self.bg.a) * self_alpha_mult) as u8,
        );

        Block {
            fg: effective_fg.blend_over(background.fg),
            bg: effective_bg.blend_over(background.bg),
            character: if self.character != ' ' && self_alpha_mult > 0.5 {
                self.character
            } else if background.character != ' ' {
                background.character
            } else {
                ' '
            },
            transparency: 0.0, // Result is composited
            z_index: self.z_index.max(background.z_index),
        }
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::empty()
    }
}

/// Size hint for sprite requests
///
/// Surfaces can request sprites at different sizes. The avatar system
/// will return the best available representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SizeHint {
    /// Exact block dimensions (width, height)
    Blocks {
        /// Width in blocks
        width: u16,
        /// Height in blocks
        height: u16,
    },
    /// Relative size (small, medium, large)
    Relative(RelativeSize),
    /// Fit within constraints (max width, max height)
    FitWithin {
        /// Maximum width in blocks
        max_width: u16,
        /// Maximum height in blocks
        max_height: u16,
    },
    /// Minimal presence (1-2 blocks)
    Minimal,
    /// Fill available space
    Fill,
}

impl SizeHint {
    /// Create a size hint for exact dimensions
    #[must_use]
    pub const fn exact(width: u16, height: u16) -> Self {
        Self::Blocks { width, height }
    }

    /// Create a size hint to fit within bounds
    #[must_use]
    pub const fn fit(max_width: u16, max_height: u16) -> Self {
        Self::FitWithin {
            max_width,
            max_height,
        }
    }
}

impl Default for SizeHint {
    fn default() -> Self {
        Self::Relative(RelativeSize::Medium)
    }
}

/// Relative size categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum RelativeSize {
    /// Tiny (1-2 blocks) - presence indicator
    Tiny,
    /// Small (3-4 blocks)
    Small,
    /// Medium (6-8 blocks) - default
    #[default]
    Medium,
    /// Large (12+ blocks) - attention grabbing
    Large,
    /// Extra large (fills significant screen space)
    ExtraLarge,
}

/// Anchor point for sprite positioning
///
/// Defines the reference point for placing a sprite on screen.
/// The anchor is the point that gets placed at the specified position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AnchorPoint {
    /// Top-left corner of sprite
    TopLeft,
    /// Top-center of sprite
    TopCenter,
    /// Top-right corner of sprite
    TopRight,
    /// Center-left of sprite
    CenterLeft,
    /// Center of sprite (default)
    #[default]
    Center,
    /// Center-right of sprite
    CenterRight,
    /// Bottom-left corner of sprite
    BottomLeft,
    /// Bottom-center of sprite
    BottomCenter,
    /// Bottom-right corner of sprite
    BottomRight,
}

impl AnchorPoint {
    /// Get the offset multipliers for this anchor point
    ///
    /// Returns (`x_mult`, `y_mult`) where:
    /// - 0.0 = left/top edge
    /// - 0.5 = center
    /// - 1.0 = right/bottom edge
    ///
    /// # Examples
    ///
    /// ```
    /// use conductor_core::avatar::block::AnchorPoint;
    ///
    /// let (x, y) = AnchorPoint::Center.offset_multipliers();
    /// assert_eq!((x, y), (0.5, 0.5));
    ///
    /// let (x, y) = AnchorPoint::TopLeft.offset_multipliers();
    /// assert_eq!((x, y), (0.0, 0.0));
    /// ```
    #[must_use]
    pub const fn offset_multipliers(&self) -> (f32, f32) {
        match self {
            Self::TopLeft => (0.0, 0.0),
            Self::TopCenter => (0.5, 0.0),
            Self::TopRight => (1.0, 0.0),
            Self::CenterLeft => (0.0, 0.5),
            Self::Center => (0.5, 0.5),
            Self::CenterRight => (1.0, 0.5),
            Self::BottomLeft => (0.0, 1.0),
            Self::BottomCenter => (0.5, 1.0),
            Self::BottomRight => (1.0, 1.0),
        }
    }

    /// Calculate the top-left position for a sprite given target position and dimensions
    ///
    /// # Arguments
    ///
    /// * `target_x` - X coordinate where anchor should be placed
    /// * `target_y` - Y coordinate where anchor should be placed
    /// * `width` - Sprite width in blocks
    /// * `height` - Sprite height in blocks
    ///
    /// # Returns
    ///
    /// (`top_left_x`, `top_left_y`) coordinates for rendering
    #[must_use]
    pub fn calculate_top_left(
        &self,
        target_x: i32,
        target_y: i32,
        width: u16,
        height: u16,
    ) -> (i32, i32) {
        let (x_mult, y_mult) = self.offset_multipliers();
        let offset_x = (f32::from(width) * x_mult).round() as i32;
        let offset_y = (f32::from(height) * y_mult).round() as i32;
        (target_x - offset_x, target_y - offset_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Color tests

    #[test]
    fn test_color_rgb() {
        let color = Color::rgb(100, 150, 200);
        assert_eq!(color.r, 100);
        assert_eq!(color.g, 150);
        assert_eq!(color.b, 200);
        assert_eq!(color.a, 255);
        assert!(color.is_opaque());
        assert!(!color.is_transparent());
    }

    #[test]
    fn test_color_rgba() {
        let color = Color::rgba(100, 150, 200, 128);
        assert_eq!(color.r, 100);
        assert_eq!(color.g, 150);
        assert_eq!(color.b, 200);
        assert_eq!(color.a, 128);
        assert!(!color.is_opaque());
        assert!(!color.is_transparent());
    }

    #[test]
    fn test_color_transparent() {
        let color = Color::transparent();
        assert_eq!(color.a, 0);
        assert!(color.is_transparent());
        assert!(!color.is_opaque());
    }

    #[test]
    fn test_color_to_hex() {
        assert_eq!(Color::rgb(255, 0, 0).to_hex(), "#ff0000");
        assert_eq!(Color::rgb(0, 255, 0).to_hex(), "#00ff00");
        assert_eq!(Color::rgb(0, 0, 255).to_hex(), "#0000ff");
        assert_eq!(Color::rgba(255, 0, 0, 128).to_hex(), "#ff000080");
        assert_eq!(Color::transparent().to_hex(), "#00000000");
    }

    #[test]
    fn test_color_blend_opaque_over() {
        let red = Color::rgb(255, 0, 0);
        let blue = Color::rgb(0, 0, 255);
        let result = red.blend_over(blue);
        assert_eq!(result, red); // Opaque completely covers
    }

    #[test]
    fn test_color_blend_transparent_over() {
        let clear = Color::transparent();
        let blue = Color::rgb(0, 0, 255);
        let result = clear.blend_over(blue);
        assert_eq!(result, blue); // Transparent shows background
    }

    #[test]
    fn test_color_lerp() {
        let black = Color::rgb(0, 0, 0);
        let white = Color::rgb(255, 255, 255);

        let mid = black.lerp(white, 0.5);
        assert!(mid.r > 120 && mid.r < 135);
        assert!(mid.g > 120 && mid.g < 135);
        assert!(mid.b > 120 && mid.b < 135);

        let start = black.lerp(white, 0.0);
        assert_eq!(start, black);

        let end = black.lerp(white, 1.0);
        assert_eq!(end, white);
    }

    #[test]
    fn test_color_lerp_clamped() {
        let black = Color::rgb(0, 0, 0);
        let white = Color::rgb(255, 255, 255);

        // Values outside 0-1 should be clamped
        let below = black.lerp(white, -0.5);
        assert_eq!(below, black);

        let above = black.lerp(white, 1.5);
        assert_eq!(above, white);
    }

    // Block tests

    #[test]
    fn test_block_solid() {
        let red = Color::rgb(255, 0, 0);
        let block = Block::solid(red);
        assert_eq!(block.fg, red);
        assert_eq!(block.bg, red);
        assert_eq!(block.character, Block::FULL_BLOCK);
        assert_eq!(block.transparency, 0.0);
        assert!(!block.is_empty());
    }

    #[test]
    fn test_block_empty() {
        let block = Block::empty();
        assert!(block.is_empty());
        assert_eq!(block.transparency, 1.0);
        assert_eq!(block.character, ' ');
    }

    #[test]
    fn test_block_character() {
        let eye = Block::character('o', Color::rgb(0, 0, 0));
        assert_eq!(eye.character, 'o');
        assert!(eye.bg.is_transparent());
        assert!(!eye.is_empty());
    }

    #[test]
    fn test_block_with_z_index() {
        let block = Block::solid(Color::rgb(255, 0, 0)).with_z_index(10);
        assert_eq!(block.z_index, 10);
    }

    #[test]
    fn test_block_with_transparency() {
        let block = Block::solid(Color::rgb(255, 0, 0)).with_transparency(0.5);
        assert_eq!(block.transparency, 0.5);

        // Test clamping
        let clamped_low = Block::solid(Color::rgb(255, 0, 0)).with_transparency(-0.5);
        assert_eq!(clamped_low.transparency, 0.0);

        let clamped_high = Block::solid(Color::rgb(255, 0, 0)).with_transparency(1.5);
        assert_eq!(clamped_high.transparency, 1.0);
    }

    // SizeHint tests

    #[test]
    fn test_size_hint_exact() {
        let hint = SizeHint::exact(10, 20);
        assert_eq!(
            hint,
            SizeHint::Blocks {
                width: 10,
                height: 20
            }
        );
    }

    #[test]
    fn test_size_hint_fit() {
        let hint = SizeHint::fit(100, 50);
        assert_eq!(
            hint,
            SizeHint::FitWithin {
                max_width: 100,
                max_height: 50
            }
        );
    }

    // AnchorPoint tests

    #[test]
    fn test_anchor_point_offset_multipliers() {
        assert_eq!(AnchorPoint::TopLeft.offset_multipliers(), (0.0, 0.0));
        assert_eq!(AnchorPoint::Center.offset_multipliers(), (0.5, 0.5));
        assert_eq!(AnchorPoint::BottomRight.offset_multipliers(), (1.0, 1.0));
    }

    #[test]
    fn test_anchor_point_calculate_top_left() {
        // Center anchor: sprite should be centered on target
        let (x, y) = AnchorPoint::Center.calculate_top_left(50, 50, 10, 10);
        assert_eq!((x, y), (45, 45));

        // TopLeft anchor: top-left should be at target
        let (x, y) = AnchorPoint::TopLeft.calculate_top_left(50, 50, 10, 10);
        assert_eq!((x, y), (50, 50));

        // BottomRight anchor: bottom-right should be at target
        let (x, y) = AnchorPoint::BottomRight.calculate_top_left(50, 50, 10, 10);
        assert_eq!((x, y), (40, 40));
    }

    // Serialization tests

    #[test]
    fn test_color_json_serialization() {
        let color = Color::rgba(255, 128, 64, 200);
        let json = serde_json::to_string(&color).unwrap();
        assert!(json.contains("\"r\":255"));
        assert!(json.contains("\"g\":128"));
        assert!(json.contains("\"b\":64"));
        assert!(json.contains("\"a\":200"));

        let parsed: Color = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, color);
    }

    #[test]
    fn test_block_json_serialization() {
        let block = Block {
            fg: Color::rgb(255, 0, 0),
            bg: Color::rgb(0, 0, 255),
            character: 'X',
            transparency: 0.5,
            z_index: 5,
        };
        let json = serde_json::to_string(&block).unwrap();

        let parsed: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.fg, block.fg);
        assert_eq!(parsed.bg, block.bg);
        assert_eq!(parsed.character, block.character);
        assert!((parsed.transparency - block.transparency).abs() < f32::EPSILON);
        assert_eq!(parsed.z_index, block.z_index);
    }

    #[test]
    fn test_size_hint_json_serialization() {
        let hints = vec![
            SizeHint::Blocks {
                width: 10,
                height: 20,
            },
            SizeHint::Relative(RelativeSize::Large),
            SizeHint::FitWithin {
                max_width: 100,
                max_height: 50,
            },
            SizeHint::Minimal,
            SizeHint::Fill,
        ];

        for hint in hints {
            let json = serde_json::to_string(&hint).unwrap();
            let parsed: SizeHint = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, hint);
        }
    }

    #[test]
    fn test_anchor_point_json_serialization() {
        let anchors = vec![
            AnchorPoint::TopLeft,
            AnchorPoint::Center,
            AnchorPoint::BottomRight,
        ];

        for anchor in anchors {
            let json = serde_json::to_string(&anchor).unwrap();
            let parsed: AnchorPoint = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, anchor);
        }
    }

    #[test]
    fn test_block_json_round_trip_with_unicode() {
        let block = Block {
            fg: Color::rgb(255, 182, 193), // Pink
            bg: Color::transparent(),
            character: '\u{2588}', // Full block
            transparency: 0.0,
            z_index: 0,
        };
        let json = serde_json::to_string_pretty(&block).unwrap();
        let parsed: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.character, '\u{2588}');
    }
}
