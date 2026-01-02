//! Avatar State, Commands, and Rendering Primitives
//!
//! This module contains:
//! - Avatar STATE and command parsing logic (what the avatar is doing)
//! - Block-based rendering primitives (how to render it)
//!
//! RENDERING is handled by the UI surface (TUI, `WebUI`, etc.).
//!
//! # Design Philosophy
//!
//! The avatar is Yollayah's visual representation - a cute axolotl that expresses
//! personality through movement, expressions, and gestures. The Conductor owns
//! the avatar's state (where it is, what mood, what animation), while UI surfaces
//! render that state however they want.
//!
//! This separation means:
//! - A TUI can render blocky ASCII art sprites
//! - A `WebUI` can render SVG or Canvas animations
//! - A mobile app can render 3D animated meshes
//! - All from the same avatar state!
//!
//! # Module Structure
//!
//! - [`block`]: Block-based rendering primitives (Color, Block, `SizeHint`, `AnchorPoint`)
//! - Avatar state types (position, mood, gestures, reactions)
//! - Command parsing for embedded avatar commands in LLM responses

pub mod block;
pub mod cache;
pub mod evolution;
pub mod security;
pub mod variants;

// Re-export block types at the avatar module level for convenience
pub use block::{
    AnchorPoint, AnimationRequest, AnimationResponse, Block, Color, LoopBehavior, Mood,
    RelativeSize, SizeHint, SpriteRequest, SpriteResponse,
};

// Re-export cache types
pub use cache::{CacheEntry, CacheError, CacheStats, SpriteCache, SpriteData};

// Re-export security types
pub use security::{
    is_allowed_block_char, validate_animation_duration, validate_animation_frames,
    validate_block_count, validate_sprite, validate_sprite_dimensions, validate_sprite_size,
    validate_unicode_char, PendingRequestGuard, PendingRequestTracker, SecurityError,
    SecurityResult, Sprite, SpriteRateLimiter, ALLOWED_UNICODE_RANGES, MAX_ANIMATION_DURATION_MS,
    MAX_ANIMATION_FRAMES, MAX_BLOCKS_PER_SPRITE, MAX_CACHE_SIZE_BYTES,
    MAX_PENDING_REQUESTS_PER_SESSION, MAX_SPRITE_HEIGHT, MAX_SPRITE_REQUESTS_PER_MINUTE,
    MAX_SPRITE_WIDTH,
};

// Re-export evolution types
pub use evolution::{
    EvolutionCallback, EvolutionCallbackManager, EvolutionContext, EvolutionEvent, EvolutionLevel,
    EvolutionProgress, THRESHOLD_DEVELOPING_INTERACTIONS, THRESHOLD_DEVELOPING_TIME_SECS,
    THRESHOLD_EVOLVED_INTERACTIONS, THRESHOLD_EVOLVED_TIME_SECS, THRESHOLD_MATURE_INTERACTIONS,
    THRESHOLD_MATURE_TIME_SECS, THRESHOLD_TRANSCENDENT_INTERACTIONS,
    THRESHOLD_TRANSCENDENT_TIME_SECS,
};

// Re-export variants types
pub use variants::{
    available_variants_count, select_variant, AnimationType, AnimationVariant, VariantRegistry,
};

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

/// Avatar positions
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AvatarPosition {
    /// Top-left corner
    TopLeft,
    /// Top-right corner
    TopRight,
    /// Bottom-left corner
    BottomLeft,
    /// Bottom-right corner
    #[default]
    BottomRight,
    /// Center of screen
    Center,
    /// Follow along with content/text
    Follow,
    /// Specific percentage position (0-100, 0-100)
    Percent {
        /// X position (0-100)
        x: u8,
        /// Y position (0-100)
        y: u8,
    },
}

/// Avatar emotional moods
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AvatarMood {
    /// Default happy state
    #[default]
    Happy,
    /// Deep in thought
    Thinking,
    /// Playful and silly
    Playful,
    /// Bashful/embarrassed
    Shy,
    /// Very excited
    Excited,
    /// Puzzled/uncertain
    Confused,
    /// Peaceful and relaxed
    Calm,
    /// Interested and engaged
    Curious,
}

impl AvatarMood {
    /// Suggested animation name for this mood
    #[must_use]
    pub fn suggested_animation(&self) -> &'static str {
        match self {
            Self::Happy | Self::Excited => "happy",
            Self::Thinking => "thinking",
            Self::Playful => "swimming",
            Self::Shy | Self::Calm => "idle",
            Self::Confused => "error",
            Self::Curious => "waiting",
        }
    }
}

