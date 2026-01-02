//! Sprite Definitions
//!
//! Blocky pixel art using Unicode block elements and colors.
//! Each cell has its own foreground color for rich, expressive sprites.
//!
//! # Protocol Alignment
//!
//! The `ColoredCell` type aligns with the protocol `Block` type from
//! `conductor_core::avatar::block`. Conversion traits (`From`/`Into`) are
//! provided for seamless interoperability between the protocol layer and
//! the TUI-specific rendering.
//!
//! Key differences handled by conversions:
//! - Protocol `Color` (RGBA) <-> `ratatui::style::Color`
//! - Protocol `transparency` (0=opaque, 1=transparent) <-> TUI `alpha` (1=opaque, 0=transparent)
//! - Protocol `z_index` <-> TUI `blend_mode` (semantic mapping)
//!
//! # Future Enhancements
//!
//! The `ColoredCell` struct includes optional fields for:
//! - Background color (`bg`)
//! - Alpha/opacity (`alpha`)
//! - Blend mode (`blend_mode`)
//!
//! These are currently unused but provide a migration path for future
//! alpha blending, compositing, and richer visual effects without
//! requiring major refactors.

use std::collections::HashMap;

use conductor_core::avatar::block::{Block, Color as ProtocolColor};
use ratatui::style::Color;

/// Blend mode for cell compositing (future use)
///
/// Currently unused, but prepared for future layered rendering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CellBlendMode {
    /// Fully opaque, replaces underlying content
    #[default]
    Opaque,
    /// Alpha blending based on opacity
    Alpha,
    /// Additive blending (brightens)
    Add,
    /// Multiplicative blending (darkens)
    Multiply,
}

/// A single colored cell in a sprite
///
/// Enhanced with optional background color, alpha, and blend mode
/// for future compositing support. Current rendering ignores these
/// optional fields for backward compatibility.
#[derive(Clone, Debug)]
pub struct ColoredCell {
    /// The character to display
    pub ch: char,
    /// Foreground color
    pub fg: Color,
    /// Background color (optional, for future use)
    pub bg: Option<Color>,
    /// Opacity (0.0 = transparent, 1.0 = opaque) - future use
    pub alpha: f32,
    /// Blend mode for compositing - future use
    pub blend_mode: CellBlendMode,
}

impl ColoredCell {
    /// Create a new colored cell (backward compatible)
    pub const fn new(ch: char, fg: Color) -> Self {
        Self {
            ch,
            fg,
            bg: None,
            alpha: 1.0,
            blend_mode: CellBlendMode::Opaque,
        }
    }

    /// Create a cell with background color
    pub const fn with_bg(ch: char, fg: Color, bg: Color) -> Self {
        Self {
            ch,
            fg,
            bg: Some(bg),
            alpha: 1.0,
            blend_mode: CellBlendMode::Opaque,
        }
    }

    /// Create a cell with alpha (for future compositing)
    pub const fn with_alpha(ch: char, fg: Color, alpha: f32) -> Self {
        Self {
            ch,
            fg,
            bg: None,
            alpha,
            blend_mode: CellBlendMode::Alpha,
        }
    }

    /// Empty/transparent cell
    pub const fn empty() -> Self {
        Self {
            ch: ' ',
            fg: Color::Reset,
            bg: None,
            alpha: 0.0,
            blend_mode: CellBlendMode::Opaque,
        }
    }

    /// Check if cell is empty/transparent
    pub fn is_empty(&self) -> bool {
        self.ch == ' '
    }

    /// Check if cell is fully transparent (alpha = 0)
    pub fn is_transparent(&self) -> bool {
        self.alpha <= 0.0 || self.ch == ' '
    }

    /// Check if cell is fully opaque
    pub fn is_opaque(&self) -> bool {
        (self.alpha - 1.0).abs() < f32::EPSILON && self.blend_mode == CellBlendMode::Opaque
    }

