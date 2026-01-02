# Implementation Plan: Multi-Surface Conductor Architecture

**Created**: 2026-01-01
**Based on**: Specialist reviews (Architect, UX, Security, Testing)

This document details the implementation order, dependencies, and activities for completing the Conductor/UX split to support multiple simultaneous surfaces.

---

## Implementation Tracks

The work is organized into parallel tracks that can proceed independently where noted:

```
Track A: Daemon & Multi-Surface (Critical Path)
Track B: Transport Hardening (Security)
Track C: Message Protocol Enhancements (UX)
Track D: Testing & Chaos Engineering (Quality)
```

---

## Track A: Daemon & Multi-Surface Infrastructure

**Owner**: Core Architecture
**Goal**: Enable Conductor to run as standalone daemon serving multiple surfaces

### A1: Conductor Daemon Binary
**Priority**: Critical | **Effort**: 2-3 days | **Dependencies**: None

**Activity**: Create standalone conductor-daemon binary that runs as a system service.

**Implementation**:
1. Create `conductor/daemon/Cargo.toml` with dependencies:
   - conductor-core (workspace)
   - clap (CLI)
   - tokio (runtime)
   - tracing-subscriber (logging)

2. Create `conductor/daemon/src/main.rs`:
   - CLI args: `--socket-path`, `--config`, `--daemonize`, `--check`
   - Signal handlers for SIGTERM, SIGHUP
   - PID file in `$XDG_RUNTIME_DIR/ai-way/conductor.pid`

3. Create `conductor/daemon/src/daemon.rs`:
   - Main event loop using `ConductorTransport::accept()`
   - Spawn task per connection
   - Graceful shutdown coordination

**Outputs**:
- `conductor-daemon` binary that starts and listens on Unix socket
- Can run in foreground or daemonize
- Logs to stdout/file

---

### A2: Multi-Surface Conductor Refactor
**Priority**: Critical | **Effort**: 2 days | **Dependencies**: A1

**Activity**: Modify Conductor internals to support multiple concurrent surface connections.

**Implementation**:
1. In `conductor/core/src/conductor.rs`, replace:
   ```rust
   // OLD
   tx: mpsc::Sender<ConductorMessage>,
   surface_type: Option<SurfaceType>,
   surface_capabilities: Option<SurfaceCapabilities>,

   // NEW
   surfaces: Arc<RwLock<HashMap<ConnectionId, SurfaceHandle>>>,
   ```

2. Create `SurfaceHandle` struct:
   ```rust
   struct SurfaceHandle {
       tx: mpsc::Sender<ConductorMessage>,
       surface_type: SurfaceType,
       capabilities: SurfaceCapabilities,
       connected_at: Instant,
   }
   ```

3. Update `send()` to `send_to(connection_id)` and `broadcast()`

4. Update `handle_event()` to track which surface sent each event

**Outputs**:
- Conductor can track multiple surfaces
- Messages route to correct surface(s)
- Backward compatible with single-surface mode

---

### A3: Surface Registration & State Sync
**Priority**: High | **Effort**: 1-2 days | **Dependencies**: A2

**Activity**: Implement protocol for surfaces to register and receive current state.

**Implementation**:
1. Add `ConductorMessage::StateSnapshot`:
   ```rust
   StateSnapshot {
       session: SessionSnapshot,
       avatar: AvatarSnapshot,
       tasks: Vec<TaskSnapshot>,
       recent_messages: Vec<MessageSnapshot>,
   }
   ```

2. On `SurfaceEvent::Connected`:
   - Generate unique ConnectionId (random, not sequential)
   - Validate authentication token
   - Add to surfaces HashMap
   - Send StateSnapshot

3. Create snapshot serialization (limit size):
   - Last N messages (configurable, default 50)
   - All active tasks
   - Current avatar state

**Outputs**:
- Late-joining surfaces receive context
- ConnectionId assigned and tracked
- Auth token validated (placeholder for B3)

