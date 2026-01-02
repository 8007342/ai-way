//! Display State Types
//!
//! Types that represent the current display state for the TUI.
//! These are derived from ConductorMessages and used for rendering.
//!
//! # Design Philosophy
//!
//! The TUI is a "thin client" - it just renders what the Conductor tells it to.
//! Display state is the bridge between ConductorMessages and rendering.
//!
//! - DisplayMessage: A rendered conversation message
//! - DisplayAvatarState: Current avatar state for rendering
//! - DisplayTask: Task info for the task panel

use std::time::{Duration, Instant};

use conductor_core::{
    AvatarGesture, AvatarMood, AvatarPosition, AvatarReaction, AvatarSize, ConductorMessage,
    ConductorState, MessageId, MessageRole, TaskId, TaskStatus,
};

/// A rendered conversation message
#[derive(Clone, Debug)]
pub struct DisplayMessage {
    /// Unique message ID
    pub id: MessageId,
    /// Who sent this message
    pub role: DisplayRole,
    /// The message content
    pub content: String,
    /// Whether this message is still being streamed
    pub streaming: bool,
}

impl DisplayMessage {
    /// Create a new display message
    pub fn new(id: MessageId, role: MessageRole, content: String) -> Self {
        Self {
            id,
            role: role.into(),
            content,
            streaming: false,
        }
    }

    /// Create a streaming message (content will be appended)
    pub fn streaming(id: MessageId) -> Self {
        Self {
            id,
            role: DisplayRole::Assistant,
            content: String::new(),
            streaming: true,
        }
    }

    /// Append token to streaming message
    pub fn append(&mut self, text: &str) {
        self.content.push_str(text);
    }

    /// Mark stream as complete
    pub fn complete(&mut self, final_content: String) {
        self.content = final_content;
        self.streaming = false;
    }
}

/// Display role for messages
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayRole {
    /// User input
    User,
    /// AI assistant (Yollayah)
    Assistant,
    /// System message
    System,
}

impl From<MessageRole> for DisplayRole {
    fn from(role: MessageRole) -> Self {
        match role {
            MessageRole::User => DisplayRole::User,
            MessageRole::Assistant => DisplayRole::Assistant,
            MessageRole::System => DisplayRole::System,
        }
    }
}

impl DisplayRole {
    /// Get the prefix for this role
    pub fn prefix(&self) -> &'static str {
        match self {
            DisplayRole::User => "You: ",
            DisplayRole::Assistant => "Yollayah: ",
            DisplayRole::System => "",
        }
    }
}

/// Avatar state for rendering
#[derive(Clone, Debug)]
pub struct DisplayAvatarState {
    /// Current mood
    pub mood: AvatarMood,
    /// Current size
    pub size: AvatarSize,
    /// Whether avatar is visible
    pub visible: bool,
    /// Whether wandering is enabled
    pub wandering: bool,
    /// Target position (for smooth movement)
    pub target_position: Option<AvatarPosition>,
    /// Current gesture (if any)
    pub current_gesture: Option<ActiveGesture>,
    /// Current reaction (if any)
    pub current_reaction: Option<ActiveReaction>,
}

impl Default for DisplayAvatarState {
    fn default() -> Self {
        Self {
            mood: AvatarMood::Happy,
            size: AvatarSize::Medium,
            visible: true,
            wandering: true,
            target_position: None,
            current_gesture: None,
            current_reaction: None,
        }
    }
}

impl DisplayAvatarState {
    /// Apply a conductor message to update avatar state
    pub fn apply_message(&mut self, msg: &ConductorMessage) {
        match msg {
            ConductorMessage::AvatarMoveTo { position } => {
                self.target_position = Some(*position);
                self.wandering = false;
            }
            ConductorMessage::AvatarMood { mood } => {
                self.mood = *mood;
            }
            ConductorMessage::AvatarSize { size } => {
                self.size = *size;
            }
            ConductorMessage::AvatarVisibility { visible } => {
                self.visible = *visible;
            }
            ConductorMessage::AvatarWander { enabled } => {
                self.wandering = *enabled;
            }
            ConductorMessage::AvatarGesture {
                gesture,
                duration_ms,
            } => {
                self.current_gesture = Some(ActiveGesture {
                    gesture: *gesture,
                    started: Instant::now(),
                    duration: Duration::from_millis(*duration_ms as u64),
                });
                self.current_reaction = None;
            }
            ConductorMessage::AvatarReact {
                reaction,
                duration_ms,
            } => {
                self.current_reaction = Some(ActiveReaction {
                    reaction: *reaction,
                    started: Instant::now(),
                    duration: Duration::from_millis(*duration_ms as u64),
                });
                self.current_gesture = None;
            }
            ConductorMessage::AvatarPointAt {
                x_percent,
                y_percent,
            } => {
                self.target_position = Some(AvatarPosition::Percent {
                    x: *x_percent,
                    y: *y_percent,
                });
                self.wandering = false;
            }
            _ => {}
        }
    }

