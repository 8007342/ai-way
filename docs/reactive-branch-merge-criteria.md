# Reactive Branch Merge Criteria

**Branch**: `feature/reactive-tui-overhaul`
**Target**: `main` (ai-way 0.2.0 breaking release)

## Mandatory Criteria (ALL must pass)

### Code Quality
- [ ] Zero `.await` calls in production code (enforced by grep in CI)
- [ ] Zero `tokio::select!` calls
- [ ] Zero `tokio::spawn` calls
- [ ] All code uses reactive streams/observables

### Testing
- [ ] All unit tests passing
- [ ] All integration tests re-enabled and passing
- [ ] Architectural enforcement tests passing
- [ ] Performance tests passing

### Performance
- [ ] TUI idle CPU < 0.1%
- [ ] TUI active CPU < 5%
- [ ] Frame rate: 10 FPS consistent
- [ ] Memory: < 50 MB idle, < 100 MB streaming
- [ ] No UI freezing ever
- [ ] No memory leaks (validated by valgrind)

### User Experience
- [ ] Smooth token streaming (no batching)
- [ ] Responsive to user input (< 100ms)
- [ ] Graceful error handling
- [ ] Clean shutdown

### Documentation
- [ ] PRINCIPLE-efficiency.md Law 1 rewritten
- [ ] Migration guide complete
- [ ] Architecture documentation updated
- [ ] CHANGELOG updated with breaking changes

### Team Sign-off
- [ ] Architect approval
- [ ] Rust/Async Specialist approval
- [ ] Hacker security review
- [ ] UX validation

## Timeline

**Earliest merge**: End of Sprint 6 (~10-14 weeks)

## Breaking Changes

This is a **breaking release** (0.2.0):
- Public API changes in TUI/Conductor
- Configuration format changes possible
- Plugin/extension API incompatible with 0.1.x

All breaking changes must be documented in CHANGELOG.md.
