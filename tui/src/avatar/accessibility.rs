//! Reduced-Motion Accessibility Mode (P2.6)
//!
//! Provides accessibility support for users who prefer reduced or no motion
//! in animations. This module detects user preferences from environment
//! variables and terminal capabilities, and provides an accessible animator
//! wrapper that respects these preferences.
//!
//! # Motion Preferences
//!
//! - `Full`: Normal animation playback
//! - `Reduced`: 0.25x animation speed for smoother, slower motion
//! - `None`: Static frame only, no animation ticks consumed
//!
//! # Environment Variable Detection
//!
//! Set `REDUCE_MOTION=1` to enable reduced motion mode:
//! - `REDUCE_MOTION=1` or `REDUCE_MOTION=reduced` -> `MotionPreference::Reduced`
//! - `REDUCE_MOTION=none` or `REDUCE_MOTION=static` -> `MotionPreference::None`
//! - Unset or other values -> `MotionPreference::Full`
//!
//! # Example
//!
//! ```ignore
//! use tui::avatar::accessibility::{MotionPreference, detect_motion_preference, AccessibleAnimator};
//! use tui::avatar::AvatarAnimator;
//!
//! let preference = detect_motion_preference();
//! let animator = AvatarAnimator::default_animator();
//! let mut accessible = AccessibleAnimator::new(animator, preference);
//!
//! // Tick will respect motion preference
//! accessible.tick(); // No CPU consumed if MotionPreference::None
//! ```

use std::env;
use std::time::Duration;

use conductor_core::avatar::Mood;

use super::animator::AvatarAnimator;
use super::sizes::AvatarSize;
use super::sprites::Frame;

/// User preference for motion and animation
///
/// Determines how animations are played back in the avatar system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MotionPreference {
    /// Full animation at normal speed
    #[default]
    Full,
    /// Reduced motion - animations play at 0.25x speed
    Reduced,
    /// No motion - static frame only, instant mood switches
    None,
}

impl MotionPreference {
    /// Speed multiplier for this preference
    ///
    /// - `Full`: 1.0 (normal speed)
    /// - `Reduced`: 0.25 (quarter speed)
    /// - `None`: 0.0 (no animation)
    #[must_use]
    pub fn speed_multiplier(&self) -> f32 {
        match self {
            MotionPreference::Full => 1.0,
            MotionPreference::Reduced => 0.25,
            MotionPreference::None => 0.0,
        }
    }

    /// Whether animations should play at all
    #[must_use]
    pub fn allows_animation(&self) -> bool {
        !matches!(self, MotionPreference::None)
    }

    /// Whether transitions should be animated
    ///
    /// In `None` mode, mood changes happen instantly without blending.
    #[must_use]
    pub fn allows_transitions(&self) -> bool {
        !matches!(self, MotionPreference::None)
    }
}

/// Detect user's motion preference from environment variables
///
/// Checks the `REDUCE_MOTION` environment variable:
/// - `1`, `true`, `yes`, `reduced` -> `MotionPreference::Reduced`
/// - `none`, `static`, `off` -> `MotionPreference::None`
/// - Unset or other values -> `MotionPreference::Full`
///
/// # Example
///
/// ```ignore
/// // With REDUCE_MOTION=1
/// let pref = detect_motion_preference();
/// assert_eq!(pref, MotionPreference::Reduced);
/// ```
#[must_use]
pub fn detect_motion_preference() -> MotionPreference {
    match env::var("REDUCE_MOTION") {
        Ok(value) => parse_motion_preference(&value),
        Err(_) => MotionPreference::Full,
    }
}

/// Parse a motion preference value string
///
/// Used internally by `detect_motion_preference()` but exposed for testing.
#[must_use]
pub fn parse_motion_preference(value: &str) -> MotionPreference {
    match value.to_lowercase().trim() {
        // Reduced motion values
        "1" | "true" | "yes" | "reduced" => MotionPreference::Reduced,
        // No motion values
        "none" | "static" | "off" | "2" => MotionPreference::None,
        // Everything else is full motion
        _ => MotionPreference::Full,
    }
}

