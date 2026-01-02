//! Multi-Conversation Management
//!
//! Support for parallel agent conversations with stacked visual rendering.
//! Each conversation can stream independently while the meta-agent orchestrates
//! focus and priority.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   ConversationManager                        │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐            │
//! │  │ Conversation│ │ Conversation│ │ Conversation│ ← z_order  │
//! │  │ (Architect) │ │ (UX Expert) │ │   (QA)      │            │
//! │  └─────────────┘ └─────────────┘ └─────────────┘            │
//! │                        ↑                                     │
//! │                    focused                                   │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

use crate::messages::{MessageId, ResponseMetadata};
use crate::session::ConversationMessage;

// ============================================================================
// Core Types
// ============================================================================

/// Unique identifier for a conversation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConversationId(pub Uuid);

impl ConversationId {
    /// Create a new unique conversation ID
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the "main" conversation ID (for single-conversation mode)
    ///
    /// Uses nil UUID to represent the default/main conversation.
    #[must_use]
    pub fn main() -> Self {
        Self(Uuid::nil())
    }

    /// Check if this is the main conversation
    #[must_use]
    pub fn is_main(&self) -> bool {
        self.0.is_nil()
    }
}

impl Default for ConversationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ConversationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_main() {
            write!(f, "main")
        } else {
            // Short form: first 8 chars of UUID
            write!(f, "{}", &self.0.to_string()[..8])
        }
    }
}

/// Current state of a conversation
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum ConversationState {
    /// Waiting for input or response
    #[default]
    Idle,
    /// Currently streaming a response
    Streaming {
        /// Message ID being streamed
        message_id: MessageId,
    },
    /// Waiting for agent to process
    WaitingForAgent,
    /// Conversation completed
    Completed {
        /// Optional summary of the conversation
        summary: Option<String>,
    },
    /// Error occurred
    Error {
        /// Error message
        message: String,
    },
}

impl ConversationState {
    /// Check if conversation is actively streaming
    #[must_use]
    pub fn is_streaming(&self) -> bool {
        matches!(self, Self::Streaming { .. })
    }

    /// Check if conversation is completed (success or error)
    #[must_use]
    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Completed { .. } | Self::Error { .. })
    }

    /// Check if conversation is in an error state
    #[must_use]
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }
}

/// A single conversation with an agent or the user
#[derive(Clone, Debug)]
pub struct Conversation {
    /// Unique conversation identifier
    pub id: ConversationId,
    /// Name of the agent (None for direct user conversation)
    pub agent_name: Option<String>,
    /// Current state
    pub state: ConversationState,
    /// Messages in this conversation
    pub messages: Vec<ConversationMessage>,
    /// When the conversation was created
    pub created_at: Instant,
    /// Last activity timestamp
    pub last_activity: Instant,
    /// Priority for meta-agent focus (higher = more important)
    pub priority: u8,
    /// Accumulated token buffer for streaming
    pub stream_buffer: String,
    /// Token count for current/last stream
    pub token_count: u32,
    /// Response metadata from last completion
    pub last_metadata: Option<ResponseMetadata>,
}

impl Conversation {
    /// Create a new conversation
    #[must_use]
    pub fn new(agent_name: Option<String>) -> Self {
        let now = Instant::now();
        Self {
            id: ConversationId::new(),
            agent_name,
            state: ConversationState::Idle,
            messages: Vec::new(),
            created_at: now,
            last_activity: now,
            priority: 50, // Default middle priority
            stream_buffer: String::new(),
            token_count: 0,
            last_metadata: None,
        }
    }

    /// Create the main conversation (for single-conversation mode)
    #[must_use]
    pub fn main() -> Self {
        let now = Instant::now();
        Self {
            id: ConversationId::main(),
            agent_name: None,
            state: ConversationState::Idle,
            messages: Vec::new(),
            created_at: now,
            last_activity: now,
            priority: 100, // Main conversation has highest priority
            stream_buffer: String::new(),
            token_count: 0,
            last_metadata: None,
        }
    }

    /// Create a conversation with a specific ID (for testing)
    #[must_use]
    pub fn with_id(id: ConversationId, agent_name: Option<String>) -> Self {
        let now = Instant::now();
        Self {
            id,
            agent_name,
            state: ConversationState::Idle,
            messages: Vec::new(),
            created_at: now,
            last_activity: now,
            priority: 50,
            stream_buffer: String::new(),
            token_count: 0,
            last_metadata: None,
        }
    }

    /// Get display title for the conversation
    #[must_use]
    pub fn title(&self) -> String {
        self.agent_name
            .clone()
            .unwrap_or_else(|| "Yollayah".to_string())
    }

