//! Stream Manager Implementation
//!
//! Manages multiple concurrent streaming responses for parallel conversations.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use crate::backend::StreamingToken;
use crate::conversation::ConversationId;
use crate::messages::MessageId;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for the stream manager
#[derive(Clone, Debug)]
pub struct StreamManagerConfig {
    /// Maximum tokens to buffer per conversation before applying overflow policy
    pub max_buffer_tokens: usize,
    /// How to handle buffer overflow
    pub overflow_policy: BufferOverflowPolicy,
    /// Minimum time between UI updates (for throttling)
    /// Default: ~33ms for 30 FPS
    pub ui_throttle_duration: Duration,
    /// Maximum number of concurrent streams
    pub max_concurrent_streams: usize,
}

impl Default for StreamManagerConfig {
    fn default() -> Self {
        Self {
            max_buffer_tokens: 1000,
            overflow_policy: BufferOverflowPolicy::DropOldest,
            ui_throttle_duration: Duration::from_millis(33), // ~30 FPS
            max_concurrent_streams: 16,
        }
    }
}

/// Policy for handling buffer overflow
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BufferOverflowPolicy {
    /// Drop oldest tokens when buffer is full (default, preserves recent content)
    #[default]
    DropOldest,
    /// Drop new tokens when buffer is full (preserves initial content)
    DropNewest,
    /// Concatenate tokens to reduce count (joins tokens without delimiter)
    Concatenate,
}

// ============================================================================
// Stream Events
// ============================================================================

/// An event from a conversation stream
#[derive(Clone, Debug)]
pub struct StreamEvent {
    /// The conversation this event belongs to
    pub conversation_id: ConversationId,
    /// The message being streamed
    pub message_id: MessageId,
    /// The event kind
    pub kind: StreamEventKind,
    /// Timestamp when this event was created
    pub timestamp: Instant,
}

/// Kind of stream event
#[derive(Clone, Debug)]
pub enum StreamEventKind {
    /// New tokens arrived
    Tokens {
        /// The tokens (may be multiple if batched)
        tokens: Vec<String>,
        /// Total token count for this stream so far
        total_count: u32,
    },
    /// Stream completed successfully
    Complete {
        /// The final complete message
        message: String,
        /// Total tokens received
        token_count: u32,
        /// Duration of the stream
        duration: Duration,
    },
    /// Stream encountered an error
    Error {
        /// Error description
        error: String,
        /// Partial content received before error
        partial_content: String,
    },
    /// Buffer overflow occurred (for monitoring)
    BufferOverflow {
        /// Number of tokens dropped/merged
        affected_tokens: usize,
        /// Policy that was applied
        policy: BufferOverflowPolicy,
    },
}

// ============================================================================
// Conversation Stream
// ============================================================================

/// Statistics for a conversation stream
#[derive(Clone, Debug, Default)]
pub struct StreamStats {
    /// Total tokens received
    pub tokens_received: u32,
    /// Tokens dropped due to overflow
    pub tokens_dropped: u32,
    /// When streaming started
    pub started_at: Option<Instant>,
    /// Last token received timestamp
    pub last_token_at: Option<Instant>,
    /// Last UI update timestamp (for throttling)
    pub last_ui_update: Option<Instant>,
}

/// A stream wrapper for a single conversation
///
/// Wraps a tokio mpsc receiver with conversation context, buffering,
/// and rate limiting for UI updates.
pub struct ConversationStream {
    /// The conversation this stream belongs to
    conversation_id: ConversationId,
    /// The message being streamed
    message_id: MessageId,
    /// The underlying token receiver
    receiver: mpsc::Receiver<StreamingToken>,
    /// Token buffer (accumulated content)
    buffer: Vec<String>,
    /// Accumulated full content
    content: String,
    /// Stream statistics
    stats: StreamStats,
    /// Whether the stream has completed
    completed: bool,
    /// Configuration reference
    config: StreamManagerConfig,
    /// Pending tokens for UI (between throttle windows)
    pending_ui_tokens: Vec<String>,
}

