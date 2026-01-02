//! Avatar Animator
//!
//! Manages animation state, frame timing, and smooth mood transitions.
//! This module implements the animation tick/update loop (P2.4) for
//! smooth, living animations in the terminal interface.
//!
//! # Design
//!
//! The animator uses `std::time::Instant` for frame timing rather than
//! ratatui's event loop, allowing precise control over animation speed
//! and smooth interpolation between frames.
//!
//! # Mood Transitions
//!
//! When the mood changes, the animator smoothly blends between the
//! current and target mood over 200-300ms, ensuring animations don't
//! feel jarring or interrupted mid-frame.

use std::time::{Duration, Instant};

use conductor_core::avatar::Mood;

use super::sizes::{load_all_sprites, AvatarSize};
use super::sprites::{Frame, SpriteSheet};

/// Duration for mood transition blending (250ms is a good middle ground)
const MOOD_TRANSITION_DURATION_MS: u64 = 250;

/// Minimum frame duration to prevent CPU spinning
const MIN_FRAME_DURATION_MS: u64 = 16; // ~60fps cap

/// Transition state for smooth mood changes
#[derive(Clone, Debug)]
pub struct MoodTransition {
    /// The mood we're transitioning from
    pub from_mood: Mood,
    /// The mood we're transitioning to
    pub to_mood: Mood,
    /// When the transition started
    pub start_time: Instant,
    /// Total duration of the transition
    pub duration: Duration,
}

impl MoodTransition {
    /// Create a new mood transition
    pub fn new(from: Mood, to: Mood) -> Self {
        Self {
            from_mood: from,
            to_mood: to,
            start_time: Instant::now(),
            duration: Duration::from_millis(MOOD_TRANSITION_DURATION_MS),
        }
    }

    /// Get the progress of the transition (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        let elapsed = self.start_time.elapsed();
        (elapsed.as_secs_f32() / self.duration.as_secs_f32()).min(1.0)
    }

    /// Check if the transition is complete
    pub fn is_complete(&self) -> bool {
        self.start_time.elapsed() >= self.duration
    }

    /// Get the blend factor for smooth easing (ease-in-out)
    pub fn blend_factor(&self) -> f32 {
        let t = self.progress();
        // Smooth ease-in-out: 3t^2 - 2t^3
        t * t * (3.0 - 2.0 * t)
    }
}

/// Avatar animator managing animation state and frame timing
///
/// This is the core animation loop for the avatar system.
/// It handles:
/// - Animation playback with precise frame timing
/// - Smooth mood transitions (blending over 200-300ms)
/// - Animation switching without jarring interruptions
#[derive(Debug)]
pub struct AvatarAnimator {
    /// Sprite sheets for all sizes (lazily loaded)
    sheets: std::collections::HashMap<AvatarSize, SpriteSheet>,
    /// Current animation name
    current_animation: String,
    /// Current frame index within the animation
    current_frame: usize,
    /// When the current frame started
    frame_timer: Instant,
    /// Current mood
    mood: Mood,
    /// Active mood transition (if any)
    mood_transition: Option<MoodTransition>,
    /// Current avatar size
    size: AvatarSize,
    /// Whether animation is paused
    paused: bool,
    /// Playback speed multiplier (1.0 = normal)
    speed: f32,
    /// Whether the frame changed on the last tick
    frame_changed: bool,
}

impl AvatarAnimator {
    /// Create a new animator with a sprite sheet
    ///
    /// # Arguments
    ///
    /// * `sprite_sheet` - Initial sprite sheet (typically for default size)
    ///
    /// The animator will load all size variants on creation for fast size switching.
    pub fn new(_sprite_sheet: SpriteSheet) -> Self {
        // Load all sprite sheets for different sizes
        let sheets = load_all_sprites();

        Self {
            sheets,
            current_animation: "idle".to_string(),
            current_frame: 0,
            frame_timer: Instant::now(),
            mood: Mood::default(),
            mood_transition: None,
            size: AvatarSize::Medium,
            paused: false,
            speed: 1.0,
            frame_changed: false,
        }
    }

    /// Create a new animator with default settings
    pub fn default_animator() -> Self {
        let sheets = load_all_sprites();

        Self {
            sheets,
            current_animation: "idle".to_string(),
            current_frame: 0,
            frame_timer: Instant::now(),
            mood: Mood::default(),
            mood_transition: None,
            size: AvatarSize::Medium,
            paused: false,
            speed: 1.0,
            frame_changed: false,
        }
    }

    /// Switch to a different animation
    ///
    /// This resets the frame counter and timer. If already playing the
    /// requested animation, this is a no-op.
    pub fn set_animation(&mut self, name: &str) {
        if self.current_animation != name {
            self.current_animation = name.to_string();
            self.current_frame = 0;
            self.frame_timer = Instant::now();
            self.frame_changed = true;
        }
    }

    /// Change mood with smooth transition
    ///
    /// This initiates a 200-300ms blend between the current mood and
    /// the new mood. The transition won't interrupt mid-animation frames.
    pub fn set_mood(&mut self, mood: Mood) {
        if self.mood != mood {
            // Start a transition from current to new mood
            self.mood_transition = Some(MoodTransition::new(self.mood, mood));
            self.mood = mood;
        }
    }