    /// Update timers, clearing expired gestures/reactions
    pub fn update(&mut self, delta: Duration) {
        // Check gesture expiration
        if let Some(ref gesture) = self.current_gesture {
            if gesture.started.elapsed() >= gesture.duration {
                self.current_gesture = None;
            }
        }

        // Check reaction expiration
        if let Some(ref reaction) = self.current_reaction {
            if reaction.started.elapsed() >= reaction.duration {
                self.current_reaction = None;
            }
        }

        let _ = delta; // Future: smooth animation updates
    }

    /// Get the suggested animation name for current state
    pub fn suggested_animation(&self) -> &'static str {
        if let Some(ref gesture) = self.current_gesture {
            return gesture.gesture.suggested_animation();
        }
        if let Some(ref reaction) = self.current_reaction {
            return reaction.reaction.suggested_animation();
        }
        self.mood.suggested_animation()
    }
}

/// An active gesture with timing
#[derive(Clone, Debug)]
pub struct ActiveGesture {
    /// The gesture type
    pub gesture: AvatarGesture,
    /// When the gesture started
    pub started: Instant,
    /// How long the gesture should last
    pub duration: Duration,
}

/// An active reaction with timing
#[derive(Clone, Debug)]
pub struct ActiveReaction {
    /// The reaction type
    pub reaction: AvatarReaction,
    /// When the reaction started
    pub started: Instant,
    /// How long the reaction should last
    pub duration: Duration,
}

/// Task info for the task panel
#[derive(Clone, Debug)]
pub struct DisplayTask {
    /// Task identifier
    pub id: TaskId,
    /// Agent handling the task
    pub agent: String,
    /// Human-readable agent name (e.g., "Cousin Rita")
    pub family_name: String,
    /// Task description
    pub description: String,
    /// Current status
    pub status: DisplayTaskStatus,
    /// Progress percentage (0-100)
    pub progress: u8,
    /// Optional status message
    pub status_message: Option<String>,
}

impl DisplayTask {
    /// Create a new display task
    pub fn new(id: TaskId, agent: String, description: String) -> Self {
        Self {
            id,
            family_name: agent_to_family_name(&agent),
            agent,
            description,
            status: DisplayTaskStatus::Pending,
            progress: 0,
            status_message: None,
        }
    }

    /// Update task progress
    pub fn update_progress(&mut self, progress: u8, message: Option<String>) {
        self.progress = progress.min(100);
        self.status_message = message;
        if self.progress > 0 && self.status == DisplayTaskStatus::Pending {
            self.status = DisplayTaskStatus::Running;
        }
    }

    /// Mark task as complete
    pub fn complete(&mut self) {
        self.status = DisplayTaskStatus::Done;
        self.progress = 100;
    }

    /// Mark task as failed
    pub fn fail(&mut self, error: &str) {
        self.status = DisplayTaskStatus::Failed;
        self.status_message = Some(error.to_string());
    }

    /// Get a progress bar string
    pub fn progress_bar(&self, width: usize) -> String {
        let filled = (self.progress as usize * width) / 100;
        let empty = width.saturating_sub(filled);
        format!("{}{}", "#".repeat(filled), "-".repeat(empty))
    }

    /// Get the display name
    pub fn display_name(&self) -> &str {
        if !self.family_name.is_empty() {
            &self.family_name
        } else {
            &self.agent
        }
    }
}

/// Display task status
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayTaskStatus {
    /// Task created but not started
    Pending,
    /// Task is actively running
    Running,
    /// Task completed successfully
    Done,
    /// Task failed
    Failed,
}

impl DisplayTaskStatus {
    /// Get a status icon
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Pending => "...",
            Self::Running => ">>>",
            Self::Done => "[+]",
            Self::Failed => "[!]",
        }
    }

    /// Whether this status indicates the task is active
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Pending | Self::Running)
    }

    /// Whether this status indicates the task is complete
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Done | Self::Failed)
    }
}

impl From<TaskStatus> for DisplayTaskStatus {
    fn from(status: TaskStatus) -> Self {
        match status {
            TaskStatus::Pending => DisplayTaskStatus::Pending,
            TaskStatus::Running => DisplayTaskStatus::Running,
            TaskStatus::Done => DisplayTaskStatus::Done,
            TaskStatus::Failed | TaskStatus::Cancelled => DisplayTaskStatus::Failed,
        }
    }
}

/// Map agent ID to family name
fn agent_to_family_name(agent_id: &str) -> String {
    match agent_id {
        "ethical-hacker" => "Cousin Rita".to_string(),
        "backend-engineer" => "Uncle Marco".to_string(),
        "frontend-specialist" => "Prima Sofia".to_string(),
        "senior-full-stack-developer" => "Tio Miguel".to_string(),
        "solutions-architect" => "Tia Carmen".to_string(),
        "ux-ui-designer" => "Cousin Lucia".to_string(),
        "qa-engineer" => "The Intern".to_string(),
        "privacy-researcher" => "Abuelo Pedro".to_string(),
        "devops-engineer" => "Primo Carlos".to_string(),
        "relational-database-expert" => "Tia Rosa".to_string(),
        _ => {
            // Capitalize first letter
            let mut chars = agent_id.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => agent_id.to_string(),
            }
        }
    }
}

