//! Ollama Backend Implementation
//!
//! LLM backend for Ollama (local LLM server).
//! Ported from tui/src/backend/client.rs with trait-based abstraction.
//!
//! # Ollama API
//!
//! Ollama provides a REST API for:
//! - `/api/generate` - Generate completions (streaming or batch)
//! - `/api/chat` - Chat completions with message history
//! - `/api/tags` - List available models
//!
//! This implementation uses the generate endpoint with streaming support.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures::StreamExt;
use tokio::sync::mpsc;

use super::traits::{
    BackendConfig, LlmBackend, LlmRequest, LlmResponse, ModelInfo, StreamingToken,
};

/// Ollama backend client
#[derive(Clone)]
pub struct OllamaBackend {
    /// Host address
    host: String,
    /// Port number
    port: u16,
    /// HTTP client
    http_client: reqwest::Client,
}

impl OllamaBackend {
    /// Create a new Ollama backend
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Create from `BackendConfig`
    #[must_use]
    pub fn from_config(config: &BackendConfig) -> Option<Self> {
        match config {
            BackendConfig::Ollama { host, port } => Some(Self::new(host.clone(), *port)),
            _ => None,
        }
    }

    /// Create from environment variables
    #[must_use]
    pub fn from_env() -> Self {
        let host = std::env::var("OLLAMA_HOST")
            .or_else(|_| std::env::var("YOLLAYAH_OLLAMA_HOST"))
            .unwrap_or_else(|_| "localhost".to_string());
        let port: u16 = std::env::var("OLLAMA_PORT")
            .or_else(|_| std::env::var("YOLLAYAH_OLLAMA_PORT"))
            .unwrap_or_else(|_| "11434".to_string())
            .parse()
            .unwrap_or(11434);

        Self::new(host, port)
    }

    /// Get the base URL
    fn base_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    /// Get generate endpoint URL
    fn generate_url(&self) -> String {
        format!("{}/api/generate", self.base_url())
    }

    /// Get tags endpoint URL
    fn tags_url(&self) -> String {
        format!("{}/api/tags", self.base_url())
    }

    /// Build the full prompt including system and context
    fn build_prompt(&self, request: &LlmRequest) -> String {
        let mut full_prompt = String::new();

        if let Some(ref system) = request.system {
            full_prompt.push_str(system);
            full_prompt.push_str("\n\n");
        }

        if let Some(ref context) = request.context {
            full_prompt.push_str(context);
            full_prompt.push('\n');
        }

        full_prompt.push_str(&request.prompt);
        full_prompt
    }
}

impl Default for OllamaBackend {
    fn default() -> Self {
        Self::new("localhost", 11434)
    }
}

