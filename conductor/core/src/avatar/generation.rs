//! Sprite Generation Pipeline
//!
//! This module provides procedural sprite generation for the Yollayah avatar system.
//! It implements P4.1-P4.3 of the avatar animation system:
//!
//! - P4.1: `SpriteGenerator` trait for sprite generation
//! - P4.2: Procedural mood variations (color tinting, expression modifications)
//! - P4.3: Accessory generation rules (party hat, glasses, coffee mug)
//!
//! # Design Philosophy
//!
//! The sprite generation system uses a rule-based approach that doesn't require
//! an LLM. This ensures:
//! - Deterministic, reproducible sprites
//! - Fast generation without API calls
//! - Consistent visual style
//! - Evolution-gated content unlocks
//!
//! # Mood Variations
//!
//! Each mood has associated color tints and expression patterns:
//! - Happy: warm colors (orange/yellow tint), wide eyes, smile
//! - Thinking: cool colors (blue tint), half-closed eyes, neutral mouth
//! - Error: red tint, concerned expression, furrowed brow
//! - Each mood has 2-3 expression variants for visual freshness
//!
//! # Accessory System
//!
//! Accessories are unlocked based on evolution level:
//! - Nascent (0): No accessories
//! - Developing (1): Basic accessories (glasses)
//! - Mature (2): Fun accessories (party hat, coffee mug)
//! - Evolved (3): Special accessories (crown, bowtie)
//! - Transcendent (4): All accessories including rare items
//!
//! # Usage
//!
//! ```
//! use conductor_core::avatar::generation::{
//!     SpriteGenerator, RuleBasedGenerator, MoodOverlay, Accessory,
//! };
//! use conductor_core::avatar::{Mood, EvolutionLevel};
//!
//! // Create a generator
//! let generator = RuleBasedGenerator::new();
//!
//! // Generate a sprite for a mood and evolution level
//! let sprite = generator.generate(Mood::Happy, EvolutionLevel::Mature);
//!
//! // Apply accessories
//! let sprite_with_hat = generator.compose_with_accessory(
//!     sprite,
//!     Accessory::PartyHat,
//!     EvolutionLevel::Mature
//! );
//! ```

use serde::{Deserialize, Serialize};

use super::block::{Block, Color, Mood, SpriteResponse};
use super::evolution::EvolutionLevel;

// =============================================================================
// Sprite Generator Trait (P4.1)
// =============================================================================

/// Trait for sprite generation implementations
///
/// This trait defines the core interface for generating avatar sprites.
/// Different implementations can use rule-based generation, LLM-assisted
/// generation, or other approaches.
pub trait SpriteGenerator: Send + Sync {
    /// Generate a sprite for the given mood and evolution level
    ///
    /// # Arguments
    ///
    /// * `mood` - The emotional mood to express
    /// * `evolution_level` - The avatar's current evolution level
    ///
    /// # Returns
    ///
    /// A `SpriteResponse` containing the generated sprite
    fn generate(&self, mood: Mood, evolution_level: EvolutionLevel) -> SpriteResponse;

    /// Generate a sprite with a specific variant index
    ///
    /// Each mood has multiple expression variants. This method allows
    /// selecting a specific variant for animation sequences or when
    /// you want deterministic output.
    ///
    /// # Arguments
    ///
    /// * `mood` - The emotional mood to express
    /// * `evolution_level` - The avatar's current evolution level
    /// * `variant` - The variant index (0-based, wraps if out of range)
    fn generate_variant(
        &self,
        mood: Mood,
        evolution_level: EvolutionLevel,
        variant: u8,
    ) -> SpriteResponse;

    /// Get the number of variants available for a mood at an evolution level
    fn variant_count(&self, mood: Mood, evolution_level: EvolutionLevel) -> u8;

    /// Compose a sprite with an accessory
    ///
    /// # Arguments
    ///
    /// * `base_sprite` - The base sprite to add the accessory to
    /// * `accessory` - The accessory to add
    /// * `evolution_level` - Used to check if accessory is unlocked
    ///
    /// # Returns
    ///
    /// The sprite with the accessory composited, or the original sprite
    /// if the accessory is not unlocked at this evolution level
    fn compose_with_accessory(
        &self,
        base_sprite: SpriteResponse,
        accessory: Accessory,
        evolution_level: EvolutionLevel,
    ) -> SpriteResponse;
}

// =============================================================================
// Accessory System (P4.3)
// =============================================================================

/// Accessories that can be added to the avatar sprite
///
/// Accessories provide visual customization and are unlocked based on
/// evolution level. Each accessory has an associated slot that determines
/// where it appears on the sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Accessory {
    /// A festive party hat (Head slot)
    PartyHat,
    /// Stylish glasses (Face slot)
    Glasses,
    /// A warm coffee mug (Hand slot)
    CoffeeMug,
    /// A royal crown (Head slot)
    Crown,
    /// A fancy bowtie (Body slot)
    Bowtie,
    /// A wizard hat (Head slot)
    WizardHat,
    /// Sunglasses (Face slot)
    Sunglasses,
    /// A small flower (Head slot)
    Flower,
    /// A book (Hand slot)
    Book,
    /// Musical notes floating (Effect)
    MusicNotes,
}

impl Accessory {
    /// Get the slot this accessory occupies
    #[must_use]
    pub const fn slot(&self) -> AccessorySlot {
        match self {
            Self::PartyHat | Self::Crown | Self::WizardHat | Self::Flower => AccessorySlot::Head,
            Self::Glasses | Self::Sunglasses => AccessorySlot::Face,
            Self::CoffeeMug | Self::Book => AccessorySlot::Hand,
            Self::Bowtie => AccessorySlot::Body,
            Self::MusicNotes => AccessorySlot::Effect,
        }
    }

    /// Get the minimum evolution level required to unlock this accessory
    #[must_use]
    pub const fn required_level(&self) -> EvolutionLevel {
        match self {
            // Developing (level 1): Basic accessories
            Self::Glasses => EvolutionLevel::Developing,

            // Mature (level 2): Fun accessories
            Self::PartyHat | Self::CoffeeMug | Self::Flower => EvolutionLevel::Mature,

            // Evolved (level 3): Special accessories
            Self::Crown | Self::Bowtie | Self::Sunglasses | Self::Book => EvolutionLevel::Evolved,

            // Transcendent (level 4): Rare items
            Self::WizardHat | Self::MusicNotes => EvolutionLevel::Transcendent,
        }
    }

