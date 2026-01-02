//! Avatar Evolution System
//!
//! This module tracks Yollayah's evolution state based on user interaction patterns.
//! The avatar "grows" over time, unlocking visual enhancements and personality traits
//! as the user engages more deeply with the system.
//!
//! # Design Philosophy
//!
//! Evolution creates a sense of relationship and investment. As users interact with
//! Yollayah, the avatar evolves through distinct levels, each with unique visual
//! characteristics. This progression:
//!
//! 1. Rewards consistent engagement
//! 2. Creates emotional investment in the avatar
//! 3. Makes each user's Yollayah feel unique
//! 4. Provides a sense of growth and progress
//!
//! # Evolution Levels
//!
//! The avatar progresses through five distinct levels:
//!
//! | Level | Name | Interactions | Session Time | Visual Characteristics |
//! |-------|------|--------------|--------------|------------------------|
//! | 0 | Nascent | 0+ | 0h+ | Simple, single-color, basic shape |
//! | 1 | Developing | 50+ | 1h+ | Added details, secondary color |
//! | 2 | Mature | 200+ | 5h+ | Full color palette, smooth animations |
//! | 3 | Evolved | 500+ | 20h+ | Special effects, particle hints |
//! | 4 | Transcendent | 1000+ | 50h+ | Unique visual signature, glow effects |
//!
//! # Q4 Answer: Evolution Level Threshold Rationale
//!
//! ## Interaction Count Thresholds: 0 -> 50 -> 200 -> 500 -> 1000
//!
//! These thresholds are designed around typical usage patterns:
//!
//! - **Nascent (0)**: First-time users immediately see their avatar
//! - **Developing (50)**: ~1-2 weeks of light daily use (5-10 interactions/day)
//! - **Mature (200)**: ~1-2 months of regular use (5-10 interactions/day)
//! - **Evolved (500)**: ~3-4 months of consistent engagement
//! - **Transcendent (1000)**: Dedicated long-term users (~6+ months)
//!
//! The exponential growth ensures:
//! 1. Early levels feel achievable and rewarding
//! 2. Later levels feel special and earned
//! 3. Users always have something to work toward
//!
//! ## Session Time Thresholds: 0h -> 1h -> 5h -> 20h -> 50h
//!
//! Session time rewards depth of engagement:
//!
//! - **1 hour**: A few meaningful sessions
//! - **5 hours**: Regular user who spends time thinking with Yollayah
//! - **20 hours**: Power user who relies on Yollayah regularly
//! - **50 hours**: True companion relationship
//!
//! ## Dual-Threshold Design
//!
//! Both interaction count AND session time must meet the threshold. This prevents:
//! - Gaming via rapid meaningless interactions (must also spend time)
//! - Leaving sessions idle (must also interact)
//!
//! Either metric can trigger a level check, but both must be met for advancement.
//!
//! # Q5 Answer: Visual Markers per Evolution Level
//!
//! ## Nascent (Level 0)
//! - Single primary color (pink #FFB6C1)
//! - Basic axolotl silhouette
//! - Simple idle animation (2-3 frames)
//! - No accessories
//! - Minimal expression variation
//!
//! ## Developing (Level 1)
//! - Secondary color appears (coral accents)
//! - Gills become more detailed
//! - Eyes gain highlight reflections
//! - 2-3 animation variants per state
//! - Basic expression range (happy, thinking, idle)
//!
//! ## Mature (Level 2)
//! - Full color palette (pink, coral, white, dark accents)
//! - Smooth animation transitions (4-6 frames)
//! - Gradient shading on body
//! - Full expression range with subtle micro-expressions
//! - Occasional spontaneous gestures
//!
//! ## Evolved (Level 3)
//! - Subtle shimmer/sparkle effect on gills
//! - Particle hints during state transitions
//! - Soft glow outline
//! - Personalized color tint based on usage patterns
//! - Unique idle behaviors (yawning, stretching)
//!
//! ## Transcendent (Level 4)
//! - Ethereal glow effect (pulsing soft light)
//! - Constellation/star particles in background
//! - Iridescent color shifts
//! - Unique visual signature (personalized pattern)
//! - Full personality expression with rare "special" animations
//!
//! # Usage
//!
//! ```
//! use conductor_core::avatar::evolution::{EvolutionContext, EvolutionLevel};
//!
//! // Create a new evolution context for a new session
//! let mut ctx = EvolutionContext::new();
//! assert_eq!(ctx.current_level(), EvolutionLevel::Nascent);
//!
//! // Record interactions over time
//! for _ in 0..50 {
//!     if let Some(event) = ctx.record_interaction() {
//!         println!("Level up! {:?}", event);
//!     }
//! }
//!
//! // Add session time
//! ctx.add_session_time(3600); // 1 hour in seconds
//!
//! // Check current level (requires both thresholds)
//! println!("Current level: {:?}", ctx.current_level());
//! ```

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

// =============================================================================
// Evolution Level Thresholds (Q4 Answer)
// =============================================================================

/// Interaction count required for Developing level (50 interactions)
///
/// Rationale: ~1-2 weeks of light daily use (5-10 interactions/day)
pub const THRESHOLD_DEVELOPING_INTERACTIONS: u64 = 50;

/// Interaction count required for Mature level (200 interactions)
///
/// Rationale: ~1-2 months of regular use
pub const THRESHOLD_MATURE_INTERACTIONS: u64 = 200;

