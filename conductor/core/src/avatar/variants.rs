//! Animation Variants System (P3.3)
//!
//! This module provides animation variants that add visual variety to the avatar's
//! animations. Higher evolution levels unlock more variants, making the avatar feel
//! more dynamic and personalized over time.
//!
//! # Design Philosophy
//!
//! Animation variants create subtle differences in the same animation type, preventing
//! the avatar from feeling robotic or repetitive. Each variant maintains character
//! consistency while offering visual freshness:
//!
//! - Variants are subtle enough to feel like natural variation
//! - Higher evolution levels unlock more expressive variants
//! - Weighted random selection creates organic, non-uniform patterns
//! - All variants for a type share the same semantic meaning
//!
//! # Usage
//!
//! ```
//! use conductor_core::avatar::variants::{VariantRegistry, AnimationType};
//! use conductor_core::avatar::evolution::EvolutionLevel;
//!
//! let registry = VariantRegistry::new();
//!
//! // Select a variant based on evolution level
//! let variant = registry.select_variant(
//!     AnimationType::Idle,
//!     EvolutionLevel::Developing
//! );
//!
//! println!("Selected variant: {} - {}", variant.id, variant.name);
//! ```

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::evolution::EvolutionLevel;

// =============================================================================
// Animation Types
// =============================================================================

/// Types of animations that can have variants
///
/// Each animation type represents a distinct semantic action or state.
/// Variants within a type differ in visual presentation but maintain
/// the same meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnimationType {
    /// Idle/resting state - the default when not doing anything specific
    Idle,
    /// Thinking/processing - shown during computation or contemplation
    Thinking,
    /// Speaking/communicating - shown when generating or displaying text
    Speaking,
    /// Happy/celebrating - positive emotional expression
    Happy,
    /// Error/confused - something went wrong
    Error,
    /// Waiting/listening - awaiting user input
    Waiting,
    /// Swimming/playful - dynamic movement animation
    Swimming,
}

impl AnimationType {
    /// Get all animation types
    #[must_use]
    pub fn all() -> &'static [AnimationType] {
        &[
            AnimationType::Idle,
            AnimationType::Thinking,
            AnimationType::Speaking,
            AnimationType::Happy,
            AnimationType::Error,
            AnimationType::Waiting,
            AnimationType::Swimming,
        ]
    }

    /// Get the corresponding animation name for this type
    #[must_use]
    pub const fn animation_name(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Thinking => "thinking",
            Self::Speaking => "talking",
            Self::Happy => "happy",
            Self::Error => "error",
            Self::Waiting => "waiting",
            Self::Swimming => "swimming",
        }
    }
}

// =============================================================================
// Animation Variant
// =============================================================================

/// A single animation variant
///
/// Represents one possible way to display a particular animation type.
/// Each variant has:
/// - A unique identifier
/// - Human-readable name and description
/// - Weight for random selection (higher = more likely)
/// - Minimum evolution level required to unlock
/// - Optional visual modifiers (frame rate, color hints, etc.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationVariant {
    /// Unique identifier for this variant (e.g., "`idle_relaxed`")
    pub id: String,
    /// Human-readable name (e.g., "Relaxed Idle")
    pub name: String,
    /// Description of what makes this variant different
    pub description: String,
    /// Animation type this variant belongs to
    pub animation_type: AnimationType,
    /// Weight for random selection (1-100, default 50)
    ///
    /// Higher weights make this variant more likely to be selected.
    /// Useful for making some variants feel more "default" while
    /// others are rare treats.
    pub weight: u8,
    /// Minimum evolution level required to unlock this variant
    pub unlock_level: EvolutionLevel,
    /// Speed multiplier for this variant (1.0 = normal)
    ///
    /// Allows variants to feel more energetic (>1.0) or calm (<1.0).
    pub speed_modifier: f32,
    /// Optional color tint hint (for evolved avatars)
    ///
    /// Format: RGB hex string like "#FFB6C1" or None for default
    pub color_hint: Option<String>,
    /// Tags for categorization and filtering
    pub tags: Vec<String>,
}