/// The full display state for the TUI
#[derive(Debug)]
pub struct DisplayState {
    /// Conversation messages
    pub messages: Vec<DisplayMessage>,
    /// Current streaming message (if any)
    pub streaming_id: Option<MessageId>,
    /// Avatar state
    pub avatar: DisplayAvatarState,
    /// Active tasks
    pub tasks: Vec<DisplayTask>,
    /// Conductor state
    pub conductor_state: ConductorState,
    /// Session info
    pub session_model: String,
    /// Whether system is ready
    pub ready: bool,
    /// Pending notification (if any)
    pub notification: Option<DisplayNotification>,
}

impl Default for DisplayState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            streaming_id: None,
            avatar: DisplayAvatarState::default(),
            tasks: Vec::new(),
            conductor_state: ConductorState::Initializing,
            session_model: String::new(),
            ready: false,
            notification: None,
        }
    }
}

impl DisplayState {
    /// Create a new display state
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a ConductorMessage to update display state
    pub fn apply_message(&mut self, msg: ConductorMessage) {
        match msg {
            // Conversation messages
            ConductorMessage::Message {
                id, role, content, ..
            } => {
                self.messages.push(DisplayMessage::new(id, role, content));
            }
            ConductorMessage::Token { message_id, text } => {
                // Find or create streaming message
                if self.streaming_id.as_ref() != Some(&message_id) {
                    // New streaming message
                    self.messages
                        .push(DisplayMessage::streaming(message_id.clone()));
                    self.streaming_id = Some(message_id.clone());
                }
                // Append token
                if let Some(msg) = self.messages.last_mut() {
                    if msg.id == message_id {
                        msg.append(&text);
                    }
                }
            }
            ConductorMessage::StreamEnd {
                message_id,
                final_content,
            } => {
                if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
                    msg.complete(final_content);
                }
                self.streaming_id = None;
            }
            ConductorMessage::StreamError { message_id, error } => {
                if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
                    msg.content = format!("Error: {}", error);
                    msg.streaming = false;
                }
                self.streaming_id = None;
            }

            // Avatar messages
            ConductorMessage::AvatarMoveTo { .. }
            | ConductorMessage::AvatarMood { .. }
            | ConductorMessage::AvatarSize { .. }
            | ConductorMessage::AvatarVisibility { .. }
            | ConductorMessage::AvatarWander { .. }
            | ConductorMessage::AvatarGesture { .. }
            | ConductorMessage::AvatarReact { .. }
            | ConductorMessage::AvatarPointAt { .. } => {
                self.avatar.apply_message(&msg);
            }

            // Task messages
            ConductorMessage::TaskCreated {
                task_id,
                agent,
                description,
            } => {
                self.tasks
                    .push(DisplayTask::new(task_id, agent, description));
            }
            ConductorMessage::TaskUpdated {
                task_id,
                progress,
                status_message,
            } => {
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
                    task.update_progress(progress, status_message);
                }
            }
            ConductorMessage::TaskCompleted { task_id, .. } => {
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
                    task.complete();
                }
            }
            ConductorMessage::TaskFailed { task_id, error } => {
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
                    task.fail(&error);
                }
            }
            ConductorMessage::TaskFocus { .. } => {
                // UI could highlight the focused task
            }

            // System messages
            ConductorMessage::State { state } => {
                self.conductor_state = state;
            }
            ConductorMessage::SessionInfo { model, ready, .. } => {
                self.session_model = model;
                self.ready = ready;
            }
            ConductorMessage::Notify {
                level,
                title,
                message,
            } => {
                self.notification = Some(DisplayNotification {
                    level,
                    title,
                    message,
                });
            }
            ConductorMessage::Quit { message } => {
                // The app will handle quitting
                if let Some(msg) = message {
                    self.notification = Some(DisplayNotification {
                        level: conductor_core::NotifyLevel::Info,
                        title: Some("Goodbye".to_string()),
                        message: msg,
                    });
                }
            }
            ConductorMessage::Ack { .. } | ConductorMessage::QueryCapabilities => {
                // No display state change needed
            }

            // Transport messages - handled at transport layer, no display change
            ConductorMessage::HandshakeAck { .. } => {
                // Handshake handled by transport
            }
            ConductorMessage::Ping { .. } => {
                // Heartbeat handled by transport (would send Pong back)
            }

            // Layout hints - future: could control panel visibility
            ConductorMessage::LayoutHint { .. } => {
                // TODO: implement layout hint handling
            }
        }
    }

    /// Update timers and animations
    pub fn update(&mut self, delta: Duration) {
        self.avatar.update(delta);
    }

    /// Check if there are active tasks
    pub fn has_active_tasks(&self) -> bool {
        self.tasks.iter().any(|t| t.status.is_active())
    }

    /// Get active tasks
    pub fn active_tasks(&self) -> impl Iterator<Item = &DisplayTask> {
        self.tasks.iter().filter(|t| t.status.is_active())
    }

    /// Clear the notification
    pub fn clear_notification(&mut self) {
        self.notification = None;
    }

    /// Check if currently streaming
    pub fn is_streaming(&self) -> bool {
        self.streaming_id.is_some()
    }
}

