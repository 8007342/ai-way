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
//! a TUI, `WebUI`, mobile app, or test harness. It communicates through:
//! - `ConductorMessage`: Commands sent TO the UI surface
//! - `SurfaceEvent`: Events received FROM the UI surface
//!
//! This separation enables:
//! - Hot-swappable UI surfaces
//! - Multiple simultaneous surfaces
//! - Headless operation for testing
//! - Clean separation of concerns

use std::sync::Arc;

use chrono::Timelike;
use tokio::sync::mpsc;

use crate::avatar::{AvatarCommand, AvatarState, CommandParser};
use crate::backend::{LlmBackend, LlmRequest, StreamingToken};
use crate::events::{ScrollDirection, SurfaceCapabilities, SurfaceEvent, SurfaceType};
use crate::messages::{
    AvatarStateSnapshot, ConductorMessage, ConductorState, ContentType, EventId, MessageId,
    MessageRole, NotifyLevel, ResponseMetadata, SessionId, SessionSnapshot, SnapshotMessage,
};
use crate::routing::{
    policy::RoutingRequest, QueryRouter, RouterConfig, RouterError, RouterResponse,
};
use crate::security::{CommandValidator, ConductorLimits, InputValidator, ValidationResult};
use crate::session::Session;
use crate::surface_registry::{ConnectionId, SurfaceHandle, SurfaceRegistry};
use crate::tasks::{TaskId, TaskManager};

/// Conductor configuration
#[derive(Clone, Debug)]
pub struct ConductorConfig {
    /// Default model to use
    pub model: String,
    /// Whether to warm up the model on startup
    pub warmup_on_start: bool,
    /// Whether to send a dynamic greeting when a surface connects
    /// This also serves as a warmup for the LLM
    pub greet_on_connect: bool,
    /// Maximum messages to keep in context
    pub max_context_messages: usize,
    /// System prompt
    pub system_prompt: Option<String>,
    /// Security limits
    pub limits: ConductorLimits,
    /// Additional allowed agents beyond defaults
    pub additional_agents: Vec<String>,
    /// Whether to enable intelligent routing (multi-model support)
    pub enable_routing: bool,
    /// Router configuration (if routing is enabled)
    pub router_config: Option<RouterConfig>,
}

impl Default for ConductorConfig {
    fn default() -> Self {
        Self {
            model: "yollayah".to_string(),
            warmup_on_start: true,
            greet_on_connect: true,
            max_context_messages: 10,
            system_prompt: None,
            limits: ConductorLimits::default(),
            additional_agents: Vec::new(),
            enable_routing: false,
            router_config: None,
        }
    }
}

impl ConductorConfig {
    /// Create configuration from environment variables
    #[must_use]
    pub fn from_env() -> Self {
        let enable_routing = std::env::var("CONDUCTOR_ENABLE_ROUTING")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        Self {
            model: std::env::var("YOLLAYAH_MODEL").unwrap_or_else(|_| "yollayah".to_string()),
            warmup_on_start: std::env::var("YOLLAYAH_SKIP_WARMUP")
                .map(|v| v != "1" && v.to_lowercase() != "true")
                .unwrap_or(true),
            greet_on_connect: std::env::var("YOLLAYAH_GREET")
                .map(|v| v == "1" || v.to_lowercase() == "true")
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
            enable_routing,
            router_config: if enable_routing {
                Some(RouterConfig::default())
            } else {
                None
            },
        }
    }

    /// Enable routing with a custom configuration
    #[must_use]
    pub fn with_routing(mut self, router_config: RouterConfig) -> Self {
        self.enable_routing = true;
        self.router_config = Some(router_config);
        self
    }
}

/// The Conductor - headless orchestration core
pub struct Conductor<B: LlmBackend> {
    /// Configuration
    config: ConductorConfig,
    /// LLM backend (fallback when routing is disabled or fails)
    backend: Arc<B>,
    /// Query router for intelligent model selection (optional)
    router: Option<Arc<QueryRouter>>,
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
    /// Registry of connected surfaces (multi-surface support)
    registry: SurfaceRegistry,
    /// Legacy single-surface channel (for backward compatibility)
    /// When using the registry, this can be None
    legacy_tx: Option<mpsc::Sender<ConductorMessage>>,
    /// Whether warmup is complete
    warmup_complete: bool,
    /// Current streaming message receiver
    streaming_rx: Option<mpsc::Receiver<StreamingToken>>,
    /// Current streaming message ID
    streaming_message_id: Option<MessageId>,
    /// Streaming response start time for metrics
    streaming_start: Option<std::time::Instant>,
    /// Token count for current streaming response
    streaming_token_count: u32,
    /// Input validator for surface events
    input_validator: InputValidator,
    /// Command validator for LLM-generated commands
    command_validator: CommandValidator,
    /// Model used for current streaming response (for metrics)
    streaming_model: Option<String>,
}