    /// Set the avatar size
    pub fn set_size(&mut self, size: AvatarSize) {
        if self.size != size {
            self.size = size;
            // Reset frame to prevent index out of bounds
            self.current_frame = 0;
            self.frame_timer = Instant::now();
            self.frame_changed = true;
        }
    }

    /// Advance frame if timer expired
    ///
    /// Returns `true` if the frame changed (useful for dirty-rect tracking).
    /// Call this regularly (e.g., every frame or on a timer tick).
    pub fn tick(&mut self) -> bool {
        self.frame_changed = false;

        if self.paused {
            return false;
        }

        // Update mood transition
        if let Some(ref transition) = self.mood_transition {
            if transition.is_complete() {
                self.mood_transition = None;
            }
        }

        // Get current animation
        let sheet = match self.sheets.get(&self.size) {
            Some(s) => s,
            None => return false,
        };

        let animation = match sheet.get(&self.current_animation) {
            Some(a) => a,
            None => return false,
        };

        if animation.frames.is_empty() {
            return false;
        }

        // Get current frame's duration
        let frame = &animation.frames[self.current_frame];
        let frame_duration_ms = (frame.duration_ms as f32 / self.speed) as u64;
        let frame_duration = Duration::from_millis(frame_duration_ms.max(MIN_FRAME_DURATION_MS));

        // Check if it's time to advance
        if self.frame_timer.elapsed() >= frame_duration {
            self.frame_timer = Instant::now();
            self.current_frame += 1;

            // Handle animation looping
            if self.current_frame >= animation.frames.len() {
                if animation.looping {
                    self.current_frame = 0;
                } else {
                    // Stay on last frame for non-looping animations
                    self.current_frame = animation.frames.len() - 1;
                }
            }

            self.frame_changed = true;
        }

        self.frame_changed
    }

    /// Get the current frame for rendering
    ///
    /// Returns the frame that should be displayed. This takes into
    /// account the current animation, frame index, and size.
    pub fn current_frame(&self) -> Option<&Frame> {
        let sheet = self.sheets.get(&self.size)?;
        let animation = sheet.get(&self.current_animation)?;
        animation.frames.get(self.current_frame)
    }

    /// Get the current animation name
    pub fn current_animation(&self) -> &str {
        &self.current_animation
    }

    /// Get the current mood
    pub fn mood(&self) -> Mood {
        self.mood
    }

    /// Get the current frame index
    pub fn frame_index(&self) -> usize {
        self.current_frame
    }

    /// Get the current size
    pub fn size(&self) -> AvatarSize {
        self.size
    }

    /// Check if a mood transition is in progress
    pub fn is_transitioning(&self) -> bool {
        self.mood_transition.is_some()
    }

    /// Get the mood transition blend factor (0.0 to 1.0)
    ///
    /// Returns 1.0 if no transition is active (fully at target mood).
    pub fn transition_blend(&self) -> f32 {
        self.mood_transition
            .as_ref()
            .map_or(1.0, |t| t.blend_factor())
    }

    /// Get the source mood during a transition
    pub fn transition_from_mood(&self) -> Option<Mood> {
        self.mood_transition.as_ref().map(|t| t.from_mood)
    }

    /// Pause animation playback
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resume animation playback
    pub fn resume(&mut self) {
        self.paused = false;
        self.frame_timer = Instant::now(); // Reset timer to prevent jump
    }

    /// Check if animation is paused
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Set playback speed (1.0 = normal, 2.0 = double speed)
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.clamp(0.1, 5.0);
    }

    /// Get current playback speed
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Get time until next frame change (for efficient event loops)
    pub fn time_to_next_frame(&self) -> Duration {
        if self.paused {
            return Duration::from_secs(60); // Long duration when paused
        }

        let sheet = match self.sheets.get(&self.size) {
            Some(s) => s,
            None => return Duration::from_millis(100),
        };

        let animation = match sheet.get(&self.current_animation) {
            Some(a) => a,
            None => return Duration::from_millis(100),
        };

        if animation.frames.is_empty() {
            return Duration::from_millis(100);
        }

        let frame = &animation.frames[self.current_frame];
        let frame_duration_ms = (frame.duration_ms as f32 / self.speed) as u64;
        let frame_duration = Duration::from_millis(frame_duration_ms.max(MIN_FRAME_DURATION_MS));

        frame_duration.saturating_sub(self.frame_timer.elapsed())
    }

    /// Get whether the frame changed on the last tick
    ///
    /// This is useful for dirty-rect tracking to know when to re-render.
    pub fn did_frame_change(&self) -> bool {
        self.frame_changed
    }

    /// Manually mark that a redraw is needed
    ///
    /// Useful when external state changes require a re-render.
    pub fn mark_dirty(&mut self) {
        self.frame_changed = true;
    }

    /// Get animation for a specific mood
    ///
    /// Maps moods to animation names following the avatar's expression system.
    pub fn animation_for_mood(mood: Mood) -> &'static str {
        match mood {
            Mood::Happy => "happy",
            Mood::Thinking => "thinking",
            Mood::Playful => "swimming",
            Mood::Shy => "idle",
            Mood::Excited => "happy",
            Mood::Confused => "error",
            Mood::Calm => "idle",
            Mood::Curious => "waiting",
            Mood::Sad => "idle",
            Mood::Focused => "thinking",
        }
    }

    /// Set mood and automatically switch to appropriate animation
    pub fn set_mood_with_animation(&mut self, mood: Mood) {
        self.set_mood(mood);
        let animation = Self::animation_for_mood(mood);
        self.set_animation(animation);
    }
}

