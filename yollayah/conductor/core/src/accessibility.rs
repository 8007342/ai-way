//! Accessibility Support
//!
//! Provides accessibility metadata for `ConductorMessage` types, enabling screen readers,
//! voice interfaces, and other assistive technologies to meaningfully interpret
//! the Conductor's output.
//!
//! # Design Philosophy
//!
//! Visual interfaces communicate through animations, colors, and spatial positioning.
//! Non-visual interfaces (screen readers, voice assistants, braille displays) need
//! explicit text descriptions of these visual states.
//!
//! This module provides:
//! - Screen reader announcements for avatar state changes
//! - ARIA role suggestions for UI elements
//! - Urgency levels for notifications (to control interruption behavior)
//! - Semantic descriptions for all visual-only feedback
//!
//! # Example
//!
//! ```ignore
//! use conductor_core::{ConductorMessage, accessibility::Accessible};
//!
//! let msg = ConductorMessage::AvatarMood { mood: AvatarMood::Thinking };
//! if let Some(announcement) = msg.screen_reader_announcement() {
//!     // "Yollayah is thinking carefully"
//!     speak(announcement);
//! }
//! ```

use crate::avatar::{AvatarGesture, AvatarMood, AvatarReaction};
use crate::messages::{ConductorMessage, ConductorState, NotifyLevel};

/// Accessibility trait for `ConductorMessage` types
///
/// Provides optional accessibility metadata that surfaces can use
/// to support assistive technologies.
pub trait Accessible {
    /// Text announcement suitable for screen readers
    ///
    /// Returns None if no announcement is needed (e.g., internal messages).
    fn screen_reader_announcement(&self) -> Option<String>;

    /// ARIA role for UI elements
    ///
    /// Helps screen readers understand the semantic purpose of UI elements.
    /// See: <https://www.w3.org/TR/wai-aria-1.2/#role_definitions>
    fn aria_role(&self) -> Option<&'static str>;

    /// Urgency level for interrupt behavior
    ///
    /// Determines whether this message should interrupt the user.
    fn urgency(&self) -> Urgency;
}

/// Urgency levels for accessibility announcements
///
/// Maps to ARIA live region politeness levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Urgency {
    /// Immediate announcement - interrupts current speech
    /// Use for errors, critical alerts, and state changes requiring attention
    Immediate,
    /// Normal announcement - queued after current speech
    /// Use for regular messages and responses
    Normal,
    /// Low priority - announced when convenient
    /// Use for status updates and background information
    Low,
    /// No announcement needed
    /// Use for internal bookkeeping messages
    None,
}

impl Urgency {
    /// ARIA live region value
    #[must_use]
    pub fn aria_live(&self) -> &'static str {
        match self {
            Urgency::Immediate => "assertive",
            Urgency::Normal | Urgency::Low => "polite",
            Urgency::None => "off",
        }
    }
}

