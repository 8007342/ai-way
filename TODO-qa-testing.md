# TODO: QA Testing Infrastructure

Tracking unit tests, integration tests, and pre-commit hooks.

## Goal

Establish comprehensive test coverage for conductor-core and TUI with automated quality gates.

## Progress

### Unit Tests (PENDING)
- [ ] conductor-core/src/conductor.rs - Orchestration logic
- [ ] conductor-core/src/messages.rs - Message serialization
- [ ] conductor-core/src/events.rs - Event handling
- [ ] conductor-core/src/avatar.rs - Avatar state machine
- [ ] conductor-core/src/session.rs - Session management
- [ ] conductor-core/src/tasks.rs - Task lifecycle
- [ ] conductor-core/src/security.rs - Command validation
- [ ] tui/src/display.rs - Display state updates
- [ ] tui/src/conductor_client.rs - Client message handling

### Integration Tests (PENDING)
- [ ] Full message flow: SurfaceEvent -> Conductor -> ConductorMessage -> Display
- [ ] Avatar animation sequences
- [ ] Task creation/update/completion cycle
- [ ] Session start/message/end flow
- [ ] Error handling and recovery

### Pre-commit Hooks (PARTIAL)
- [x] Integrity checksums update (.sh files)
- [ ] cargo fmt check (Rust formatting)
- [ ] cargo clippy (Rust linting)
- [ ] cargo test (run unit tests)
- [ ] Shell script linting (shellcheck)

### CI/CD (FUTURE)
- [ ] GitHub Actions workflow for tests
- [ ] Coverage reporting
- [ ] Release builds

## Test Infrastructure

```
conductor/core/
    tests/
        unit/
            conductor_tests.rs
            message_tests.rs
            avatar_tests.rs
        integration/
            message_flow_tests.rs

tui/
    tests/
        unit/
            display_tests.rs
        integration/
            e2e_tests.rs
```

## Feature Creep Items (Do Later)

- [ ] Performance benchmarks
- [ ] Fuzz testing for message parsing
- [ ] UI snapshot testing
- [ ] Load testing for multiple surfaces

---

**Last Updated**: 2025-12-31
