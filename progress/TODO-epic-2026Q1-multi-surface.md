# Epic: Multi-Surface Architecture

> Enable Conductor to serve multiple simultaneous surfaces (TUI, GUI, WebUI, mobile) via daemon architecture with secure transport layer.

## Status

- **Phase**: Execution
- **Started**: 2025-12-31
- **Target Completion**: 2026-Q1 (Sprint 8)
- **Sprints Completed**: 6 (Core infrastructure ~95% complete, security hardening done)

## Overview

The Multi-Surface Architecture epic transforms ai-way from a single-process TUI application into a client-server architecture where:

1. **Conductor Daemon** - Runs as a persistent background process managing state, LLM backends, and agent orchestration
2. **Thin Surfaces** - TUI (and future GUI/Web/Mobile) are rendering layers that connect via IPC
3. **Secure Transport** - Unix sockets for local connections, WebSocket (future) for remote
4. **Multi-Connection Support** - Multiple surfaces can connect simultaneously with state synchronization

This architecture enables powerful use cases:
- Multiple terminals showing the same conversation
- Desktop GUI alongside TUI
- Remote access from mobile devices
- Session persistence across surface disconnects

## Planning Team Sign-off

| Role | Name/ID | Date | Notes |
|------|---------|------|-------|
| Architect | - | 2026-01-01 | Multi-surface conductor design complete |
| UX Specialist | - | 2026-01-01 | ContentType, LayoutHint messages designed |
| Lawyer | - | 2026-01-01 | No concerns with daemon architecture |
| QA | - | 2026-01-01 | Integration tests in tui/tests/ |
| Security Specialist | - | 2026-01-01 | Unix socket security, peer validation |
| Backend Developer | - | 2026-01-02 | SurfaceRegistry implemented |
| TUI Developer | - | 2026-01-01 | ConductorClient transport abstraction |
| DevOps | - | 2026-01-02 | yollayah.sh launcher updated |

## Security Considerations

- [x] Threat model reviewed (see TODO-conductor-ux-split.md Ethical Hacker Assessment)
- [x] Unix socket permissions (0o600)
- [x] SO_PEERCRED peer validation (Linux)
- [x] Frame integrity (CRC32 checksum)
- [x] Random ConnectionId (H-001) ✓ Sprint 6
- [x] Connection pool reuse (H-002) ✓ Sprint 6
- [ ] macOS getpeereid() implementation (B4)
- [ ] Surface authentication tokens (B3)
- [ ] Transport rate limiting (B5)

### Security Invariants

| Invariant | Status | Notes |
|-----------|--------|-------|
| Socket file permissions 0o600 | Implemented | Owner-only access |
| Peer UID validation | Linux only | macOS pending (B4) |
| Frame checksum verification | Implemented | CRC32 in transport layer |
| Session isolation | Partial | ConnectionId scoping in SurfaceRegistry |
| Auth tokens | Pending | B3 in Sprint 6 |

### Identified Threats (from H-001, H-002, M-001)

1. **Sequential ConnectionId (H-001)** - ✓ RESOLVED Sprint 6: UUID v4 (122 bits entropy)
2. **Connection pool reuse (H-002)** - ✓ RESOLVED Sprint 6: Arc<Self> + mpsc channel
3. **Unused auth_token (M-001)** - Implement or remove (Sprint 8)

## Test Strategy

### Unit Tests

- [x] ConductorMessage serialization (conductor-core)
- [x] SurfaceEvent serialization (conductor-core)
- [x] Frame encoding/decoding (transport)
- [x] SurfaceRegistry operations (conductor-core)
- [x] StateSnapshot serialization (A3) ✓ Sprint 6
- [ ] Authentication token validation (B3)

### Integration Tests

| Test | Status | File |
|------|--------|------|
| test_conductor_startup_and_greeting | Passing | tui/tests/integration_test.rs |
| test_user_message_and_response | Passing | tui/tests/integration_test.rs |
| test_multi_turn_conversation | Passing | tui/tests/integration_test.rs |
| test_avatar_messages_processed | Passing | tui/tests/integration_test.rs |
| test_graceful_shutdown | Passing | tui/tests/integration_test.rs |
| test_exit_during_streaming | Passing | tui/tests/integration_test.rs |
| test_channel_backpressure | Passing | tui/tests/integration_test.rs |
| test_concurrent_surface_connections | Pending | D1 |
| test_surface_disconnect_mid_stream | Pending | D1 |
| test_late_joining_state_snapshot | Pending | D1 |

