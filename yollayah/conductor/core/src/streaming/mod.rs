//! Streaming Infrastructure for Parallel Conversations
//!
//! This module provides the core streaming infrastructure for managing multiple
//! concurrent LLM response streams. Each conversation can have an independent
//! streaming response, and the `StreamManager` coordinates polling all streams
//! without cross-contamination.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                       StreamManager                               │
//! │  ┌────────────────────────────────────────────────────────────┐  │
//! │  │                    Active Streams Map                       │  │
//! │  │  ┌─────────────────┐  ┌─────────────────┐                  │  │
//! │  │  │ConversationStream│  │ConversationStream│  ...            │  │
//! │  │  │  (conv_id: A)    │  │  (conv_id: B)    │                  │  │
//! │  │  │  [buffer: 1000]  │  │  [buffer: 1000]  │                  │  │
//! │  │  └────────┬─────────┘  └────────┬─────────┘                  │  │
//! │  │           │                     │                            │  │
//! │  └───────────┼─────────────────────┼────────────────────────────┘  │
//! │              │                     │                               │
//! │         poll_all() ─────────────────┘                              │
//! │              │                                                     │
//! │              ▼                                                     │
//! │     StreamEvent { conv_id, tokens }                                │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - **Non-blocking parallel polling**: `poll_all()` polls all streams concurrently
//! - **Buffer management**: Each stream has a 1000 token limit to prevent memory bloat
//! - **UI throttling**: Rate limiting at ~30 FPS for smooth rendering
//! - **No cross-contamination**: Streams are isolated by conversation ID
//!
//! # Example
//!
//! ```ignore
//! use conductor_core::streaming::{StreamManager, ConversationStream};
//! use conductor_core::{ConversationId, StreamingToken};
//! use tokio::sync::mpsc;
//!
//! // Create manager
//! let mut manager = StreamManager::new();
//!
//! // Register a stream for a conversation
//! let conv_id = ConversationId::new();
//! let (tx, rx) = mpsc::channel(100);
//! manager.register(conv_id, rx);
//!
//! // Poll all streams (non-blocking)
//! let events = manager.poll_all().await;
//! for event in events {
//!     println!("Conversation {}: {:?}", event.conversation_id, event.tokens);
//! }
//! ```

mod stream_manager;

pub use stream_manager::{
    BufferOverflowPolicy, ConversationStream, StreamEvent, StreamEventKind, StreamManager,
    StreamManagerConfig, StreamStats,
};

// Re-export StreamingToken from backend for convenience
pub use crate::backend::StreamingToken;
