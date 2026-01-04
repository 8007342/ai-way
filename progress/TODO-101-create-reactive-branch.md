# TODO-101: Create Reactive Overhaul Branch

**EPIC**: EPIC-001-TUI-reactive-overhaul.md
**Sprint**: SPRINT-00-foundation.md
**Story**: STORY-001
**Owner**: Hacker
**Status**: 🟡 READY TO START
**Effort**: 1 hour
**Priority**: P0 - CRITICAL (blocks all other work)

---

## Objective

Create isolated feature branch for TUI reactive architecture rewrite.

---

## Tasks

- [ ] Create branch `feature/reactive-tui-overhaul` from `main`
- [ ] Push branch to remote
- [ ] Update branch README with development notice
- [ ] Configure CI to skip integration tests on this branch
- [ ] Document branch merge criteria

---

## Branch README Notice

Add to top of README.md on this branch:

```markdown
# 🚧 DEVELOPMENT BRANCH: Reactive Architecture Overhaul

**Status**: UNDER ACTIVE DEVELOPMENT - DO NOT USE IN PRODUCTION

This branch contains a **complete rewrite** of the TUI and Conductor async architecture.

**What's happening**:
- Migrating from manual `.await` patterns to reactive streams (tokio-stream/RxRust)
- Eliminating memory leaks and thread pool conflicts
- Building correct async architecture from first principles

**Integration tests are DISABLED** on this branch during development.

**Merge criteria**:
- Zero `.await` violations
- All unit tests passing
- Performance targets met
- Integration tests re-enabled and passing
- Team code review and sign-off

**Timeline**: 10-14 weeks (see EPIC-001-TUI-reactive-overhaul.md)

**DO NOT merge to main until Sprint 6 complete.**
```

---

## CI Configuration

Update `.github/workflows/` (or equivalent) to skip integration tests:

```yaml
# Skip integration tests on reactive overhaul branch
- name: Run tests
  run: |
    if [[ "${{ github.ref }}" == "refs/heads/feature/reactive-tui-overhaul" ]]; then
      echo "Development branch - running unit tests only"
      cargo test --lib --all
    else
      echo "Running full test suite"
      cargo test --all
    fi
```

---

## Merge Criteria Document

Create `docs/reactive-branch-merge-criteria.md`:

```markdown
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
```

---

## Acceptance Criteria

- [ ] Branch `feature/reactive-tui-overhaul` exists on remote
- [ ] README updated with development notice
- [ ] CI configured to skip integration tests
- [ ] Merge criteria documented
- [ ] Team aware of branch and its purpose

---

## Commands

```bash
# Create and push branch
git checkout main
git pull origin main
git checkout -b feature/reactive-tui-overhaul
git push -u origin feature/reactive-tui-overhaul

# Update README
# (edit README.md to add notice at top)

# Update CI
# (edit .github/workflows/ files)

# Create merge criteria doc
mkdir -p docs
# (create docs/reactive-branch-merge-criteria.md)

# Commit setup
git add README.md .github/ docs/
git commit -m "Setup reactive overhaul development branch

- Added development notice to README
- Configured CI to skip integration tests
- Documented merge criteria

Branch will be merged to main after Sprint 6 (~10-14 weeks)
as breaking release ai-way 0.2.0

Related: EPIC-001-TUI-reactive-overhaul.md"

git push
```

---

**Next**: TODO-102-add-reactive-dependencies.md

**Status**: 🟡 READY TO START
