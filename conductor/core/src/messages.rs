//! Conductor Messages
//!
//! Messages sent from the Conductor to UI surfaces. These represent all the ways
//! the orchestration layer can communicate with any connected UI (TUI, `WebUI`, GUI, etc.).
//!
//! # Design Philosophy
//!
//! The Conductor is the "brain" that orchestrates AI interactions, avatar behavior,
//! and task management. UI surfaces are pure renderers that display what the Conductor
//! tells them to. This separation enables:
//!
//! - Hot-swappable UI surfaces (switch from TUI to `WebUI` mid-session)
//! - Multiple simultaneous surfaces (TUI + mobile notification)
//! - Headless operation for testing and automation
//! - Clean separation of concerns

use serde::{Deserialize, Serialize};

use crate::avatar::{AvatarGesture, AvatarMood, AvatarPosition, AvatarReaction, AvatarSize};
use crate::tasks::TaskId;

/// Messages from Conductor to UI Surface
///
/// These messages tell the UI what to display and how to behave.
/// The UI should not have any business logic - just render what it's told.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConductorMessage {
    // ============================================
    // Conversation Messages
    // ============================================
    /// A complete message to display
    Message {
        /// Unique message ID for tracking
        id: MessageId,
        /// Who sent this message
        role: MessageRole,
        /// The message content
        content: String,
        /// Content type hint for rendering
        #[serde(default)]
        content_type: ContentType,
    },

    /// A streaming token (partial response)
    Token {
        /// Message ID this token belongs to
        message_id: MessageId,
        /// The token text
        text: String,
    },

    /// Stream has completed
    StreamEnd {
        /// Message ID that completed
        message_id: MessageId,
        /// Final complete content (may differ from concatenated tokens due to cleanup)
        final_content: String,
    },

    /// Stream encountered an error
    StreamError {
        /// Message ID that errored
        message_id: MessageId,
        /// Error description
        error: String,
    },

    // ============================================
    // Avatar Directives
    // ============================================
    /// Move avatar to a position
    AvatarMoveTo {
        /// Target position
        position: AvatarPosition,
    },

    /// Set avatar mood/expression
    AvatarMood {
        /// The mood to display
        mood: AvatarMood,
    },

    /// Set avatar size
    AvatarSize {
        /// The size to use
        size: AvatarSize,
    },

    /// Perform a gesture animation
    AvatarGesture {
        /// The gesture to perform
        gesture: AvatarGesture,
        /// Duration in milliseconds (0 = default)
        duration_ms: u32,
    },

    /// Perform a reaction animation
    AvatarReact {
        /// The reaction to show
        reaction: AvatarReaction,
        /// Duration in milliseconds (0 = default)
        duration_ms: u32,
    },

    /// Show/hide the avatar
    AvatarVisibility {
        /// Whether avatar should be visible
        visible: bool,
    },

    /// Enable/disable wandering behavior
    AvatarWander {
        /// Whether wandering is enabled
        enabled: bool,
    },

    /// Point at something on screen
    AvatarPointAt {
        /// X position (0-100 percentage)
        x_percent: u8,
        /// Y position (0-100 percentage)
        y_percent: u8,
    },

    // ============================================
    // Task Directives
    // ============================================
    /// A new task was created
    TaskCreated {
        /// Task identifier
        task_id: TaskId,
        /// Agent handling the task
        agent: String,
        /// Human-readable description
        description: String,
    },

    /// Task progress updated
    TaskUpdated {
        /// Task identifier
        task_id: TaskId,
        /// Progress percentage (0-100)
        progress: u8,
        /// Optional status message
        status_message: Option<String>,
    },

    /// Task completed successfully
    TaskCompleted {
        /// Task identifier
        task_id: TaskId,
        /// Result summary
        summary: Option<String>,
    },

    /// Task failed
    TaskFailed {
        /// Task identifier
        task_id: TaskId,
        /// Error message
        error: String,
    },

    /// Focus UI on a specific task
    TaskFocus {
        /// Task identifier
        task_id: TaskId,
    },

    // ============================================
    // Layout Directives
    // ============================================
    /// Layout hint for surface UI organization
    LayoutHint {
        /// The layout directive to apply
        directive: LayoutDirective,
    },

    // ============================================
    // System Messages
    // ============================================
    /// System notification
    Notify {
        /// Notification level
        level: NotifyLevel,
        /// Title (optional)
        title: Option<String>,
        /// Message content
        message: String,
    },

    /// Conductor state change
    State {
        /// The new state
        state: ConductorState,
    },

    /// Query surface capabilities
    QueryCapabilities,

    /// Acknowledgment of received event
    Ack {
        /// Event ID being acknowledged
        event_id: EventId,
    },

    /// Session information
    SessionInfo {
        /// Session ID
        session_id: SessionId,
        /// Model being used
        model: String,
        /// Whether warmup is complete
        ready: bool,
    },

    /// Request surface to quit
    Quit {
        /// Optional goodbye message
        message: Option<String>,
    },

    // ============================================
    // Transport/Handshake Messages
    // ============================================
    /// Handshake acknowledgment (response to Handshake event)
    ///
    /// Sent by the Conductor after receiving a Handshake event.
    HandshakeAck {
        /// Whether the handshake was accepted
        accepted: bool,
        /// Connection ID assigned by Conductor (unique per session)
        connection_id: String,
        /// Reason for rejection (if not accepted)
        rejection_reason: Option<String>,
        /// Protocol version supported by Conductor
        protocol_version: u32,
    },

    /// Heartbeat request
    ///
    /// Sent periodically to detect dead connections.
    /// Surface should respond with Pong event.
    Ping {
        /// Sequence number (surface echoes this back)
        seq: u64,
    },
}