impl Accessible for ConductorMessage {
    fn screen_reader_announcement(&self) -> Option<String> {
        match self {
            // Conversation messages
            ConductorMessage::Message { role, content, .. } => {
                let prefix = match role {
                    crate::messages::MessageRole::User => "You said",
                    crate::messages::MessageRole::Assistant => "Yollayah says",
                    crate::messages::MessageRole::System => "System message",
                };
                Some(format!("{prefix}: {content}"))
            }

            ConductorMessage::StreamEnd { final_content, .. } => {
                Some(format!("Yollayah says: {final_content}"))
            }

            ConductorMessage::StreamError { error, .. } => Some(format!("Error: {error}")),

            // Avatar state changes need explicit announcements
            ConductorMessage::AvatarMood { mood } => {
                Some(format!("Yollayah is {}", mood.accessibility_description()))
            }

            ConductorMessage::AvatarGesture { gesture, .. } => {
                Some(gesture.accessibility_description().to_string())
            }

            ConductorMessage::AvatarReact { reaction, .. } => {
                Some(reaction.accessibility_description().to_string())
            }

            ConductorMessage::AvatarVisibility { visible } => {
                if *visible {
                    Some("Yollayah appears".to_string())
                } else {
                    Some("Yollayah hides".to_string())
                }
            }

            // Task updates
            ConductorMessage::TaskCreated {
                agent, description, ..
            } => Some(format!("{agent} is now working on: {description}")),

            ConductorMessage::TaskCompleted { summary, .. } => {
                let msg = summary.as_ref().map_or_else(
                    || "Task completed successfully".to_string(),
                    |s| format!("Task completed: {s}"),
                );
                Some(msg)
            }

            ConductorMessage::TaskFailed { error, .. } => Some(format!("Task failed: {error}")),

            ConductorMessage::TaskUpdated {
                progress,
                status_message,
                ..
            } => {
                let msg = status_message.as_ref().map_or_else(
                    || format!("{progress}% complete"),
                    |s| format!("{progress}% complete: {s}"),
                );
                Some(msg)
            }

            // System messages
            ConductorMessage::Notify {
                level,
                title,
                message,
            } => {
                let prefix = match level {
                    NotifyLevel::Error => "Error",
                    NotifyLevel::Warning => "Warning",
                    NotifyLevel::Success => "Success",
                    NotifyLevel::Info => "Notice",
                };
                let announcement = match title {
                    Some(t) => format!("{prefix}: {t} - {message}"),
                    None => format!("{prefix}: {message}"),
                };
                Some(announcement)
            }

            ConductorMessage::State { state } => {
                Some(state.accessibility_description().to_string())
            }

            ConductorMessage::Quit { message } => {
                let msg = message.as_ref().map_or_else(
                    || "Yollayah is leaving".to_string(),
                    |m| format!("Goodbye: {m}"),
                );
                Some(msg)
            }

            // Layout hints
            ConductorMessage::LayoutHint { directive } => {
                use crate::messages::{LayoutDirective, PanelId};
                match directive {
                    LayoutDirective::ShowPanel { panel } => {
                        let panel_name = match panel {
                            PanelId::Tasks => "Tasks",
                            PanelId::Developer => "Developer",
                            PanelId::Settings => "Settings",
                            PanelId::History => "History",
                        };
                        Some(format!("{panel_name} panel opened"))
                    }
                    LayoutDirective::HidePanel { panel } => {
                        let panel_name = match panel {
                            PanelId::Tasks => "Tasks",
                            PanelId::Developer => "Developer",
                            PanelId::Settings => "Settings",
                            PanelId::History => "History",
                        };
                        Some(format!("{panel_name} panel closed"))
                    }
                    LayoutDirective::FocusInput => Some("Input focused".to_string()),
                    LayoutDirective::ToggleDeveloperMode => {
                        Some("Developer mode toggled".to_string())
                    }
                    // Scroll actions don't need announcements
                    LayoutDirective::ScrollToMessage { .. }
                    | LayoutDirective::ScrollToTask { .. } => None,
                }
            }

            // Multi-conversation messages
            ConductorMessage::ConversationCreated { agent_name, .. } => {
                let name = agent_name.as_ref().map_or("Yollayah", |n| n.as_str());
                Some(format!("New conversation started with {name}"))
            }

            ConductorMessage::ConversationFocused { .. } => {
                Some("Conversation focused".to_string())
            }

            ConductorMessage::ConversationStateChanged { .. } => None, // Too noisy

            ConductorMessage::ConversationStreamEnd { final_content, .. } => {
                Some(format!("Agent says: {final_content}"))
            }

            ConductorMessage::SummaryReady { summary, .. } => {
                Some(format!("Summary ready: {summary}"))
            }

            ConductorMessage::ConversationRemoved { .. } => Some("Conversation closed".to_string()),

            // Internal/transport messages - no announcement needed
            ConductorMessage::Token { .. }
            | ConductorMessage::ConversationStreamToken { .. }
            | ConductorMessage::QueryCapabilities
            | ConductorMessage::Ack { .. }
            | ConductorMessage::SessionInfo { .. }
            | ConductorMessage::HandshakeAck { .. }
            | ConductorMessage::Ping { .. }
            | ConductorMessage::StateSnapshot { .. }
            | ConductorMessage::AvatarMoveTo { .. }
            | ConductorMessage::AvatarSize { .. }
            | ConductorMessage::AvatarWander { .. }
            | ConductorMessage::AvatarPointAt { .. }
            | ConductorMessage::TaskFocus { .. } => None,
        }
    }

    fn aria_role(&self) -> Option<&'static str> {
        match self {
            ConductorMessage::Notify { level, .. } => match level {
                NotifyLevel::Error | NotifyLevel::Warning => Some("alert"),
                _ => Some("status"),
            },
            ConductorMessage::State { .. } => Some("status"),
            ConductorMessage::TaskUpdated { .. } => Some("progressbar"),
            ConductorMessage::Message { .. } | ConductorMessage::StreamEnd { .. } => {
                Some("article")
            }
            _ => None,
        }
    }

    fn urgency(&self) -> Urgency {
        match self {
            ConductorMessage::Notify {
                level: NotifyLevel::Error,
                ..
            } => Urgency::Immediate,
            ConductorMessage::StreamError { .. } | ConductorMessage::TaskFailed { .. } => {
                Urgency::Immediate
            }
            ConductorMessage::Notify { .. }
            | ConductorMessage::Message { .. }
            | ConductorMessage::StreamEnd { .. }
            | ConductorMessage::TaskCompleted { .. } => Urgency::Normal,
            ConductorMessage::TaskUpdated { .. }
            | ConductorMessage::State { .. }
            | ConductorMessage::AvatarMood { .. }
            | ConductorMessage::AvatarGesture { .. }
            | ConductorMessage::AvatarReact { .. } => Urgency::Low,
            _ => Urgency::None,
        }
    }
}

