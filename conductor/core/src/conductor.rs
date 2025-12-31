//! Conductor - The Orchestration Core
//!
//! The Conductor is the "brain" of ai-way. It orchestrates:
//! - LLM backend communication
//! - Session and conversation management
//! - Avatar state and commands
//! - Background task management
//! - Communication with UI surfaces
//!
//! # Design Philosophy
//!
//! The Conductor is UI-agnostic. It doesn't know or care whether it's talking to
//! a TUI, WebUI, mobile app, or test harness. It communicates through:
//! - `ConductorMessage`: Commands sent TO the UI surface
//! - `SurfaceEvent`: Events received FROM the UI surface
//!
//! This separation enables:
//! - Hot-swappable UI surfaces
//! - Multiple simultaneous surfaces
//! - Headless operation for testing
//! - Clean separation of concerns

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::avatar::{AvatarCommand, AvatarState, CommandParser};
use crate::backend::{LlmBackend, LlmRequest, StreamingToken};
use crate::events::{ScrollDirection, SurfaceCapabilities, SurfaceEvent, SurfaceType};
use crate::messages::{
    ConductorMessage, ConductorState, EventId, MessageId, MessageRole, NotifyLevel, SessionId,
};
use crate::security::{CommandValidator, ConductorLimits, InputValidator, ValidationResult};
use crate::session::Session;
use crate::tasks::{TaskId, TaskManager};

/// Conductor configuration
#[derive(Clone, Debug)]
pub struct ConductorConfig {
    /// Default model to use
    pub model: String,
    /// Whether to warm up the model on startup
    pub warmup_on_start: bool,
    /// Maximum messages to keep in context
    pub max_context_messages: usize,
    /// System prompt
    pub system_prompt: Option<String>,
    /// Security limits
    pub limits: ConductorLimits,
    /// Additional allowed agents beyond defaults
    pub additional_agents: Vec<String>,
}

impl Default for ConductorConfig {
    fn default() -> Self {
        Self {
            model: "yollayah".to_string(),
            warmup_on_start: true,
            max_context_messages: 10,
            system_prompt: None,
            limits: ConductorLimits::default(),
            additional_agents: Vec::new(),
        }
    }
}

impl ConductorConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            model: std::env::var("YOLLAYAH_MODEL").unwrap_or_else(|_| "yollayah".to_string()),
            warmup_on_start: std::env::var("YOLLAYAH_SKIP_WARMUP")
                .map(|v| v != "1" && v.to_lowercase() != "true")
                .unwrap_or(true),
            max_context_messages: std::env::var("YOLLAYAH_MAX_CONTEXT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            system_prompt: std::env::var("YOLLAYAH_SYSTEM_PROMPT").ok(),
            limits: ConductorLimits::from_env(),
            additional_agents: std::env::var("CONDUCTOR_ADDITIONAL_AGENTS")
                .ok()
                .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default(),
        }
    }
}

/// The Conductor - headless orchestration core
pub struct Conductor<B: LlmBackend> {
    /// Configuration
    config: ConductorConfig,
    /// LLM backend
    backend: Arc<B>,
    /// Current session
    session: Session,
    /// Avatar state
    avatar: AvatarState,
    /// Command parser for extracting avatar commands from responses
    command_parser: CommandParser,
    /// Task manager
    tasks: TaskManager,
    /// Current operational state
    state: ConductorState,
    /// Channel to send messages to UI surface
    tx: mpsc::Sender<ConductorMessage>,
    /// Connected surface info
    surface_type: Option<SurfaceType>,
    surface_capabilities: Option<SurfaceCapabilities>,
    /// Whether warmup is complete
    warmup_complete: bool,
    /// Current streaming message receiver
    streaming_rx: Option<mpsc::Receiver<StreamingToken>>,
    /// Current streaming message ID
    streaming_message_id: Option<MessageId>,
    /// Input validator for surface events
    input_validator: InputValidator,
    /// Command validator for LLM-generated commands
    command_validator: CommandValidator,
}

