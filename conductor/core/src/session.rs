//! Session Management
//!
//! Manages conversation sessions between users and the AI.
//! Sessions track message history, context, and metadata.
//!
//! # Design Philosophy
//!
//! A session represents an ongoing conversation. The Conductor maintains
//! session state so UI surfaces can connect, disconnect, and reconnect
//! without losing context. Sessions can be persisted and resumed.

use serde::{Deserialize, Serialize};

use crate::messages::{MessageId, MessageRole, SessionId};

/// A message in the conversation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// Unique message ID
    pub id: MessageId,
    /// Who sent this message
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// When the message was created (Unix timestamp ms)
    pub timestamp: u64,
    /// Whether the message is still being streamed
    pub streaming: bool,
}

impl ConversationMessage {
    /// Create a new message
    pub fn new(role: MessageRole, content: String) -> Self {
        Self {
            id: MessageId::new(),
            role,
            content,
            timestamp: now_ms(),
            streaming: false,
        }
    }

    /// Create a new streaming message (content will be updated)
    pub fn streaming(role: MessageRole) -> Self {
        Self {
            id: MessageId::new(),
            role,
            content: String::new(),
            timestamp: now_ms(),
            streaming: true,
        }
    }

    /// Append content to a streaming message
    pub fn append(&mut self, text: &str) {
        self.content.push_str(text);
    }

    /// Mark streaming as complete
    pub fn complete(&mut self) {
        self.streaming = false;
    }
}

/// Session state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    /// Session is active and ready
    Active,
    /// Session is processing a query
    Busy,
    /// Session is paused (no active connection)
    Paused,
    /// Session has ended
    Ended,
}

/// Session metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// When the session was created (Unix timestamp ms)
    pub created_at: u64,
    /// When the session was last active (Unix timestamp ms)
    pub last_active_at: u64,
    /// Total messages exchanged
    pub message_count: u32,
    /// Total tokens used (if available)
    pub tokens_used: Option<u64>,
    /// Model used for this session
    pub model: String,
    /// Custom session title (optional)
    pub title: Option<String>,
}

impl SessionMetadata {
    /// Create new metadata
    pub fn new(model: String) -> Self {
        let now = now_ms();
        Self {
            created_at: now,
            last_active_at: now,
            message_count: 0,
            tokens_used: None,
            model,
            title: None,
        }
    }

    /// Update last active timestamp
    pub fn touch(&mut self) {
        self.last_active_at = now_ms();
    }

    /// Increment message count
    pub fn add_message(&mut self) {
        self.message_count += 1;
        self.touch();
    }

    /// Add token usage
    pub fn add_tokens(&mut self, count: u64) {
        self.tokens_used = Some(self.tokens_used.unwrap_or(0) + count);
    }
}

/// A conversation session
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    /// Unique session ID
    pub id: SessionId,
    /// Session state
    pub state: SessionState,
    /// Session metadata
    pub metadata: SessionMetadata,
    /// Conversation history
    pub messages: Vec<ConversationMessage>,
    /// Currently streaming message (if any)
    current_streaming_id: Option<MessageId>,
    /// Maximum number of messages to keep (0 = unlimited)
    #[serde(default)]
    max_messages: usize,
    /// Maximum total content bytes (0 = unlimited)
    #[serde(default)]
    max_content_bytes: usize,
    /// Current total content bytes
    #[serde(default)]
    current_content_bytes: usize,
}

impl Session {
    /// Create a new session
    pub fn new(model: String) -> Self {
        Self {
            id: SessionId::new(),
            state: SessionState::Active,
            metadata: SessionMetadata::new(model),
            messages: Vec::new(),
            current_streaming_id: None,
            max_messages: 0,       // 0 = unlimited (backwards compatible)
            max_content_bytes: 0,  // 0 = unlimited (backwards compatible)
            current_content_bytes: 0,
        }
    }

