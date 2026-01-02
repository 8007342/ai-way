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

use std::time::Duration;

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

// ============================================
// Sprite Protocol Types (P1.2-P1.3)
// ============================================

/// Mood for sprite requests
///
/// Represents the emotional state to apply to a sprite.
/// Maps to avatar moods but is protocol-specific for sprite requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Mood {
    /// Default happy state
    #[default]
    Happy,
    /// Deep in thought
    Thinking,
    /// Playful and silly
    Playful,
    /// Bashful/embarrassed
    Shy,
    /// Very excited
    Excited,
    /// Puzzled/uncertain
    Confused,
    /// Peaceful and relaxed
    Calm,
    /// Interested and engaged
    Curious,
    /// Sad or disappointed
    Sad,
    /// Focused and determined
    Focused,
}

/// Loop behavior for animations
///
/// Defines how an animation should repeat after completing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum LoopBehavior {
    /// Play once and stop
    #[default]
    Once,
    /// Loop indefinitely
    Loop,
    /// Play forward then backward, repeat
    PingPong,
}

/// Request for a sprite from the Conductor
///
/// Surfaces send this to request sprite data for rendering.
/// The Conductor responds with a `SpriteResponse`.
///
/// # Examples
///
/// ```
/// use conductor_core::avatar::block::{SpriteRequest, Mood, SizeHint};
///
/// // Request an idle sprite in happy mood
/// let request = SpriteRequest {
///     base: "idle".to_string(),
///     mood: Some(Mood::Happy),
///     size: Some(SizeHint::Relative(conductor_core::avatar::block::RelativeSize::Medium)),
///     context: Some("greeting user".to_string()),
///     evolution: Some(25),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpriteRequest {
    /// Base sprite name (e.g., "idle", "wave", "thinking")
    pub base: String,
    /// Optional mood to apply to the sprite
    pub mood: Option<Mood>,
    /// Optional size hint for the sprite
    pub size: Option<SizeHint>,
    /// Optional context describing what the avatar is doing
    pub context: Option<String>,
    /// Evolution level (0-100) for progressive sprite variations
    ///
    /// Higher values indicate more evolved/personalized sprites.
    /// 0 = base sprite, 100 = fully evolved with all accessories
    pub evolution: Option<u8>,
}

impl SpriteRequest {
    /// Create a new sprite request with just a base sprite name
    #[must_use]
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            mood: None,
            size: None,
            context: None,
            evolution: None,
        }
    }

    /// Set the mood for this request
    #[must_use]
    pub fn with_mood(mut self, mood: Mood) -> Self {
        self.mood = Some(mood);
        self
    }

    /// Set the size hint for this request
    #[must_use]
    pub fn with_size(mut self, size: SizeHint) -> Self {
        self.size = Some(size);
        self
    }

    /// Set the context for this request
    #[must_use]
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Set the evolution level for this request
    #[must_use]
    pub fn with_evolution(mut self, evolution: u8) -> Self {
        self.evolution = Some(evolution.min(100));
        self
    }
}

impl Default for SpriteRequest {
    fn default() -> Self {
        Self::new("idle")
    }
}

/// Response containing sprite data
///
/// Returned by the Conductor in response to a `SpriteRequest`.
/// Contains all the blocks needed to render the sprite.
///
/// # Examples
///
/// ```
/// use conductor_core::avatar::block::{SpriteResponse, Block, Color, AnchorPoint};
/// use std::time::Duration;
///
/// let response = SpriteResponse {
///     blocks: vec![
///         Block::solid(Color::rgb(255, 182, 193)),
///         Block::solid(Color::rgb(255, 182, 193)),
///     ],
///     dimensions: (2, 1),
///     anchor: AnchorPoint::Center,
///     cache_key: Some("idle_happy_medium_v1".to_string()),
///     ttl: Some(Duration::from_secs(300)),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpriteResponse {
    /// The blocks that make up the sprite (row-major order)
    ///
    /// Blocks are stored in row-major order: blocks[y * width + x]
    pub blocks: Vec<Block>,
    /// Dimensions of the sprite (width, height) in blocks
    pub dimensions: (u16, u16),
    /// Anchor point for positioning the sprite
    pub anchor: AnchorPoint,
    /// Optional cache key for client-side caching
    ///
    /// If provided, clients should cache this sprite and use the key
    /// to avoid re-requesting the same sprite.
    pub cache_key: Option<String>,
    /// Optional time-to-live for cached sprites
    ///
    /// After this duration, the cached sprite should be considered stale.
    #[serde(default, with = "optional_duration_millis")]
    pub ttl: Option<Duration>,
}