impl ConversationStream {
    /// Create a new conversation stream
    #[must_use]
    pub fn new(
        conversation_id: ConversationId,
        message_id: MessageId,
        receiver: mpsc::Receiver<StreamingToken>,
        config: StreamManagerConfig,
    ) -> Self {
        Self {
            conversation_id,
            message_id,
            receiver,
            buffer: Vec::with_capacity(config.max_buffer_tokens),
            content: String::new(),
            stats: StreamStats {
                started_at: Some(Instant::now()),
                ..Default::default()
            },
            completed: false,
            config,
            pending_ui_tokens: Vec::new(),
        }
    }

    /// Get the conversation ID
    #[must_use]
    pub fn conversation_id(&self) -> ConversationId {
        self.conversation_id
    }

    /// Get the message ID
    #[must_use]
    pub fn message_id(&self) -> MessageId {
        self.message_id.clone()
    }

    /// Check if the stream has completed
    #[must_use]
    pub fn is_completed(&self) -> bool {
        self.completed
    }

    /// Get the current accumulated content
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get stream statistics
    #[must_use]
    pub fn stats(&self) -> &StreamStats {
        &self.stats
    }

    /// Get the token count
    #[must_use]
    pub fn token_count(&self) -> u32 {
        self.stats.tokens_received
    }

    /// Check if UI update is due (based on throttle)
    fn should_update_ui(&self) -> bool {
        match self.stats.last_ui_update {
            Some(last) => last.elapsed() >= self.config.ui_throttle_duration,
            None => true, // First update always allowed
        }
    }

    /// Mark UI as updated
    fn mark_ui_updated(&mut self) {
        self.stats.last_ui_update = Some(Instant::now());
    }

    /// Apply buffer overflow policy
    fn apply_overflow_policy(&mut self) -> Option<StreamEvent> {
        if self.buffer.len() <= self.config.max_buffer_tokens {
            return None;
        }

        let excess = self.buffer.len() - self.config.max_buffer_tokens;
        let policy = self.config.overflow_policy;

        match policy {
            BufferOverflowPolicy::DropOldest => {
                // Remove oldest tokens
                self.buffer.drain(0..excess);
                self.stats.tokens_dropped += excess as u32;
            }
            BufferOverflowPolicy::DropNewest => {
                // Remove newest tokens
                self.buffer.truncate(self.config.max_buffer_tokens);
                self.stats.tokens_dropped += excess as u32;
            }
            BufferOverflowPolicy::Concatenate => {
                // Merge tokens to reduce count
                let merge_count = excess.min(self.buffer.len() / 2);
                let mut merged = String::new();
                for token in self.buffer.drain(0..merge_count) {
                    merged.push_str(&token);
                }
                self.buffer.insert(0, merged);
            }
        }

        Some(StreamEvent {
            conversation_id: self.conversation_id,
            message_id: self.message_id.clone(),
            kind: StreamEventKind::BufferOverflow {
                affected_tokens: excess,
                policy,
            },
            timestamp: Instant::now(),
        })
    }

    /// Poll the stream for new tokens (non-blocking)
    ///
    /// Returns a vector of events. May return multiple events if tokens
    /// arrived and the stream completed in the same poll.
    pub fn poll(&mut self) -> Vec<StreamEvent> {
        if self.completed {
            return Vec::new();
        }

        let mut events = Vec::new();
        let mut new_tokens = Vec::new();

        // Drain all available tokens (non-blocking)
        loop {
            match self.receiver.try_recv() {
                Ok(token) => match token {
                    StreamingToken::Token(text) => {
                        self.stats.tokens_received += 1;
                        self.stats.last_token_at = Some(Instant::now());
                        self.content.push_str(&text);
                        self.buffer.push(text.clone());
                        new_tokens.push(text);

                        // Check for buffer overflow
                        if let Some(overflow_event) = self.apply_overflow_policy() {
                            events.push(overflow_event);
                        }
                    }
                    StreamingToken::Complete { message } => {
                        self.completed = true;
                        self.content = message.clone();

                        let duration = self
                            .stats
                            .started_at
                            .map(|s| s.elapsed())
                            .unwrap_or_default();

                        events.push(StreamEvent {
                            conversation_id: self.conversation_id,
                            message_id: self.message_id.clone(),
                            kind: StreamEventKind::Complete {
                                message,
                                token_count: self.stats.tokens_received,
                                duration,
                            },
                            timestamp: Instant::now(),
                        });
                        break;
                    }
                    StreamingToken::Error(error) => {
                        self.completed = true;
                        events.push(StreamEvent {
                            conversation_id: self.conversation_id,
                            message_id: self.message_id.clone(),
                            kind: StreamEventKind::Error {
                                error,
                                partial_content: self.content.clone(),
                            },
                            timestamp: Instant::now(),
                        });
                        break;
                    }
                },
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    // Channel closed without proper completion
                    if !self.completed {
                        self.completed = true;
                        events.push(StreamEvent {
                            conversation_id: self.conversation_id,
                            message_id: self.message_id.clone(),
                            kind: StreamEventKind::Error {
                                error: "Stream disconnected unexpectedly".to_string(),
                                partial_content: self.content.clone(),
                            },
                            timestamp: Instant::now(),
                        });
                    }
                    break;
                }
            }
        }

