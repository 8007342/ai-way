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
2. [File Naming Standards](#file-naming-standards)
3. [TODO File Hierarchy](#todo-file-hierarchy)
4. [File Navigation](#file-navigation)
5. [File Specifications](#file-specifications)
6. [Lifecycle Management](#lifecycle-management)
7. [QA Verification Workflow](#qa-verification-workflow)
8. [Agent Update Guidelines](#agent-update-guidelines)
9. [Security Considerations](#security-considerations)
10. [Quality Assurance Integration](#quality-assurance-integration)
11. [Integration with deps.yaml](#integration-with-depsyaml)
12. [Freshness Tracking](#freshness-tracking)
13. [Anti-Patterns and Pitfalls](#anti-patterns-and-pitfalls)
14. [Templates](#templates)

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

## File Naming Standards

### Mandatory Prefixes

**CRITICAL**: All files in `progress/` directory MUST use one of these prefixes:

| Prefix | Meaning | Example |
|--------|---------|---------|
| `TODO-` | Active work, pending tasks | `TODO-BUG-001-tui-streaming.md` |
| `DONE-` | Completed work, archived | `DONE-BUG-001-tui-streaming.md` |
| `TODO-QA-verify-` | QA verification in progress | `TODO-QA-verify-BUG-001.md` |
| `DONE-QA-verify-` | QA verification complete | `DONE-QA-verify-BUG-001.md` |

**Rationale**: Prefixes prevent work from being redone due to unclear status. Unprefixed files like `BUG-001.md` or `ANALYSIS-*.md` are ambiguous—are they active or complete?

### Naming Patterns

```
TODO-[TYPE]-[ID]-[description].md
DONE-[TYPE]-[ID]-[description].md
TODO-QA-verify-[TYPE]-[ID].md
```

**Common Types**:
- `BUG` - Bug fixes
- `EPIC` - Multi-sprint initiatives
- `STORY` - User-facing features
- `SPRINT` - Sprint planning and tracking
- `ANALYSIS` - Investigation results
- `ODYSSEY` - Long-term exploration
- `PERFORMANCE-AUDIT` - Performance analysis
- `ADR` - Architecture Decision Record

**Examples**:
```
TODO-BUG-001-tui-waits-for-full-stream.md
DONE-BUG-001-tui-waits-for-full-stream.md
TODO-QA-verify-BUG-001.md
TODO-EPIC-001-TUI-reactive-overhaul.md
TODO-SPRINT-00-foundation.md
```

### The TODO → DONE Lifecycle

Files transition **in place** through these states:

```
Creation → Active Work → Complete → QA Verify → Done
    ↓           ↓            ↓           ↓          ↓
(unnamed) → TODO-xxxx → TODO-xxxx → TODO-QA-  → DONE-xxxx
                                     verify-     (+ DONE-QA-)
```

1. **Creation**: File starts as `TODO-xxxx.md`
2. **Active Work**: Tasks are completed, checkboxes marked
3. **Complete**: All tasks done, ready for verification
4. **QA Verify**: `TODO-QA-verify-xxxx.md` created (see [QA Verification Workflow](#qa-verification-workflow))
5. **Done**: Both files renamed to `DONE-xxxx.md` and `DONE-QA-verify-xxxx.md`

### Migration of Unprefixed Files

If a file in `progress/` lacks `TODO-` or `DONE-` prefix:

1. **Assess Status**: Does it have pending work?
   - Yes → Rename to `TODO-xxxx.md`
   - No → Rename to `DONE-xxxx.md`
   - Obsolete → Rename to `DONE-xxxx.md` and mark as obsolete in header

2. **Update References**: Update any links in other files

3. **Create QA Task** (if pending): Add `TODO-QA-verify-xxxx.md` if work needs verification

**Example Migration**:
```bash
# Unprefixed file with pending work
git mv progress/BUG-001-tui-streaming.md progress/TODO-BUG-001-tui-streaming.md

# Unprefixed file that's complete
git mv progress/ANALYSIS-blocking-await.md progress/DONE-ANALYSIS-blocking-await.md

# Obsolete file
git mv progress/DELIVERABLES.txt progress/DONE-DELIVERABLES.txt
# (Mark as obsolete in file header)
```

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

## File Navigation

### Parent/Sibling/Children Links

**REQUIRED**: Every TODO file MUST include navigation links at the top for quick traversal of related work.

**Location**: Immediately after the file header (title, status, metadata)

**Format**:

```markdown
# TODO-EPIC-001-TUI-Reactive-Overhaul

**Status**: 🟢 Active
**Created**: 2026-01-02
**Priority**: P0

---

## Navigation

**Parent**: [TODO-main.md](TODO-main.md)
**Siblings**: [TODO-EPIC-002-ARCHITECTURE-REVIEW.md](TODO-EPIC-002-ARCHITECTURE-REVIEW.md)
**Children**:
- [TODO-SPRINT-00-foundation.md](TODO-SPRINT-00-foundation.md)
- [TODO-101-create-reactive-branch.md](TODO-101-create-reactive-branch.md)
- [TODO-102-add-reactive-dependencies.md](TODO-102-add-reactive-dependencies.md)

**QA Verification**: [TODO-QA-verify-EPIC-001.md](TODO-QA-verify-EPIC-001.md) _(when complete)_

---
```

### Navigation Relationship Types

| Relationship | Definition | Example |
|--------------|------------|---------|
| **Parent** | File that spawned or owns this work | EPIC → SPRINT has parent EPIC |
| **Siblings** | Related files at same hierarchy level | Multiple EPICs are siblings under TODO-main |
| **Children** | Files spawned from this work | EPIC has children SPRINTs, STORYs, BUGs |
| **QA Verification** | Associated QA verification task | TODO-BUG-001 → TODO-QA-verify-BUG-001 |

### Hierarchy Relationships

```
TODO-main.md (root index)
    ├── TODO-EPIC-001.md (parent of sprints)
    │   ├── TODO-SPRINT-00.md (child of epic, parent of tasks)
    │   │   ├── TODO-101.md (child of sprint)
    │   │   └── TODO-102.md (child of sprint)
    │   └── TODO-SPRINT-01.md (sibling of sprint-00)
    ├── TODO-EPIC-002.md (sibling of epic-001)
    └── TODO-BUG-001.md (standalone, parent is TODO-main)
        └── TODO-QA-verify-BUG-001.md (child of BUG-001)
```

### Navigation Rules

1. **Root Files** (`TODO-main.md`, `TODO-next.md`): No parent, list all epics as children
2. **Epic Files**: Parent is `TODO-main.md`, children are sprints/stories
3. **Sprint Files**: Parent is epic, children are tasks/bugs fixed in sprint
4. **Task/Bug Files**: Parent is sprint or epic, children are subtasks
5. **QA Verification Files**: Parent is the file being verified, no children

### Navigation Link Maintenance

**When creating a new file**:
1. Add navigation section with parent link
2. Update parent file to add this as a child
3. List relevant siblings (same parent, related work)

**When completing work**:
1. Create `TODO-QA-verify-xxxx.md` and link to it
2. Keep navigation links intact (helps track history)

**When moving to DONE**:
1. Preserve navigation links (DONE files are documentation)
2. Remove from parent's children list (or move to "Completed" section)

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

## QA Verification Workflow

### The QA Gate

When a TODO file's tasks are complete, it enters QA verification **before** moving to DONE status. This ensures:
1. Work actually meets the stated requirements
2. Tests exist and pass
3. Documentation is updated
4. No regressions introduced

### Creating a QA Verification Task

**Trigger**: All subtasks in `TODO-xxxx.md` are marked complete

**Action**: Create `TODO-QA-verify-xxxx.md` adjacent to the completed file

**Format**:

```markdown
# TODO-QA-verify-BUG-001

**Parent**: [TODO-BUG-001-tui-waits-for-full-stream.md](TODO-BUG-001-tui-waits-for-full-stream.md)
**Created**: 2026-01-03
**QA Team**: QA Engineer, Chaos Monkey Intern, Hacker (supervised by Architect)

---

## Verification Checklist

### Requirements Verification
- [ ] All tasks in parent TODO file are complete
- [ ] Stated goal is achieved
- [ ] No scope creep included

### Testing Verification
- [ ] Unit tests exist for new code
- [ ] Integration tests pass
- [ ] No tests disabled without justification
- [ ] Manual testing performed (if applicable)

### Quality Verification
- [ ] Code follows project patterns
- [ ] No principle violations (PRINCIPLE-*.md)
- [ ] No security regressions
- [ ] Documentation updated

### Build Verification
- [ ] Builds pass (0 errors)
- [ ] Warnings are justified or fixed
- [ ] Checksums regenerated (if .sh files changed)

---

## Test Results

[QA team fills in results of testing]

---

## Approval

**QA Engineer**: [ ] Approved
**Hacker**: [ ] No security issues
**Architect**: [ ] Approved for DONE status

---

**When all checkboxes are complete**:
1. Rename `TODO-BUG-001.md` → `DONE-BUG-001.md`
2. Rename `TODO-QA-verify-BUG-001.md` → `DONE-QA-verify-BUG-001.md`
```

### QA Team Authority

**CRITICAL**: QA verification files have special authority:

| Team Role | Authority |
|-----------|-----------|
| **QA Engineer** | Can mark TODO-QA-verify-xxxx as DONE |
| **Chaos Monkey Intern** | Stress testing, edge cases |
| **Hacker** | Security regression review |
| **Architect** | Final approval for DONE transition |

**Self-Completion Rule**: When all subtasks in `TODO-QA-verify-xxxx.md` are complete and approved:
1. QA team can rename `TODO-QA-verify-xxxx.md` → `DONE-QA-verify-xxxx.md`
2. QA team can rename parent `TODO-xxxx.md` → `DONE-xxxx.md`
3. Update parent file's navigation links
4. Update siblings to reflect completion

### QA Workflow Diagram

```
Development Complete
        ↓
   TODO-xxxx.md (all tasks done)
        ↓
   Create TODO-QA-verify-xxxx.md
        ↓
   QA Team Verification
   ├─ Requirements met?
   ├─ Tests pass?
   ├─ Quality checks?
   └─ Security review?
        ↓
   All Approved?
   ├─ YES → DONE-xxxx.md + DONE-QA-verify-xxxx.md
   └─ NO → Add fixes to TODO-xxxx.md, restart
```

### Verification Failure Handling

If QA verification finds issues:

1. **Add subtasks to parent `TODO-xxxx.md`** - Do NOT mark as DONE
2. **Document failures in QA file** - What didn't work, why
3. **Re-trigger development** - Fix the issues
4. **Re-verify** - QA team reviews fixes

**Example**:

```markdown
# TODO-QA-verify-BUG-001

## Verification Results

### ❌ FAILED - Testing Verification
- [x] Unit tests exist ✅
- [ ] Integration tests pass ❌ - TUI still hangs on slow network
- [ ] Manual testing performed ❌ - Not tested with large responses

## Issues Found

1. **TUI hangs on slow network** - Need network latency simulation test
2. **Large responses (>10KB) still batch** - Buffer size issue

## Action Required

Added to parent TODO-BUG-001.md:
- [ ] Add network latency test (slow.localhost:11434)
- [ ] Fix buffer batching for responses >10KB

**Status**: Returned to development, QA verification pending fixes
```

### When to Skip QA Verification

QA verification can be skipped for:
- **Documentation-only changes** (typo fixes, clarifications)
- **Trivial refactors** (renaming variables, no logic changes)
- **Methodology updates** (this file!)

**Rule**: If it changes behavior, QA verifies it.

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

## Checkpointing: Session Resume System

### The Checkpoint Pattern

When working on complex tasks that may span multiple sessions (e.g., before system reboot, end of day), use **checkpoints** to preserve context and enable seamless resume.

**Checkpoint Location**: Root `TODO.md` file (not in progress/)

### Making a Checkpoint

When the user says **"make a checkpoint"**, create or replace `TODO.md` with:

1. **Current Session Summary** - What you were working on
2. **Active TODO References** - Links to relevant TODO-*.md, BUG-*.md, EPIC-*.md files
3. **Key Findings** - Important discoveries or root causes identified
4. **Next Steps** - Clear continuation points
5. **Context** - File locations, line numbers, specific variables/functions

**Template**:

```markdown
# Session Checkpoint

**Created**: YYYY-MM-DD HH:MM
**Context**: [Brief description of what you were doing]

---

## Current Work

[2-3 sentences describing the immediate task]

## Active TODO Files

- **Primary**: `progress/TODO-XXX.md` - [Brief status]
- **Related**: `progress/BUG-XXX.md` - [Brief status]
- **Epic**: `progress/EPIC-XXX.md` - [Current phase]

## Key Findings

[Bulleted list of important discoveries, root causes, or decisions]

## Next Steps

1. [Clear action item with file reference]
2. [Clear action item with file reference]
3. [Clear action item with file reference]

## Important Context

**Files Modified**:
- `path/to/file.rs:line` - [What was changed]

**Commands Run**:
```bash
./command --flags
```

**Critical Values**:
- Variable X = Y
- Function Z at line N
```

### Resuming from Checkpoint

When the user says **"resume from @TODO.md"**:

1. Read `TODO.md` to understand context
2. Read referenced TODO/BUG/EPIC files for full details
3. Continue work from Next Steps
4. Update checkpoint when reaching new milestone

### Checkpoint Lifecycle

- **Created**: At user request ("make a checkpoint")
- **Updated**: When major findings occur or before long breaks
- **Deleted**: When session work is fully complete and committed
- **Not Tracked**: `TODO.md` should be in `.gitignore` (ephemeral)

### Checkpoint vs. TODO Files

| Aspect | TODO.md (Checkpoint) | progress/TODO-*.md |
|--------|---------------------|-------------------|
| **Lifespan** | Single session or day | Entire feature/epic |
| **Scope** | Cross-cutting current work | Specific feature/bug |
| **Detail** | High (line numbers, variables) | Medium (phases, tasks) |
| **Tracked** | No (gitignored) | Yes (committed) |
| **Audience** | AI resuming work | Team + AI |

### Example Checkpoint

```markdown
# Session Checkpoint

**Created**: 2026-01-03 18:30
**Context**: Investigating TUI streaming bug root cause

---

## Current Work

Just completed investigation of BUG-001 (TUI waits for full stream). Root cause identified: blocking `rx.recv().await` in event loop causes 2-5 second freeze during model loading. One-line fix ready to implement.

## Active TODO Files

- **Primary**: `progress/TODO-BUG-001-tui-waits-for-full-stream.md` - ✅ Investigation complete, ready for fix
- **Related**: `progress/EPIC-001-TUI-reactive-overhaul.md` - Sprint 0 merged to main
- **Context**: `progress/TODO-104-document-migration-patterns.md` - Migration guide complete

## Key Findings

- **Root Cause**: `conductor.rs:1034` uses blocking `rx.recv().await` in polling loop
- **Impact**: Event loop freezes for 2-5 seconds during GPU model loading
- **Fix**: Change ONE line from `rx.recv().await` to `rx.try_recv()`
- **Why It Works**: Non-blocking check allows event loop to continue at 10 FPS
- **Workaround Available**: `--interactive` flag bypasses TUI, uses bash interface

## Next Steps

1. Implement the one-line fix in `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/conductor.rs:1034`
2. Run build verification: `./yollayah/yollayah-build-logs.sh --all`
3. Test with: `./yollayah.sh` (TUI should now stream smoothly)
4. Update TODO-BUG-001 status to FIXED when verified

## Important Context

**Files to Modify**:
- `yollayah/conductor/core/src/conductor.rs:1034` - Change `rx.recv().await` to `rx.try_recv()`

**Test Commands**:
```bash
./yollayah.sh --test-interactive  # Fast integration test
./yollayah.sh --interactive       # Bash interface (working)
./yollayah.sh                     # TUI (will test fix)
```

**Critical Function**: `poll_streaming()` in conductor.rs
**Error Pattern**: Blocking await in polling loop violates PRINCIPLE-efficiency.md
```

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-01-02 | Initial methodology documentation |
| 1.1 | 2026-01-03 | Added checkpointing system for session resume |
| 1.2 | 2026-01-03 | Added TODO-/DONE- naming standards, QA verification workflow, Parent/Sibling/Children navigation |

---

## See Also

- `workflows/sprint.md` - Sprint workflow and phases
- `TODO-next.md` - Current sprint priorities
- `TODO-main.md` - General backlog and TODO file index
- `TODO-security-findings.md` - Security issue tracking
- `TODO.md` - Current session checkpoint (ephemeral, not tracked)