### Chaos Tests (Planned)

- [ ] chaos_socket_close_mid_frame
- [ ] chaos_backend_hang
- [ ] chaos_session_memory_pressure
- [ ] chaos_concurrent_pruning

## Sprint Plan

### Sprint 6: Security Hardening and State Sync [COMPLETE]

**Theme**: Production-ready security and late-joining surface support

- [x] **4.3**: Surface Registration Protocol (extend) ✓
  - StateSnapshot message implementation ✓
  - StateSnapshot sent on successful handshake ✓
  - ConnectionId assignment ✓
  - Capability-based message filtering ✓
  - auth_token validation (deferred to B3)
- [x] **H-002**: Connection Pool Reuse ✓
  - Arc<Self> pattern for shared ownership
  - mpsc::UnboundedChannel for async connection returns
  - RAII PooledConnection with Deref impl
  - Idle timeout cleanup (configurable)
  - >90% connection reuse ratio achieved
  - scenario_7_connection_pool test re-enabled and passing
- [x] **H-001**: Random ConnectionId ✓
  - Replaced sequential AtomicU64 with UUID v4
  - 122 bits of cryptographic randomness
  - Connection hijacking prevention
- [ ] **B3**: Surface Authentication Tokens (deferred to Sprint 7)
  - Generate session token on daemon start
  - Token file in $XDG_RUNTIME_DIR/ai-way/session.token
  - Validate in handshake

**Dependencies**: Phase 3 transport layer complete

**Exit Criteria**: ✓ All critical items met
- ✓ Late-joining surfaces receive full state snapshot
- ✓ Connection pool properly reuses connections
- ✓ All HIGH security findings (H-001, H-002) addressed
- Partial: Authentication tokens deferred to Sprint 7

### Sprint 7: Transport Hardening and Rate Limiting

**Theme**: DoS prevention and cross-platform support

- [ ] **B4**: macOS getpeereid() Implementation
  - Platform-specific peer validation
  - Parity with Linux SO_PEERCRED
- [ ] **B5**: Transport Rate Limiting
  - Per-connection message rate limits
  - Per-UID connection limits
  - Token bucket implementation
- [ ] **5.3**: Heartbeat Enforcement
  - Watchdog task per connection
  - Configurable heartbeat interval
  - Disconnect unresponsive surfaces
- [ ] **5.4**: Health Check Endpoint
  - TCP or Unix socket probe
  - Return: status, connected surfaces, session info

**Dependencies**: Sprint 6 (H-002, B3)

**Exit Criteria**:
- macOS peer validation working
- Rate limiting prevents DoS scenarios
- Unhealthy connections automatically cleaned up
- Health check operational for monitoring

### Sprint 8: Configuration and Stabilization

**Theme**: Production readiness

- [ ] **5.1**: TOML Configuration File
  - Create config/ module in conductor-core
  - Support ~/.config/ai-way/conductor.toml
  - Priority: CLI > env > file > defaults
- [ ] **A5**: Transport Factory Pattern (if not done)
  - Abstract transport creation from ConductorClient
  - Easy addition of new transports
- [ ] **M-001**: auth_token Resolution
  - Implement proper validation OR
  - Remove unused field
- [ ] Enable all chaos tests (D2, D3)
- [ ] Documentation completion
  - Update launcher documentation
  - Add multi-surface architecture guide
  - Security best practices

**Dependencies**: Sprint 7 (Rate limiting, health checks)

**Exit Criteria**:
- Configuration via TOML file works
- All security findings resolved
- Chaos tests pass
- Documentation complete

## Progress Log

### Sprint 6 (2026-01-02) - Security Hardening Complete

**Completed**:
- **H-001: Random ConnectionId**
  - Replaced AtomicU64 sequential counter with UUID v4
  - 122 bits of cryptographic randomness
  - Updated surface_registry.rs and all tests
  - Connection hijacking prevention achieved

- **H-002: Connection Pool Reuse**
  - Rewrote connection_pool.rs with Arc<Self> pattern
  - mpsc::UnboundedChannel for async connection returns from sync Drop
  - RAII PooledConnection with Deref impl for transparent usage
  - Configurable idle timeout with cleanup task
  - Re-enabled scenario_7_connection_pool test (now passing)
  - >90% connection reuse ratio achieved