impl AnimationVariant {
    /// Create a new animation variant with default values
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        animation_type: AnimationType,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            animation_type,
            weight: 50,
            unlock_level: EvolutionLevel::Nascent,
            speed_modifier: 1.0,
            color_hint: None,
            tags: Vec::new(),
        }
    }

    /// Set the description
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the weight for random selection
    #[must_use]
    pub fn with_weight(mut self, weight: u8) -> Self {
        self.weight = weight.clamp(1, 100);
        self
    }

    /// Set the minimum evolution level to unlock
    #[must_use]
    pub fn with_unlock_level(mut self, level: EvolutionLevel) -> Self {
        self.unlock_level = level;
        self
    }

    /// Set the speed modifier
    #[must_use]
    pub fn with_speed_modifier(mut self, speed: f32) -> Self {
        self.speed_modifier = speed.clamp(0.1, 3.0);
        self
    }

    /// Set the color hint
    #[must_use]
    pub fn with_color_hint(mut self, color: impl Into<String>) -> Self {
        self.color_hint = Some(color.into());
        self
    }

    /// Add a tag
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Check if this variant is available at the given evolution level
    #[must_use]
    pub fn is_available_at(&self, level: EvolutionLevel) -> bool {
        level >= self.unlock_level
    }
}

// =============================================================================
// Variant Registry
// =============================================================================

/// Registry of all animation variants
///
/// Stores and manages variants for all animation types. Provides
/// weighted random selection based on evolution level.
#[derive(Debug, Clone)]
pub struct VariantRegistry {
    /// Variants indexed by animation type
    variants: HashMap<AnimationType, Vec<AnimationVariant>>,
}

