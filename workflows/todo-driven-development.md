# TODO-Driven Development (TDD)

> A methodology where TODO files serve as the source of truth for planning, tracking, and coordinating AI-assisted development.
>
> **Note**: TDD in this context refers to TODO-Driven Development, not Test-Driven Development.

**Created**: 2026-01-02
**Last Updated**: 2026-01-02
**Authors**: Hacker (Security Specialist), QA Engineer

---

## Table of Contents

1. [Core Philosophy](#core-philosophy)
2. [TODO File Hierarchy](#todo-file-hierarchy)
3. [File Specifications](#file-specifications)
4. [Lifecycle Management](#lifecycle-management)
5. [Agent Update Guidelines](#agent-update-guidelines)
6. [Security Considerations](#security-considerations)
7. [Quality Assurance Integration](#quality-assurance-integration)
8. [Integration with deps.yaml](#integration-with-depsyaml)
9. [Freshness Tracking](#freshness-tracking)
10. [Anti-Patterns and Pitfalls](#anti-patterns-and-pitfalls)
11. [Templates](#templates)

---

## Core Philosophy

TODO-Driven Development centers on three principles:

1. **TODO files are the source of truth** - Not project management tools, not Slack, not mental notes. If it is not in a TODO file, it does not exist as tracked work.

2. **Agents and humans share the same tracking system** - AI agents update TODO files after completing work. Humans review and triage. Both operate on the same artifact.

3. **Transparency over convenience** - Every change is attributed. Every decision is documented. Freshness is tracked. Stale items are visible.

### Why TODO Files?

| Approach | Pros | Cons |
|----------|------|------|
| External tools (Jira, etc.) | Rich features, dashboards | Disconnected from code, context switching |
| GitHub Issues | Integrated with PRs | Separate from codebase, requires network |
| **TODO files in repo** | Versioned with code, grep-able, offline-first | Manual discipline required |

TODO files live with the code. When you checkout a branch, you get its TODO state. When you review a PR, you see TODO changes. When agents work, they update TODO files directly.

---

## TODO File Hierarchy

```
TODO-next.md              <- Sprint root (immediate priorities, 3+ sprint lookahead)
TODO-main.md              <- General triage and index of all TODO files
TODO-security-findings.md <- Security-specific tracking with severity levels
TODO-disabled-tests.md    <- Centralized tracking of all #[ignore] tests
TODO-epic-YYYYQN-*.md     <- Epic-level planning (e.g., TODO-epic-2026Q1-avatar.md)
TODO-*.md                 <- Feature/component specific files
```

### Hierarchy Rules

1. **TODO-next.md is king** - It references everything currently active
2. **TODO-main.md is the index** - New items land here first, then get triaged
3. **Specialized files focus** - Security findings stay in security file, tests in test file
4. **Epic files have lifecycle** - Created for major initiatives, archived when complete
5. **Feature files are ephemeral** - Created as needed, archived when feature ships

### File Relationships

```
                    TODO-next.md
                         |
          +--------------+--------------+
          |              |              |
    TODO-main.md   TODO-security   TODO-epic-*
          |        -findings.md         |
          v              |              v
    [Feature Files]      |        [Phase tracking]
                         v
              [Severity-based items]
```

---

## File Specifications

### TODO-next.md (Sprint Root)

The single source of truth for current sprint priorities.

**Required Sections:**

| Section | Purpose |
|---------|---------|
| Header | Generated date, updated date, triage team |
| Sprint N Progress | Just completed items with notes |
| Sprint N-1 Recap | Previous sprint summary |
| Top Priorities | Numbered list with Why now, Tasks, Unblocks |
| Quick Wins | Low-effort items that fit around major work |
| Work Streams Status | Priority, status, next tasks per stream |
| Blocked Items | What is blocked and by what |
| Recently Unblocked | Items freed up by recent work |
| Open Questions | Decisions needed with owner and status |
| Disabled Tests | Test name, priority, owner, target sprint |
| Technical Debt | Known debt with risk assessment |
| Recommended Split | Team member focus areas |
| Test Summary | Current passing test counts |

**Example Priority Item:**

```markdown
### 1. Animation Evolution System (Phase P3.1-P3.2) [HIGH - Backend]
**File**: `TODO-avatar-animation-system.md` Phase 3

**Why now**: All Phase 1 (protocol) and Phase 2 (TUI) work is complete.

**Tasks**:
- P3.1: Implement `EvolutionState` tracking in Conductor
- P3.2: Add evolution triggers (interaction count, session time)
- Define evolution level thresholds and visual markers

**Unblocks**: Progressive avatar personality
```

### TODO-security-findings.md (Security Tracking)

Dedicated security issue tracking with severity-based triage.

**Severity Levels:**

| Level | Definition | Response Time |
|-------|------------|---------------|
| **CRITICAL** | Actively exploitable, data breach risk | Immediate (block release) |
| **HIGH** | Exploitable with effort, significant impact | Next sprint |
| **MEDIUM** | Limited exploitability or impact | Within 2-3 sprints |
| **LOW** | Theoretical risk, defense-in-depth | Backlog |
| **INFO** | Best practice recommendations | As time permits |

**Finding Format:**

```markdown
#### H-001: Sequential ConnectionId Generation
**Location**: `conductor/core/src/surface.rs`
**Status**: Open
**Found**: Sprint 5
**Description**: ConnectionId uses sequential counter (AtomicU64). Predictable IDs could allow connection hijacking if combined with other vulnerabilities.
**Recommendation**: Use cryptographically random UUIDs for production.
**Mitigation**: Unix socket peer credential validation provides defense-in-depth.
```

**Required Sections:**
- Active Findings (by severity)
- Resolved Findings (with resolution notes)
- Audit Schedule
- Compliance Notes
- Adding New Findings (process documentation)

### TODO-disabled-tests.md (Test Tracking)

Centralized view of all ignored tests with structured metadata.

**Disabled Test Format:**

```rust
#[test]
#[ignore] // Epic: 2026Q1-Avatar, Feature: P4.2 Sprite Generation, Fix: Sprint 8
fn test_sprite_generation_under_load() {
    // Test requires sprite generation pipeline (P4.2) to be complete
    // See: TODO-avatar-animation-system.md Phase 4
}
```

**Tracking Table:**

| Test Name | Epic | Feature | Target Sprint | Reason | Status |
|-----------|------|---------|---------------|--------|--------|
| scenario_7_connection_pool | - | H-002 | Sprint 6 | Pool refactor needed | Blocked |
| scenario_10_stress_test | - | - | N/A | Intentional (10 min runtime) | Permanent |

### TODO-epic-YYYYQN-*.md (Epic Files)

Long-running initiatives spanning multiple sprints.

**Naming Convention**: `TODO-epic-2026Q1-avatar-animation.md`

**Required Sections:**
- Team Analysis (perspectives from different roles)
- Implementation Phases (with task lists)
- Dependency Graph
- Security Considerations
- UX Guidelines
- Open Questions (must answer before phase N)
- Metrics and Success Criteria
- References

### TODO-*.md (Feature Files)

Component or feature-specific tracking.

**Naming Examples:**
- `TODO-conductor-ux-split.md` - Architecture refactor
- `TODO-integration-testing.md` - Test infrastructure
- `TODO-accessibility.md` - Accessibility roadmap

**Common Sections:**
- Goal
- Progress (with checkbox items)
- Feature Creep Items (scope expansion to defer)
- Last Updated date

---

## Lifecycle Management

### 1. Creation

TODO files are created during:

| Trigger | Creator | File Type |
|---------|---------|-----------|
| Sprint planning | Triage team | TODO-next.md updates |
| Security audit | Hacker | TODO-security-findings.md entries |
| New feature initiative | Architect | TODO-epic-*.md or TODO-*.md |
| Bug discovery | Any agent | Entry in relevant TODO file |
| Test disabled | Developer | TODO-disabled-tests.md entry + inline comment |

### 2. Updates

**Update Attribution Format:**

```markdown
**Updated**: 2026-01-02 (Sprint 5 - Added P3.1 evolution tracking)
```

**Update Protocol:**

1. Note the sprint number
2. Note what changed
3. Note why (brief)
4. Track dependencies affected
5. Update freshness timestamp

### 3. Triage

Regular review to prioritize and clean up.

| Frequency | Scope | Participants |
|-----------|-------|--------------|
| Every sprint | TODO-next.md | Full triage team |
| Every 2-3 sprints | Security findings | Architect, Hacker, QA |
| Every 3-4 sprints | Architecture review | Architect, Backend, UX |
| Every 4-5 sprints | Full audit | All specialists + Lawyer |

**Triage Checklist:**
- [ ] Are priorities still correct?
- [ ] Are blocked items still blocked?
- [ ] Are recently unblocked items captured?
- [ ] Are stale items identified (> 2 sprints untouched)?
- [ ] Should any items move to Feature Creep?
- [ ] Are disabled tests still necessary?

### 4. Archival

Completed items are preserved, not deleted.

**Archival Patterns:**

```markdown
## Completed (Sprint 5)

| Item | Category | Notes |
|------|----------|-------|
| **Sprite Protocol Messages (P1.2-P1.3)** | Architecture | `SpriteRequest`, `SpriteResponse` |
```

For security findings:

```markdown
## Resolved Findings

### R-001: QuickResponse Hard Latency Filter (was HIGH)
**Location**: `conductor/core/src/routing/policy.rs`
**Resolved**: Sprint 4
**Description**: Hard latency filter rejected all models for short messages.
**Resolution**: Changed to scoring-only approach.
```

---

## Agent Update Guidelines

When AI agents complete work, they must update relevant TODO files.

### Required Updates

| Action | Update Required |
|--------|-----------------|
| Completing a task | Mark checkbox, add completion note |
| Finding a bug | Add entry to relevant TODO file |
| Disabling a test | Add to TODO-disabled-tests.md + inline comment |
| Discovering security issue | Add to TODO-security-findings.md with severity |
| Scope creep | Add to Feature Creep section, not main backlog |
| Unblocking work | Update Blocked/Unblocked sections in TODO-next.md |

### Formatting Standards

**Checkboxes:**
```markdown
- [x] **P1.1** Define `Block` struct in `conductor-core` - Sprint 3
- [ ] **P1.2** Add `SpriteRequest` to protocol
```

**Tables:**
```markdown
| Item | Priority | Status | Notes |
|------|----------|--------|-------|
| Connection pool refactor | HIGH | In Progress | Sprint 6 target |
```

**Cross-References:**
```markdown
**File**: `TODO-avatar-animation-system.md` Phase 3
**See**: `TODO-security-findings.md` H-002
```

### Do Not

- Delete items without moving to history/archive
- Change priorities without triage approval
- Mark items complete if tests are failing
- Add items without owner/priority
- Create new TODO files without updating TODO-main.md index

---

## Security Considerations

### Threat Model for TODO Files

| Threat | Risk | Mitigation |
|--------|------|------------|
| Sensitive data in TODOs | Information disclosure | Never include credentials, keys, or PII |
| Stale security findings | False sense of security | Enforce freshness tracking |
| Ignored tests hiding bugs | Security regression | Mandatory tracking with sprint targets |
| Agent manipulation | Malicious TODO edits | Review agent changes like code changes |

### Security Finding Workflow

```
Discovery -> Severity Assessment -> TODO Entry -> Triage -> Sprint Assignment
     |                                                              |
     v                                                              v
  Immediate                                                    Resolution
  (if CRITICAL)                                                     |
                                                                    v
                                                            Resolved Section
```

### Required Security Reviews

| Milestone | Review Type | Documented In |
|-----------|-------------|---------------|
| Every 2-3 sprints | Security review | TODO-security-findings.md |
| Before v1.0 | Penetration testing | TODO-security-findings.md |
| Before WebSocket | Transport security audit | TODO-security-findings.md |
| On dependency update | Dependency audit | TODO-security-findings.md |

---

## Quality Assurance Integration

### Test Coverage Tracking

Tests are tracked at multiple levels:

1. **TODO-qa-testing.md** - Unit test progress per module
2. **TODO-integration-testing.md** - Integration test suite
3. **TODO-disabled-tests.md** - Ignored tests with justification

### Test Summary Format

```markdown
## Test Summary

- **conductor-core**: 378 tests passing
- **yollayah-tui**: 26 tests passing + integration tests
- **Total**: 506+ tests passing
- **Ignored**: 11 (stress tests + setup-dependent doc tests)
```

### Disabled Test Requirements

Every `#[ignore]` test must have:

1. **Inline comment** with Epic, Feature, Target Sprint
2. **Entry in TODO-disabled-tests.md** with priority and owner
3. **Entry in TODO-next.md** if actively blocking work

**Example:**

```rust
#[test]
#[ignore] // Epic: -, Feature: H-002, Fix: Sprint 6, Owner: Backend
fn scenario_7_connection_pool() {
    // Connection pool doesn't return connections - see TODO-security-findings.md H-002
    // Blocked by: Pool refactor to use Arc<ConnectionPool>
}
```

### Pre-Commit Integration

TODO files can be validated in pre-commit hooks:

```bash
# Check for TODO file freshness (> 14 days stale)
# Check for orphaned disabled tests (not in TODO-disabled-tests.md)
# Check for security findings without severity
```

---

## Integration with deps.yaml

### Component Dependencies

`deps.yaml` at repo root tracks component dependencies:

```yaml
components:
  conductor-core:
    version: 0.5.0
    depends_on: []

  yollayah-tui:
    version: 0.5.0
    depends_on:
      - conductor-core

  conductor-daemon:
    version: 0.5.0
    depends_on:
      - conductor-core
```

### TODO File References

TODO files reference components by their deps.yaml names:

```markdown
### 2. Sprite Generation Pipeline [HIGH - conductor-core]
**Component**: `conductor-core/src/avatar/`
**Depends On**: conductor-core (no external deps)
```

### Freshness Alignment

When deps.yaml component is updated, check related TODO files:

| Component Updated | Check These TODO Files |
|-------------------|----------------------|
| conductor-core | TODO-conductor-ux-split.md, TODO-avatar-animation-system.md |
| yollayah-tui | TODO-accessibility.md, TODO-integration-testing.md |
| conductor-daemon | TODO-conductor-ux-split.md (Phase 4) |

---

## Freshness Tracking

### Freshness Indicators

Every TODO file should have:

```markdown
**Created**: 2026-01-02
**Last Updated**: 2026-01-02 (Sprint 5 - reason)
```

### Staleness Thresholds

| Age | Status | Action |
|-----|--------|--------|
| < 2 sprints | Fresh | None |
| 2-4 sprints | Stale | Review in next triage |
| > 4 sprints | Very Stale | Must address or archive |

### Stale Item Detection

During triage, identify stale items:

```markdown
## Stale Items (> 2 sprints untouched)

| Item | Last Updated | Action Needed |
|------|--------------|---------------|
| RTL language support | Sprint 2 | Defer to Q2 or archive |
| Multi-conductor federation | Sprint 1 | Move to Feature Creep |
```

---

## Anti-Patterns and Pitfalls

### Anti-Patterns to Avoid

| Anti-Pattern | Why It Is Bad | Correct Approach |
|--------------|---------------|------------------|
| **TODO sprawl** | Too many files, nothing findable | Consolidate, use sections |
| **Orphaned TODOs** | Items with no owner/priority | Every item needs owner and priority |
| **TODO as documentation** | Long prose, not actionable | Keep items actionable, docs go elsewhere |
| **Silent completion** | Marking done without notes | Always note what was done and when |
| **Priority inflation** | Everything is HIGH | Use triage to ruthlessly prioritize |
| **Ignored test graveyard** | Tests ignored forever | Sprint targets, regular review |
| **Stale security findings** | Old issues never addressed | Enforce response time SLAs |

### Common Pitfalls

1. **Creating TODO files but not updating TODO-main.md index**
   - Fix: Add to TODO-main.md immediately upon creation

2. **Scope creep in Feature Creep section**
   - Fix: Feature Creep is parking lot, not backlog. Review and prune.

3. **Conflicting priorities across files**
   - Fix: TODO-next.md is authoritative. Other files defer to it.

4. **Agent changes without review**
   - Fix: Agent TODO changes go through normal PR review

5. **Missing cross-references**
   - Fix: Always link related items. Use File: and See: patterns.

---

## Templates

### New Feature TODO File

```markdown
# TODO: [Feature Name]

> Brief description of the feature.
>
> **Created**: YYYY-MM-DD
> **Last Updated**: YYYY-MM-DD (Sprint N - initial creation)
> **Owner**: [Role]

---

## Goal

[One paragraph describing the goal]

## Progress

### Phase 1: [Phase Name]

- [ ] Task 1
- [ ] Task 2
- [ ] Task 3

### Phase 2: [Phase Name]

- [ ] Task 1
- [ ] Task 2

## Dependencies

| Dependency | Status | Notes |
|------------|--------|-------|
| [Dep 1] | Pending | [Notes] |

## Feature Creep Items

_Items discovered during development that should NOT block this work:_

- [ ] Item 1
- [ ] Item 2

---

**See Also**:
- [Related TODO file]
- [Related documentation]
```

### New Security Finding

```markdown
#### [SEVERITY]-[NNN]: [Title]
**Location**: `path/to/file.rs`
**Status**: Open
**Found**: Sprint N
**Description**: [What is the vulnerability]
**Recommendation**: [How to fix it]
**Mitigation**: [Any existing defense-in-depth]
```

### New Disabled Test Entry

```markdown
| Test Name | Epic | Feature | Target Sprint | Reason | Owner | Status |
|-----------|------|---------|---------------|--------|-------|--------|
| test_name | EPIC-ID | Feature ref | Sprint N | Why disabled | Role | Blocked/Pending |
```

### Sprint Priority Item

```markdown
### N. [Item Name] (Phase X.Y) [PRIORITY - Team]
**File**: `TODO-*.md` reference

**Why now**: [Justification for doing this now]

**Tasks**:
- Task 1
- Task 2
- Task 3

**Unblocks**: [What this enables]
```

---

## Design Principles and Reference Documents

All development in ai-way MUST adhere to documented design principles and architectural requirements. These are living documents that capture lessons learned, best practices, and hard requirements.

### Core Principles

Located in `reference/PRINCIPLE-*.md`:

1. **PRINCIPLE-efficiency.md** - Async efficiency, zero sleep, aggressive caching
   - Law 1: No Sleep, Only Wait on I/O
   - Law 2: Lazy Initialization, Aggressive Caching
   - Law 3: Surfaces Are Thin Clients
   - **Violation Severity**: CRITICAL

2. **PRINCIPLE-separation.md** (planned) - TUI/Conductor architecture patterns
   - Message-based communication
   - Surface-agnostic core
   - State ownership rules

3. **PRINCIPLE-ux.md** (planned) - UX consistency, minimalism, theming
   - Axolotl blocky aesthetic
   - Color palette standards
   - Animation principles

### Hard Requirements

Located in `reference/REQUIRED-*.md`:

1. **REQUIRED-separation.md** - TUI/Conductor must be fully separated
   - Conductor MUST compile without TUI dependency
   - TUI MUST be swappable (embedded, daemon, web, CLI, headless)
   - All communication via message protocol
   - **Violation Severity**: CRITICAL (blocks production)

### Forbidden Practices

Located in `reference/FORBIDDEN-*.md`:

1. **FORBIDDEN-inefficient-calculations.md** - Anti-patterns found in codebase
   - Sleep in polling loops
   - Recalculating throwaway data
   - Allocating in hot paths
   - Rendering unchanged regions
   - **Purpose**: Prevent recurrence of known bad practices

### Integration with TODO-Driven Development

**When creating TODO files**:
1. Reference relevant `PRINCIPLE-*.md` if work requires architectural decisions
2. Link to `REQUIRED-*.md` for compliance-critical work
3. Cite `FORBIDDEN-*.md` when fixing anti-patterns

**When filing bugs**:
1. Create `BUG-XXX-*.md` for principle violations
2. Link to violated principle document
3. Create corresponding `TODO-XXX-*.md` for remediation

**Example**:
```markdown
# TODO-015-eliminate-polling-sleeps.md

**Principle**: reference/PRINCIPLE-efficiency.md (Law 1: No Sleep)
**Bug**: BUG-015-sleep-in-polling-loops.md
**Priority**: P0 - CRITICAL

## Tasks
- [ ] Replace daemon polling loop with event-driven (conductor-daemon.rs:231)
- [ ] Fix Unix socket server polling (unix_socket/server.rs:419)
- [ ] Add regression test for idle CPU usage
```

### Enforcement

**Code Review Checklist**:
- [ ] No `std::thread::sleep()` in async code
- [ ] No `tokio::time::sleep()` in loops (except frame limiting/backoff)
- [ ] TUI doesn't import Conductor internals
- [ ] All I/O is async and non-blocking
- [ ] Caching implemented for repeated calculations

**CI/CD Checks**:
- Lint rules enforce principle adherence
- Performance regression tests
- Compilation test (Conductor without TUI)

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-01-02 | Initial methodology documentation |

---

## See Also

- `workflows/sprint.md` - Sprint workflow and phases
- `TODO-next.md` - Current sprint priorities
- `TODO-main.md` - General backlog and TODO file index
- `TODO-security-findings.md` - Security issue tracking
