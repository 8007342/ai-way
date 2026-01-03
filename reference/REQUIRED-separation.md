# REQUIRED: TUI/Conductor Separation

**Status**: ✅ HARD REQUIREMENT
**Severity**: CRITICAL - Violations Block Production
**Last Updated**: 2026-01-03

---

## Core Requirement

**The TUI and Conductor MUST be completely separate, independently deployable components.**

The TUI is a **thin client surface** that can be disabled, replaced, or swapped entirely without any modifications to the Conductor. The Conductor is the **business logic core** that operates independently of any specific UI.

---

## The Five Separation Laws

### Law 1: No Direct Dependencies

**The TUI MUST NOT import Conductor business logic.**

**FORBIDDEN**:
```rust
// ❌ TERRIBLE - TUI importing Conductor internals
use conductor_core::session::Session;
use conductor_core::llm::LlmBackend;

impl TUI {
    fn handle_input(&mut self, input: String) {
        let session = Session::new(); // ← Business logic in UI!
        let backend = OllamaBackend::new(); // ← Direct backend access!
    }
}
```

**REQUIRED**:
```rust
// ✅ GOOD - TUI only imports messages and transport
use conductor_core::messages::ConductorMessage;
use conductor_core::events::SurfaceEvent;
use conductor_core::transport::InProcessTransport;

impl TUI {
    fn handle_input(&mut self, input: String) {
        // Send event, Conductor handles everything
        self.conductor.send_event(SurfaceEvent::UserMessage {
            content: input,
        });
    }
}
```

---

### Law 2: Message-Based Communication Only

**All communication MUST use defined message types over a transport layer.**

**Message Flow**:
```
┌─────────┐                      ┌───────────┐
│   TUI   │  SurfaceEvent        │ Conductor │
│         │ ──────────────────►  │           │
│         │                      │           │
│         │  ConductorMessage    │           │
│         │ ◄────────────────── │           │
└─────────┘                      └───────────┘
```

**Transport Abstraction**:
```rust
pub trait Transport {
    async fn send(&self, event: SurfaceEvent) -> Result<()>;
    async fn recv(&mut self) -> Option<ConductorMessage>;
}

// Two implementations:
// 1. InProcessTransport - For embedded TUI (mpsc channels)
// 2. UnixSocketTransport - For daemon mode (IPC)
```

---

### Law 3: Conductor is Surface-Agnostic

**The Conductor MUST NOT know about TUI-specific details.**

**FORBIDDEN**:
```rust
// ❌ TERRIBLE - Conductor knows about TUI specifics
impl Conductor {
    fn update_avatar_position(&mut self, x: u16, y: u16) { // ← TUI details!
        self.avatar_x = x;
        self.avatar_y = y;
    }
}
```

**REQUIRED**:
```rust
// ✅ GOOD - Conductor sends abstract state changes
impl Conductor {
    async fn update_avatar_state(&mut self, mood: AvatarMood) {
        self.avatar_state.mood = mood;

        // Send message - Surface interprets it
        self.send(ConductorMessage::AvatarMood { mood }).await;
    }
}
```

---

### Law 4: Swappable Surfaces

**Any surface can be swapped without Conductor changes.**

**Deployment Modes**:

| Mode | Surface | Conductor | Transport | Use Case |
|------|---------|-----------|-----------|----------|
| **Embedded TUI** | Same process | Same process | InProcess (channels) | Default user mode |
| **Daemon + TUI** | Separate process | Daemon process | Unix Socket (IPC) | Background service |
| **Daemon + Web** | Web browser | Daemon process | WebSocket (IPC) | Remote access |
| **Daemon + CLI** | Terminal CLI | Daemon process | Unix Socket (IPC) | Scripting |
| **Headless** | None | Standalone | N/A | API server |

**All modes MUST work without Conductor code changes.**

---

### Law 5: State Belongs to Conductor

**All business state MUST live in the Conductor.**

**REQUIRED State Ownership**:
- ✅ Session history → Conductor
- ✅ LLM backend state → Conductor
- ✅ Avatar personality → Conductor
- ✅ Task management → Conductor
- ✅ Model routing → Conductor