/// A notification to display
#[derive(Clone, Debug)]
pub struct DisplayNotification {
    /// Notification level
    pub level: conductor_core::NotifyLevel,
    /// Optional title
    pub title: Option<String>,
    /// Message content
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use conductor_core::{NotifyLevel, SessionId};
    use pretty_assertions::assert_eq;

    // ========================================================================
    // DisplayMessage Tests
    // ========================================================================

    #[test]
    fn test_display_message_new() {
        let id = MessageId::new();
        let msg = DisplayMessage::new(id.clone(), MessageRole::User, "Hello".to_string());
        assert_eq!(msg.id, id);
        assert_eq!(msg.role, DisplayRole::User);
        assert_eq!(msg.content, "Hello");
        assert!(!msg.streaming);
    }

    #[test]
    fn test_display_message_new_assistant() {
        let id = MessageId::new();
        let msg = DisplayMessage::new(id.clone(), MessageRole::Assistant, "Hi there!".to_string());
        assert_eq!(msg.role, DisplayRole::Assistant);
        assert_eq!(msg.content, "Hi there!");
    }

    #[test]
    fn test_display_message_new_system() {
        let id = MessageId::new();
        let msg = DisplayMessage::new(id.clone(), MessageRole::System, "Welcome".to_string());
        assert_eq!(msg.role, DisplayRole::System);
    }

    #[test]
    fn test_display_message_streaming() {
        let id = MessageId::new();
        let msg = DisplayMessage::streaming(id.clone());
        assert_eq!(msg.id, id);
        assert_eq!(msg.role, DisplayRole::Assistant);
        assert!(msg.content.is_empty());
        assert!(msg.streaming);
    }

    #[test]
    fn test_display_message_append() {
        let id = MessageId::new();
        let mut msg = DisplayMessage::streaming(id);
        msg.append("Hello ");
        msg.append("World!");
        assert_eq!(msg.content, "Hello World!");
        assert!(msg.streaming);
    }

    #[test]
    fn test_display_message_complete() {
        let id = MessageId::new();
        let mut msg = DisplayMessage::streaming(id);
        msg.append("Partial");
        msg.complete("Final content".to_string());
        assert_eq!(msg.content, "Final content");
        assert!(!msg.streaming);
    }

    // ========================================================================
    // DisplayRole Tests
    // ========================================================================

    #[test]
    fn test_display_role_from_message_role() {
        assert_eq!(DisplayRole::from(MessageRole::User), DisplayRole::User);
        assert_eq!(
            DisplayRole::from(MessageRole::Assistant),
            DisplayRole::Assistant
        );
        assert_eq!(DisplayRole::from(MessageRole::System), DisplayRole::System);
    }

    #[test]
    fn test_display_role_prefix() {
        assert_eq!(DisplayRole::User.prefix(), "You: ");
        assert_eq!(DisplayRole::Assistant.prefix(), "Yollayah: ");
        assert_eq!(DisplayRole::System.prefix(), "");
    }

    // ========================================================================
    // DisplayAvatarState Tests
    // ========================================================================

    #[test]
    fn test_display_avatar_state_default() {
        let state = DisplayAvatarState::default();
        assert!(state.visible);
        assert!(state.wandering);
        assert_eq!(state.mood, AvatarMood::Happy);
        assert_eq!(state.size, AvatarSize::Medium);
        assert!(state.target_position.is_none());
        assert!(state.current_gesture.is_none());
        assert!(state.current_reaction.is_none());
    }

    #[test]
    fn test_display_avatar_apply_move_message() {
        let mut state = DisplayAvatarState::default();
        state.apply_message(&ConductorMessage::AvatarMoveTo {
            position: AvatarPosition::Center,
        });
        assert_eq!(state.target_position, Some(AvatarPosition::Center));
        assert!(!state.wandering);
    }

    #[test]
    fn test_display_avatar_apply_mood_message() {
        let mut state = DisplayAvatarState::default();
        state.apply_message(&ConductorMessage::AvatarMood {
            mood: AvatarMood::Thinking,
        });
        assert_eq!(state.mood, AvatarMood::Thinking);
    }

    #[test]
    fn test_display_avatar_apply_size_message() {
        let mut state = DisplayAvatarState::default();
        state.apply_message(&ConductorMessage::AvatarSize {
            size: AvatarSize::Large,
        });
        assert_eq!(state.size, AvatarSize::Large);
    }

    #[test]
    fn test_display_avatar_apply_visibility_message() {
        let mut state = DisplayAvatarState::default();
        assert!(state.visible);
        state.apply_message(&ConductorMessage::AvatarVisibility { visible: false });
        assert!(!state.visible);
        state.apply_message(&ConductorMessage::AvatarVisibility { visible: true });
        assert!(state.visible);
    }

