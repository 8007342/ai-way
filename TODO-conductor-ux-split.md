# TODO: Conductor/UX Split Refactor

Tracking the separation of TUI and Conductor into independent components with IPC.

## Goal

Enable multiple surfaces (TUI, GUI, WebUI, iPad, TV, etc.) to connect to a single Conductor instance via configurable transport (Unix Sockets default, WebSockets optional).

## Architecture

```
yollayah.sh
    |
    +-- Conductor (daemon) <-- Unix Socket/WebSocket --> Surface(s)
    |       |
    |       +-- Agent orchestration
    |       +-- Task management
    |       +-- LLM backend
    |       +-- Session state
    |
    +-- TUI Surface (default)
    +-- GUI Surface (optional)
    +-- WebUI Surface (optional)
    +-- Remote surfaces (iPad, TV, etc.)
```

## Progress

### Phase 1: Core Infrastructure (DONE)
- [x] Create conductor-core crate (UI-agnostic)
- [x] Define ConductorMessage enum (Conductor -> Surface)
- [x] Define SurfaceEvent enum (Surface -> Conductor)
- [x] Avatar state machine (moods, sizes, positions, gestures)
- [x] Task management framework
- [x] Session management
- [x] Security validation

### Phase 2: TUI as Thin Client (IN PROGRESS - ~85%)
- [x] ConductorClient wrapper for TUI
- [x] DisplayState types (derived from ConductorMessages)
- [x] Activity overlay system (11 activity types)
- [x] Task panel widget
- [x] Goodbye message system
- [x] App refactor to use ConductorClient
- [x] Both crates compile successfully
- [x] Integration test of message flow (11 tests passing)
- [ ] Full event loop completion (handle all SurfaceEvents)
- [ ] Developer mode (PJ) debug panel

### Phase 3: IPC Transport Layer (IN PROGRESS - ~85%)
- [x] Abstract transport trait (SurfaceTransport/ConductorTransport)
- [x] Unix Socket server transport (ConductorTransport impl)
- [x] Unix Socket client transport (SurfaceTransport impl)
- [x] InProcess transport (backwards compatible embedded mode)
- [x] Transport configuration types (TransportConfig, TransportType)
- [x] Length-prefixed JSON frame protocol
- [x] Connection handshake messages (Handshake, HandshakeAck, Ping, Pong)
- [x] SO_PEERCRED validation on Linux for Unix sockets
- [x] Reconnection handling (try_reconnect with exponential backoff)
- [x] ConductorClient refactor to use transport abstraction (ClientMode enum)
- [ ] macOS getpeereid() peer validation
- [x] Frame integrity verification (CRC32 checksum) ✓ Sprint 2
- [ ] WebSocket transport implementation (optional feature)
- [ ] yollayah.sh updates to launch Conductor as separate process

### Phase 4: Process Separation (PENDING - Detailed Plan Below)

#### 4.1 Conductor Daemon Binary [Priority: Critical] ✓ DONE Sprint 2
- [x] Create `conductor/daemon/` crate structure
- [x] Entry point with CLI args (clap)
- [x] Daemon loop with signal handling (SIGTERM, SIGHUP)
- [x] PID file management
- [x] Multi-connection accept loop

#### 4.2 Multi-Surface Conductor Refactor [Priority: Critical] ✓ DONE Sprint 3
- [x] Replace single `tx` channel with `HashMap<ConnectionId, SurfaceHandle>`
- [x] Implement `SurfaceHandle` struct (tx, type, capabilities)
- [x] Add `Arc<RwLock<>>` for concurrent surface access
- [x] Implement surface-specific message routing
  - **Completed**: Created `conductor/core/src/surface_registry.rs` with:
    - `ConnectionId` (unique, atomic counter)
    - `SurfaceHandle` (tx, type, capabilities, connected_at, metadata)
    - `SurfaceRegistry` (thread-safe HashMap with RwLock)
    - Methods: broadcast(), send_to(), send_to_capable(), cleanup_disconnected()
    - Updated Conductor with new_with_registry() and handle_event_from()
    - Updated daemon server.rs for per-connection channels

#### 4.3 Surface Registration Protocol [Priority: High] ✓ DONE Sprint 6
- [x] Extend handshake with capability declaration ✓ Sprint 5
- [x] Assign and validate ConnectionId ✓ Sprint 5
- [x] Send current state snapshot on connect ✓ Sprint 6
- [ ] Implement surface authentication tokens (Sprint 6 - tracked as B3)

