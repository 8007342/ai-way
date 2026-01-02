# Sprint Workflow

> High-level guide for running development sprints on ai-way.

**Related Documents**:
- [Epic Workflow](epic.md) - Major feature initiatives spanning multiple sprints
- [TODO-Driven Development](todo-driven-development.md) - Complete methodology documentation
- [PRINCIPLES.md](../PRINCIPLES.md) - Ethical, technical, and design guidelines

## Overview

A sprint is a focused development cycle that includes planning, implementation, testing, and review phases. `TODO-next.md` serves as the root of all TODO references and sprint tracking.

---

## Sprint Phases

### Phase 1: Triage & Planning

1. **Review TODO-next.md** - The source of truth for sprint priorities
2. **Identify targets**:
   - Critical bugs (fix immediately)
   - High-priority features (sprint focus)
   - Quick wins (fill gaps between major work)
3. **Assign roles**:
   - Architect: Protocol, architecture, integration
   - Backend: Core implementation, performance
   - TUI Developer: Terminal interface, UX
   - Hacker: Security, hardening, edge cases
   - QA: Testing, coverage, validation

### Phase 2: Development

1. **Spawn developers** in parallel for independent tasks
2. **Track progress** in TODO-next.md "Sprint N Progress" section
3. **Update relevant TODO files** as work progresses:
   - `TODO-avatar-animation-system.md` - Avatar/sprite work
   - `TODO-conductor-ux-split.md` - Architecture split
   - `TODO-integration-testing.md` - Test coverage
   - `TODO-main.md` - General features
   - `TODO-security-findings.md` - Security issues

### Phase 3: QA & Validation

1. **Run full test suite** - All tests must pass
2. **Review test coverage** - Ensure new code has tests
3. **Check for regressions** - Verify existing functionality
4. **Document any disabled tests** with root cause analysis

### Phase 4: Documentation & Commit

1. **Update TODO-next.md**:
   - Move completed items to "Just Completed" section
   - Update blocked/unblocked items
   - Adjust priorities based on discoveries
2. **Commit and push** with descriptive message
3. **Tag milestone commits** when appropriate

---

## Periodic Reviews

### Every 2-3 Sprints: Security Review

- **Participants**: Architect, Hacker, QA
- **Activities**:
  - Audit authentication/authorization
  - Review input validation
  - Check for injection vulnerabilities
  - Validate rate limiting and DoS protection
  - Update `TODO-security-findings.md`

### Every 3-4 Sprints: Architecture Review

- **Participants**: Architect, Backend, UX
- **Activities**:
  - Review technical debt
  - Assess performance bottlenecks
  - Evaluate API consistency
  - Plan refactoring if needed
  - Clean up obsolete TODO items

### Every 4-5 Sprints: Full Audit

- **Participants**: Architect, Hacker, QA, Lawyer (compliance)
- **Activities**:
  - Comprehensive security audit
  - Performance profiling
  - UX consistency review
  - License/compliance check
  - Re-triage all pending items
  - Remove stale/irrelevant items
  - Document new feature requests

---

## TODO File Hierarchy

```
TODO-next.md                    <- Sprint root (current priorities)
  |
  +-- TODO-main.md              <- General features and enhancements
  +-- TODO-avatar-animation-system.md  <- Avatar/sprite system
  +-- TODO-conductor-ux-split.md       <- Architecture refactor
  +-- TODO-integration-testing.md      <- Test coverage tracking
  +-- TODO-security-findings.md        <- Security issues and fixes
  +-- TODO-disabled-tests.md           <- Centralized ignored test tracking
  +-- TODO-epic-YYYYQN-*.md            <- Epic-level planning files

deps.yaml                       <- Component and dependency tracking
```

See [TODO-Driven Development](todo-driven-development.md) for complete file specifications.

---

## Quick Reference

### Starting a Sprint

```
1. Read TODO-next.md
2. Pick 3-5 items based on priority and dependencies
3. Spawn developers for parallel work
4. Monitor progress and unblock as needed
```

### Ending a Sprint

```
1. Verify all tests pass
2. Update TODO-next.md with completed items
3. Commit with message: "feat: Sprint N - [summary]"
4. Push to remote
```

### Sprint Commit Format

```
feat: Sprint N - Brief summary of major changes

- Item 1: Description
- Item 2: Description
- Item 3: Description
```

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.1 | 2026-01-02 | Added related docs links, updated hierarchy with deps.yaml and epic files |
| 1.0 | 2026-01-02 | Initial workflow documentation |