        // Add pending tokens from previous throttle window
        self.pending_ui_tokens.append(&mut new_tokens);

        // Generate token event if we should update UI
        if !self.pending_ui_tokens.is_empty() && self.should_update_ui() {
            let tokens = std::mem::take(&mut self.pending_ui_tokens);
            self.mark_ui_updated();

            // Insert at the beginning so token events come before complete/error
            events.insert(
                0,
                StreamEvent {
                    conversation_id: self.conversation_id,
                    message_id: self.message_id.clone(),
                    kind: StreamEventKind::Tokens {
                        tokens,
                        total_count: self.stats.tokens_received,
                    },
                    timestamp: Instant::now(),
                },
            );
        }

        events
    }
}

// ============================================================================
// Stream Manager
// ============================================================================

/// Manages multiple concurrent conversation streams
///
/// Provides non-blocking parallel polling of all active streams,
/// with buffer management and UI throttling.
pub struct StreamManager {
    /// Active streams by conversation ID
    streams: HashMap<ConversationId, ConversationStream>,
    /// Configuration
    config: StreamManagerConfig,
    /// Global statistics
    total_streams_created: u64,
    total_tokens_processed: u64,
}

impl Default for StreamManager {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamManager {
    /// Create a new stream manager with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(StreamManagerConfig::default())
    }

    /// Create a new stream manager with custom configuration
    #[must_use]
    pub fn with_config(config: StreamManagerConfig) -> Self {
        Self {
            streams: HashMap::new(),
            config,
            total_streams_created: 0,
            total_tokens_processed: 0,
        }
    }

    /// Get the current configuration
    #[must_use]
    pub fn config(&self) -> &StreamManagerConfig {
        &self.config
    }

