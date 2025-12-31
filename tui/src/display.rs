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
    AvatarGesture, AvatarMood, AvatarPosition, AvatarReaction, AvatarSize,
    ConductorMessage, ConductorState, MessageId, MessageRole, TaskId, TaskStatus,
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
            ConductorMessage::AvatarGesture { gesture, duration_ms } => {
                self.current_gesture = Some(ActiveGesture {
                    gesture: *gesture,
                    started: Instant::now(),
                    duration: Duration::from_millis(*duration_ms as u64),
                });
                self.current_reaction = None;
            }
            ConductorMessage::AvatarReact { reaction, duration_ms } => {
                self.current_reaction = Some(ActiveReaction {
                    reaction: *reaction,
                    started: Instant::now(),
                    duration: Duration::from_millis(*duration_ms as u64),
                });
                self.current_gesture = None;
            }
            ConductorMessage::AvatarPointAt { x_percent, y_percent } => {
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
            ConductorMessage::Message { id, role, content } => {
                self.messages.push(DisplayMessage::new(id, role, content));
            }
            ConductorMessage::Token { message_id, text } => {
                // Find or create streaming message
                if self.streaming_id.as_ref() != Some(&message_id) {
                    // New streaming message
                    self.messages.push(DisplayMessage::streaming(message_id.clone()));
                    self.streaming_id = Some(message_id.clone());
                }
                // Append token
                if let Some(msg) = self.messages.last_mut() {
                    if msg.id == message_id {
                        msg.append(&text);
                    }
                }
            }
            ConductorMessage::StreamEnd { message_id, final_content } => {
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
            ConductorMessage::TaskCreated { task_id, agent, description } => {
                self.tasks.push(DisplayTask::new(task_id, agent, description));
            }
            ConductorMessage::TaskUpdated { task_id, progress, status_message } => {
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
            ConductorMessage::Notify { level, title, message } => {
                self.notification = Some(DisplayNotification { level, title, message });
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
