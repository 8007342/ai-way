//! Frame Timing Abstraction
//!
//! Provides frame-rate independent animation timing with easing functions.
//! This allows animations to play correctly at any target frame rate.

use serde::{Deserialize, Serialize};

/// Per-frame timing specification
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct FrameTiming {
    /// Relative duration (0.0-1.0 representing portion of total animation)
    /// If None, frame has uniform duration
    pub relative_duration: Option<f32>,

    /// Easing function for this frame's transition
    pub easing: EasingFunction,

    /// Whether this is a safe point to interrupt the animation
    pub interruptible: bool,
}

impl FrameTiming {
    /// Create uniform timing (equal duration frames)
    #[must_use]
    pub const fn uniform() -> Self {
        Self {
            relative_duration: None,
            easing: EasingFunction::Linear,
            interruptible: true,
        }
    }

    /// Create timing with specific relative duration
    #[must_use]
    pub const fn with_duration(relative: f32) -> Self {
        Self {
            relative_duration: Some(relative),
            easing: EasingFunction::Linear,
            interruptible: true,
        }
    }

    /// Set easing function
    #[must_use]
    pub const fn with_easing(mut self, easing: EasingFunction) -> Self {
        self.easing = easing;
        self
    }

    /// Mark as non-interruptible
    #[must_use]
    pub const fn non_interruptible(mut self) -> Self {
        self.interruptible = false;
        self
    }
}

impl Default for FrameTiming {
    fn default() -> Self {
        Self::uniform()
    }
}

/// Easing functions for smooth animation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EasingFunction {
    /// No easing (constant speed)
    #[default]
    Linear,

    /// Slow start, fast end
    EaseIn,

    /// Fast start, slow end
    EaseOut,

    /// Slow start and end
    EaseInOut,

    /// Quadratic ease in
    EaseInQuad,

    /// Quadratic ease out
    EaseOutQuad,

    /// Quadratic ease in and out
    EaseInOutQuad,

    /// Cubic ease in
    EaseInCubic,

    /// Cubic ease out
    EaseOutCubic,

    /// Cubic ease in and out
    EaseInOutCubic,

    /// Bounce effect at end
    EaseOutBounce,

    /// Elastic effect at end
    EaseOutElastic,

    /// Overshoot then settle
    EaseOutBack,
}

impl EasingFunction {
    /// Apply the easing function to a progress value (0.0 to 1.0)
    #[must_use]
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);

        match self {
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => 1.0 - (1.0 - t).powi(2),
            Self::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            Self::EaseInQuad => t * t,
            Self::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Self::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            Self::EaseInCubic => t * t * t,
            Self::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            Self::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Self::EaseOutBounce => {
                const N1: f32 = 7.5625;
                const D1: f32 = 2.75;

                if t < 1.0 / D1 {
                    N1 * t * t
                } else if t < 2.0 / D1 {
                    let t = t - 1.5 / D1;
                    N1 * t * t + 0.75
                } else if t < 2.5 / D1 {
                    let t = t - 2.25 / D1;
                    N1 * t * t + 0.9375
                } else {
                    let t = t - 2.625 / D1;
                    N1 * t * t + 0.984_375
                }
            }
            Self::EaseOutElastic => {
                if t == 0.0 {
                    0.0
                } else if (t - 1.0).abs() < f32::EPSILON {
                    1.0
                } else {
                    let c4 = (2.0 * std::f32::consts::PI) / 3.0;
                    2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
                }
            }
            Self::EaseOutBack => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                let t_minus_1 = t - 1.0;
                1.0 + c3 * t_minus_1.powi(3) + c1 * t_minus_1.powi(2)
            }
        }
    }
}

/// Animation playback controller
///
/// Manages animation state and timing in a frame-rate independent way.
#[derive(Clone, Debug)]
pub struct AnimationController {
    /// Current frame index
    current_frame: usize,

    /// Progress within current frame (0.0 to 1.0)
    frame_progress: f32,

    /// Total number of frames
    frame_count: usize,