---

### A4: Launcher Script Update
**Priority**: High | **Effort**: 0.5 days | **Dependencies**: A1

**Activity**: Update yollayah.sh to manage daemon lifecycle.

**Implementation**:
```bash
# Check if daemon running
if conductor-daemon --check 2>/dev/null; then
    echo "Connecting to existing Conductor..."
else
    echo "Starting Conductor daemon..."
    conductor-daemon --daemonize
    sleep 0.5  # Wait for socket
fi

# Launch TUI connected to daemon
CONDUCTOR_TRANSPORT=unix yollayah-tui

# Option: --embedded to use old behavior
if [[ "$1" == "--embedded" ]]; then
    CONDUCTOR_TRANSPORT=inprocess yollayah-tui
fi
```

**Outputs**:
- `yollayah.sh` starts daemon if needed
- `yollayah.sh --embedded` for old single-process mode
- `yollayah.sh --stop` to stop daemon

---

### A5: Transport Factory Pattern
**Priority**: Medium | **Effort**: 1 day | **Dependencies**: None (parallel)

**Activity**: Abstract transport creation from ConductorClient.

**Implementation**:
1. Create `conductor/core/src/transport/factory.rs`:
   ```rust
   pub fn create_surface_transport(
       config: &TransportConfig
   ) -> Result<Box<dyn SurfaceTransport>, TransportError> {
       match config.transport_type {
           TransportType::InProcess => /* ... */,
           TransportType::UnixSocket { ref path } => /* ... */,
           TransportType::WebSocket { ref addr } => /* ... */,
       }
   }
   ```

2. Update `ConductorClient` in TUI:
   - Remove `ClientMode` enum
   - Use `Box<dyn SurfaceTransport>`
   - Keep embedded Conductor as optional field

**Outputs**:
- Transport selection via config, not code
- Easy to add new transports
- Cleaner ConductorClient

---

## Track B: Transport Security Hardening

**Owner**: Security
**Goal**: Harden IPC layer before multi-surface production use

### B1: Frame Integrity Verification
**Priority**: High | **Effort**: 0.5 days | **Dependencies**: None

**Activity**: Add checksum to frame protocol to detect corruption.

**Implementation**:
1. Modify frame format in `transport/frame.rs`:
   ```
   +----------------+----------------+------------------+
   | Length (4)     | Checksum (4)   | JSON Payload     |
   | big-endian u32 | XXH3 (32-bit)  | variable         |
   +----------------+----------------+------------------+
   ```

2. Use xxhash-rust crate for XXH3 (fast, no crypto needed)

3. Verify checksum on decode, return `FrameError::ChecksumMismatch`

**Outputs**:
- Frames validated on receive
- Corruption detected and logged
- Backward-incompatible (version bump)

---

### B2: Random ConnectionId
**Priority**: Medium | **Effort**: 0.5 days | **Dependencies**: None

**Activity**: Replace sequential IDs with cryptographically random IDs.

**Implementation**:
1. In `transport/traits.rs`:
   ```rust
   impl ConnectionId {
       pub fn new() -> Self {
           use rand::Rng;
           let bytes: [u8; 16] = rand::thread_rng().gen();
           Self(format!("conn_{}", hex::encode(bytes)))
       }
   }
   ```

2. Add `rand` and `hex` to dependencies

**Outputs**:
- Unpredictable connection IDs
- No enumeration attacks

---

### B3: Surface Authentication Tokens
**Priority**: High | **Effort**: 1 day | **Dependencies**: A2

**Activity**: Implement token-based surface authentication.

**Implementation**:
1. Generate session token on daemon start:
   ```rust
   let session_token = generate_secure_token();
   write_token_file(&token_path, &session_token)?;
   ```

2. Token file: `$XDG_RUNTIME_DIR/ai-way/session.token` (mode 0600)

