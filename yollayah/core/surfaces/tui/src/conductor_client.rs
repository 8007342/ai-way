//! Conductor Client
//!
//! Thin wrapper around the Conductor for TUI integration.
//! Supports two transport modes:
//! - InProcess: Embeds the Conductor directly (no network)
//! - UnixSocket: Connects to an external Conductor daemon via Unix socket
//!
//! # Architecture
//!
//! The TUI is a "thin client" - it doesn't contain any business logic.
//! All orchestration happens in the Conductor. The TUI's job is:
//! 1. Convert terminal events to SurfaceEvents
//! 2. Send SurfaceEvents to Conductor (via transport)
//! 3. Receive ConductorMessages (via transport)
//! 4. Render display state based on messages
//!
//! # Transport Selection
//!
//! Transport is selected via `CONDUCTOR_TRANSPORT` environment variable:
//! - "inprocess" or "embedded" (default): Embed Conductor in TUI process
//! - "unix" or "socket": Connect to external Conductor via Unix socket
//!
//! # Reconnection
//!
//! For remote transports (Unix socket), the client supports automatic reconnection
//! with exponential backoff. Configure via TransportConfig:
//! - reconnect_attempts: Number of retry attempts (0 = disabled)
//! - reconnect_delay_ms: Initial delay between attempts (doubles each retry)

use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use conductor_core::{
    transport::{
        SurfaceTransport, TransportConfig, TransportError, TransportType, UnixSocketClient,
    },
    Conductor, ConductorConfig, ConductorMessage, ConductorState, OllamaBackend,
    SurfaceCapabilities, SurfaceEvent, SurfaceType,
};

/// Connection state for remote transports
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected to Conductor
    Disconnected,
    /// Attempting initial connection
    Connecting,
    /// Successfully connected
    Connected,
    /// Lost connection, attempting to reconnect
    Reconnecting,
}

/// Client mode - either embedded Conductor or remote via transport
enum ClientMode {
    /// In-process mode with embedded Conductor
    InProcess {
        conductor: Conductor<OllamaBackend>,
        rx: mpsc::Receiver<ConductorMessage>,
    },
    /// Unix socket mode connecting to external Conductor
    UnixSocket { transport: UnixSocketClient },
}

/// Client for communicating with the Conductor
///
/// Abstracts over different transport mechanisms. The TUI doesn't need
/// to know whether the Conductor is embedded or remote.
pub struct ConductorClient {
    /// The client mode (in-process or remote)
    mode: ClientMode,
    /// Transport configuration
    config: TransportConfig,
    /// Current connection state for remote mode
    connection_state: ConnectionState,
    /// Number of reconnection attempts made
    reconnect_count: u32,
}

impl ConductorClient {
    /// Create a new ConductorClient with default (embedded) transport
    pub fn new() -> Self {
        Self::with_config(TransportConfig::embedded())
    }

    /// Create a ConductorClient from environment configuration
    ///
    /// Reads `CONDUCTOR_TRANSPORT` and related environment variables.
    pub fn from_env() -> Self {
        Self::with_config(TransportConfig::from_env())
    }

    /// Create a ConductorClient with specific transport configuration
    pub fn with_config(config: TransportConfig) -> Self {
        match &config.transport {
            TransportType::InProcess => {
                info!("Starting in embedded mode (Conductor in-process)");

                // Create channel for Conductor -> TUI messages
                let (tx, rx) = mpsc::channel(100);

                // Create Ollama backend from environment
                let backend = OllamaBackend::from_env();

                // Create Conductor config from environment
                // Disable warmup so TUI starts immediately responsive
                // The first user message will warm up the model instead
                let mut conductor_config = ConductorConfig::from_env();
                conductor_config.warmup_on_start = false;

                // Create the Conductor
                let conductor = Conductor::new(backend, conductor_config, tx);

                Self {
                    mode: ClientMode::InProcess { conductor, rx },
                    config,
                    connection_state: ConnectionState::Disconnected,
                    reconnect_count: 0,
                }
            }

            #[cfg(unix)]
            TransportType::UnixSocket { path } => {
                let socket_path = path
                    .clone()
                    .unwrap_or_else(conductor_core::transport::unix_socket::default_socket_path);

                info!(socket = %socket_path.display(), "Starting in Unix socket mode (remote Conductor)");

                let transport = UnixSocketClient::new(socket_path);

                Self {
                    mode: ClientMode::UnixSocket { transport },
                    config,
                    connection_state: ConnectionState::Disconnected,
                    reconnect_count: 0,
                }
            }
        }
    }

    /// Start the Conductor (initialize and warm up)
    ///
    /// For in-process mode, this starts the embedded Conductor.
    /// For remote modes, this is a no-op (the daemon starts separately).
    pub async fn start(&mut self) -> anyhow::Result<()> {
        match &mut self.mode {
            ClientMode::InProcess { conductor, .. } => conductor.start().await,
            ClientMode::UnixSocket { .. } => {
                // Remote conductor is started separately
                Ok(())
            }
        }
    }

