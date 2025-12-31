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

### Phase 2: TUI as Thin Client (IN PROGRESS - ~70%)
- [x] ConductorClient wrapper for TUI
- [x] DisplayState types (derived from ConductorMessages)
- [x] Activity overlay system (11 activity types)
- [x] Task panel widget
- [x] Goodbye message system
- [x] App refactor to use ConductorClient
- [x] Both crates compile successfully
- [ ] Full event loop completion (handle all SurfaceEvents)
- [ ] Integration test of message flow
- [ ] Developer mode (PJ) debug panel

### Phase 3: IPC Transport Layer (PENDING)
- [ ] Abstract transport trait (Transport: Send/Recv ConductorMessage/SurfaceEvent)
- [ ] Unix Socket transport implementation (default)
- [ ] WebSocket transport implementation (optional)
- [ ] Transport configuration in conductor-core
- [ ] yollayah.sh updates to launch Conductor as separate process
- [ ] Connection handshake and authentication
- [ ] Reconnection handling

### Phase 4: Process Separation (PENDING)
- [ ] Conductor as standalone binary (conductor-daemon)
- [ ] TUI connects via IPC instead of embedded
- [ ] Multiple surface support (concurrent connections)
- [ ] Surface registration and capabilities negotiation
- [ ] Graceful degradation if Conductor unreachable

### Phase 5: Configuration & Polish (PENDING)
- [ ] Config file for transport selection
- [ ] Runtime switching between transports
- [ ] Health checks and status reporting
- [ ] Performance profiling
- [ ] Documentation updates

## Feature Creep Items (Do Later)
_Items discovered during refactor that should NOT block this work:_

- [ ] Remote surface authentication (security review needed)
- [ ] Multi-Conductor federation (multiple machines)
- [ ] Surface hot-swap during session
- [ ] Conductor state persistence across restarts

## Commits (Stable Checkpoints)

| Commit | Description | Date |
|--------|-------------|------|
| _pending_ | Phase 2 completion checkpoint | - |

## Notes

- Original plan used Python Textual; switched to Rust ratatui for performance
- The TUI already has embedded Conductor via ConductorClient
- Need to extract to separate process with IPC for true independence
- Unix Sockets preferred for local security (no network exposure)
- WebSockets for remote surfaces (with auth)

---

**Last Updated**: 2025-12-31
