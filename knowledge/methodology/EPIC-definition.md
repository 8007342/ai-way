# Epic Workflow

> High-level guide for planning and executing Epics - major development phases focused on significant features.

## Overview

An Epic is a milestone-based development phase that introduces and stabilizes a significant feature. Unlike time-boxed sprints, Epics complete when the feature reaches stability - not after a fixed duration.

**Key Characteristics**:
- **Milestone-based**: Completion is defined by feature stability, not time
- **Multi-sprint scope**: Epics span multiple sprints to achieve their goal
- **Cross-functional**: Requires input from architecture, UX, security, legal, and QA
- **Test-driven**: Tests are planned upfront and guide development

---

## Epic vs Sprint

| Aspect | Sprint | Epic |
|--------|--------|------|
| Scope | Focused work items | Major feature introduction |
| Duration | Fixed time-box | Variable (until stable) |
| Completion | Time expires | Milestone achieved |
| Planning | Quick triage | Full team audit |
| Tests | Written during dev | Planned upfront (even if failing) |

Epics contain multiple sprints. Each sprint advances the Epic toward its stability milestone.

---

## Epic Planning Process

### Planning Team

During Epic planning, the following roles gather for collaborative planning:

| Role | Responsibilities |
|------|------------------|
| **Architect** | System design, integration points, technical constraints |
| **UX Specialist** | User experience, accessibility, interaction patterns |
| **Lawyer** | Compliance requirements, licensing, legal constraints |
| **QA** | Test coverage strategy, quality gates, acceptance criteria |
| **Security Specialist** | Threat modeling, security requirements, hardening |
| **Developers (3)** | Implementation feasibility, effort estimation, technical input |

### Planning Checklist

#### 1. System State Audit

- [ ] Review current codebase architecture
- [ ] Identify components affected by the new feature
- [ ] Document existing technical debt in affected areas
- [ ] List integration points with external systems
- [ ] Assess current test coverage in affected areas

#### 2. TODO Item Triage

- [ ] Review all existing TODO files
- [ ] Categorize items by relevance to the Epic
- [ ] Identify blockers that must be resolved first
- [ ] Flag items that become obsolete with the new feature
- [ ] Note items that can be addressed opportunistically

#### 3. Security Triage

- [ ] Perform threat modeling for the new feature
- [ ] Review authentication/authorization requirements
- [ ] Identify sensitive data handling needs
- [ ] Document required security controls
- [ ] Update TODO-security-findings.md with new concerns

#### 4. Test Planning

- [ ] Define unit test strategy and coverage targets
- [ ] Plan integration test scenarios
- [ ] Identify edge cases and error conditions
- [ ] Document acceptance criteria
- [ ] Create test file stubs (may be failing/disabled initially)

#### 5. TODO File Updates

- [ ] Update existing TODO files with findings
- [ ] Remove irrelevant or out-of-scope items
- [ ] Create new TODO files for additional findings
- [ ] Update TODO-next.md with plans for at least 3 sprints

---

## Epic Lifecycle

### Phase 1: Planning

**Entry Criteria**:
- Feature concept approved
- Planning team available
- Current sprints completed or paused

**Activities**:
1. Gather planning team
2. Conduct system state audit
3. Perform security threat modeling
4. Design test strategy
5. Create Epic TODO file
6. Plan initial sprints (minimum 3)

**Exit Criteria**:
- Epic TODO file created with naming convention
- Test stubs committed (disabled if failing)
- TODO-next.md updated with sprint plans
- All planning team members have signed off

### Phase 2: Sprint Execution

**Entry Criteria**:
- Planning phase complete
- Epic TODO file exists
- First sprint items identified

**Activities**:
1. Execute sprints following [Sprint Workflow](./sprint.md)
2. Track progress in Epic TODO file
3. Enable and fix tests as features stabilize
4. Conduct regular check-ins with planning team
5. Adjust scope based on discoveries

**Exit Criteria**:
- Core feature functionality implemented
- Primary use cases working
- No critical bugs remaining

### Phase 3: Stabilization

**Entry Criteria**:
- Core feature implemented
- Primary use cases functional

**Activities**:
1. Focus sprints on bug fixes and polish
2. Enable all remaining tests
3. Performance optimization
4. Accessibility review
5. Security hardening
6. Documentation completion

**Exit Criteria**:
- All tests passing
- No known critical or high-priority bugs
- Performance meets targets
- Accessibility requirements met
- Security review passed

### Phase 4: Completion

**Entry Criteria**:
- Stabilization phase exit criteria met

**Activities**:
1. Final documentation review
2. Update TODO-next.md (mark Epic items complete)
3. Archive Epic TODO file to completed section
4. Retrospective with planning team
5. Knowledge transfer if needed

**Exit Criteria**:
- Feature stable in production-ready state
- Documentation complete
- Epic TODO file archived
- Lessons learned documented

---

## Epic Naming Convention

Epic TODO files follow this naming pattern:

```
TODO-epic-{YYYY}Q{N}-{feature-name}.md
```

**Components**:
- `YYYY`: Four-digit year
- `Q{N}`: Quarter (Q1, Q2, Q3, Q4)
- `feature-name`: Kebab-case feature description