    /// Connect this surface to the Conductor
    ///
    /// For in-process mode, sends Connected event directly.
    /// For remote modes, establishes the transport connection first.
    pub async fn connect(&mut self) -> anyhow::Result<()> {
        self.connection_state = ConnectionState::Connecting;

        match &mut self.mode {
            ClientMode::InProcess { conductor, .. } => {
                let event = SurfaceEvent::Connected {
                    event_id: SurfaceEvent::new_event_id(),
                    surface_type: SurfaceType::Tui,
                    capabilities: SurfaceCapabilities::tui(),
                };
                conductor.handle_event(event).await?;
                self.connection_state = ConnectionState::Connected;
                self.reconnect_count = 0;
                Ok(())
            }
            ClientMode::UnixSocket { transport } => {
                // Connect to the Unix socket
                transport.connect().await.map_err(transport_to_anyhow)?;

                // Send Connected event
                let event = SurfaceEvent::Connected {
                    event_id: SurfaceEvent::new_event_id(),
                    surface_type: SurfaceType::Tui,
                    capabilities: SurfaceCapabilities::tui(),
                };
                transport.send(event).await.map_err(transport_to_anyhow)?;

                self.connection_state = ConnectionState::Connected;
                self.reconnect_count = 0;
                info!("Connected to Conductor daemon");
                Ok(())
            }
        }
    }

    /// Attempt to reconnect to the Conductor with exponential backoff
    ///
    /// Only applicable for remote transports (Unix socket). For in-process
    /// mode, this returns immediately as there's nothing to reconnect.
    ///
    /// Uses TransportConfig settings:
    /// - reconnect_attempts: Maximum number of retries (0 = disabled)
    /// - reconnect_delay_ms: Initial delay, doubles each attempt
    ///
    /// Returns Ok(true) if reconnected, Ok(false) if max attempts reached,
    /// Err if a non-retriable error occurred.
    pub async fn try_reconnect(&mut self) -> anyhow::Result<bool> {
        // In-process mode doesn't need reconnection
        if matches!(self.mode, ClientMode::InProcess { .. }) {
            return Ok(true);
        }

        // Check if reconnection is disabled
        if self.config.reconnect_attempts == 0 {
            warn!("Reconnection disabled (reconnect_attempts = 0)");
            return Ok(false);
        }

        // Check if we've exhausted attempts
        if self.reconnect_count >= self.config.reconnect_attempts {
            error!(
                "Max reconnection attempts ({}) reached",
                self.config.reconnect_attempts
            );
            self.connection_state = ConnectionState::Disconnected;
            return Ok(false);
        }

        self.connection_state = ConnectionState::Reconnecting;

        // Calculate delay with exponential backoff
        let delay_ms = self.config.reconnect_delay_ms * (1 << self.reconnect_count.min(6));
        self.reconnect_count += 1;

        info!(
            attempt = self.reconnect_count,
            max_attempts = self.config.reconnect_attempts,
            delay_ms = delay_ms,
            "Attempting reconnection"
        );

        // Wait before attempting
        sleep(Duration::from_millis(delay_ms)).await;

        // Attempt reconnection
        match &mut self.mode {
            ClientMode::UnixSocket { transport } => {
                // Disconnect first to clean up any stale state
                let _ = transport.disconnect().await;

                // Try to connect
                match transport.connect().await {
                    Ok(()) => {
                        // Re-send Connected event
                        let event = SurfaceEvent::Connected {
                            event_id: SurfaceEvent::new_event_id(),
                            surface_type: SurfaceType::Tui,
                            capabilities: SurfaceCapabilities::tui(),
                        };
                        transport.send(event).await.map_err(transport_to_anyhow)?;

                        self.connection_state = ConnectionState::Connected;
                        self.reconnect_count = 0;
                        info!("Reconnected to Conductor daemon");
                        Ok(true)
                    }
                    Err(e) => {
                        debug!(error = %e, "Reconnection attempt failed");
                        // Stay in Reconnecting state, caller should retry
                        Ok(false)
                    }
                }
            }
            ClientMode::InProcess { .. } => Ok(true),
        }
    }

    /// Get the current connection state
    pub fn connection_state(&self) -> ConnectionState {
        self.connection_state
    }

    /// Check if a reconnection attempt is in progress
    pub fn is_reconnecting(&self) -> bool {
        self.connection_state == ConnectionState::Reconnecting
    }

    /// Reset reconnection counter (call after successful operations)
    pub fn reset_reconnect_count(&mut self) {
        self.reconnect_count = 0;
    }

    /// Handle a potential connection loss
    ///
    /// Call this when you detect the connection may have dropped
    /// (e.g., send/recv errors). Returns true if reconnection should be attempted.
    pub fn handle_connection_loss(&mut self) -> bool {
        if matches!(self.mode, ClientMode::InProcess { .. }) {
            // In-process mode doesn't lose connection
            return false;
        }

        if self.connection_state == ConnectionState::Connected {
            warn!("Connection to Conductor lost");
            self.connection_state = ConnectionState::Reconnecting;
            true
        } else {
            // Already disconnected or reconnecting
            false
        }
    }

