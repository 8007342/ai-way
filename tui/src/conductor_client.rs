//! Conductor Client
//!
//! Thin wrapper around the Conductor for TUI integration.
//! This client embeds the Conductor directly (no network) and provides
//! a convenient interface for sending events and receiving messages.
//!
//! # Architecture
//!
//! The TUI is a "thin client" - it doesn't contain any business logic.
//! All orchestration happens in the Conductor. The TUI's job is:
//! 1. Convert terminal events to SurfaceEvents
//! 2. Send SurfaceEvents to Conductor
//! 3. Receive ConductorMessages
//! 4. Render display state based on messages

use tokio::sync::mpsc;

use conductor_core::{
    Conductor, ConductorConfig, ConductorMessage, ConductorState,
    OllamaBackend, SurfaceCapabilities, SurfaceEvent, SurfaceType,
};

/// Client for communicating with the embedded Conductor
pub struct ConductorClient {
    /// The embedded Conductor instance
    conductor: Conductor<OllamaBackend>,
    /// Receiver for messages from Conductor
    rx: mpsc::Receiver<ConductorMessage>,
}

impl ConductorClient {
    /// Create a new ConductorClient with embedded Conductor
    pub fn new() -> Self {
        // Create channel for Conductor -> TUI messages
        let (tx, rx) = mpsc::channel(100);

        // Create Ollama backend from environment
        let backend = OllamaBackend::from_env();

        // Create Conductor config from environment
        let config = ConductorConfig::from_env();

        // Create the Conductor
        let conductor = Conductor::new(backend, config, tx);

        Self { conductor, rx }
    }

    /// Start the Conductor (initialize and warm up)
    pub async fn start(&mut self) -> anyhow::Result<()> {
        self.conductor.start().await
    }

    /// Connect this surface to the Conductor
    pub async fn connect(&mut self) -> anyhow::Result<()> {
        let event = SurfaceEvent::Connected {
            event_id: SurfaceEvent::new_event_id(),
            surface_type: SurfaceType::Tui,
            capabilities: SurfaceCapabilities::tui(),
        };
        self.conductor.handle_event(event).await
    }

    /// Send a user message to the Conductor
    pub async fn send_message(&mut self, content: String) -> anyhow::Result<()> {
        let event = SurfaceEvent::UserMessage {
            event_id: SurfaceEvent::new_event_id(),
            content,
        };
        self.conductor.handle_event(event).await
    }

    /// Send a user command to the Conductor
    pub async fn send_command(&mut self, command: String, args: Vec<String>) -> anyhow::Result<()> {
        let event = SurfaceEvent::UserCommand {
            event_id: SurfaceEvent::new_event_id(),
            command,
            args,
        };
        self.conductor.handle_event(event).await
    }

    /// Notify Conductor that user is typing
    pub async fn user_typing(&mut self, typing: bool) -> anyhow::Result<()> {
        let event = SurfaceEvent::UserTyping { typing };
        self.conductor.handle_event(event).await
    }

    /// Notify Conductor that user scrolled
    pub async fn user_scrolled(
        &mut self,
        direction: conductor_core::ScrollDirection,
        amount: u32,
    ) -> anyhow::Result<()> {
        let event = SurfaceEvent::UserScrolled { direction, amount };
        self.conductor.handle_event(event).await
    }

    /// Notify Conductor that user clicked the avatar
    pub async fn avatar_clicked(&mut self) -> anyhow::Result<()> {
        let event = SurfaceEvent::AvatarClicked {
            event_id: SurfaceEvent::new_event_id(),
        };
        self.conductor.handle_event(event).await
    }

    /// Notify Conductor that user wants to quit
    pub async fn request_quit(&mut self) -> anyhow::Result<()> {
        let event = SurfaceEvent::QuitRequested {
            event_id: SurfaceEvent::new_event_id(),
        };
        self.conductor.handle_event(event).await
    }

    /// Notify Conductor of resize
    pub async fn resized(&mut self, width: u32, height: u32) -> anyhow::Result<()> {
        let event = SurfaceEvent::Resized {
            event_id: SurfaceEvent::new_event_id(),
            width,
            height,
        };
        self.conductor.handle_event(event).await
    }

    /// Poll for streaming tokens (must be called regularly)
    pub async fn poll_streaming(&mut self) -> bool {
        self.conductor.poll_streaming().await
    }

    /// Try to receive a message from the Conductor (non-blocking)
    pub fn try_recv(&mut self) -> Option<ConductorMessage> {
        self.rx.try_recv().ok()
    }

    /// Receive all pending messages from the Conductor (non-blocking)
    pub fn recv_all(&mut self) -> Vec<ConductorMessage> {
        let mut messages = Vec::new();
        while let Ok(msg) = self.rx.try_recv() {
            messages.push(msg);
        }
        messages
    }

    /// Get the current Conductor state
    pub fn state(&self) -> ConductorState {
        self.conductor.state()
    }

    /// Check if the Conductor is ready
    pub fn is_ready(&self) -> bool {
        self.conductor.is_ready()
    }

    /// Send raw surface event to Conductor
    pub async fn send_event(&mut self, event: SurfaceEvent) -> anyhow::Result<()> {
        self.conductor.handle_event(event).await
    }
}

impl Default for ConductorClient {
    fn default() -> Self {
        Self::new()
    }
}
