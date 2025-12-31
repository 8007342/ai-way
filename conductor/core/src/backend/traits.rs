//! LLM Backend Traits
//!
//! Trait definitions for LLM backends. This abstraction allows the Conductor
//! to work with different LLM providers (Ollama, OpenAI, Anthropic, etc.)
//! without changing core logic.
//!
//! # Design Philosophy
//!
//! The LlmBackend trait provides a common interface for:
//! - Sending prompts and receiving responses (streaming or batch)
//! - Health checking the backend
//! - Querying available models
//!
//! Implementations handle provider-specific details (API formats, auth, etc.)

use async_trait::async_trait;
use tokio::sync::mpsc;

/// Token stream events from LLM backends
#[derive(Clone, Debug)]
pub enum StreamingToken {
    /// A token from the response
    Token(String),
    /// Response completed successfully
    Complete {
        /// The complete message (may differ from concatenated tokens)
        message: String,
    },
    /// Error occurred during streaming
    Error(String),
}

/// Configuration for LLM requests
#[derive(Clone, Debug)]
pub struct LlmRequest {
    /// The prompt/message to send
    pub prompt: String,
    /// Model to use (backend-specific identifier)
    pub model: String,
    /// Whether to stream the response
    pub stream: bool,
    /// Maximum tokens in response (0 = default)
    pub max_tokens: u32,
    /// Temperature (0.0-1.0, higher = more creative)
    pub temperature: f32,
    /// System prompt (optional, prepended to conversation)
    pub system: Option<String>,
    /// Conversation context (previous messages)
    pub context: Option<String>,
}

impl Default for LlmRequest {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            model: String::new(),
            stream: true,
            max_tokens: 0,
            temperature: 0.7,
            system: None,
            context: None,
        }
    }
}

impl LlmRequest {
    /// Create a new request with prompt and model
    pub fn new(prompt: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            model: model.into(),
            ..Default::default()
        }
    }

    /// Set streaming mode
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 1.0);
        self
    }

    /// Set system prompt
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Set context
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }
}

/// Response from non-streaming LLM request
#[derive(Clone, Debug)]
pub struct LlmResponse {
    /// The response text
    pub content: String,
    /// Model that generated the response
    pub model: String,
    /// Tokens used (if available)
    pub tokens_used: Option<u32>,
    /// Response generation time in milliseconds
    pub duration_ms: Option<u64>,
}

/// Information about an available model
#[derive(Clone, Debug)]
pub struct ModelInfo {
    /// Model identifier
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Model size in bytes (if known)
    pub size: Option<u64>,
    /// Parameter count (if known)
    pub parameters: Option<String>,
    /// Whether the model is loaded/ready
    pub loaded: bool,
}

/// LLM Backend trait
///
/// Implement this trait to add support for different LLM providers.
#[async_trait]
pub trait LlmBackend: Send + Sync {
    /// Get the backend name (e.g., "Ollama", "OpenAI")
    fn name(&self) -> &str;

    /// Check if the backend is healthy and reachable
    async fn health_check(&self) -> bool;

    /// Send a request and get a streaming response
    ///
    /// Returns a channel receiver that will receive tokens as they arrive.
    /// The channel will be closed when the response is complete or an error occurs.
    async fn send_streaming(
        &self,
        request: &LlmRequest,
    ) -> anyhow::Result<mpsc::Receiver<StreamingToken>>;

    /// Send a request and wait for complete response (non-streaming)
    ///
    /// This is useful for quick queries where streaming isn't needed.
    async fn send(&self, request: &LlmRequest) -> anyhow::Result<LlmResponse>;

    /// List available models
    async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>>;

    /// Check if a specific model is available
    async fn has_model(&self, model: &str) -> anyhow::Result<bool> {
        let models = self.list_models().await?;
        Ok(models.iter().any(|m| m.name == model))
    }

    /// Get information about a specific model
    async fn model_info(&self, model: &str) -> anyhow::Result<Option<ModelInfo>> {
        let models = self.list_models().await?;
        Ok(models.into_iter().find(|m| m.name == model))
    }
}

/// Backend connection configuration
#[derive(Clone, Debug)]
pub enum BackendConfig {
    /// Direct Ollama connection
    Ollama {
        /// Ollama host address
        host: String,
        /// Ollama port number
        port: u16,
    },
    /// OpenAI-compatible API
    OpenAI {
        /// API key for authentication
        api_key: String,
        /// Custom base URL (optional)
        base_url: Option<String>,
    },
    /// Anthropic API
    Anthropic {
        /// API key for authentication
        api_key: String,
    },
    /// Custom backend
    Custom {
        /// Backend name
        name: String,
        /// Configuration key-value pairs
        config: std::collections::HashMap<String, String>,
    },
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self::Ollama {
            host: "localhost".to_string(),
            port: 11434,
        }
    }
}

impl BackendConfig {
    /// Create Ollama configuration
    pub fn ollama(host: impl Into<String>, port: u16) -> Self {
        Self::Ollama {
            host: host.into(),
            port,
        }
    }

    /// Create Ollama configuration from environment
    pub fn ollama_from_env() -> Self {
        let host = std::env::var("OLLAMA_HOST")
            .or_else(|_| std::env::var("YOLLAYAH_OLLAMA_HOST"))
            .unwrap_or_else(|_| "localhost".to_string());
        let port: u16 = std::env::var("OLLAMA_PORT")
            .or_else(|_| std::env::var("YOLLAYAH_OLLAMA_PORT"))
            .unwrap_or_else(|_| "11434".to_string())
            .parse()
            .unwrap_or(11434);

        Self::Ollama { host, port }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_request_builder() {
        let request = LlmRequest::new("Hello", "llama2")
            .with_stream(false)
            .with_temperature(0.5)
            .with_system("You are helpful")
            .with_max_tokens(100);

        assert_eq!(request.prompt, "Hello");
        assert_eq!(request.model, "llama2");
        assert!(!request.stream);
        assert!((request.temperature - 0.5).abs() < f32::EPSILON);
        assert_eq!(request.system, Some("You are helpful".to_string()));
        assert_eq!(request.max_tokens, 100);
    }

    #[test]
    fn test_backend_config_default() {
        let config = BackendConfig::default();
        match config {
            BackendConfig::Ollama { host, port } => {
                assert_eq!(host, "localhost");
                assert_eq!(port, 11434);
            }
            _ => panic!("Expected Ollama config"),
        }
    }
}
