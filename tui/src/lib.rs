//! Yollayah TUI - Terminal interface for ai-way
//!
//! This crate provides a full-screen terminal UI featuring an animated
//! axolotl avatar (Yollayah) as the soul of the user experience.
//!
//! # Architecture
//!
//! - **Compositor**: Layered rendering with z-ordering for avatar pop-in/out
//! - **Avatar**: Multi-size animated sprite system with state machine
//! - **Widgets**: Borderless scrollable text blocks
//! - **Backend**: Ollama/server integration with streaming

pub mod app;
pub mod compositor;
pub mod avatar;
pub mod widgets;
pub mod backend;
pub mod events;
pub mod theme;

pub use app::App;