/// Interaction count required for Evolved level (500 interactions)
///
/// Rationale: ~3-4 months of consistent engagement
pub const THRESHOLD_EVOLVED_INTERACTIONS: u64 = 500;

/// Interaction count required for Transcendent level (1000 interactions)
///
/// Rationale: Dedicated long-term users (~6+ months)
pub const THRESHOLD_TRANSCENDENT_INTERACTIONS: u64 = 1000;

/// Session time required for Developing level (1 hour = 3600 seconds)
///
/// Rationale: A few meaningful sessions
pub const THRESHOLD_DEVELOPING_TIME_SECS: u64 = 3600;

/// Session time required for Mature level (5 hours = 18000 seconds)
///
/// Rationale: Regular user who spends quality time
pub const THRESHOLD_MATURE_TIME_SECS: u64 = 5 * 3600;

/// Session time required for Evolved level (20 hours = 72000 seconds)
///
/// Rationale: Power user who relies on Yollayah regularly
pub const THRESHOLD_EVOLVED_TIME_SECS: u64 = 20 * 3600;

/// Session time required for Transcendent level (50 hours = 180000 seconds)
///
/// Rationale: True companion relationship
pub const THRESHOLD_TRANSCENDENT_TIME_SECS: u64 = 50 * 3600;

// =============================================================================
// Evolution Level Enum
// =============================================================================

/// Evolution level of the avatar
///
/// Each level represents a stage of growth with distinct visual characteristics.
/// Progression requires meeting both interaction count AND session time thresholds.
///
/// # Visual Characteristics (Q5 Answer)
///
/// - **Nascent**: Simple, single-color, basic shape
/// - **Developing**: Added details, secondary color
/// - **Mature**: Full color palette, smooth animations
/// - **Evolved**: Special effects, particle hints
/// - **Transcendent**: Unique visual signature, glow effects
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub enum EvolutionLevel {
    /// Initial state - simple, single-color avatar
    ///
    /// Visual markers:
    /// - Single primary color (pink #FFB6C1)
    /// - Basic axolotl silhouette
    /// - Simple idle animation (2-3 frames)
    /// - No accessories
    /// - Minimal expression variation
    #[default]
    Nascent = 0,

    /// Early growth - added details and secondary color
    ///
    /// Unlocked at: 50 interactions AND 1 hour session time
    ///
    /// Visual markers:
    /// - Secondary color appears (coral accents)
    /// - Gills become more detailed
    /// - Eyes gain highlight reflections
    /// - 2-3 animation variants per state
    /// - Basic expression range (happy, thinking, idle)
    Developing = 1,

    /// Established presence - full visual palette
    ///
    /// Unlocked at: 200 interactions AND 5 hours session time
    ///
    /// Visual markers:
    /// - Full color palette (pink, coral, white, dark accents)
    /// - Smooth animation transitions (4-6 frames)
    /// - Gradient shading on body
    /// - Full expression range with subtle micro-expressions
    /// - Occasional spontaneous gestures
    Mature = 2,

    /// Advanced state - special effects unlocked
    ///
    /// Unlocked at: 500 interactions AND 20 hours session time
    ///
    /// Visual markers:
    /// - Subtle shimmer/sparkle effect on gills
    /// - Particle hints during state transitions
    /// - Soft glow outline
    /// - Personalized color tint based on usage patterns
    /// - Unique idle behaviors (yawning, stretching)
    Evolved = 3,

    /// Maximum evolution - unique visual signature
    ///
    /// Unlocked at: 1000 interactions AND 50 hours session time
    ///
    /// Visual markers:
    /// - Ethereal glow effect (pulsing soft light)
    /// - Constellation/star particles in background
    /// - Iridescent color shifts
    /// - Unique visual signature (personalized pattern)
    /// - Full personality expression with rare "special" animations
    Transcendent = 4,
}