    /// Get the number of active streams
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.streams.len()
    }

    /// Check if a conversation has an active stream
    #[must_use]
    pub fn has_stream(&self, conversation_id: ConversationId) -> bool {
        self.streams.contains_key(&conversation_id)
    }

    /// Get a reference to a conversation stream
    #[must_use]
    pub fn get(&self, conversation_id: ConversationId) -> Option<&ConversationStream> {
        self.streams.get(&conversation_id)
    }

    /// Get a mutable reference to a conversation stream
    pub fn get_mut(&mut self, conversation_id: ConversationId) -> Option<&mut ConversationStream> {
        self.streams.get_mut(&conversation_id)
    }

    /// Register a new stream for a conversation
    ///
    /// Returns `Err` if max concurrent streams reached or conversation already has a stream.
    pub fn register(
        &mut self,
        conversation_id: ConversationId,
        message_id: MessageId,
        receiver: mpsc::Receiver<StreamingToken>,
    ) -> Result<(), StreamRegisterError> {
        // Check limits
        if self.streams.len() >= self.config.max_concurrent_streams {
            return Err(StreamRegisterError::MaxStreamsReached);
        }

        if self.streams.contains_key(&conversation_id) {
            return Err(StreamRegisterError::StreamAlreadyExists);
        }

        let stream =
            ConversationStream::new(conversation_id, message_id, receiver, self.config.clone());

        self.streams.insert(conversation_id, stream);
        self.total_streams_created += 1;

        Ok(())
    }

    /// Unregister a stream (removes it from the manager)
    ///
    /// Returns the stream if it existed.
    pub fn unregister(&mut self, conversation_id: ConversationId) -> Option<ConversationStream> {
        self.streams.remove(&conversation_id)
    }

    /// Poll all active streams (non-blocking)
    ///
    /// Returns all events from all streams. Events are grouped by conversation
    /// but interleaved based on when they were received.
    pub fn poll_all(&mut self) -> Vec<StreamEvent> {
        let mut all_events = Vec::new();
        let mut completed_ids = Vec::new();

        for (conv_id, stream) in &mut self.streams {
            let events = stream.poll();

            for event in &events {
                if let StreamEventKind::Tokens { tokens, .. } = &event.kind {
                    self.total_tokens_processed += tokens.len() as u64;
                }

                if matches!(
                    event.kind,
                    StreamEventKind::Complete { .. } | StreamEventKind::Error { .. }
                ) {
                    completed_ids.push(*conv_id);
                }
            }

            all_events.extend(events);
        }

        // Remove completed streams
        for id in completed_ids {
            self.streams.remove(&id);
        }

        // Sort by timestamp for consistent ordering
        all_events.sort_by_key(|e| e.timestamp);

        all_events
    }

    /// Poll a specific conversation stream
    pub fn poll_one(&mut self, conversation_id: ConversationId) -> Vec<StreamEvent> {
        let events = if let Some(stream) = self.streams.get_mut(&conversation_id) {
            let events = stream.poll();

            for event in &events {
                if let StreamEventKind::Tokens { tokens, .. } = &event.kind {
                    self.total_tokens_processed += tokens.len() as u64;
                }
            }

            events
        } else {
            Vec::new()
        };

        // Check if stream completed
        if let Some(stream) = self.streams.get(&conversation_id) {
            if stream.is_completed() {
                self.streams.remove(&conversation_id);
            }
        }

        events
    }

    /// Get IDs of all active streams
    #[must_use]
    pub fn active_stream_ids(&self) -> Vec<ConversationId> {
        self.streams.keys().copied().collect()
    }

    /// Get statistics for a specific stream
    #[must_use]
    pub fn stream_stats(&self, conversation_id: ConversationId) -> Option<&StreamStats> {
        self.streams
            .get(&conversation_id)
            .map(ConversationStream::stats)
    }

    /// Get total number of streams created (lifetime)
    #[must_use]
    pub fn total_streams_created(&self) -> u64 {
        self.total_streams_created
    }

    /// Get total tokens processed (lifetime)
    #[must_use]
    pub fn total_tokens_processed(&self) -> u64 {
        self.total_tokens_processed
    }

    /// Clear all streams (cancels any in-progress streams)
    pub fn clear(&mut self) {
        self.streams.clear();
    }

    /// Check if any streams are active
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.streams.is_empty()
    }
}

/// Error when registering a stream
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamRegisterError {
    /// Maximum concurrent streams reached
    MaxStreamsReached,
    /// Conversation already has an active stream
    StreamAlreadyExists,
}

impl std::fmt::Display for StreamRegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxStreamsReached => write!(f, "maximum concurrent streams reached"),
            Self::StreamAlreadyExists => write!(f, "conversation already has an active stream"),
        }
    }
}

impl std::error::Error for StreamRegisterError {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn create_test_receiver(tokens: Vec<StreamingToken>) -> mpsc::Receiver<StreamingToken> {
        let (tx, rx) = mpsc::channel(100);
        tokio::spawn(async move {
            for token in tokens {
                if tx.send(token).await.is_err() {
                    break;
                }
            }
        });
        rx
    }

    #[test]
    fn test_config_default() {
        let config = StreamManagerConfig::default();
        assert_eq!(config.max_buffer_tokens, 1000);
        assert_eq!(config.overflow_policy, BufferOverflowPolicy::DropOldest);
        assert_eq!(config.ui_throttle_duration, Duration::from_millis(33));
        assert_eq!(config.max_concurrent_streams, 16);
    }

    #[test]
    fn test_stream_manager_creation() {
        let manager = StreamManager::new();
        assert_eq!(manager.active_count(), 0);
        assert!(manager.is_empty());
    }

