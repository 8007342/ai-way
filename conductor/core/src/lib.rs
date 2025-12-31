//! Conductor Core - Headless Meta-Agent Orchestration for ai-way
//!
//! This crate provides the core orchestration logic for ai-way, completely
//! independent of any UI framework. It can drive a TUI, WebUI, native GUI,
//! mobile app, or run headless for testing/automation.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        UI Surfaces                               │
//! │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────────────┐ │
//! │  │   TUI   │  │  WebUI  │  │ Desktop │  │  Mobile / Headless  │ │
//! │  │(ratatui)│  │ (Yew)   │  │ (Tauri) │  │                     │ │
//! │  └────┬────┘  └────┬────┘  └────┬────┘  └──────────┬──────────┘ │
//! │       │            │            │                  │            │
//! │       └────────────┴────────────┴──────────────────┘            │
//! │                           │                                      │
//! │                    SurfaceEvent (up)                            │
//! │                  ConductorMessage (down)                        │
//! │                           │                                      │
//! └───────────────────────────┼──────────────────────────────────────┘
//!                             │
//! ┌───────────────────────────┼──────────────────────────────────────┐
//! │                    CONDUCTOR CORE                                │
//! │  ┌────────────────────────┴────────────────────────────────────┐ │
//! │  │                      Conductor                               │ │
//! │  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐ │ │
//! │  │  │ Session  │  │  Avatar  │  │  Tasks   │  │   Backend    │ │ │
//! │  │  │ Manager  │  │  State   │  │ Manager  │  │   (LLM)      │ │ │
//! │  │  └──────────┘  └──────────┘  └──────────┘  └──────────────┘ │ │
//! │  └─────────────────────────────────────────────────────────────┘ │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Types
//!
//! - [`Conductor`]: The main orchestration struct that manages everything
//! - [`ConductorMessage`]: Messages sent from Conductor to UI surfaces
//! - [`SurfaceEvent`]: Events sent from UI surfaces to Conductor
//! - [`Session`]: Conversation session with message history
//! - [`AvatarState`]: Current state of Yollayah's avatar
//! - [`TaskManager`]: Manages background tasks (specialist agents)
//!
//! # Quick Start
//!
//! ```ignore
//! use conductor_core::{
//!     Conductor, ConductorConfig,
//!     backend::OllamaBackend,
//!     events::{SurfaceEvent, SurfaceType, SurfaceCapabilities},
//! };
//! use tokio::sync::mpsc;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create channels for communication
//!     let (tx, mut rx) = mpsc::channel(100);
//!
//!     // Create Conductor with Ollama backend
//!     let backend = OllamaBackend::from_env();
//!     let config = ConductorConfig::from_env();
//!     let mut conductor = Conductor::new(backend, config, tx);
//!
//!     // Start the Conductor
//!     conductor.start().await.unwrap();
//!
//!     // Connect a surface
//!     conductor.handle_event(SurfaceEvent::Connected {
//!         event_id: SurfaceEvent::new_event_id(),
//!         surface_type: SurfaceType::Tui,
//!         capabilities: SurfaceCapabilities::tui(),
//!     }).await.unwrap();
//!
//!     // Main loop: process events and poll streaming
//!     loop {
//!         // Handle incoming messages from Conductor
//!         while let Ok(msg) = rx.try_recv() {
//!             // Render message to UI
//!         }
//!
//!         // Poll for streaming tokens
//!         conductor.poll_streaming().await;
//!
//!         // Handle user input, send as SurfaceEvent
//!     }
//! }
//! ```
//!
//! # Module Overview
//!
//! - [`avatar`]: Avatar state, moods, gestures, and command parsing
//! - [`backend`]: LLM backend abstraction (Ollama, etc.)
//! - [`events`]: Events from UI surfaces to Conductor
//! - [`messages`]: Messages from Conductor to UI surfaces
//! - [`session`]: Conversation session management
//! - [`tasks`]: Background task management
//! - [`conductor`]: Main Conductor struct
//! - [`transport`]: IPC transport layer (Unix sockets, WebSocket)
//!
//! # No TUI Dependencies
//!
//! This crate has **zero** dependencies on ratatui, crossterm, or any other
//! UI framework. It's pure business logic that can be used anywhere.

#![deny(missing_docs)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod avatar;
pub mod backend;
pub mod conductor;
pub mod events;
pub mod messages;
pub mod security;
pub mod session;
pub mod tasks;
pub mod transport;

// Re-exports for convenience
pub use avatar::{
    AvatarCommand, AvatarGesture, AvatarMood, AvatarPosition, AvatarReaction, AvatarSize,
    AvatarState, CommandParser, PeekDirection, TaskCommand,
};
pub use backend::{BackendConfig, LlmBackend, LlmRequest, LlmResponse, OllamaBackend, StreamingToken};
pub use conductor::{Conductor, ConductorConfig};
pub use events::{ScrollDirection, SurfaceCapabilities, SurfaceEvent, SurfaceType};
pub use messages::{
    ConductorMessage, ConductorState, EventId, MessageId, MessageRole, NotifyLevel, SessionId,
};
pub use session::{ConversationMessage, Session, SessionMetadata, SessionState};
pub use security::{
    CommandRejectionReason, CommandValidator, ConductorLimits, InputValidator, SecurityConfig,
    ValidationResult,
};
pub use tasks::{Task, TaskCreationError, TaskId, TaskManager, TaskStatus};
