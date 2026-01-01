//! Sprite Definitions
//!
//! Blocky pixel art using Unicode block elements and colors.
//! Each cell has its own foreground color for rich, expressive sprites.

use std::collections::HashMap;

use ratatui::style::Color;

/// A single colored cell in a sprite
#[derive(Clone, Debug)]
pub struct ColoredCell {
    /// The character to display
    pub ch: char,
    /// Foreground color
    pub fg: Color,
}

impl ColoredCell {
    /// Create a new colored cell
    pub const fn new(ch: char, fg: Color) -> Self {
        Self { ch, fg }
    }

    /// Empty/transparent cell
    pub const fn empty() -> Self {
        Self {
            ch: ' ',
            fg: Color::Reset,
        }
    }

    /// Check if cell is empty/transparent
    pub fn is_empty(&self) -> bool {
        self.ch == ' '
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
