//! Avatar State Machine
//!
//! The state machine drives avatar behavior based on application events.
//! It determines which animation to play and what size the avatar should be.

use std::time::{Duration, Instant};

use super::sizes::AvatarSize;

/// Avatar states
#[derive(Clone, Debug, PartialEq)]
pub enum AvatarState {
    /// Default idle state
    Idle,
    /// Processing a query
    Thinking,
    /// Streaming a response
    Responding,
    /// Waiting for user input
    WaitingForInput { since: Instant },
    /// Celebrating success
    Celebrating { until: Instant },
    /// Being playful
    Playful { until: Instant },
    /// Error occurred
    Error { message: String, until: Instant },
    /// Background work (tiny, unobtrusive)
    BackgroundWork,
    /// Peeking at content
    Peeking { direction: Direction },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Left,
    Right,
}

/// Events that can trigger state transitions
#[derive(Clone, Debug)]
pub enum AvatarTrigger {
    /// User started typing
    UserTyping,
    /// User submitted a query
    QuerySubmitted,
    /// Response streaming started
    ResponseStarted,
    /// Response completed
    ResponseCompleted { success: bool },
    /// Error occurred
    ErrorOccurred { message: String },
    /// User scrolled
    UserScrolled,
    /// Idle timeout
    IdleTimeout,
    /// Window resized
    WindowResized,
}

/// State machine for avatar behavior
pub struct AvatarStateMachine {
    /// Current state
    state: AvatarState,
    /// When the current state was entered
    state_entered: Instant,
    /// Playfulness level (0.0 = serious, 1.0 = very playful)
    playfulness: f32,
}

impl AvatarStateMachine {
    /// Create a new state machine
    pub fn new() -> Self {
        Self {
            state: AvatarState::Idle,
            state_entered: Instant::now(),
            playfulness: 0.7, // Default: fairly playful
        }
    }

    /// Process a trigger event
    pub fn trigger(&mut self, trigger: AvatarTrigger) {
        let new_state = self.compute_transition(&trigger);

        if new_state != self.state {
            self.state = new_state;
            self.state_entered = Instant::now();
        }
    }

    fn compute_transition(&self, trigger: &AvatarTrigger) -> AvatarState {
        use AvatarState::*;
        use AvatarTrigger::*;

        match trigger {
            // Error takes priority
            ErrorOccurred { message } => Error {
                message: message.clone(),
                until: Instant::now() + Duration::from_secs(5),
            },

            // Query flow
            QuerySubmitted => Thinking,
            ResponseStarted => Responding,

            ResponseCompleted { success: true } => {
                // Maybe celebrate
                if rand::random::<f32>() < self.playfulness * 0.3 {
                    Celebrating {
                        until: Instant::now() + Duration::from_secs(2),
                    }
                } else {
                    WaitingForInput {
                        since: Instant::now(),
                    }
                }
            }

            ResponseCompleted { success: false } => WaitingForInput {
                since: Instant::now(),
            },

            // User typing resets waiting timer
            UserTyping => match &self.state {
                WaitingForInput { .. } | Idle => WaitingForInput {
                    since: Instant::now(),
                },
                other => other.clone(),
            },

            // Scrolling - peek if idle
            UserScrolled => match &self.state {
                Idle => Peeking {
                    direction: Direction::Right,
                },
                other => other.clone(),
            },

            // Idle timeout - maybe get playful
            IdleTimeout => match &self.state {
                WaitingForInput { since } if since.elapsed() > Duration::from_secs(30) => {
                    if rand::random::<f32>() < self.playfulness {
                        Playful {
                            until: Instant::now() + Duration::from_secs(3),
                        }
                    } else {
                        Idle
                    }
                }
                _ => self.state.clone(),
            },

            // Window resize - return to idle
            WindowResized => Idle,
        }
    }

    /// Get current state
    pub fn state(&self) -> &AvatarState {
        &self.state
    }

    /// Get recommended animation for current state
    pub fn recommended_animation(&self) -> &str {
        match &self.state {
            AvatarState::Idle => "idle",
            AvatarState::Thinking => "thinking",
            AvatarState::Responding => "talking",
            AvatarState::WaitingForInput { since } => {
                if since.elapsed() > Duration::from_secs(10) {
                    "waiting"
                } else {
                    "idle"
                }
            }
            AvatarState::Celebrating { .. } => "happy",
            AvatarState::Playful { .. } => "joking",
            AvatarState::Error { .. } => "error",
            AvatarState::BackgroundWork => "idle",
            AvatarState::Peeking { .. } => "idle",
        }
    }

    /// Get recommended size for current state
    pub fn recommended_size(&self) -> AvatarSize {
        match &self.state {
            AvatarState::BackgroundWork => AvatarSize::Tiny,
            AvatarState::Peeking { .. } => AvatarSize::Small,
            AvatarState::Idle | AvatarState::WaitingForInput { .. } => AvatarSize::Medium,
            AvatarState::Thinking | AvatarState::Responding => AvatarSize::Medium,
            AvatarState::Celebrating { .. } | AvatarState::Playful { .. } => AvatarSize::Large,
            AvatarState::Error { .. } => AvatarSize::Medium,
        }
    }

    /// Whether avatar should be in front of content
    pub fn should_be_foreground(&self) -> bool {
        matches!(
            self.state,
            AvatarState::Celebrating { .. }
                | AvatarState::Playful { .. }
                | AvatarState::Error { .. }
        )
    }

    /// Set playfulness level
    pub fn set_playfulness(&mut self, level: f32) {
        self.playfulness = level.clamp(0.0, 1.0);
    }
}

impl Default for AvatarStateMachine {
    fn default() -> Self {
        Self::new()
    }
}