impl<B: LlmBackend + 'static> Conductor<B> {
    /// Create a new Conductor with the given backend
    pub fn new(
        backend: B,
        config: ConductorConfig,
        tx: mpsc::Sender<ConductorMessage>,
    ) -> Self {
        let session = Session::new_with_limits(
            config.model.clone(),
            config.limits.max_session_messages,
            config.limits.max_session_content_bytes,
        );
        let input_validator = InputValidator::new(config.limits.clone());
        let mut command_validator = CommandValidator::new(&config.limits);

        // Add any additional allowed agents from config
        for agent in &config.additional_agents {
            command_validator.allow_agent(agent.clone());
        }

        let tasks = TaskManager::new_with_limits(
            config.limits.max_active_tasks,
            config.limits.max_total_tasks,
            config.limits.task_cleanup_age_ms,
        );

        Self {
            config,
            backend: Arc::new(backend),
            session,
            avatar: AvatarState::default(),
            command_parser: CommandParser::new(),
            tasks,
            state: ConductorState::Initializing,
            tx,
            surface_type: None,
            surface_capabilities: None,
            warmup_complete: false,
            streaming_rx: None,
            streaming_message_id: None,
            input_validator,
            command_validator,
        }
    }

    /// Get the session ID
    pub fn session_id(&self) -> &SessionId {
        &self.session.id
    }

    /// Get current state
    pub fn state(&self) -> ConductorState {
        self.state
    }

    /// Get avatar state
    pub fn avatar(&self) -> &AvatarState {
        &self.avatar
    }

    /// Get task manager
    pub fn tasks(&self) -> &TaskManager {
        &self.tasks
    }

    /// Check if warmup is complete
    pub fn is_ready(&self) -> bool {
        self.warmup_complete
    }

    /// Start the Conductor (initialize and optionally warm up)
    pub async fn start(&mut self) -> anyhow::Result<()> {
        self.set_state(ConductorState::Initializing).await;

        // Check backend health
        if !self.backend.health_check().await {
            self.notify(
                NotifyLevel::Warning,
                "Backend not available - first query may be slow",
            )
            .await;
        }

        // Warm up if configured
        if self.config.warmup_on_start {
            self.warmup().await?;
        } else {
            self.warmup_complete = true;
            self.set_state(ConductorState::Ready).await;
        }

        // Send session info
        self.send(ConductorMessage::SessionInfo {
            session_id: self.session.id.clone(),
            model: self.config.model.clone(),
            ready: self.warmup_complete,
        })
        .await;

        Ok(())
    }

    /// Warm up the model
    async fn warmup(&mut self) -> anyhow::Result<()> {
        self.set_state(ConductorState::WarmingUp).await;

        let request = LlmRequest::new("Say hi in 5 words or less.", &self.config.model)
            .with_stream(true);

        match self.backend.send_streaming(&request).await {
            Ok(mut rx) => {
                // Drain the warmup response
                while let Some(token) = rx.recv().await {
                    match token {
                        StreamingToken::Complete { .. } => break,
                        StreamingToken::Error(e) => {
                            tracing::warn!("Warmup error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
                self.warmup_complete = true;
                self.set_state(ConductorState::Ready).await;
            }
            Err(e) => {
                tracing::warn!("Warmup failed: {}", e);
                self.warmup_complete = true; // Allow proceeding anyway
                self.set_state(ConductorState::Ready).await;
            }
        }

        Ok(())
    }

    /// Handle an event from the UI surface
    pub async fn handle_event(&mut self, event: SurfaceEvent) -> anyhow::Result<()> {
        match event {
            SurfaceEvent::Connected {
                event_id,
                surface_type,
                capabilities,
            } => {
                self.surface_type = Some(surface_type);
                self.surface_capabilities = Some(capabilities);
                self.ack(event_id).await;

                // Send current state to new surface
                self.send(ConductorMessage::State { state: self.state }).await;
                self.send(ConductorMessage::SessionInfo {
                    session_id: self.session.id.clone(),
                    model: self.config.model.clone(),
                    ready: self.warmup_complete,
                })
                .await;
            }

            SurfaceEvent::Disconnected { event_id, .. } => {
                self.surface_type = None;
                self.surface_capabilities = None;
                self.ack(event_id).await;
            }

            SurfaceEvent::Resized { event_id, .. } => {
                self.ack(event_id).await;
            }

            SurfaceEvent::UserMessage { event_id, content } => {
                self.ack(event_id).await;
                // Validate input before processing
                match self.input_validator.validate_message(&content) {
                    ValidationResult::Valid => {
                        self.handle_user_message(content).await?;
                    }
                    ValidationResult::Invalid(reason) => {
                        tracing::warn!(reason = %reason, "Rejected user message");
                        self.notify(NotifyLevel::Warning, &format!("Invalid message: {}", reason))
                            .await;
                    }
                    ValidationResult::RateLimited(reason) => {
                        tracing::warn!(reason = %reason, "Rate limited user message");
                        self.notify(NotifyLevel::Warning, &reason).await;
                    }
                }
            }

            SurfaceEvent::UserCommand {
                event_id,
                command,
                args,
            } => {
                self.ack(event_id).await;
                // Validate command before processing
                match self.input_validator.validate_command(&command, &args) {
                    ValidationResult::Valid => {
                        self.handle_command(&command, &args).await?;
                    }
                    ValidationResult::Invalid(reason) => {
                        tracing::warn!(command = %command, reason = %reason, "Rejected user command");
                        self.notify(NotifyLevel::Warning, &format!("Invalid command: {}", reason))
                            .await;
                    }
                    ValidationResult::RateLimited(reason) => {
                        tracing::warn!(command = %command, reason = %reason, "Rate limited user command");
                        self.notify(NotifyLevel::Warning, &reason).await;
                    }
                }
            }

            SurfaceEvent::UserTyping { typing } => {
                if typing && self.state == ConductorState::Ready {
                    self.set_state(ConductorState::Listening).await;
                }
            }

            SurfaceEvent::UserScrolled { direction, .. } => {
                // Avatar might react to scrolling
                if matches!(direction, ScrollDirection::Up) && self.avatar.wandering {
                    // Curious about what user is looking at
                }
            }

            SurfaceEvent::AvatarClicked { event_id } => {
                self.ack(event_id).await;
                // Avatar was clicked - could trigger playful behavior
                self.avatar.current_gesture =
                    Some(crate::avatar::AvatarGesture::Wave);
                self.send_avatar_gesture().await;
            }

            SurfaceEvent::TaskClicked { event_id, task_id } => {
                self.ack(event_id).await;
                self.send(ConductorMessage::TaskFocus { task_id }).await;
            }

            SurfaceEvent::MessageClicked { event_id, .. } => {
                self.ack(event_id).await;
            }

            SurfaceEvent::MessageReceived { .. } => {
                // Surface acknowledged receiving a message
            }

            SurfaceEvent::RenderComplete { .. } => {
                // Surface finished rendering a frame
            }

            SurfaceEvent::CapabilitiesReport {
                event_id,
                capabilities,
            } => {
                self.surface_capabilities = Some(capabilities);
                self.ack(event_id).await;
            }

            SurfaceEvent::QuitRequested { event_id } => {
                self.ack(event_id).await;
                self.shutdown().await?;
            }

            SurfaceEvent::SurfaceError {
                event_id,
                error,
                recoverable,
            } => {
                self.ack(event_id).await;
                if !recoverable {
                    tracing::error!("Surface error (fatal): {}", error);
                } else {
                    tracing::warn!("Surface error (recoverable): {}", error);
                }
            }
        }

        Ok(())
    }

    /// Handle a user message
    async fn handle_user_message(&mut self, content: String) -> anyhow::Result<()> {
        // Add to session
        let user_msg_id = self.session.add_user_message(content.clone());

        // Send to UI
        self.send(ConductorMessage::Message {
            id: user_msg_id,
            role: MessageRole::User,
            content: content.clone(),
        })
        .await;

        // Start processing
        self.set_state(ConductorState::Thinking).await;

        // Build request with context
        let context = self.session.build_context(self.config.max_context_messages);
        let mut request = LlmRequest::new(&content, &self.config.model)
            .with_stream(true);

        if !context.is_empty() {
            request = request.with_context(context);
        }

        if let Some(ref system) = self.config.system_prompt {
            request = request.with_system(system.clone());
        }

        // Start streaming response
        match self.backend.send_streaming(&request).await {
            Ok(rx) => {
                let msg_id = self.session.start_assistant_response();
                self.streaming_rx = Some(rx);
                self.streaming_message_id = Some(msg_id);
                self.set_state(ConductorState::Responding).await;
            }
            Err(e) => {
                self.session.add_system_message(format!("Error: {}", e));
                self.notify(NotifyLevel::Error, &format!("Failed to send message: {}", e))
                    .await;
                self.set_state(ConductorState::Ready).await;
            }
        }

        Ok(())
    }

    /// Poll for streaming tokens
    ///
    /// Call this regularly to process incoming tokens.
    /// Returns true if there was activity.
    pub async fn poll_streaming(&mut self) -> bool {
        // First, collect all available tokens to avoid borrow issues
        let tokens: Vec<StreamingToken> = {
            let rx = match self.streaming_rx.as_mut() {
                Some(rx) => rx,
                None => return false,
            };

            let mut collected = Vec::new();
            while let Ok(token) = rx.try_recv() {
                let is_terminal = matches!(
                    token,
                    StreamingToken::Complete { .. } | StreamingToken::Error(_)
                );
                collected.push(token);
                if is_terminal {
                    break;
                }
            }
            collected
        };

        if tokens.is_empty() {
            return false;
        }

        // Reset command counter for this response batch
        self.command_validator.reset_response_counter();

        // Now process the collected tokens
        for token in tokens {
            match token {
                StreamingToken::Token(text) => {
                    // Parse for avatar commands
                    let clean_text = self.command_parser.parse(&text);

                    // Collect commands to process
                    let mut commands = Vec::new();
                    while let Some(cmd) = self.command_parser.next_command() {
                        commands.push(cmd);
                    }

                    // Process extracted commands WITH VALIDATION
                    for cmd in commands {
                        // Validate command before execution
                        match self.command_validator.validate_command(&cmd) {
                            Ok(()) => {
                                self.apply_avatar_command(&cmd).await;
                            }
                            Err(reason) => {
                                tracing::warn!(
                                    command = ?cmd,
                                    reason = %reason,
                                    "Rejected LLM command"
                                );
                                // Don't execute rejected commands, but continue processing
                            }
                        }
                    }

                    // Append to session
                    self.session.append_streaming(&clean_text);

                    // Send token to UI
                    if let Some(ref msg_id) = self.streaming_message_id {
                        self.send(ConductorMessage::Token {
                            message_id: msg_id.clone(),
                            text: clean_text,
                        })
                        .await;
                    }
                }

                StreamingToken::Complete { message } => {
                    // Complete the session message
                    self.session.complete_streaming();

                    // Send completion to UI
                    if let Some(msg_id) = self.streaming_message_id.take() {
                        self.send(ConductorMessage::StreamEnd {
                            message_id: msg_id,
                            final_content: message,
                        })
                        .await;
                    }

                    self.streaming_rx = None;
                    self.set_state(ConductorState::Ready).await;
                }

                StreamingToken::Error(error) => {
                    // Cancel the streaming message
                    self.session.cancel_streaming();

                    // Send error to UI
                    if let Some(msg_id) = self.streaming_message_id.take() {
                        self.send(ConductorMessage::StreamError {
                            message_id: msg_id,
                            error: error.clone(),
                        })
                        .await;
                    }

                    self.notify(NotifyLevel::Error, &error).await;
                    self.streaming_rx = None;
                    self.set_state(ConductorState::Ready).await;
                }
            }
        }

        true
    }

    /// Handle a user command
    async fn handle_command(&mut self, command: &str, args: &[String]) -> anyhow::Result<()> {
        match command {
            "help" => {
                self.session.add_system_message(
                    "Available commands: /help, /clear, /quit".to_string(),
                );
                self.notify(NotifyLevel::Info, "Available commands: /help, /clear, /quit")
                    .await;
            }
            "clear" => {
                self.session.clear_history();
                self.notify(NotifyLevel::Info, "Conversation cleared").await;
            }
            "quit" | "exit" => {
                self.shutdown().await?;
            }
            "model" if !args.is_empty() => {
                self.config.model = args[0].clone();
                self.session.metadata.model = args[0].clone();
                self.notify(NotifyLevel::Info, &format!("Model set to: {}", args[0]))
                    .await;
            }
            _ => {
                self.notify(
                    NotifyLevel::Warning,
                    &format!("Unknown command: /{}", command),
                )
                .await;
            }
        }

        Ok(())
    }

    /// Apply an avatar command
    async fn apply_avatar_command(&mut self, cmd: &AvatarCommand) {
        // Update internal state
        self.avatar.apply_command(cmd);

        // Send to UI based on command type
        match cmd {
            AvatarCommand::MoveTo(pos) => {
                self.send(ConductorMessage::AvatarMoveTo { position: *pos })
                    .await;
            }
            AvatarCommand::PointAt {
                x_percent,
                y_percent,
            } => {
                self.send(ConductorMessage::AvatarPointAt {
                    x_percent: *x_percent,
                    y_percent: *y_percent,
                })
                .await;
            }
            AvatarCommand::Wander(enabled) => {
                self.send(ConductorMessage::AvatarWander { enabled: *enabled })
                    .await;
            }
            AvatarCommand::Mood(mood) => {
                self.send(ConductorMessage::AvatarMood { mood: *mood }).await;
            }
            AvatarCommand::Size(size) => {
                self.send(ConductorMessage::AvatarSize { size: *size }).await;
            }
            AvatarCommand::Gesture(gesture) => {
                self.send(ConductorMessage::AvatarGesture {
                    gesture: *gesture,
                    duration_ms: gesture.default_duration_ms(),
                })
                .await;
            }
            AvatarCommand::React(reaction) => {
                self.send(ConductorMessage::AvatarReact {
                    reaction: *reaction,
                    duration_ms: reaction.default_duration_ms(),
                })
                .await;
            }
            AvatarCommand::Hide => {
                self.send(ConductorMessage::AvatarVisibility { visible: false })
                    .await;
            }
            AvatarCommand::Show => {
                self.send(ConductorMessage::AvatarVisibility { visible: true })
                    .await;
            }
            AvatarCommand::CustomSprite(_) => {
                // Future: handle custom sprites
            }
            AvatarCommand::Task(task_cmd) => {
                self.handle_task_command(task_cmd).await;
            }
        }
    }

    /// Handle a task command from avatar
    async fn handle_task_command(&mut self, cmd: &crate::avatar::TaskCommand) {
        use crate::avatar::TaskCommand as TC;

        match cmd {
            TC::Start { agent, description } => {
                let task_id = self.tasks.create_task(agent.clone(), description.clone());
                self.send(ConductorMessage::TaskCreated {
                    task_id,
                    agent: agent.clone(),
                    description: description.clone(),
                })
                .await;
            }
            TC::Progress { task_id, percent } => {
                let id = TaskId::new(task_id.clone());
                self.tasks.update_progress(&id, *percent, None);
                self.send(ConductorMessage::TaskUpdated {
                    task_id: id,
                    progress: *percent,
                    status_message: None,
                })
                .await;
            }
            TC::Done { task_id } => {
                let id = TaskId::new(task_id.clone());
                self.tasks.complete_task(&id, None);
                self.send(ConductorMessage::TaskCompleted {
                    task_id: id,
                    summary: None,
                })
                .await;
            }
            TC::Fail { task_id, reason } => {
                let id = TaskId::new(task_id.clone());
                self.tasks.fail_task(&id, reason.clone());
                self.send(ConductorMessage::TaskFailed {
                    task_id: id,
                    error: reason.clone(),
                })
                .await;
            }
            TC::Focus { task_id } => {
                self.send(ConductorMessage::TaskFocus {
                    task_id: TaskId::new(task_id.clone()),
                })
                .await;
            }
            TC::PointAt { .. } | TC::Hover { .. } => {
                // These affect avatar positioning, handled elsewhere
            }
            TC::Celebrate { task_id } => {
                self.avatar.current_reaction =
                    Some(crate::avatar::AvatarReaction::Tada);
                self.send(ConductorMessage::AvatarReact {
                    reaction: crate::avatar::AvatarReaction::Tada,
                    duration_ms: 2500,
                })
                .await;
                let _ = task_id; // Used for animation targeting
            }
        }
    }

    /// Send current avatar gesture to UI
    async fn send_avatar_gesture(&self) {
        if let Some(gesture) = self.avatar.current_gesture {
            self.send(ConductorMessage::AvatarGesture {
                gesture,
                duration_ms: gesture.default_duration_ms(),
            })
            .await;
        }
    }

    /// Shut down the Conductor
    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        self.set_state(ConductorState::ShuttingDown).await;
        self.session.end();

        // Send quit to UI
        self.send(ConductorMessage::Quit {
            message: Some("Goodbye!".to_string()),
        })
        .await;

        Ok(())
    }

    /// Set state and notify UI
    async fn set_state(&mut self, state: ConductorState) {
        self.state = state;
        self.send(ConductorMessage::State { state }).await;
    }

    /// Send acknowledgment
    async fn ack(&self, event_id: EventId) {
        self.send(ConductorMessage::Ack { event_id }).await;
    }

    /// Send notification
    async fn notify(&self, level: NotifyLevel, message: &str) {
        self.send(ConductorMessage::Notify {
            level,
            title: None,
            message: message.to_string(),
        })
        .await;
    }

    /// Send a message to the UI surface
    async fn send(&self, msg: ConductorMessage) {
        if let Err(e) = self.tx.send(msg).await {
            tracing::warn!("Failed to send message to surface: {}", e);
        }
    }

    /// Create a task manually
    pub fn create_task(&mut self, agent: String, description: String) -> TaskId {
        self.tasks.create_task(agent, description)
    }

    /// Update a task's progress
    pub fn update_task_progress(&mut self, id: &TaskId, progress: u8, message: Option<String>) {
        self.tasks.update_progress(id, progress, message);
    }

    /// Complete a task
    pub fn complete_task(&mut self, id: &TaskId, output: Option<String>) {
        self.tasks.complete_task(id, output);
    }

    /// Fail a task
    pub fn fail_task(&mut self, id: &TaskId, error: String) {
        self.tasks.fail_task(id, error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock backend for testing
    struct MockBackend;

    #[async_trait::async_trait]
    impl LlmBackend for MockBackend {
        fn name(&self) -> &str {
            "Mock"
        }

        async fn health_check(&self) -> bool {
            true
        }

        async fn send_streaming(
            &self,
            _request: &LlmRequest,
        ) -> anyhow::Result<mpsc::Receiver<StreamingToken>> {
            let (tx, rx) = mpsc::channel(10);
            tokio::spawn(async move {
                let _ = tx.send(StreamingToken::Token("Hello ".to_string())).await;
                let _ = tx.send(StreamingToken::Token("world!".to_string())).await;
                let _ = tx
                    .send(StreamingToken::Complete {
                        message: "Hello world!".to_string(),
                    })
                    .await;
            });
            Ok(rx)
        }

        async fn send(
            &self,
            _request: &LlmRequest,
        ) -> anyhow::Result<crate::backend::LlmResponse> {
            Ok(crate::backend::LlmResponse {
                content: "Hello!".to_string(),
                model: "mock".to_string(),
                tokens_used: None,
                duration_ms: None,
            })
        }

        async fn list_models(&self) -> anyhow::Result<Vec<crate::backend::ModelInfo>> {
            Ok(vec![crate::backend::ModelInfo {
                name: "mock".to_string(),
                description: None,
                size: None,
                parameters: None,
                loaded: true,
            }])
        }
    }

    #[tokio::test]
    async fn test_conductor_creation() {
        let (tx, _rx) = mpsc::channel(100);
        let conductor = Conductor::new(MockBackend, ConductorConfig::default(), tx);

        assert_eq!(conductor.state(), ConductorState::Initializing);
        assert!(!conductor.is_ready());
    }

    #[tokio::test]
    async fn test_conductor_start() {
        let (tx, mut rx) = mpsc::channel(100);
        let mut conductor = Conductor::new(
            MockBackend,
            ConductorConfig {
                warmup_on_start: false,
                ..Default::default()
            },
            tx,
        );

        conductor.start().await.unwrap();

        assert!(conductor.is_ready());
        assert_eq!(conductor.state(), ConductorState::Ready);

        // Should have received state and session info
        let msg = rx.recv().await.unwrap();
        assert!(matches!(msg, ConductorMessage::State { .. }));
    }
}