    /// Send a user message to the Conductor
    pub async fn send_message(&mut self, content: String) -> anyhow::Result<()> {
        let event = SurfaceEvent::UserMessage {
            event_id: SurfaceEvent::new_event_id(),
            content,
        };
        self.send_event(event).await
    }

    /// Send a user command to the Conductor
    pub async fn send_command(&mut self, command: String, args: Vec<String>) -> anyhow::Result<()> {
        let event = SurfaceEvent::UserCommand {
            event_id: SurfaceEvent::new_event_id(),
            command,
            args,
        };
        self.send_event(event).await
    }

    /// Notify Conductor that user is typing
    pub async fn user_typing(&mut self, typing: bool) -> anyhow::Result<()> {
        let event = SurfaceEvent::UserTyping { typing };
        self.send_event(event).await
    }

    /// Notify Conductor that user scrolled
    pub async fn user_scrolled(
        &mut self,
        direction: conductor_core::ScrollDirection,
        amount: u32,
    ) -> anyhow::Result<()> {
        let event = SurfaceEvent::UserScrolled { direction, amount };
        self.send_event(event).await
    }

    /// Notify Conductor that user clicked the avatar
    pub async fn avatar_clicked(&mut self) -> anyhow::Result<()> {
        let event = SurfaceEvent::AvatarClicked {
            event_id: SurfaceEvent::new_event_id(),
        };
        self.send_event(event).await
    }

    /// Notify Conductor that user wants to quit
    pub async fn request_quit(&mut self) -> anyhow::Result<()> {
        let event = SurfaceEvent::QuitRequested {
            event_id: SurfaceEvent::new_event_id(),
        };
        self.send_event(event).await
    }

    /// Notify Conductor of resize
    pub async fn resized(&mut self, width: u32, height: u32) -> anyhow::Result<()> {
        let event = SurfaceEvent::Resized {
            event_id: SurfaceEvent::new_event_id(),
            width,
            height,
        };
        self.send_event(event).await
    }

    /// Poll for streaming tokens (must be called regularly)
    ///
    /// For in-process mode, this directly polls the embedded Conductor.
    /// For remote modes, streaming is handled by the transport layer.
    pub async fn poll_streaming(&mut self) -> bool {
        match &mut self.mode {
            ClientMode::InProcess { conductor, .. } => conductor.poll_streaming().await,
            ClientMode::UnixSocket { .. } => {
                // Remote conductor handles streaming internally
                // Messages arrive via transport
                false
            }
        }
    }

    /// Try to receive a message from the Conductor (non-blocking)
    pub fn try_recv(&mut self) -> Option<ConductorMessage> {
        match &mut self.mode {
            ClientMode::InProcess { rx, .. } => rx.try_recv().ok(),
            ClientMode::UnixSocket { transport } => transport.try_recv(),
        }
    }

    /// Receive all pending messages from the Conductor (non-blocking)
    pub fn recv_all(&mut self) -> Vec<ConductorMessage> {
        let mut messages = Vec::new();
        while let Some(msg) = self.try_recv() {
            messages.push(msg);
        }
        messages
    }

    /// Get the current Conductor state
    ///
    /// For in-process mode, returns the actual Conductor state.
    /// For remote modes, returns best-known state (may be stale).
    pub fn state(&self) -> ConductorState {
        match &self.mode {
            ClientMode::InProcess { conductor, .. } => conductor.state(),
            ClientMode::UnixSocket { transport } => {
                // For remote mode, we track state via messages
                // Return Ready if connected, Initializing otherwise
                if transport.is_connected() {
                    ConductorState::Ready
                } else {
                    ConductorState::Initializing
                }
            }
        }
    }

    /// Check if the Conductor is ready
    pub fn is_ready(&self) -> bool {
        match &self.mode {
            ClientMode::InProcess { conductor, .. } => conductor.is_ready(),
            ClientMode::UnixSocket { transport } => transport.is_connected(),
        }
    }

    /// Check if using in-process (embedded) mode
    pub fn is_embedded(&self) -> bool {
        matches!(self.mode, ClientMode::InProcess { .. })
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connection_state == ConnectionState::Connected
    }

    /// Get the transport configuration
    pub fn transport_config(&self) -> &TransportConfig {
        &self.config
    }

    /// Send raw surface event to Conductor
    pub async fn send_event(&mut self, event: SurfaceEvent) -> anyhow::Result<()> {
        match &mut self.mode {
            ClientMode::InProcess { conductor, .. } => conductor.handle_event(event).await,
            ClientMode::UnixSocket { transport } => {
                transport.send(event).await.map_err(transport_to_anyhow)
            }
        }
    }
}

impl Default for ConductorClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert TransportError to anyhow::Error
fn transport_to_anyhow(err: TransportError) -> anyhow::Error {
    anyhow::anyhow!("{}", err)
}
