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
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use super::accessibility::MotionPreference;
use super::sizes::{load_all_sprites, AvatarSize};
use super::sprites::{Frame, SpriteSheet};

/// Duration for mood transition blending (250ms is a good middle ground)
const MOOD_TRANSITION_DURATION_MS: u64 = 250;

/// Minimum frame duration to prevent CPU spinning
const MIN_FRAME_DURATION_MS: u64 = 16; // ~60fps cap

/// Default minimum jitter multiplier (85% of base duration)
const DEFAULT_JITTER_MIN: f32 = 0.85;

/// Default maximum jitter multiplier (115% of base duration)
const DEFAULT_JITTER_MAX: f32 = 1.15;

/// Timing jitter configuration for organic animation feel
///
/// This struct provides configurable timing variance to prevent animations
/// from feeling robotic. It uses a bell-curve distribution centered around
/// 1.0, making small variations more common than large ones.
///
/// # Distribution
///
/// The jitter uses a triangular distribution approximated by averaging
/// two uniform random values. This creates a bell-curve-like distribution
/// where values near the center (1.0) are more likely than extremes.
///
/// # Determinism
///
/// When created with a seed, the jitter sequence is deterministic,
/// which is essential for reproducible tests.
#[derive(Debug)]
pub struct TimingJitter {
    /// Minimum multiplier (e.g., 0.85 for 85% of base duration)
    min_multiplier: f32,
    /// Maximum multiplier (e.g., 1.15 for 115% of base duration)
    max_multiplier: f32,
    /// Whether jitter is enabled
    enabled: bool,
    /// Random number generator (seeded for reproducibility)
    rng: StdRng,
}

impl TimingJitter {
    /// Create a new TimingJitter with default settings (5-15% variance)
    pub fn new() -> Self {
        Self {
            min_multiplier: DEFAULT_JITTER_MIN,
            max_multiplier: DEFAULT_JITTER_MAX,
            enabled: true,
            rng: StdRng::from_entropy(),
        }
    }