    /// Create a new session with limits
    pub fn new_with_limits(model: String, max_messages: usize, max_content_bytes: usize) -> Self {
        Self {
            id: SessionId::new(),
            state: SessionState::Active,
            metadata: SessionMetadata::new(model),
            messages: Vec::new(),
            current_streaming_id: None,
            max_messages,
            max_content_bytes,
            current_content_bytes: 0,
        }
    }

    /// Create a session with a specific ID
    pub fn with_id(id: SessionId, model: String) -> Self {
        Self {
            id,
            state: SessionState::Active,
            metadata: SessionMetadata::new(model),
            messages: Vec::new(),
            current_streaming_id: None,
            max_messages: 0,
            max_content_bytes: 0,
            current_content_bytes: 0,
        }
    }

    /// Add a user message
    pub fn add_user_message(&mut self, content: String) -> MessageId {
        let content_len = content.len();
        let msg = ConversationMessage::new(MessageRole::User, content);
        let id = msg.id.clone();
        self.messages.push(msg);
        self.current_content_bytes += content_len;
        self.metadata.add_message();
        self.prune_if_needed();
        id
    }

    /// Add a system message
    pub fn add_system_message(&mut self, content: String) -> MessageId {
        let content_len = content.len();
        let msg = ConversationMessage::new(MessageRole::System, content);
        let id = msg.id.clone();
        self.messages.push(msg);
        self.current_content_bytes += content_len;
        self.metadata.add_message();
        self.prune_if_needed();
        id
    }

    /// Start a streaming assistant response
    pub fn start_assistant_response(&mut self) -> MessageId {
        let msg = ConversationMessage::streaming(MessageRole::Assistant);
        let id = msg.id.clone();
        self.current_streaming_id = Some(id.clone());
        self.messages.push(msg);
        self.state = SessionState::Busy;
        id
    }

    /// Append to the current streaming response
    pub fn append_streaming(&mut self, text: &str) -> Option<&ConversationMessage> {
        if let Some(ref streaming_id) = self.current_streaming_id {
            if let Some(msg) = self.messages.iter_mut().find(|m| &m.id == streaming_id) {
                msg.append(text);
                self.current_content_bytes += text.len();
                return Some(msg);
            }
        }
        None
    }

    /// Complete the current streaming response
    pub fn complete_streaming(&mut self) -> Option<&ConversationMessage> {
        let streaming_id = self.current_streaming_id.take()?;

        // Find and complete the message
        let msg_idx = self.messages.iter().position(|m| m.id == streaming_id)?;
        self.messages[msg_idx].complete();
        self.metadata.add_message();
        self.state = SessionState::Active;

        // Prune after completing (this may invalidate indices)
        self.prune_if_needed();

        // Find the message again after pruning
        self.messages.iter().find(|m| m.id == streaming_id)
    }

    /// Cancel the current streaming response
    pub fn cancel_streaming(&mut self) {
        if let Some(streaming_id) = self.current_streaming_id.take() {
            // Remove the incomplete message
            self.messages.retain(|m| m.id != streaming_id);
            self.state = SessionState::Active;
        }
    }

    /// Get the current streaming message ID
    pub fn streaming_message_id(&self) -> Option<&MessageId> {
        self.current_streaming_id.as_ref()
    }

    /// Check if currently streaming
    pub fn is_streaming(&self) -> bool {
        self.current_streaming_id.is_some()
    }

    /// Get message by ID
    pub fn get_message(&self, id: &MessageId) -> Option<&ConversationMessage> {
        self.messages.iter().find(|m| &m.id == id)
    }

    /// Get the last N messages for context
    pub fn recent_messages(&self, count: usize) -> &[ConversationMessage] {
        let start = self.messages.len().saturating_sub(count);
        &self.messages[start..]
    }

    /// Get all messages
    pub fn all_messages(&self) -> &[ConversationMessage] {
        &self.messages
    }