impl<B: LlmBackend + 'static> Conductor<B> {
    /// Create a new Conductor with the given backend (legacy single-surface mode)
    ///
    /// This constructor maintains backward compatibility with existing code.
    /// For multi-surface support, use `new_with_registry` instead.
    pub fn new(backend: B, config: ConductorConfig, tx: mpsc::Sender<ConductorMessage>) -> Self {
        Self::create_conductor(backend, config, Some(tx), SurfaceRegistry::new())
    }

    /// Create a new Conductor with a `SurfaceRegistry` for multi-surface support
    ///
    /// This is the preferred constructor for the daemon which manages multiple connections.
    pub fn new_with_registry(
        backend: B,
        config: ConductorConfig,
        registry: SurfaceRegistry,
    ) -> Self {
        Self::create_conductor(backend, config, None, registry)
    }

    /// Internal constructor used by both public constructors
    fn create_conductor(
        backend: B,
        config: ConductorConfig,
        legacy_tx: Option<mpsc::Sender<ConductorMessage>>,
        registry: SurfaceRegistry,
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

        // Create the router if routing is enabled
        let router = if config.enable_routing {
            config
                .router_config
                .as_ref()
                .map(|rc| Arc::new(QueryRouter::new(rc.clone())))
        } else {
            None
        };

        Self {
            config,
            backend: Arc::new(backend),
            router,
            session,
            avatar: AvatarState::default(),
            command_parser: CommandParser::new(),
            tasks,
            state: ConductorState::Initializing,
            registry,
            legacy_tx,
            warmup_complete: false,
            streaming_rx: None,
            streaming_message_id: None,
            streaming_start: None,
            streaming_token_count: 0,
            input_validator,
            command_validator,
            streaming_model: None,
        }
    }

    /// Get a reference to the surface registry
    pub fn registry(&self) -> &SurfaceRegistry {
        &self.registry
    }

    /// Register a new surface connection
    ///
    /// Returns the assigned `ConnectionId`.
    pub fn register_surface(
        &self,
        tx: mpsc::Sender<ConductorMessage>,
        surface_type: SurfaceType,
        capabilities: SurfaceCapabilities,
    ) -> ConnectionId {
        let id = ConnectionId::new();
        let handle = SurfaceHandle::new(id, tx, surface_type, capabilities);
        self.registry.register(handle);
        id
    }

    /// Unregister a surface connection
    pub fn unregister_surface(&self, id: &ConnectionId) -> bool {
        self.registry.unregister(id).is_some()
    }

    /// Get the number of connected surfaces
    pub fn surface_count(&self) -> usize {
        self.registry.count()
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

    /// Get a reference to the session
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Get the model name
    pub fn model(&self) -> &str {
        &self.config.model
    }

    /// Check if warmup is complete
    pub fn is_ready(&self) -> bool {
        self.warmup_complete
    }

    /// Create a state snapshot for late-joining surfaces
    ///
    /// The snapshot includes:
    /// - Recent conversation history (limited to prevent overwhelming)
    /// - Current avatar state
    /// - Session metadata
    ///
    /// # Arguments
    /// * `max_messages` - Maximum number of recent messages to include (0 = use default of 20)
    #[must_use]
    pub fn create_state_snapshot(&self, max_messages: usize) -> ConductorMessage {
        let max_messages = if max_messages == 0 { 20 } else { max_messages };

        // Convert conversation messages to snapshot messages
        let conversation_history: Vec<SnapshotMessage> = self
            .session
            .recent_messages(max_messages)
            .iter()
            .map(|msg| SnapshotMessage {
                id: msg.id.clone(),
                role: msg.role,
                content: msg.content.clone(),
                content_type: ContentType::default(),
                timestamp: msg.timestamp,
            })
            .collect();

        // Create avatar state snapshot
        let avatar_state = AvatarStateSnapshot::from(&self.avatar);

        // Create session snapshot
        let session_info = SessionSnapshot::new(
            self.session.id.clone(),
            self.config.model.clone(),
            self.warmup_complete,
            self.state,
            self.session.metadata.created_at,
            self.session.metadata.message_count,
        );

        ConductorMessage::StateSnapshot {
            conversation_history,
            avatar_state,
            session_info,
        }
    }

    /// Start the Conductor (initialize and optionally warm up)
    pub async fn start(&mut self) -> anyhow::Result<()> {
        self.set_state(ConductorState::Initializing).await;

        // Initialize the router if enabled
        if let Some(ref router) = self.router {
            if let Err(e) = router.start().await {
                tracing::warn!(error = %e, "Failed to start query router, falling back to direct backend");
                // Don't fail startup - we can fall back to the direct backend
            } else {
                tracing::info!("Query router started successfully");
            }
        }

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

        let request =
            LlmRequest::new("Say hi in 5 words or less.", &self.config.model).with_stream(true);

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

    /// Generate a dynamic greeting
    ///
    /// This sends a request to the LLM asking for a quick, cute greeting.
    /// It doubles as a warmup (preloads the model) while making Yollayah
    /// feel alive by starting the conversation.
    async fn generate_greeting(&mut self) {
        // Build a prompt that encourages a quick, dynamic response
        let now = chrono::Local::now();
        let day = now.format("%A").to_string();
        let time_of_day = match now.hour() {
            5..=11 => "morning",
            12..=16 => "afternoon",
            17..=20 => "evening",
            _ => "night",
        };

        // Quick prompt for a one-liner greeting
        let prompt = format!(
            "Say a quick, cute one-liner greeting to start our chat. \
             It's {day} {time_of_day}. Be yourself - warm, playful, maybe a Spanish expression. \
             ONE sentence max. Include avatar commands for wave/mood."
        );

        let mut request = LlmRequest::new(&prompt, &self.config.model).with_stream(true);

        if let Some(ref system) = self.config.system_prompt {
            request = request.with_system(system.clone());
        }

        self.set_state(ConductorState::Responding).await;

        match self.backend.send_streaming(&request).await {
            Ok(rx) => {
                // Start streaming the greeting as an assistant message
                let msg_id = self.session.start_assistant_response();
                self.streaming_rx = Some(rx);
                self.streaming_message_id = Some(msg_id);
                self.streaming_start = Some(std::time::Instant::now());
                self.streaming_token_count = 0;
                // Note: poll_streaming() will handle the tokens and set state to Ready when done
            }
            Err(e) => {
                tracing::warn!("Greeting generation failed: {}", e);
                // Fall back to static greeting
                self.send(ConductorMessage::Message {
                    id: MessageId::new(),
                    role: MessageRole::Assistant,
                    content: "[yolla:wave][yolla:mood happy]¡Hola! Ready to chat!".to_string(),
                    content_type: ContentType::Plain,
                })
                .await;
                self.set_state(ConductorState::Ready).await;
            }
        }
    }

    /// Handle an event from the UI surface
    pub async fn handle_event(&mut self, event: SurfaceEvent) -> anyhow::Result<()> {
        match event {
            SurfaceEvent::Connected {
                event_id,
                surface_type: _,
                capabilities: _,
            } => {
                // Note: In legacy single-surface mode, surface info is not tracked
                // Use handle_event_for_connection with SurfaceRegistry for multi-surface support
                self.ack(event_id).await;

                // Send current state to new surface
                self.send(ConductorMessage::State { state: self.state })
                    .await;
                self.send(ConductorMessage::SessionInfo {
                    session_id: self.session.id.clone(),
                    model: self.config.model.clone(),
                    ready: self.warmup_complete,
                })
                .await;

                // Generate dynamic greeting if configured and ready
                // This also warms up the LLM while making Yollayah feel alive
                if self.config.greet_on_connect && self.warmup_complete {
                    self.generate_greeting().await;
                } else if self.warmup_complete {
                    // Fall back to static welcome if greeting disabled
                    self.send(ConductorMessage::Message {
                        id: MessageId::new(),
                        role: MessageRole::System,
                        content: "Ready to chat! Type a message below.".to_string(),
                        content_type: ContentType::System,
                    })
                    .await;
                }
            }

            SurfaceEvent::Disconnected { event_id, .. } => {
                // Note: In legacy single-surface mode, surface info is not tracked
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
                        self.notify(NotifyLevel::Warning, &format!("Invalid message: {reason}"))
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
                        self.notify(NotifyLevel::Warning, &format!("Invalid command: {reason}"))
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
                self.avatar.current_gesture = Some(crate::avatar::AvatarGesture::Wave);
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
                capabilities: _,
            } => {
                // Note: In legacy single-surface mode, surface info is not tracked
                // Use handle_event_for_connection with SurfaceRegistry for multi-surface support
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
                if recoverable {
                    tracing::warn!("Surface error (recoverable): {}", error);
                } else {
                    tracing::error!("Surface error (fatal): {}", error);
                }
            }

            // Transport/Handshake events
            SurfaceEvent::Handshake {
                event_id,
                protocol_version,
                surface_type: _,
                capabilities: _,
                auth_token: _,
            } => {
                // Note: In legacy single-surface mode, surface info is not tracked
                // Use handle_event_for_connection with SurfaceRegistry for multi-surface support

                // Protocol version 1 is currently supported
                let accepted = protocol_version == 1;
                let rejection_reason = if accepted {
                    None
                } else {
                    Some(format!(
                        "Unsupported protocol version: {protocol_version} (expected 1)"
                    ))
                };

                // Send handshake acknowledgment
                self.send(ConductorMessage::HandshakeAck {
                    accepted,
                    connection_id: format!("conn_{}", self.session.id.0),
                    rejection_reason,
                    protocol_version: 1,
                })
                .await;
                self.ack(event_id).await;

                if accepted {
                    // Send current state
                    self.send(ConductorMessage::State { state: self.state })
                        .await;
                    self.send(ConductorMessage::SessionInfo {
                        session_id: self.session.id.clone(),
                        model: self.config.model.clone(),
                        ready: self.warmup_complete,
                    })
                    .await;

                    // Send state snapshot for late-joining surfaces
                    let snapshot = self.create_state_snapshot(20);
                    self.send(snapshot).await;
                }
            }

            SurfaceEvent::Pong { seq } => {
                // Heartbeat response received
                tracing::trace!(seq = seq, "Received pong");
            }

            // Multi-conversation events
            SurfaceEvent::FocusConversation {
                event_id,
                conversation_id,
            } => {
                self.ack(event_id).await;
                // TODO: Implement conversation focus switching
                tracing::debug!(conversation_id = %conversation_id, "Focus conversation request");
            }

            SurfaceEvent::ScrollConversation {
                event_id,
                conversation_id,
                direction: _,
                amount: _,
            } => {
                self.ack(event_id).await;
                // TODO: Track per-conversation scroll state
                tracing::debug!(conversation_id = %conversation_id, "Scroll conversation request");
            }

            SurfaceEvent::RequestSummary { event_id } => {
                self.ack(event_id).await;
                // TODO: Generate summary view
                tracing::debug!("Summary view requested");
            }

            SurfaceEvent::ExitSummary { event_id } => {
                self.ack(event_id).await;
                // TODO: Return to conversation view
                tracing::debug!("Exit summary requested");
            }

            SurfaceEvent::FocusNextConversation { event_id } => {
                self.ack(event_id).await;
                // TODO: Cycle to next conversation
                tracing::debug!("Focus next conversation requested");
            }

            SurfaceEvent::FocusPrevConversation { event_id } => {
                self.ack(event_id).await;
                // TODO: Cycle to previous conversation
                tracing::debug!("Focus previous conversation requested");
            }
        }

        Ok(())
    }

    /// Handle an event from a specific connected surface
    ///
    /// This is the preferred method for the daemon which tracks individual connections.
    /// It properly handles per-surface state and targeted responses.
    pub async fn handle_event_from(
        &mut self,
        conn_id: ConnectionId,
        event: SurfaceEvent,
    ) -> anyhow::Result<()> {
        match event {
            SurfaceEvent::Connected {
                event_id,
                surface_type,
                capabilities,
            } => {
                // Update capabilities in registry if already registered
                self.registry
                    .update_capabilities(&conn_id, capabilities.clone());
                self.ack_to(&conn_id, event_id).await;

                // Send current state to the new surface
                self.send_to(&conn_id, ConductorMessage::State { state: self.state })
                    .await;
                self.send_to(
                    &conn_id,
                    ConductorMessage::SessionInfo {
                        session_id: self.session.id.clone(),
                        model: self.config.model.clone(),
                        ready: self.warmup_complete,
                    },
                )
                .await;

                tracing::info!(
                    connection_id = %conn_id,
                    surface_type = %surface_type.name(),
                    "Surface connected"
                );

                // Generate dynamic greeting if configured and ready
                if self.config.greet_on_connect && self.warmup_complete {
                    // For multi-surface, generate greeting for all (broadcast)
                    self.generate_greeting().await;
                } else if self.warmup_complete {
                    self.send_to(
                        &conn_id,
                        ConductorMessage::Message {
                            id: MessageId::new(),
                            role: MessageRole::System,
                            content: "Ready to chat! Type a message below.".to_string(),
                            content_type: ContentType::System,
                        },
                    )
                    .await;
                }
            }

            SurfaceEvent::Disconnected { event_id, reason } => {
                self.ack_to(&conn_id, event_id).await;
                self.unregister_surface(&conn_id);
                tracing::info!(
                    connection_id = %conn_id,
                    reason = ?reason,
                    "Surface disconnected"
                );
            }

            SurfaceEvent::CapabilitiesReport {
                event_id,
                capabilities,
            } => {
                self.registry.update_capabilities(&conn_id, capabilities);
                self.ack_to(&conn_id, event_id).await;
            }

            SurfaceEvent::Handshake {
                event_id,
                protocol_version,
                surface_type,
                capabilities,
                auth_token: _,
            } => {
                // Update capabilities in registry
                self.registry.update_capabilities(&conn_id, capabilities);

                // Protocol version 1 is currently supported
                let accepted = protocol_version == 1;
                let rejection_reason = if accepted {
                    None
                } else {
                    Some(format!(
                        "Unsupported protocol version: {protocol_version} (expected 1)"
                    ))
                };

                // Send handshake acknowledgment to this specific surface
                self.send_to(
                    &conn_id,
                    ConductorMessage::HandshakeAck {
                        accepted,
                        connection_id: conn_id.to_string(),
                        rejection_reason,
                        protocol_version: 1,
                    },
                )
                .await;
                self.ack_to(&conn_id, event_id).await;

                if accepted {
                    // Send current state to this surface
                    self.send_to(&conn_id, ConductorMessage::State { state: self.state })
                        .await;
                    self.send_to(
                        &conn_id,
                        ConductorMessage::SessionInfo {
                            session_id: self.session.id.clone(),
                            model: self.config.model.clone(),
                            ready: self.warmup_complete,
                        },
                    )
                    .await;

                    // Send state snapshot for late-joining surfaces
                    // This allows surfaces to sync up with existing conversation state
                    let snapshot = self.create_state_snapshot(20);
                    self.send_to(&conn_id, snapshot).await;

                    tracing::info!(
                        connection_id = %conn_id,
                        surface_type = %surface_type.name(),
                        message_count = self.session.metadata.message_count,
                        "Handshake accepted, state snapshot sent"
                    );
                }
            }

            // For all other events, delegate to the standard handler
            // (they don't need connection-specific handling)
            _ => {
                self.handle_event(event).await?;
            }
        }

        Ok(())
    }

    /// Send acknowledgment to a specific surface
    async fn ack_to(&self, conn_id: &ConnectionId, event_id: EventId) {
        self.send_to(conn_id, ConductorMessage::Ack { event_id })
            .await;
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
            content_type: ContentType::Plain,
        })
        .await;

        // Start processing
        self.set_state(ConductorState::Thinking).await;

        // Try routing first if enabled, fall back to direct backend
        if let Some(ref router) = self.router {
            if router.is_healthy().await {
                match self.route_message(&content).await {
                    Ok(()) => return Ok(()),
                    Err(e) => {
                        tracing::warn!(error = %e, "Routing failed, falling back to direct backend");
                        // Fall through to direct backend
                    }
                }
            }
        }

        // Direct backend path (fallback or when routing is disabled)
        self.send_via_backend(&content).await
    }

    /// Route a message through the `QueryRouter`
    async fn route_message(&mut self, content: &str) -> Result<(), RouterError> {
        let router = self.router.as_ref().ok_or(RouterError::NotRunning)?;

        // Build a routing request with context
        let routing_request =
            RoutingRequest::new(content).with_conversation(self.session.id.0.clone());

        // Route the request
        match router.route(routing_request).await? {
            RouterResponse::Streaming {
                receiver, model_id, ..
            } => {
                let msg_id = self.session.start_assistant_response();
                self.streaming_rx = Some(receiver);
                self.streaming_message_id = Some(msg_id);
                self.streaming_start = Some(std::time::Instant::now());
                self.streaming_token_count = 0;
                self.streaming_model = Some(model_id);
                self.set_state(ConductorState::Responding).await;
                Ok(())
            }
            RouterResponse::Complete {
                response, model_id, ..
            } => {
                // Handle non-streaming response
                let msg_id = self.session.start_assistant_response();
                self.session.append_streaming(&response.content);
                self.session.complete_streaming();

                // Build response metadata with model info
                let mut metadata = ResponseMetadata::with_timing(
                    response.duration_ms.unwrap_or(0),
                    response.tokens_used.unwrap_or(0),
                );
                metadata.model_id = Some(model_id.clone());

                // Send complete message to UI
                self.send(ConductorMessage::StreamEnd {
                    message_id: msg_id,
                    final_content: response.content,
                    metadata,
                })
                .await;

                tracing::debug!(model = %model_id, "Completed non-streaming response via router");
                self.set_state(ConductorState::Ready).await;
                Ok(())
            }
        }
    }

    /// Send a message directly via the backend (fallback path)
    async fn send_via_backend(&mut self, content: &str) -> anyhow::Result<()> {
        // Build request with conversation history
        let history = self.session.build_context(self.config.max_context_messages);
        let mut request = LlmRequest::new(content, &self.config.model).with_stream(true);

        if !history.is_empty() {
            request = request.with_context(history);
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
                self.streaming_start = Some(std::time::Instant::now());
                self.streaming_token_count = 0;
                self.streaming_model = Some(self.config.model.clone());
                self.set_state(ConductorState::Responding).await;
            }
            Err(e) => {
                self.session.add_system_message(format!("Error: {e}"));
                self.notify(NotifyLevel::Error, &format!("Failed to send message: {e}"))
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

            // ✅ NON-BLOCKING: Check for available tokens without blocking event loop
            // Returns immediately if no tokens available, keeping UI responsive
            match rx.try_recv() {
                Ok(token) => {
                    let is_terminal = matches!(
                        token,
                        StreamingToken::Complete { ..} | StreamingToken::Error(_)
                    );
                    collected.push(token);

                    // Don't drain more if this was terminal
                    if !is_terminal {
                        // Drain any additional tokens that arrived while we were processing
                        // (non-blocking, just clears the buffer)
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
                    }
                }
                Err(_) => {
                    // No tokens available yet, or channel closed
                    return false;
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
                    // Count tokens for metrics
                    self.streaming_token_count += 1;

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

                    // Build response metadata
                    let elapsed_ms = self
                        .streaming_start
                        .map_or(0, |s| s.elapsed().as_millis() as u64);
                    let token_count = self.streaming_token_count;
                    let active_tasks = self.tasks.active_count() as u32;

                    let mut metadata = ResponseMetadata::with_timing(elapsed_ms, token_count);
                    metadata.agent_tasks_spawned = active_tasks;
                    metadata.model_id = self.streaming_model.take();

                    // Reset streaming metrics
                    self.streaming_start = None;
                    self.streaming_token_count = 0;

                    // Send completion to UI with metadata
                    if let Some(msg_id) = self.streaming_message_id.take() {
                        self.send(ConductorMessage::StreamEnd {
                            message_id: msg_id,
                            final_content: message,
                            metadata,
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
                self.session
                    .add_system_message("Available commands: /help, /clear, /quit".to_string());
                self.notify(
                    NotifyLevel::Info,
                    "Available commands: /help, /clear, /quit",
                )
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
                    &format!("Unknown command: /{command}"),
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
                self.send(ConductorMessage::AvatarMood { mood: *mood })
                    .await;
            }
            AvatarCommand::Size(size) => {
                self.send(ConductorMessage::AvatarSize { size: *size })
                    .await;
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
                self.avatar.current_reaction = Some(crate::avatar::AvatarReaction::Tada);
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

        // Shutdown the router if running
        if let Some(ref router) = self.router {
            router.shutdown().await;
            tracing::info!("Query router shut down");
        }

        // Send quit to UI
        self.send(ConductorMessage::Quit {
            message: Some("Goodbye!".to_string()),
        })
        .await;

        Ok(())
    }

    /// Get a reference to the query router (if enabled)
    pub fn router(&self) -> Option<&Arc<QueryRouter>> {
        self.router.as_ref()
    }

    /// Check if routing is enabled and healthy
    pub async fn is_routing_active(&self) -> bool {
        if let Some(ref router) = self.router {
            router.is_healthy().await
        } else {
            false
        }
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

    /// Send a message to all connected UI surfaces
    ///
    /// This method broadcasts to all surfaces in the registry.
    /// For backward compatibility, it also sends to the `legacy_tx` if present.
    async fn send(&self, msg: ConductorMessage) {
        // If we have a legacy single-surface channel, use it
        // Use try_send to avoid blocking if channel is full (prevents streaming stalls)
        if let Some(ref tx) = self.legacy_tx {
            if let Err(e) = tx.try_send(msg.clone()) {
                tracing::warn!("Failed to send message to legacy surface (channel may be full): {}", e);
            }
        }

        // Broadcast to all registered surfaces
        if self.registry.count() > 0 {
            let result = self.registry.broadcast(msg);
            if result.failed > 0 {
                tracing::warn!(
                    failed = result.failed,
                    successful = result.successful,
                    "Some surfaces failed to receive message"
                );
            }
        }
    }

    /// Send a message to a specific surface by `ConnectionId`
    pub async fn send_to(&self, id: &ConnectionId, msg: ConductorMessage) -> bool {
        self.registry.send_to_async(id, msg).await
    }

    /// Send a message only to surfaces with specific capabilities
    ///
    /// Useful for sending avatar animations only to surfaces that support them.
    pub fn send_to_capable(
        &self,
        msg: ConductorMessage,
        required_caps: impl Fn(&SurfaceCapabilities) -> bool,
    ) {
        let result = self.registry.send_to_capable(msg.clone(), required_caps);

        // Also send to legacy_tx if present (assume it supports the capability)
        if let Some(ref tx) = self.legacy_tx {
            let _ = tx.try_send(msg);
        }

        if result.failed > 0 {
            tracing::debug!(
                failed = result.failed,
                successful = result.successful,
                "Some capable surfaces failed to receive message"
            );
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

        async fn send(&self, _request: &LlmRequest) -> anyhow::Result<crate::backend::LlmResponse> {
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

    #[tokio::test]
    async fn test_conductor_with_routing() {
        use crate::routing::{ModelProfile, RouterConfig, TaskClass};

        let (tx, mut rx) = mpsc::channel(100);

        // Create a router config with a test model
        let mut router_config = RouterConfig::default();
        let mut test_model = ModelProfile::new("test-model", "ollama");
        test_model.supports_streaming = true;
        test_model.avg_ttft_ms = 500;
        router_config.models.push(test_model);
        router_config
            .default_models
            .insert(TaskClass::General, "test-model".to_string());

        // Create conductor with routing enabled
        let config = ConductorConfig {
            warmup_on_start: false,
            enable_routing: true,
            router_config: Some(router_config),
            ..Default::default()
        };

        let mut conductor = Conductor::new(MockBackend, config, tx);

        // Verify router was created
        assert!(conductor.router().is_some());

        // Start conductor
        conductor.start().await.unwrap();

        // Drain initial messages
        while rx.try_recv().is_ok() {}

        // Router should not be active yet (no backends registered)
        // but conductor should still work via fallback
        assert!(conductor.is_ready());
        assert_eq!(conductor.state(), ConductorState::Ready);
    }

    #[tokio::test]
    async fn test_conductor_routing_fallback() {
        // Test that when routing fails, we fall back to direct backend
        let (tx, mut rx) = mpsc::channel(100);

        // Create conductor with routing enabled but no models configured
        // This should trigger fallback to direct backend
        let config = ConductorConfig {
            warmup_on_start: false,
            enable_routing: true,
            router_config: Some(RouterConfig::default()), // Empty config
            ..Default::default()
        };

        let mut conductor = Conductor::new(MockBackend, config, tx);
        conductor.start().await.unwrap();

        // Drain initial messages
        while rx.try_recv().is_ok() {}

        // Send a user message - should fall back to direct backend
        conductor
            .handle_event(SurfaceEvent::UserMessage {
                event_id: SurfaceEvent::new_event_id(),
                content: "Hello!".to_string(),
            })
            .await
            .unwrap();

        // Should receive acknowledgment and user message echo
        let mut found_user_message = false;
        while let Ok(msg) = rx.try_recv() {
            if matches!(
                msg,
                ConductorMessage::Message {
                    role: MessageRole::User,
                    ..
                }
            ) {
                found_user_message = true;
                break;
            }
        }
        assert!(found_user_message, "Should have received user message echo");
    }

    #[tokio::test]
    async fn test_conductor_config_with_routing() {
        // Test the with_routing builder method
        let router_config = RouterConfig::default();
        let config = ConductorConfig::default().with_routing(router_config);

        assert!(config.enable_routing);
        assert!(config.router_config.is_some());
    }

    #[tokio::test]
    async fn test_create_state_snapshot_empty() {
        // Test state snapshot with no messages
        let (tx, _rx) = mpsc::channel(100);
        let conductor = Conductor::new(
            MockBackend,
            ConductorConfig {
                warmup_on_start: false,
                ..Default::default()
            },
            tx,
        );

        let snapshot = conductor.create_state_snapshot(20);

        match snapshot {
            ConductorMessage::StateSnapshot {
                conversation_history,
                avatar_state,
                session_info,
            } => {
                // Should have empty history for new conductor
                assert!(
                    conversation_history.is_empty(),
                    "Empty conductor should have empty history"
                );

                // Avatar should be in default state
                assert!(avatar_state.visible, "Avatar should be visible by default");
                assert!(
                    avatar_state.wandering,
                    "Avatar should have wandering enabled by default"
                );

                // Session info should exist
                assert!(
                    !session_info.session_id.0.is_empty(),
                    "Should have session ID"
                );
                assert_eq!(session_info.message_count, 0, "Should have 0 messages");
            }
            _ => panic!("Expected StateSnapshot message"),
        }
    }

    #[tokio::test]
    async fn test_create_state_snapshot_with_history() {
        // Test state snapshot with conversation history
        let (tx, mut rx) = mpsc::channel(100);
        let mut conductor = Conductor::new(
            MockBackend,
            ConductorConfig {
                warmup_on_start: false,
                greet_on_connect: false,
                ..Default::default()
            },
            tx,
        );

        // Start conductor
        conductor.start().await.unwrap();

        // Drain initial messages
        while rx.try_recv().is_ok() {}

        // Add a user message to create history
        conductor
            .handle_event(SurfaceEvent::UserMessage {
                event_id: SurfaceEvent::new_event_id(),
                content: "Test message".to_string(),
            })
            .await
            .unwrap();

        // Wait for streaming to complete
        for _ in 0..10 {
            if conductor.poll_streaming().await {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            } else {
                break;
            }
        }

        // Create snapshot
        let snapshot = conductor.create_state_snapshot(20);

        match snapshot {
            ConductorMessage::StateSnapshot {
                conversation_history,
                session_info,
                ..
            } => {
                // Should have at least the user message in history
                assert!(
                    !conversation_history.is_empty(),
                    "Should have conversation history"
                );

                // Check session has messages
                assert!(session_info.message_count >= 1, "Should have message count");
            }
            _ => panic!("Expected StateSnapshot message"),
        }
    }

    #[tokio::test]
    async fn test_create_state_snapshot_limit() {
        // Test that snapshot respects message limit
        let (tx, _rx) = mpsc::channel(100);
        let mut conductor = Conductor::new(
            MockBackend,
            ConductorConfig {
                warmup_on_start: false,
                greet_on_connect: false,
                ..Default::default()
            },
            tx,
        );

        // Manually add messages to session (bypassing event handling)
        for i in 0..30 {
            conductor.session.add_user_message(format!("Message {}", i));
        }

        // Request snapshot with limit of 5
        let snapshot = conductor.create_state_snapshot(5);

        match snapshot {
            ConductorMessage::StateSnapshot {
                conversation_history,
                ..
            } => {
                assert_eq!(conversation_history.len(), 5, "Should limit to 5 messages");
                // Should be the most recent 5 messages (25-29)
                assert!(
                    conversation_history[0].content.contains("Message 25"),
                    "Should have message 25"
                );
                assert!(
                    conversation_history[4].content.contains("Message 29"),
                    "Should have message 29"
                );
            }
            _ => panic!("Expected StateSnapshot message"),
        }
    }

    #[tokio::test]
    async fn test_state_snapshot_sent_on_handshake() {
        // Test that StateSnapshot is sent when handshake is accepted
        let (tx, mut rx) = mpsc::channel(100);
        let mut conductor = Conductor::new(
            MockBackend,
            ConductorConfig {
                warmup_on_start: false,
                greet_on_connect: false,
                ..Default::default()
            },
            tx,
        );

        // Start conductor
        conductor.start().await.unwrap();

        // Drain initial messages
        while rx.try_recv().is_ok() {}

        // Send handshake event
        conductor
            .handle_event(SurfaceEvent::Handshake {
                event_id: SurfaceEvent::new_event_id(),
                protocol_version: 1,
                surface_type: SurfaceType::Tui,
                capabilities: SurfaceCapabilities::tui(),
                auth_token: None,
            })
            .await
            .unwrap();

        // Collect messages
        let mut found_handshake_ack = false;
        let mut found_state_snapshot = false;

        while let Ok(msg) = rx.try_recv() {
            match msg {
                ConductorMessage::HandshakeAck { accepted: true, .. } => {
                    found_handshake_ack = true;
                }
                ConductorMessage::StateSnapshot { .. } => {
                    found_state_snapshot = true;
                }
                _ => {}
            }
        }

        assert!(found_handshake_ack, "Should have received HandshakeAck");
        assert!(
            found_state_snapshot,
            "Should have received StateSnapshot after handshake"
        );
    }
}