3. Surface reads token, sends in `SurfaceEvent::Connected.auth_token`

4. Conductor validates token matches

5. Reject with `HandshakeAck { accepted: false, rejection_reason: "Invalid token" }`

**Outputs**:
- Only authorized surfaces can connect
- Token rotates per daemon start
- Token file secured by permissions

---

### B4: macOS Peer Validation
**Priority**: Medium | **Effort**: 0.5 days | **Dependencies**: None

**Activity**: Implement getpeereid() for macOS.

**Implementation**:
```rust
#[cfg(target_os = "macos")]
fn validate_peer(stream: &UnixStream) -> Result<(), TransportError> {
    use std::os::unix::io::AsRawFd;
    let fd = stream.as_raw_fd();
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;

    let ret = unsafe { libc::getpeereid(fd, &mut uid, &mut gid) };
    if ret != 0 {
        return Err(TransportError::AuthenticationFailed(
            "getpeereid failed".into()
        ));
    }

    let my_uid = unsafe { libc::getuid() };
    if uid != my_uid {
        return Err(TransportError::AuthenticationFailed(
            format!("Peer UID mismatch")
        ));
    }
    Ok(())
}
```

**Outputs**:
- Unix socket security on macOS
- Parity with Linux implementation

---

### B5: Transport Rate Limiting
**Priority**: Medium | **Effort**: 1 day | **Dependencies**: A2

**Activity**: Add rate limiting at transport layer.

**Implementation**:
1. Limit concurrent connections per UID (default: 10)
2. Limit messages per second per connection (default: 100)
3. Limit total connections (default: 50)

4. Use token bucket algorithm:
   ```rust
   struct RateLimiter {
       tokens: AtomicU32,
       last_refill: AtomicU64,
       rate: u32,  // tokens per second
   }
   ```

**Outputs**:
- DoS protection at transport layer
- Configurable limits
- Logged when limits hit

---

## Track C: Message Protocol Enhancements

**Owner**: UX
**Goal**: Enable consistent, rich experience across all surface types

### C1: ContentType for Messages
**Priority**: High | **Effort**: 0.5 days | **Dependencies**: None

**Activity**: Add content type hints to messages.

**Implementation**:
1. Add enum in `messages.rs`:
   ```rust
   #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
   pub enum ContentType {
       Plain,
       Markdown,
       Code { language: Option<String> },
       Error,
       System,
       Quote,
   }
   ```

2. Add to `ConductorMessage::Message`:
   ```rust
   Message {
       message_id: MessageId,
       role: MessageRole,
       content: String,
       content_type: ContentType,  // NEW
   }
   ```

3. Default to `ContentType::Markdown` for assistant messages

**Outputs**:
- Surfaces know how to render content
- Consistent markdown/code handling

---

### C2: LayoutHint Messages
**Priority**: Medium | **Effort**: 0.5 days | **Dependencies**: None

**Activity**: Add layout orchestration messages.

