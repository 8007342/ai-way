//! Avatar Command Protocol
//!
//! The agent can control Yollayah's avatar by embedding commands in its response.
//! Commands are parsed and stripped from the displayed text.
//!
//! ## Design Philosophy
//!
//! Yollayah is an animated character that happens to be rendered in blocky terminal style.
//! The agent controls its puppet - position, expression, animations, and reactions.
//! The TUI just renders what it's told, enabling rich character behavior.
//!
//! ## Command Format
//!
//! Commands use a simple inline format that's easy for the LLM to emit:
//!
//! ```text
//! [yolla:command arg1 arg2]
//! ```
//!
//! ## Available Commands
//!
//! ### Movement
//! - `[yolla:move corner]` - Move to a corner (tl, tr, bl, br, center)
//! - `[yolla:move x y]` - Move to specific position (percentages 0-100)
//! - `[yolla:wander]` - Roam freely around the screen
//! - `[yolla:follow]` - Follow near the current text output
//! - `[yolla:point x y]` - Point at a screen location
//!
//! ### Expression
//! - `[yolla:mood NAME]` - Set mood (happy, thinking, playful, shy, excited, confused, etc.)
//! - `[yolla:size SIZE]` - Set size (tiny, small, medium, large)
//! - `[yolla:hide]` / `[yolla:show]` - Visibility control
//!
//! ### Animations & Gestures
//! - `[yolla:wave]` - Friendly wave
//! - `[yolla:bounce]` - Excited bounce
//! - `[yolla:spin]` - Playful spin
//! - `[yolla:dance]` - Happy dance
//! - `[yolla:swim]` - Swimming motion
//! - `[yolla:peek DIR]` - Peek from edge
//! - `[yolla:nod]` - Agreeing nod
//! - `[yolla:shake]` - Disagreeing shake
//!
//! ### Reactions (contextual animations)
//! - `[yolla:react laugh]` - Laughing reaction
//! - `[yolla:react gasp]` - Surprised gasp
//! - `[yolla:react hmm]` - Pondering
//! - `[yolla:react tada]` - Celebration
//! - `[yolla:react oops]` - Mistake/error
//! - `[yolla:react love]` - Heart eyes
//!
//! ### Future: Custom Sprites
//! - `[yolla:sprite BASE64_DATA]` - Push custom sprite frame
//! - `[yolla:sequence NAME FRAMES]` - Define animation sequence
//!
//! ## Example
//!
//! Agent response: "Hello! [yolla:move center][yolla:mood happy][yolla:wave] How can I help?"
//! Displayed: "Hello! How can I help?"
//! Effect: Avatar moves to center, shows happy face, waves

use std::collections::VecDeque;

/// A command from the agent to control the avatar
#[derive(Clone, Debug, PartialEq)]
pub enum AvatarCommand {
    /// Move to a named position
    MoveTo(Position),
    /// Move to percentage coordinates (0-100, 0-100)
    MoveToPercent(u8, u8),
    /// Point at a screen location (percentage)
    PointAt(u8, u8),
    /// Enable free wandering
    Wander(bool),
    /// Set emotional mood
    Mood(Mood),
    /// Set size
    Size(Size),
    /// Do a gesture/animation
    Gesture(Gesture),
    /// Reaction animation (contextual)
    React(Reaction),
    /// Hide the avatar
    Hide,
    /// Show the avatar
    Show,
    /// Custom sprite data (future)
    CustomSprite(String),
    /// Task-related command
    Task(TaskCommand),
}

/// Commands for background task management
#[derive(Clone, Debug, PartialEq)]
pub enum TaskCommand {
    /// Start a new background task
    Start { agent: String, description: String },
    /// Update task progress
    Progress { task_id: String, percent: u8 },
    /// Mark task as done
    Done { task_id: String },
    /// Mark task as failed
    Fail { task_id: String, reason: String },
    /// Focus/highlight a specific task
    Focus { task_id: String },
    /// Point at a task
    PointAt { task_id: String },
    /// Hover near a task
    Hover { task_id: String },
    /// Celebrate task completion
    Celebrate { task_id: String },
}

/// Named positions
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Position {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
    /// Follow along with the text (near current output)
    Follow,
}

/// Emotional moods
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mood {
    Happy,
    Thinking,
    Playful,
    Shy,
    Excited,
    Confused,
    Calm,
    Curious,
}

/// Size variants
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Size {
    Tiny,
    Small,
    Medium,
    Large,
}