impl VariantRegistry {
    /// Create a new registry with default variants
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            variants: HashMap::new(),
        };
        registry.register_default_variants();
        registry
    }

    /// Create an empty registry (for testing or custom setups)
    #[must_use]
    pub fn empty() -> Self {
        Self {
            variants: HashMap::new(),
        }
    }

    /// Register a variant
    pub fn register(&mut self, variant: AnimationVariant) {
        self.variants
            .entry(variant.animation_type)
            .or_default()
            .push(variant);
    }

    /// Get all variants for an animation type
    #[must_use]
    pub fn get_variants(&self, animation_type: AnimationType) -> &[AnimationVariant] {
        self.variants
            .get(&animation_type)
            .map_or(&[], Vec::as_slice)
    }

    /// Get variants available at a specific evolution level
    #[must_use]
    pub fn get_available_variants(
        &self,
        animation_type: AnimationType,
        level: EvolutionLevel,
    ) -> Vec<&AnimationVariant> {
        self.variants
            .get(&animation_type)
            .map(|variants| {
                variants
                    .iter()
                    .filter(|v| v.is_available_at(level))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Select a random variant using weighted selection
    ///
    /// Only considers variants available at the given evolution level.
    /// Returns the base variant if no variants are registered.
    ///
    /// # Arguments
    ///
    /// * `animation_type` - The type of animation to select a variant for
    /// * `level` - Current evolution level (determines available variants)
    ///
    /// # Returns
    ///
    /// A reference to the selected variant, or a default if none available.
    #[must_use]
    pub fn select_variant(
        &self,
        animation_type: AnimationType,
        level: EvolutionLevel,
    ) -> AnimationVariant {
        let available = self.get_available_variants(animation_type, level);

        if available.is_empty() {
            // Return a basic default variant
            return AnimationVariant::new(
                format!("{}_default", animation_type.animation_name()),
                "Default",
                animation_type,
            );
        }

        // Calculate total weight
        let total_weight: u32 = available.iter().map(|v| u32::from(v.weight)).sum();

        if total_weight == 0 {
            // All weights are zero (shouldn't happen, but handle gracefully)
            return available[0].clone();
        }

        // Weighted random selection
        let mut rng = rand::thread_rng();
        let roll: u32 = rng.gen_range(0..total_weight);

        let mut cumulative = 0u32;
        for variant in &available {
            cumulative += u32::from(variant.weight);
            if roll < cumulative {
                return (*variant).clone();
            }
        }

        // Fallback (shouldn't reach here)
        available.first().copied().cloned().unwrap_or_else(|| {
            AnimationVariant::new(
                format!("{}_default", animation_type.animation_name()),
                "Default",
                animation_type,
            )
        })
    }

    /// Select a variant deterministically (for testing or reproducible behavior)
    ///
    /// Uses the provided seed to select consistently.
    #[must_use]
    pub fn select_variant_seeded(
        &self,
        animation_type: AnimationType,
        level: EvolutionLevel,
        seed: u64,
    ) -> AnimationVariant {
        let available = self.get_available_variants(animation_type, level);

        if available.is_empty() {
            return AnimationVariant::new(
                format!("{}_default", animation_type.animation_name()),
                "Default",
                animation_type,
            );
        }

        // Calculate total weight
        let total_weight: u32 = available.iter().map(|v| u32::from(v.weight)).sum();

        if total_weight == 0 {
            return available[0].clone();
        }

        // Seeded selection
        let roll = (seed % u64::from(total_weight)) as u32;

        let mut cumulative = 0u32;
        for variant in &available {
            cumulative += u32::from(variant.weight);
            if roll < cumulative {
                return (*variant).clone();
            }
        }

        // Fallback to first variant
        available[0].clone()
    }

    /// Get the count of variants for an animation type
    #[must_use]
    pub fn variant_count(&self, animation_type: AnimationType) -> usize {
        self.variants.get(&animation_type).map_or(0, Vec::len)
    }

    /// Get the count of variants available at a specific level
    #[must_use]
    pub fn available_variant_count(
        &self,
        animation_type: AnimationType,
        level: EvolutionLevel,
    ) -> usize {
        self.get_available_variants(animation_type, level).len()
    }

    /// Register the default set of variants
    fn register_default_variants(&mut self) {
        // Register Idle variants (3 variants as required)
        self.register_idle_variants();
        // Register Thinking variants (2 variants as required)
        self.register_thinking_variants();
        // Register Speaking variants (2 variants as required)
        self.register_speaking_variants();
        // Register additional variants for other types
        self.register_happy_variants();
        self.register_error_variants();
        self.register_waiting_variants();
        self.register_swimming_variants();
    }

    /// Register Idle animation variants (3 variants)
    fn register_idle_variants(&mut self) {
        // Variant 1: Relaxed Idle (base, available from Nascent)
        self.register(
            AnimationVariant::new("idle_relaxed", "Relaxed", AnimationType::Idle)
                .with_description("Calm, gentle breathing with minimal movement")
                .with_weight(60) // Most common
                .with_unlock_level(EvolutionLevel::Nascent)
                .with_speed_modifier(0.9)
                .with_tag("calm")
                .with_tag("default"),
        );

        // Variant 2: Alert Idle (unlocks at Developing)
        self.register(
            AnimationVariant::new("idle_alert", "Alert", AnimationType::Idle)
                .with_description("Slightly perked up, eyes more active, subtle head movements")
                .with_weight(35)
                .with_unlock_level(EvolutionLevel::Developing)
                .with_speed_modifier(1.1)
                .with_tag("attentive"),
        );

        // Variant 3: Dreamy Idle (unlocks at Mature)
        self.register(
            AnimationVariant::new("idle_dreamy", "Dreamy", AnimationType::Idle)
                .with_description("Slow, floaty movements with occasional slow blinks")
                .with_weight(25)
                .with_unlock_level(EvolutionLevel::Mature)
                .with_speed_modifier(0.7)
                .with_color_hint("#E8D4F0") // Soft lavender tint
                .with_tag("relaxed")
                .with_tag("ethereal"),
        );
    }

    /// Register Thinking animation variants (2 variants)
    fn register_thinking_variants(&mut self) {
        // Variant 1: Focused Thinking (base)
        self.register(
            AnimationVariant::new("thinking_focused", "Focused", AnimationType::Thinking)
                .with_description("Concentrated gaze, subtle frown, steady processing")
                .with_weight(55)
                .with_unlock_level(EvolutionLevel::Nascent)
                .with_speed_modifier(1.0)
                .with_tag("intense")
                .with_tag("default"),
        );

        // Variant 2: Pondering Thinking (unlocks at Developing)
        self.register(
            AnimationVariant::new("thinking_pondering", "Pondering", AnimationType::Thinking)
                .with_description("Tilted head, eyes wandering, more whimsical contemplation")
                .with_weight(45)
                .with_unlock_level(EvolutionLevel::Developing)
                .with_speed_modifier(0.85)
                .with_tag("curious")
                .with_tag("playful"),
        );
    }

    /// Register Speaking animation variants (2 variants)
    fn register_speaking_variants(&mut self) {
        // Variant 1: Conversational Speaking (base)
        self.register(
            AnimationVariant::new(
                "speaking_conversational",
                "Conversational",
                AnimationType::Speaking,
            )
            .with_description("Natural mouth movements, friendly expression, steady pace")
            .with_weight(60)
            .with_unlock_level(EvolutionLevel::Nascent)
            .with_speed_modifier(1.0)
            .with_tag("friendly")
            .with_tag("default"),
        );

        // Variant 2: Expressive Speaking (unlocks at Developing)
        self.register(
            AnimationVariant::new("speaking_expressive", "Expressive", AnimationType::Speaking)
                .with_description("More animated mouth, occasional eyebrow raises, dynamic pace")
                .with_weight(40)
                .with_unlock_level(EvolutionLevel::Developing)
                .with_speed_modifier(1.15)
                .with_tag("animated")
                .with_tag("enthusiastic"),
        );
    }

    /// Register Happy animation variants
    fn register_happy_variants(&mut self) {
        // Variant 1: Gentle Happy (base)
        self.register(
            AnimationVariant::new("happy_gentle", "Gentle Smile", AnimationType::Happy)
                .with_description("Soft smile, warm eyes, content expression")
                .with_weight(50)
                .with_unlock_level(EvolutionLevel::Nascent)
                .with_speed_modifier(0.95)
                .with_tag("warm")
                .with_tag("default"),
        );

        // Variant 2: Beaming Happy (unlocks at Developing)
        self.register(
            AnimationVariant::new("happy_beaming", "Beaming", AnimationType::Happy)
                .with_description("Wide smile, sparkling eyes, joyful bouncing")
                .with_weight(35)
                .with_unlock_level(EvolutionLevel::Developing)
                .with_speed_modifier(1.2)
                .with_tag("excited")
                .with_tag("joyful"),
        );

        // Variant 3: Radiant Happy (unlocks at Evolved)
        self.register(
            AnimationVariant::new("happy_radiant", "Radiant", AnimationType::Happy)
                .with_description("Full joy expression with subtle sparkle effects")
                .with_weight(20)
                .with_unlock_level(EvolutionLevel::Evolved)
                .with_speed_modifier(1.1)
                .with_color_hint("#FFF0F5") // Lavender blush
                .with_tag("special")
                .with_tag("evolved"),
        );
    }

    /// Register Error animation variants
    fn register_error_variants(&mut self) {
        // Variant 1: Confused Error (base)
        self.register(
            AnimationVariant::new("error_confused", "Confused", AnimationType::Error)
                .with_description("Puzzled expression, tilted head, question marks")
                .with_weight(55)
                .with_unlock_level(EvolutionLevel::Nascent)
                .with_speed_modifier(1.0)
                .with_tag("default"),
        );

        // Variant 2: Concerned Error (unlocks at Developing)
        self.register(
            AnimationVariant::new("error_concerned", "Concerned", AnimationType::Error)
                .with_description("Worried expression, apologetic body language")
                .with_weight(45)
                .with_unlock_level(EvolutionLevel::Developing)
                .with_speed_modifier(0.9)
                .with_tag("apologetic"),
        );
    }

    /// Register Waiting animation variants
    fn register_waiting_variants(&mut self) {
        // Variant 1: Patient Waiting (base)
        self.register(
            AnimationVariant::new("waiting_patient", "Patient", AnimationType::Waiting)
                .with_description("Calm waiting posture, occasional slow blink")
                .with_weight(55)
                .with_unlock_level(EvolutionLevel::Nascent)
                .with_speed_modifier(0.85)
                .with_tag("calm")
                .with_tag("default"),
        );

        // Variant 2: Eager Waiting (unlocks at Developing)
        self.register(
            AnimationVariant::new("waiting_eager", "Eager", AnimationType::Waiting)
                .with_description("Attentive posture, subtle anticipation movements")
                .with_weight(45)
                .with_unlock_level(EvolutionLevel::Developing)
                .with_speed_modifier(1.1)
                .with_tag("excited")
                .with_tag("attentive"),
        );
    }

    /// Register Swimming animation variants
    fn register_swimming_variants(&mut self) {
        // Variant 1: Leisurely Swimming (base)
        self.register(
            AnimationVariant::new("swimming_leisurely", "Leisurely", AnimationType::Swimming)
                .with_description("Gentle, flowing movements at a relaxed pace")
                .with_weight(50)
                .with_unlock_level(EvolutionLevel::Nascent)
                .with_speed_modifier(0.8)
                .with_tag("relaxed")
                .with_tag("default"),
        );

        // Variant 2: Playful Swimming (unlocks at Developing)
        self.register(
            AnimationVariant::new("swimming_playful", "Playful", AnimationType::Swimming)
                .with_description("Energetic darting movements with occasional spins")
                .with_weight(35)
                .with_unlock_level(EvolutionLevel::Developing)
                .with_speed_modifier(1.3)
                .with_tag("energetic")
                .with_tag("fun"),
        );

        // Variant 3: Graceful Swimming (unlocks at Mature)
        self.register(
            AnimationVariant::new("swimming_graceful", "Graceful", AnimationType::Swimming)
                .with_description("Elegant, flowing movements with smooth transitions")
                .with_weight(25)
                .with_unlock_level(EvolutionLevel::Mature)
                .with_speed_modifier(0.95)
                .with_color_hint("#B0E0E6") // Powder blue
                .with_tag("elegant")
                .with_tag("mature"),
        );
    }
}

impl Default for VariantRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Variant Selection Helper
// =============================================================================

/// Helper function to select a variant for an animation type
///
/// Convenience function that creates a temporary registry and selects a variant.
/// For production use, prefer creating a `VariantRegistry` once and reusing it.
#[must_use]
pub fn select_variant(animation_type: AnimationType, level: EvolutionLevel) -> AnimationVariant {
    let registry = VariantRegistry::new();
    registry.select_variant(animation_type, level)
}

/// Get the number of variants available for an animation type at a given level
#[must_use]
pub fn available_variants_count(animation_type: AnimationType, level: EvolutionLevel) -> usize {
    let registry = VariantRegistry::new();
    registry.available_variant_count(animation_type, level)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // AnimationType Tests
    // =========================================================================

    #[test]
    fn test_animation_type_all() {
        let all = AnimationType::all();
        assert_eq!(all.len(), 7);
        assert!(all.contains(&AnimationType::Idle));
        assert!(all.contains(&AnimationType::Thinking));
        assert!(all.contains(&AnimationType::Speaking));
    }

    #[test]
    fn test_animation_type_names() {
        assert_eq!(AnimationType::Idle.animation_name(), "idle");
        assert_eq!(AnimationType::Thinking.animation_name(), "thinking");
        assert_eq!(AnimationType::Speaking.animation_name(), "talking");
        assert_eq!(AnimationType::Happy.animation_name(), "happy");
        assert_eq!(AnimationType::Error.animation_name(), "error");
        assert_eq!(AnimationType::Waiting.animation_name(), "waiting");
        assert_eq!(AnimationType::Swimming.animation_name(), "swimming");
    }

    // =========================================================================
    // AnimationVariant Tests
    // =========================================================================

    #[test]
    fn test_variant_creation() {
        let variant = AnimationVariant::new("test_idle", "Test Idle", AnimationType::Idle);
        assert_eq!(variant.id, "test_idle");
        assert_eq!(variant.name, "Test Idle");
        assert_eq!(variant.animation_type, AnimationType::Idle);
        assert_eq!(variant.weight, 50); // Default weight
        assert_eq!(variant.unlock_level, EvolutionLevel::Nascent);
        assert!((variant.speed_modifier - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_variant_builder_methods() {
        let variant = AnimationVariant::new("test", "Test", AnimationType::Idle)
            .with_description("A test variant")
            .with_weight(75)
            .with_unlock_level(EvolutionLevel::Mature)
            .with_speed_modifier(1.5)
            .with_color_hint("#FF0000")
            .with_tag("test")
            .with_tag("example");

        assert_eq!(variant.description, "A test variant");
        assert_eq!(variant.weight, 75);
        assert_eq!(variant.unlock_level, EvolutionLevel::Mature);
        assert!((variant.speed_modifier - 1.5).abs() < f32::EPSILON);
        assert_eq!(variant.color_hint, Some("#FF0000".to_string()));
        assert_eq!(variant.tags, vec!["test", "example"]);
    }

    #[test]
    fn test_variant_weight_clamping() {
        let variant_low = AnimationVariant::new("test", "Test", AnimationType::Idle).with_weight(0);
        assert_eq!(variant_low.weight, 1); // Clamped to minimum

        let variant_high =
            AnimationVariant::new("test", "Test", AnimationType::Idle).with_weight(200);
        assert_eq!(variant_high.weight, 100); // Clamped to maximum
    }

    #[test]
    fn test_variant_speed_clamping() {
        let variant_slow =
            AnimationVariant::new("test", "Test", AnimationType::Idle).with_speed_modifier(0.01);
        assert!((variant_slow.speed_modifier - 0.1).abs() < f32::EPSILON);

        let variant_fast =
            AnimationVariant::new("test", "Test", AnimationType::Idle).with_speed_modifier(10.0);
        assert!((variant_fast.speed_modifier - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_variant_availability() {
        let variant = AnimationVariant::new("test", "Test", AnimationType::Idle)
            .with_unlock_level(EvolutionLevel::Developing);

        assert!(!variant.is_available_at(EvolutionLevel::Nascent));
        assert!(variant.is_available_at(EvolutionLevel::Developing));
        assert!(variant.is_available_at(EvolutionLevel::Mature));
        assert!(variant.is_available_at(EvolutionLevel::Evolved));
        assert!(variant.is_available_at(EvolutionLevel::Transcendent));
    }

    // =========================================================================
    // VariantRegistry Tests
    // =========================================================================

    #[test]
    fn test_registry_creation() {
        let registry = VariantRegistry::new();
        // Should have default variants registered
        assert!(registry.variant_count(AnimationType::Idle) > 0);
        assert!(registry.variant_count(AnimationType::Thinking) > 0);
        assert!(registry.variant_count(AnimationType::Speaking) > 0);
    }

    #[test]
    fn test_registry_empty() {
        let registry = VariantRegistry::empty();
        assert_eq!(registry.variant_count(AnimationType::Idle), 0);
    }

    #[test]
    fn test_registry_register_custom() {
        let mut registry = VariantRegistry::empty();
        registry.register(AnimationVariant::new(
            "custom_idle",
            "Custom",
            AnimationType::Idle,
        ));
        assert_eq!(registry.variant_count(AnimationType::Idle), 1);
    }

    #[test]
    fn test_registry_idle_variants_count() {
        let registry = VariantRegistry::new();
        // Should have exactly 3 idle variants as specified in requirements
        assert_eq!(registry.variant_count(AnimationType::Idle), 3);
    }

    #[test]
    fn test_registry_thinking_variants_count() {
        let registry = VariantRegistry::new();
        // Should have exactly 2 thinking variants as specified
        assert_eq!(registry.variant_count(AnimationType::Thinking), 2);
    }

    #[test]
    fn test_registry_speaking_variants_count() {
        let registry = VariantRegistry::new();
        // Should have exactly 2 speaking variants as specified
        assert_eq!(registry.variant_count(AnimationType::Speaking), 2);
    }

    #[test]
    fn test_registry_get_available_variants() {
        let registry = VariantRegistry::new();

        // At Nascent, only base variants should be available
        let nascent_variants =
            registry.get_available_variants(AnimationType::Idle, EvolutionLevel::Nascent);
        assert_eq!(nascent_variants.len(), 1);

        // At Developing, more variants should be available
        let developing_variants =
            registry.get_available_variants(AnimationType::Idle, EvolutionLevel::Developing);
        assert_eq!(developing_variants.len(), 2);

        // At Mature, all idle variants should be available
        let mature_variants =
            registry.get_available_variants(AnimationType::Idle, EvolutionLevel::Mature);
        assert_eq!(mature_variants.len(), 3);
    }

    #[test]
    fn test_registry_select_variant_returns_valid() {
        let registry = VariantRegistry::new();
        let variant = registry.select_variant(AnimationType::Idle, EvolutionLevel::Nascent);

        assert_eq!(variant.animation_type, AnimationType::Idle);
        assert!(variant.is_available_at(EvolutionLevel::Nascent));
    }

    #[test]
    fn test_registry_select_variant_seeded_consistent() {
        let registry = VariantRegistry::new();

        let variant1 =
            registry.select_variant_seeded(AnimationType::Idle, EvolutionLevel::Mature, 12345);
        let variant2 =
            registry.select_variant_seeded(AnimationType::Idle, EvolutionLevel::Mature, 12345);

        assert_eq!(variant1.id, variant2.id);
    }

    #[test]
    fn test_registry_select_variant_seeded_different_seeds() {
        let registry = VariantRegistry::new();

        // With enough variants and different seeds, we should eventually get different results
        // This test just verifies the function works - actual randomness is tested by weight distribution
        let variant1 =
            registry.select_variant_seeded(AnimationType::Idle, EvolutionLevel::Mature, 0);
        let variant2 =
            registry.select_variant_seeded(AnimationType::Idle, EvolutionLevel::Mature, 1000);

        // Just verify both return valid variants
        assert_eq!(variant1.animation_type, AnimationType::Idle);
        assert_eq!(variant2.animation_type, AnimationType::Idle);
    }

    #[test]
    fn test_registry_select_variant_empty_fallback() {
        let registry = VariantRegistry::empty();
        let variant = registry.select_variant(AnimationType::Idle, EvolutionLevel::Nascent);

        // Should return a default variant
        assert_eq!(variant.animation_type, AnimationType::Idle);
        assert_eq!(variant.name, "Default");
    }

    #[test]
    fn test_registry_available_variant_count() {
        let registry = VariantRegistry::new();

        assert_eq!(
            registry.available_variant_count(AnimationType::Idle, EvolutionLevel::Nascent),
            1
        );
        assert_eq!(
            registry.available_variant_count(AnimationType::Idle, EvolutionLevel::Developing),
            2
        );
        assert_eq!(
            registry.available_variant_count(AnimationType::Idle, EvolutionLevel::Mature),
            3
        );
    }

    // =========================================================================
    // Variant Properties Tests
    // =========================================================================

    #[test]
    fn test_idle_variant_properties() {
        let registry = VariantRegistry::new();
        let variants = registry.get_variants(AnimationType::Idle);

        // Check that we have the expected variants
        let ids: Vec<&str> = variants.iter().map(|v| v.id.as_str()).collect();
        assert!(ids.contains(&"idle_relaxed"));
        assert!(ids.contains(&"idle_alert"));
        assert!(ids.contains(&"idle_dreamy"));

        // Check unlock levels
        let relaxed = variants.iter().find(|v| v.id == "idle_relaxed").unwrap();
        assert_eq!(relaxed.unlock_level, EvolutionLevel::Nascent);

        let alert = variants.iter().find(|v| v.id == "idle_alert").unwrap();
        assert_eq!(alert.unlock_level, EvolutionLevel::Developing);

        let dreamy = variants.iter().find(|v| v.id == "idle_dreamy").unwrap();
        assert_eq!(dreamy.unlock_level, EvolutionLevel::Mature);
    }

    #[test]
    fn test_thinking_variant_properties() {
        let registry = VariantRegistry::new();
        let variants = registry.get_variants(AnimationType::Thinking);

        let ids: Vec<&str> = variants.iter().map(|v| v.id.as_str()).collect();
        assert!(ids.contains(&"thinking_focused"));
        assert!(ids.contains(&"thinking_pondering"));
    }

    #[test]
    fn test_speaking_variant_properties() {
        let registry = VariantRegistry::new();
        let variants = registry.get_variants(AnimationType::Speaking);

        let ids: Vec<&str> = variants.iter().map(|v| v.id.as_str()).collect();
        assert!(ids.contains(&"speaking_conversational"));
        assert!(ids.contains(&"speaking_expressive"));
    }

    // =========================================================================
    // Weight Distribution Tests
    // =========================================================================

    #[test]
    fn test_weight_distribution_biased() {
        let mut registry = VariantRegistry::empty();

        // Add two variants with very different weights
        registry.register(
            AnimationVariant::new("common", "Common", AnimationType::Idle).with_weight(99),
        );
        registry
            .register(AnimationVariant::new("rare", "Rare", AnimationType::Idle).with_weight(1));

        // With seeded selection, we can verify the weight distribution works
        let mut common_count = 0;
        let mut rare_count = 0;

        for seed in 0..100 {
            let variant =
                registry.select_variant_seeded(AnimationType::Idle, EvolutionLevel::Nascent, seed);
            if variant.id == "common" {
                common_count += 1;
            } else {
                rare_count += 1;
            }
        }

        // Common should be selected much more frequently
        assert!(common_count > rare_count);
        assert!(common_count > 90); // Should be selected ~99% of the time
    }

    // =========================================================================
    // Helper Function Tests
    // =========================================================================

    #[test]
    fn test_select_variant_helper() {
        let variant = select_variant(AnimationType::Idle, EvolutionLevel::Nascent);
        assert_eq!(variant.animation_type, AnimationType::Idle);
    }

    #[test]
    fn test_available_variants_count_helper() {
        let count = available_variants_count(AnimationType::Idle, EvolutionLevel::Mature);
        assert_eq!(count, 3);
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[test]
    fn test_animation_type_serialization() {
        let animation_type = AnimationType::Idle;
        let json = serde_json::to_string(&animation_type).unwrap();
        let parsed: AnimationType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, animation_type);
    }

    #[test]
    fn test_variant_serialization() {
        let variant = AnimationVariant::new("test_idle", "Test Idle", AnimationType::Idle)
            .with_description("A test variant")
            .with_weight(75)
            .with_unlock_level(EvolutionLevel::Developing)
            .with_color_hint("#FF0000")
            .with_tag("test");

        let json = serde_json::to_string(&variant).unwrap();
        let parsed: AnimationVariant = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, variant.id);
        assert_eq!(parsed.name, variant.name);
        assert_eq!(parsed.animation_type, variant.animation_type);
        assert_eq!(parsed.weight, variant.weight);
        assert_eq!(parsed.unlock_level, variant.unlock_level);
        assert_eq!(parsed.color_hint, variant.color_hint);
        assert_eq!(parsed.tags, variant.tags);
    }

    // =========================================================================
    // Evolution Level Integration Tests
    // =========================================================================

    #[test]
    fn test_evolution_level_progression_unlocks_variants() {
        let registry = VariantRegistry::new();

        // Test that higher evolution levels unlock more variants across all types
        for animation_type in AnimationType::all() {
            let nascent_count =
                registry.available_variant_count(*animation_type, EvolutionLevel::Nascent);
            let transcendent_count =
                registry.available_variant_count(*animation_type, EvolutionLevel::Transcendent);

            // Transcendent should have at least as many variants as Nascent
            assert!(transcendent_count >= nascent_count);
        }
    }

    #[test]
    fn test_all_animation_types_have_variants() {
        let registry = VariantRegistry::new();

        for animation_type in AnimationType::all() {
            assert!(
                registry.variant_count(*animation_type) > 0,
                "Animation type {:?} should have at least one variant",
                animation_type
            );
        }
    }
}
