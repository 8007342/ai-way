//! LLM Backend Integration
//!
//! This module provides abstracted access to LLM backends (Ollama, OpenAI, etc.)
//! through a common trait interface.
//!
//! # Available Backends
//!
//! - **Ollama**: Local LLM server (default)
//! - More to come: OpenAI, Anthropic, etc.
//!
//! # Usage
//!
//! ```ignore
//! use conductor_core::backend::{OllamaBackend, LlmBackend, LlmRequest};
//!
//! let backend = OllamaBackend::from_env();
//! let request = LlmRequest::new("Hello!", "llama2");
//! let rx = backend.send_streaming(&request).await?;
//! ```

mod ollama;
mod traits;

pub use ollama::OllamaBackend;
pub use traits::{
    BackendConfig, LlmBackend, LlmRequest, LlmResponse, ModelInfo, StreamingToken,
};