/// Gesture animations
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gesture {
    Wave,
    Nod,
    Shake,
    Peek(PeekDirection),
    Bounce,
    Spin,
    Dance,
    Swim,
    Stretch,
    Yawn,
    Wiggle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PeekDirection {
    Left,
    Right,
    Top,
    Bottom,
}

/// Reaction animations (contextual responses)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Reaction {
    Laugh,  // Ha ha!
    Gasp,   // Surprised!
    Hmm,    // Thinking...
    Tada,   // Celebration!
    Oops,   // Made a mistake
    Love,   // Heart eyes
    Blush,  // Embarrassed
    Wink,   // Playful wink
    Cry,    // Sad tears
    Angry,  // Frustrated
    Sleepy, // Tired
    Dizzy,  // Confused spinning
}

/// Parser for avatar commands embedded in text
pub struct CommandParser {
    /// Pending commands extracted from text
    pub commands: VecDeque<AvatarCommand>,
}

impl CommandParser {
    pub fn new() -> Self {
        Self {
            commands: VecDeque::new(),
        }
    }

    /// Parse text, extract commands, return cleaned text
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
        let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        match parts[0] {
            // Movement
            "move" => self.parse_move(&parts[1..]),
            "wander" => Some(AvatarCommand::Wander(true)),
            "stop" => Some(AvatarCommand::Wander(false)),
            "point" => self.parse_point(&parts[1..]),
            "follow" => Some(AvatarCommand::MoveTo(Position::Follow)),

            // Expression
            "mood" => self.parse_mood(&parts[1..]),
            "size" => self.parse_size(&parts[1..]),
            "hide" => Some(AvatarCommand::Hide),
            "show" => Some(AvatarCommand::Show),

            // Gestures
            "wave" => Some(AvatarCommand::Gesture(Gesture::Wave)),
            "nod" => Some(AvatarCommand::Gesture(Gesture::Nod)),
            "shake" => Some(AvatarCommand::Gesture(Gesture::Shake)),
            "bounce" => Some(AvatarCommand::Gesture(Gesture::Bounce)),
            "spin" => Some(AvatarCommand::Gesture(Gesture::Spin)),
            "dance" => Some(AvatarCommand::Gesture(Gesture::Dance)),
            "swim" => Some(AvatarCommand::Gesture(Gesture::Swim)),
            "stretch" => Some(AvatarCommand::Gesture(Gesture::Stretch)),
            "yawn" => Some(AvatarCommand::Gesture(Gesture::Yawn)),
            "wiggle" => Some(AvatarCommand::Gesture(Gesture::Wiggle)),
            "peek" => self.parse_peek(&parts[1..]),

            // Reactions
            "react" => self.parse_react(&parts[1..]),
            "laugh" => Some(AvatarCommand::React(Reaction::Laugh)),
            "gasp" => Some(AvatarCommand::React(Reaction::Gasp)),
            "tada" => Some(AvatarCommand::React(Reaction::Tada)),
            "oops" => Some(AvatarCommand::React(Reaction::Oops)),
            "love" => Some(AvatarCommand::React(Reaction::Love)),
            "wink" => Some(AvatarCommand::React(Reaction::Wink)),

            // Future: custom sprites
            "sprite" if parts.len() > 1 => Some(AvatarCommand::CustomSprite(parts[1..].join(" "))),

            // Task management
            "task" => self.parse_task(&parts[1..]),

            _ => None,
        }
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

    fn parse_point(&self, args: &[&str]) -> Option<AvatarCommand> {
        if args.len() >= 2 {
            let x: u8 = args[0].parse().ok()?;
            let y: u8 = args[1].parse().ok()?;
            Some(AvatarCommand::PointAt(x.min(100), y.min(100)))
        } else {
            None
        }
    }

    fn parse_react(&self, args: &[&str]) -> Option<AvatarCommand> {
        if args.is_empty() {
            return None;
        }

        let reaction = match args[0] {
            "laugh" | "lol" | "haha" => Reaction::Laugh,
            "gasp" | "surprise" | "wow" => Reaction::Gasp,
            "hmm" | "think" | "ponder" => Reaction::Hmm,
            "tada" | "celebrate" | "yay" => Reaction::Tada,
            "oops" | "error" | "mistake" => Reaction::Oops,
            "love" | "heart" | "adore" => Reaction::Love,
            "blush" | "shy" | "embarrassed" => Reaction::Blush,
            "wink" | "flirt" => Reaction::Wink,
            "cry" | "sad" | "tears" => Reaction::Cry,
            "angry" | "mad" | "frustrated" => Reaction::Angry,
            "sleepy" | "tired" | "zzz" => Reaction::Sleepy,
            "dizzy" | "confused" | "spinning" => Reaction::Dizzy,
            _ => return None,
        };

        Some(AvatarCommand::React(reaction))
    }