**Surface-Only State** (Display state, not business state):
- ✅ Scroll position → TUI
- ✅ Input buffer → TUI
- ✅ Terminal size → TUI
- ✅ Color scheme → TUI
- ✅ Animation frame index → TUI

**FORBIDDEN** (Business state in TUI):
```rust
// ❌ TERRIBLE - TUI tracking business state
struct TUI {
    conversation_history: Vec<Message>, // ← Should be in Conductor!
    current_model: String,              // ← Should be in Conductor!
}
```

**REQUIRED** (TUI tracks display state only):
```rust
// ✅ GOOD - TUI tracks display representation
struct TUI {
    display: DisplayState,  // ← Derived from Conductor messages
    scroll_offset: usize,   // ← UI-only state
    input_buffer: String,   // ← UI-only state
}

struct DisplayState {
    messages: Vec<DisplayMessage>,  // ← Reflects Conductor state
    avatar: AvatarDisplayState,     // ← Reflects Conductor state
}
```

---

## Architecture Diagram

```
┌────────────────────────────────────────────────────────────────────┐
│                          AI-WAY SYSTEM                             │
│                                                                    │
│  ┌──────────────────────────┐       ┌──────────────────────────┐  │
│  │      SURFACE LAYER       │       │    CONDUCTOR CORE        │  │
│  │    (Thin Clients)        │       │   (Business Logic)       │  │
│  │                          │       │                          │  │
│  │  ┌────────┐              │       │  ┌──────────────────┐   │  │
│  │  │  TUI   │──────┐       │       │  │ Session Manager  │   │  │
│  │  └────────┘      │       │       │  └──────────────────┘   │  │
│  │                  │       │       │                          │  │
│  │  ┌────────┐      │       │       │  ┌──────────────────┐   │  │
│  │  │  Web   │──────┼───────┼───────┼─►│  LLM Backends    │   │  │
│  │  └────────┘      │       │       │  └──────────────────┘   │  │
│  │                  │       │       │                          │  │
│  │  ┌────────┐      │       │       │  ┌──────────────────┐   │  │
│  │  │  CLI   │──────┘       │       │  │ Avatar Engine    │   │  │
│  │  └────────┘              │       │  └──────────────────┘   │  │
│  │                          │       │                          │  │
│  │  All surfaces are        │       │  Conductor is            │  │
│  │  independent and         │       │  surface-agnostic        │  │
│  │  swappable               │       │                          │  │
│  └──────────────────────────┘       └──────────────────────────┘  │
│           │                                     │                  │
│           │          TRANSPORT LAYER            │                  │
│           │     (InProcess | UnixSocket         │                  │
│           │      | WebSocket | gRPC)            │                  │
│           └─────────────────────────────────────┘                  │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

---

## Message Protocol

### SurfaceEvent (Surface → Conductor)

```rust
pub enum SurfaceEvent {
    // User interaction
    UserMessage { content: String },
    UserTyping { partial_input: String },
    UserScrolled { offset: usize },
    UserCancelled,

    // Surface lifecycle
    Connected { capabilities: SurfaceCapabilities },
    Disconnected,
    Resized { width: u16, height: u16 },

    // Commands
    ClearHistory,
    ChangeModel { model: String },
    ExportConversation { format: ExportFormat },
}
```

### ConductorMessage (Conductor → Surface)

```rust
pub enum ConductorMessage {
    // Streaming response
    StreamStart { message_id: MessageId },
    Token { message_id: MessageId, text: String },
    StreamEnd { message_id: MessageId, metadata: ResponseMetadata },
    StreamError { error: String },

    // State updates
    Message { id: MessageId, role: MessageRole, content: String },
    AvatarMood { mood: AvatarMood },
    AvatarGesture { gesture: AvatarGesture },
    ModelChanged { model: String },