- **4.3: State Snapshot (Extended)**
  - `Conductor::create_state_snapshot(max_messages)` method
  - `StateSnapshot` sent on successful handshake
  - Includes: conversation history, avatar state, session metadata
  - Configurable message limit (default 20, prevents huge payloads)
  - 4 unit tests in conductor.rs

**Security Impact**:
- All HIGH findings (H-001, H-002) resolved
- Connection hijacking now requires guessing 2^122 UUID
- DoS via connection exhaustion mitigated by pool reuse

### Sprint 5 (2026-01-02) - Multi-Surface Infrastructure

**Completed**:
- 4.1: Conductor daemon binary (clap CLI, signal handling, PID file)
- 4.2: SurfaceRegistry with HashMap<ConnectionId, SurfaceHandle>
- 5.2: yollayah.sh launcher (start, daemon, connect, stop, restart, status)
- Frame integrity verification (CRC32)
- Security findings documentation (TODO-security-findings.md)
- Disabled tests tracking (TODO-disabled-tests.md)

**Discoveries**:
- Connection pool needs Arc refactor (H-002)
- auth_token field unused (M-001)
- macOS peer validation missing (B4)

### Sprint 3 (2025-12-31) - Transport Layer

**Completed**:
- Unix socket server/client transport
- InProcess transport (backward compat)
- Length-prefixed JSON frame protocol
- Handshake messages (Handshake, HandshakeAck, Ping, Pong)
- SO_PEERCRED validation (Linux)
- Reconnection with exponential backoff

### Sprint 2 (2025-12-30) - Conductor Core

**Completed**:
- ConductorClient wrapper for TUI
- DisplayState types
- Activity overlay system
- Task panel widget
- Conductor-TUI message flow (11 tests)

## Completion Criteria

- [x] Conductor runs as standalone daemon reliably
- [x] Multiple TUI instances connect simultaneously
- [x] Late-joining surfaces receive complete state
- [x] All HIGH security findings (H-001, H-002) resolved
- [ ] All MEDIUM security findings addressed or documented
- [ ] Rate limiting prevents DoS
- [ ] Health check endpoint operational
- [ ] TOML configuration supported
- [ ] Chaos tests pass
- [ ] macOS support complete (getpeereid)
- [ ] Documentation complete

## Blocked Items

| Item | Blocked By | Notes |
|------|------------|-------|
| WebSocket transport | Security review + TLS setup | Phase 5+ |
| macOS getpeereid() | Developer with Mac | External dependency |
| Production deployment | Full security audit | Before v1.0 |

## Open Questions

### Active

| ID | Question | Owner | Target Sprint |
|----|----------|-------|---------------|
| - | WebSocket security requirements | Hacker | Post-Epic |
| - | systemd socket activation? | DevOps | Sprint 8 |
| - | Remote surface authentication flow | Architect | Post-Epic |

### Resolved

| ID | Question | Resolution | Sprint |
|----|----------|------------|--------|
| - | Session model | Broadcast (all surfaces see all) | Sprint 3 |
| - | Transport encoding | JSON (length-prefixed) | Sprint 3 |
| - | Daemon lifecycle | PID file + signal handling | Sprint 5 |

## Dependencies

### External Dependencies

| Dependency | Version | Purpose | Security Reviewed |
|------------|---------|---------|-------------------|
| tokio | 1.41 | Async runtime | Yes |
| clap | 4.5 | CLI parsing | Yes |
| nix | 0.29 | Signal handling | Yes |
| uuid | 1.11 | Random IDs | Yes |
| crc32fast | 1.4 | Frame integrity | Yes |

### Internal Dependencies

| Dependency | Status | Blocks |
|------------|--------|--------|
| conductor-core | Active | daemon, tui |
| Transport layer | Complete | Multi-surface |
| SurfaceRegistry | Complete | State sync |
| Launcher script | Complete | Process management |

## Related Documents

- `TODO-conductor-ux-split.md` - Detailed task breakdown
- `TODO-implementation-plan.md` - Implementation tracks A-D
- `TODO-meta-agent-conductor-interactions.md` - Agent orchestration
- `TODO-security-findings.md` - Security audit findings
- `TODO-disabled-tests.md` - Ignored tests (H-002 related)
- `deps.yaml` - Dependency tracking

---

**Epic Owner**: Architect + Backend Developer
**Last Updated**: 2026-01-02 (Sprint 6 complete - H-001, H-002 resolved)