/// Accessible animator wrapper that respects motion preferences
///
/// This wrapper around `AvatarAnimator` enforces motion preferences:
/// - In `Full` mode: Normal animation behavior
/// - In `Reduced` mode: Animations play at 0.25x speed
/// - In `None` mode: Always shows first frame, no CPU for ticks
///
/// # CPU Efficiency
///
/// When `MotionPreference::None` is set, `tick()` is a no-op that doesn't
/// perform any timing calculations or frame updates, minimizing CPU usage.
///
/// # Example
///
/// ```ignore
/// let mut accessible = AccessibleAnimator::new(animator, MotionPreference::None);
/// accessible.set_mood(Mood::Thinking); // Instant switch, no transition
/// accessible.tick(); // No-op, doesn't consume CPU
/// ```
#[derive(Debug)]
pub struct AccessibleAnimator {
    /// The underlying animator
    animator: AvatarAnimator,
    /// Current motion preference
    preference: MotionPreference,
    /// Whether we've applied the preference to the animator
    preference_applied: bool,
}

impl AccessibleAnimator {
    /// Create a new accessible animator with the given preference
    pub fn new(animator: AvatarAnimator, preference: MotionPreference) -> Self {
        let mut accessible = Self {
            animator,
            preference,
            preference_applied: false,
        };
        accessible.apply_preference();
        accessible
    }

    /// Create with auto-detected motion preference
    pub fn with_detected_preference(animator: AvatarAnimator) -> Self {
        Self::new(animator, detect_motion_preference())
    }

    /// Apply the current motion preference to the animator
    fn apply_preference(&mut self) {
        match self.preference {
            MotionPreference::Full => {
                self.animator.set_speed(1.0);
                self.animator.resume();
            }
            MotionPreference::Reduced => {
                // 0.25x speed for reduced motion
                self.animator.set_speed(0.25);
                self.animator.resume();
            }
            MotionPreference::None => {
                // Pause animation - we'll only show first frame
                self.animator.pause();
                // Reset to first frame by getting the animation name first
                let current_anim = self.animator.current_animation().to_string();
                self.animator.set_animation(&current_anim);
            }
        }
        self.preference_applied = true;
    }

    /// Set the motion preference
    ///
    /// This will immediately apply the new preference to the animator.
    pub fn set_motion_preference(&mut self, preference: MotionPreference) {
        if self.preference != preference {
            self.preference = preference;
            self.apply_preference();
        }
    }

    /// Get the current motion preference
    #[must_use]
    pub fn motion_preference(&self) -> MotionPreference {
        self.preference
    }

    /// Advance animation frame (respects motion preference)
    ///
    /// - `Full`/`Reduced`: Delegates to underlying animator
    /// - `None`: No-op, returns false immediately
    ///
    /// Returns `true` if the frame changed.
    pub fn tick(&mut self) -> bool {
        match self.preference {
            MotionPreference::None => {
                // Static mode: no tick processing, no CPU usage
                false
            }
            _ => self.animator.tick(),
        }
    }

    /// Change mood (respects motion preference)
    ///
    /// - `Full`/`Reduced`: Smooth transition (slower in Reduced mode)
    /// - `None`: Instant switch, no transition
    pub fn set_mood(&mut self, mood: Mood) {
        if self.preference == MotionPreference::None {
            // In static mode, we skip transitions and just set the mood directly
            // We need to update the animation but not trigger a transition
            self.animator.set_mood_with_animation(mood);
            // Clear any transition that might have been started
            self.clear_transition();
        } else {
            self.animator.set_mood(mood);
        }
    }

    /// Set mood and switch to appropriate animation
    pub fn set_mood_with_animation(&mut self, mood: Mood) {
        if self.preference == MotionPreference::None {
            self.animator.set_mood_with_animation(mood);
            self.clear_transition();
        } else {
            self.animator.set_mood_with_animation(mood);
        }
    }

