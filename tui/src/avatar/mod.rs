//! Avatar System - The Heart of Yollayah's UX
//!
//! The animated axolotl avatar is the soul of the terminal interface.
//! It expresses personality through:
//! - Multi-size blocky pixel art sprites
//! - Per-cell coloring using the axolotl palette
//! - Context-aware animations at ~10fps
//! - State-driven behavior
//! - Activity overlays (thinking, building, studying, etc.)
//!
//! # Animation System (P2.4-P2.5)
//!
//! The animation system provides:
//! - [`AvatarAnimator`]: Frame timing and mood transitions
//! - [`DirtyTracker`]: Partial rendering for CPU optimization
//!
//! # Accessibility (P2.6)
//!
//! The accessibility module provides reduced-motion support:
//! - [`accessibility::MotionPreference`]: Full, Reduced, or None motion modes
//! - [`accessibility::detect_motion_preference`]: Auto-detect from `REDUCE_MOTION` env var
//! - [`accessibility::AccessibleAnimator`]: Wrapper that respects preferences

pub mod accessibility;
mod activity;
mod animation;
mod animator;
pub mod commands;
mod dirty_tracker;
mod sizes;
mod sprites;
mod states;

use std::time::Duration;

use ratatui::buffer::Buffer;

pub use accessibility::{
    detect_motion_preference, parse_motion_preference, AccessibleAnimator, MotionPreference,
};
pub use activity::{Activity, ActivityManager, OverlaySize};
pub use animation::AnimationEngine;
pub use animator::{AvatarAnimator, MoodTransition};
pub use dirty_tracker::{DirtyRect, DirtyTracker, DirtyTrackingExt};
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

    /// Update animation (call every frame) - returns true if avatar changed
    pub fn update(&mut self, delta: Duration) -> bool {
        // Track previous state
        let prev_frame = self.engine.current_frame_index();
        let prev_animation = self.engine.current_animation().to_string();

        // Update animation
        self.engine.update(delta, self.size);
        self.activity.update(delta);

        // Check if anything changed
        let frame_changed = self.engine.current_frame_index() != prev_frame;
        let animation_changed = self.engine.current_animation() != prev_animation;

        frame_changed || animation_changed
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

                // Set the cell with its specific color (no allocation)
                if let Some(target_cell) = buf.cell_mut((x, y)) {
                    target_cell.set_char(cell.ch);
                    target_cell.set_fg(cell.fg);
                }
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

                // Set the cell with its specific color (no allocation)
                if let Some(target_cell) = buf.cell_mut((x, y)) {
                    target_cell.set_char(cell.ch);
                    target_cell.set_fg(cell.fg);
                }
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