    /// Build context for LLM (message history as formatted text)
    pub fn build_context(&self, max_messages: usize) -> String {
        let recent = self.recent_messages(max_messages);
        let mut context = String::new();

        for msg in recent {
            let role = match msg.role {
                MessageRole::User => "User",
                MessageRole::Assistant => "Assistant",
                MessageRole::System => "System",
            };
            context.push_str(&format!("{}: {}\n\n", role, msg.content));
        }

        context
    }

    /// Pause the session
    pub fn pause(&mut self) {
        if self.state == SessionState::Active {
            self.state = SessionState::Paused;
        }
    }

    /// Resume the session
    pub fn resume(&mut self) {
        if self.state == SessionState::Paused {
            self.state = SessionState::Active;
            self.metadata.touch();
        }
    }

    /// End the session
    pub fn end(&mut self) {
        self.cancel_streaming();
        self.state = SessionState::Ended;
    }

    /// Clear message history (keeps metadata)
    pub fn clear_history(&mut self) {
        self.messages.clear();
        self.current_streaming_id = None;
        self.current_content_bytes = 0;
        self.state = SessionState::Active;
    }

    /// Prune messages if limits are exceeded
    ///
    /// This is called automatically after adding messages.
    /// Removes oldest messages (except the current streaming message) until within limits.
    fn prune_if_needed(&mut self) {
        // Don't prune if limits are disabled (0 = unlimited)
        if self.max_messages == 0 && self.max_content_bytes == 0 {
            return;
        }

        // Prune by message count
        if self.max_messages > 0 && self.messages.len() > self.max_messages {
            let to_remove = self.messages.len() - self.max_messages;
            self.remove_oldest_messages(to_remove);
        }

        // Prune by content bytes
        if self.max_content_bytes > 0 && self.current_content_bytes > self.max_content_bytes {
            self.prune_by_bytes();
        }
    }

    /// Remove the N oldest messages (except current streaming)
    fn remove_oldest_messages(&mut self, count: usize) {
        let mut removed = 0;
        let streaming_id = self.current_streaming_id.clone();

        self.messages.retain(|msg| {
            // Never remove the current streaming message
            if Some(&msg.id) == streaming_id.as_ref() {
                return true;
            }

            if removed < count {
                self.current_content_bytes = self.current_content_bytes.saturating_sub(msg.content.len());
                removed += 1;
                false
            } else {
                true
            }
        });

        tracing::debug!(
            removed = removed,
            remaining = self.messages.len(),
            "Pruned session messages by count"
        );
    }

    /// Prune messages until content bytes is under limit
    fn prune_by_bytes(&mut self) {
        let streaming_id = self.current_streaming_id.clone();

        while self.current_content_bytes > self.max_content_bytes && !self.messages.is_empty() {
            // Find the oldest message that's not streaming
            let oldest_idx = self.messages.iter().position(|msg| {
                Some(&msg.id) != streaming_id.as_ref()
            });

            if let Some(idx) = oldest_idx {
                let removed = self.messages.remove(idx);
                self.current_content_bytes = self.current_content_bytes.saturating_sub(removed.content.len());
            } else {
                // Only the streaming message remains, can't prune further
                break;
            }
        }

        tracing::debug!(
            remaining = self.messages.len(),
            bytes = self.current_content_bytes,
            "Pruned session messages by bytes"
        );
    }

    /// Manually trigger pruning (for external use)
    pub fn prune_messages(&mut self) {
        self.prune_if_needed();
    }

    /// Get current content size in bytes
    pub fn content_bytes(&self) -> usize {
        self.current_content_bytes
    }

    /// Get current message count
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get configured limits
    pub fn limits(&self) -> (usize, usize) {
        (self.max_messages, self.max_content_bytes)
    }
}