    /// Check if this accessory is unlocked at the given evolution level
    #[must_use]
    pub fn is_unlocked(&self, level: EvolutionLevel) -> bool {
        level >= self.required_level()
    }

    /// Get all accessories unlocked at a given evolution level
    #[must_use]
    pub fn unlocked_at(level: EvolutionLevel) -> Vec<Accessory> {
        ALL_ACCESSORIES
            .iter()
            .filter(|a| a.is_unlocked(level))
            .copied()
            .collect()
    }

    /// Get the primary color for this accessory
    #[must_use]
    pub const fn primary_color(&self) -> Color {
        match self {
            Self::PartyHat => Color::rgb(255, 105, 180), // Hot pink
            Self::Glasses => Color::rgb(50, 50, 50),     // Dark gray
            Self::CoffeeMug => Color::rgb(139, 90, 43),  // Brown
            Self::Crown => Color::rgb(255, 215, 0),      // Gold
            Self::Bowtie => Color::rgb(128, 0, 128),     // Purple
            Self::WizardHat => Color::rgb(75, 0, 130),   // Indigo
            Self::Sunglasses => Color::rgb(0, 0, 0),     // Black
            Self::Flower => Color::rgb(255, 192, 203),   // Pink
            Self::Book => Color::rgb(139, 69, 19),       // Saddle brown
            Self::MusicNotes => Color::rgb(100, 149, 237), // Cornflower blue
        }
    }
}

/// All available accessories
pub const ALL_ACCESSORIES: &[Accessory] = &[
    Accessory::PartyHat,
    Accessory::Glasses,
    Accessory::CoffeeMug,
    Accessory::Crown,
    Accessory::Bowtie,
    Accessory::WizardHat,
    Accessory::Sunglasses,
    Accessory::Flower,
    Accessory::Book,
    Accessory::MusicNotes,
];

/// Slots where accessories can be placed on the avatar
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessorySlot {
    /// Top of the head (hats, crowns)
    Head,
    /// Face area (glasses)
    Face,
    /// Hand/arm area (held items)
    Hand,
    /// Body/torso area (bowties, clothing)
    Body,
    /// Floating effects around the avatar
    Effect,
}

// =============================================================================
// Mood Overlay System (P4.2)
// =============================================================================

/// Mood overlay configuration for procedural variations
///
/// Each mood has an associated color tint and expression modifiers
/// that are applied to the base sprite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoodOverlay {
    /// The mood this overlay applies to
    pub mood: Mood,
    /// Color tint to apply (blended over base colors)
    pub tint: Color,
    /// Tint intensity (0.0 = no tint, 1.0 = full tint)
    pub tint_intensity: f32,
    /// Eye expression pattern
    pub eye_pattern: EyePattern,
    /// Mouth expression pattern
    pub mouth_pattern: MouthPattern,
    /// Number of expression variants available
    pub variants: u8,
}

impl MoodOverlay {
    /// Create a mood overlay for the given mood
    #[must_use]
    pub fn for_mood(mood: Mood) -> Self {
        match mood {
            Mood::Happy => Self {
                mood,
                tint: Color::rgba(255, 200, 100, 50), // Warm orange/yellow
                tint_intensity: 0.15,
                eye_pattern: EyePattern::Wide,
                mouth_pattern: MouthPattern::Smile,
                variants: 3,
            },
            Mood::Thinking => Self {
                mood,
                tint: Color::rgba(100, 150, 255, 50), // Cool blue
                tint_intensity: 0.12,
                eye_pattern: EyePattern::HalfClosed,
                mouth_pattern: MouthPattern::Neutral,
                variants: 2,
            },
            Mood::Confused => Self {
                mood,
                tint: Color::rgba(255, 100, 100, 50), // Red (error)
                tint_intensity: 0.20,
                eye_pattern: EyePattern::Worried,
                mouth_pattern: MouthPattern::Frown,
                variants: 2,
            },
            Mood::Playful => Self {
                mood,
                tint: Color::rgba(255, 150, 200, 50), // Pink
                tint_intensity: 0.15,
                eye_pattern: EyePattern::Wide,
                mouth_pattern: MouthPattern::OpenSmile,
                variants: 3,
            },
            Mood::Shy => Self {
                mood,
                tint: Color::rgba(255, 180, 180, 50), // Light blush
                tint_intensity: 0.18,
                eye_pattern: EyePattern::AvoidGaze,
                mouth_pattern: MouthPattern::SmallSmile,
                variants: 2,
            },
            Mood::Excited => Self {
                mood,
                tint: Color::rgba(255, 220, 100, 60), // Bright yellow
                tint_intensity: 0.20,
                eye_pattern: EyePattern::Sparkle,
                mouth_pattern: MouthPattern::OpenSmile,
                variants: 3,
            },
            Mood::Calm => Self {
                mood,
                tint: Color::rgba(150, 200, 150, 40), // Soft green
                tint_intensity: 0.10,
                eye_pattern: EyePattern::Relaxed,
                mouth_pattern: MouthPattern::SmallSmile,
                variants: 2,
            },
            Mood::Curious => Self {
                mood,
                tint: Color::rgba(180, 150, 255, 50), // Light purple
                tint_intensity: 0.12,
                eye_pattern: EyePattern::Wide,
                mouth_pattern: MouthPattern::OhShape,
                variants: 2,
            },
            Mood::Sad => Self {
                mood,
                tint: Color::rgba(100, 120, 180, 60), // Muted blue
                tint_intensity: 0.18,
                eye_pattern: EyePattern::Droopy,
                mouth_pattern: MouthPattern::Frown,
                variants: 2,
            },
            Mood::Focused => Self {
                mood,
                tint: Color::rgba(150, 150, 200, 40), // Steel blue
                tint_intensity: 0.10,
                eye_pattern: EyePattern::Narrow,
                mouth_pattern: MouthPattern::Neutral,
                variants: 2,
            },
        }
    }

