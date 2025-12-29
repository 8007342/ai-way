//! Backend Integration
//!
//! Communication with Ollama and the ai-way server.

mod client;
mod messages;

pub use client::{BackendClient, BackendConnection};
pub use messages::{BackendResponse, StreamingToken, UserMessage};