// Accessibility descriptions for avatar types
impl AvatarMood {
    /// Human-readable description for screen readers
    #[must_use]
    pub fn accessibility_description(&self) -> &'static str {
        match self {
            AvatarMood::Happy => "feeling happy and content",
            AvatarMood::Thinking => "thinking carefully",
            AvatarMood::Excited => "excited and enthusiastic",
            AvatarMood::Confused => "confused or puzzled",
            AvatarMood::Calm => "calm and relaxed",
            AvatarMood::Curious => "curious and interested",
            AvatarMood::Playful => "playful and silly",
            AvatarMood::Shy => "shy and reserved",
        }
    }
}

impl AvatarGesture {
    /// Human-readable description for screen readers
    #[must_use]
    pub fn accessibility_description(&self) -> &'static str {
        match self {
            AvatarGesture::Wave => "Yollayah waves hello",
            AvatarGesture::Nod => "Yollayah nods in agreement",
            AvatarGesture::Shake => "Yollayah shakes head",
            AvatarGesture::Peek(_) => "Yollayah peeks around",
            AvatarGesture::Bounce => "Yollayah bounces with excitement",
            AvatarGesture::Spin => "Yollayah spins around",
            AvatarGesture::Dance => "Yollayah does a happy dance",
            AvatarGesture::Swim => "Yollayah swims happily",
            AvatarGesture::Stretch => "Yollayah stretches",
            AvatarGesture::Yawn => "Yollayah yawns",
            AvatarGesture::Wiggle => "Yollayah wiggles playfully",
        }
    }
}

impl AvatarReaction {
    /// Human-readable description for screen readers
    #[must_use]
    pub fn accessibility_description(&self) -> &'static str {
        match self {
            AvatarReaction::Laugh => "Yollayah laughs",
            AvatarReaction::Gasp => "Yollayah gasps with surprise",
            AvatarReaction::Hmm => "Yollayah thinks about that",
            AvatarReaction::Tada => "Yollayah celebrates",
            AvatarReaction::Oops => "Yollayah made a mistake",
            AvatarReaction::Love => "Yollayah shows affection",
            AvatarReaction::Blush => "Yollayah is embarrassed",
            AvatarReaction::Wink => "Yollayah winks playfully",
            AvatarReaction::Cry => "Yollayah is sad",
            AvatarReaction::Angry => "Yollayah is frustrated",
            AvatarReaction::Sleepy => "Yollayah is sleepy",
            AvatarReaction::Dizzy => "Yollayah is dizzy",
        }
    }
}

impl ConductorState {
    /// Human-readable description for screen readers
    #[must_use]
    pub fn accessibility_description(&self) -> &'static str {
        match self {
            ConductorState::Initializing => "Yollayah is starting up",
            ConductorState::Ready => "Yollayah is ready to help",
            ConductorState::Thinking => "Yollayah is thinking about your question",
            ConductorState::Responding => "Yollayah is responding",
            ConductorState::Listening => "Yollayah is listening",
            ConductorState::Error => "Yollayah encountered an error",
            ConductorState::ShuttingDown => "Yollayah is shutting down",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::MessageRole;

    #[test]
    fn test_message_announcement() {
        let msg = ConductorMessage::Message {
            id: crate::messages::MessageId::new(),
            role: MessageRole::Assistant,
            content: "Hello world".to_string(),
            content_type: crate::messages::ContentType::Plain,
        };
        let announcement = msg.screen_reader_announcement().unwrap();
        assert!(announcement.contains("Yollayah says"));
        assert!(announcement.contains("Hello world"));
    }

    #[test]
    fn test_error_urgency() {
        let msg = ConductorMessage::Notify {
            level: NotifyLevel::Error,
            title: None,
            message: "Something went wrong".to_string(),
        };
        assert_eq!(msg.urgency(), Urgency::Immediate);
        assert_eq!(msg.aria_role(), Some("alert"));
    }

    #[test]
    fn test_avatar_mood_announcement() {
        let msg = ConductorMessage::AvatarMood {
            mood: AvatarMood::Thinking,
        };
        let announcement = msg.screen_reader_announcement().unwrap();
        assert!(announcement.contains("thinking carefully"));
    }

    #[test]
    fn test_token_no_announcement() {
        let msg = ConductorMessage::Token {
            message_id: crate::messages::MessageId::new(),
            text: "partial".to_string(),
        };
        assert!(msg.screen_reader_announcement().is_none());
        assert_eq!(msg.urgency(), Urgency::None);
    }

    #[test]
    fn test_task_completed_announcement() {
        let msg = ConductorMessage::TaskCompleted {
            task_id: crate::tasks::TaskId::new("test"),
            summary: Some("Analysis complete".to_string()),
        };
        let announcement = msg.screen_reader_announcement().unwrap();
        assert!(announcement.contains("completed"));
        assert!(announcement.contains("Analysis complete"));
    }
}