    /// Apply this mood's tint to a color
    #[must_use]
    pub fn apply_tint(&self, base: Color) -> Color {
        if self.tint_intensity <= 0.0 {
            return base;
        }

        let tint = self.tint;
        let intensity = self.tint_intensity.clamp(0.0, 1.0);

        // Blend the tint color with the base
        Color::rgba(
            blend_channel(base.r, tint.r, intensity),
            blend_channel(base.g, tint.g, intensity),
            blend_channel(base.b, tint.b, intensity),
            base.a, // Preserve alpha
        )
    }
}

/// Helper function to blend color channels
fn blend_channel(base: u8, tint: u8, intensity: f32) -> u8 {
    let base_f = f32::from(base);
    let tint_f = f32::from(tint);
    let result = base_f * (1.0 - intensity) + tint_f * intensity;
    result.clamp(0.0, 255.0) as u8
}

/// Eye expression patterns for mood variations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EyePattern {
    /// Normal open eyes
    Normal,
    /// Wide open (surprised, happy)
    Wide,
    /// Half-closed (thinking, sleepy)
    HalfClosed,
    /// Worried expression
    Worried,
    /// Looking away (shy)
    AvoidGaze,
    /// Sparkly eyes (excited)
    Sparkle,
    /// Relaxed, content expression
    Relaxed,
    /// Sad, droopy eyes
    Droopy,
    /// Narrowed, focused eyes
    Narrow,
    /// Closed eyes
    Closed,
}

/// Mouth expression patterns for mood variations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouthPattern {
    /// Neutral expression
    Neutral,
    /// Happy smile
    Smile,
    /// Small, subtle smile
    SmallSmile,
    /// Big open smile
    OpenSmile,
    /// Sad frown
    Frown,
    /// "O" shape (surprised, curious)
    OhShape,
    /// Open mouth (talking)
    Open,
}

// =============================================================================
// Rule-Based Generator Implementation
// =============================================================================

/// Rule-based sprite generator that produces sprites without LLM assistance
///
/// This generator uses predefined rules and patterns to create sprites
/// procedurally. It's deterministic, fast, and produces consistent results.
#[derive(Debug, Clone)]
pub struct RuleBasedGenerator {
    /// Base sprite dimensions (width, height)
    base_dimensions: (u16, u16),
    /// Primary avatar color
    primary_color: Color,
    /// Secondary avatar color
    secondary_color: Color,
}

impl Default for RuleBasedGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleBasedGenerator {
    /// Create a new rule-based generator with default settings
    #[must_use]
    pub fn new() -> Self {
        Self {
            base_dimensions: (8, 8),
            primary_color: Color::rgb(255, 182, 193), // Light pink (Yollayah)
            secondary_color: Color::rgb(255, 218, 185), // Peach (gills/accents)
        }
    }

    /// Create a generator with custom dimensions
    #[must_use]
    pub fn with_dimensions(mut self, width: u16, height: u16) -> Self {
        self.base_dimensions = (width, height);
        self
    }

    /// Create a generator with custom colors
    #[must_use]
    pub fn with_colors(mut self, primary: Color, secondary: Color) -> Self {
        self.primary_color = primary;
        self.secondary_color = secondary;
        self
    }

    /// Generate the base axolotl sprite pattern
    fn generate_base_sprite(&self, evolution_level: EvolutionLevel) -> Vec<Block> {
        let (width, height) = self.base_dimensions;
        let mut blocks = vec![Block::empty(); (width * height) as usize];

        // Simple axolotl pattern - centered body with gills
        // This creates a cute blocky axolotl shape

        // Scale based on evolution level for visual complexity
        let detail_level = evolution_level.as_u8();

        // Body outline (center)
        for y in 2..6 {
            for x in 2..6 {
                let idx = (y * width + x) as usize;
                if idx < blocks.len() {
                    blocks[idx] = Block::solid(self.primary_color);
                }
            }
        }

        // Head (top-center, slightly wider)
        for y in 1..3 {
            for x in 1..7 {
                let idx = (y * width + x) as usize;
                if idx < blocks.len() && (y == 1 && (2..=5).contains(&x) || y == 2) {
                    blocks[idx] = Block::solid(self.primary_color);
                }
            }
        }

        // Gills (sides) - more detailed at higher evolution
        if detail_level >= 1 {
            // Left gills
            let left_gill = width as usize;
            if left_gill < blocks.len() {
                blocks[left_gill] = Block::solid(self.secondary_color);
            }
            let left_gill2 = (2 * width) as usize;
            if left_gill2 < blocks.len() {
                blocks[left_gill2] = Block::solid(self.secondary_color);
            }

            // Right gills
            let right_gill = (width + 7) as usize;
            if right_gill < blocks.len() {
                blocks[right_gill] = Block::solid(self.secondary_color);
            }
            let right_gill2 = (2 * width + 7) as usize;
            if right_gill2 < blocks.len() {
                blocks[right_gill2] = Block::solid(self.secondary_color);
            }
        }

        // Additional detail at higher evolution levels
        if detail_level >= 2 {
            // Add gradient shading hint
            let idx = (3 * width + 3) as usize;
            if idx < blocks.len() {
                let shaded = self.primary_color.lerp(Color::rgb(200, 150, 160), 0.2);
                blocks[idx] = Block::solid(shaded);
            }
        }

        // Tail (bottom)
        let tail_idx = (6 * width + 3) as usize;
        if tail_idx < blocks.len() {
            blocks[tail_idx] = Block::solid(self.primary_color);
        }
        let tail_idx2 = (6 * width + 4) as usize;
        if tail_idx2 < blocks.len() {
            blocks[tail_idx2] = Block::solid(self.primary_color);
        }
        let tail_idx3 = (7 * width + 4) as usize;
        if tail_idx3 < blocks.len() {
            blocks[tail_idx3] = Block::solid(self.primary_color);
        }

        blocks
    }

