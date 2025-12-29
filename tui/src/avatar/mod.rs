//! Avatar System - The Heart of Yollayah's UX
//!
//! The animated axolotl avatar is the soul of the terminal interface.
//! It expresses personality through:
//! - Multi-size blocky pixel art sprites
//! - Per-cell coloring using the axolotl palette
//! - Context-aware animations at ~10fps
//! - State-driven behavior

mod animation;
pub mod commands;
mod sizes;
mod sprites;
mod states;

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::style::Style;

pub use animation::AnimationEngine;
pub use sizes::AvatarSize;
pub use sprites::{Animation, ColoredCell, Frame};
pub use states::{AvatarState, AvatarStateMachine, AvatarTrigger};

/// The animated axolotl avatar
pub struct Avatar {
    /// Animation engine
    engine: AnimationEngine,
    /// Current size
    size: AvatarSize,
}

impl Avatar {
    /// Create a new avatar
    pub fn new() -> Self {
        Self {
            engine: AnimationEngine::new(),
            size: AvatarSize::Medium,
        }
    }

    /// Update animation (call every frame)
    pub fn update(&mut self, delta: Duration) {
        self.engine.update(delta, self.size);
    }

    /// Play a named animation
    pub fn play(&mut self, name: &str) {
        self.engine.play(name);
    }

    /// Set the avatar size
    pub fn set_size(&mut self, size: AvatarSize) {
        self.size = size;
    }

    /// Render the avatar to a buffer with per-cell coloring
    pub fn render(&self, buf: &mut Buffer) {
        let frame = match self.engine.current_frame(self.size) {
            Some(f) => f,
            None => return,
        };

        let area = buf.area;

        // Center the frame in the buffer
        let x_offset = area.width.saturating_sub(frame.width) / 2;
        let y_offset = area.height.saturating_sub(frame.height) / 2;

        // Render each colored cell
        for (row_idx, row) in frame.cells.iter().enumerate() {
            let y = area.y + y_offset + row_idx as u16;
            if y >= area.y + area.height {
                break;
            }

            for (col_idx, cell) in row.iter().enumerate() {
                let x = area.x + x_offset + col_idx as u16;
                if x >= area.x + area.width {
                    break;
                }

                // Skip empty/transparent cells
                if cell.is_empty() {
                    continue;
                }

                // Set the cell with its specific color
                let style = Style::default().fg(cell.fg);
                buf.set_string(x, y, cell.ch.to_string(), style);
            }
        }
    }

    /// Get current size bounds
    pub fn bounds(&self) -> (u16, u16) {
        self.size.max_bounds()
    }

    /// Get current animation name
    pub fn current_animation(&self) -> &str {
        self.engine.current_animation()
    }
}

impl Default for Avatar {
    fn default() -> Self {
        Self::new()
    }
}