impl Default for AvatarAnimator {
    fn default() -> Self {
        Self::default_animator()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_animator_creation() {
        let animator = AvatarAnimator::default_animator();
        assert_eq!(animator.current_animation(), "idle");
        assert_eq!(animator.mood(), Mood::Happy);
        assert!(!animator.is_paused());
    }

    #[test]
    fn test_set_animation() {
        let mut animator = AvatarAnimator::default_animator();
        animator.set_animation("happy");
        assert_eq!(animator.current_animation(), "happy");
        assert_eq!(animator.frame_index(), 0);
    }

    #[test]
    fn test_set_same_animation_noop() {
        let mut animator = AvatarAnimator::default_animator();
        animator.set_animation("idle");
        // Should not reset frame
        let frame_before = animator.frame_index();
        animator.set_animation("idle");
        assert_eq!(animator.frame_index(), frame_before);
    }

    #[test]
    fn test_mood_transition() {
        let mut animator = AvatarAnimator::default_animator();
        animator.set_mood(Mood::Thinking);

        assert!(animator.is_transitioning());
        assert_eq!(animator.mood(), Mood::Thinking);
        assert_eq!(animator.transition_from_mood(), Some(Mood::Happy));
    }

    #[test]
    fn test_mood_transition_completes() {
        let mut animator = AvatarAnimator::default_animator();
        animator.set_mood(Mood::Excited);

        // Wait for transition to complete
        thread::sleep(Duration::from_millis(300));
        animator.tick();

        assert!(!animator.is_transitioning());
        assert_eq!(animator.transition_blend(), 1.0);
    }

    #[test]
    fn test_pause_resume() {
        let mut animator = AvatarAnimator::default_animator();

        animator.pause();
        assert!(animator.is_paused());

        // Tick should not advance when paused
        let frame_before = animator.frame_index();
        animator.tick();
        assert_eq!(animator.frame_index(), frame_before);

        animator.resume();
        assert!(!animator.is_paused());
    }

    #[test]
    fn test_speed_clamping() {
        let mut animator = AvatarAnimator::default_animator();

        animator.set_speed(0.0);
        assert_eq!(animator.speed(), 0.1); // Clamped to minimum

        animator.set_speed(10.0);
        assert_eq!(animator.speed(), 5.0); // Clamped to maximum
    }

    #[test]
    fn test_set_size() {
        let mut animator = AvatarAnimator::default_animator();
        animator.set_size(AvatarSize::Large);
        assert_eq!(animator.size(), AvatarSize::Large);
    }

    #[test]
    fn test_animation_for_mood() {
        assert_eq!(AvatarAnimator::animation_for_mood(Mood::Happy), "happy");
        assert_eq!(
            AvatarAnimator::animation_for_mood(Mood::Thinking),
            "thinking"
        );
        assert_eq!(
            AvatarAnimator::animation_for_mood(Mood::Playful),
            "swimming"
        );
        assert_eq!(AvatarAnimator::animation_for_mood(Mood::Confused), "error");
    }

    #[test]
    fn test_mood_transition_easing() {
        let transition = MoodTransition::new(Mood::Happy, Mood::Sad);

        // At start, blend should be near 0
        let initial_blend = transition.blend_factor();
        assert!(initial_blend < 0.1);

        // Wait a bit
        thread::sleep(Duration::from_millis(125));
        let mid_blend = transition.blend_factor();
        assert!(mid_blend > 0.3 && mid_blend < 0.7);

        // Wait for completion
        thread::sleep(Duration::from_millis(150));
        let final_blend = transition.blend_factor();
        assert!(final_blend > 0.9);
    }

    #[test]
    fn test_frame_changed_tracking() {
        let mut animator = AvatarAnimator::default_animator();

        // Initially no change
        assert!(!animator.did_frame_change());

        // After set_animation, should be marked as changed
        animator.set_animation("happy");
        assert!(animator.did_frame_change());

        // After tick (without frame advance), should not be changed
        animator.tick();
        // Note: frame_changed is reset at start of tick
    }

    #[test]
    fn test_time_to_next_frame_when_paused() {
        let mut animator = AvatarAnimator::default_animator();
        animator.pause();

        let time = animator.time_to_next_frame();
        assert!(time >= Duration::from_secs(30)); // Long duration when paused
    }

    #[test]
    fn test_mark_dirty() {
        let mut animator = AvatarAnimator::default_animator();
        animator.tick(); // Reset frame_changed
        assert!(!animator.did_frame_change());

        animator.mark_dirty();
        assert!(animator.did_frame_change());
    }
}