impl SpriteResponse {
    /// Create a new sprite response
    #[must_use]
    pub fn new(blocks: Vec<Block>, width: u16, height: u16) -> Self {
        Self {
            blocks,
            dimensions: (width, height),
            anchor: AnchorPoint::default(),
            cache_key: None,
            ttl: None,
        }
    }

    /// Get the width of the sprite in blocks
    #[must_use]
    pub fn width(&self) -> u16 {
        self.dimensions.0
    }

    /// Get the height of the sprite in blocks
    #[must_use]
    pub fn height(&self) -> u16 {
        self.dimensions.1
    }

    /// Get a block at the given coordinates
    ///
    /// Returns `None` if coordinates are out of bounds.
    #[must_use]
    pub fn get_block(&self, x: u16, y: u16) -> Option<&Block> {
        if x >= self.width() || y >= self.height() {
            return None;
        }
        let index = (y as usize) * (self.width() as usize) + (x as usize);
        self.blocks.get(index)
    }

    /// Set the anchor point for this response
    #[must_use]
    pub fn with_anchor(mut self, anchor: AnchorPoint) -> Self {
        self.anchor = anchor;
        self
    }

    /// Set the cache key for this response
    #[must_use]
    pub fn with_cache_key(mut self, key: impl Into<String>) -> Self {
        self.cache_key = Some(key.into());
        self
    }

    /// Set the TTL for this response
    #[must_use]
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }
}

/// Request for an animation sequence
///
/// Surfaces send this to request a complete animation (multiple frames).
/// The Conductor responds with an `AnimationResponse`.
///
/// # Examples
///
/// ```
/// use conductor_core::avatar::block::{AnimationRequest, LoopBehavior};
/// use std::time::Duration;
///
/// let request = AnimationRequest {
///     name: "celebrate".to_string(),
///     duration: Some(Duration::from_secs(2)),
///     loop_behavior: LoopBehavior::Once,
///     interruptible: true,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationRequest {
    /// Animation name (e.g., "celebrate", "work", "rest")
    pub name: String,
    /// Optional total duration for the animation
    ///
    /// If specified, frame timing will be adjusted to fit this duration.
    #[serde(default, with = "optional_duration_millis")]
    pub duration: Option<Duration>,
    /// How the animation should loop
    pub loop_behavior: LoopBehavior,
    /// Whether this animation can be interrupted by other animations
    pub interruptible: bool,
}

impl AnimationRequest {
    /// Create a new animation request
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            duration: None,
            loop_behavior: LoopBehavior::default(),
            interruptible: true,
        }
    }

    /// Set the duration for this animation
    #[must_use]
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Set the loop behavior for this animation
    #[must_use]
    pub fn with_loop_behavior(mut self, behavior: LoopBehavior) -> Self {
        self.loop_behavior = behavior;
        self
    }

    /// Set whether this animation is interruptible
    #[must_use]
    pub fn with_interruptible(mut self, interruptible: bool) -> Self {
        self.interruptible = interruptible;
        self
    }

    /// Create a looping animation request
    #[must_use]
    pub fn looping(name: impl Into<String>) -> Self {
        Self::new(name).with_loop_behavior(LoopBehavior::Loop)
    }

    /// Create a one-shot animation request
    #[must_use]
    pub fn once(name: impl Into<String>) -> Self {
        Self::new(name).with_loop_behavior(LoopBehavior::Once)
    }
}

