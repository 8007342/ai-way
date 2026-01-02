//! Animation System - Surface-Agnostic Animation Abstractions
//!
//! This module provides animation primitives that are independent of any
//! specific rendering surface. The Conductor uses these to describe
//! animations semantically, and surfaces translate them to their native
//! rendering systems.
//!
//! # Design Philosophy
//!
//! - **Frame-rate independent**: Animations use relative timing, not absolute ms
//! - **Composable**: Multiple animations can be layered and blended
//! - **Extensible**: New animation types without changing core protocol
//! - **Cacheable**: Designed for efficient frame caching and eviction
//!
//! # Architecture
//!
//! ```text
//! Conductor (owns AnimationSpec)
//!     │
//!     ├─→ TUI Surface (renders as ASCII sprites @ 10fps)
//!     ├─→ GUI Surface (renders as vector graphics @ 60fps)
//!     └─→ Web Surface (renders as CSS keyframes @ 60fps)
//! ```

mod timing;

pub use timing::{AnimationController, EasingFunction, FrameTiming};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Animation specification - surface-agnostic description
///
/// This describes WHAT an animation is, not HOW to render it.
/// Each surface interprets this spec according to its capabilities.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationSpec {
    /// Unique animation name (e.g., "idle", "thinking", "bounce")
    pub name: String,

    /// Number of frames in the animation
    pub frame_count: usize,

    /// Whether the animation loops
    pub looping: bool,

    /// Base playback rate in frames per second
    /// Surfaces may adjust based on their capabilities
    pub base_fps: f32,

    /// Emotional category for mood-aware timing
    pub emotional_category: Option<EmotionalCategory>,

    /// Per-frame timing (if not uniform)
    pub frame_timings: Option<Vec<FrameTiming>>,

    /// Animation priority for interruption handling
    pub priority: AnimationPriority,

    /// Whether this animation can be interrupted mid-playback
    pub interruptible: bool,

    /// Extensible properties for surface-specific hints
    pub properties: HashMap<String, serde_json::Value>,
}

impl AnimationSpec {
    /// Create a simple looping animation
    pub fn looping(name: impl Into<String>, frame_count: usize, base_fps: f32) -> Self {
        Self {
            name: name.into(),
            frame_count,
            looping: true,
            base_fps,
            emotional_category: None,
            frame_timings: None,
            priority: AnimationPriority::Normal,
            interruptible: true,
            properties: HashMap::new(),
        }
    }

    /// Create a one-shot animation (plays once)
    pub fn oneshot(name: impl Into<String>, frame_count: usize, base_fps: f32) -> Self {
        Self {
            name: name.into(),
            frame_count,
            looping: false,
            base_fps,
            emotional_category: None,
            frame_timings: None,
            priority: AnimationPriority::Normal,
            interruptible: true,
            properties: HashMap::new(),
        }
    }

    /// Set emotional category for mood-aware timing
    #[must_use]
    pub fn with_emotion(mut self, category: EmotionalCategory) -> Self {
        self.emotional_category = Some(category);
        self
    }

    /// Set animation priority
    #[must_use]
    pub fn with_priority(mut self, priority: AnimationPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Mark as non-interruptible
    #[must_use]
    pub fn non_interruptible(mut self) -> Self {
        self.interruptible = false;
        self
    }

    /// Add a custom property
    #[must_use]
    pub fn with_property(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.properties.insert(key.into(), value);
        self
    }

    /// Get duration at a given mood speed multiplier
    pub fn duration_at_speed(&self, speed_multiplier: f32) -> f32 {
        let base_duration = self.frame_count as f32 / self.base_fps;
        base_duration / speed_multiplier
    }
}

impl Default for AnimationSpec {
    fn default() -> Self {
        Self::looping("idle", 2, 10.0)
    }
}

/// Emotional categories for mood-aware animation timing
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmotionalCategory {
    /// Neutral, baseline animations
    Neutral,
    /// Happy, upbeat animations (faster)
    Joy,
    /// Excited, energetic animations (fastest)
    Excitement,
    /// Calm, relaxed animations (slower)
    Calm,
    /// Thinking, contemplative animations
    Contemplation,
    /// Confused, uncertain animations (with jitter)
    Confusion,
    /// Sad, subdued animations (slowest)
    Sadness,
    /// Error, distress animations
    Distress,
}

impl EmotionalCategory {
    /// Get speed multiplier for this emotional category
    #[must_use]
    pub fn speed_multiplier(self) -> f32 {
        match self {
            Self::Neutral => 1.0,
            Self::Joy => 1.2,
            Self::Excitement => 1.4,
            Self::Calm => 0.7,
            Self::Contemplation => 0.9,
            Self::Confusion => 1.0, // Normal speed but with jitter
            Self::Sadness => 0.6,
            Self::Distress => 1.1,
        }
    }

    /// Whether this category should add timing jitter
    #[must_use]
    pub fn has_jitter(self) -> bool {
        matches!(self, Self::Confusion)
    }