    /// Mark activity (updates `last_activity` timestamp)
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Start streaming a new message
    pub fn start_streaming(&mut self, message_id: MessageId) {
        self.state = ConversationState::Streaming { message_id };
        self.stream_buffer.clear();
        self.token_count = 0;
        self.touch();
    }

    /// Append a streaming token
    pub fn append_token(&mut self, token: &str) {
        self.stream_buffer.push_str(token);
        self.token_count += 1;
        self.touch();
    }

    /// Complete streaming
    pub fn complete_streaming(&mut self, final_content: String, metadata: ResponseMetadata) {
        self.state = ConversationState::Idle;
        self.stream_buffer = final_content;
        self.last_metadata = Some(metadata);
        self.touch();
    }

    /// Set error state
    pub fn set_error(&mut self, message: String) {
        self.state = ConversationState::Error { message };
        self.touch();
    }

    /// Complete the conversation with an optional summary
    pub fn complete(&mut self, summary: Option<String>) {
        self.state = ConversationState::Completed { summary };
        self.touch();
    }

    /// Calculate relevance score for focus selection
    #[must_use]
    pub fn relevance_score(&self) -> f32 {
        let mut score = 0.0;

        // Recency boost (decays over time)
        let age_secs = self.last_activity.elapsed().as_secs_f32();
        score += 1.0 / (1.0 + age_secs * 0.1);

        // Activity boost
        if self.state.is_streaming() {
            score += 2.0;
        }

        // Priority boost
        score += f32::from(self.priority) * 0.02;

        // Error penalty (demote errored conversations)
        if self.state.is_error() {
            score *= 0.5;
        }

        score
    }
}

// ============================================================================
// Conversation Manager
// ============================================================================

/// Manages multiple concurrent conversations
#[derive(Debug)]
pub struct ConversationManager {
    /// All active conversations
    conversations: HashMap<ConversationId, Conversation>,
    /// Currently focused conversation (if any)
    focused: Option<ConversationId>,
    /// Z-order for stacking (bottom to top)
    z_order: Vec<ConversationId>,
    /// Maximum number of concurrent conversations
    max_conversations: usize,
    /// User has manually focused (sticky until reset)
    user_focused: bool,
}