impl EvolutionLevel {
    /// Get the numeric value of this level (0-4)
    #[must_use]
    pub const fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// Create an evolution level from a numeric value
    ///
    /// Values above 4 are clamped to Transcendent.
    #[must_use]
    pub const fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Nascent,
            1 => Self::Developing,
            2 => Self::Mature,
            3 => Self::Evolved,
            _ => Self::Transcendent,
        }
    }

    /// Get the next evolution level, if any
    ///
    /// Returns `None` if already at Transcendent.
    #[must_use]
    pub const fn next(&self) -> Option<Self> {
        match self {
            Self::Nascent => Some(Self::Developing),
            Self::Developing => Some(Self::Mature),
            Self::Mature => Some(Self::Evolved),
            Self::Evolved => Some(Self::Transcendent),
            Self::Transcendent => None,
        }
    }

    /// Get the previous evolution level, if any
    ///
    /// Returns `None` if already at Nascent.
    #[must_use]
    pub const fn previous(&self) -> Option<Self> {
        match self {
            Self::Nascent => None,
            Self::Developing => Some(Self::Nascent),
            Self::Mature => Some(Self::Developing),
            Self::Evolved => Some(Self::Mature),
            Self::Transcendent => Some(Self::Evolved),
        }
    }

    /// Get the interaction count threshold for this level
    #[must_use]
    pub const fn interaction_threshold(&self) -> u64 {
        match self {
            Self::Nascent => 0,
            Self::Developing => THRESHOLD_DEVELOPING_INTERACTIONS,
            Self::Mature => THRESHOLD_MATURE_INTERACTIONS,
            Self::Evolved => THRESHOLD_EVOLVED_INTERACTIONS,
            Self::Transcendent => THRESHOLD_TRANSCENDENT_INTERACTIONS,
        }
    }

    /// Get the session time threshold for this level (in seconds)
    #[must_use]
    pub const fn session_time_threshold_secs(&self) -> u64 {
        match self {
            Self::Nascent => 0,
            Self::Developing => THRESHOLD_DEVELOPING_TIME_SECS,
            Self::Mature => THRESHOLD_MATURE_TIME_SECS,
            Self::Evolved => THRESHOLD_EVOLVED_TIME_SECS,
            Self::Transcendent => THRESHOLD_TRANSCENDENT_TIME_SECS,
        }
    }

    /// Get a human-readable name for this level
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Nascent => "Nascent",
            Self::Developing => "Developing",
            Self::Mature => "Mature",
            Self::Evolved => "Evolved",
            Self::Transcendent => "Transcendent",
        }
    }

    /// Get a description of the visual characteristics at this level
    #[must_use]
    pub const fn visual_description(&self) -> &'static str {
        match self {
            Self::Nascent => "Simple, single-color avatar with basic animations",
            Self::Developing => "Added details with secondary color and eye highlights",
            Self::Mature => "Full color palette with smooth animations and expressions",
            Self::Evolved => "Special effects including sparkles and soft glow",
            Self::Transcendent => "Unique ethereal presence with personalized visual signature",
        }
    }

    /// Get the number of animation variants available at this level
    ///
    /// Higher levels have more animation variety for a "fresher" feel.
    #[must_use]
    pub const fn animation_variants(&self) -> u8 {
        match self {
            Self::Nascent => 1,
            Self::Developing => 2,
            Self::Mature => 3,
            Self::Evolved => 4,
            Self::Transcendent => 5,
        }
    }
}

impl std::fmt::Display for EvolutionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

// =============================================================================
// Evolution Event
// =============================================================================

/// Event emitted when the avatar evolves to a new level
///
/// This event can be used to trigger visual feedback (celebration animation,
/// notification, etc.) when the user achieves a new evolution level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionEvent {
    /// The previous evolution level
    pub from_level: EvolutionLevel,
    /// The new evolution level
    pub to_level: EvolutionLevel,
    /// Total interactions at time of evolution
    pub interaction_count: u64,
    /// Total session time at time of evolution (in seconds)
    pub session_time_secs: u64,
    /// Timestamp when the evolution occurred
    #[serde(with = "system_time_serde")]
    pub timestamp: SystemTime,
}

impl EvolutionEvent {
    /// Create a new evolution event
    #[must_use]
    pub fn new(
        from_level: EvolutionLevel,
        to_level: EvolutionLevel,
        interaction_count: u64,
        session_time_secs: u64,
    ) -> Self {
        Self {
            from_level,
            to_level,
            interaction_count,
            session_time_secs,
            timestamp: SystemTime::now(),
        }
    }

    /// Get the number of levels gained in this evolution
    #[must_use]
    pub fn levels_gained(&self) -> u8 {
        self.to_level
            .as_u8()
            .saturating_sub(self.from_level.as_u8())
    }
}

// =============================================================================
// Evolution Context
// =============================================================================

/// Tracks the evolution state of an avatar across a session
///
/// The evolution context maintains:
/// - Current evolution level
/// - Total interaction count
/// - Total session time
/// - Creation and last interaction timestamps
///
/// # Example
///
/// ```
/// use conductor_core::avatar::evolution::{EvolutionContext, EvolutionLevel};
///
/// let mut ctx = EvolutionContext::new();
///
/// // Simulate interactions
/// for _ in 0..50 {
///     ctx.record_interaction();
/// }
///
/// // Add session time (1 hour)
/// ctx.add_session_time(3600);
///
/// // Check if we've evolved
/// assert_eq!(ctx.current_level(), EvolutionLevel::Developing);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionContext {
    /// Current evolution level (cached for quick access)
    level: EvolutionLevel,
    /// Total number of interactions recorded
    interaction_count: u64,
    /// Total session time in seconds
    session_time_secs: u64,
    /// When this context was created
    #[serde(with = "system_time_serde")]
    created_at: SystemTime,
    /// When the last interaction occurred
    #[serde(with = "system_time_serde")]
    last_interaction: SystemTime,
}

impl EvolutionContext {
    /// Create a new evolution context with default values
    ///
    /// The context starts at Nascent level with zero interactions.
    #[must_use]
    pub fn new() -> Self {
        let now = SystemTime::now();
        Self {
            level: EvolutionLevel::Nascent,
            interaction_count: 0,
            session_time_secs: 0,
            created_at: now,
            last_interaction: now,
        }
    }

    /// Create an evolution context with specific values (for restoration)
    ///
    /// # Arguments
    ///
    /// * `interaction_count` - Total interactions to restore
    /// * `session_time_secs` - Total session time in seconds
    /// * `created_at` - When the original context was created
    ///
    /// The level is automatically calculated from the provided metrics.
    #[must_use]
    pub fn restore(interaction_count: u64, session_time_secs: u64, created_at: SystemTime) -> Self {
        let level = Self::calculate_level(interaction_count, session_time_secs);
        let now = SystemTime::now();
        Self {
            level,
            interaction_count,
            session_time_secs,
            created_at,
            last_interaction: now,
        }
    }

