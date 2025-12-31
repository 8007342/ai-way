//! Yollayah TUI - Terminal interface for ai-way
//!
//! This crate provides a full-screen terminal UI featuring an animated
//! axolotl avatar (Yollayah) as the soul of the user experience.
//!
//! # Architecture (Phase 2 - Thin Client)
//!
//! The TUI is a "thin client" that renders what the Conductor tells it to.
//! All business logic lives in `conductor-core`.
//!
//! - **ConductorClient**: Wraps communication with the embedded Conductor
//! - **Display**: Display state types derived from ConductorMessages
//! - **Compositor**: Layered rendering with z-ordering for avatar pop-in/out
//! - **Avatar**: Multi-size animated sprite system
//! - **Widgets**: Borderless scrollable text blocks
//!
//! ## Event Flow
//!
//! ```text
//! Terminal Events -> SurfaceEvent -> Conductor -> ConductorMessage -> Display State -> Render
//! ```

pub mod app;
pub mod compositor;
pub mod conductor_client;
pub mod display;
pub mod avatar;
pub mod widgets;
pub mod backend;
pub mod events;
pub mod theme;
pub mod tasks;

pub use app::App;
pub use conductor_client::ConductorClient;
pub use display::{DisplayMessage, DisplayState, DisplayAvatarState, DisplayTask};
pub use tasks::{TaskPanel, TaskState, BackgroundTask};