/// Avatar sizes
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AvatarSize {
    /// Tiny (unobtrusive)
    Tiny,
    /// Small
    Small,
    /// Normal size
    #[default]
    Medium,
    /// Large (attention-grabbing)
    Large,
}

/// Gesture animations (short, one-time)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AvatarGesture {
    /// Friendly wave
    Wave,
    /// Agreeing nod
    Nod,
    /// Disagreeing shake
    Shake,
    /// Peek from direction
    Peek(PeekDirection),
    /// Excited bounce
    Bounce,
    /// Playful spin
    Spin,
    /// Happy dance
    Dance,
    /// Swimming motion
    Swim,
    /// Stretching
    Stretch,
    /// Tired yawn
    Yawn,
    /// Wiggle animation
    Wiggle,
}

impl AvatarGesture {
    /// Default duration in milliseconds for this gesture
    #[must_use]
    pub fn default_duration_ms(&self) -> u32 {
        match self {
            Self::Wave => 1500,
            Self::Nod => 800,
            Self::Shake => 800,
            Self::Bounce => 1000,
            Self::Spin => 1200,
            Self::Dance => 2000,
            Self::Swim => 2000,
            Self::Stretch => 1500,
            Self::Yawn => 2000,
            Self::Wiggle => 1000,
            Self::Peek(_) => 1000,
        }
    }

    /// Suggested animation name for this gesture
    #[must_use]
    pub fn suggested_animation(&self) -> &'static str {
        match self {
            Self::Wave | Self::Bounce | Self::Dance => "happy",
            Self::Nod => "talking",
            Self::Shake => "error",
            Self::Spin | Self::Swim | Self::Wiggle => "swimming",
            Self::Stretch | Self::Yawn | Self::Peek(_) => "idle",
        }
    }
}

/// Direction for peek gesture
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PeekDirection {
    /// Peek from left
    Left,
    /// Peek from right
    #[default]
    Right,
    /// Peek from top
    Top,
    /// Peek from bottom
    Bottom,
}

/// Reaction animations (contextual, expressive)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AvatarReaction {
    /// Ha ha!
    Laugh,
    /// Surprised!
    Gasp,
    /// Thinking...
    Hmm,
    /// Celebration!
    Tada,
    /// Made a mistake
    Oops,
    /// Heart eyes
    Love,
    /// Embarrassed
    Blush,
    /// Playful wink
    Wink,
    /// Sad tears
    Cry,
    /// Frustrated
    Angry,
    /// Tired
    Sleepy,
    /// Confused spinning
    Dizzy,
}

impl AvatarReaction {
    /// Default duration in milliseconds for this reaction
    #[must_use]
    pub fn default_duration_ms(&self) -> u32 {
        match self {
            Self::Laugh => 2000,
            Self::Gasp => 1000,
            Self::Hmm => 2000,
            Self::Tada => 2500,
            Self::Oops => 1500,
            Self::Love => 2000,
            Self::Blush => 1500,
            Self::Wink => 1000,
            Self::Cry => 2000,
            Self::Angry => 1500,
            Self::Sleepy => 2500,
            Self::Dizzy => 2000,
        }
    }

    /// Suggested animation name for this reaction
    #[must_use]
    pub fn suggested_animation(&self) -> &'static str {
        match self {
            Self::Laugh | Self::Tada | Self::Love => "happy",
            Self::Gasp | Self::Oops | Self::Cry | Self::Angry => "error",
            Self::Hmm => "thinking",
            Self::Blush | Self::Sleepy => "idle",
            Self::Wink => "wink",
            Self::Dizzy => "swimming",
        }
    }
}

/// Avatar commands that can be embedded in text responses
///
/// The agent (LLM) can control the avatar by embedding commands in responses.
/// Commands are parsed and stripped from the displayed text.
///
/// Format: `[yolla:command arg1 arg2]`
#[derive(Clone, Debug, PartialEq)]
pub enum AvatarCommand {
    /// Move to a named position
    MoveTo(AvatarPosition),
    /// Point at screen location (percentage)
    PointAt {
        /// X position (0-100)
        x_percent: u8,
        /// Y position (0-100)
        y_percent: u8,
    },
    /// Enable/disable free wandering
    Wander(bool),
    /// Set emotional mood
    Mood(AvatarMood),
    /// Set size
    Size(AvatarSize),
    /// Perform a gesture animation
    Gesture(AvatarGesture),
    /// Perform a reaction animation
    React(AvatarReaction),
    /// Hide the avatar
    Hide,
    /// Show the avatar
    Show,
    /// Custom sprite data (future use)
    CustomSprite(String),
    /// Task-related command
    Task(TaskCommand),
}