**Examples**:
- `TODO-epic-2026Q1-avatar-animation.md`
- `TODO-epic-2026Q2-multi-user-sessions.md`
- `TODO-epic-2026Q3-plugin-system.md`

---

## Epic TODO File Template

```markdown
# Epic: [Feature Name]

> [One-line description of the feature]

## Status

- **Phase**: Planning | Execution | Stabilization | Complete
- **Started**: YYYY-MM-DD
- **Target Completion**: YYYY-MM-DD (estimated)
- **Sprints Completed**: N

## Overview

[2-3 paragraph description of the feature, its purpose, and expected impact]

## Planning Team Sign-off

| Role | Name/ID | Date | Notes |
|------|---------|------|-------|
| Architect | | | |
| UX Specialist | | | |
| Lawyer | | | |
| QA | | | |
| Security Specialist | | | |
| Developer 1 | | | |
| Developer 2 | | | |
| Developer 3 | | | |

## Security Considerations

- [ ] Threat model reviewed
- [ ] Authentication requirements defined
- [ ] Authorization requirements defined
- [ ] Data protection requirements defined
- [ ] Security controls documented

### Identified Threats

1. [Threat description and mitigation]

## Test Strategy

### Unit Tests

- [ ] [Test area 1]: [Coverage target]
- [ ] [Test area 2]: [Coverage target]

### Integration Tests

- [ ] [Integration scenario 1]
- [ ] [Integration scenario 2]

### Test Files Created

| File | Status | Notes |
|------|--------|-------|
| tests/unit/feature_test.py | Disabled | Stub created during planning |

## Sprint Plan

### Sprint 1: [Theme]

- [ ] Item 1
- [ ] Item 2
- [ ] Item 3

### Sprint 2: [Theme]

- [ ] Item 1
- [ ] Item 2

### Sprint 3: [Theme]

- [ ] Item 1
- [ ] Item 2

## Progress Log

### Sprint N (YYYY-MM-DD)

- Completed: [items]
- Blocked: [items and reasons]
- Discoveries: [new findings]

## Completion Criteria

- [ ] All planned features implemented
- [ ] All tests passing
- [ ] Security review passed
- [ ] Performance targets met
- [ ] Documentation complete
- [ ] No critical bugs

## Notes

[Additional context, decisions, or considerations]
```

---

## Examples of Well-Planned Epics

### Example 1: Avatar Animation System

**Epic**: `TODO-epic-2026Q1-avatar-animation.md`

**Planning Highlights**:
- Architect defined sprite rendering pipeline
- UX Specialist designed animation states and transitions
- Security reviewed asset loading (no remote execution)
- QA planned visual regression tests
- Tests created for animation state machine (disabled until implementation)

**Sprint Breakdown**:
1. **Sprint 1**: Core sprite rendering, basic animations
2. **Sprint 2**: Animation state machine, transitions
3. **Sprint 3**: Performance optimization, caching
4. **Sprint 4**: Polish, edge cases, documentation

**Stabilization Focus**:
- Memory usage optimization
- Animation smoothness on low-end systems
- Accessibility (reduced motion support)

### Example 2: Plugin System

**Epic**: `TODO-epic-2026Q2-plugin-system.md`

**Planning Highlights**:
- Architect designed plugin API and lifecycle
- Security performed extensive threat modeling (sandboxing, permissions)
- Lawyer reviewed plugin licensing requirements
- QA planned isolation tests and malicious plugin scenarios
- Integration tests stubbed for plugin loading/unloading

**Sprint Breakdown**:
1. **Sprint 1**: Plugin discovery and loading
2. **Sprint 2**: Plugin API surface
3. **Sprint 3**: Sandboxing and permissions
4. **Sprint 4**: Plugin marketplace integration
5. **Sprint 5**: Documentation and examples

**Stabilization Focus**:
- Security hardening
- Error handling for malformed plugins
- Performance impact of plugin system

---

## Relationship to TODO Hierarchy

Epics integrate with the existing TODO structure:

```
TODO-next.md                         <- Sprint root (references active Epic)
  |
  +-- TODO-epic-YYYYQN-feature.md    <- Epic file (active)
  |     |
  |     +-- Sprint 1 items
  |     +-- Sprint 2 items
  |     +-- Sprint 3 items
  |
  +-- TODO-main.md                   <- General features
  +-- TODO-security-findings.md      <- Security issues
  +-- TODO-integration-testing.md    <- Test coverage
  |
  +-- completed/                     <- Archived Epics
        +-- TODO-epic-2025Q4-initial-release.md
```

---

## Quick Reference

### Starting an Epic

```
1. Gather planning team (8 roles)
2. Audit system state and existing TODOs
3. Triage security issues
4. Plan test strategy
5. Create Epic TODO file with naming convention
6. Update TODO-next.md with at least 3 sprints
7. Commit test stubs (disabled if failing)
```

### During an Epic

```
1. Execute sprints per Sprint Workflow
2. Update Epic TODO file progress
3. Enable tests as features stabilize
4. Conduct periodic planning team check-ins
5. Adjust scope based on discoveries
```

### Completing an Epic

```
1. Verify all tests passing
2. Confirm stability criteria met
3. Complete documentation
4. Archive Epic TODO file
5. Conduct retrospective
6. Update TODO-next.md
```

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-01-02 | Initial Epic workflow documentation |