    /// Set blend mode (builder pattern)
    #[must_use]
    pub const fn blend(mut self, mode: CellBlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    /// Set alpha (builder pattern)
    #[must_use]
    pub const fn opacity(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }
}

// ============================================================================
// Protocol Color Conversions
// ============================================================================

/// Convert protocol `Color` (RGBA) to `ratatui::style::Color`
///
/// This handles the mapping from the surface-agnostic protocol color
/// to the terminal-specific ratatui color type.
///
/// # Color Mapping
///
/// - Fully transparent colors (alpha=0) map to `Color::Reset`
/// - Opaque colors map to `Color::Rgb(r, g, b)`
/// - Semi-transparent colors also map to RGB (terminal doesn't support alpha)
#[must_use]
pub fn protocol_color_to_ratatui(c: ProtocolColor) -> Color {
    if c.is_transparent() {
        Color::Reset
    } else {
        Color::Rgb(c.r, c.g, c.b)
    }
}

/// Convert `ratatui::style::Color` to protocol `Color` (RGBA)
///
/// # Color Mapping
///
/// - `Color::Reset` -> transparent (0, 0, 0, 0)
/// - `Color::Rgb(r, g, b)` -> opaque (r, g, b, 255)
/// - Named colors (Red, Blue, etc.) -> opaque with standard RGB values
/// - Indexed colors -> approximated RGB values
#[must_use]
pub fn ratatui_color_to_protocol(c: Color) -> ProtocolColor {
    match c {
        Color::Reset => ProtocolColor::transparent(),
        Color::Rgb(r, g, b) => ProtocolColor::rgb(r, g, b),
        // Named colors with standard terminal RGB values
        Color::Black => ProtocolColor::rgb(0, 0, 0),
        Color::Red => ProtocolColor::rgb(205, 49, 49),
        Color::Green => ProtocolColor::rgb(13, 188, 121),
        Color::Yellow => ProtocolColor::rgb(229, 229, 16),
        Color::Blue => ProtocolColor::rgb(36, 114, 200),
        Color::Magenta => ProtocolColor::rgb(188, 63, 188),
        Color::Cyan => ProtocolColor::rgb(17, 168, 205),
        Color::Gray => ProtocolColor::rgb(128, 128, 128),
        Color::DarkGray => ProtocolColor::rgb(102, 102, 102),
        Color::LightRed => ProtocolColor::rgb(241, 76, 76),
        Color::LightGreen => ProtocolColor::rgb(35, 209, 139),
        Color::LightYellow => ProtocolColor::rgb(245, 245, 67),
        Color::LightBlue => ProtocolColor::rgb(59, 142, 234),
        Color::LightMagenta => ProtocolColor::rgb(214, 112, 214),
        Color::LightCyan => ProtocolColor::rgb(41, 184, 219),
        Color::White => ProtocolColor::rgb(229, 229, 229),
        Color::Indexed(idx) => {
            // Standard 256-color palette approximation
            // 0-15: Standard colors (handled above for named)
            // 16-231: 6x6x6 color cube
            // 232-255: Grayscale ramp
            if idx < 16 {
                // Standard colors - fallback to gray
                ProtocolColor::rgb(128, 128, 128)
            } else if idx < 232 {
                // 6x6x6 color cube
                let idx = idx - 16;
                let r = (idx / 36) % 6;
                let g = (idx / 6) % 6;
                let b = idx % 6;
                let to_rgb = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
                ProtocolColor::rgb(to_rgb(r), to_rgb(g), to_rgb(b))
            } else {
                // Grayscale ramp (232-255)
                let gray = 8 + (idx - 232) * 10;
                ProtocolColor::rgb(gray, gray, gray)
            }
        }
    }
}

// ============================================================================
// ColoredCell <-> Block Conversions
// ============================================================================

/// Convert protocol `Block` to TUI `ColoredCell`
///
/// This conversion handles the semantic differences:
/// - Protocol `transparency` (0=opaque, 1=transparent) -> TUI `alpha` (1=opaque, 0=transparent)
/// - Protocol `z_index` -> TUI `blend_mode` (positive z_index implies layering)
impl From<Block> for ColoredCell {
    fn from(block: Block) -> Self {
        // Convert transparency to alpha (inverted semantics)
        let alpha = 1.0 - block.transparency;

        // Determine blend mode based on transparency and z_index
        let blend_mode = if block.transparency > 0.0 {
            CellBlendMode::Alpha
        } else {
            CellBlendMode::Opaque
        };

        Self {
            ch: block.character,
            fg: protocol_color_to_ratatui(block.fg),
            bg: if block.bg.is_transparent() {
                None
            } else {
                Some(protocol_color_to_ratatui(block.bg))
            },
            alpha,
            blend_mode,
        }
    }
}

/// Convert TUI `ColoredCell` to protocol `Block`
#[must_use]
pub fn colored_cell_to_block(cell: ColoredCell) -> Block {
    // Convert alpha to transparency (inverted semantics)
    let transparency = 1.0 - cell.alpha;

    // Determine z_index based on blend mode
    let z_index = match cell.blend_mode {
        CellBlendMode::Opaque => 0,
        CellBlendMode::Alpha => 1,
        CellBlendMode::Add => 2,
        CellBlendMode::Multiply => -1,
    };

    Block {
        fg: ratatui_color_to_protocol(cell.fg),
        bg: cell
            .bg
            .map_or(ProtocolColor::transparent(), ratatui_color_to_protocol),
        character: cell.ch,
        transparency,
        z_index,
    }
}

/// A single animation frame with per-cell coloring
#[derive(Clone, Debug)]
pub struct Frame {
    /// 2D grid of colored cells (row-major)
    pub cells: Vec<Vec<ColoredCell>>,
    /// Width in terminal cells
    pub width: u16,
    /// Height in terminal cells
    pub height: u16,
    /// Duration in milliseconds (default 100ms for ~10fps)
    pub duration_ms: u64,
}

impl Frame {
    /// Create a frame from a grid of colored cells
    pub fn new(cells: Vec<Vec<ColoredCell>>, duration_ms: u64) -> Self {
        let height = cells.len() as u16;
        let width = cells.iter().map(|row| row.len() as u16).max().unwrap_or(0);

        Self {
            cells,
            width,
            height,
            duration_ms,
        }
    }