    /// Apply eye pattern to the sprite
    fn apply_eyes(&self, blocks: &mut [Block], pattern: EyePattern, variant: u8) {
        let width = self.base_dimensions.0 as usize;
        let eye_color = Color::rgb(0, 0, 0); // Black eyes
        let highlight_color = Color::rgb(255, 255, 255); // Eye highlights

        // Eye positions (row 2, columns 3 and 4 for an 8x8 sprite)
        let left_eye_idx = 2 * width + 3;
        let right_eye_idx = 2 * width + 4;

        match pattern {
            EyePattern::Normal | EyePattern::Relaxed => {
                if left_eye_idx < blocks.len() {
                    blocks[left_eye_idx] = Block::character('o', eye_color);
                }
                if right_eye_idx < blocks.len() {
                    blocks[right_eye_idx] = Block::character('o', eye_color);
                }
            }
            EyePattern::Wide | EyePattern::Sparkle => {
                let eye_char = if pattern == EyePattern::Sparkle && variant.is_multiple_of(2) {
                    '*'
                } else {
                    'O'
                };
                if left_eye_idx < blocks.len() {
                    blocks[left_eye_idx] = Block::character(eye_char, eye_color);
                }
                if right_eye_idx < blocks.len() {
                    blocks[right_eye_idx] = Block::character(eye_char, eye_color);
                }
            }
            EyePattern::HalfClosed => {
                if left_eye_idx < blocks.len() {
                    blocks[left_eye_idx] = Block::character('-', eye_color);
                }
                if right_eye_idx < blocks.len() {
                    blocks[right_eye_idx] = Block::character('-', eye_color);
                }
            }
            EyePattern::Worried => {
                // Asymmetric worried eyes
                if left_eye_idx < blocks.len() {
                    blocks[left_eye_idx] = Block::character('o', eye_color);
                }
                if right_eye_idx < blocks.len() {
                    blocks[right_eye_idx] = Block::character(';', eye_color);
                }
            }
            EyePattern::AvoidGaze => {
                // Looking to the side based on variant
                let (left_char, right_char) = if variant.is_multiple_of(2) {
                    ('.', 'o')
                } else {
                    ('o', '.')
                };
                if left_eye_idx < blocks.len() {
                    blocks[left_eye_idx] = Block::character(left_char, eye_color);
                }
                if right_eye_idx < blocks.len() {
                    blocks[right_eye_idx] = Block::character(right_char, eye_color);
                }
            }
            EyePattern::Droopy => {
                if left_eye_idx < blocks.len() {
                    blocks[left_eye_idx] = Block::character('v', eye_color);
                }
                if right_eye_idx < blocks.len() {
                    blocks[right_eye_idx] = Block::character('v', eye_color);
                }
            }
            EyePattern::Narrow => {
                if left_eye_idx < blocks.len() {
                    blocks[left_eye_idx] = Block::character('>', eye_color);
                }
                if right_eye_idx < blocks.len() {
                    blocks[right_eye_idx] = Block::character('<', eye_color);
                }
            }
            EyePattern::Closed => {
                if left_eye_idx < blocks.len() {
                    blocks[left_eye_idx] = Block::character('^', eye_color);
                }
                if right_eye_idx < blocks.len() {
                    blocks[right_eye_idx] = Block::character('^', eye_color);
                }
            }
        }

        // Add eye highlights at higher detail
        if pattern == EyePattern::Wide || pattern == EyePattern::Sparkle {
            // Highlight above left eye
            let highlight_idx = width + 3;
            if highlight_idx < blocks.len() && blocks[highlight_idx].is_empty() {
                blocks[highlight_idx] = Block::character('.', highlight_color);
            }
        }
    }

    /// Apply mouth pattern to the sprite
    fn apply_mouth(&self, blocks: &mut [Block], pattern: MouthPattern, variant: u8) {
        let width = self.base_dimensions.0 as usize;
        let mouth_color = Color::rgb(80, 40, 40); // Dark mouth color

        // Mouth position (row 3, centered)
        let mouth_idx = 3 * width + 3;
        let _mouth_idx_right = 3 * width + 4; // Reserved for multi-block mouths

        match pattern {
            MouthPattern::Neutral => {
                if mouth_idx < blocks.len() {
                    blocks[mouth_idx] = Block::character('-', mouth_color);
                }
            }
            MouthPattern::Smile => {
                // Smile spans two blocks
                if mouth_idx < blocks.len() {
                    blocks[mouth_idx] = Block::character('u', mouth_color);
                }
            }
            MouthPattern::SmallSmile => {
                if mouth_idx < blocks.len() {
                    blocks[mouth_idx] = Block::character('~', mouth_color);
                }
            }
            MouthPattern::OpenSmile => {
                let char = if variant.is_multiple_of(2) { 'D' } else { 'U' };
                if mouth_idx < blocks.len() {
                    blocks[mouth_idx] = Block::character(char, mouth_color);
                }
            }
            MouthPattern::Frown => {
                if mouth_idx < blocks.len() {
                    blocks[mouth_idx] = Block::character('n', mouth_color);
                }
            }
            MouthPattern::OhShape => {
                if mouth_idx < blocks.len() {
                    blocks[mouth_idx] = Block::character('o', mouth_color);
                }
            }
            MouthPattern::Open => {
                if mouth_idx < blocks.len() {
                    blocks[mouth_idx] = Block::character('O', mouth_color);
                }
            }
        }
    }

    /// Apply color tint to all non-empty blocks
    fn apply_tint(&self, blocks: &mut [Block], overlay: &MoodOverlay) {
        for block in blocks.iter_mut() {
            if !block.is_empty() {
                block.fg = overlay.apply_tint(block.fg);
                block.bg = overlay.apply_tint(block.bg);
            }
        }
    }