    /// Clear any active mood transition (for static mode)
    fn clear_transition(&mut self) {
        // We access the animator's internal state to clear transitions
        // For static mode, we want instant switches - just mark dirty once
        if self.animator.is_transitioning() {
            // Mark the animator as dirty to force immediate redraw
            // The transition will be ignored in static mode since we don't tick
            self.animator.mark_dirty();
        }
    }

    /// Switch to a different animation
    pub fn set_animation(&mut self, name: &str) {
        self.animator.set_animation(name);
    }

    /// Set the avatar size
    pub fn set_size(&mut self, size: AvatarSize) {
        self.animator.set_size(size);
    }

    /// Get the current frame for rendering
    ///
    /// In static mode, this always returns the first frame of the current animation.
    #[must_use]
    pub fn current_frame(&self) -> Option<&Frame> {
        self.animator.current_frame()
    }

    /// Get the current animation name
    #[must_use]
    pub fn current_animation(&self) -> &str {
        self.animator.current_animation()
    }

    /// Get the current mood
    #[must_use]
    pub fn mood(&self) -> Mood {
        self.animator.mood()
    }

    /// Get the current frame index
    #[must_use]
    pub fn frame_index(&self) -> usize {
        self.animator.frame_index()
    }

    /// Get the current size
    #[must_use]
    pub fn size(&self) -> AvatarSize {
        self.animator.size()
    }

    /// Check if a mood transition is in progress
    ///
    /// Always returns `false` in static mode.
    #[must_use]
    pub fn is_transitioning(&self) -> bool {
        if self.preference == MotionPreference::None {
            false
        } else {
            self.animator.is_transitioning()
        }
    }

    /// Get time until next frame change
    ///
    /// In static mode, returns a very long duration to avoid waking up for animation.
    #[must_use]
    pub fn time_to_next_frame(&self) -> Duration {
        if self.preference == MotionPreference::None {
            // Static mode: very long sleep, we don't need animation ticks
            Duration::from_secs(3600) // 1 hour
        } else {
            self.animator.time_to_next_frame()
        }
    }