    /// Create a new TimingJitter with a specific seed for reproducibility
    pub fn with_seed(seed: u64) -> Self {
        Self {
            min_multiplier: DEFAULT_JITTER_MIN,
            max_multiplier: DEFAULT_JITTER_MAX,
            enabled: true,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Create a TimingJitter with custom variance range
    ///
    /// # Arguments
    ///
    /// * `min_multiplier` - Minimum duration multiplier (e.g., 0.85)
    /// * `max_multiplier` - Maximum duration multiplier (e.g., 1.15)
    ///
    /// # Panics
    ///
    /// Panics if min_multiplier > max_multiplier or if either is <= 0
    pub fn with_range(min_multiplier: f32, max_multiplier: f32) -> Self {
        assert!(min_multiplier > 0.0, "min_multiplier must be positive");
        assert!(
            max_multiplier >= min_multiplier,
            "max_multiplier must be >= min_multiplier"
        );

        Self {
            min_multiplier,
            max_multiplier,
            enabled: true,
            rng: StdRng::from_entropy(),
        }
    }

    /// Create a TimingJitter with custom range and seed
    pub fn with_range_and_seed(min_multiplier: f32, max_multiplier: f32, seed: u64) -> Self {
        assert!(min_multiplier > 0.0, "min_multiplier must be positive");
        assert!(
            max_multiplier >= min_multiplier,
            "max_multiplier must be >= min_multiplier"
        );

        Self {
            min_multiplier,
            max_multiplier,
            enabled: true,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Enable or disable jitter
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if jitter is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the minimum multiplier
    pub fn min_multiplier(&self) -> f32 {
        self.min_multiplier
    }

    /// Get the maximum multiplier
    pub fn max_multiplier(&self) -> f32 {
        self.max_multiplier
    }

    /// Set the variance range
    ///
    /// # Arguments
    ///
    /// * `min_multiplier` - Minimum duration multiplier
    /// * `max_multiplier` - Maximum duration multiplier
    pub fn set_range(&mut self, min_multiplier: f32, max_multiplier: f32) {
        assert!(min_multiplier > 0.0, "min_multiplier must be positive");
        assert!(
            max_multiplier >= min_multiplier,
            "max_multiplier must be >= min_multiplier"
        );

        self.min_multiplier = min_multiplier;
        self.max_multiplier = max_multiplier;
    }

    /// Apply jitter to a base duration
    ///
    /// Returns the duration multiplied by a random factor within the
    /// configured range. Uses a bell-curve-like distribution where
    /// values near 1.0 are more likely than extremes.
    ///
    /// If jitter is disabled, returns the original duration unchanged.
    pub fn apply_jitter(&mut self, base_duration: Duration) -> Duration {
        if !self.enabled {
            return base_duration;
        }

        let multiplier = self.generate_bell_curve_multiplier();
        let jittered_ms = base_duration.as_millis() as f32 * multiplier;

        // Ensure we don't go below minimum frame duration
        Duration::from_millis((jittered_ms as u64).max(MIN_FRAME_DURATION_MS))
    }

    /// Generate a bell-curve distributed multiplier
    ///
    /// Uses the Irwin-Hall distribution (sum of uniform randoms) to
    /// approximate a normal distribution. Averaging two uniform values
    /// creates a triangular distribution centered at 0.5, which we then
    /// map to our desired range centered around 1.0.
    fn generate_bell_curve_multiplier(&mut self) -> f32 {
        // Generate triangular distribution by averaging two uniform samples
        // This creates a bell-curve-like distribution centered at 0.5
        let u1: f32 = self.rng.gen();
        let u2: f32 = self.rng.gen();
        let triangular = (u1 + u2) / 2.0; // Range [0, 1], peak at 0.5

        // Map from [0, 1] to [min_multiplier, max_multiplier]
        // with the peak at the center (typically 1.0)
        self.min_multiplier + triangular * (self.max_multiplier - self.min_multiplier)
    }

    /// Get the next multiplier without applying to a duration
    ///
    /// Useful for testing or debugging the distribution.
    pub fn next_multiplier(&mut self) -> f32 {
        if !self.enabled {
            return 1.0;
        }
        self.generate_bell_curve_multiplier()
    }
}

impl Default for TimingJitter {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TimingJitter {
    fn clone(&self) -> Self {
        // Clone creates a new RNG with fresh entropy
        // Use with_seed() for reproducible clones
        Self {
            min_multiplier: self.min_multiplier,
            max_multiplier: self.max_multiplier,
            enabled: self.enabled,
            rng: StdRng::from_entropy(),
        }
    }
}

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
/// - Micro-variation timing for organic animation feel
/// - Reduced-motion accessibility mode (P2.6)
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
    /// Timing jitter for organic animation feel
    jitter: TimingJitter,
    /// Whether timing jitter is enabled
    jitter_enabled: bool,
    /// Motion preference for accessibility (P2.6)
    motion_preference: MotionPreference,
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
            jitter: TimingJitter::new(),
            jitter_enabled: true,
            motion_preference: MotionPreference::Full,
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
            jitter: TimingJitter::new(),
            jitter_enabled: true,
            motion_preference: MotionPreference::Full,
        }
    }

    /// Create a new animator with a specific jitter seed for reproducibility
    ///
    /// This is useful for tests where deterministic timing is needed.
    pub fn with_jitter_seed(seed: u64) -> Self {
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
            jitter: TimingJitter::with_seed(seed),
            jitter_enabled: true,
            motion_preference: MotionPreference::Full,
        }
    }

    /// Create a new animator with a specific motion preference
    ///
    /// This is useful for accessibility support where reduced or no motion
    /// is preferred by the user.
    pub fn with_motion_preference(motion_preference: MotionPreference) -> Self {
        let sheets = load_all_sprites();

        let (speed, paused) = match motion_preference {
            MotionPreference::Full => (1.0, false),
            MotionPreference::Reduced => (0.25, false),
            MotionPreference::None => (1.0, true), // Paused for static mode
        };

        Self {
            sheets,
            current_animation: "idle".to_string(),
            current_frame: 0,
            frame_timer: Instant::now(),
            mood: Mood::default(),
            mood_transition: None,
            size: AvatarSize::Medium,
            paused,
            speed,
            frame_changed: false,
            jitter: TimingJitter::new(),
            jitter_enabled: motion_preference == MotionPreference::Full, // Disable jitter in reduced/none mode
            motion_preference,
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
    ///
    /// When jitter is enabled, frame durations are varied by 5-15% (configurable)
    /// using a bell-curve distribution for organic, non-robotic animations.
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
        let base_duration = Duration::from_millis(frame_duration_ms.max(MIN_FRAME_DURATION_MS));

        // Apply timing jitter for organic feel if enabled
        let frame_duration = if self.jitter_enabled {
            self.jitter.apply_jitter(base_duration)
        } else {
            base_duration
        };

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

    /// Enable or disable timing jitter
    ///
    /// When enabled, frame durations are varied by the configured amount
    /// (default 5-15%) for a more organic, natural feel.
    pub fn set_jitter_enabled(&mut self, enabled: bool) {
        self.jitter_enabled = enabled;
        self.jitter.set_enabled(enabled);
    }

    /// Check if timing jitter is enabled
    pub fn is_jitter_enabled(&self) -> bool {
        self.jitter_enabled
    }

    /// Set the jitter variance range
    ///
    /// # Arguments
    ///
    /// * `min_multiplier` - Minimum duration multiplier (e.g., 0.85 for 85%)
    /// * `max_multiplier` - Maximum duration multiplier (e.g., 1.15 for 115%)
    pub fn set_jitter_range(&mut self, min_multiplier: f32, max_multiplier: f32) {
        self.jitter.set_range(min_multiplier, max_multiplier);
    }

    /// Get a reference to the timing jitter configuration
    pub fn jitter(&self) -> &TimingJitter {
        &self.jitter
    }

    /// Get a mutable reference to the timing jitter configuration
    pub fn jitter_mut(&mut self) -> &mut TimingJitter {
        &mut self.jitter
    }

    /// Set the motion preference for accessibility
    ///
    /// This controls how animations are played:
    /// - `Full`: Normal animation at normal speed
    /// - `Reduced`: Animation at 0.25x speed, no jitter
    /// - `None`: Static frame only (paused), instant mood switches
    ///
    /// # Example
    ///
    /// ```ignore
    /// use tui::avatar::animator::AvatarAnimator;
    /// use tui::avatar::accessibility::MotionPreference;
    ///
    /// let mut animator = AvatarAnimator::default_animator();
    /// animator.set_motion_preference(MotionPreference::Reduced);
    /// assert_eq!(animator.speed(), 0.25);
    /// ```
    pub fn set_motion_preference(&mut self, preference: MotionPreference) {
        if self.motion_preference == preference {
            return;
        }

        self.motion_preference = preference;

        match preference {
            MotionPreference::Full => {
                self.speed = 1.0;
                self.paused = false;
                self.jitter_enabled = true;
                self.jitter.set_enabled(true);
            }
            MotionPreference::Reduced => {
                self.speed = 0.25;
                self.paused = false;
                self.jitter_enabled = false;
                self.jitter.set_enabled(false);
            }
            MotionPreference::None => {
                // Pause animation and reset to first frame
                self.paused = true;
                self.current_frame = 0;
                self.jitter_enabled = false;
                self.jitter.set_enabled(false);
                // Clear any active transition for instant mood switches
                self.mood_transition = None;
                self.frame_changed = true;
            }
        }
    }

    /// Get the current motion preference
    pub fn motion_preference(&self) -> MotionPreference {
        self.motion_preference
    }

    /// Set mood with motion preference awareness
    ///
    /// In `None` mode, mood changes happen instantly without transition.
    /// In other modes, uses the normal smooth transition.
    pub fn set_mood_accessible(&mut self, mood: Mood) {
        if self.motion_preference == MotionPreference::None {
            // In static mode, skip transition and just set mood directly
            self.mood = mood;
            self.mood_transition = None;
            // Switch animation but stay on first frame
            let animation = Self::animation_for_mood(mood);
            if self.current_animation != animation {
                self.current_animation = animation.to_string();
                self.current_frame = 0;
                self.frame_changed = true;
            }
        } else {
            self.set_mood(mood);
        }
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

    // =========================================================================
    // Motion Preference (Accessibility P2.6) Tests
    // =========================================================================

    #[test]
    fn test_motion_preference_default() {
        let animator = AvatarAnimator::default_animator();
        assert_eq!(animator.motion_preference(), MotionPreference::Full);
    }

    #[test]
    fn test_motion_preference_with_constructor() {
        let animator = AvatarAnimator::with_motion_preference(MotionPreference::Reduced);
        assert_eq!(animator.motion_preference(), MotionPreference::Reduced);
        assert!((animator.speed() - 0.25).abs() < f32::EPSILON);
        assert!(!animator.is_paused());
        assert!(!animator.is_jitter_enabled());
    }

    #[test]
    fn test_motion_preference_none_constructor() {
        let animator = AvatarAnimator::with_motion_preference(MotionPreference::None);
        assert_eq!(animator.motion_preference(), MotionPreference::None);
        assert!(animator.is_paused());
        assert!(!animator.is_jitter_enabled());
    }

    #[test]
    fn test_set_motion_preference_full() {
        let mut animator = AvatarAnimator::with_motion_preference(MotionPreference::None);
        animator.set_motion_preference(MotionPreference::Full);

        assert_eq!(animator.motion_preference(), MotionPreference::Full);
        assert!((animator.speed() - 1.0).abs() < f32::EPSILON);
        assert!(!animator.is_paused());
        assert!(animator.is_jitter_enabled());
    }

    #[test]
    fn test_set_motion_preference_reduced() {
        let mut animator = AvatarAnimator::default_animator();
        animator.set_motion_preference(MotionPreference::Reduced);

        assert_eq!(animator.motion_preference(), MotionPreference::Reduced);
        assert!((animator.speed() - 0.25).abs() < f32::EPSILON);
        assert!(!animator.is_paused());
        assert!(!animator.is_jitter_enabled());
    }

    #[test]
    fn test_set_motion_preference_none() {
        let mut animator = AvatarAnimator::default_animator();
        animator.set_animation("happy");
        animator.set_motion_preference(MotionPreference::None);

        assert_eq!(animator.motion_preference(), MotionPreference::None);
        assert!(animator.is_paused());
        assert_eq!(animator.frame_index(), 0); // Reset to first frame
        assert!(!animator.is_jitter_enabled());
    }

    #[test]
    fn test_set_motion_preference_noop_if_same() {
        let mut animator = AvatarAnimator::default_animator();
        animator.set_speed(2.0); // Modify speed

        animator.set_motion_preference(MotionPreference::Full); // Same as default

        // Speed should NOT be reset since preference didn't change
        assert!((animator.speed() - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_motion_preference_none_clears_transition() {
        let mut animator = AvatarAnimator::default_animator();
        animator.set_mood(Mood::Thinking); // Start a transition
        assert!(animator.is_transitioning());

        animator.set_motion_preference(MotionPreference::None);
        assert!(!animator.is_transitioning()); // Transition cleared
    }

    #[test]
    fn test_set_mood_accessible_in_none_mode() {
        let mut animator = AvatarAnimator::with_motion_preference(MotionPreference::None);
        animator.set_mood_accessible(Mood::Thinking);

        assert_eq!(animator.mood(), Mood::Thinking);
        assert!(!animator.is_transitioning()); // No transition in static mode
        assert_eq!(animator.current_animation(), "thinking");
        assert_eq!(animator.frame_index(), 0); // Still on first frame
    }

    #[test]
    fn test_set_mood_accessible_in_full_mode() {
        let mut animator = AvatarAnimator::default_animator();
        animator.set_mood_accessible(Mood::Thinking);

        assert_eq!(animator.mood(), Mood::Thinking);
        assert!(animator.is_transitioning()); // Normal transition in full mode
    }

    #[test]
    fn test_static_mode_tick_is_noop() {
        let mut animator = AvatarAnimator::with_motion_preference(MotionPreference::None);
        let frame_before = animator.frame_index();

        // Tick should not advance frames in static mode
        let changed = animator.tick();
        assert!(!changed);
        assert_eq!(animator.frame_index(), frame_before);
    }

    #[test]
    fn test_static_mode_time_to_next_frame() {
        let animator = AvatarAnimator::with_motion_preference(MotionPreference::None);

        // Should return long duration since we're paused
        let time = animator.time_to_next_frame();
        assert!(time >= Duration::from_secs(30));
    }

    // =========================================================================
    // TimingJitter Tests (P3.4 - Micro-Variation Timing)
    // =========================================================================

    #[test]
    fn test_timing_jitter_creation() {
        let jitter = TimingJitter::new();
        assert!(jitter.is_enabled());
        assert!((jitter.min_multiplier() - 0.85).abs() < 0.001);
        assert!((jitter.max_multiplier() - 1.15).abs() < 0.001);
    }

    #[test]
    fn test_timing_jitter_with_seed_reproducibility() {
        // Two jitters with the same seed should produce identical sequences
        let mut jitter1 = TimingJitter::with_seed(12345);
        let mut jitter2 = TimingJitter::with_seed(12345);

        for _ in 0..10 {
            let m1 = jitter1.next_multiplier();
            let m2 = jitter2.next_multiplier();
            assert!(
                (m1 - m2).abs() < 0.0001,
                "Seeded jitter should be reproducible"
            );
        }
    }

    #[test]
    fn test_timing_jitter_stays_within_bounds() {
        let mut jitter = TimingJitter::with_seed(42);

        // Generate many samples and verify all are within bounds
        for _ in 0..1000 {
            let multiplier = jitter.next_multiplier();
            assert!(
                multiplier >= 0.85,
                "Multiplier {} below min 0.85",
                multiplier
            );
            assert!(
                multiplier <= 1.15,
                "Multiplier {} above max 1.15",
                multiplier
            );
        }
    }

    #[test]
    fn test_timing_jitter_custom_range() {
        let mut jitter = TimingJitter::with_range_and_seed(0.90, 1.10, 99);

        for _ in 0..100 {
            let multiplier = jitter.next_multiplier();
            assert!(
                multiplier >= 0.90,
                "Multiplier {} below custom min 0.90",
                multiplier
            );
            assert!(
                multiplier <= 1.10,
                "Multiplier {} above custom max 1.10",
                multiplier
            );
        }
    }

    #[test]
    fn test_timing_jitter_distribution_centered() {
        let mut jitter = TimingJitter::with_seed(777);
        let mut sum = 0.0;
        let samples = 10000;

        for _ in 0..samples {
            sum += jitter.next_multiplier();
        }

        let mean = sum / samples as f32;
        // With triangular distribution centered at 1.0, mean should be very close to 1.0
        // (actually (0.85 + 1.15) / 2 = 1.0)
        assert!(
            (mean - 1.0).abs() < 0.02,
            "Mean {} should be close to 1.0 (centered distribution)",
            mean
        );
    }

    #[test]
    fn test_timing_jitter_can_be_disabled() {
        let mut jitter = TimingJitter::with_seed(123);
        jitter.set_enabled(false);

        // When disabled, should always return 1.0
        for _ in 0..10 {
            let multiplier = jitter.next_multiplier();
            assert!(
                (multiplier - 1.0).abs() < 0.0001,
                "Disabled jitter should return 1.0, got {}",
                multiplier
            );
        }
    }

    #[test]
    fn test_timing_jitter_apply_jitter() {
        let mut jitter = TimingJitter::with_seed(456);
        let base_duration = Duration::from_millis(100);

        // Apply jitter multiple times and verify range
        for _ in 0..100 {
            let jittered = jitter.apply_jitter(base_duration);
            let ms = jittered.as_millis() as f32;
            assert!(ms >= 85.0, "Jittered duration {}ms below 85ms", ms);
            assert!(ms <= 115.0, "Jittered duration {}ms above 115ms", ms);
        }
    }

    #[test]
    fn test_timing_jitter_respects_min_frame_duration() {
        let mut jitter = TimingJitter::with_seed(789);
        let tiny_duration = Duration::from_millis(10);

        // Even with jitter reducing duration, should not go below MIN_FRAME_DURATION_MS
        for _ in 0..50 {
            let jittered = jitter.apply_jitter(tiny_duration);
            assert!(
                jittered.as_millis() >= MIN_FRAME_DURATION_MS as u128,
                "Jittered duration should not go below minimum"
            );
        }
    }

    #[test]
    fn test_timing_jitter_disabled_returns_unchanged() {
        let mut jitter = TimingJitter::with_seed(111);
        jitter.set_enabled(false);
        let base_duration = Duration::from_millis(100);

        let result = jitter.apply_jitter(base_duration);
        assert_eq!(
            result, base_duration,
            "Disabled jitter should not modify duration"
        );
    }

    #[test]
    fn test_timing_jitter_set_range() {
        let mut jitter = TimingJitter::with_seed(222);
        jitter.set_range(0.80, 1.20);

        assert!((jitter.min_multiplier() - 0.80).abs() < 0.001);
        assert!((jitter.max_multiplier() - 1.20).abs() < 0.001);

        // Verify new range is respected
        for _ in 0..100 {
            let m = jitter.next_multiplier();
            assert!(m >= 0.80 && m <= 1.20);
        }
    }

    // =========================================================================
    // AvatarAnimator Jitter Integration Tests
    // =========================================================================

    #[test]
    fn test_animator_jitter_enabled_by_default() {
        let animator = AvatarAnimator::default_animator();
        assert!(animator.is_jitter_enabled());
    }

    #[test]
    fn test_animator_jitter_can_be_disabled() {
        let mut animator = AvatarAnimator::default_animator();
        animator.set_jitter_enabled(false);
        assert!(!animator.is_jitter_enabled());
    }

    #[test]
    fn test_animator_jitter_range_configurable() {
        let mut animator = AvatarAnimator::default_animator();
        animator.set_jitter_range(0.90, 1.10);

        let jitter = animator.jitter();
        assert!((jitter.min_multiplier() - 0.90).abs() < 0.001);
        assert!((jitter.max_multiplier() - 1.10).abs() < 0.001);
    }

    #[test]
    fn test_animator_with_jitter_seed() {
        let animator1 = AvatarAnimator::with_jitter_seed(54321);
        let animator2 = AvatarAnimator::with_jitter_seed(54321);

        // Both should have jitter enabled
        assert!(animator1.is_jitter_enabled());
        assert!(animator2.is_jitter_enabled());
    }

    #[test]
    fn test_animator_jitter_access() {
        let mut animator = AvatarAnimator::default_animator();

        // Test read access
        let _jitter = animator.jitter();

        // Test write access
        animator.jitter_mut().set_enabled(false);
        assert!(!animator.jitter().is_enabled());
    }
}