    /// Generate accessory blocks for composition
    fn generate_accessory_blocks(
        &self,
        accessory: Accessory,
    ) -> (Vec<Block>, (u16, u16), (i16, i16)) {
        // Returns (blocks, dimensions, offset from sprite top-left)
        match accessory {
            Accessory::PartyHat => {
                // 3x2 party hat
                let color = accessory.primary_color();
                let accent = Color::rgb(255, 255, 100); // Yellow stripe
                let blocks = vec![
                    Block::empty(),
                    Block::solid(color),
                    Block::empty(),
                    Block::solid(color),
                    Block::solid(accent),
                    Block::solid(color),
                ];
                (blocks, (3, 2), (2, -2))
            }
            Accessory::Glasses => {
                // 4x1 glasses
                let color = accessory.primary_color();
                let lens = Color::rgba(100, 150, 255, 180); // Tinted lens
                let blocks = vec![
                    Block::solid(color),
                    Block::solid(lens),
                    Block::solid(lens),
                    Block::solid(color),
                ];
                (blocks, (4, 1), (2, 2))
            }
            Accessory::CoffeeMug => {
                // 2x2 mug
                let color = accessory.primary_color();
                let liquid = Color::rgb(60, 30, 10); // Coffee color
                let blocks = vec![
                    Block::solid(color),
                    Block::solid(liquid),
                    Block::solid(color),
                    Block::solid(color),
                ];
                (blocks, (2, 2), (6, 3))
            }
            Accessory::Crown => {
                // 4x2 crown
                let color = accessory.primary_color();
                let gem = Color::rgb(255, 0, 0); // Red gem
                let blocks = vec![
                    Block::solid(color),
                    Block::empty(),
                    Block::empty(),
                    Block::solid(color),
                    Block::solid(color),
                    Block::solid(gem),
                    Block::solid(gem),
                    Block::solid(color),
                ];
                (blocks, (4, 2), (2, -2))
            }
            Accessory::Bowtie => {
                // 3x1 bowtie
                let color = accessory.primary_color();
                let center = Color::rgb(200, 100, 200);
                let blocks = vec![
                    Block::solid(color),
                    Block::solid(center),
                    Block::solid(color),
                ];
                (blocks, (3, 1), (2, 5))
            }
            Accessory::WizardHat => {
                // 4x3 wizard hat
                let color = accessory.primary_color();
                let star = Color::rgb(255, 255, 100);
                let blocks = vec![
                    Block::empty(),
                    Block::solid(color),
                    Block::solid(color),
                    Block::empty(),
                    Block::empty(),
                    Block::solid(color),
                    Block::solid(color),
                    Block::empty(),
                    Block::solid(color),
                    Block::solid(star),
                    Block::solid(color),
                    Block::solid(color),
                ];
                (blocks, (4, 3), (2, -3))
            }
            Accessory::Sunglasses => {
                // 4x1 sunglasses (dark)
                let color = accessory.primary_color();
                let blocks = vec![
                    Block::solid(color),
                    Block::solid(color),
                    Block::solid(color),
                    Block::solid(color),
                ];
                (blocks, (4, 1), (2, 2))
            }
            Accessory::Flower => {
                // 2x2 flower
                let color = accessory.primary_color();
                let center = Color::rgb(255, 255, 0); // Yellow center
                let blocks = vec![
                    Block::solid(color),
                    Block::solid(color),
                    Block::solid(center),
                    Block::solid(color),
                ];
                (blocks, (2, 2), (0, 0))
            }
            Accessory::Book => {
                // 2x2 book
                let color = accessory.primary_color();
                let pages = Color::rgb(255, 250, 240); // Cream pages
                let blocks = vec![
                    Block::solid(color),
                    Block::solid(pages),
                    Block::solid(color),
                    Block::solid(pages),
                ];
                (blocks, (2, 2), (6, 4))
            }
            Accessory::MusicNotes => {
                // 3x2 floating notes
                let color = accessory.primary_color();
                let blocks = vec![
                    Block::character('\u{266A}', color), // Music note
                    Block::empty(),
                    Block::character('\u{266B}', color), // Beamed notes
                    Block::empty(),
                    Block::character('\u{266A}', color),
                    Block::empty(),
                ];
                (blocks, (3, 2), (-1, -1))
            }
        }
    }
}

impl SpriteGenerator for RuleBasedGenerator {
    fn generate(&self, mood: Mood, evolution_level: EvolutionLevel) -> SpriteResponse {
        self.generate_variant(mood, evolution_level, 0)
    }

    fn generate_variant(
        &self,
        mood: Mood,
        evolution_level: EvolutionLevel,
        variant: u8,
    ) -> SpriteResponse {
        let overlay = MoodOverlay::for_mood(mood);
        let actual_variant = variant % overlay.variants.max(1);

        // Generate base sprite
        let mut blocks = self.generate_base_sprite(evolution_level);

        // Apply expression patterns
        self.apply_eyes(&mut blocks, overlay.eye_pattern, actual_variant);
        self.apply_mouth(&mut blocks, overlay.mouth_pattern, actual_variant);

        // Apply mood tint
        self.apply_tint(&mut blocks, &overlay);

        let (width, height) = self.base_dimensions;
        SpriteResponse::new(blocks, width, height).with_cache_key(format!(
            "generated_{}_{}_v{}",
            mood_to_string(mood),
            evolution_level.as_u8(),
            actual_variant
        ))
    }

    fn variant_count(&self, mood: Mood, _evolution_level: EvolutionLevel) -> u8 {
        MoodOverlay::for_mood(mood).variants
    }

    fn compose_with_accessory(
        &self,
        base_sprite: SpriteResponse,
        accessory: Accessory,
        evolution_level: EvolutionLevel,
    ) -> SpriteResponse {
        // Check if accessory is unlocked
        if !accessory.is_unlocked(evolution_level) {
            return base_sprite;
        }

        let (acc_blocks, (acc_width, acc_height), (offset_x, offset_y)) =
            self.generate_accessory_blocks(accessory);

        let base_width = base_sprite.width() as i16;
        let base_height = base_sprite.height() as i16;

        // Calculate new sprite dimensions to accommodate both base and accessory
        let new_left = offset_x.min(0);
        let new_top = offset_y.min(0);
        let new_right = (offset_x + acc_width as i16).max(base_width);
        let new_bottom = (offset_y + acc_height as i16).max(base_height);

        let new_width = (new_right - new_left) as u16;
        let new_height = (new_bottom - new_top) as u16;

        // Create new block array
        let mut new_blocks = vec![Block::empty(); (new_width * new_height) as usize];

        // Copy base sprite (offset by new_left and new_top)
        let base_offset_x = -new_left;
        let base_offset_y = -new_top;

        for y in 0..base_sprite.height() {
            for x in 0..base_sprite.width() {
                let src_idx = (y * base_sprite.width() + x) as usize;
                let dst_x = (x as i16 + base_offset_x) as u16;
                let dst_y = (y as i16 + base_offset_y) as u16;
                let dst_idx = (dst_y * new_width + dst_x) as usize;

                if dst_idx < new_blocks.len() && src_idx < base_sprite.blocks.len() {
                    new_blocks[dst_idx] = base_sprite.blocks[src_idx].clone();
                }
            }
        }

        // Composite accessory on top
        let acc_start_x = (offset_x - new_left) as u16;
        let acc_start_y = (offset_y - new_top) as u16;

        for y in 0..acc_height {
            for x in 0..acc_width {
                let src_idx = (y * acc_width + x) as usize;
                let dst_x = acc_start_x + x;
                let dst_y = acc_start_y + y;
                let dst_idx = (dst_y * new_width + dst_x) as usize;

                if dst_idx < new_blocks.len() && src_idx < acc_blocks.len() {
                    let acc_block = &acc_blocks[src_idx];
                    if !acc_block.is_empty() {
                        // Blend accessory over base
                        new_blocks[dst_idx] = acc_block.blend_over(&new_blocks[dst_idx]);
                    }
                }
            }
        }

        SpriteResponse::new(new_blocks, new_width, new_height).with_cache_key(format!(
            "{}_with_{:?}",
            base_sprite
                .cache_key
                .unwrap_or_else(|| "sprite".to_string()),
            accessory
        ))
    }
}