    #[test]
    fn test_display_avatar_apply_wander_message() {
        let mut state = DisplayAvatarState::default();
        state.wandering = false;
        state.apply_message(&ConductorMessage::AvatarWander { enabled: true });
        assert!(state.wandering);
        state.apply_message(&ConductorMessage::AvatarWander { enabled: false });
        assert!(!state.wandering);
    }

    #[test]
    fn test_display_avatar_gesture_clears_reaction() {
        let mut state = DisplayAvatarState::default();
        state.current_reaction = Some(ActiveReaction {
            reaction: AvatarReaction::Laugh,
            started: Instant::now(),
            duration: Duration::from_secs(2),
        });

        state.apply_message(&ConductorMessage::AvatarGesture {
            gesture: AvatarGesture::Wave,
            duration_ms: 1000,
        });

        assert!(state.current_gesture.is_some());
        assert!(state.current_reaction.is_none());
    }

    #[test]
    fn test_display_avatar_reaction_clears_gesture() {
        let mut state = DisplayAvatarState::default();
        state.current_gesture = Some(ActiveGesture {
            gesture: AvatarGesture::Wave,
            started: Instant::now(),
            duration: Duration::from_secs(1),
        });

        state.apply_message(&ConductorMessage::AvatarReact {
            reaction: AvatarReaction::Tada,
            duration_ms: 2000,
        });

        assert!(state.current_reaction.is_some());
        assert!(state.current_gesture.is_none());
    }

    #[test]
    fn test_display_avatar_point_at() {
        let mut state = DisplayAvatarState::default();
        state.apply_message(&ConductorMessage::AvatarPointAt {
            x_percent: 75,
            y_percent: 25,
        });
        assert_eq!(
            state.target_position,
            Some(AvatarPosition::Percent { x: 75, y: 25 })
        );
        assert!(!state.wandering);
    }

    #[test]
    fn test_display_avatar_update_expires_gesture() {
        let mut state = DisplayAvatarState::default();
        state.current_gesture = Some(ActiveGesture {
            gesture: AvatarGesture::Wave,
            started: Instant::now() - Duration::from_secs(10),
            duration: Duration::from_secs(1),
        });

        state.update(Duration::from_millis(16));
        assert!(state.current_gesture.is_none());
    }

    #[test]
    fn test_display_avatar_update_expires_reaction() {
        let mut state = DisplayAvatarState::default();
        state.current_reaction = Some(ActiveReaction {
            reaction: AvatarReaction::Laugh,
            started: Instant::now() - Duration::from_secs(10),
            duration: Duration::from_secs(2),
        });

        state.update(Duration::from_millis(16));
        assert!(state.current_reaction.is_none());
    }

    #[test]
    fn test_display_avatar_update_preserves_active_gesture() {
        let mut state = DisplayAvatarState::default();
        state.current_gesture = Some(ActiveGesture {
            gesture: AvatarGesture::Wave,
            started: Instant::now(),
            duration: Duration::from_secs(10),
        });

        state.update(Duration::from_millis(16));
        assert!(state.current_gesture.is_some());
    }

    #[test]
    fn test_display_avatar_suggested_animation_gesture_priority() {
        let mut state = DisplayAvatarState::default();
        state.mood = AvatarMood::Thinking;
        state.current_gesture = Some(ActiveGesture {
            gesture: AvatarGesture::Wave,
            started: Instant::now(),
            duration: Duration::from_secs(1),
        });

        // Gesture should take priority over mood
        assert_eq!(state.suggested_animation(), "happy"); // Wave suggests "happy"
    }

    #[test]
    fn test_display_avatar_suggested_animation_reaction_priority() {
        let mut state = DisplayAvatarState::default();
        state.mood = AvatarMood::Happy;
        state.current_reaction = Some(ActiveReaction {
            reaction: AvatarReaction::Hmm,
            started: Instant::now(),
            duration: Duration::from_secs(2),
        });

        // Reaction should take priority over mood
        assert_eq!(state.suggested_animation(), "thinking"); // Hmm suggests "thinking"
    }

    #[test]
    fn test_display_avatar_suggested_animation_mood_fallback() {
        let mut state = DisplayAvatarState::default();
        state.mood = AvatarMood::Confused;

        assert_eq!(state.suggested_animation(), "error");
    }

    // ========================================================================
    // DisplayTask Tests
    // ========================================================================

    #[test]
    fn test_display_task_new() {
        let task = DisplayTask::new(
            TaskId::new("test"),
            "ethical-hacker".to_string(),
            "Test task".to_string(),
        );
        assert_eq!(task.family_name, "Cousin Rita");
        assert_eq!(task.agent, "ethical-hacker");
        assert_eq!(task.description, "Test task");
        assert_eq!(task.status, DisplayTaskStatus::Pending);
        assert_eq!(task.progress, 0);
        assert!(task.status_message.is_none());
    }