    /// Get the current evolution level
    #[must_use]
    pub fn current_level(&self) -> EvolutionLevel {
        self.level
    }

    /// Get the total interaction count
    #[must_use]
    pub fn interaction_count(&self) -> u64 {
        self.interaction_count
    }

    /// Get the total session time in seconds
    #[must_use]
    pub fn session_time_secs(&self) -> u64 {
        self.session_time_secs
    }

    /// Get the total session time as a Duration
    #[must_use]
    pub fn session_time(&self) -> Duration {
        Duration::from_secs(self.session_time_secs)
    }

    /// Get when this context was created
    #[must_use]
    pub fn created_at(&self) -> SystemTime {
        self.created_at
    }

    /// Get when the last interaction occurred
    #[must_use]
    pub fn last_interaction(&self) -> SystemTime {
        self.last_interaction
    }

    /// Record a new interaction
    ///
    /// Increments the interaction counter and checks for evolution.
    /// Returns an `EvolutionEvent` if the avatar leveled up.
    ///
    /// # Returns
    ///
    /// `Some(EvolutionEvent)` if this interaction triggered a level up,
    /// `None` otherwise.
    pub fn record_interaction(&mut self) -> Option<EvolutionEvent> {
        self.interaction_count = self.interaction_count.saturating_add(1);
        self.last_interaction = SystemTime::now();
        self.check_evolution()
    }

    /// Record multiple interactions at once
    ///
    /// Useful for batch imports or restoring state.
    ///
    /// # Returns
    ///
    /// `Some(EvolutionEvent)` if these interactions triggered a level up,
    /// `None` otherwise.
    pub fn record_interactions(&mut self, count: u64) -> Option<EvolutionEvent> {
        self.interaction_count = self.interaction_count.saturating_add(count);
        self.last_interaction = SystemTime::now();
        self.check_evolution()
    }

    /// Add session time in seconds
    ///
    /// # Returns
    ///
    /// `Some(EvolutionEvent)` if this time addition triggered a level up,
    /// `None` otherwise.
    pub fn add_session_time(&mut self, seconds: u64) -> Option<EvolutionEvent> {
        self.session_time_secs = self.session_time_secs.saturating_add(seconds);
        self.check_evolution()
    }

    /// Add session time as a Duration
    ///
    /// # Returns
    ///
    /// `Some(EvolutionEvent)` if this time addition triggered a level up,
    /// `None` otherwise.
    pub fn add_session_duration(&mut self, duration: Duration) -> Option<EvolutionEvent> {
        self.add_session_time(duration.as_secs())
    }

    /// Check if evolution to a new level has occurred
    ///
    /// Compares current metrics against thresholds and updates level if needed.
    ///
    /// # Returns
    ///
    /// `Some(EvolutionEvent)` if a level up occurred, `None` otherwise.
    pub fn check_evolution(&mut self) -> Option<EvolutionEvent> {
        let new_level = Self::calculate_level(self.interaction_count, self.session_time_secs);

        if new_level > self.level {
            let event = EvolutionEvent::new(
                self.level,
                new_level,
                self.interaction_count,
                self.session_time_secs,
            );
            self.level = new_level;
            Some(event)
        } else {
            None
        }
    }

    /// Calculate the evolution level for given metrics
    ///
    /// Both interaction count AND session time must meet the threshold.
    #[must_use]
    pub fn calculate_level(interaction_count: u64, session_time_secs: u64) -> EvolutionLevel {
        if interaction_count >= THRESHOLD_TRANSCENDENT_INTERACTIONS
            && session_time_secs >= THRESHOLD_TRANSCENDENT_TIME_SECS
        {
            EvolutionLevel::Transcendent
        } else if interaction_count >= THRESHOLD_EVOLVED_INTERACTIONS
            && session_time_secs >= THRESHOLD_EVOLVED_TIME_SECS
        {
            EvolutionLevel::Evolved
        } else if interaction_count >= THRESHOLD_MATURE_INTERACTIONS
            && session_time_secs >= THRESHOLD_MATURE_TIME_SECS
        {
            EvolutionLevel::Mature
        } else if interaction_count >= THRESHOLD_DEVELOPING_INTERACTIONS
            && session_time_secs >= THRESHOLD_DEVELOPING_TIME_SECS
        {
            EvolutionLevel::Developing
        } else {
            EvolutionLevel::Nascent
        }
    }