    #[tokio::test]
    async fn test_stream_registration() {
        let mut manager = StreamManager::new();
        let conv_id = ConversationId::new();
        let msg_id = MessageId::new();
        let (_, rx) = mpsc::channel::<StreamingToken>(10);

        let result = manager.register(conv_id, msg_id, rx);
        assert!(result.is_ok());
        assert_eq!(manager.active_count(), 1);
        assert!(manager.has_stream(conv_id));
    }

    #[tokio::test]
    async fn test_duplicate_stream_registration() {
        let mut manager = StreamManager::new();
        let conv_id = ConversationId::new();
        let msg_id = MessageId::new();
        let (_, rx1) = mpsc::channel::<StreamingToken>(10);
        let (_, rx2) = mpsc::channel::<StreamingToken>(10);

        manager.register(conv_id, msg_id.clone(), rx1).unwrap();
        let result = manager.register(conv_id, msg_id, rx2);
        assert_eq!(result, Err(StreamRegisterError::StreamAlreadyExists));
    }

    #[tokio::test]
    async fn test_max_streams_limit() {
        let config = StreamManagerConfig {
            max_concurrent_streams: 2,
            ..Default::default()
        };
        let mut manager = StreamManager::with_config(config);

        for i in 0..2 {
            let (_, rx) = mpsc::channel::<StreamingToken>(10);
            let conv_id = ConversationId::new();
            let msg_id = MessageId::new();
            manager.register(conv_id, msg_id, rx).unwrap();
            assert_eq!(manager.active_count(), i + 1);
        }

        let (_, rx) = mpsc::channel::<StreamingToken>(10);
        let conv_id = ConversationId::new();
        let msg_id = MessageId::new();
        let result = manager.register(conv_id, msg_id, rx);
        assert_eq!(result, Err(StreamRegisterError::MaxStreamsReached));
    }

    #[tokio::test]
    async fn test_stream_tokens() {
        let mut manager = StreamManager::with_config(StreamManagerConfig {
            ui_throttle_duration: Duration::ZERO, // Disable throttling for test
            ..Default::default()
        });

        let conv_id = ConversationId::new();
        let msg_id = MessageId::new();

        let tokens = vec![
            StreamingToken::Token("Hello ".to_string()),
            StreamingToken::Token("world!".to_string()),
            StreamingToken::Complete {
                message: "Hello world!".to_string(),
            },
        ];

        let rx = create_test_receiver(tokens);
        manager.register(conv_id, msg_id, rx).unwrap();

        // Give the async task time to send tokens
        tokio::time::sleep(Duration::from_millis(50)).await;

        let events = manager.poll_all();

        // Should have token event(s) and complete event
        assert!(!events.is_empty());

        // Find the complete event
        let complete_event = events
            .iter()
            .find(|e| matches!(e.kind, StreamEventKind::Complete { .. }));
        assert!(complete_event.is_some());

        if let Some(event) = complete_event {
            if let StreamEventKind::Complete {
                message,
                token_count,
                ..
            } = &event.kind
            {
                assert_eq!(message, "Hello world!");
                assert_eq!(*token_count, 2);
            }
        }

        // Stream should be removed after completion
        assert_eq!(manager.active_count(), 0);
    }

    #[tokio::test]
    async fn test_stream_error() {
        let mut manager = StreamManager::with_config(StreamManagerConfig {
            ui_throttle_duration: Duration::ZERO,
            ..Default::default()
        });

        let conv_id = ConversationId::new();
        let msg_id = MessageId::new();

        let tokens = vec![
            StreamingToken::Token("Partial ".to_string()),
            StreamingToken::Error("Connection lost".to_string()),
        ];

        let rx = create_test_receiver(tokens);
        manager.register(conv_id, msg_id, rx).unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let events = manager.poll_all();

        let error_event = events
            .iter()
            .find(|e| matches!(e.kind, StreamEventKind::Error { .. }));
        assert!(error_event.is_some());

        if let Some(event) = error_event {
            if let StreamEventKind::Error {
                error,
                partial_content,
            } = &event.kind
            {
                assert_eq!(error, "Connection lost");
                assert_eq!(partial_content, "Partial ");
            }
        }

        assert_eq!(manager.active_count(), 0);
    }