/// Task management commands embedded in responses
#[derive(Clone, Debug, PartialEq)]
pub enum TaskCommand {
    /// Start a new background task
    Start {
        /// Agent to handle the task
        agent: String,
        /// Task description
        description: String,
    },
    /// Update task progress
    Progress {
        /// Task identifier
        task_id: String,
        /// Progress percentage (0-100)
        percent: u8,
    },
    /// Mark task as done
    Done {
        /// Task identifier
        task_id: String,
    },
    /// Mark task as failed
    Fail {
        /// Task identifier
        task_id: String,
        /// Failure reason
        reason: String,
    },
    /// Focus/highlight a specific task
    Focus {
        /// Task identifier
        task_id: String,
    },
    /// Point at a task
    PointAt {
        /// Task identifier
        task_id: String,
    },
    /// Hover near a task
    Hover {
        /// Task identifier
        task_id: String,
    },
    /// Celebrate task completion
    Celebrate {
        /// Task identifier
        task_id: String,
    },
}

/// Parser for avatar commands embedded in text
///
/// Extracts `[yolla:command arg1 arg2]` patterns from text,
/// queues the commands, and returns cleaned text.
pub struct CommandParser {
    /// Pending commands extracted from text
    pub commands: VecDeque<AvatarCommand>,
}

impl CommandParser {
    /// Create a new command parser
    #[must_use]
    pub fn new() -> Self {
        Self {
            commands: VecDeque::new(),
        }
    }

