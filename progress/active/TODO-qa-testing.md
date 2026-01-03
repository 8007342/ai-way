# TODO: QA Testing Infrastructure

Tracking unit tests, integration tests, and pre-commit hooks.

## Goal

Establish comprehensive test coverage for conductor-core and TUI with automated quality gates.

## Progress

### Unit Tests

#### conductor-core (121 tests total)
- [x] conductor-core/src/conductor.rs - Orchestration logic (2 tests)
- [x] conductor-core/src/messages.rs - Message serialization (3 tests)
- [x] conductor-core/src/events.rs - Event handling (3 tests)
- [x] conductor-core/src/avatar.rs - Avatar state machine (5 tests)
- [x] conductor-core/src/session.rs - Session management (9 tests)
- [x] conductor-core/src/tasks.rs - Task lifecycle (9 tests)
- [x] conductor-core/src/security.rs - Command validation (60 tests) **ENHANCED**
  - Input injection prevention tests
  - Command injection prevention tests
  - Task agent injection prevention tests
  - CustomSprite validation tests
  - PointAt boundary validation
  - Allowlist management tests
  - Rejection logging tests
  - ValidationResult tests
  - SecurityConfig tests

#### TUI (59 tests total)
- [x] tui/src/display.rs - Display state updates (56 tests) **NEW**
  - DisplayMessage tests
  - DisplayRole tests
  - DisplayAvatarState tests
  - DisplayTask tests
  - DisplayTaskStatus tests
  - DisplayState tests
  - agent_to_family_name tests
- [ ] tui/src/conductor_client.rs - Client message handling (needs mock backend)

### Integration Tests (PENDING)
- [ ] Full message flow: SurfaceEvent -> Conductor -> ConductorMessage -> Display
- [ ] Avatar animation sequences
- [ ] Task creation/update/completion cycle
- [ ] Session start/message/end flow
- [ ] Error handling and recovery

### Pre-commit Hooks
- [x] Integrity checksums update (.sh files)
- [x] cargo fmt --check (Rust formatting)
- [x] cargo clippy -- -D warnings (Rust linting)
- [x] cargo test (run unit tests)
- [ ] Shell script linting (shellcheck)

### CI/CD (FUTURE)
- [ ] GitHub Actions workflow for tests
- [ ] Coverage reporting
- [ ] Release builds

## Test Infrastructure

Tests are implemented using `#[cfg(test)]` modules within each source file, following Rust conventions. The `pretty_assertions` crate is used for clearer test output.

### Test Count Summary
| Crate | Module | Tests |
|-------|--------|-------|
| conductor-core | security.rs | 60 |
| conductor-core | tasks.rs | 9 |
| conductor-core | session.rs | 9 |
| conductor-core | avatar.rs | 5 |
| conductor-core | accessibility.rs | 5 |
| conductor-core | messages.rs | 3 |
| conductor-core | events.rs | 3 |
| conductor-core | conductor.rs | 2 |
| conductor-core | (other) | 30 |
| **conductor-core total** | | **126** |
| tui | display.rs | 56 |
| tui | (other) | 3 |
| **tui total** | | **59** |
| **Grand Total** | | **185** |

## Feature Creep Items (Do Later)

- [ ] Performance benchmarks
- [ ] Fuzz testing for message parsing
- [ ] UI snapshot testing
- [ ] Load testing for multiple surfaces
- [ ] Mock backend for conductor_client.rs testing

---

**Last Updated**: 2025-12-31
