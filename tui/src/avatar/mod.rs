//! Avatar System - The Heart of Yollayah's UX
//!
//! The animated axolotl avatar is the soul of the terminal interface.
//! It expresses personality through:
//! - Multi-size blocky pixel art sprites
//! - Per-cell coloring using the axolotl palette
//! - Context-aware animations at ~10fps
//! - State-driven behavior
//! - Activity overlays (thinking, building, studying, etc.)

mod activity;
mod animation;
pub mod commands;
mod sizes;
mod sprites;
mod states;

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::style::Style;

pub use activity::{Activity, ActivityManager, OverlaySize};
pub use animation::AnimationEngine;
pub use sizes::AvatarSize;
pub use sprites::{Animation, CellBlendMode, ColoredCell, Frame};
pub use states::{AvatarState, AvatarStateMachine, AvatarTrigger};

/// The animated axolotl avatar
pub struct Avatar {
    /// Animation engine
    engine: AnimationEngine,
    /// Current size
    size: AvatarSize,
    /// Activity overlay manager
    activity: ActivityManager,
}

impl Avatar {
    /// Create a new avatar
    pub fn new() -> Self {
        Self {
            engine: AnimationEngine::new(),
            size: AvatarSize::Medium,
            activity: ActivityManager::new(),
        }
    }

    /// Update animation (call every frame)
    pub fn update(&mut self, delta: Duration) {
        self.engine.update(delta, self.size);
        self.activity.update(delta);
    }

    /// Play a named animation
    pub fn play(&mut self, name: &str) {
        self.engine.play(name);
    }

    /// Set the avatar size
    pub fn set_size(&mut self, size: AvatarSize) {
        self.size = size;
        // Sync overlay size
        let overlay_size = match size {
            AvatarSize::Tiny => OverlaySize::Tiny,
            AvatarSize::Small => OverlaySize::Small,
            AvatarSize::Medium => OverlaySize::Medium,
            AvatarSize::Large => OverlaySize::Large,
        };
        self.activity.set_size(overlay_size);
    }

    /// Set the current activity (shows overlay)
    pub fn set_activity(&mut self, activity: Activity) {
        self.activity.set_activity(activity);
    }

    /// Get current activity
    pub fn current_activity(&self) -> Activity {
        self.activity.current_activity()
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

        // Render the base sprite
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

        // Render activity overlay on top
        self.render_overlay(buf, x_offset, y_offset);
    }

    /// Render the activity overlay
    fn render_overlay(&self, buf: &mut Buffer, avatar_x: u16, avatar_y: u16) {
        let overlay = match self.activity.overlay() {
            Some(o) => o,
            None => return,
        };

        let overlay_frame = match overlay.current_frame() {
            Some(f) => f,
            None => return,
        };

        let area = buf.area;

        // Calculate overlay position (avatar center + offset)
        let (offset_x, offset_y) = overlay.offset;
        let overlay_x = (avatar_x as i32 + offset_x as i32).max(0) as u16;
        let overlay_y = (avatar_y as i32 + offset_y as i32).max(0) as u16;

        // Render overlay cells
        for (row_idx, row) in overlay_frame.cells.iter().enumerate() {
            let y = area.y + overlay_y + row_idx as u16;
            if y >= area.y + area.height {
                break;
            }

            for (col_idx, cell) in row.iter().enumerate() {
                let x = area.x + overlay_x + col_idx as u16;
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