    /// Base FPS for the animation
    base_fps: f32,

    /// Current playback speed multiplier
    speed: f32,

    /// Whether the animation is playing
    is_playing: bool,

    /// Whether the animation loops
    looping: bool,

    /// Whether the animation has completed (for non-looping)
    completed: bool,
}

impl AnimationController {
    /// Create a new controller for an animation
    #[must_use]
    pub fn new(frame_count: usize, base_fps: f32, looping: bool) -> Self {
        Self {
            current_frame: 0,
            frame_progress: 0.0,
            frame_count: frame_count.max(1),
            base_fps: base_fps.max(0.1),
            speed: 1.0,
            is_playing: false,
            looping,
            completed: false,
        }
    }

    /// Start playing the animation
    pub fn play(&mut self) {
        self.is_playing = true;
        self.completed = false;
    }

    /// Pause the animation
    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    /// Stop and reset the animation
    pub fn stop(&mut self) {
        self.is_playing = false;
        self.current_frame = 0;
        self.frame_progress = 0.0;
        self.completed = false;
    }

    /// Reset to beginning but keep playing state
    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.frame_progress = 0.0;
        self.completed = false;
    }

    /// Set playback speed (1.0 = normal)
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.max(0.01);
    }

    /// Get current playback speed
    #[must_use]
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Update the animation by delta time
    ///
    /// # Arguments
    /// * `delta_ms` - Time since last update in milliseconds
    ///
    /// # Returns
    /// * `true` if the frame changed
    pub fn update(&mut self, delta_ms: u32) -> bool {
        if !self.is_playing || self.completed {
            return false;
        }

        let frame_duration_ms = self.frame_duration_ms();
        let progress_delta = (delta_ms as f32) / frame_duration_ms;

        self.frame_progress += progress_delta;

        if self.frame_progress >= 1.0 {
            return self.advance_frame();
        }

        false
    }

    /// Advance to the next frame
    fn advance_frame(&mut self) -> bool {
        // Handle overflow progress
        while self.frame_progress >= 1.0 {
            self.frame_progress -= 1.0;
            self.current_frame += 1;

            if self.current_frame >= self.frame_count {
                if self.looping {
                    self.current_frame = 0;
                } else {
                    self.current_frame = self.frame_count - 1;
                    self.frame_progress = 1.0;
                    self.completed = true;
                    self.is_playing = false;
                    return true;
                }
            }
        }

        true
    }

    /// Get current frame duration in milliseconds
    #[must_use]
    pub fn frame_duration_ms(&self) -> f32 {
        (1000.0 / self.base_fps) / self.speed
    }

    /// Get current frame index
    #[must_use]
    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    /// Get progress within current frame (0.0 to 1.0)
    #[must_use]
    pub fn frame_progress(&self) -> f32 {
        self.frame_progress
    }

    /// Get overall animation progress (0.0 to 1.0)
    #[must_use]
    pub fn animation_progress(&self) -> f32 {
        if self.frame_count == 0 {
            return 0.0;
        }

        let frame_contribution = self.current_frame as f32 / self.frame_count as f32;
        let progress_contribution = self.frame_progress / self.frame_count as f32;

        (frame_contribution + progress_contribution).min(1.0)
    }

    /// Check if animation is playing
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// Check if animation has completed (non-looping only)
    #[must_use]
    pub fn is_completed(&self) -> bool {
        self.completed
    }

    /// Check if animation loops
    #[must_use]
    pub fn is_looping(&self) -> bool {
        self.looping
    }

    /// Get frame count
    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    /// Seek to a specific frame
    pub fn seek_to_frame(&mut self, frame: usize) {
        self.current_frame = frame.min(self.frame_count.saturating_sub(1));
        self.frame_progress = 0.0;
        if self.completed && frame < self.frame_count - 1 {
            self.completed = false;
        }
    }

    /// Seek to a specific progress (0.0 to 1.0)
    pub fn seek_to_progress(&mut self, progress: f32) {
        let progress = progress.clamp(0.0, 1.0);
        let total_frames = self.frame_count as f32;
        let target_frame = (progress * total_frames).floor() as usize;
        let frame_offset = (progress * total_frames) - target_frame as f32;

        self.current_frame = target_frame.min(self.frame_count.saturating_sub(1));
        self.frame_progress = frame_offset;

        if progress < 1.0 {
            self.completed = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_linear() {
        assert!((EasingFunction::Linear.apply(0.0)).abs() < f32::EPSILON);
        assert!((EasingFunction::Linear.apply(0.5) - 0.5).abs() < f32::EPSILON);
        assert!((EasingFunction::Linear.apply(1.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_easing_boundaries() {
        for easing in [
            EasingFunction::Linear,
            EasingFunction::EaseIn,
            EasingFunction::EaseOut,
            EasingFunction::EaseInOut,
            EasingFunction::EaseOutBounce,
        ] {
            // All easings should map 0 -> 0 and 1 -> 1
            assert!(
                easing.apply(0.0).abs() < 0.001,
                "{easing:?} at 0.0 = {}",
                easing.apply(0.0)
            );
            assert!(
                (easing.apply(1.0) - 1.0).abs() < 0.001,
                "{easing:?} at 1.0 = {}",
                easing.apply(1.0)
            );
        }
    }

    #[test]
    fn test_controller_basic() {
        let mut controller = AnimationController::new(4, 10.0, true);

        assert_eq!(controller.current_frame(), 0);
        assert!(!controller.is_playing());

        controller.play();
        assert!(controller.is_playing());

        // At 10fps, each frame is 100ms
        // Advance by 50ms - should be halfway through first frame
        controller.update(50);
        assert_eq!(controller.current_frame(), 0);
        assert!((controller.frame_progress() - 0.5).abs() < 0.01);

        // Advance by another 60ms - should be in second frame
        controller.update(60);
        assert_eq!(controller.current_frame(), 1);
    }

    #[test]
    fn test_controller_looping() {
        let mut controller = AnimationController::new(2, 10.0, true);
        controller.play();

        // At 10fps, each frame is 100ms. 2 frames = 200ms total
        // Advance by 250ms - should loop back to frame 0 with 50ms progress
        controller.update(250);
        assert_eq!(controller.current_frame(), 0);
        assert!((controller.frame_progress() - 0.5).abs() < 0.01);
        assert!(!controller.is_completed());
    }

    #[test]
    fn test_controller_non_looping() {
        let mut controller = AnimationController::new(2, 10.0, false);
        controller.play();

        // Advance past end
        controller.update(300);
        assert!(controller.is_completed());
        assert!(!controller.is_playing());
        assert_eq!(controller.current_frame(), 1); // Stays on last frame
    }

    #[test]
    fn test_controller_speed() {
        let mut controller = AnimationController::new(4, 10.0, true);
        controller.play();
        controller.set_speed(2.0);

        // At 2x speed, each frame is 50ms instead of 100ms
        controller.update(50);
        assert_eq!(controller.current_frame(), 1);
    }

    #[test]
    fn test_controller_seek() {
        let mut controller = AnimationController::new(10, 10.0, true);

        controller.seek_to_frame(5);
        assert_eq!(controller.current_frame(), 5);

        controller.seek_to_progress(0.25);
        assert_eq!(controller.current_frame(), 2);
    }

    #[test]
    fn test_animation_progress() {
        let mut controller = AnimationController::new(4, 10.0, true);
        controller.play();

        // At start
        assert!(controller.animation_progress() < 0.01);

        // After 2 full frames (200ms at 10fps)
        controller.update(200);
        assert!((controller.animation_progress() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_frame_timing() {
        let timing = FrameTiming::with_duration(0.5)
            .with_easing(EasingFunction::EaseOut)
            .non_interruptible();

        assert_eq!(timing.relative_duration, Some(0.5));
        assert_eq!(timing.easing, EasingFunction::EaseOut);
        assert!(!timing.interruptible);
    }
}