impl Default for ConversationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationManager {
    /// Create a new conversation manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            conversations: HashMap::new(),
            focused: None,
            z_order: Vec::new(),
            max_conversations: 10,
            user_focused: false,
        }
    }

    /// Create a new conversation manager with a main conversation
    #[must_use]
    pub fn with_main() -> Self {
        let mut manager = Self::new();
        let main = Conversation::main();
        let id = main.id;
        manager.conversations.insert(id, main);
        manager.z_order.push(id);
        manager.focused = Some(id);
        manager
    }

    /// Create a new conversation
    ///
    /// Returns the conversation ID, or None if max conversations reached.
    pub fn create(&mut self, agent_name: Option<String>) -> Option<ConversationId> {
        if self.conversations.len() >= self.max_conversations {
            return None;
        }

        let conversation = Conversation::new(agent_name);
        let id = conversation.id;
        self.conversations.insert(id, conversation);
        self.z_order.push(id);

        // Auto-focus if this is the first conversation
        if self.focused.is_none() {
            self.focused = Some(id);
        }

        Some(id)
    }

    /// Focus on a specific conversation
    pub fn focus(&mut self, id: ConversationId) {
        if self.conversations.contains_key(&id) {
            self.focused = Some(id);
            self.pop_to_top(id);
        }
    }

    /// User explicitly focused (sticky)
    pub fn user_focus(&mut self, id: ConversationId) {
        self.focus(id);
        self.user_focused = true;
    }

    /// Reset user focus (allow meta-agent to control again)
    pub fn reset_user_focus(&mut self) {
        self.user_focused = false;
    }

    /// Check if user has manual focus
    #[must_use]
    pub fn has_user_focus(&self) -> bool {
        self.user_focused
    }

    /// Pop a conversation to the top of the z-order
    pub fn pop_to_top(&mut self, id: ConversationId) {
        if let Some(pos) = self.z_order.iter().position(|&x| x == id) {
            self.z_order.remove(pos);
            self.z_order.push(id);
        }
    }

    /// Get the currently focused conversation
    #[must_use]
    pub fn focused(&self) -> Option<&Conversation> {
        self.focused
            .as_ref()
            .and_then(|id| self.conversations.get(id))
    }

    /// Get the currently focused conversation (mutable)
    pub fn focused_mut(&mut self) -> Option<&mut Conversation> {
        self.focused
            .as_ref()
            .and_then(|id| self.conversations.get_mut(id))
    }

    /// Get focused conversation ID
    #[must_use]
    pub fn focused_id(&self) -> Option<ConversationId> {
        self.focused
    }

    /// Get a conversation by ID
    #[must_use]
    pub fn get(&self, id: ConversationId) -> Option<&Conversation> {
        self.conversations.get(&id)
    }

    /// Get a conversation by ID (mutable)
    pub fn get_mut(&mut self, id: ConversationId) -> Option<&mut Conversation> {
        self.conversations.get_mut(&id)
    }

    /// Get the main conversation
    #[must_use]
    pub fn main(&self) -> Option<&Conversation> {
        self.conversations.get(&ConversationId::main())
    }

    /// Get the main conversation (mutable)
    pub fn main_mut(&mut self) -> Option<&mut Conversation> {
        self.conversations.get_mut(&ConversationId::main())
    }

    /// Get number of active conversations
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.conversations
            .values()
            .filter(|c| !c.state.is_finished())
            .count()
    }

    /// Get total number of conversations
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.conversations.len()
    }

    /// Check if all conversations are completed
    #[must_use]
    pub fn all_completed(&self) -> bool {
        self.conversations.values().all(|c| c.state.is_finished())
    }

    /// Get conversations in z-order (bottom to top)
    #[must_use]
    pub fn in_z_order(&self) -> Vec<&Conversation> {
        self.z_order
            .iter()
            .filter_map(|id| self.conversations.get(id))
            .collect()
    }

    /// Get number of streaming conversations
    #[must_use]
    pub fn streaming_count(&self) -> usize {
        self.conversations
            .values()
            .filter(|c| c.state.is_streaming())
            .count()
    }

    /// Auto-select best conversation to focus (for meta-agent)
    ///
    /// Returns Some(id) if focus should change, None if no change needed.
    pub fn auto_focus(&mut self) -> Option<ConversationId> {
        // Don't auto-focus if user has manual control
        if self.user_focused {
            return None;
        }

        // Find highest relevance conversation
        let best = self
            .conversations
            .values()
            .max_by(|a, b| {
                a.relevance_score()
                    .partial_cmp(&b.relevance_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|c| c.id);

        if best != self.focused {
            if let Some(id) = best {
                self.focus(id);
                return Some(id);
            }
        }

        None
    }

    /// Remove a conversation
    pub fn remove(&mut self, id: ConversationId) {
        self.conversations.remove(&id);
        self.z_order.retain(|&x| x != id);

        // Update focus if needed
        if self.focused == Some(id) {
            self.focused = self.z_order.last().copied();
        }
    }

    /// Generate a summary of all conversations
    #[must_use]
    pub fn generate_summary(&self) -> String {
        let mut summary = String::new();
        summary.push_str("# Conversation Summary\n\n");

        for conv in self.in_z_order() {
            summary.push_str(&format!("## {}\n", conv.title()));
            summary.push_str(&format!("- Status: {:?}\n", conv.state));
            summary.push_str(&format!("- Messages: {}\n", conv.messages.len()));

            if let Some(ref meta) = conv.last_metadata {
                summary.push_str(&format!(
                    "- Tokens: {} ({:.1} tok/s)\n",
                    meta.token_count,
                    meta.tokens_per_second.unwrap_or(0.0)
                ));
            }

            if let ConversationState::Completed {
                summary: Some(ref s),
            } = conv.state
            {
                summary.push_str(&format!("- Summary: {s}\n"));
            }

            summary.push('\n');
        }

        summary
    }

    /// Cycle focus to next conversation
    pub fn focus_next(&mut self) {
        if self.z_order.len() <= 1 {
            return;
        }

        if let Some(current) = self.focused {
            if let Some(pos) = self.z_order.iter().position(|&x| x == current) {
                let next_pos = (pos + 1) % self.z_order.len();
                self.focused = Some(self.z_order[next_pos]);
            }
        }
    }

    /// Cycle focus to previous conversation
    pub fn focus_prev(&mut self) {
        if self.z_order.len() <= 1 {
            return;
        }

        if let Some(current) = self.focused {
            if let Some(pos) = self.z_order.iter().position(|&x| x == current) {
                let prev_pos = if pos == 0 {
                    self.z_order.len() - 1
                } else {
                    pos - 1
                };
                self.focused = Some(self.z_order[prev_pos]);
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_id_uniqueness() {
        let id1 = ConversationId::new();
        let id2 = ConversationId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_conversation_id_main() {
        let main = ConversationId::main();
        assert!(main.is_main());

        let other = ConversationId::new();
        assert!(!other.is_main());
    }

    #[test]
    fn test_conversation_id_display() {
        let main = ConversationId::main();
        assert_eq!(format!("{main}"), "main");

        let other = ConversationId::new();
        let display = format!("{other}");
        assert_eq!(display.len(), 8);
    }

    #[test]
    fn test_conversation_state_checks() {
        let idle = ConversationState::Idle;
        assert!(!idle.is_streaming());
        assert!(!idle.is_finished());

        let streaming = ConversationState::Streaming {
            message_id: MessageId::new(),
        };
        assert!(streaming.is_streaming());
        assert!(!streaming.is_finished());

        let completed = ConversationState::Completed { summary: None };
        assert!(!completed.is_streaming());
        assert!(completed.is_finished());

        let error = ConversationState::Error {
            message: "oops".to_string(),
        };
        assert!(error.is_error());
        assert!(error.is_finished());
    }

    #[test]
    fn test_conversation_streaming() {
        let mut conv = Conversation::new(Some("Test".to_string()));

        let msg_id = MessageId::new();
        conv.start_streaming(msg_id);
        assert!(conv.state.is_streaming());

        conv.append_token("Hello ");
        conv.append_token("world!");
        assert_eq!(conv.stream_buffer, "Hello world!");
        assert_eq!(conv.token_count, 2);

        conv.complete_streaming(
            "Hello world!".to_string(),
            ResponseMetadata::with_timing(100, 2),
        );
        assert!(!conv.state.is_streaming());
        assert!(conv.last_metadata.is_some());
    }

    #[test]
    fn test_conversation_manager_create() {
        let mut manager = ConversationManager::new();
        assert_eq!(manager.total_count(), 0);

        let id = manager.create(Some("Agent1".to_string())).unwrap();
        assert_eq!(manager.total_count(), 1);
        assert_eq!(manager.focused_id(), Some(id));

        let id2 = manager.create(Some("Agent2".to_string())).unwrap();
        assert_eq!(manager.total_count(), 2);
        // First conversation remains focused
        assert_eq!(manager.focused_id(), Some(id));

        // Focus second
        manager.focus(id2);
        assert_eq!(manager.focused_id(), Some(id2));
    }

    #[test]
    fn test_conversation_manager_z_order() {
        let mut manager = ConversationManager::new();

        let id1 = manager.create(Some("A".to_string())).unwrap();
        let id2 = manager.create(Some("B".to_string())).unwrap();
        let id3 = manager.create(Some("C".to_string())).unwrap();

        // Initial order: [A, B, C]
        let order: Vec<_> = manager.in_z_order().iter().map(|c| c.id).collect();
        assert_eq!(order, vec![id1, id2, id3]);

        // Pop A to top: [B, C, A]
        manager.pop_to_top(id1);
        let order: Vec<_> = manager.in_z_order().iter().map(|c| c.id).collect();
        assert_eq!(order, vec![id2, id3, id1]);
    }

    #[test]
    fn test_conversation_manager_with_main() {
        let manager = ConversationManager::with_main();
        assert_eq!(manager.total_count(), 1);
        assert!(manager.main().is_some());
        assert_eq!(manager.focused_id(), Some(ConversationId::main()));
    }

    #[test]
    fn test_conversation_manager_focus_cycle() {
        let mut manager = ConversationManager::new();

        let id1 = manager.create(Some("A".to_string())).unwrap();
        let id2 = manager.create(Some("B".to_string())).unwrap();
        let id3 = manager.create(Some("C".to_string())).unwrap();

        manager.focus(id1);
        assert_eq!(manager.focused_id(), Some(id1));

        manager.focus_next();
        assert_eq!(manager.focused_id(), Some(id2));

        manager.focus_next();
        assert_eq!(manager.focused_id(), Some(id3));

        manager.focus_next(); // Wraps
        assert_eq!(manager.focused_id(), Some(id1));

        manager.focus_prev(); // Back to C
        assert_eq!(manager.focused_id(), Some(id3));
    }

    #[test]
    fn test_conversation_manager_user_focus() {
        let mut manager = ConversationManager::new();

        let id1 = manager.create(Some("A".to_string())).unwrap();
        let _id2 = manager.create(Some("B".to_string())).unwrap();

        manager.user_focus(id1);
        assert!(manager.has_user_focus());

        // Auto-focus should be blocked
        assert!(manager.auto_focus().is_none());

        manager.reset_user_focus();
        assert!(!manager.has_user_focus());
    }

    #[test]
    fn test_conversation_manager_remove() {
        let mut manager = ConversationManager::new();

        let id1 = manager.create(Some("A".to_string())).unwrap();
        let id2 = manager.create(Some("B".to_string())).unwrap();

        manager.focus(id1);
        manager.remove(id1);

        assert_eq!(manager.total_count(), 1);
        assert_eq!(manager.focused_id(), Some(id2));
    }
}