    /// Get progress toward the next level
    ///
    /// Returns `None` if already at Transcendent level.
    ///
    /// # Returns
    ///
    /// A tuple of (`interaction_progress`, `time_progress`) where each is 0.0-1.0.
    /// Both must reach 1.0 for level up.
    #[must_use]
    pub fn progress_to_next(&self) -> Option<EvolutionProgress> {
        let next_level = self.level.next()?;

        let current_interaction_threshold = self.level.interaction_threshold();
        let next_interaction_threshold = next_level.interaction_threshold();
        let interaction_range = next_interaction_threshold - current_interaction_threshold;

        let current_time_threshold = self.level.session_time_threshold_secs();
        let next_time_threshold = next_level.session_time_threshold_secs();
        let time_range = next_time_threshold - current_time_threshold;

        let interaction_progress = if interaction_range > 0 {
            let progress_in_range = self
                .interaction_count
                .saturating_sub(current_interaction_threshold);
            (progress_in_range as f64 / interaction_range as f64).min(1.0)
        } else {
            1.0
        };

        let time_progress = if time_range > 0 {
            let progress_in_range = self
                .session_time_secs
                .saturating_sub(current_time_threshold);
            (progress_in_range as f64 / time_range as f64).min(1.0)
        } else {
            1.0
        };

        Some(EvolutionProgress {
            target_level: next_level,
            interaction_progress,
            time_progress,
            interactions_needed: next_interaction_threshold.saturating_sub(self.interaction_count),
            time_needed_secs: next_time_threshold.saturating_sub(self.session_time_secs),
        })
    }

    /// Check if evolution is imminent (both metrics at 90%+ of threshold)
    #[must_use]
    pub fn is_evolution_imminent(&self) -> bool {
        if let Some(progress) = self.progress_to_next() {
            progress.interaction_progress >= 0.9 && progress.time_progress >= 0.9
        } else {
            false
        }
    }

    /// Reset evolution context to initial state
    ///
    /// This preserves the `created_at` timestamp but resets all other values.
    pub fn reset(&mut self) {
        let now = SystemTime::now();
        self.level = EvolutionLevel::Nascent;
        self.interaction_count = 0;
        self.session_time_secs = 0;
        self.last_interaction = now;
    }
}

impl Default for EvolutionContext {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Evolution Progress
// =============================================================================

/// Progress information toward the next evolution level
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvolutionProgress {
    /// The level being progressed toward
    pub target_level: EvolutionLevel,
    /// Progress based on interaction count (0.0-1.0)
    pub interaction_progress: f64,
    /// Progress based on session time (0.0-1.0)
    pub time_progress: f64,
    /// Number of interactions still needed
    pub interactions_needed: u64,
    /// Session time still needed in seconds
    pub time_needed_secs: u64,
}

impl EvolutionProgress {
    /// Get overall progress as the minimum of both metrics
    ///
    /// Both metrics must reach 1.0 for level up, so the minimum
    /// represents the limiting factor.
    #[must_use]
    pub fn overall_progress(&self) -> f64 {
        self.interaction_progress.min(self.time_progress)
    }

    /// Check if interaction threshold has been met
    #[must_use]
    pub fn interaction_met(&self) -> bool {
        self.interaction_progress >= 1.0
    }

    /// Check if time threshold has been met
    #[must_use]
    pub fn time_met(&self) -> bool {
        self.time_progress >= 1.0
    }

    /// Get the time needed as a Duration
    #[must_use]
    pub fn time_needed(&self) -> Duration {
        Duration::from_secs(self.time_needed_secs)
    }
}

// =============================================================================
// Evolution Callback
// =============================================================================

/// Callback type for evolution level changes
pub type EvolutionCallback = Box<dyn Fn(&EvolutionEvent) + Send + Sync>;

/// Manager for evolution callbacks
///
/// Allows registering multiple callbacks that are invoked when evolution occurs.
pub struct EvolutionCallbackManager {
    callbacks: Vec<EvolutionCallback>,
}

impl EvolutionCallbackManager {
    /// Create a new callback manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            callbacks: Vec::new(),
        }
    }

    /// Register a callback for evolution events
    ///
    /// Callbacks are invoked in registration order.
    pub fn on_evolution<F>(&mut self, callback: F)
    where
        F: Fn(&EvolutionEvent) + Send + Sync + 'static,
    {
        self.callbacks.push(Box::new(callback));
    }

    /// Notify all callbacks of an evolution event
    pub fn notify(&self, event: &EvolutionEvent) {
        for callback in &self.callbacks {
            callback(event);
        }
    }

    /// Get the number of registered callbacks
    #[must_use]
    pub fn callback_count(&self) -> usize {
        self.callbacks.len()
    }

    /// Clear all registered callbacks
    pub fn clear(&mut self) {
        self.callbacks.clear();
    }
}

impl Default for EvolutionCallbackManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for EvolutionCallbackManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvolutionCallbackManager")
            .field("callback_count", &self.callbacks.len())
            .finish()
    }
}

// =============================================================================
// Serde helpers for SystemTime
// =============================================================================