impl Default for AnimationRequest {
    fn default() -> Self {
        Self::new("idle")
    }
}

/// Response containing animation data
///
/// Returned by the Conductor in response to an `AnimationRequest`.
/// Contains all frames and timing information for the animation.
///
/// # Examples
///
/// ```
/// use conductor_core::avatar::block::{AnimationResponse, SpriteResponse, Block, Color, AnchorPoint};
/// use std::time::Duration;
///
/// let frame1 = SpriteResponse::new(
///     vec![Block::solid(Color::rgb(255, 0, 0))],
///     1, 1
/// );
/// let frame2 = SpriteResponse::new(
///     vec![Block::solid(Color::rgb(0, 255, 0))],
///     1, 1
/// );
///
/// let animation = AnimationResponse {
///     frames: vec![frame1, frame2],
///     timing: vec![Duration::from_millis(100), Duration::from_millis(100)],
///     cache_key: Some("blink_v1".to_string()),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationResponse {
    /// The frames of the animation (each is a complete sprite)
    pub frames: Vec<SpriteResponse>,
    /// Duration to display each frame
    ///
    /// Length should match `frames.len()`. If shorter, the last
    /// duration is used for remaining frames.
    #[serde(with = "duration_vec_millis")]
    pub timing: Vec<Duration>,
    /// Optional cache key for client-side caching
    pub cache_key: Option<String>,
}

impl AnimationResponse {
    /// Create a new animation response
    #[must_use]
    pub fn new(frames: Vec<SpriteResponse>, timing: Vec<Duration>) -> Self {
        Self {
            frames,
            timing,
            cache_key: None,
        }
    }

    /// Create an animation with uniform frame timing
    #[must_use]
    pub fn uniform(frames: Vec<SpriteResponse>, frame_duration: Duration) -> Self {
        let timing = vec![frame_duration; frames.len()];
        Self::new(frames, timing)
    }

    /// Get the number of frames in this animation
    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Get the total duration of the animation
    #[must_use]
    pub fn total_duration(&self) -> Duration {
        self.timing.iter().sum()
    }

    /// Get the timing for a specific frame
    ///
    /// Returns the last timing value if index is out of bounds.
    #[must_use]
    pub fn frame_duration(&self, index: usize) -> Duration {
        self.timing
            .get(index)
            .or_else(|| self.timing.last())
            .copied()
            .unwrap_or(Duration::from_millis(100))
    }

    /// Set the cache key for this response
    #[must_use]
    pub fn with_cache_key(mut self, key: impl Into<String>) -> Self {
        self.cache_key = Some(key.into());
        self
    }
}

// ============================================
// Serde helpers for Duration
// ============================================

