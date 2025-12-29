//! Message Types

use serde::{Deserialize, Serialize};

/// User message to backend
#[derive(Clone, Debug, Serialize)]
pub struct UserMessage {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Streaming token events
#[derive(Clone, Debug)]
pub enum StreamingToken {
    /// A token from the response
    Token(String),
    /// Response completed
    Complete { message: String },
    /// Error occurred
    Error(String),
}

/// Full response from backend
#[derive(Clone, Debug, Deserialize)]
pub struct BackendResponse {
    pub message: String,
    pub session_id: String,
    #[serde(default)]
    pub tokens_used: i32,
}