    #[tokio::test]
    async fn test_multiple_streams() {
        let mut manager = StreamManager::with_config(StreamManagerConfig {
            ui_throttle_duration: Duration::ZERO,
            ..Default::default()
        });

        let conv_id_1 = ConversationId::new();
        let conv_id_2 = ConversationId::new();
        let msg_id_1 = MessageId::new();
        let msg_id_2 = MessageId::new();

        let tokens_1 = vec![
            StreamingToken::Token("Stream 1".to_string()),
            StreamingToken::Complete {
                message: "Stream 1".to_string(),
            },
        ];

        let tokens_2 = vec![
            StreamingToken::Token("Stream 2".to_string()),
            StreamingToken::Complete {
                message: "Stream 2".to_string(),
            },
        ];

        let rx1 = create_test_receiver(tokens_1);
        let rx2 = create_test_receiver(tokens_2);

        manager.register(conv_id_1, msg_id_1, rx1).unwrap();
        manager.register(conv_id_2, msg_id_2, rx2).unwrap();

        assert_eq!(manager.active_count(), 2);

        tokio::time::sleep(Duration::from_millis(50)).await;

        let events = manager.poll_all();

        // Should have events from both streams
        let conv_1_events: Vec<_> = events
            .iter()
            .filter(|e| e.conversation_id == conv_id_1)
            .collect();
        let conv_2_events: Vec<_> = events
            .iter()
            .filter(|e| e.conversation_id == conv_id_2)
            .collect();

        assert!(!conv_1_events.is_empty());
        assert!(!conv_2_events.is_empty());

        // Both streams should be removed after completion
        assert_eq!(manager.active_count(), 0);
    }

    #[tokio::test]
    async fn test_buffer_overflow_drop_oldest() {
        let config = StreamManagerConfig {
            max_buffer_tokens: 5,
            overflow_policy: BufferOverflowPolicy::DropOldest,
            ui_throttle_duration: Duration::ZERO,
            ..Default::default()
        };

        let conv_id = ConversationId::new();
        let msg_id = MessageId::new();
        let (tx, rx) = mpsc::channel(100);

        let mut stream = ConversationStream::new(conv_id, msg_id, rx, config);

        // Send 7 tokens (2 over limit)
        for i in 0..7 {
            tx.send(StreamingToken::Token(format!("t{i}")))
                .await
                .unwrap();
        }

        tokio::time::sleep(Duration::from_millis(10)).await;

        let events = stream.poll();

        // Should have overflow event
        let overflow_event = events
            .iter()
            .find(|e| matches!(e.kind, StreamEventKind::BufferOverflow { .. }));
        assert!(overflow_event.is_some());

        // Buffer should be at max size
        assert!(stream.buffer.len() <= 5);

        // Stats should show dropped tokens
        assert!(stream.stats().tokens_dropped > 0);
    }

    #[tokio::test]
    async fn test_poll_one() {
        let mut manager = StreamManager::with_config(StreamManagerConfig {
            ui_throttle_duration: Duration::ZERO,
            ..Default::default()
        });

        let conv_id = ConversationId::new();
        let msg_id = MessageId::new();

        let tokens = vec![
            StreamingToken::Token("Test".to_string()),
            StreamingToken::Complete {
                message: "Test".to_string(),
            },
        ];

        let rx = create_test_receiver(tokens);
        manager.register(conv_id, msg_id, rx).unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let events = manager.poll_one(conv_id);
        assert!(!events.is_empty());

        // All events should be for the requested conversation
        for event in &events {
            assert_eq!(event.conversation_id, conv_id);
        }
    }

    #[tokio::test]
    async fn test_unregister_stream() {
        let mut manager = StreamManager::new();
        let conv_id = ConversationId::new();
        let msg_id = MessageId::new();
        let (_, rx) = mpsc::channel::<StreamingToken>(10);

        manager.register(conv_id, msg_id, rx).unwrap();
        assert_eq!(manager.active_count(), 1);

        let stream = manager.unregister(conv_id);
        assert!(stream.is_some());
        assert_eq!(manager.active_count(), 0);
    }