/// Get current timestamp in milliseconds
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("test-model".to_string());
        assert_eq!(session.state, SessionState::Active);
        assert!(session.messages.is_empty());
        assert_eq!(session.metadata.model, "test-model");
    }

    #[test]
    fn test_add_messages() {
        let mut session = Session::new("test".to_string());

        let user_id = session.add_user_message("Hello".to_string());
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.metadata.message_count, 1);

        let msg = session.get_message(&user_id).unwrap();
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_streaming_response() {
        let mut session = Session::new("test".to_string());

        let stream_id = session.start_assistant_response();
        assert!(session.is_streaming());
        assert_eq!(session.state, SessionState::Busy);

        session.append_streaming("Hello ");
        session.append_streaming("world!");

        let msg = session.get_message(&stream_id).unwrap();
        assert!(msg.streaming);
        assert_eq!(msg.content, "Hello world!");

        session.complete_streaming();
        assert!(!session.is_streaming());
        assert_eq!(session.state, SessionState::Active);

        let msg = session.get_message(&stream_id).unwrap();
        assert!(!msg.streaming);
    }

    #[test]
    fn test_build_context() {
        let mut session = Session::new("test".to_string());

        session.add_user_message("First question".to_string());
        session.add_user_message("Second question".to_string());
        session.add_user_message("Third question".to_string());

        let context = session.build_context(2);
        assert!(!context.contains("First"));
        assert!(context.contains("Second"));
        assert!(context.contains("Third"));
    }

    #[test]
    fn test_session_state_transitions() {
        let mut session = Session::new("test".to_string());
        assert_eq!(session.state, SessionState::Active);

        session.pause();
        assert_eq!(session.state, SessionState::Paused);

        session.resume();
        assert_eq!(session.state, SessionState::Active);

        session.end();
        assert_eq!(session.state, SessionState::Ended);
    }

    #[test]
    fn test_session_with_limits() {
        let mut session = Session::new_with_limits("test".to_string(), 3, 1000);
        assert_eq!(session.limits(), (3, 1000));

        // Add 5 messages
        session.add_user_message("Message 1".to_string());
        session.add_user_message("Message 2".to_string());
        session.add_user_message("Message 3".to_string());
        session.add_user_message("Message 4".to_string());
        session.add_user_message("Message 5".to_string());

        // Should have pruned to 3 messages
        assert_eq!(session.message_count(), 3);

        // Oldest messages should be removed
        let messages: Vec<_> = session.all_messages().iter().map(|m| m.content.as_str()).collect();
        assert!(!messages.contains(&"Message 1"));
        assert!(!messages.contains(&"Message 2"));
        assert!(messages.contains(&"Message 3"));
        assert!(messages.contains(&"Message 4"));
        assert!(messages.contains(&"Message 5"));
    }

    #[test]
    fn test_session_prune_by_bytes() {
        // 50 bytes limit
        let mut session = Session::new_with_limits("test".to_string(), 0, 50);

        // Add messages that exceed the limit
        session.add_user_message("AAAAAAAAAA".to_string()); // 10 bytes
        session.add_user_message("BBBBBBBBBB".to_string()); // 10 bytes
        session.add_user_message("CCCCCCCCCC".to_string()); // 10 bytes
        session.add_user_message("DDDDDDDDDD".to_string()); // 10 bytes
        session.add_user_message("EEEEEEEEEE".to_string()); // 10 bytes
        session.add_user_message("FFFFFFFFFF".to_string()); // 10 bytes - total 60, should prune

        // Should be under 50 bytes
        assert!(session.content_bytes() <= 50);
    }

    #[test]
    fn test_session_content_bytes_tracking() {
        let mut session = Session::new_with_limits("test".to_string(), 1000, 10000);

        session.add_user_message("Hello".to_string()); // 5 bytes
        assert_eq!(session.content_bytes(), 5);

        session.add_user_message("World!".to_string()); // 6 bytes
        assert_eq!(session.content_bytes(), 11);

        session.clear_history();
        assert_eq!(session.content_bytes(), 0);
    }

    #[test]
    fn test_session_streaming_content_tracking() {
        let mut session = Session::new_with_limits("test".to_string(), 1000, 10000);

        session.start_assistant_response();
        session.append_streaming("Hello ");
        session.append_streaming("World!");
        session.complete_streaming();

        assert_eq!(session.content_bytes(), 12);
    }
}