    /// Check if animation is paused
    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.animator.is_paused()
    }

    /// Pause animation playback
    pub fn pause(&mut self) {
        self.animator.pause();
    }

    /// Resume animation playback (respects motion preference)
    pub fn resume(&mut self) {
        if self.preference != MotionPreference::None {
            self.animator.resume();
        }
        // In static mode, resume is a no-op
    }

    /// Get whether the frame changed on the last tick
    #[must_use]
    pub fn did_frame_change(&self) -> bool {
        self.animator.did_frame_change()
    }

    /// Manually mark that a redraw is needed
    pub fn mark_dirty(&mut self) {
        self.animator.mark_dirty();
    }

    /// Get playback speed (adjusted for motion preference)
    #[must_use]
    pub fn speed(&self) -> f32 {
        self.animator.speed()
    }

    /// Set playback speed (will be adjusted by motion preference)
    ///
    /// In `Reduced` mode, the speed is capped at 0.25x.
    /// In `None` mode, speed changes are ignored.
    pub fn set_speed(&mut self, speed: f32) {
        match self.preference {
            MotionPreference::Full => self.animator.set_speed(speed),
            MotionPreference::Reduced => {
                // Cap at 0.25x for reduced motion
                self.animator.set_speed(speed.min(0.25));
            }
            MotionPreference::None => {
                // Ignore speed changes in static mode
            }
        }
    }

    /// Get immutable reference to the underlying animator
    #[must_use]
    pub fn animator(&self) -> &AvatarAnimator {
        &self.animator
    }

    /// Get mutable reference to the underlying animator
    ///
    /// Warning: Direct modifications may bypass motion preference enforcement.
    pub fn animator_mut(&mut self) -> &mut AvatarAnimator {
        &mut self.animator
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_motion_preference_speed_multiplier() {
        assert!((MotionPreference::Full.speed_multiplier() - 1.0).abs() < f32::EPSILON);
        assert!((MotionPreference::Reduced.speed_multiplier() - 0.25).abs() < f32::EPSILON);
        assert!((MotionPreference::None.speed_multiplier() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_motion_preference_allows_animation() {
        assert!(MotionPreference::Full.allows_animation());
        assert!(MotionPreference::Reduced.allows_animation());
        assert!(!MotionPreference::None.allows_animation());
    }

    #[test]
    fn test_motion_preference_allows_transitions() {
        assert!(MotionPreference::Full.allows_transitions());
        assert!(MotionPreference::Reduced.allows_transitions());
        assert!(!MotionPreference::None.allows_transitions());
    }

    #[test]
    fn test_parse_motion_preference_reduced() {
        assert_eq!(parse_motion_preference("1"), MotionPreference::Reduced);
        assert_eq!(parse_motion_preference("true"), MotionPreference::Reduced);
        assert_eq!(parse_motion_preference("TRUE"), MotionPreference::Reduced);
        assert_eq!(parse_motion_preference("yes"), MotionPreference::Reduced);
        assert_eq!(
            parse_motion_preference("reduced"),
            MotionPreference::Reduced
        );
        assert_eq!(
            parse_motion_preference("REDUCED"),
            MotionPreference::Reduced
        );
    }

    #[test]
    fn test_parse_motion_preference_none() {
        assert_eq!(parse_motion_preference("none"), MotionPreference::None);
        assert_eq!(parse_motion_preference("NONE"), MotionPreference::None);
        assert_eq!(parse_motion_preference("static"), MotionPreference::None);
        assert_eq!(parse_motion_preference("off"), MotionPreference::None);
        assert_eq!(parse_motion_preference("2"), MotionPreference::None);
    }

    #[test]
    fn test_parse_motion_preference_full() {
        assert_eq!(parse_motion_preference("0"), MotionPreference::Full);
        assert_eq!(parse_motion_preference("false"), MotionPreference::Full);
        assert_eq!(parse_motion_preference(""), MotionPreference::Full);
        assert_eq!(parse_motion_preference("full"), MotionPreference::Full);
        assert_eq!(parse_motion_preference("anything"), MotionPreference::Full);
    }

    #[test]
    fn test_detect_motion_preference_unset() {
        // When REDUCE_MOTION is not set, should return Full
        // Note: This test may fail if REDUCE_MOTION is set in the environment
        // In a real test environment, we'd want to use temp_env or similar
        env::remove_var("REDUCE_MOTION");
        let pref = detect_motion_preference();
        assert_eq!(pref, MotionPreference::Full);
    }

    #[test]
    fn test_accessible_animator_full_mode() {
        let animator = AvatarAnimator::default_animator();
        let accessible = AccessibleAnimator::new(animator, MotionPreference::Full);

        assert_eq!(accessible.motion_preference(), MotionPreference::Full);
        assert!(!accessible.is_paused());
        assert!((accessible.speed() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_accessible_animator_reduced_mode() {
        let animator = AvatarAnimator::default_animator();
        let accessible = AccessibleAnimator::new(animator, MotionPreference::Reduced);

        assert_eq!(accessible.motion_preference(), MotionPreference::Reduced);
        assert!(!accessible.is_paused());
        assert!((accessible.speed() - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn test_accessible_animator_none_mode() {
        let animator = AvatarAnimator::default_animator();
        let accessible = AccessibleAnimator::new(animator, MotionPreference::None);

        assert_eq!(accessible.motion_preference(), MotionPreference::None);
        assert!(accessible.is_paused());
    }

    #[test]
    fn test_accessible_animator_static_mode_tick() {
        let animator = AvatarAnimator::default_animator();
        let mut accessible = AccessibleAnimator::new(animator, MotionPreference::None);

        // Tick should be a no-op and return false
        let changed = accessible.tick();
        assert!(!changed);

        // Frame should stay at 0
        assert_eq!(accessible.frame_index(), 0);
    }

    #[test]
    fn test_accessible_animator_static_mode_no_cpu() {
        let animator = AvatarAnimator::default_animator();
        let mut accessible = AccessibleAnimator::new(animator, MotionPreference::None);

        // Measure time for many ticks - should be essentially instant
        let start = Instant::now();
        for _ in 0..10000 {
            accessible.tick();
        }
        let elapsed = start.elapsed();

        // Should complete in under 10ms for 10000 no-op ticks
        assert!(
            elapsed.as_millis() < 10,
            "Static mode tick took too long: {:?}",
            elapsed
        );
    }

    #[test]
    fn test_accessible_animator_static_mode_mood_change() {
        let animator = AvatarAnimator::default_animator();
        let mut accessible = AccessibleAnimator::new(animator, MotionPreference::None);

        // Mood change should be instant without transition
        accessible.set_mood_with_animation(Mood::Thinking);

        assert_eq!(accessible.mood(), Mood::Thinking);
        // In static mode, should not be transitioning
        assert!(!accessible.is_transitioning());
    }

    #[test]
    fn test_accessible_animator_static_mode_time_to_next_frame() {
        let animator = AvatarAnimator::default_animator();
        let accessible = AccessibleAnimator::new(animator, MotionPreference::None);

        // Should return very long duration to avoid waking up
        let time = accessible.time_to_next_frame();
        assert!(time >= Duration::from_secs(3600));
    }

    #[test]
    fn test_accessible_animator_change_preference() {
        let animator = AvatarAnimator::default_animator();
        let mut accessible = AccessibleAnimator::new(animator, MotionPreference::Full);

        assert_eq!(accessible.motion_preference(), MotionPreference::Full);
        assert!((accessible.speed() - 1.0).abs() < f32::EPSILON);

        // Change to reduced
        accessible.set_motion_preference(MotionPreference::Reduced);
        assert_eq!(accessible.motion_preference(), MotionPreference::Reduced);
        assert!((accessible.speed() - 0.25).abs() < f32::EPSILON);

        // Change to none
        accessible.set_motion_preference(MotionPreference::None);
        assert_eq!(accessible.motion_preference(), MotionPreference::None);
        assert!(accessible.is_paused());
    }

    #[test]
    fn test_accessible_animator_reduced_speed_cap() {
        let animator = AvatarAnimator::default_animator();
        let mut accessible = AccessibleAnimator::new(animator, MotionPreference::Reduced);

        // Try to set speed higher than 0.25x
        accessible.set_speed(2.0);

        // Should be capped at 0.25x
        assert!(accessible.speed() <= 0.25);
    }

    #[test]
    fn test_accessible_animator_none_ignores_speed() {
        let animator = AvatarAnimator::default_animator();
        let mut accessible = AccessibleAnimator::new(animator, MotionPreference::None);

        let original_speed = accessible.speed();

        // Try to change speed
        accessible.set_speed(2.0);

        // Speed should remain unchanged
        assert!((accessible.speed() - original_speed).abs() < f32::EPSILON);
    }

    #[test]
    fn test_accessible_animator_resume_respects_preference() {
        let animator = AvatarAnimator::default_animator();
        let mut accessible = AccessibleAnimator::new(animator, MotionPreference::None);

        // In static mode, resume should be a no-op
        accessible.resume();
        assert!(accessible.is_paused());

        // Switch to full mode
        accessible.set_motion_preference(MotionPreference::Full);
        accessible.pause();
        assert!(accessible.is_paused());

        // Now resume should work
        accessible.resume();
        assert!(!accessible.is_paused());
    }

    #[test]
    fn test_default_motion_preference() {
        assert_eq!(MotionPreference::default(), MotionPreference::Full);
    }
}