    #[tokio::test]
    async fn test_clear_streams() {
        let mut manager = StreamManager::new();

        for _ in 0..3 {
            let (_, rx) = mpsc::channel::<StreamingToken>(10);
            let conv_id = ConversationId::new();
            let msg_id = MessageId::new();
            manager.register(conv_id, msg_id, rx).unwrap();
        }

        assert_eq!(manager.active_count(), 3);

        manager.clear();
        assert_eq!(manager.active_count(), 0);
        assert!(manager.is_empty());
    }

    #[tokio::test]
    async fn test_stream_stats() {
        let mut manager = StreamManager::with_config(StreamManagerConfig {
            ui_throttle_duration: Duration::ZERO,
            ..Default::default()
        });

        let conv_id = ConversationId::new();
        let msg_id = MessageId::new();

        let tokens = vec![
            StreamingToken::Token("a".to_string()),
            StreamingToken::Token("b".to_string()),
            StreamingToken::Token("c".to_string()),
        ];

        let rx = create_test_receiver(tokens);
        manager.register(conv_id, msg_id, rx).unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;
        manager.poll_all();

        let stats = manager.stream_stats(conv_id);
        // Stream might have completed and been removed, so stats might be None
        // This is expected behavior - we just verify it doesn't panic
        if let Some(stats) = stats {
            assert!(stats.started_at.is_some());
        }
    }

    #[test]
    fn test_stream_event_kinds() {
        // Test that all event kinds can be created
        let conv_id = ConversationId::new();
        let msg_id = MessageId::new();

        let tokens_event = StreamEvent {
            conversation_id: conv_id,
            message_id: msg_id.clone(),
            kind: StreamEventKind::Tokens {
                tokens: vec!["hello".to_string()],
                total_count: 1,
            },
            timestamp: Instant::now(),
        };
        assert!(matches!(tokens_event.kind, StreamEventKind::Tokens { .. }));

        let complete_event = StreamEvent {
            conversation_id: conv_id,
            message_id: msg_id.clone(),
            kind: StreamEventKind::Complete {
                message: "done".to_string(),
                token_count: 5,
                duration: Duration::from_secs(1),
            },
            timestamp: Instant::now(),
        };
        assert!(matches!(
            complete_event.kind,
            StreamEventKind::Complete { .. }
        ));

        let error_event = StreamEvent {
            conversation_id: conv_id,
            message_id: msg_id.clone(),
            kind: StreamEventKind::Error {
                error: "oops".to_string(),
                partial_content: "partial".to_string(),
            },
            timestamp: Instant::now(),
        };
        assert!(matches!(error_event.kind, StreamEventKind::Error { .. }));

        let overflow_event = StreamEvent {
            conversation_id: conv_id,
            message_id: msg_id,
            kind: StreamEventKind::BufferOverflow {
                affected_tokens: 10,
                policy: BufferOverflowPolicy::DropOldest,
            },
            timestamp: Instant::now(),
        };
        assert!(matches!(
            overflow_event.kind,
            StreamEventKind::BufferOverflow { .. }
        ));
    }

    #[test]
    fn test_stream_register_error_display() {
        let err1 = StreamRegisterError::MaxStreamsReached;
        assert_eq!(err1.to_string(), "maximum concurrent streams reached");

        let err2 = StreamRegisterError::StreamAlreadyExists;
        assert_eq!(
            err2.to_string(),
            "conversation already has an active stream"
        );
    }

    #[tokio::test]
    async fn test_no_cross_contamination() {
        // This test verifies that tokens from different conversations
        // never get mixed up
        let mut manager = StreamManager::with_config(StreamManagerConfig {
            ui_throttle_duration: Duration::ZERO,
            ..Default::default()
        });

        let conv_a = ConversationId::new();
        let conv_b = ConversationId::new();
        let msg_a = MessageId::new();
        let msg_b = MessageId::new();

        // Create streams with distinctive tokens
        let tokens_a = vec![
            StreamingToken::Token("AAA".to_string()),
            StreamingToken::Token("AAAA".to_string()),
            StreamingToken::Complete {
                message: "AAAAAAAA".to_string(),
            },
        ];

        let tokens_b = vec![
            StreamingToken::Token("BBB".to_string()),
            StreamingToken::Token("BBBB".to_string()),
            StreamingToken::Complete {
                message: "BBBBBBB".to_string(),
            },
        ];

        let rx_a = create_test_receiver(tokens_a);
        let rx_b = create_test_receiver(tokens_b);

        manager.register(conv_a, msg_a, rx_a).unwrap();
        manager.register(conv_b, msg_b, rx_b).unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let events = manager.poll_all();

        // Verify no cross-contamination
        for event in &events {
            match &event.kind {
                StreamEventKind::Tokens { tokens, .. } => {
                    for token in tokens {
                        if event.conversation_id == conv_a {
                            assert!(token.contains('A'), "Conv A got non-A token: {}", token);
                        } else if event.conversation_id == conv_b {
                            assert!(token.contains('B'), "Conv B got non-B token: {}", token);
                        }
                    }
                }
                StreamEventKind::Complete { message, .. } => {
                    if event.conversation_id == conv_a {
                        assert!(message.contains('A'), "Conv A got non-A complete");
                    } else if event.conversation_id == conv_b {
                        assert!(message.contains('B'), "Conv B got non-B complete");
                    }
                }
                _ => {}
            }
        }
    }