/// Serialize/deserialize optional Duration as milliseconds
mod optional_duration_millis {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match duration {
            Some(d) => serializer.serialize_some(&d.as_millis()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis: Option<u64> = Option::deserialize(deserializer)?;
        Ok(millis.map(Duration::from_millis))
    }
}

/// Serialize/deserialize Vec<Duration> as milliseconds
mod duration_vec_millis {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(durations: &[Duration], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let millis: Vec<u64> = durations.iter().map(|d| d.as_millis() as u64).collect();
        millis.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis: Vec<u64> = Vec::deserialize(deserializer)?;
        Ok(millis.into_iter().map(Duration::from_millis).collect())
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

    // ============================================
    // Sprite Protocol Type Tests (P1.2-P1.3)
    // ============================================

    #[test]
    fn test_mood_default() {
        assert_eq!(Mood::default(), Mood::Happy);
    }

    #[test]
    fn test_loop_behavior_default() {
        assert_eq!(LoopBehavior::default(), LoopBehavior::Once);
    }

    #[test]
    fn test_sprite_request_builder() {
        let request = SpriteRequest::new("wave")
            .with_mood(Mood::Excited)
            .with_size(SizeHint::Relative(RelativeSize::Large))
            .with_context("greeting")
            .with_evolution(50);

        assert_eq!(request.base, "wave");
        assert_eq!(request.mood, Some(Mood::Excited));
        assert_eq!(request.size, Some(SizeHint::Relative(RelativeSize::Large)));
        assert_eq!(request.context, Some("greeting".to_string()));
        assert_eq!(request.evolution, Some(50));
    }

    #[test]
    fn test_sprite_request_evolution_clamped() {
        let request = SpriteRequest::new("idle").with_evolution(150);
        assert_eq!(request.evolution, Some(100));
    }

    #[test]
    fn test_sprite_request_json_serialization() {
        let request = SpriteRequest {
            base: "thinking".to_string(),
            mood: Some(Mood::Thinking),
            size: Some(SizeHint::Relative(RelativeSize::Medium)),
            context: Some("solving problem".to_string()),
            evolution: Some(25),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: SpriteRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, request);
    }

    #[test]
    fn test_sprite_response_new() {
        let blocks = vec![
            Block::solid(Color::rgb(255, 0, 0)),
            Block::solid(Color::rgb(0, 255, 0)),
            Block::solid(Color::rgb(0, 0, 255)),
            Block::solid(Color::rgb(255, 255, 0)),
        ];
        let response = SpriteResponse::new(blocks.clone(), 2, 2);

        assert_eq!(response.width(), 2);
        assert_eq!(response.height(), 2);
        assert_eq!(response.dimensions, (2, 2));
        assert_eq!(response.blocks.len(), 4);
    }

    #[test]
    fn test_sprite_response_get_block() {
        let blocks = vec![
            Block::solid(Color::rgb(255, 0, 0)),   // (0, 0)
            Block::solid(Color::rgb(0, 255, 0)),   // (1, 0)
            Block::solid(Color::rgb(0, 0, 255)),   // (0, 1)
            Block::solid(Color::rgb(255, 255, 0)), // (1, 1)
        ];
        let response = SpriteResponse::new(blocks, 2, 2);

        // Check valid coordinates
        assert_eq!(response.get_block(0, 0).unwrap().fg, Color::rgb(255, 0, 0));
        assert_eq!(response.get_block(1, 0).unwrap().fg, Color::rgb(0, 255, 0));
        assert_eq!(response.get_block(0, 1).unwrap().fg, Color::rgb(0, 0, 255));
        assert_eq!(
            response.get_block(1, 1).unwrap().fg,
            Color::rgb(255, 255, 0)
        );

        // Check out of bounds
        assert!(response.get_block(2, 0).is_none());
        assert!(response.get_block(0, 2).is_none());
    }

    #[test]
    fn test_sprite_response_builder() {
        let response = SpriteResponse::new(vec![Block::solid(Color::rgb(255, 0, 0))], 1, 1)
            .with_anchor(AnchorPoint::BottomCenter)
            .with_cache_key("test_key")
            .with_ttl(Duration::from_secs(60));

        assert_eq!(response.anchor, AnchorPoint::BottomCenter);
        assert_eq!(response.cache_key, Some("test_key".to_string()));
        assert_eq!(response.ttl, Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_sprite_response_json_with_ttl() {
        let response = SpriteResponse {
            blocks: vec![Block::solid(Color::rgb(255, 0, 0))],
            dimensions: (1, 1),
            anchor: AnchorPoint::Center,
            cache_key: Some("cache_v1".to_string()),
            ttl: Some(Duration::from_millis(5000)),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("5000")); // TTL serialized as millis

        let parsed: SpriteResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.ttl, Some(Duration::from_millis(5000)));
    }

    #[test]
    fn test_animation_request_builder() {
        let request = AnimationRequest::new("celebrate")
            .with_duration(Duration::from_secs(2))
            .with_loop_behavior(LoopBehavior::PingPong)
            .with_interruptible(false);

        assert_eq!(request.name, "celebrate");
        assert_eq!(request.duration, Some(Duration::from_secs(2)));
        assert_eq!(request.loop_behavior, LoopBehavior::PingPong);
        assert!(!request.interruptible);
    }

    #[test]
    fn test_animation_request_shortcuts() {
        let looping = AnimationRequest::looping("idle");
        assert_eq!(looping.loop_behavior, LoopBehavior::Loop);

        let once = AnimationRequest::once("wave");
        assert_eq!(once.loop_behavior, LoopBehavior::Once);
    }

    #[test]
    fn test_animation_request_json_serialization() {
        let request = AnimationRequest {
            name: "thinking".to_string(),
            duration: Some(Duration::from_millis(1500)),
            loop_behavior: LoopBehavior::Loop,
            interruptible: true,
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: AnimationRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, request);
    }

    #[test]
    fn test_animation_response_new() {
        let frame1 = SpriteResponse::new(vec![Block::solid(Color::rgb(255, 0, 0))], 1, 1);
        let frame2 = SpriteResponse::new(vec![Block::solid(Color::rgb(0, 255, 0))], 1, 1);

        let response = AnimationResponse::new(
            vec![frame1, frame2],
            vec![Duration::from_millis(100), Duration::from_millis(200)],
        );

        assert_eq!(response.frame_count(), 2);
        assert_eq!(response.total_duration(), Duration::from_millis(300));
    }

    #[test]
    fn test_animation_response_uniform() {
        let frame1 = SpriteResponse::new(vec![Block::solid(Color::rgb(255, 0, 0))], 1, 1);
        let frame2 = SpriteResponse::new(vec![Block::solid(Color::rgb(0, 255, 0))], 1, 1);
        let frame3 = SpriteResponse::new(vec![Block::solid(Color::rgb(0, 0, 255))], 1, 1);

        let response =
            AnimationResponse::uniform(vec![frame1, frame2, frame3], Duration::from_millis(100));

        assert_eq!(response.frame_count(), 3);
        assert_eq!(response.timing.len(), 3);
        assert_eq!(response.total_duration(), Duration::from_millis(300));
    }

    #[test]
    fn test_animation_response_frame_duration() {
        let frame = SpriteResponse::new(vec![Block::solid(Color::rgb(255, 0, 0))], 1, 1);

        let response = AnimationResponse::new(
            vec![frame.clone(), frame.clone(), frame],
            vec![Duration::from_millis(100), Duration::from_millis(200)],
        );

        // Normal indexing
        assert_eq!(response.frame_duration(0), Duration::from_millis(100));
        assert_eq!(response.frame_duration(1), Duration::from_millis(200));

        // Out of bounds uses last value
        assert_eq!(response.frame_duration(2), Duration::from_millis(200));
        assert_eq!(response.frame_duration(100), Duration::from_millis(200));
    }

    #[test]
    fn test_animation_response_json_serialization() {
        let frame1 = SpriteResponse::new(vec![Block::solid(Color::rgb(255, 0, 0))], 1, 1);
        let frame2 = SpriteResponse::new(vec![Block::solid(Color::rgb(0, 255, 0))], 1, 1);

        let response = AnimationResponse {
            frames: vec![frame1, frame2],
            timing: vec![Duration::from_millis(100), Duration::from_millis(150)],
            cache_key: Some("anim_v1".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: AnimationResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.frames.len(), response.frames.len());
        assert_eq!(parsed.timing, response.timing);
        assert_eq!(parsed.cache_key, response.cache_key);
    }

    #[test]
    fn test_mood_json_serialization() {
        let moods = vec![
            Mood::Happy,
            Mood::Thinking,
            Mood::Playful,
            Mood::Shy,
            Mood::Excited,
            Mood::Confused,
            Mood::Calm,
            Mood::Curious,
            Mood::Sad,
            Mood::Focused,
        ];

        for mood in moods {
            let json = serde_json::to_string(&mood).unwrap();
            let parsed: Mood = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, mood);
        }
    }

    #[test]
    fn test_loop_behavior_json_serialization() {
        let behaviors = vec![
            LoopBehavior::Once,
            LoopBehavior::Loop,
            LoopBehavior::PingPong,
        ];

        for behavior in behaviors {
            let json = serde_json::to_string(&behavior).unwrap();
            let parsed: LoopBehavior = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, behavior);
        }
    }
}