**Implementation**:
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LayoutDirective {
    ShowPanel { panel: PanelId },
    HidePanel { panel: PanelId },
    FocusInput,
    ScrollToMessage { message_id: MessageId },
    ToggleDeveloperMode,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PanelId {
    Tasks,
    Developer,
    Settings,
    History,
}

ConductorMessage::LayoutHint {
    directive: LayoutDirective,
}
```

**Outputs**:
- Conductor can orchestrate UI layout
- Consistent panel behavior across surfaces

---

### C3: VoiceState Messages
**Priority**: Medium | **Effort**: 1 day | **Dependencies**: None

**Activity**: Add voice-first interaction support.

**Implementation**:
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VoiceState {
    Idle,
    Listening,
    Processing,
    Speaking { text: String },
    Muted,
    Error { reason: String },
}

ConductorMessage::VoiceState {
    state: VoiceState,
}

ConductorMessage::Transcription {
    text: String,
    confidence: f32,
    is_final: bool,
}

// Surface can send:
SurfaceEvent::VoiceInput {
    event_id: EventId,
    audio_data: Vec<u8>,
    format: AudioFormat,
}
```

**Outputs**:
- Voice interaction state synchronized
- Ready for voice-enabled surfaces

---

### C4: Enhanced SurfaceCapabilities
**Priority**: Low | **Effort**: 0.5 days | **Dependencies**: None

**Activity**: Expand capabilities for future surfaces.

**Implementation**:
```rust
// Add to SurfaceCapabilities:
pub voice_input: bool,
pub voice_output: bool,
pub haptic_feedback: bool,
pub notifications: bool,
pub locale: String,
pub reduced_motion: bool,
pub high_contrast: bool,
```

**Outputs**:
- Surfaces self-describe fully
- Conductor adapts behavior to capabilities

---

## Track D: Testing & Chaos Engineering

**Owner**: QA
**Goal**: Ensure reliability under edge cases and failures

### D1: Edge Case Integration Tests
**Priority**: High | **Effort**: 2 days | **Dependencies**: A2

**Activity**: Add tests for identified edge cases.

**Tests to implement**:
```rust
// In tui/tests/integration_test.rs

#[tokio::test]
async fn test_surface_disconnect_mid_stream() {
    // Start streaming, drop receiver, verify cleanup
}

#[tokio::test]
async fn test_conductor_crash_recovery() {
    // Connect, kill conductor, verify reconnect
}

#[tokio::test]
async fn test_concurrent_surface_connections() {
    // Two surfaces connect simultaneously
}

#[tokio::test]
async fn test_rapid_message_sending() {
    // 50 messages in quick succession
}

#[tokio::test]
async fn test_channel_backpressure() {
    // Slow consumer, fast producer
}
```

**Outputs**:
- Coverage for all identified edge cases
- Regression prevention

---

### D2: Chaos Engineering Infrastructure
**Priority**: Medium | **Effort**: 1-2 days | **Dependencies**: D1

**Activity**: Build reusable chaos test framework.

**Implementation**:
1. Create `tui/tests/chaos/mod.rs`:
   ```rust
   pub struct ChaosBackend {
       inner: Box<dyn LlmBackend>,
       failure_mode: FailureMode,
       trigger_after: usize,
   }

   pub enum FailureMode {
       Hang,
       PartialTokens(usize),
       ErrorAfterTokens(usize),
       GarbageData,
   }
   ```

2. Create chaos transport wrappers:
   ```rust
   pub struct ChaosTransport {
       inner: Box<dyn SurfaceTransport>,
       drop_probability: f32,
       corrupt_probability: f32,
   }
   ```

**Outputs**:
- Reusable chaos injection
- Configurable failure modes
- Easy to add new chaos scenarios

---

### D3: Chaos Test Suite
**Priority**: Medium | **Effort**: 1-2 days | **Dependencies**: D2

**Activity**: Implement chaos tests.

**Tests**:
```rust
#[tokio::test]
async fn chaos_socket_close_mid_frame() { ... }

#[tokio::test]
async fn chaos_backend_hang() { ... }

#[tokio::test]
async fn chaos_backend_partial_tokens() { ... }

#[tokio::test]
async fn chaos_session_memory_pressure() { ... }

#[tokio::test]
async fn chaos_concurrent_pruning() { ... }
```

**Outputs**:
- Verified behavior under chaos
- Confidence in production resilience

---

## Dependency Graph

```
                    ┌─────────────────────────────────────────┐
                    │           Track A (Critical)             │
                    └─────────────────────────────────────────┘
                                       │
    ┌──────────────────────────────────┼──────────────────────────────────┐
    │                                  │                                  │
    ▼                                  ▼                                  ▼
┌───────┐                         ┌───────┐                          ┌───────┐
│  A1   │                         │  A5   │ (parallel)               │  B1   │ (parallel)
│Daemon │                         │Factory│                          │Frame  │
└───┬───┘                         └───────┘                          │Integ. │
    │                                  │                             └───────┘
    ▼                                  │
┌───────┐                              │
│  A2   │◄─────────────────────────────┘
│Multi- │
│Surface│
└───┬───┘
    │
    ├───────────────────┬───────────────────┐
    │                   │                   │
    ▼                   ▼                   ▼
┌───────┐          ┌───────┐          ┌───────┐
│  A3   │          │  A4   │          │  B3   │
│Regist.│          │Launch │          │ Auth  │
│& Sync │          │Script │          │Tokens │
└───────┘          └───────┘          └───┬───┘
                                          │
                                          ▼
                                     ┌───────┐
                                     │  B5   │
                                     │ Rate  │
                                     │ Limit │
                                     └───────┘

┌─────────────────────────────────────────┐
│         Track C (Parallel)               │
└─────────────────────────────────────────┘
    C1, C2, C3, C4 - No dependencies, can run in parallel

┌─────────────────────────────────────────┐
│         Track D (After A2)               │
└─────────────────────────────────────────┘
    D1 ──► D2 ──► D3
```

---

## Implementation Schedule

### Week 1: Foundation

| Day | Tasks | Owner |
|-----|-------|-------|
| 1-2 | A1: Conductor daemon binary | Arch |
| 1 | B1: Frame integrity | Sec |
| 1 | B2: Random ConnectionId | Sec |
| 1 | C1: ContentType | UX |
| 2 | A5: Transport factory | Arch |
| 2 | B4: macOS getpeereid | Sec |

### Week 2: Multi-Surface Core

| Day | Tasks | Owner |
|-----|-------|-------|
| 1-2 | A2: Multi-surface conductor refactor | Arch |
| 1 | C2: LayoutHint messages | UX |
| 2 | C3: VoiceState messages | UX |
| 2 | A4: Launcher script | Arch |

### Week 3: Security & Testing

| Day | Tasks | Owner |
|-----|-------|-------|
| 1 | A3: Registration & state sync | Arch |
| 1 | B3: Surface auth tokens | Sec |
| 2 | B5: Transport rate limiting | Sec |
| 2-3 | D1: Edge case tests | QA |

### Week 4: Hardening & Polish

| Day | Tasks | Owner |
|-----|-------|-------|
| 1-2 | D2: Chaos infrastructure | QA |
| 2-3 | D3: Chaos test suite | QA |
| 3 | C4: Enhanced capabilities | UX |
| 3 | Documentation updates | All |

---

## Success Criteria

### Phase 4 Complete When:
- [ ] Conductor runs as standalone daemon
- [ ] Multiple TUI instances can connect simultaneously
- [ ] Late-joining surfaces receive state snapshot
- [ ] All security hardening items (B1-B5) complete
- [ ] Edge case tests passing (D1)

### Phase 5 Complete When:
- [ ] TOML config file support
- [ ] yollayah.sh manages daemon lifecycle
- [ ] Health check endpoint operational
- [ ] Chaos tests passing (D3)
- [ ] Documentation complete

---

## Risk Register

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Multi-surface message ordering | Medium | High | Sequence numbers in messages |
| Daemon startup race | Low | Medium | Socket existence as gate |
| Breaking TUI regression | Medium | High | Feature flag for embedded mode |
| Session snapshot too large | Low | Medium | Limit message history in snapshot |
| Auth token file permissions | Low | High | Explicit chmod, verify on read |

---

## Post-Completion: WebSocket Preparation

After Phase 4/5 complete, prerequisites for WebSocket:

1. **Security Design Document** - TLS requirements, auth flow, threat model
2. **External Security Review** - Before production remote access
3. **Proxy Architecture Decision** - Native TLS vs. reverse proxy
4. **Capability Filtering** - Which capabilities available remotely

---

**Document Owner**: Project Lead
**Last Updated**: 2026-01-01