#[async_trait]
impl LlmBackend for OllamaBackend {
    fn name(&self) -> &'static str {
        "Ollama"
    }

    async fn health_check(&self) -> bool {
        self.http_client
            .get(self.tags_url())
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .is_ok()
    }

    async fn send_streaming(
        &self,
        request: &LlmRequest,
    ) -> anyhow::Result<mpsc::Receiver<StreamingToken>> {
        let (tx, rx) = mpsc::channel(100);

        let url = self.generate_url();
        let prompt = self.build_prompt(request);

        let mut json_request = serde_json::json!({
            "model": request.model,
            "prompt": prompt,
            "stream": true,
        });

        // Add optional parameters
        if request.temperature != 0.7 {
            json_request["options"] = serde_json::json!({
                "temperature": request.temperature
            });
        }

        if request.max_tokens > 0 {
            let options = json_request["options"].as_object_mut().map(|o| {
                o.insert(
                    "num_predict".to_string(),
                    serde_json::json!(request.max_tokens),
                );
            });
            if options.is_none() {
                json_request["options"] = serde_json::json!({
                    "num_predict": request.max_tokens
                });
            }
        }

        let response = self
            .http_client
            .post(&url)
            .json(&json_request)
            .send()
            .await?;

        // Check for HTTP errors
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Ollama returned {status}: {body}");
        }

        let mut stream = response.bytes_stream();

        // Spawn task to process stream
        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut full_response = String::new();

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        // Parse newline-delimited JSON
                        while let Some(pos) = buffer.find('\n') {
                            let line = buffer[..pos].trim();
                            if !line.is_empty() {
                                if let Ok(data) = serde_json::from_str::<serde_json::Value>(line) {
                                    // Extract token
                                    if let Some(token) =
                                        data.get("response").and_then(|r| r.as_str())
                                    {
                                        full_response.push_str(token);
                                        if tx
                                            .send(StreamingToken::Token(token.to_string()))
                                            .await
                                            .is_err()
                                        {
                                            // Receiver dropped, stop streaming
                                            return;
                                        }
                                    }

                                    // Check if done
                                    if data
                                        .get("done")
                                        .and_then(serde_json::Value::as_bool)
                                        .unwrap_or(false)
                                    {
                                        let _ = tx
                                            .send(StreamingToken::Complete {
                                                message: full_response,
                                            })
                                            .await;
                                        return;
                                    }
                                }
                            }
                            buffer = buffer[pos + 1..].to_string();
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(StreamingToken::Error(e.to_string())).await;
                        return;
                    }
                }
            }

            // Stream ended without done signal
            if !full_response.is_empty() {
                let _ = tx
                    .send(StreamingToken::Complete {
                        message: full_response,
                    })
                    .await;
            }
        });

        Ok(rx)
    }

    async fn send(&self, request: &LlmRequest) -> anyhow::Result<LlmResponse> {
        let start = Instant::now();
        let url = self.generate_url();
        let prompt = self.build_prompt(request);

        let mut json_request = serde_json::json!({
            "model": request.model,
            "prompt": prompt,
            "stream": false,
        });

        if request.temperature != 0.7 {
            json_request["options"] = serde_json::json!({
                "temperature": request.temperature
            });
        }

        if request.max_tokens > 0 {
            let options = json_request["options"].as_object_mut().map(|o| {
                o.insert(
                    "num_predict".to_string(),
                    serde_json::json!(request.max_tokens),
                );
            });
            if options.is_none() {
                json_request["options"] = serde_json::json!({
                    "num_predict": request.max_tokens
                });
            }
        }

        let response = self
            .http_client
            .post(&url)
            .json(&json_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Ollama returned {status}: {body}");
        }

        let data: serde_json::Value = response.json().await?;

        let content = data
            .get("response")
            .and_then(|r| r.as_str())
            .unwrap_or("")
            .to_string();

        let tokens_used = data
            .get("eval_count")
            .and_then(serde_json::Value::as_u64)
            .map(|c| c as u32);

        Ok(LlmResponse {
            content,
            model: request.model.clone(),
            tokens_used,
            duration_ms: Some(start.elapsed().as_millis() as u64),
        })
    }

    async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        let response = self
            .http_client
            .get(self.tags_url())
            .timeout(Duration::from_secs(10))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Ollama returned {status}: {body}");
        }

        let data: serde_json::Value = response.json().await?;

        let models = data
            .get("models")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| {
                        let name = m.get("name")?.as_str()?.to_string();
                        let size = m.get("size").and_then(serde_json::Value::as_u64);
                        let parameters = m
                            .get("details")
                            .and_then(|d| d.get("parameter_size"))
                            .and_then(|p| p.as_str())
                            .map(String::from);

                        Some(ModelInfo {
                            name,
                            description: None,
                            size,
                            parameters,
                            loaded: true, // Ollama models in tags are available
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_backend_creation() {
        let backend = OllamaBackend::new("localhost", 11434);
        assert_eq!(backend.host, "localhost");
        assert_eq!(backend.port, 11434);
        assert_eq!(backend.base_url(), "http://localhost:11434");
    }

    #[test]
    fn test_build_prompt() {
        let backend = OllamaBackend::default();

        // Simple prompt
        let request = LlmRequest::new("Hello", "test");
        assert_eq!(backend.build_prompt(&request), "Hello");

        // With system
        let request = LlmRequest::new("Hello", "test").with_system("Be helpful");
        assert_eq!(backend.build_prompt(&request), "Be helpful\n\nHello");

        // With context
        let request = LlmRequest::new("Hello", "test").with_context("Previous: Hi");
        assert_eq!(backend.build_prompt(&request), "Previous: Hi\nHello");

        // With both
        let request = LlmRequest::new("Hello", "test")
            .with_system("Be helpful")
            .with_context("Previous: Hi");
        assert_eq!(
            backend.build_prompt(&request),
            "Be helpful\n\nPrevious: Hi\nHello"
        );
    }

    #[test]
    fn test_from_config() {
        let config = BackendConfig::Ollama {
            host: "example.com".to_string(),
            port: 8080,
        };

        let backend = OllamaBackend::from_config(&config).unwrap();
        assert_eq!(backend.host, "example.com");
        assert_eq!(backend.port, 8080);

        // Wrong config type returns None
        let config = BackendConfig::OpenAI {
            api_key: "test".to_string(),
            base_url: None,
        };
        assert!(OllamaBackend::from_config(&config).is_none());
    }
}