#### 4.4 Transport Factory Pattern [Priority: Medium]
- [ ] Create `transport/factory.rs` module
- [ ] `create_surface_transport(config)` function
- [ ] Abstract ConductorClient from specific transports

#### 4.5 ConductorClient Refactor [Priority: Medium]
- [ ] Replace `ClientMode` enum with `Box<dyn SurfaceTransport>`
- [ ] Maintain backward-compat embedded mode option

#### 4.6 State Snapshot for Late-Joining Surfaces [Priority: Medium] ✓ DONE Sprint 6
- [x] Add `ConductorMessage::StateSnapshot` variant ✓ Sprint 5
- [x] Include session, avatar, tasks, recent messages ✓ Sprint 6
- [x] Limit snapshot size to prevent overwhelming new surfaces ✓ Sprint 6
  - **Completed**: `Conductor::create_state_snapshot(max_messages)` method
  - Sends StateSnapshot on handshake acceptance
  - Configurable message limit (default 20)
  - Unit tests in conductor.rs

### Phase 5: Configuration & Polish (PENDING - Detailed Plan Below)

#### 5.1 TOML Configuration File [Priority: High]
- [ ] Create `config/` module in conductor-core
- [ ] Support `~/.config/ai-way/conductor.toml`
- [ ] Priority: CLI > env > file > defaults
- [ ] Document all configuration options

#### 5.2 Launcher Script Update [Priority: High] ✓ DONE Sprint 3
- [x] Check for running daemon, connect if exists
- [x] Start daemon if not running (--daemonize)
- [x] Pass socket path to TUI
- [x] Add restart/stop commands
  - **Completed**: Updated `yollayah.sh` with:
    - Commands: start, daemon, connect, stop, restart, status
    - Functions: check_daemon(), start_daemon(), stop_daemon(), show_status(), connect_tui()
    - Environment variables: CONDUCTOR_SOCKET, CONDUCTOR_PID, AI_WAY_LOG
    - Graceful shutdown (SIGTERM with fallback to SIGKILL)
    - Socket/PID file management

#### 5.3 Heartbeat Enforcement [Priority: Medium]
- [ ] Watchdog task per connection
- [ ] Configurable heartbeat interval
- [ ] Disconnect unresponsive surfaces
- [ ] Log connection health metrics

#### 5.4 Health Check Endpoint [Priority: Medium]
- [ ] TCP or Unix socket health probe
- [ ] Return: status, connected surfaces, session info
- [ ] Integration with systemd socket activation (optional)

#### 5.5 Performance Profiling [Priority: Low]
- [ ] Optional metrics collection (feature flag)
- [ ] Message latency histograms
- [ ] Queue depth monitoring
- [ ] Streaming token rates

---

## Specialist Reviews (2026-01-01)

### Solutions Architect Assessment

**Overall**: Architecture is sound with solid foundations. Key gaps for Phase 4:

| Gap | Impact | Resolution |
|-----|--------|------------|
| No daemon binary | Blocks multi-surface | Create conductor/daemon crate |
| Single tx channel in Conductor | Cannot route to multiple surfaces | Refactor to HashMap<ConnectionId, SurfaceHandle> |
| Session-surface binding undefined | Ambiguous multi-surface behavior | Define broadcast model initially |
| Late-joining surfaces miss state | Poor UX for new connections | Implement StateSnapshot message |
| ConductorClient knows transports | Tight coupling | Use transport factory pattern |

### UX Designer Assessment

**Overall**: Clean separation enables consistent multi-surface UX. Key gaps:

| Gap | Impact | Resolution |
|-----|--------|------------|
| No ContentType hints in messages | Inconsistent rendering (markdown vs plain) | Add ContentType enum to messages |
| No LayoutDirective messages | Cannot orchestrate panel visibility | Add LayoutHint message variant |
| Missing VoiceState messages | Voice-first goal blocked | Add voice state/transcription messages |
| No theme synchronization | Inconsistent colors across surfaces | Add ThemeConfig message |
| Animations lack structure | Surface-dependent avatar behavior | Add Animation struct with keyframes |
| Missing internationalization | English-only UI strings | Add LocalizedText message type |

### Ethical Hacker Assessment

**Overall**: Solid Unix socket security. Critical gaps for multi-surface:

