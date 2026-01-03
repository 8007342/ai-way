# TODO: Documentation Updates

Rolling tracker for documentation that needs updating based on code changes.

## How to Use

When making code changes, add an entry here if the change affects:
- User-facing behavior
- Installation/setup process
- Configuration options
- API changes
- New features

Mark items done when README.md or other user docs are updated.

---

## Pending Documentation

### High Priority (Affects User Experience)

- [ ] **Conductor/TUI Architecture Split** - README needs section explaining:
  - New architecture with separate Conductor and Surface components
  - Environment variable `CONDUCTOR_TRANSPORT` (inprocess/unix)
  - Unix socket location (`$XDG_RUNTIME_DIR/ai-way/conductor.sock`)

- [ ] **Transport Configuration** - Document new config options:
  - `CONDUCTOR_TRANSPORT` - Transport type selection
  - `CONDUCTOR_SOCKET` - Custom socket path
  - Default behavior (embedded/inprocess mode)

- [ ] **New IPC Messages** - Developer docs for:
  - Handshake protocol (Handshake/HandshakeAck)
  - Heartbeat (Ping/Pong)
  - Frame protocol (length-prefixed JSON)

### Medium Priority

- [ ] **Avatar Commands** - Document new yolla: commands:
  - Task commands (task_start, task_progress, task_done, task_fail)
  - All mood/size/gesture aliases
  - Position commands

- [ ] **Security Features** - Document:
  - SO_PEERCRED peer validation on Linux
  - Socket permissions (0600)
  - Rate limiting defaults

- [ ] **Pre-commit Hooks** - Developer setup:
  - cargo fmt, clippy, test requirements
  - How to install hooks (`scripts/install-hooks.sh`)

### Low Priority

- [ ] **Test Coverage** - Document test infrastructure:
  - 180 unit tests across conductor-core and TUI
  - How to run tests
  - Test categories (security, display, transport)

- [ ] **Agent Family Names** - Document the mapping:
  - ethical-hacker -> Cousin Rita
  - qa-engineer -> The Intern
  - etc.

---

## Recently Completed

| Date | Change | Doc Updated |
|------|--------|-------------|
| 2025-12-31 | Phase 2: TUI thin client refactor | Pending |
| 2025-12-31 | Phase 3: IPC transport layer | Pending |
| 2025-12-31 | QA: 180 unit tests added | Pending |

---

## Guidelines

1. Add entries as you code - don't wait until the end
2. Be specific about what needs documenting
3. Link to relevant code files when helpful
4. Prioritize user-facing changes over internal details
5. Keep README.md focused on "getting started" - detailed docs go elsewhere

---

**Last Updated**: 2025-12-31