    /// Parse text, extract commands, return cleaned text
    ///
    /// Commands are queued and can be retrieved with `next_command()`.
    pub fn parse(&mut self, text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '[' {
                // Potential command start
                let mut cmd_buf = String::new();
                let mut found_end = false;

                for inner in chars.by_ref() {
                    if inner == ']' {
                        found_end = true;
                        break;
                    }
                    cmd_buf.push(inner);
                }

                if found_end && cmd_buf.starts_with("yolla:") {
                    // Parse the command
                    if let Some(cmd) = self.parse_command(&cmd_buf[6..]) {
                        self.commands.push_back(cmd);
                    }
                    // Command consumed, don't add to result
                } else {
                    // Not a valid command, restore the text
                    result.push('[');
                    result.push_str(&cmd_buf);
                    if found_end {
                        result.push(']');
                    }
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Parse a single command (without the [yolla: prefix])
    fn parse_command(&self, cmd: &str) -> Option<AvatarCommand> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        match parts[0] {
            // Movement
            "move" => self.parse_move(&parts[1..]),
            "wander" => Some(AvatarCommand::Wander(true)),
            "stop" => Some(AvatarCommand::Wander(false)),
            "point" => self.parse_point(&parts[1..]),
            "follow" => Some(AvatarCommand::MoveTo(AvatarPosition::Follow)),

            // Expression
            "mood" => self.parse_mood(&parts[1..]),
            "size" => self.parse_size(&parts[1..]),
            "hide" => Some(AvatarCommand::Hide),
            "show" => Some(AvatarCommand::Show),

            // Gestures
            "wave" => Some(AvatarCommand::Gesture(AvatarGesture::Wave)),
            "nod" => Some(AvatarCommand::Gesture(AvatarGesture::Nod)),
            "shake" => Some(AvatarCommand::Gesture(AvatarGesture::Shake)),
            "bounce" => Some(AvatarCommand::Gesture(AvatarGesture::Bounce)),
            "spin" => Some(AvatarCommand::Gesture(AvatarGesture::Spin)),
            "dance" => Some(AvatarCommand::Gesture(AvatarGesture::Dance)),
            "swim" => Some(AvatarCommand::Gesture(AvatarGesture::Swim)),
            "stretch" => Some(AvatarCommand::Gesture(AvatarGesture::Stretch)),
            "yawn" => Some(AvatarCommand::Gesture(AvatarGesture::Yawn)),
            "wiggle" => Some(AvatarCommand::Gesture(AvatarGesture::Wiggle)),
            "peek" => self.parse_peek(&parts[1..]),

            // Reactions
            "react" => self.parse_react(&parts[1..]),
            "laugh" => Some(AvatarCommand::React(AvatarReaction::Laugh)),
            "gasp" => Some(AvatarCommand::React(AvatarReaction::Gasp)),
            "tada" => Some(AvatarCommand::React(AvatarReaction::Tada)),
            "oops" => Some(AvatarCommand::React(AvatarReaction::Oops)),
            "love" => Some(AvatarCommand::React(AvatarReaction::Love)),
            "wink" => Some(AvatarCommand::React(AvatarReaction::Wink)),

            // Custom sprites (future)
            "sprite" if parts.len() > 1 => Some(AvatarCommand::CustomSprite(parts[1..].join(" "))),

            // Task management
            "task" => self.parse_task(&parts[1..]),

            _ => None,
        }
    }

    fn parse_move(&self, args: &[&str]) -> Option<AvatarCommand> {
        if args.is_empty() {
            return None;
        }

        // Named position
        match args[0] {
            "tl" | "topleft" | "top-left" => Some(AvatarCommand::MoveTo(AvatarPosition::TopLeft)),
            "tr" | "topright" | "top-right" => {
                Some(AvatarCommand::MoveTo(AvatarPosition::TopRight))
            }
            "bl" | "bottomleft" | "bottom-left" => {
                Some(AvatarCommand::MoveTo(AvatarPosition::BottomLeft))
            }
            "br" | "bottomright" | "bottom-right" => {
                Some(AvatarCommand::MoveTo(AvatarPosition::BottomRight))
            }
            "center" | "middle" => Some(AvatarCommand::MoveTo(AvatarPosition::Center)),
            "follow" => Some(AvatarCommand::MoveTo(AvatarPosition::Follow)),
            _ => {
                // Try parsing as x y coordinates
                if args.len() >= 2 {
                    let x: u8 = args[0].parse().ok()?;
                    let y: u8 = args[1].parse().ok()?;
                    Some(AvatarCommand::MoveTo(AvatarPosition::Percent {
                        x: x.min(100),
                        y: y.min(100),
                    }))
                } else {
                    None
                }
            }
        }
    }

    fn parse_point(&self, args: &[&str]) -> Option<AvatarCommand> {
        if args.len() >= 2 {
            let x: u8 = args[0].parse().ok()?;
            let y: u8 = args[1].parse().ok()?;
            Some(AvatarCommand::PointAt {
                x_percent: x.min(100),
                y_percent: y.min(100),
            })
        } else {
            None
        }
    }

    fn parse_mood(&self, args: &[&str]) -> Option<AvatarCommand> {
        if args.is_empty() {
            return None;
        }

        let mood = match args[0] {
            "happy" | "joy" => AvatarMood::Happy,
            "thinking" | "think" | "ponder" => AvatarMood::Thinking,
            "playful" | "silly" => AvatarMood::Playful,
            "shy" | "bashful" => AvatarMood::Shy,
            "excited" | "eager" => AvatarMood::Excited,
            "confused" | "puzzled" => AvatarMood::Confused,
            "calm" | "peaceful" => AvatarMood::Calm,
            "curious" | "interested" => AvatarMood::Curious,
            _ => return None,
        };

        Some(AvatarCommand::Mood(mood))
    }

    fn parse_size(&self, args: &[&str]) -> Option<AvatarCommand> {
        if args.is_empty() {
            return None;
        }

        let size = match args[0] {
            "tiny" | "xs" => AvatarSize::Tiny,
            "small" | "sm" => AvatarSize::Small,
            "medium" | "md" | "normal" => AvatarSize::Medium,
            "large" | "lg" | "big" => AvatarSize::Large,
            _ => return None,
        };

        Some(AvatarCommand::Size(size))
    }

    fn parse_peek(&self, args: &[&str]) -> Option<AvatarCommand> {
        let dir = if args.is_empty() {
            PeekDirection::Right
        } else {
            match args[0] {
                "left" | "l" => PeekDirection::Left,
                "right" | "r" => PeekDirection::Right,
                "top" | "up" | "t" => PeekDirection::Top,
                "bottom" | "down" | "b" => PeekDirection::Bottom,
                _ => PeekDirection::Right,
            }
        };

        Some(AvatarCommand::Gesture(AvatarGesture::Peek(dir)))
    }

    fn parse_react(&self, args: &[&str]) -> Option<AvatarCommand> {
        if args.is_empty() {
            return None;
        }

        let reaction = match args[0] {
            "laugh" | "lol" | "haha" => AvatarReaction::Laugh,
            "gasp" | "surprise" | "wow" => AvatarReaction::Gasp,
            "hmm" | "think" | "ponder" => AvatarReaction::Hmm,
            "tada" | "celebrate" | "yay" => AvatarReaction::Tada,
            "oops" | "error" | "mistake" => AvatarReaction::Oops,
            "love" | "heart" | "adore" => AvatarReaction::Love,
            "blush" | "shy" | "embarrassed" => AvatarReaction::Blush,
            "wink" | "flirt" => AvatarReaction::Wink,
            "cry" | "sad" | "tears" => AvatarReaction::Cry,
            "angry" | "mad" | "frustrated" => AvatarReaction::Angry,
            "sleepy" | "tired" | "zzz" => AvatarReaction::Sleepy,
            "dizzy" | "confused" | "spinning" => AvatarReaction::Dizzy,
            _ => return None,
        };

        Some(AvatarCommand::React(reaction))
    }

    fn parse_task(&self, args: &[&str]) -> Option<AvatarCommand> {
        if args.is_empty() {
            return None;
        }

        match args[0] {
            "start" if args.len() >= 3 => {
                let agent = args[1].to_string();
                // Description is the rest, possibly quoted
                let desc_parts = &args[2..];
                let description = desc_parts.join(" ").trim_matches('"').to_string();
                Some(AvatarCommand::Task(TaskCommand::Start {
                    agent,
                    description,
                }))
            }
            "progress" if args.len() >= 3 => {
                let task_id = args[1].to_string();
                let percent: u8 = args[2].parse().ok()?;
                Some(AvatarCommand::Task(TaskCommand::Progress {
                    task_id,
                    percent: percent.min(100),
                }))
            }
            "done" if args.len() >= 2 => {
                let task_id = args[1].to_string();
                Some(AvatarCommand::Task(TaskCommand::Done { task_id }))
            }
            "fail" if args.len() >= 2 => {
                let task_id = args[1].to_string();
                let reason = if args.len() > 2 {
                    args[2..].join(" ").trim_matches('"').to_string()
                } else {
                    "Unknown error".to_string()
                };
                Some(AvatarCommand::Task(TaskCommand::Fail { task_id, reason }))
            }
            "focus" if args.len() >= 2 => {
                let task_id = args[1].to_string();
                Some(AvatarCommand::Task(TaskCommand::Focus { task_id }))
            }
            "point" if args.len() >= 2 => {
                let task_id = args[1].to_string();
                Some(AvatarCommand::Task(TaskCommand::PointAt { task_id }))
            }
            "hover" if args.len() >= 2 => {
                let task_id = args[1].to_string();
                Some(AvatarCommand::Task(TaskCommand::Hover { task_id }))
            }
            "celebrate" if args.len() >= 2 => {
                let task_id = args[1].to_string();
                Some(AvatarCommand::Task(TaskCommand::Celebrate { task_id }))
            }
            _ => None,
        }
    }

    /// Get the next pending command
    pub fn next_command(&mut self) -> Option<AvatarCommand> {
        self.commands.pop_front()
    }

    /// Check if there are pending commands
    #[must_use]
    pub fn has_commands(&self) -> bool {
        !self.commands.is_empty()
    }

    /// Clear all pending commands
    pub fn clear(&mut self) {
        self.commands.clear();
    }
}

impl Default for CommandParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Avatar state that the Conductor maintains
///
/// This represents the current state of the avatar that UI surfaces
/// should render. The Conductor updates this based on LLM responses,
/// state machine transitions, and embedded commands.
#[derive(Clone, Debug)]
pub struct AvatarState {
    /// Current position
    pub position: AvatarPosition,
    /// Target position (for smooth movement)
    pub target_position: AvatarPosition,
    /// Current mood
    pub mood: AvatarMood,
    /// Current size
    pub size: AvatarSize,
    /// Whether avatar is visible
    pub visible: bool,
    /// Whether wandering is enabled
    pub wandering: bool,
    /// Current gesture (if any)
    pub current_gesture: Option<AvatarGesture>,
    /// Current reaction (if any)
    pub current_reaction: Option<AvatarReaction>,
}

impl Default for AvatarState {
    fn default() -> Self {
        Self {
            position: AvatarPosition::BottomRight,
            target_position: AvatarPosition::BottomRight,
            mood: AvatarMood::Happy,
            size: AvatarSize::Medium,
            visible: true,
            wandering: true,
            current_gesture: None,
            current_reaction: None,
        }
    }
}

impl AvatarState {
    /// Create a new avatar state with defaults
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply an avatar command to this state
    pub fn apply_command(&mut self, cmd: &AvatarCommand) {
        match cmd {
            AvatarCommand::MoveTo(pos) => {
                self.target_position = *pos;
                self.wandering = false;
            }
            AvatarCommand::PointAt {
                x_percent,
                y_percent,
            } => {
                self.target_position = AvatarPosition::Percent {
                    x: *x_percent,
                    y: *y_percent,
                };
                self.wandering = false;
            }
            AvatarCommand::Wander(enabled) => {
                self.wandering = *enabled;
            }
            AvatarCommand::Mood(mood) => {
                self.mood = *mood;
            }
            AvatarCommand::Size(size) => {
                self.size = *size;
            }
            AvatarCommand::Gesture(gesture) => {
                self.current_gesture = Some(*gesture);
                self.current_reaction = None;
            }
            AvatarCommand::React(reaction) => {
                self.current_reaction = Some(*reaction);
                self.current_gesture = None;
            }
            AvatarCommand::Hide => {
                self.visible = false;
            }
            AvatarCommand::Show => {
                self.visible = true;
            }
            AvatarCommand::CustomSprite(_) => {
                // Future: handle custom sprites
            }
            AvatarCommand::Task(_) => {
                // Task commands don't directly affect avatar state
                // They're handled by the Conductor's task manager
            }
        }
    }

    /// Clear any active gesture or reaction
    pub fn clear_animation(&mut self) {
        self.current_gesture = None;
        self.current_reaction = None;
    }

    /// Get the suggested animation name for current state
    #[must_use]
    pub fn suggested_animation(&self) -> &'static str {
        if let Some(gesture) = &self.current_gesture {
            return gesture.suggested_animation();
        }
        if let Some(reaction) = &self.current_reaction {
            return reaction.suggested_animation();
        }
        self.mood.suggested_animation()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_command() {
        let mut parser = CommandParser::new();
        let result = parser.parse("Hello [yolla:move center] world!");
        assert_eq!(result, "Hello  world!");
        assert_eq!(
            parser.next_command(),
            Some(AvatarCommand::MoveTo(AvatarPosition::Center))
        );
    }

    #[test]
    fn test_parse_multiple_commands() {
        let mut parser = CommandParser::new();
        let result = parser.parse("[yolla:mood happy][yolla:move tr]Hi!");
        assert_eq!(result, "Hi!");
        assert_eq!(
            parser.next_command(),
            Some(AvatarCommand::Mood(AvatarMood::Happy))
        );
        assert_eq!(
            parser.next_command(),
            Some(AvatarCommand::MoveTo(AvatarPosition::TopRight))
        );
    }

    #[test]
    fn test_preserve_normal_brackets() {
        let mut parser = CommandParser::new();
        let result = parser.parse("Array[0] and [other] text");
        assert_eq!(result, "Array[0] and [other] text");
        assert!(!parser.has_commands());
    }

    #[test]
    fn test_parse_percent_position() {
        let mut parser = CommandParser::new();
        let result = parser.parse("[yolla:move 50 75]");
        assert_eq!(result, "");
        assert_eq!(
            parser.next_command(),
            Some(AvatarCommand::MoveTo(AvatarPosition::Percent {
                x: 50,
                y: 75
            }))
        );
    }

    #[test]
    fn test_avatar_state_apply_command() {
        let mut state = AvatarState::new();
        assert!(state.visible);
        assert!(state.wandering);

        state.apply_command(&AvatarCommand::Hide);
        assert!(!state.visible);

        state.apply_command(&AvatarCommand::MoveTo(AvatarPosition::Center));
        assert_eq!(state.target_position, AvatarPosition::Center);
        assert!(!state.wandering);

        state.apply_command(&AvatarCommand::Mood(AvatarMood::Thinking));
        assert_eq!(state.mood, AvatarMood::Thinking);
    }
}
