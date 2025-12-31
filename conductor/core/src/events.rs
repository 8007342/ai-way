//! Surface Events
//!
//! Events sent from UI surfaces to the Conductor. These represent all the ways
//! a UI can communicate user actions and state changes to the orchestration layer.
//!
//! # Design Philosophy
//!
//! UI surfaces are "dumb" renderers that forward user actions to the Conductor.
//! They don't interpret what actions mean - they just report what happened.
//! The Conductor decides how to respond.

use serde::{Deserialize, Serialize};

use crate::messages::{EventId, MessageId};
use crate::tasks::TaskId;

/// Events from UI Surface to Conductor
///
/// These events tell the Conductor what the user is doing or what's happening
/// in the UI. The Conductor responds with ConductorMessages.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SurfaceEvent {
    // ============================================
    // Connection Events
    // ============================================
    /// Surface connected to Conductor
    Connected {
        /// Event ID for acknowledgment
        event_id: EventId,
        /// Surface type identifier
        surface_type: SurfaceType,
        /// Surface capabilities
        capabilities: SurfaceCapabilities,
    },

    /// Handshake request (first message after transport connect)
    ///
    /// Sent immediately after transport connection is established.
    /// The Conductor responds with HandshakeAck.
    Handshake {
        /// Event ID for acknowledgment
        event_id: EventId,
        /// Protocol version (current: 1)
        protocol_version: u32,
        /// Surface type identifier
        surface_type: SurfaceType,
        /// Surface capabilities
        capabilities: SurfaceCapabilities,
        /// Optional authentication token (for remote surfaces)
        auth_token: Option<String>,
    },

    /// Heartbeat response (reply to Ping)
    Pong {
        /// Sequence number (echoed from Ping)
        seq: u64,
    },

    /// Surface disconnecting gracefully
    Disconnected {
        /// Event ID for acknowledgment
        event_id: EventId,
        /// Reason for disconnect (optional)
        reason: Option<String>,
    },

    /// Surface window/viewport resized
    Resized {
        /// Event ID for acknowledgment
        event_id: EventId,
        /// New width (in surface-specific units)
        width: u32,
        /// New height (in surface-specific units)
        height: u32,
    },

    // ============================================
    // User Input Events
    // ============================================
    /// User submitted a message
    UserMessage {
        /// Event ID for acknowledgment
        event_id: EventId,
        /// The message content
        content: String,
    },

    /// User executed a command (e.g., /help, /quit)
    UserCommand {
        /// Event ID for acknowledgment
        event_id: EventId,
        /// Command name (without leading /)
        command: String,
        /// Command arguments
        args: Vec<String>,
    },

    /// User is typing (for real-time feedback)
    UserTyping {
        /// Whether user is currently typing
        typing: bool,
    },

    /// User scrolled the conversation
    UserScrolled {
        /// Direction of scroll
        direction: ScrollDirection,
        /// Amount scrolled (surface-specific units)
        amount: u32,
    },

    // ============================================
    // Interaction Events
    // ============================================
    /// User clicked/tapped the avatar
    AvatarClicked {
        /// Event ID for acknowledgment
        event_id: EventId,
    },

    /// User clicked/tapped a task
    TaskClicked {
        /// Event ID for acknowledgment
        event_id: EventId,
        /// Which task was clicked
        task_id: TaskId,
    },

    /// User clicked/tapped a message
    MessageClicked {
        /// Event ID for acknowledgment
        event_id: EventId,
        /// Which message was clicked
        message_id: MessageId,
    },

    // ============================================
    // Acknowledgment Events
    // ============================================
    /// Surface received a message from Conductor
    MessageReceived {
        /// Message ID that was received
        message_id: MessageId,
    },

    /// Surface completed rendering
    RenderComplete {
        /// Frame number or timestamp
        frame: u64,
    },

    // ============================================
    // Capability Response
    // ============================================
    /// Response to QueryCapabilities
    CapabilitiesReport {
        /// Event ID for acknowledgment
        event_id: EventId,
        /// Surface capabilities
        capabilities: SurfaceCapabilities,
    },

    // ============================================
    // Lifecycle Events
    // ============================================
    /// User requested quit
    QuitRequested {
        /// Event ID for acknowledgment
        event_id: EventId,
    },

    /// Surface encountered an error
    SurfaceError {
        /// Event ID for acknowledgment
        event_id: EventId,
        /// Error description
        error: String,
        /// Whether the surface can continue
        recoverable: bool,
    },
}

impl SurfaceEvent {
    /// Generate a new event ID for this event
    pub fn new_event_id() -> EventId {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        EventId(format!("evt_{}", id))
    }

