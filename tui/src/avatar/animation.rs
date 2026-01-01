//! Animation Engine
//!
//! Manages animation playback, frame timing, and transitions.

use std::collections::HashMap;
use std::time::Duration;

use super::sizes::{load_all_sprites, AvatarSize};
use super::sprites::{Frame, SpriteSheet};

/// Engine that manages animation playback
pub struct AnimationEngine {
    /// Sprite sheets for each size
    sheets: HashMap<AvatarSize, SpriteSheet>,
    /// Current animation name
    current_animation: String,
    /// Current frame index
    current_frame: usize,
    /// Time accumulated on current frame
    frame_time: Duration,
    /// Playback speed multiplier
    speed: f32,
}

impl AnimationEngine {
    /// Create a new animation engine with all sprites loaded
    pub fn new() -> Self {
        Self {
            sheets: load_all_sprites(),
            current_animation: "idle".to_string(),
            current_frame: 0,
            frame_time: Duration::ZERO,
            speed: 1.0,
        }
    }

    /// Update animation state
    pub fn update(&mut self, delta: Duration, size: AvatarSize) {
        let sheet = match self.sheets.get(&size) {
            Some(s) => s,
            None => return,
        };

        let animation = match sheet.get(&self.current_animation) {
            Some(a) => a,
            None => return,
        };

        if animation.frames.is_empty() {
            return;
        }

        let frame = &animation.frames[self.current_frame];
        let frame_duration = Duration::from_millis((frame.duration_ms as f32 / self.speed) as u64);

        self.frame_time += delta;

        if self.frame_time >= frame_duration {
            self.frame_time = Duration::ZERO;
            self.current_frame += 1;

            if self.current_frame >= animation.frames.len() {
                if animation.looping {
                    self.current_frame = 0;
                } else {
                    // Stay on last frame
                    self.current_frame = animation.frames.len() - 1;
                }
            }
        }
    }

    /// Switch to a different animation
    pub fn play(&mut self, name: &str) {
        if self.current_animation != name {
            self.current_animation = name.to_string();
            self.current_frame = 0;
            self.frame_time = Duration::ZERO;
        }
    }

    /// Get the current frame for rendering
    pub fn current_frame(&self, size: AvatarSize) -> Option<&Frame> {
        let sheet = self.sheets.get(&size)?;
        let animation = sheet.get(&self.current_animation)?;
        animation.frames.get(self.current_frame)
    }

    /// Set playback speed (1.0 = normal)
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.clamp(0.1, 5.0);
    }

    /// Get current animation name
    pub fn current_animation(&self) -> &str {
        &self.current_animation
    }
}

impl Default for AnimationEngine {
    fn default() -> Self {
        Self::new()
    }
}