    #[tokio::test]
    async fn test_ui_throttling() {
        let config = StreamManagerConfig {
            ui_throttle_duration: Duration::from_millis(100),
            ..Default::default()
        };

        let conv_id = ConversationId::new();
        let msg_id = MessageId::new();
        let (tx, rx) = mpsc::channel(100);

        let mut stream = ConversationStream::new(conv_id, msg_id, rx, config);

        // Send first token
        tx.send(StreamingToken::Token("first".to_string()))
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;
        let events1 = stream.poll();

        // Should get token event (first update allowed)
        let has_tokens1 = events1
            .iter()
            .any(|e| matches!(e.kind, StreamEventKind::Tokens { .. }));
        assert!(has_tokens1, "First poll should produce token event");

        // Send second token immediately
        tx.send(StreamingToken::Token("second".to_string()))
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;
        let events2 = stream.poll();

        // Should NOT get token event (within throttle window)
        let has_tokens2 = events2
            .iter()
            .any(|e| matches!(e.kind, StreamEventKind::Tokens { .. }));
        assert!(
            !has_tokens2,
            "Second poll within throttle window should not produce token event"
        );

        // Wait for throttle window to pass
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Send third token
        tx.send(StreamingToken::Token("third".to_string()))
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;
        let events3 = stream.poll();

        // Should get token event (throttle window passed)
        let has_tokens3 = events3
            .iter()
            .any(|e| matches!(e.kind, StreamEventKind::Tokens { .. }));
        assert!(
            has_tokens3,
            "Poll after throttle window should produce token event"
        );
    }

    #[tokio::test]
    async fn test_conversation_stream_accessors() {
        let conv_id = ConversationId::new();
        let msg_id = MessageId::new();
        let (_, rx) = mpsc::channel::<StreamingToken>(10);
        let config = StreamManagerConfig::default();

        let stream = ConversationStream::new(conv_id, msg_id.clone(), rx, config);

        assert_eq!(stream.conversation_id(), conv_id);
        assert_eq!(stream.message_id(), msg_id);
        assert!(!stream.is_completed());
        assert!(stream.content().is_empty());
        assert_eq!(stream.token_count(), 0);
        assert!(stream.stats().started_at.is_some());
    }

    #[tokio::test]
    async fn test_lifetime_statistics() {
        let mut manager = StreamManager::with_config(StreamManagerConfig {
            ui_throttle_duration: Duration::ZERO,
            ..Default::default()
        });

        assert_eq!(manager.total_streams_created(), 0);
        assert_eq!(manager.total_tokens_processed(), 0);

        let tokens = vec![
            StreamingToken::Token("a".to_string()),
            StreamingToken::Token("b".to_string()),
            StreamingToken::Complete {
                message: "ab".to_string(),
            },
        ];

        let rx = create_test_receiver(tokens);
        let conv_id = ConversationId::new();
        let msg_id = MessageId::new();
        manager.register(conv_id, msg_id, rx).unwrap();

        assert_eq!(manager.total_streams_created(), 1);

        tokio::time::sleep(Duration::from_millis(50)).await;
        manager.poll_all();

        assert_eq!(manager.total_streams_created(), 1);
        assert!(manager.total_tokens_processed() >= 2);
    }
}