    fn parse_move(&self, args: &[&str]) -> Option<AvatarCommand> {
        if args.is_empty() {
            return None;
        }

        // Named position
        match args[0] {
            "tl" | "topleft" | "top-left" => Some(AvatarCommand::MoveTo(Position::TopLeft)),
            "tr" | "topright" | "top-right" => Some(AvatarCommand::MoveTo(Position::TopRight)),
            "bl" | "bottomleft" | "bottom-left" => {
                Some(AvatarCommand::MoveTo(Position::BottomLeft))
            }
            "br" | "bottomright" | "bottom-right" => {
                Some(AvatarCommand::MoveTo(Position::BottomRight))
            }
            "center" | "middle" => Some(AvatarCommand::MoveTo(Position::Center)),
            "follow" => Some(AvatarCommand::MoveTo(Position::Follow)),
            _ => {
                // Try parsing as x y coordinates
                if args.len() >= 2 {
                    let x: u8 = args[0].parse().ok()?;
                    let y: u8 = args[1].parse().ok()?;
                    Some(AvatarCommand::MoveToPercent(x.min(100), y.min(100)))
                } else {
                    None
                }
            }
        }
    }

    fn parse_mood(&self, args: &[&str]) -> Option<AvatarCommand> {
        if args.is_empty() {
            return None;
        }

        let mood = match args[0] {
            "happy" | "joy" => Mood::Happy,
            "thinking" | "think" | "ponder" => Mood::Thinking,
            "playful" | "silly" => Mood::Playful,
            "shy" | "bashful" => Mood::Shy,
            "excited" | "eager" => Mood::Excited,
            "confused" | "puzzled" => Mood::Confused,
            "calm" | "peaceful" => Mood::Calm,
            "curious" | "interested" => Mood::Curious,
            _ => return None,
        };

        Some(AvatarCommand::Mood(mood))
    }

    fn parse_size(&self, args: &[&str]) -> Option<AvatarCommand> {
        if args.is_empty() {
            return None;
        }

        let size = match args[0] {
            "tiny" | "xs" => Size::Tiny,
            "small" | "sm" => Size::Small,
            "medium" | "md" | "normal" => Size::Medium,
            "large" | "lg" | "big" => Size::Large,
            _ => return None,
        };

        Some(AvatarCommand::Size(size))
    }

    fn parse_peek(&self, args: &[&str]) -> Option<AvatarCommand> {
        if args.is_empty() {
            return Some(AvatarCommand::Gesture(Gesture::Peek(PeekDirection::Right)));
        }

        let dir = match args[0] {
            "left" | "l" => PeekDirection::Left,
            "right" | "r" => PeekDirection::Right,
            "top" | "up" | "t" => PeekDirection::Top,
            "bottom" | "down" | "b" => PeekDirection::Bottom,
            _ => PeekDirection::Right,
        };

        Some(AvatarCommand::Gesture(Gesture::Peek(dir)))
    }

    /// Get the next pending command
    pub fn next_command(&mut self) -> Option<AvatarCommand> {
        self.commands.pop_front()
    }

    /// Check if there are pending commands
    pub fn has_commands(&self) -> bool {
        !self.commands.is_empty()
    }
}

impl Default for CommandParser {
    fn default() -> Self {
        Self::new()
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
            Some(AvatarCommand::MoveTo(Position::Center))
        );
    }

    #[test]
    fn test_parse_multiple_commands() {
        let mut parser = CommandParser::new();
        let result = parser.parse("[yolla:mood happy][yolla:move tr]Hi!");
        assert_eq!(result, "Hi!");
        assert_eq!(
            parser.next_command(),
            Some(AvatarCommand::Mood(Mood::Happy))
        );
        assert_eq!(
            parser.next_command(),
            Some(AvatarCommand::MoveTo(Position::TopRight))
        );
    }

    #[test]
    fn test_preserve_normal_brackets() {
        let mut parser = CommandParser::new();
        let result = parser.parse("Array[0] and [other] text");
        assert_eq!(result, "Array[0] and [other] text");
        assert!(!parser.has_commands());
    }
}