    /// Jitter amount (percentage of frame duration)
    #[must_use]
    pub fn jitter_amount(self) -> f32 {
        if self.has_jitter() {
            0.15 // 15% jitter
        } else {
            0.0
        }
    }
}

/// Animation priority for interruption handling
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum AnimationPriority {
    /// Background animations (idle, breathing)
    Low,
    /// Standard animations (gestures, reactions)
    #[default]
    Normal,
    /// Important animations (responses to user)
    High,
    /// Critical animations (errors, safety)
    Critical,
}

/// Blend mode for animation compositing
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BlendMode {
    /// Fully opaque, replaces underlying content
    #[default]
    Opaque,
    /// Alpha blending based on opacity
    Alpha,
    /// Additive blending (brightens)
    Add,
    /// Multiplicative blending (darkens)
    Multiply,
    /// Screen blending (lightens)
    Screen,
}

/// A layer in a composite animation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationLayer {
    /// Name of the animation to play
    pub animation_name: String,

    /// Offset from base position
    pub offset: (i16, i16),

    /// Layer opacity (0.0 = transparent, 1.0 = opaque)
    pub opacity: f32,

    /// How to blend with layers below
    pub blend_mode: BlendMode,

    /// Z-order (higher = on top)
    pub z_index: i32,
}

impl AnimationLayer {
    /// Create a new animation layer
    pub fn new(animation_name: impl Into<String>) -> Self {
        Self {
            animation_name: animation_name.into(),
            offset: (0, 0),
            opacity: 1.0,
            blend_mode: BlendMode::Opaque,
            z_index: 0,
        }
    }

    /// Set offset
    #[must_use]
    pub fn with_offset(mut self, x: i16, y: i16) -> Self {
        self.offset = (x, y);
        self
    }

    /// Set opacity
    #[must_use]
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set blend mode
    #[must_use]
    pub fn with_blend_mode(mut self, mode: BlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    /// Set z-index
    #[must_use]
    pub fn with_z_index(mut self, z: i32) -> Self {
        self.z_index = z;
        self
    }
}

/// Transition between two animations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationTransition {
    /// Source animation name
    pub from: String,

    /// Target animation name
    pub to: String,

    /// Transition duration in seconds
    pub duration_secs: f32,

    /// Easing function for the transition
    pub easing: EasingFunction,

    /// Transition type
    pub transition_type: TransitionType,
}

impl AnimationTransition {
    /// Create a crossfade transition
    pub fn crossfade(from: impl Into<String>, to: impl Into<String>, duration_secs: f32) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            duration_secs,
            easing: EasingFunction::EaseInOut,
            transition_type: TransitionType::Crossfade,
        }
    }

    /// Create an immediate cut transition
    pub fn cut(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            duration_secs: 0.0,
            easing: EasingFunction::Linear,
            transition_type: TransitionType::Cut,
        }
    }
}

/// Type of transition between animations
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TransitionType {
    /// Immediate switch (no blending)
    #[default]
    Cut,
    /// Crossfade between animations
    Crossfade,
    /// Fade out then fade in
    FadeThrough,
    /// Slide transition
    Slide,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_spec_creation() {
        let spec = AnimationSpec::looping("bounce", 4, 10.0)
            .with_emotion(EmotionalCategory::Joy)
            .with_priority(AnimationPriority::High);

        assert_eq!(spec.name, "bounce");
        assert_eq!(spec.frame_count, 4);
        assert!(spec.looping);
        assert_eq!(spec.emotional_category, Some(EmotionalCategory::Joy));
        assert_eq!(spec.priority, AnimationPriority::High);
    }

    #[test]
    fn test_emotional_speed_multipliers() {
        assert!((EmotionalCategory::Excitement.speed_multiplier() - 1.4).abs() < f32::EPSILON);
        assert!((EmotionalCategory::Calm.speed_multiplier() - 0.7).abs() < f32::EPSILON);
        assert!(EmotionalCategory::Confusion.has_jitter());
        assert!(!EmotionalCategory::Joy.has_jitter());
    }

    #[test]
    fn test_animation_layer() {
        let layer = AnimationLayer::new("overlay")
            .with_offset(5, -3)
            .with_opacity(0.8)
            .with_blend_mode(BlendMode::Alpha)
            .with_z_index(10);

        assert_eq!(layer.animation_name, "overlay");
        assert_eq!(layer.offset, (5, -3));
        assert!((layer.opacity - 0.8).abs() < f32::EPSILON);
        assert_eq!(layer.blend_mode, BlendMode::Alpha);
        assert_eq!(layer.z_index, 10);
    }

    #[test]
    fn test_animation_priority_ordering() {
        assert!(AnimationPriority::Critical > AnimationPriority::High);
        assert!(AnimationPriority::High > AnimationPriority::Normal);
        assert!(AnimationPriority::Normal > AnimationPriority::Low);
    }

    #[test]
    fn test_duration_at_speed() {
        let spec = AnimationSpec::looping("test", 10, 10.0); // 1 second at normal speed

        // Normal speed
        assert!((spec.duration_at_speed(1.0) - 1.0).abs() < 0.001);

        // Double speed = half duration
        assert!((spec.duration_at_speed(2.0) - 0.5).abs() < 0.001);

        // Half speed = double duration
        assert!((spec.duration_at_speed(0.5) - 2.0).abs() < 0.001);
    }
}