/// Helper function to convert mood to string for cache keys
fn mood_to_string(mood: Mood) -> &'static str {
    match mood {
        Mood::Happy => "happy",
        Mood::Thinking => "thinking",
        Mood::Playful => "playful",
        Mood::Shy => "shy",
        Mood::Excited => "excited",
        Mood::Confused => "confused",
        Mood::Calm => "calm",
        Mood::Curious => "curious",
        Mood::Sad => "sad",
        Mood::Focused => "focused",
    }
}

// =============================================================================
// Convenience Functions
// =============================================================================

/// Compose a base sprite with an accessory using the default generator
///
/// This is a convenience function for quick accessory composition.
///
/// # Arguments
///
/// * `base_sprite` - The base sprite to add the accessory to
/// * `accessory` - The accessory to add
/// * `evolution_level` - Used to check if accessory is unlocked
///
/// # Returns
///
/// The sprite with the accessory composited
pub fn compose_with_accessory(
    base_sprite: SpriteResponse,
    accessory: Accessory,
    evolution_level: EvolutionLevel,
) -> SpriteResponse {
    let generator = RuleBasedGenerator::new();
    generator.compose_with_accessory(base_sprite, accessory, evolution_level)
}

/// Get all accessories available at an evolution level
#[must_use]
pub fn available_accessories(level: EvolutionLevel) -> Vec<Accessory> {
    Accessory::unlocked_at(level)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // SpriteGenerator Trait Tests
    // =========================================================================

    #[test]
    fn test_generator_produces_valid_sprite() {
        let generator = RuleBasedGenerator::new();
        let sprite = generator.generate(Mood::Happy, EvolutionLevel::Nascent);

        assert!(!sprite.blocks.is_empty());
        assert_eq!(sprite.width(), 8);
        assert_eq!(sprite.height(), 8);
        assert_eq!(sprite.blocks.len(), 64);
    }

    #[test]
    fn test_generator_different_moods_produce_different_sprites() {
        let generator = RuleBasedGenerator::new();

        let happy = generator.generate(Mood::Happy, EvolutionLevel::Mature);
        let sad = generator.generate(Mood::Sad, EvolutionLevel::Mature);
        let thinking = generator.generate(Mood::Thinking, EvolutionLevel::Mature);

        // Sprites should have different cache keys
        assert_ne!(happy.cache_key, sad.cache_key);
        assert_ne!(happy.cache_key, thinking.cache_key);
        assert_ne!(sad.cache_key, thinking.cache_key);
    }

    #[test]
    fn test_generator_variants() {
        let generator = RuleBasedGenerator::new();

        let variant_count = generator.variant_count(Mood::Happy, EvolutionLevel::Mature);
        assert!(variant_count >= 2);

        let v0 = generator.generate_variant(Mood::Happy, EvolutionLevel::Mature, 0);
        let v1 = generator.generate_variant(Mood::Happy, EvolutionLevel::Mature, 1);

        assert!(v0.cache_key.is_some());
        assert!(v1.cache_key.is_some());
        assert_ne!(v0.cache_key, v1.cache_key);
    }

    #[test]
    fn test_generator_variant_wrapping() {
        let generator = RuleBasedGenerator::new();

        // High variant number should wrap
        let v0 = generator.generate_variant(Mood::Happy, EvolutionLevel::Mature, 0);
        let v_high = generator.generate_variant(Mood::Happy, EvolutionLevel::Mature, 100);

        // v_high should wrap to v0 or another valid variant
        assert!(v_high.cache_key.is_some());
    }

    // =========================================================================
    // Mood Overlay Tests
    // =========================================================================

    #[test]
    fn test_mood_overlay_for_each_mood() {
        let moods = [
            Mood::Happy,
            Mood::Thinking,
            Mood::Playful,
            Mood::Shy,
            Mood::Excited,
            Mood::Confused,
            Mood::Calm,
            Mood::Curious,
            Mood::Sad,
            Mood::Focused,
        ];

        for mood in moods {
            let overlay = MoodOverlay::for_mood(mood);
            assert_eq!(overlay.mood, mood);
            assert!(overlay.variants >= 2);
            assert!(overlay.tint_intensity >= 0.0);
            assert!(overlay.tint_intensity <= 1.0);
        }
    }

    #[test]
    fn test_mood_tinting() {
        let overlay = MoodOverlay::for_mood(Mood::Happy);
        let base = Color::rgb(200, 200, 200); // Use gray so tinting is visible
        let tinted = overlay.apply_tint(base);

        // At least one color channel should be different due to tinting
        let is_different = tinted.r != base.r || tinted.g != base.g || tinted.b != base.b;
        assert!(is_different, "Tinted color should differ from base");
        // Alpha should be preserved
        assert_eq!(tinted.a, base.a);
    }

    #[test]
    fn test_mood_tinting_zero_intensity() {
        let mut overlay = MoodOverlay::for_mood(Mood::Happy);
        overlay.tint_intensity = 0.0;

        let base = Color::rgb(100, 150, 200);
        let tinted = overlay.apply_tint(base);

        // With zero intensity, should be unchanged
        assert_eq!(tinted, base);
    }

    #[test]
    fn test_happy_mood_warm_tint() {
        let overlay = MoodOverlay::for_mood(Mood::Happy);

        // Happy should have a warm (orange/yellow) tint
        let base = Color::rgb(200, 200, 200); // Gray base
        let tinted = overlay.apply_tint(base);

        // Red and green channels should increase more than blue (warm color)
        // Or at least the tint color has higher r/g values
        assert!(overlay.tint.r > overlay.tint.b || overlay.tint.g > overlay.tint.b);
    }

    #[test]
    fn test_thinking_mood_cool_tint() {
        let overlay = MoodOverlay::for_mood(Mood::Thinking);

        // Thinking should have a cool (blue) tint
        assert!(overlay.tint.b > overlay.tint.r);
    }

    #[test]
    fn test_confused_mood_red_tint() {
        let overlay = MoodOverlay::for_mood(Mood::Confused);

        // Confused/Error should have a red tint
        assert!(overlay.tint.r > overlay.tint.b);
        assert!(overlay.tint.r > overlay.tint.g);
    }

    // =========================================================================
    // Accessory Tests
    // =========================================================================

    #[test]
    fn test_accessory_slots() {
        assert_eq!(Accessory::PartyHat.slot(), AccessorySlot::Head);
        assert_eq!(Accessory::Glasses.slot(), AccessorySlot::Face);
        assert_eq!(Accessory::CoffeeMug.slot(), AccessorySlot::Hand);
        assert_eq!(Accessory::Bowtie.slot(), AccessorySlot::Body);
        assert_eq!(Accessory::MusicNotes.slot(), AccessorySlot::Effect);
    }

    #[test]
    fn test_accessory_required_levels() {
        // Glasses should be unlocked at Developing
        assert_eq!(
            Accessory::Glasses.required_level(),
            EvolutionLevel::Developing
        );

        // PartyHat should be unlocked at Mature
        assert_eq!(Accessory::PartyHat.required_level(), EvolutionLevel::Mature);

        // Crown should be unlocked at Evolved
        assert_eq!(Accessory::Crown.required_level(), EvolutionLevel::Evolved);

        // WizardHat should be unlocked at Transcendent
        assert_eq!(
            Accessory::WizardHat.required_level(),
            EvolutionLevel::Transcendent
        );
    }

    #[test]
    fn test_accessory_unlock_check() {
        // Glasses at different levels
        assert!(!Accessory::Glasses.is_unlocked(EvolutionLevel::Nascent));
        assert!(Accessory::Glasses.is_unlocked(EvolutionLevel::Developing));
        assert!(Accessory::Glasses.is_unlocked(EvolutionLevel::Transcendent));

        // WizardHat only at Transcendent
        assert!(!Accessory::WizardHat.is_unlocked(EvolutionLevel::Evolved));
        assert!(Accessory::WizardHat.is_unlocked(EvolutionLevel::Transcendent));
    }

    #[test]
    fn test_unlocked_accessories_at_levels() {
        let nascent = Accessory::unlocked_at(EvolutionLevel::Nascent);
        assert!(nascent.is_empty());

        let developing = Accessory::unlocked_at(EvolutionLevel::Developing);
        assert!(developing.contains(&Accessory::Glasses));
        assert!(!developing.contains(&Accessory::PartyHat));

        let mature = Accessory::unlocked_at(EvolutionLevel::Mature);
        assert!(mature.contains(&Accessory::Glasses));
        assert!(mature.contains(&Accessory::PartyHat));
        assert!(mature.contains(&Accessory::CoffeeMug));
        assert!(!mature.contains(&Accessory::Crown));

        let transcendent = Accessory::unlocked_at(EvolutionLevel::Transcendent);
        assert!(transcendent.len() == ALL_ACCESSORIES.len());
    }

    #[test]
    fn test_all_accessories_have_colors() {
        for accessory in ALL_ACCESSORIES {
            let color = accessory.primary_color();
            // Color should not be transparent
            assert!(!color.is_transparent());
        }
    }

    // =========================================================================
    // Accessory Composition Tests
    // =========================================================================

    #[test]
    fn test_compose_with_accessory_locked() {
        let generator = RuleBasedGenerator::new();
        let base = generator.generate(Mood::Happy, EvolutionLevel::Nascent);
        let original_len = base.blocks.len();

        // Try to add party hat at Nascent (should be locked)
        let result = generator.compose_with_accessory(
            base.clone(),
            Accessory::PartyHat,
            EvolutionLevel::Nascent,
        );

        // Should return unchanged sprite
        assert_eq!(result.blocks.len(), original_len);
    }

    #[test]
    fn test_compose_with_accessory_unlocked() {
        let generator = RuleBasedGenerator::new();
        let base = generator.generate(Mood::Happy, EvolutionLevel::Mature);

        // Add party hat at Mature (should work)
        let result = generator.compose_with_accessory(
            base.clone(),
            Accessory::PartyHat,
            EvolutionLevel::Mature,
        );

        // Cache key should include accessory info
        assert!(result.cache_key.is_some());
        assert!(result.cache_key.as_ref().unwrap().contains("PartyHat"));
    }

    #[test]
    fn test_compose_maintains_sprite_integrity() {
        let generator = RuleBasedGenerator::new();
        let base = generator.generate(Mood::Happy, EvolutionLevel::Evolved);

        let result = generator.compose_with_accessory(
            base.clone(),
            Accessory::Glasses,
            EvolutionLevel::Evolved,
        );

        // Result should have valid dimensions
        assert!(result.width() > 0);
        assert!(result.height() > 0);
        assert_eq!(
            result.blocks.len(),
            (result.width() * result.height()) as usize
        );

        // Check that blocks are valid
        for block in &result.blocks {
            // All blocks should have valid colors
            assert!(block.fg.a <= 255);
            assert!(block.bg.a <= 255);
        }
    }

    #[test]
    fn test_compose_multiple_accessories() {
        let generator = RuleBasedGenerator::new();
        let base = generator.generate(Mood::Happy, EvolutionLevel::Transcendent);

        // Add multiple accessories
        let with_glasses = generator.compose_with_accessory(
            base,
            Accessory::Glasses,
            EvolutionLevel::Transcendent,
        );

        let with_both = generator.compose_with_accessory(
            with_glasses,
            Accessory::PartyHat,
            EvolutionLevel::Transcendent,
        );

        // Should still be valid
        assert!(with_both.width() > 0);
        assert!(with_both.height() > 0);
        assert_eq!(
            with_both.blocks.len(),
            (with_both.width() * with_both.height()) as usize
        );
    }

    // =========================================================================
    // Evolution Level Integration Tests
    // =========================================================================

    #[test]
    fn test_evolution_affects_sprite_detail() {
        let generator = RuleBasedGenerator::new();

        let nascent = generator.generate(Mood::Happy, EvolutionLevel::Nascent);
        let evolved = generator.generate(Mood::Happy, EvolutionLevel::Evolved);

        // Both should be valid
        assert_eq!(nascent.blocks.len(), evolved.blocks.len());

        // Cache keys should differ by evolution level
        let nascent_key = nascent.cache_key.unwrap();
        let evolved_key = evolved.cache_key.unwrap();
        assert_ne!(nascent_key, evolved_key);
    }

    // =========================================================================
    // Convenience Function Tests
    // =========================================================================

    #[test]
    fn test_compose_with_accessory_convenience() {
        let generator = RuleBasedGenerator::new();
        let base = generator.generate(Mood::Happy, EvolutionLevel::Mature);

        let result = compose_with_accessory(base, Accessory::CoffeeMug, EvolutionLevel::Mature);

        // Should work just like the method
        assert!(result.cache_key.is_some());
    }

    #[test]
    fn test_available_accessories_convenience() {
        let nascent = available_accessories(EvolutionLevel::Nascent);
        assert!(nascent.is_empty());

        let mature = available_accessories(EvolutionLevel::Mature);
        assert!(!mature.is_empty());
        assert!(mature.contains(&Accessory::PartyHat));
    }

    // =========================================================================
    // Eye and Mouth Pattern Tests
    // =========================================================================

    #[test]
    fn test_all_eye_patterns_rendered() {
        let patterns = [
            EyePattern::Normal,
            EyePattern::Wide,
            EyePattern::HalfClosed,
            EyePattern::Worried,
            EyePattern::AvoidGaze,
            EyePattern::Sparkle,
            EyePattern::Relaxed,
            EyePattern::Droopy,
            EyePattern::Narrow,
            EyePattern::Closed,
        ];

        let generator = RuleBasedGenerator::new();

        for pattern in patterns {
            let mut blocks = generator.generate_base_sprite(EvolutionLevel::Mature);
            generator.apply_eyes(&mut blocks, pattern, 0);

            // Should have applied some eye pattern (blocks should be modified)
            let non_empty: Vec<_> = blocks.iter().filter(|b| !b.is_empty()).collect();
            assert!(!non_empty.is_empty());
        }
    }

    #[test]
    fn test_all_mouth_patterns_rendered() {
        let patterns = [
            MouthPattern::Neutral,
            MouthPattern::Smile,
            MouthPattern::SmallSmile,
            MouthPattern::OpenSmile,
            MouthPattern::Frown,
            MouthPattern::OhShape,
            MouthPattern::Open,
        ];

        let generator = RuleBasedGenerator::new();

        for pattern in patterns {
            let mut blocks = generator.generate_base_sprite(EvolutionLevel::Mature);
            generator.apply_mouth(&mut blocks, pattern, 0);

            // Should have applied some mouth pattern
            let non_empty: Vec<_> = blocks.iter().filter(|b| !b.is_empty()).collect();
            assert!(!non_empty.is_empty());
        }
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[test]
    fn test_accessory_serialization() {
        let accessory = Accessory::PartyHat;
        let json = serde_json::to_string(&accessory).unwrap();
        let parsed: Accessory = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, accessory);
    }

    #[test]
    fn test_accessory_slot_serialization() {
        let slot = AccessorySlot::Head;
        let json = serde_json::to_string(&slot).unwrap();
        let parsed: AccessorySlot = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, slot);
    }

    #[test]
    fn test_mood_overlay_serialization() {
        let overlay = MoodOverlay::for_mood(Mood::Happy);
        let json = serde_json::to_string(&overlay).unwrap();
        let parsed: MoodOverlay = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.mood, overlay.mood);
        assert_eq!(parsed.variants, overlay.variants);
    }

    #[test]
    fn test_eye_pattern_serialization() {
        let patterns = [EyePattern::Normal, EyePattern::Wide, EyePattern::Sparkle];

        for pattern in patterns {
            let json = serde_json::to_string(&pattern).unwrap();
            let parsed: EyePattern = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, pattern);
        }
    }

    #[test]
    fn test_mouth_pattern_serialization() {
        let patterns = [
            MouthPattern::Smile,
            MouthPattern::Frown,
            MouthPattern::OhShape,
        ];

        for pattern in patterns {
            let json = serde_json::to_string(&pattern).unwrap();
            let parsed: MouthPattern = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, pattern);
        }
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[test]
    fn test_generator_with_custom_dimensions() {
        let generator = RuleBasedGenerator::new().with_dimensions(16, 16);
        let sprite = generator.generate(Mood::Happy, EvolutionLevel::Mature);

        assert_eq!(sprite.width(), 16);
        assert_eq!(sprite.height(), 16);
        assert_eq!(sprite.blocks.len(), 256);
    }

    #[test]
    fn test_generator_with_custom_colors() {
        let custom_primary = Color::rgb(100, 200, 150);
        let custom_secondary = Color::rgb(50, 100, 75);

        let generator = RuleBasedGenerator::new().with_colors(custom_primary, custom_secondary);

        let sprite = generator.generate(Mood::Happy, EvolutionLevel::Mature);

        // Should have blocks with colors derived from custom colors
        let has_custom_color = sprite
            .blocks
            .iter()
            .any(|b| !b.is_empty() && (b.fg.g > b.fg.r || b.bg.g > b.bg.r));
        assert!(has_custom_color);
    }

    #[test]
    fn test_blend_channel_boundaries() {
        // Test boundary values
        assert_eq!(blend_channel(0, 0, 0.0), 0);
        assert_eq!(blend_channel(255, 255, 1.0), 255);
        // At 50% blend, result should be approximately midpoint (127 or 128 due to rounding)
        let mid_low = blend_channel(0, 255, 0.5);
        let mid_high = blend_channel(255, 0, 0.5);
        assert!(
            mid_low >= 127 && mid_low <= 128,
            "Expected ~127-128, got {}",
            mid_low
        );
        assert!(
            mid_high >= 127 && mid_high <= 128,
            "Expected ~127-128, got {}",
            mid_high
        );
    }
}