mod system_time_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_secs(secs))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // EvolutionLevel Tests
    // =========================================================================

    #[test]
    fn test_evolution_level_default() {
        assert_eq!(EvolutionLevel::default(), EvolutionLevel::Nascent);
    }

    #[test]
    fn test_evolution_level_ordering() {
        assert!(EvolutionLevel::Nascent < EvolutionLevel::Developing);
        assert!(EvolutionLevel::Developing < EvolutionLevel::Mature);
        assert!(EvolutionLevel::Mature < EvolutionLevel::Evolved);
        assert!(EvolutionLevel::Evolved < EvolutionLevel::Transcendent);
    }

    #[test]
    fn test_evolution_level_as_u8() {
        assert_eq!(EvolutionLevel::Nascent.as_u8(), 0);
        assert_eq!(EvolutionLevel::Developing.as_u8(), 1);
        assert_eq!(EvolutionLevel::Mature.as_u8(), 2);
        assert_eq!(EvolutionLevel::Evolved.as_u8(), 3);
        assert_eq!(EvolutionLevel::Transcendent.as_u8(), 4);
    }

    #[test]
    fn test_evolution_level_from_u8() {
        assert_eq!(EvolutionLevel::from_u8(0), EvolutionLevel::Nascent);
        assert_eq!(EvolutionLevel::from_u8(1), EvolutionLevel::Developing);
        assert_eq!(EvolutionLevel::from_u8(2), EvolutionLevel::Mature);
        assert_eq!(EvolutionLevel::from_u8(3), EvolutionLevel::Evolved);
        assert_eq!(EvolutionLevel::from_u8(4), EvolutionLevel::Transcendent);
        // Values above 4 clamp to Transcendent
        assert_eq!(EvolutionLevel::from_u8(5), EvolutionLevel::Transcendent);
        assert_eq!(EvolutionLevel::from_u8(255), EvolutionLevel::Transcendent);
    }

    #[test]
    fn test_evolution_level_next() {
        assert_eq!(
            EvolutionLevel::Nascent.next(),
            Some(EvolutionLevel::Developing)
        );
        assert_eq!(
            EvolutionLevel::Developing.next(),
            Some(EvolutionLevel::Mature)
        );
        assert_eq!(EvolutionLevel::Mature.next(), Some(EvolutionLevel::Evolved));
        assert_eq!(
            EvolutionLevel::Evolved.next(),
            Some(EvolutionLevel::Transcendent)
        );
        assert_eq!(EvolutionLevel::Transcendent.next(), None);
    }

    #[test]
    fn test_evolution_level_previous() {
        assert_eq!(EvolutionLevel::Nascent.previous(), None);
        assert_eq!(
            EvolutionLevel::Developing.previous(),
            Some(EvolutionLevel::Nascent)
        );
        assert_eq!(
            EvolutionLevel::Mature.previous(),
            Some(EvolutionLevel::Developing)
        );
        assert_eq!(
            EvolutionLevel::Evolved.previous(),
            Some(EvolutionLevel::Mature)
        );
        assert_eq!(
            EvolutionLevel::Transcendent.previous(),
            Some(EvolutionLevel::Evolved)
        );
    }

    #[test]
    fn test_evolution_level_thresholds() {
        assert_eq!(EvolutionLevel::Nascent.interaction_threshold(), 0);
        assert_eq!(EvolutionLevel::Developing.interaction_threshold(), 50);
        assert_eq!(EvolutionLevel::Mature.interaction_threshold(), 200);
        assert_eq!(EvolutionLevel::Evolved.interaction_threshold(), 500);
        assert_eq!(EvolutionLevel::Transcendent.interaction_threshold(), 1000);

        assert_eq!(EvolutionLevel::Nascent.session_time_threshold_secs(), 0);
        assert_eq!(
            EvolutionLevel::Developing.session_time_threshold_secs(),
            3600
        );
        assert_eq!(
            EvolutionLevel::Mature.session_time_threshold_secs(),
            5 * 3600
        );
        assert_eq!(
            EvolutionLevel::Evolved.session_time_threshold_secs(),
            20 * 3600
        );
        assert_eq!(
            EvolutionLevel::Transcendent.session_time_threshold_secs(),
            50 * 3600
        );
    }

    #[test]
    fn test_evolution_level_animation_variants() {
        assert_eq!(EvolutionLevel::Nascent.animation_variants(), 1);
        assert_eq!(EvolutionLevel::Developing.animation_variants(), 2);
        assert_eq!(EvolutionLevel::Mature.animation_variants(), 3);
        assert_eq!(EvolutionLevel::Evolved.animation_variants(), 4);
        assert_eq!(EvolutionLevel::Transcendent.animation_variants(), 5);
    }

    #[test]
    fn test_evolution_level_names() {
        assert_eq!(EvolutionLevel::Nascent.name(), "Nascent");
        assert_eq!(EvolutionLevel::Developing.name(), "Developing");
        assert_eq!(EvolutionLevel::Mature.name(), "Mature");
        assert_eq!(EvolutionLevel::Evolved.name(), "Evolved");
        assert_eq!(EvolutionLevel::Transcendent.name(), "Transcendent");
    }

    #[test]
    fn test_evolution_level_display() {
        assert_eq!(format!("{}", EvolutionLevel::Nascent), "Nascent");
        assert_eq!(format!("{}", EvolutionLevel::Transcendent), "Transcendent");
    }

    #[test]
    fn test_evolution_level_serialization() {
        let level = EvolutionLevel::Mature;
        let json = serde_json::to_string(&level).unwrap();
        let parsed: EvolutionLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, level);
    }

    // =========================================================================
    // EvolutionContext Tests
    // =========================================================================

    #[test]
    fn test_evolution_context_new() {
        let ctx = EvolutionContext::new();
        assert_eq!(ctx.current_level(), EvolutionLevel::Nascent);
        assert_eq!(ctx.interaction_count(), 0);
        assert_eq!(ctx.session_time_secs(), 0);
    }

    #[test]
    fn test_evolution_context_record_interaction() {
        let mut ctx = EvolutionContext::new();
        assert_eq!(ctx.interaction_count(), 0);

        ctx.record_interaction();
        assert_eq!(ctx.interaction_count(), 1);

        ctx.record_interaction();
        assert_eq!(ctx.interaction_count(), 2);
    }

    #[test]
    fn test_evolution_context_record_interactions_batch() {
        let mut ctx = EvolutionContext::new();
        ctx.record_interactions(100);
        assert_eq!(ctx.interaction_count(), 100);
    }

    #[test]
    fn test_evolution_context_add_session_time() {
        let mut ctx = EvolutionContext::new();
        ctx.add_session_time(3600);
        assert_eq!(ctx.session_time_secs(), 3600);
        assert_eq!(ctx.session_time(), Duration::from_secs(3600));
    }

    #[test]
    fn test_evolution_context_add_session_duration() {
        let mut ctx = EvolutionContext::new();
        ctx.add_session_duration(Duration::from_secs(7200));
        assert_eq!(ctx.session_time_secs(), 7200);
    }

    #[test]
    fn test_evolution_level_up_requires_both_thresholds() {
        let mut ctx = EvolutionContext::new();

        // Only interactions - should stay Nascent
        ctx.record_interactions(100);
        assert_eq!(ctx.current_level(), EvolutionLevel::Nascent);

        // Only time - should stay Nascent
        let mut ctx2 = EvolutionContext::new();
        ctx2.add_session_time(7200);
        assert_eq!(ctx2.current_level(), EvolutionLevel::Nascent);

        // Both - should level up to Developing
        ctx.add_session_time(3600);
        assert_eq!(ctx.current_level(), EvolutionLevel::Developing);
    }

    #[test]
    fn test_evolution_context_level_up_event() {
        let mut ctx = EvolutionContext::new();
        ctx.record_interactions(49);
        ctx.add_session_time(3600);
        assert!(ctx.current_level() == EvolutionLevel::Nascent);

        // This interaction should trigger level up
        let event = ctx.record_interaction();
        assert!(event.is_some());

        let event = event.unwrap();
        assert_eq!(event.from_level, EvolutionLevel::Nascent);
        assert_eq!(event.to_level, EvolutionLevel::Developing);
        assert_eq!(event.interaction_count, 50);
        assert_eq!(event.session_time_secs, 3600);
        assert_eq!(event.levels_gained(), 1);
    }

    #[test]
    fn test_evolution_context_multiple_level_ups() {
        let mut ctx = EvolutionContext::new();

        // Jump to Mature directly
        ctx.record_interactions(200);
        ctx.add_session_time(5 * 3600);

        assert_eq!(ctx.current_level(), EvolutionLevel::Mature);
    }

    #[test]
    fn test_evolution_context_transcendent() {
        let mut ctx = EvolutionContext::new();
        ctx.record_interactions(1000);
        ctx.add_session_time(50 * 3600);

        assert_eq!(ctx.current_level(), EvolutionLevel::Transcendent);
    }

    #[test]
    fn test_evolution_context_restore() {
        let created = SystemTime::now() - Duration::from_secs(86400);
        let ctx = EvolutionContext::restore(500, 20 * 3600, created);

        assert_eq!(ctx.current_level(), EvolutionLevel::Evolved);
        assert_eq!(ctx.interaction_count(), 500);
        assert_eq!(ctx.session_time_secs(), 20 * 3600);
        assert_eq!(ctx.created_at(), created);
    }

    #[test]
    fn test_evolution_context_reset() {
        let mut ctx = EvolutionContext::new();
        ctx.record_interactions(500);
        ctx.add_session_time(20 * 3600);

        ctx.reset();

        assert_eq!(ctx.current_level(), EvolutionLevel::Nascent);
        assert_eq!(ctx.interaction_count(), 0);
        assert_eq!(ctx.session_time_secs(), 0);
    }

    #[test]
    fn test_evolution_context_serialization() {
        let mut ctx = EvolutionContext::new();
        ctx.record_interactions(100);
        ctx.add_session_time(7200);

        let json = serde_json::to_string(&ctx).unwrap();
        let parsed: EvolutionContext = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.current_level(), ctx.current_level());
        assert_eq!(parsed.interaction_count(), ctx.interaction_count());
        assert_eq!(parsed.session_time_secs(), ctx.session_time_secs());
    }

    // =========================================================================
    // Evolution Progress Tests
    // =========================================================================

    #[test]
    fn test_evolution_progress_at_start() {
        let ctx = EvolutionContext::new();
        let progress = ctx.progress_to_next().unwrap();

        assert_eq!(progress.target_level, EvolutionLevel::Developing);
        assert_eq!(progress.interaction_progress, 0.0);
        assert_eq!(progress.time_progress, 0.0);
        assert_eq!(progress.interactions_needed, 50);
        assert_eq!(progress.time_needed_secs, 3600);
    }

    #[test]
    fn test_evolution_progress_partial() {
        let mut ctx = EvolutionContext::new();
        ctx.record_interactions(25); // 50% of 50
        ctx.add_session_time(1800); // 50% of 3600

        let progress = ctx.progress_to_next().unwrap();
        assert!((progress.interaction_progress - 0.5).abs() < f64::EPSILON);
        assert!((progress.time_progress - 0.5).abs() < f64::EPSILON);
        assert_eq!(progress.overall_progress(), 0.5);
    }

    #[test]
    fn test_evolution_progress_one_met() {
        let mut ctx = EvolutionContext::new();
        ctx.record_interactions(50); // 100% interactions
        ctx.add_session_time(1800); // 50% time

        let progress = ctx.progress_to_next().unwrap();
        assert!(progress.interaction_met());
        assert!(!progress.time_met());
        assert_eq!(progress.overall_progress(), 0.5);
    }

    #[test]
    fn test_evolution_progress_transcendent_none() {
        let mut ctx = EvolutionContext::new();
        ctx.record_interactions(1000);
        ctx.add_session_time(50 * 3600);

        assert!(ctx.progress_to_next().is_none());
    }

    #[test]
    fn test_evolution_imminent() {
        let mut ctx = EvolutionContext::new();
        ctx.record_interactions(45); // 90%
        ctx.add_session_time(3240); // 90%

        assert!(ctx.is_evolution_imminent());

        let mut ctx2 = EvolutionContext::new();
        ctx2.record_interactions(25); // 50%
        ctx2.add_session_time(1800); // 50%

        assert!(!ctx2.is_evolution_imminent());
    }

    // =========================================================================
    // EvolutionEvent Tests
    // =========================================================================

    #[test]
    fn test_evolution_event_creation() {
        let event = EvolutionEvent::new(
            EvolutionLevel::Nascent,
            EvolutionLevel::Developing,
            50,
            3600,
        );

        assert_eq!(event.from_level, EvolutionLevel::Nascent);
        assert_eq!(event.to_level, EvolutionLevel::Developing);
        assert_eq!(event.interaction_count, 50);
        assert_eq!(event.session_time_secs, 3600);
        assert_eq!(event.levels_gained(), 1);
    }

    #[test]
    fn test_evolution_event_multi_level_gain() {
        let event = EvolutionEvent::new(
            EvolutionLevel::Nascent,
            EvolutionLevel::Mature,
            200,
            5 * 3600,
        );

        assert_eq!(event.levels_gained(), 2);
    }

    #[test]
    fn test_evolution_event_serialization() {
        let event = EvolutionEvent::new(
            EvolutionLevel::Developing,
            EvolutionLevel::Mature,
            200,
            5 * 3600,
        );

        let json = serde_json::to_string(&event).unwrap();
        let parsed: EvolutionEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.from_level, event.from_level);
        assert_eq!(parsed.to_level, event.to_level);
        assert_eq!(parsed.interaction_count, event.interaction_count);
        assert_eq!(parsed.session_time_secs, event.session_time_secs);
    }

    // =========================================================================
    // EvolutionCallbackManager Tests
    // =========================================================================

    #[test]
    fn test_callback_manager_new() {
        let manager = EvolutionCallbackManager::new();
        assert_eq!(manager.callback_count(), 0);
    }

    #[test]
    fn test_callback_manager_register() {
        let mut manager = EvolutionCallbackManager::new();
        manager.on_evolution(|_| {});
        assert_eq!(manager.callback_count(), 1);

        manager.on_evolution(|_| {});
        assert_eq!(manager.callback_count(), 2);
    }

    #[test]
    fn test_callback_manager_notify() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let mut manager = EvolutionCallbackManager::new();
        manager.on_evolution(move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        let event = EvolutionEvent::new(
            EvolutionLevel::Nascent,
            EvolutionLevel::Developing,
            50,
            3600,
        );

        manager.notify(&event);
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        manager.notify(&event);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_callback_manager_clear() {
        let mut manager = EvolutionCallbackManager::new();
        manager.on_evolution(|_| {});
        manager.on_evolution(|_| {});
        assert_eq!(manager.callback_count(), 2);

        manager.clear();
        assert_eq!(manager.callback_count(), 0);
    }

    #[test]
    fn test_callback_manager_debug() {
        let manager = EvolutionCallbackManager::new();
        let debug_str = format!("{:?}", manager);
        assert!(debug_str.contains("EvolutionCallbackManager"));
        assert!(debug_str.contains("callback_count"));
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[test]
    fn test_overflow_protection() {
        let mut ctx = EvolutionContext::new();
        ctx.record_interactions(u64::MAX - 10);
        ctx.record_interactions(20);
        assert_eq!(ctx.interaction_count(), u64::MAX);
    }

    #[test]
    fn test_calculate_level_boundary_values() {
        // Just under Developing
        assert_eq!(
            EvolutionContext::calculate_level(49, 3600),
            EvolutionLevel::Nascent
        );
        assert_eq!(
            EvolutionContext::calculate_level(50, 3599),
            EvolutionLevel::Nascent
        );

        // Exactly at Developing
        assert_eq!(
            EvolutionContext::calculate_level(50, 3600),
            EvolutionLevel::Developing
        );

        // Interactions met but not time (all levels)
        assert_eq!(
            EvolutionContext::calculate_level(1000, 0),
            EvolutionLevel::Nascent
        );

        // Time met but not interactions (all levels)
        assert_eq!(
            EvolutionContext::calculate_level(0, 50 * 3600),
            EvolutionLevel::Nascent
        );
    }

    #[test]
    fn test_no_duplicate_level_up_events() {
        let mut ctx = EvolutionContext::new();
        ctx.record_interactions(50);
        ctx.add_session_time(3600);

        // First check should have triggered level up
        assert_eq!(ctx.current_level(), EvolutionLevel::Developing);

        // Subsequent record should not trigger another event
        let event = ctx.record_interaction();
        assert!(event.is_none());
    }
}
