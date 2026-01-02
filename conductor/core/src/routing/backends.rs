//! Backend Adapters
//!
//! Adapters for different LLM backends that can be used with the router.
//! Each adapter implements a common interface for the router to interact with.

use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;

use super::config::BackendType;
use crate::backend::{LlmRequest, LlmResponse, StreamingToken};

// ============================================================================
// Backend Adapter Trait
// ============================================================================

/// Trait for backend adapters
#[async_trait]
pub trait BackendAdapter: Send + Sync {
    /// Get backend type name
    fn name(&self) -> &str;

    /// Check if backend is healthy
    async fn health_check(&self) -> bool;

    /// Send a streaming request
    async fn send_streaming(
        &self,
        request: &LlmRequest,
    ) -> Result<mpsc::Receiver<StreamingToken>, BackendAdapterError>;

    /// Send a non-streaming request
    async fn send(&self, request: &LlmRequest) -> Result<LlmResponse, BackendAdapterError>;

    /// List available models
    async fn list_models(&self) -> Result<Vec<String>, BackendAdapterError>;

    /// Load a model (for local backends)
    async fn load_model(&self, model_id: &str) -> Result<(), BackendAdapterError> {
        // Default: no-op for cloud backends
        let _ = model_id;
        Ok(())
    }

    /// Unload a model (for local backends)
    async fn unload_model(&self, model_id: &str) -> Result<(), BackendAdapterError> {
        // Default: no-op for cloud backends
        let _ = model_id;
        Ok(())
    }

    /// Get current resource usage
    fn resource_usage(&self) -> ResourceUsage {
        ResourceUsage::default()
    }
}

/// Resource usage info
#[derive(Clone, Debug, Default)]
pub struct ResourceUsage {
    /// GPU memory used (bytes)
    pub gpu_memory_bytes: Option<u64>,
    /// CPU memory used (bytes)
    pub cpu_memory_bytes: Option<u64>,
    /// Number of loaded models
    pub loaded_models: usize,
    /// Active request count
    pub active_requests: usize,
}

/// Backend adapter errors
#[derive(Clone, Debug)]
pub enum BackendAdapterError {
    /// Connection failed
    ConnectionFailed(String),
    /// Request failed
    RequestFailed(String),
    /// Model not found
    ModelNotFound(String),
    /// Resource exhausted
    ResourceExhausted(String),
    /// Timeout
    Timeout,
    /// Rate limited
    RateLimited { retry_after_ms: Option<u64> },
    /// Authentication failed
    AuthenticationFailed,
    /// Internal error
    Internal(String),
}

impl std::fmt::Display for BackendAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionFailed(e) => write!(f, "Connection failed: {}", e),
            Self::RequestFailed(e) => write!(f, "Request failed: {}", e),
            Self::ModelNotFound(m) => write!(f, "Model not found: {}", m),
            Self::ResourceExhausted(e) => write!(f, "Resource exhausted: {}", e),
            Self::Timeout => write!(f, "Request timed out"),
            Self::RateLimited { retry_after_ms } => {
                if let Some(ms) = retry_after_ms {
                    write!(f, "Rate limited, retry after {}ms", ms)
                } else {
                    write!(f, "Rate limited")
                }
            }
            Self::AuthenticationFailed => write!(f, "Authentication failed"),
            Self::Internal(e) => write!(f, "Internal error: {}", e),
        }
    }
}

impl std::error::Error for BackendAdapterError {}

// ============================================================================
// Ollama Adapter
// ============================================================================

/// Adapter for Ollama backend
pub struct OllamaAdapter {
    host: String,
    port: u16,
    client: reqwest::Client,
}

impl OllamaAdapter {
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    fn base_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

#[async_trait]
impl BackendAdapter for OllamaAdapter {
    fn name(&self) -> &str {
        "Ollama"
    }

    async fn health_check(&self) -> bool {
        self.client
            .get(format!("{}/api/tags", self.base_url()))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .is_ok()
    }

    async fn send_streaming(
        &self,
        request: &LlmRequest,
    ) -> Result<mpsc::Receiver<StreamingToken>, BackendAdapterError> {
        // Implementation would be similar to existing OllamaBackend
        // For now, return a placeholder
        let (tx, rx) = mpsc::channel(100);
        let model = request.model.clone();

        tokio::spawn(async move {
            let _ = tx.send(StreamingToken::Token("Hello ".to_string())).await;
            let _ = tx.send(StreamingToken::Token("from ".to_string())).await;
            let _ = tx.send(StreamingToken::Token(model.clone())).await;
            let _ = tx
                .send(StreamingToken::Complete {
                    message: format!("Hello from {}", model),
                })
                .await;
        });

        Ok(rx)
    }

