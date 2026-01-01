//! Backend Client
//!
//! Handles communication with Ollama or the ai-way server.

use std::time::Duration;

use tokio::sync::mpsc;

use super::messages::StreamingToken;

/// Connection configuration
#[derive(Clone, Debug)]
pub enum BackendConnection {
    /// Direct connection to Ollama
    DirectOllama { host: String, port: u16 },
    /// Through ai-way server
    CoreServer { host: String, port: u16 },
}

impl Default for BackendConnection {
    fn default() -> Self {
        Self::DirectOllama {
            host: "localhost".to_string(),
            port: 11434,
        }
    }
}

/// Backend client
#[derive(Clone)]
pub struct BackendClient {
    connection: BackendConnection,
    http_client: reqwest::Client,
}

impl BackendClient {
    /// Create a new client
    pub fn new(connection: BackendConnection) -> Self {
        Self {
            connection,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Send a message and get streaming response
    pub async fn send_message(
        &self,
        message: &str,
        model: &str,
    ) -> anyhow::Result<mpsc::Receiver<StreamingToken>> {
        let (tx, rx) = mpsc::channel(100);

        match &self.connection {
            BackendConnection::DirectOllama { host, port } => {
                self.stream_from_ollama(host, *port, message, model, tx)
                    .await?;
            }
            BackendConnection::CoreServer { host, port } => {
                // TODO: Implement server connection
                let _ = (host, port);
                tx.send(StreamingToken::Error(
                    "Server mode not yet implemented".to_string(),
                ))
                .await?;
            }
        }

        Ok(rx)
    }

    async fn stream_from_ollama(
        &self,
        host: &str,
        port: u16,
        message: &str,
        model: &str,
        tx: mpsc::Sender<StreamingToken>,
    ) -> anyhow::Result<()> {
        use futures::StreamExt;

        let url = format!("http://{}:{}/api/generate", host, port);

        let request = serde_json::json!({
            "model": model,
            "prompt": message,
            "stream": true,
        });

        let response = self.http_client.post(&url).json(&request).send().await?;

        let mut stream = response.bytes_stream();

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
                                    if let Some(token) =
                                        data.get("response").and_then(|r| r.as_str())
                                    {
                                        full_response.push_str(token);
                                        let _ =
                                            tx.send(StreamingToken::Token(token.to_string())).await;
                                    }
                                    if data.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
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
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Send a message and wait for complete response (non-streaming)
    /// Used for quick queries like generating goodbye messages
    pub async fn send_message_sync(&self, message: &str, model: &str) -> anyhow::Result<String> {
        match &self.connection {
            BackendConnection::DirectOllama { host, port } => {
                let url = format!("http://{}:{}/api/generate", host, port);

                let request = serde_json::json!({
                    "model": model,
                    "prompt": message,
                    "stream": false,
                });

                let response = self.http_client.post(&url).json(&request).send().await?;

                let data: serde_json::Value = response.json().await?;
                let response_text = data
                    .get("response")
                    .and_then(|r| r.as_str())
                    .unwrap_or("")
                    .to_string();

                Ok(response_text)
            }
            BackendConnection::CoreServer { .. } => {
                anyhow::bail!("Server mode not yet implemented")
            }
        }
    }

    /// Check if backend is healthy
    pub async fn health_check(&self) -> bool {
        let url = match &self.connection {
            BackendConnection::DirectOllama { host, port } => {
                format!("http://{}:{}/api/tags", host, port)
            }
            BackendConnection::CoreServer { host, port } => {
                format!("http://{}:{}/", host, port)
            }
        };

        self.http_client.get(&url).send().await.is_ok()
    }
}

impl Default for BackendClient {
    fn default() -> Self {
        Self::new(BackendConnection::default())
    }
}