    #[test]
    fn test_display_task_family_names() {
        let agents_and_names = [
            ("ethical-hacker", "Cousin Rita"),
            ("backend-engineer", "Uncle Marco"),
            ("frontend-specialist", "Prima Sofia"),
            ("qa-engineer", "The Intern"),
            ("devops-engineer", "Primo Carlos"),
        ];

        for (agent, expected_name) in agents_and_names {
            let task = DisplayTask::new(TaskId::new("t"), agent.to_string(), "Test".to_string());
            assert_eq!(task.family_name, expected_name, "Agent: {}", agent);
        }
    }

    #[test]
    fn test_display_task_unknown_agent_capitalized() {
        let task = DisplayTask::new(
            TaskId::new("t"),
            "custom-agent".to_string(),
            "Test".to_string(),
        );
        assert_eq!(task.family_name, "Custom-agent");
    }

    #[test]
    fn test_display_task_update_progress() {
        let mut task = DisplayTask::new(TaskId::new("t"), "agent".to_string(), "Task".to_string());
        task.update_progress(50, Some("Halfway".to_string()));
        assert_eq!(task.progress, 50);
        assert_eq!(task.status, DisplayTaskStatus::Running);
        assert_eq!(task.status_message, Some("Halfway".to_string()));
    }

    #[test]
    fn test_display_task_update_progress_from_pending() {
        let mut task = DisplayTask::new(TaskId::new("t"), "agent".to_string(), "Task".to_string());
        assert_eq!(task.status, DisplayTaskStatus::Pending);

        task.update_progress(10, None);
        assert_eq!(task.status, DisplayTaskStatus::Running);
    }

    #[test]
    fn test_display_task_progress_clamp() {
        let mut task = DisplayTask::new(TaskId::new("t"), "a".to_string(), "d".to_string());
        task.update_progress(150, None);
        assert_eq!(task.progress, 100);
    }

    #[test]
    fn test_display_task_complete() {
        let mut task = DisplayTask::new(TaskId::new("t"), "a".to_string(), "d".to_string());
        task.complete();
        assert_eq!(task.status, DisplayTaskStatus::Done);
        assert_eq!(task.progress, 100);
    }

    #[test]
    fn test_display_task_fail() {
        let mut task = DisplayTask::new(TaskId::new("t"), "a".to_string(), "d".to_string());
        task.fail("Something went wrong");
        assert_eq!(task.status, DisplayTaskStatus::Failed);
        assert_eq!(
            task.status_message,
            Some("Something went wrong".to_string())
        );
    }

    #[test]
    fn test_display_task_progress_bar() {
        let mut task = DisplayTask::new(TaskId::new("t"), "a".to_string(), "d".to_string());
        task.progress = 50;
        assert_eq!(task.progress_bar(10), "#####-----");

        task.progress = 0;
        assert_eq!(task.progress_bar(10), "----------");

        task.progress = 100;
        assert_eq!(task.progress_bar(10), "##########");

        task.progress = 33;
        assert_eq!(task.progress_bar(10), "###-------");
    }

    #[test]
    fn test_display_task_display_name() {
        let task = DisplayTask::new(
            TaskId::new("t"),
            "ethical-hacker".to_string(),
            "Test".to_string(),
        );
        assert_eq!(task.display_name(), "Cousin Rita");

        let mut task2 = DisplayTask::new(TaskId::new("t"), "agent".to_string(), "Test".to_string());
        task2.family_name = String::new();
        assert_eq!(task2.display_name(), "agent");
    }

    // ========================================================================
    // DisplayTaskStatus Tests
    // ========================================================================

    #[test]
    fn test_display_task_status_from_task_status() {
        assert_eq!(
            DisplayTaskStatus::from(TaskStatus::Pending),
            DisplayTaskStatus::Pending
        );
        assert_eq!(
            DisplayTaskStatus::from(TaskStatus::Running),
            DisplayTaskStatus::Running
        );
        assert_eq!(
            DisplayTaskStatus::from(TaskStatus::Done),
            DisplayTaskStatus::Done
        );
        assert_eq!(
            DisplayTaskStatus::from(TaskStatus::Failed),
            DisplayTaskStatus::Failed
        );
        assert_eq!(
            DisplayTaskStatus::from(TaskStatus::Cancelled),
            DisplayTaskStatus::Failed
        );
    }

    #[test]
    fn test_display_task_status_icons() {
        assert_eq!(DisplayTaskStatus::Pending.icon(), "...");
        assert_eq!(DisplayTaskStatus::Running.icon(), ">>>");
        assert_eq!(DisplayTaskStatus::Done.icon(), "[+]");
        assert_eq!(DisplayTaskStatus::Failed.icon(), "[!]");
    }

    #[test]
    fn test_display_task_status_is_active() {
        assert!(DisplayTaskStatus::Pending.is_active());
        assert!(DisplayTaskStatus::Running.is_active());
        assert!(!DisplayTaskStatus::Done.is_active());
        assert!(!DisplayTaskStatus::Failed.is_active());
    }