    async fn send(&self, request: &LlmRequest) -> Result<LlmResponse, BackendAdapterError> {
        // Placeholder implementation
        Ok(LlmResponse {
            content: format!("Response from {}", request.model),
            model: request.model.clone(),
            tokens_used: Some(10),
            duration_ms: Some(100),
        })
    }

    async fn list_models(&self) -> Result<Vec<String>, BackendAdapterError> {
        let response = self
            .client
            .get(format!("{}/api/tags", self.base_url()))
            .send()
            .await
            .map_err(|e| BackendAdapterError::ConnectionFailed(e.to_string()))?;

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| BackendAdapterError::RequestFailed(e.to_string()))?;

        let models = data
            .get("models")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }
}

// ============================================================================
// OpenAI-Compatible Adapter
// ============================================================================

/// Adapter for OpenAI-compatible APIs
pub struct OpenAIAdapter {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl OpenAIAdapter {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            base_url,
            api_key,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }
}

#[async_trait]
impl BackendAdapter for OpenAIAdapter {
    fn name(&self) -> &str {
        "OpenAI"
    }

    async fn health_check(&self) -> bool {
        self.client
            .get(format!("{}/v1/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    async fn send_streaming(
        &self,
        request: &LlmRequest,
    ) -> Result<mpsc::Receiver<StreamingToken>, BackendAdapterError> {
        // Implementation would use SSE for streaming
        let (tx, rx) = mpsc::channel(100);
        let model = request.model.clone();

        tokio::spawn(async move {
            let _ = tx
                .send(StreamingToken::Token("Response from ".to_string()))
                .await;
            let _ = tx.send(StreamingToken::Token(model.clone())).await;
            let _ = tx
                .send(StreamingToken::Complete {
                    message: format!("Response from {}", model),
                })
                .await;
        });

        Ok(rx)
    }

    async fn send(&self, request: &LlmRequest) -> Result<LlmResponse, BackendAdapterError> {
        // Placeholder
        Ok(LlmResponse {
            content: format!("Response from {}", request.model),
            model: request.model.clone(),
            tokens_used: Some(10),
            duration_ms: Some(100),
        })
    }

    async fn list_models(&self) -> Result<Vec<String>, BackendAdapterError> {
        let response = self
            .client
            .get(format!("{}/v1/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| BackendAdapterError::ConnectionFailed(e.to_string()))?;

        if response.status() == 401 {
            return Err(BackendAdapterError::AuthenticationFailed);
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| BackendAdapterError::RequestFailed(e.to_string()))?;

        let models = data
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("id").and_then(|n| n.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }
}

// ============================================================================
// Backend Factory
// ============================================================================

/// Create a backend adapter from configuration
pub fn create_adapter(config: &BackendType) -> Box<dyn BackendAdapter> {
    match config {
        BackendType::Ollama { host, port } => Box::new(OllamaAdapter::new(host.clone(), *port)),
        BackendType::OpenAI {
            base_url,
            api_key_env,
        } => {
            let api_key = std::env::var(api_key_env).unwrap_or_default();
            Box::new(OpenAIAdapter::new(base_url.clone(), api_key))
        }
        BackendType::Anthropic { api_key_env } => {
            let api_key = std::env::var(api_key_env).unwrap_or_default();
            // Anthropic uses OpenAI-compatible adapter with different base URL
            Box::new(OpenAIAdapter::new(
                "https://api.anthropic.com".to_string(),
                api_key,
            ))
        }
        BackendType::LocalGgml {
            model_path: _,
            gpu_layers: _,
        } => {
            // For now, use Ollama adapter as a placeholder
            // A real implementation would load the model directly
            Box::new(OllamaAdapter::new("localhost".to_string(), 11434))
        }
        BackendType::Grpc {
            endpoint: _,
            use_tls: _,
        } => {
            // Placeholder - would need gRPC implementation
            Box::new(OllamaAdapter::new("localhost".to_string(), 11434))
        }
        BackendType::CustomHttp {
            base_url,
            auth_header: _,
        } => {
            // Use OpenAI adapter as base
            Box::new(OpenAIAdapter::new(base_url.clone(), String::new()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_adapter_creation() {
        let adapter = OllamaAdapter::new("localhost".to_string(), 11434);
        assert_eq!(adapter.name(), "Ollama");
        assert_eq!(adapter.base_url(), "http://localhost:11434");
    }

    #[test]
    fn test_backend_factory() {
        let config = BackendType::Ollama {
            host: "localhost".to_string(),
            port: 11434,
        };
        let adapter = create_adapter(&config);
        assert_eq!(adapter.name(), "Ollama");
    }
}
