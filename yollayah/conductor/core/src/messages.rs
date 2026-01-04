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

use crate::avatar::{
    AvatarGesture, AvatarMood, AvatarPosition, AvatarReaction, AvatarSize, AvatarState,
};
use crate::conversation::{ConversationId, ConversationState};
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
        /// Response metadata for surface display
        #[serde(default)]
        metadata: ResponseMetadata,
    },

    /// Stream encountered an error
    StreamError {
        /// Message ID that errored
        message_id: MessageId,
        /// Error description
        error: String,
    },

    // ============================================
    // Multi-Conversation Messages
    // ============================================
    /// A new conversation was created
    ConversationCreated {
        /// Unique conversation identifier
        conversation_id: ConversationId,
        /// Name of the agent (None for direct user conversation)
        agent_name: Option<String>,
    },

    /// Conversation focus changed
    ConversationFocused {
        /// Conversation that is now focused
        conversation_id: ConversationId,
    },

    /// Conversation state changed
    ConversationStateChanged {
        /// Conversation that changed
        conversation_id: ConversationId,
        /// New state
        state: ConversationState,
    },

    /// Streaming token for a specific conversation
    ConversationStreamToken {
        /// Conversation receiving the token
        conversation_id: ConversationId,
        /// Message ID this token belongs to
        message_id: MessageId,
        /// The token text
        token: String,
    },

    /// Stream completed for a specific conversation
    ConversationStreamEnd {
        /// Conversation that completed streaming
        conversation_id: ConversationId,
        /// Message ID that completed
        message_id: MessageId,
        /// Final complete content
        final_content: String,
        /// Response metadata
        metadata: ResponseMetadata,
    },

    /// Summary of completed conversations is ready
    SummaryReady {
        /// Main conversation ID
        conversation_id: ConversationId,
        /// Compiled summary text
        summary: String,
        /// IDs of sub-conversations included
        sub_conversations: Vec<ConversationId>,
    },

    /// Conversation was removed
    ConversationRemoved {
        /// Conversation that was removed
        conversation_id: ConversationId,
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

    /// State snapshot for newly connected or late-joining surfaces
    ///
    /// Sent after successful handshake to synchronize the surface with
    /// the current conductor state. This allows surfaces to join mid-session
    /// and immediately see the current conversation and avatar state.
    StateSnapshot {
        /// Recent conversation history (limited to avoid overwhelming)
        conversation_history: Vec<SnapshotMessage>,
        /// Current avatar state
        avatar_state: AvatarStateSnapshot,
        /// Session information
        session_info: SessionSnapshot,
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

/// Response metadata for surface display
///
/// Contains metrics and context about a completed response that surfaces
/// can use to display meaningful information to users. Surfaces decide
/// how to present this information in their own style.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// Model that generated this response
    pub model_id: Option<String>,
    /// Response generation time in milliseconds
    pub elapsed_ms: u64,
    /// Total tokens generated
    pub token_count: u32,
    /// Tokens per second (if calculable)
    pub tokens_per_second: Option<f32>,
    /// Number of sub-agent tasks spawned during this response
    pub agent_tasks_spawned: u32,
    /// Number of files read/processed
    pub files_processed: u32,
    /// Bytes of content processed (for context about size)
    pub bytes_processed: u64,
    /// Whether response involved network calls (API, web)
    pub network_involved: bool,
    /// Optional context hint for surface commentary
    /// e.g., "`large_file`", "`slow_network`", "`complex_reasoning`"
    pub context_hint: Option<String>,
}

impl ResponseMetadata {
    /// Create metadata with timing info
    #[must_use]
    pub fn with_timing(elapsed_ms: u64, token_count: u32) -> Self {
        let tokens_per_second = if elapsed_ms > 0 {
            Some((token_count as f32 / elapsed_ms as f32) * 1000.0)
        } else {
            None
        };
        Self {
            elapsed_ms,
            token_count,
            tokens_per_second,
            ..Default::default()
        }
    }

    /// Check if this was a "slow" response (> 10 seconds)
    #[must_use]
    pub fn is_slow(&self) -> bool {
        self.elapsed_ms > 10_000
    }

    /// Check if this involved significant processing
    #[must_use]
    pub fn is_heavy(&self) -> bool {
        self.agent_tasks_spawned > 0 || self.files_processed > 3 || self.bytes_processed > 100_000
    }
}

/// Conductor operational states
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConductorState {
    /// Starting up, not ready
    Initializing,
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
            Self::Ready => "Ready",
            Self::Thinking => "Thinking...",
            Self::Responding => "Responding...",
            Self::Listening => "Listening",
            Self::Error => "Error",
            Self::ShuttingDown => "Shutting down...",
        }
    }
}

// ============================================
// State Snapshot Types
// ============================================

/// A message in the state snapshot
///
/// Simplified version of conversation message for initial sync.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SnapshotMessage {
    /// Message ID
    pub id: MessageId,
    /// Who sent this message
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Content type hint
    #[serde(default)]
    pub content_type: ContentType,
    /// Timestamp (Unix ms)
    pub timestamp: u64,
}

impl SnapshotMessage {
    /// Create a new snapshot message
    #[must_use]
    pub fn new(id: MessageId, role: MessageRole, content: String) -> Self {
        Self {
            id,
            role,
            content,
            content_type: ContentType::default(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

/// Avatar state snapshot for initial sync
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AvatarStateSnapshot {
    /// Current position
    pub position: AvatarPosition,
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

impl Default for AvatarStateSnapshot {
    fn default() -> Self {
        Self {
            position: AvatarPosition::default(),
            mood: AvatarMood::default(),
            size: AvatarSize::default(),
            visible: true,
            wandering: true,
            current_gesture: None,
            current_reaction: None,
        }
    }
}

impl From<&AvatarState> for AvatarStateSnapshot {
    fn from(state: &AvatarState) -> Self {
        Self {
            position: state.position,
            mood: state.mood,
            size: state.size,
            visible: state.visible,
            wandering: state.wandering,
            current_gesture: state.current_gesture,
            current_reaction: state.current_reaction,
        }
    }
}

/// Session information snapshot
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionSnapshot {
    /// Session ID
    pub session_id: SessionId,
    /// Model being used
    pub model: String,
    /// Whether conductor is ready
    pub ready: bool,
    /// Current conductor state
    pub state: ConductorState,
    /// When the session was created (Unix timestamp ms)
    pub created_at: u64,
    /// Total messages exchanged
    pub message_count: u32,
}

impl SessionSnapshot {
    /// Create a new session snapshot
    #[must_use]
    pub fn new(
        session_id: SessionId,
        model: String,
        ready: bool,
        state: ConductorState,
        created_at: u64,
        message_count: u32,
    ) -> Self {
        Self {
            session_id,
            model,
            ready,
            state,
            created_at,
            message_count,
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
