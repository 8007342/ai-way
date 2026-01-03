# TODO: Multi-Conversation Architecture

> Parallel agent conversations with stacked visual rendering, border creatures, and meta-agent orchestration.

## Overview

Yollayah can have multiple simultaneous conversations with agents. Each conversation:
- Is wrapped in box lines with its own scrolling
- Has border decorations with small creatures (axolotls/lizards) showing activity
- Streams text responses independently
- Can be navigated by the user
- Gets compiled into a summary tab at the end

---

## Phase 1: Core Data Types (conductor-core)

### A. ConversationId and State

- [ ] **A1. Add ConversationId type** (`conductor/core/src/conversation.rs`)
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
  pub struct ConversationId(pub Uuid);

  impl ConversationId {
      pub fn new() -> Self { Self(Uuid::new_v4()) }
      pub fn main() -> Self { Self(Uuid::nil()) } // Single-conversation mode
  }
  ```

- [ ] **A2. Add ConversationState enum**
  ```rust
  #[derive(Clone, Debug, Serialize, Deserialize)]
  pub enum ConversationState {
      Idle,
      Streaming { message_id: MessageId },
      WaitingForAgent,
      Completed { summary: Option<String> },
      Error { message: String },
  }
  ```

- [ ] **A3. Add Conversation struct**
  ```rust
  #[derive(Clone, Debug)]
  pub struct Conversation {
      pub id: ConversationId,
      pub agent_name: Option<String>,  // None = direct user conversation
      pub state: ConversationState,
      pub messages: Vec<ConversationMessage>,
      pub created_at: Instant,
      pub last_activity: Instant,
      pub priority: u8,  // For meta-agent sorting
  }
  ```

- [ ] **A4. Add ConversationManager**
  ```rust
  pub struct ConversationManager {
      conversations: HashMap<ConversationId, Conversation>,
      focused: Option<ConversationId>,
      z_order: Vec<ConversationId>,  // Bottom to top
  }

  impl ConversationManager {
      pub fn create(&mut self, agent_name: Option<String>) -> ConversationId;
      pub fn focus(&mut self, id: ConversationId);
      pub fn pop_to_top(&mut self, id: ConversationId);
      pub fn get(&self, id: ConversationId) -> Option<&Conversation>;
      pub fn get_mut(&mut self, id: ConversationId) -> Option<&mut Conversation>;
      pub fn active_count(&self) -> usize;
      pub fn all_completed(&self) -> bool;
      pub fn generate_summary(&self) -> String;
  }
  ```

- [ ] **A5. Export from lib.rs**

### B. New Message Protocol

- [ ] **B1. Add conversation-aware ConductorMessage variants**
  ```rust
  // New variants for ConductorMessage:
  ConversationCreated {
      conversation_id: ConversationId,
      agent_name: Option<String>,
  },
  ConversationFocused {
      conversation_id: ConversationId,
  },
  ConversationStateChanged {
      conversation_id: ConversationId,
      state: ConversationState,
  },
  ConversationStreamToken {
      conversation_id: ConversationId,
      message_id: MessageId,
      token: String,
  },
  ConversationStreamEnd {
      conversation_id: ConversationId,
      message_id: MessageId,
      final_content: String,
      metadata: ResponseMetadata,
  },
  SummaryReady {
      conversation_id: ConversationId,
      summary: String,
      sub_conversations: Vec<ConversationId>,
  },
  ```

- [ ] **B2. Add conversation-aware SurfaceEvent variants**
  ```rust
  // New variants for SurfaceEvent:
  FocusConversation {
      event_id: EventId,
      conversation_id: ConversationId,
  },
  ScrollConversation {
      event_id: EventId,
      conversation_id: ConversationId,
      direction: ScrollDirection,
  },
  RequestSummary {
      event_id: EventId,
  },
  ```

---

## Phase 2: Parallel Streaming Infrastructure

### C. Channel Architecture (Hacker Design)

- [ ] **C1. Per-conversation stream receiver**
  ```rust
  pub struct ConversationStream {
      id: ConversationId,
      receiver: mpsc::Receiver<StreamingToken>,
      buffer: VecDeque<String>,
      buffer_limit: usize,  // Default: 1000 tokens
  }

  impl ConversationStream {
      pub fn try_recv(&mut self) -> Option<StreamingToken>;
      pub fn is_active(&self) -> bool;
  }
  ```

- [ ] **C2. StreamManager for parallel polling**
  ```rust
  pub struct StreamManager {
      streams: HashMap<ConversationId, ConversationStream>,
      focus_tx: broadcast::Sender<ConversationId>,
  }

  impl StreamManager {
      pub async fn poll_all(&mut self) -> Vec<(ConversationId, StreamingToken)>;
      pub fn add_stream(&mut self, id: ConversationId, rx: mpsc::Receiver<StreamingToken>);
      pub fn remove_stream(&mut self, id: ConversationId);
  }
  ```

- [ ] **C3. Non-blocking poll in Conductor**
  ```rust
  // In Conductor::poll_streaming():
  pub async fn poll_streaming(&mut self) {
      let tokens = self.stream_manager.poll_all().await;
      for (conv_id, token) in tokens {
          self.handle_conversation_token(conv_id, token).await;
      }
  }
  ```

### D. UI Update Throttling

- [ ] **D1. UIUpdateThrottle for rate limiting**
  ```rust
  pub struct UIUpdateThrottle {
      last_update: Instant,
      min_interval: Duration,  // 33ms = ~30 FPS
      pending_updates: HashMap<ConversationId, PendingUpdate>,
  }

  impl UIUpdateThrottle {
      pub fn should_update(&mut self, conv_id: ConversationId) -> bool;
      pub fn mark_updated(&mut self, conv_id: ConversationId);
      pub fn flush_pending(&mut self) -> Vec<(ConversationId, String)>;
  }
  ```

- [ ] **D2. Token batching for efficiency**
  - Batch tokens accumulated between UI updates
  - Send as single ConversationStreamToken with concatenated content

### E. Memory Management

- [ ] **E1. Per-stream buffer limits**
  - Default: 1000 tokens per stream
  - Evict oldest when limit reached

- [ ] **E2. Global memory bounds**
  - Max 10 active conversations
  - Auto-complete stale conversations after timeout

---

## Phase 3: TUI Rendering

### F. Box Drawing and Layout

- [ ] **F1. ConversationBox widget**
  ```rust
  pub struct ConversationBox {
      conversation_id: ConversationId,
      title: String,
      is_focused: bool,
      is_active: bool,  // Has streaming activity
      scroll_offset: usize,
      content_height: usize,
  }
  ```

- [ ] **F2. Box characters**
  ```
  Active (focused):
  ┏━━━━━━━━━━ Architect ━━━━━━━━━━┓
  ┃ Content here...               ┃
  ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛

  Inactive:
  ┌────────── UX Expert ──────────┐
  │ Content here...               │
  └───────────────────────────────┘
  ```

- [ ] **F3. Stacked visualization**
  ```
  ┌─ Hacker (2 of 4) ─────────────────────────┐
  │ Analyzing async patterns...               │
  │                                           │
  │ ┌─ QA ─────────────────────────────────┐ │
  │ │ Writing test cases...                │ │
  │ │                                      │ │
  │ │ ┏━ Architect (focused) ━━━━━━━━━━━┓ │ │
  │ │ ┃ Layer separation design:        ┃ │ │
  │ │ ┃ 1. Conductor manages...         ┃ │ │
  │ │ ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛ │ │
  │ └──────────────────────────────────────┘ │
  └───────────────────────────────────────────┘
  ```

### G. Border Creatures (UX Design)

- [ ] **G1. Micro axolotl sprites**
  ```
  Idle:      ∘°∘    or    ·.·
  Walking:   >°<    or    ᵔᴥᵔ
  Active:    ⁂°⁂   (gills wiggling)
  ```

- [ ] **G2. Micro lizard sprites**
  ```
  Idle:      ~:>    or    ·:·
  Walking:   >:~    (tail swishing)
  Active:    ≈:>    (running)
  ```

- [ ] **G3. Bubble animations**
  ```
  Rising: ○ ◦ · (from bottom to top of border)
  ```

- [ ] **G4. CreatureAnimator**
  ```rust
  pub struct BorderCreature {
      kind: CreatureKind,  // Axolotl, Lizard, Bubble
      position: usize,     // Position along border (0 = top-left, clockwise)
      direction: i8,       // -1 = CCW, 0 = idle, 1 = CW
      animation_frame: u8,
      last_move: Instant,
  }

  pub struct CreatureAnimator {
      creatures: Vec<BorderCreature>,
      spawn_probability: f32,  // Per second
  }
  ```

- [ ] **G5. Activity-based spawning**
  - Spawn creature when conversation receives token
  - Creature walks border while streaming active
  - Fade out when conversation completes

### H. Navigation

- [ ] **H1. Keyboard bindings**
  - `Tab` / `Shift+Tab`: Cycle through conversations
  - `1-9`: Jump to conversation by index
  - `Esc`: Return to summary view
  - Arrow keys: Scroll within focused conversation

- [ ] **H2. Visual focus indicators**
  - Bold border for focused conversation
  - Breathing glow effect on focused title
  - Dim unfocused conversations

### I. Summary Tab

- [ ] **I1. Summary view layout**
  ```
  ┏━━━━━━━━━━━ Summary ━━━━━━━━━━━━━━━━━━━━━━━┓
  ┃                                           ┃
  ┃  Task: Multi-conversation architecture    ┃
  ┃                                           ┃
  ┃  Agents consulted: 4                      ┃
  ┃  ├─ Architect: Layer separation ✓         ┃
  ┃  ├─ UX Expert: Visual design ✓            ┃
  ┃  ├─ Hacker: Async patterns ✓              ┃
  ┃  └─ QA: Test strategy ✓                   ┃
  ┃                                           ┃
  ┃  Key findings:                            ┃
  ┃  • ConversationManager for orchestration  ┃
  ┃  • Non-blocking parallel streaming        ┃
  ┃  • Border creatures for activity          ┃
  ┃                                           ┃
  ┃  [Press 1-4 to view agent details]        ┃
  ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
  ```

- [ ] **I2. Summary generation**
  - Meta-agent compiles outputs from all conversations
  - Extracts key points and decisions
  - Provides navigation hints

---

## Phase 4: Meta-Agent Orchestration

### J. Focus Selection Logic

- [ ] **J1. Relevance scoring**
  ```rust
  pub fn calculate_relevance(conv: &Conversation) -> f32 {
      let mut score = 0.0;

      // Recency boost
      let age = conv.last_activity.elapsed();
      score += 1.0 / (1.0 + age.as_secs_f32());

      // Activity boost
      if matches!(conv.state, ConversationState::Streaming { .. }) {
          score += 2.0;
      }

      // Priority boost
      score += conv.priority as f32 * 0.5;

      score
  }
  ```

- [ ] **J2. Auto-focus on significant events**
  - New conversation created
  - Conversation completes with important findings
  - Error occurs in conversation

- [ ] **J3. Smooth transitions**
  - Animate Yollayah moving to new conversation
  - Visual transition effect (fade/slide)

---

## Phase 5: Testing Strategy (QA Design)

### K. Unit Tests

- [ ] **K1. ConversationManager tests**
  - Create/focus/remove conversations
  - Z-order management
  - Summary generation

- [ ] **K2. StreamManager tests**
  - Parallel polling correctness
  - Buffer overflow handling
  - Stream cleanup

- [ ] **K3. CreatureAnimator tests**
  - Spawn probability
  - Movement along border
  - Activity correlation

### L. Integration Tests

- [ ] **L1. Parallel streaming test**
  - 3 conversations streaming simultaneously
  - Verify all tokens received correctly
  - Verify no cross-contamination

- [ ] **L2. Focus switching test**
  - Switch focus during streaming
  - Verify streams continue
  - Verify UI updates correctly

- [ ] **L3. Summary compilation test**
  - Complete multiple conversations
  - Verify summary accuracy
  - Verify navigation to sub-conversations

### M. Performance Tests

- [ ] **M1. Streaming throughput**
  - Target: 1000 tokens/sec per conversation
  - Measure latency distribution

- [ ] **M2. Memory usage**
  - Max 10 conversations
  - Verify eviction works

- [ ] **M3. UI responsiveness**
  - Maintain 30 FPS during heavy streaming
  - Measure input latency

### N. Edge Cases

- [ ] **N1. Rapid conversation creation**
  - Create 10 conversations in <1 second
  - Verify no race conditions

- [ ] **N2. Conversation timeout**
  - Stale conversation handling
  - Auto-cleanup after 5 minutes idle

- [ ] **N3. Error propagation**
  - Backend error in one conversation
  - Other conversations unaffected

---

## Phase 6: Documentation

### O. User Documentation

- [ ] **O1. Multi-conversation usage guide**
  - Keyboard shortcuts
  - Visual indicators explained
  - Summary tab usage

### P. Developer Documentation

- [ ] **P1. Architecture guide**
  - Layer separation diagram
  - Message flow documentation

- [ ] **P2. API documentation**
  - ConversationManager API
  - StreamManager API
  - Widget documentation

---

## Implementation Order

1. **Core types** (A1-A5) - Foundation for everything
2. **Message protocol** (B1-B2) - Communication layer
3. **Parallel streaming** (C1-C3) - Enable concurrent agents
4. **Basic TUI rendering** (F1-F3) - Visual representation
5. **Navigation** (H1-H2) - User interaction
6. **Border creatures** (G1-G5) - Polish and delight
7. **Summary tab** (I1-I2) - Compilation view
8. **Meta-agent orchestration** (J1-J3) - Intelligent focus
9. **Testing** (K-N) - Verification
10. **Documentation** (O-P) - Knowledge transfer

---

## Color Reference (from theme/mod.rs)

```rust
// Conversation borders
ACTIVE_BORDER: Color::Rgb(255, 182, 193)   // AXOLOTL_BODY
INACTIVE_BORDER: Color::Rgb(100, 100, 100) // DIM_GRAY
FOCUSED_GLOW: Color::Rgb(255, 150, 255)    // Breathing magenta

// Creatures
CREATURE_AXOLOTL: Color::Rgb(255, 182, 193)
CREATURE_LIZARD: Color::Rgb(130, 220, 130)
CREATURE_BUBBLE: Color::Rgb(200, 230, 255)

// Summary
SUMMARY_HEADER: Color::Rgb(255, 223, 128)  // MOOD_HAPPY
SUMMARY_CHECKMARK: Color::Rgb(120, 230, 120) // SUCCESS_GREEN
```

---

## Notes

- All streaming uses non-blocking `try_recv()` to prevent UI freezes
- Border creatures are purely decorative but indicate activity
- Summary tab is optional - user can always view individual conversations
- Meta-agent focus selection respects user override (manual focus sticky)