    /// Get cell at position (returns empty if out of bounds)
    pub fn get(&self, x: u16, y: u16) -> &ColoredCell {
        static EMPTY: ColoredCell = ColoredCell::empty();
        self.cells
            .get(y as usize)
            .and_then(|row| row.get(x as usize))
            .unwrap_or(&EMPTY)
    }
}

/// An animation sequence
#[derive(Clone, Debug)]
pub struct Animation {
    /// Animation name
    pub name: String,
    /// Frames in sequence
    pub frames: Vec<Frame>,
    /// Whether to loop
    pub looping: bool,
}

impl Animation {
    /// Total duration of all frames
    pub fn total_duration(&self) -> u64 {
        self.frames.iter().map(|f| f.duration_ms).sum()
    }
}

/// Collection of animations for a size
#[derive(Debug)]
pub struct SpriteSheet {
    /// Animations by name
    pub animations: HashMap<String, Animation>,
    /// Default animation name
    pub default: String,
}

impl SpriteSheet {
    /// Get an animation by name, falling back to default
    pub fn get(&self, name: &str) -> Option<&Animation> {
        self.animations
            .get(name)
            .or_else(|| self.animations.get(&self.default))
    }
}

// ============================================================================
// Sprite Builder Helpers
// ============================================================================

/// Parse a sprite definition using a color map
///
/// Format: each character in the pattern maps to a (char, Color) in the palette.
/// Special: ' ' (space) is always transparent.
///
/// Example:
/// ```ignore
/// let palette = [('B', ('█', BODY)), ('G', ('█', GILLS)), ('e', ('o', EYES))];
/// let pattern = [
///     "  GBG  ",
///     " BeeeB ",
///     "  BBB  ",
/// ];
/// ```
pub fn build_frame(pattern: &[&str], palette: &[(char, char, Color)], duration_ms: u64) -> Frame {
    let color_map: HashMap<char, (char, Color)> = palette
        .iter()
        .map(|&(key, ch, color)| (key, (ch, color)))
        .collect();

    let cells: Vec<Vec<ColoredCell>> = pattern
        .iter()
        .map(|line| {
            line.chars()
                .map(|c| {
                    if c == ' ' {
                        ColoredCell::empty()
                    } else if let Some(&(ch, color)) = color_map.get(&c) {
                        ColoredCell::new(ch, color)
                    } else {
                        // Unknown char - show as-is in default color
                        ColoredCell::new(c, Color::Reset)
                    }
                })
                .collect()
        })
        .collect();

    Frame::new(cells, duration_ms)
}

/// Build multiple frames with the same palette
pub fn build_animation(
    name: &str,
    frames_data: &[(&[&str], u64)],
    palette: &[(char, char, Color)],
    looping: bool,
) -> Animation {
    let frames = frames_data
        .iter()
        .map(|(pattern, duration)| build_frame(pattern, palette, *duration))
        .collect();

    Animation {
        name: name.to_string(),
        frames,
        looping,
    }
}