    // System
    Notify { level: NotifyLevel, message: String },
    SessionInfo { id: SessionId, created_at: Timestamp },
    Quit { message: Option<String> },
}
```

---

## Testing Separation

**REQUIRED Test Coverage**:

1. **Conductor runs without TUI**
   ```rust
   #[tokio::test]
   async fn test_conductor_standalone() {
       let conductor = Conductor::new(/* no surface */);
       // Should operate normally
   }
   ```

2. **TUI swaps transports**
   ```rust
   #[tokio::test]
   async fn test_tui_with_daemon() {
       let transport = UnixSocketTransport::connect("socket.sock").await;
       let mut tui = TUI::new(transport);
       // Should work identically to embedded mode
   }
   ```

3. **Multiple surfaces simultaneously**
   ```rust
   #[tokio::test]
   async fn test_multi_surface() {
       let conductor = Conductor::new_daemon();
       conductor.register_surface(tui_surface);
       conductor.register_surface(web_surface);
       conductor.register_surface(cli_surface);
       // All should receive state updates
   }
   ```

---

## Deployment Scenarios

### Scenario 1: Default (Embedded TUI)
```bash
./yollayah.sh
```
- TUI and Conductor in same process
- InProcess transport (channels)
- ~50 MB memory, instant startup

### Scenario 2: Daemon + TUI
```bash
# Terminal 1: Start daemon
yollayah-daemon --socket /tmp/yollayah.sock

# Terminal 2: Connect TUI
yollayah-tui --socket /tmp/yollayah.sock
```
- TUI and Conductor in separate processes
- Unix Socket transport (IPC)
- TUI can disconnect/reconnect without losing state

### Scenario 3: Daemon + Web UI
```bash
# Terminal 1: Start daemon
yollayah-daemon --http 127.0.0.1:8080

# Browser: Navigate to http://127.0.0.1:8080
```
- Web UI and Conductor in separate processes
- WebSocket transport
- Access from remote machines

### Scenario 4: Headless API
```bash
yollayah-daemon --api --no-tui
```
- No UI, just HTTP API
- Conductor only
- For integration with other tools

---

## Violations and Enforcement

**Any violation of separation is a CRITICAL BUG.**

### Common Violations

1. **TUI importing Conductor internals**
   ```rust
   // ❌ WRONG
   use conductor_core::session::Session;
   ```

2. **Conductor knowing about UI specifics**
   ```rust
   // ❌ WRONG
   impl Conductor {
       fn set_terminal_size(&mut self, w: u16, h: u16) { ... }
   }
   ```

3. **Business logic in TUI**
   ```rust
   // ❌ WRONG
   impl TUI {
       async fn call_llm(&mut self, prompt: &str) { ... }
   }
   ```

### Enforcement

1. **Compilation Test**: Conductor MUST compile without TUI dependency
   ```bash
   cargo build -p conductor-core --no-default-features
   ```

2. **Module Boundaries**: `pub(crate)` for internals, only message types are `pub`

3. **Code Review**: All PRs reviewed for separation violations

---

## Migration Path

**For existing violations:**

1. Identify coupling (TUI → Conductor direct calls)
2. Define message for the operation
3. Move logic from TUI to Conductor
4. TUI sends event, Conductor handles
5. Conductor sends result message
6. TUI updates display

**Example: Moving model selection from TUI to Conductor**

Before (❌ COUPLED):
```rust
// TUI directly changes Conductor internals
impl TUI {
    fn select_model(&mut self, model: String) {
        self.conductor.backend.set_model(model); // ← Direct access!
    }
}
```

After (✅ DECOUPLED):
```rust
// TUI sends event
impl TUI {
    fn select_model(&mut self, model: String) {
        self.conductor.send_event(SurfaceEvent::ChangeModel { model });
    }
}

// Conductor handles event
impl Conductor {
    async fn handle_change_model(&mut self, model: String) {
        self.backend.set_model(model).await;
        self.send(ConductorMessage::ModelChanged { model }).await;
    }
}
```

---

## Summary

| Requirement | Description | Enforcement |
|-------------|-------------|-------------|
| **Message-Only Communication** | All TUI↔Conductor via messages | Code review, module boundaries |
| **No Direct Dependencies** | TUI can't import Conductor internals | Compilation test |
| **Swappable Surfaces** | Any UI works with Conductor | Integration tests |
| **State in Conductor** | Business logic never in UI | Architecture review |
| **Transport Abstraction** | Works over channels, sockets, etc | Transport trait |

**Remember**: The TUI is a **view** of Conductor state. It displays, it doesn't decide.