    #[test]
    fn test_display_task_status_is_terminal() {
        assert!(!DisplayTaskStatus::Pending.is_terminal());
        assert!(!DisplayTaskStatus::Running.is_terminal());
        assert!(DisplayTaskStatus::Done.is_terminal());
        assert!(DisplayTaskStatus::Failed.is_terminal());
    }

    // ========================================================================
    // DisplayState Tests
    // ========================================================================

    #[test]
    fn test_display_state_default() {
        let state = DisplayState::new();
        assert!(state.messages.is_empty());
        assert!(state.streaming_id.is_none());
        assert!(state.tasks.is_empty());
        assert_eq!(state.conductor_state, ConductorState::Initializing);
        assert!(!state.ready);
        assert!(state.notification.is_none());
    }

    #[test]
    fn test_display_state_apply_message() {
        use conductor_core::messages::ContentType;
        let mut state = DisplayState::new();
        let id = MessageId::new();
        state.apply_message(ConductorMessage::Message {
            id: id.clone(),
            role: MessageRole::User,
            content: "Hello".to_string(),
            content_type: ContentType::Plain,
        });
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].content, "Hello");
        assert_eq!(state.messages[0].id, id);
    }

    #[test]
    fn test_display_state_apply_token_creates_streaming() {
        let mut state = DisplayState::new();
        let id = MessageId::new();

        state.apply_message(ConductorMessage::Token {
            message_id: id.clone(),
            text: "Hello ".to_string(),
        });

        assert_eq!(state.messages.len(), 1);
        assert!(state.is_streaming());
        assert_eq!(state.streaming_id, Some(id.clone()));
        assert_eq!(state.messages[0].content, "Hello ");
    }

    #[test]
    fn test_display_state_apply_token_appends() {
        let mut state = DisplayState::new();
        let id = MessageId::new();

        state.apply_message(ConductorMessage::Token {
            message_id: id.clone(),
            text: "Hello ".to_string(),
        });
        state.apply_message(ConductorMessage::Token {
            message_id: id.clone(),
            text: "World!".to_string(),
        });

        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].content, "Hello World!");
    }

    #[test]
    fn test_display_state_apply_stream_end() {
        let mut state = DisplayState::new();
        let id = MessageId::new();

        state.apply_message(ConductorMessage::Token {
            message_id: id.clone(),
            text: "Hello".to_string(),
        });
        state.apply_message(ConductorMessage::StreamEnd {
            message_id: id.clone(),
            final_content: "Final content".to_string(),
        });

        assert!(!state.is_streaming());
        assert!(state.streaming_id.is_none());
        assert_eq!(state.messages[0].content, "Final content");
        assert!(!state.messages[0].streaming);
    }

    #[test]
    fn test_display_state_apply_stream_error() {
        let mut state = DisplayState::new();
        let id = MessageId::new();

        state.apply_message(ConductorMessage::Token {
            message_id: id.clone(),
            text: "Partial".to_string(),
        });
        state.apply_message(ConductorMessage::StreamError {
            message_id: id.clone(),
            error: "Connection lost".to_string(),
        });

        assert!(!state.is_streaming());
        assert!(state.messages[0].content.contains("Error"));
        assert!(!state.messages[0].streaming);
    }

    #[test]
    fn test_display_state_task_lifecycle() {
        let mut state = DisplayState::new();
        let task_id = TaskId::new("task1");

        // Create task
        state.apply_message(ConductorMessage::TaskCreated {
            task_id: task_id.clone(),
            agent: "qa-engineer".to_string(),
            description: "Testing".to_string(),
        });
        assert_eq!(state.tasks.len(), 1);
        assert!(state.has_active_tasks());
        assert_eq!(state.tasks[0].family_name, "The Intern");

        // Update progress
        state.apply_message(ConductorMessage::TaskUpdated {
            task_id: task_id.clone(),
            progress: 50,
            status_message: Some("Halfway done".to_string()),
        });
        assert_eq!(state.tasks[0].progress, 50);
        assert_eq!(
            state.tasks[0].status_message,
            Some("Halfway done".to_string())
        );

        // Complete task
        state.apply_message(ConductorMessage::TaskCompleted {
            task_id: task_id.clone(),
            summary: None,
        });
        assert_eq!(state.tasks[0].status, DisplayTaskStatus::Done);
        assert!(!state.has_active_tasks());
    }

    #[test]
    fn test_display_state_task_failed() {
        let mut state = DisplayState::new();
        let task_id = TaskId::new("task1");

        state.apply_message(ConductorMessage::TaskCreated {
            task_id: task_id.clone(),
            agent: "backend-engineer".to_string(),
            description: "Build".to_string(),
        });
        state.apply_message(ConductorMessage::TaskFailed {
            task_id: task_id.clone(),
            error: "Compilation failed".to_string(),
        });

        assert_eq!(state.tasks[0].status, DisplayTaskStatus::Failed);
        assert!(!state.has_active_tasks());
    }

    #[test]
    fn test_display_state_conductor_state() {
        let mut state = DisplayState::new();
        assert_eq!(state.conductor_state, ConductorState::Initializing);

        state.apply_message(ConductorMessage::State {
            state: ConductorState::Ready,
        });
        assert_eq!(state.conductor_state, ConductorState::Ready);

        state.apply_message(ConductorMessage::State {
            state: ConductorState::Thinking,
        });
        assert_eq!(state.conductor_state, ConductorState::Thinking);
    }

    #[test]
    fn test_display_state_session_info() {
        let mut state = DisplayState::new();
        assert!(!state.ready);
        assert!(state.session_model.is_empty());

        state.apply_message(ConductorMessage::SessionInfo {
            session_id: SessionId::new(),
            model: "yollayah".to_string(),
            ready: true,
        });

        assert!(state.ready);
        assert_eq!(state.session_model, "yollayah");
    }

    #[test]
    fn test_display_state_notification() {
        let mut state = DisplayState::new();
        assert!(state.notification.is_none());

        state.apply_message(ConductorMessage::Notify {
            level: NotifyLevel::Warning,
            title: Some("Heads up".to_string()),
            message: "Something happened".to_string(),
        });

        assert!(state.notification.is_some());
        let notif = state.notification.as_ref().unwrap();
        assert_eq!(notif.title, Some("Heads up".to_string()));
        assert_eq!(notif.message, "Something happened");
    }

    #[test]
    fn test_display_state_clear_notification() {
        let mut state = DisplayState::new();
        state.notification = Some(DisplayNotification {
            level: NotifyLevel::Info,
            title: None,
            message: "Test".to_string(),
        });

        state.clear_notification();
        assert!(state.notification.is_none());
    }

    #[test]
    fn test_display_state_quit_message() {
        let mut state = DisplayState::new();
        state.apply_message(ConductorMessage::Quit {
            message: Some("Goodbye!".to_string()),
        });

        assert!(state.notification.is_some());
        let notif = state.notification.as_ref().unwrap();
        assert_eq!(notif.message, "Goodbye!");
    }

    #[test]
    fn test_display_state_active_tasks_iterator() {
        let mut state = DisplayState::new();

        state.apply_message(ConductorMessage::TaskCreated {
            task_id: TaskId::new("t1"),
            agent: "a".to_string(),
            description: "d".to_string(),
        });
        state.apply_message(ConductorMessage::TaskCreated {
            task_id: TaskId::new("t2"),
            agent: "b".to_string(),
            description: "d".to_string(),
        });
        state.apply_message(ConductorMessage::TaskCompleted {
            task_id: TaskId::new("t1"),
            summary: None,
        });

        let active: Vec<_> = state.active_tasks().collect();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_display_state_update_passes_to_avatar() {
        let mut state = DisplayState::new();
        state.avatar.current_gesture = Some(ActiveGesture {
            gesture: AvatarGesture::Wave,
            started: Instant::now() - Duration::from_secs(100),
            duration: Duration::from_secs(1),
        });

        state.update(Duration::from_millis(16));

        // Expired gesture should be cleared
        assert!(state.avatar.current_gesture.is_none());
    }

    #[test]
    fn test_display_state_avatar_messages_applied() {
        let mut state = DisplayState::new();

        state.apply_message(ConductorMessage::AvatarMood {
            mood: AvatarMood::Excited,
        });
        assert_eq!(state.avatar.mood, AvatarMood::Excited);

        state.apply_message(ConductorMessage::AvatarSize {
            size: AvatarSize::Small,
        });
        assert_eq!(state.avatar.size, AvatarSize::Small);
    }

    // ========================================================================
    // agent_to_family_name Tests
    // ========================================================================

    #[test]
    fn test_agent_to_family_name_known() {
        assert_eq!(agent_to_family_name("ethical-hacker"), "Cousin Rita");
        assert_eq!(agent_to_family_name("backend-engineer"), "Uncle Marco");
        assert_eq!(agent_to_family_name("frontend-specialist"), "Prima Sofia");
        assert_eq!(
            agent_to_family_name("senior-full-stack-developer"),
            "Tio Miguel"
        );
        assert_eq!(agent_to_family_name("solutions-architect"), "Tia Carmen");
        assert_eq!(agent_to_family_name("ux-ui-designer"), "Cousin Lucia");
        assert_eq!(agent_to_family_name("qa-engineer"), "The Intern");
        assert_eq!(agent_to_family_name("privacy-researcher"), "Abuelo Pedro");
        assert_eq!(agent_to_family_name("devops-engineer"), "Primo Carlos");
        assert_eq!(
            agent_to_family_name("relational-database-expert"),
            "Tia Rosa"
        );
    }

    #[test]
    fn test_agent_to_family_name_unknown() {
        assert_eq!(agent_to_family_name("unknown"), "Unknown");
        assert_eq!(agent_to_family_name("custom-agent"), "Custom-agent");
        assert_eq!(agent_to_family_name("myAgent"), "MyAgent");
    }

    #[test]
    fn test_agent_to_family_name_empty() {
        assert_eq!(agent_to_family_name(""), "");
    }
}