    /// Get the event ID if this event has one
    pub fn event_id(&self) -> Option<&EventId> {
        match self {
            Self::Connected { event_id, .. }
            | Self::Handshake { event_id, .. }
            | Self::Disconnected { event_id, .. }
            | Self::Resized { event_id, .. }
            | Self::UserMessage { event_id, .. }
            | Self::UserCommand { event_id, .. }
            | Self::AvatarClicked { event_id }
            | Self::TaskClicked { event_id, .. }
            | Self::MessageClicked { event_id, .. }
            | Self::CapabilitiesReport { event_id, .. }
            | Self::QuitRequested { event_id }
            | Self::SurfaceError { event_id, .. } => Some(event_id),
            Self::Pong { .. }
            | Self::UserTyping { .. }
            | Self::UserScrolled { .. }
            | Self::MessageReceived { .. }
            | Self::RenderComplete { .. } => None,
        }
    }
}

/// Type of UI surface
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceType {
    /// Terminal UI (ratatui/crossterm)
    Tui,
    /// Web browser UI
    Web,
    /// Native desktop GUI
    Desktop,
    /// Mobile app
    Mobile,
    /// Headless (for testing/automation)
    Headless,
    /// Custom surface type
    Custom(String),
}

impl SurfaceType {
    /// Human-readable name
    pub fn name(&self) -> &str {
        match self {
            Self::Tui => "Terminal",
            Self::Web => "Web",
            Self::Desktop => "Desktop",
            Self::Mobile => "Mobile",
            Self::Headless => "Headless",
            Self::Custom(name) => name,
        }
    }
}

/// Capabilities that a surface can support
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SurfaceCapabilities {
    /// Can display colored text
    pub color: bool,
    /// Can display the avatar
    pub avatar: bool,
    /// Can display avatar animations
    pub avatar_animations: bool,
    /// Can display the task panel
    pub tasks: bool,
    /// Can handle streaming tokens (vs batched messages)
    pub streaming: bool,
    /// Can display images
    pub images: bool,
    /// Can play sounds
    pub audio: bool,
    /// Can display rich text (markdown, etc.)
    pub rich_text: bool,
    /// Can handle mouse/touch input
    pub pointer_input: bool,
    /// Can handle keyboard input
    pub keyboard_input: bool,
    /// Supports copy/paste
    pub clipboard: bool,
    /// Maximum text width (0 = unlimited)
    pub max_width: u32,
    /// Maximum text height (0 = unlimited)
    pub max_height: u32,
}

impl SurfaceCapabilities {
    /// Create capabilities for a standard TUI
    pub fn tui() -> Self {
        Self {
            color: true,
            avatar: true,
            avatar_animations: true,
            tasks: true,
            streaming: true,
            images: false,
            audio: false,
            rich_text: false, // Terminal doesn't do markdown natively
            pointer_input: true,
            keyboard_input: true,
            clipboard: false, // Depends on terminal
            max_width: 0,
            max_height: 0,
        }
    }

    /// Create capabilities for a web UI
    pub fn web() -> Self {
        Self {
            color: true,
            avatar: true,
            avatar_animations: true,
            tasks: true,
            streaming: true,
            images: true,
            audio: true,
            rich_text: true,
            pointer_input: true,
            keyboard_input: true,
            clipboard: true,
            max_width: 0,
            max_height: 0,
        }
    }

    /// Create minimal capabilities for headless/testing
    pub fn headless() -> Self {
        Self {
            color: false,
            avatar: false,
            avatar_animations: false,
            tasks: true,
            streaming: true,
            images: false,
            audio: false,
            rich_text: false,
            pointer_input: false,
            keyboard_input: true,
            clipboard: false,
            max_width: 80,
            max_height: 24,
        }
    }
}

/// Scroll direction
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScrollDirection {
    /// Scroll up (see older content)
    Up,
    /// Scroll down (see newer content)
    Down,
    /// Jump to top
    Top,
    /// Jump to bottom (latest)
    Bottom,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_id_unique() {
        let id1 = SurfaceEvent::new_event_id();
        let id2 = SurfaceEvent::new_event_id();
        assert_ne!(id1.0, id2.0);
    }

    #[test]
    fn test_surface_capabilities_tui() {
        let caps = SurfaceCapabilities::tui();
        assert!(caps.color);
        assert!(caps.avatar);
        assert!(caps.streaming);
        assert!(!caps.images);
    }

    #[test]
    fn test_surface_type_name() {
        assert_eq!(SurfaceType::Tui.name(), "Terminal");
        assert_eq!(SurfaceType::Web.name(), "Web");
        assert_eq!(SurfaceType::Custom("MyUI".to_string()).name(), "MyUI");
    }
}