| Vulnerability | Severity | Resolution |
|---------------|----------|------------|
| No auth beyond UID matching | High | Implement surface authentication tokens |
| No session isolation | High | Isolate session per ConnectionId |
| Frame lacks integrity check | High | Add CRC32/XXH3 checksum |
| macOS missing getpeereid | Medium | Implement platform-specific validation |
| Sequential ConnectionId | Medium | Use cryptographically random IDs |
| auth_token field unused | Medium | Implement or remove |
| Transport has no rate limiting | Medium | Add connection/message rate limits |

### Mad Scientist Assessment

**Overall**: Happy-path testing only. Edge cases untested:

| Edge Case | Risk | Test Needed |
|-----------|------|-------------|
| Surface disconnect mid-stream | Resource leak, hung state | test_surface_disconnect_mid_stream |
| Conductor crash recovery | Lost session, confused surfaces | test_conductor_crash_recovery |
| Concurrent surface connections | Race conditions, overwrites | test_concurrent_surface_connections |
| Rapid message sending | Ordering issues, drops | test_rapid_message_sending |
| Channel buffer exhaustion | Deadlock | test_channel_backpressure |

**Experimental Ideas** (feasibility noted):
- Surface hot-swap during session (MEDIUM-HIGH)
- Conductor state persistence (HIGH - Session already serializable)
- Multi-Conductor federation (LOW - significant architecture change)
- Speculative response streaming (MEDIUM)

---

## Security Hardening Checklist

Before Production:
- [ ] P1.1: Surface authentication tokens
- [ ] P1.2: Session isolation per ConnectionId
- [ ] P1.3: Frame integrity (checksum)
- [ ] P1.4: macOS getpeereid implementation
- [ ] P1.5: Random ConnectionId format

Before WebSocket:
- [ ] P2.1: WebSocket security design document
- [ ] P2.2: Capability-based access system
- [ ] P2.3: Transport-layer rate limiting
- [ ] P2.4: Security audit logging

---

## Message Protocol Enhancements

### New ConductorMessage Variants Needed

```rust
// Content type for rendering hints
ContentType { Plain, Markdown, Code { lang }, Error, System, Quote }

// Layout orchestration
LayoutHint { directive: LayoutDirective }

// Voice-first support
VoiceState { state: VoiceState }
Transcription { text, confidence, is_final }

// Theme synchronization
Theme { theme: ThemeConfig }

// Input feedback
InputState { state: InputState }

// Late-joining surface support
StateSnapshot { session, avatar, tasks, messages }

// Enhanced errors
Error { code, message, details, recovery: Vec<RecoveryAction> }
```

### Enhanced SurfaceCapabilities

```rust
// Add to existing struct:
voice_input: bool,
voice_output: bool,
haptic_feedback: bool,
notifications: bool,
locale: String,
reduced_motion: bool,
high_contrast: bool,
```

---

## Chaos Engineering Tests to Add

| Test | Category | Priority |
|------|----------|----------|
| chaos_socket_close_mid_frame | Transport | High |
| chaos_backend_hang | LLM | High |
| chaos_backend_partial_tokens | LLM | High |
| chaos_session_memory_pressure | Session | Medium |
| chaos_concurrent_pruning | Session | Medium |
| chaos_state_double_transition | State | Medium |
| chaos_shutdown_during_warmup | Lifecycle | Medium |

---

## Feature Creep Items (Do Later)
_Items discovered during refactor that should NOT block this work:_

- [ ] Remote surface authentication (security review needed)
- [ ] Multi-Conductor federation (multiple machines)
- [ ] Surface hot-swap during session
- [ ] Conductor state persistence across restarts
- [ ] Speculative response streaming
- [ ] Avatar autonomy mode

---

## Commits (Stable Checkpoints)

| Commit | Description | Date |
|--------|-------------|------|
| 493e221 | Phase 2 checkpoint: Conductor-core and TUI thin client | 2025-12-31 |
| b24333e | Phase 3 transport layer: Unix socket IPC infrastructure | 2025-12-31 |
| d73a6f3 | Add integration test suite and documentation | 2026-01-01 |

---

## Notes

- Original plan used Python Textual; switched to Rust ratatui for performance
- The TUI already has embedded Conductor via ConductorClient
- Need to extract to separate process with IPC for true independence
- Unix Sockets preferred for local security (no network exposure)
- WebSockets for remote surfaces (with auth)
- Session follows broadcast model (all surfaces see all messages initially)
- Specialist reviews conducted 2026-01-01

---

**Last Updated**: 2026-01-02 (Sprint 6 - See TODO-epic-2026Q1-multi-surface.md for Epic tracking)