/// Message identifier
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub String);

impl MessageId {
    /// Generate a new unique message ID
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        Self(format!("msg_{id}"))
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

/// Event identifier (for acks)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub String);

/// Session identifier
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    /// Generate a new unique session ID
    ///
    /// Uses an atomic counter combined with timestamp to ensure uniqueness
    /// even when multiple sessions are created in the same millisecond.
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::{SystemTime, UNIX_EPOCH};

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let count = COUNTER.fetch_add(1, Ordering::SeqCst);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        Self(format!("session_{timestamp}_{count}"))
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Who sent a message
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    /// User input
    User,
    /// AI assistant (Yollayah)
    Assistant,
    /// System message
    System,
}

/// Content type hints for message rendering
///
/// Tells UI surfaces how to render the message content appropriately.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ContentType {
    /// Plain text content
    #[default]
    Plain,
    /// Markdown-formatted content
    Markdown,
    /// Code content with optional language hint
    Code {
        /// Programming language for syntax highlighting (e.g., "rust", "python")
        language: Option<String>,
    },
    /// Error message content
    Error,
    /// System-level message content
    System,
    /// Quoted content (e.g., from another source)
    Quote,
}

/// Panel identifiers for layout orchestration
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PanelId {
    /// Tasks panel showing active/completed tasks
    Tasks,
    /// Developer panel for debugging/inspection
    Developer,
    /// Settings panel for configuration
    Settings,
    /// History panel for conversation history
    History,
}

/// Layout directives for controlling UI surface organization
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LayoutDirective {
    /// Show a specific panel
    ShowPanel {
        /// The panel to show
        panel: PanelId,
    },
    /// Hide a specific panel
    HidePanel {
        /// The panel to hide
        panel: PanelId,
    },
    /// Focus the input field
    FocusInput,
    /// Scroll to a specific message
    ScrollToMessage {
        /// The message ID to scroll to
        message_id: MessageId,
    },
    /// Scroll to a specific task
    ScrollToTask {
        /// The task ID to scroll to
        task_id: String,
    },
    /// Toggle developer mode on/off
    ToggleDeveloperMode,
}

/// Notification levels
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotifyLevel {
    /// Informational
    Info,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Success
    Success,
}

/// Conductor operational states
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConductorState {
    /// Starting up, not ready
    Initializing,
    /// Warming up the model
    WarmingUp,
    /// Ready for input
    Ready,
    /// Processing a query
    Thinking,
    /// Streaming a response
    Responding,
    /// Waiting for user input
    Listening,
    /// An error occurred
    Error,
    /// Shutting down
    ShuttingDown,
}

impl ConductorState {
    /// Human-readable description
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Initializing => "Starting up...",
            Self::WarmingUp => "Loading model...",
            Self::Ready => "Ready",
            Self::Thinking => "Thinking...",
            Self::Responding => "Responding...",
            Self::Listening => "Listening",
            Self::Error => "Error",
            Self::ShuttingDown => "Shutting down...",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_id_unique() {
        let id1 = MessageId::new();
        let id2 = MessageId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_session_id_unique() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        // Now guaranteed unique due to atomic counter
        assert_ne!(id1, id2);
        assert!(!id1.0.is_empty());
        assert!(!id2.0.is_empty());
    }

    #[test]
    fn test_conductor_state_description() {
        assert_eq!(ConductorState::Ready.description(), "Ready");
        assert_eq!(ConductorState::Thinking.description(), "Thinking...");
    }
}
