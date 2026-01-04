# Progress Directory Migration Plan

**Created**: 2026-01-03
**Purpose**: Prefix all files in progress/ with TODO- or DONE- according to methodology

---

## Unprefixed Files Found

### Architecture Decision Records (ADR)
1. `ADR-001-fresh-start-vs-migration.md`
   - **Assess**: Decision already made?
   - **Action**: DONE-ADR-001 (architectural decision is final)

### Analysis Files
2. `ANALYSIS-blocking-await-anti-pattern-triage.md`
   - **Assess**: Investigation complete?
   - **Action**: DONE-ANALYSIS-blocking-await (investigation completed, fix applied)

### Bug Files (NAMING COLLISION DETECTED!)
3. `BUG-001-toolbox-detection.md` ⚠️ COLLISION with TODO-BUG-001-tui-waits
   - **Status**: DISCOVERED, NOT FIXED (per file header)
   - **Action**: Renumber to TODO-BUG-004 (next available BUG number)

4. `BUG-002-toolbox-execution-command.md`
   - **Status**: Unknown
   - **Action**: Check status → TODO-BUG-005 or DONE-BUG-002

5. `BUG-003-tui-performance-regression.md`
   - **Status**: Unknown (may be superseded by TODO-BUG-001-tui-waits)
   - **Action**: Check status → DONE-BUG-003 or merge with TODO-BUG-001

6. `BUG-015-sleep-in-polling-loops.md`
   - **Status**: Unknown
   - **Action**: Check status → TODO-BUG-015 or DONE-BUG-015

7. `BUG-016-config-test-failure.md`
   - **Status**: Unknown
   - **Action**: Check status → TODO-BUG-016 or DONE-BUG-016

### Deliverables
8. `DELIVERABLES.txt`
   - **Assess**: List of deliverables tracking
   - **Action**: DONE-DELIVERABLES.txt (obsolete tracking artifact)

### Epic Files
9. `EPIC-001-TUI-reactive-overhaul.md` ⚠️ DUPLICATE of TODO-EPIC-001
   - **Status**: Sprint 0 merged per TODO.md
   - **Action**: Check if duplicate → merge or DONE-EPIC-001

10. `EPIC-002-ARCHITECTURE-REVIEW.md`
    - **Status**: Unknown
    - **Action**: Check status → TODO-EPIC-002 or DONE-EPIC-002

11. `EPIC-002-REVIEW-SUMMARY.md`
    - **Status**: Summary suggests completion
    - **Action**: DONE-EPIC-002-REVIEW-SUMMARY

### Odyssey Files
12. `ODYSSEY-tui-to-framebuffer.md`
    - **Assess**: Long-term exploration
    - **Action**: TODO-ODYSSEY-tui-to-framebuffer (ongoing exploration)

### Performance Audits
13. `PERFORMANCE-AUDIT-ASYNC.md`
    - **Status**: Audit complete?
    - **Action**: DONE-PERFORMANCE-AUDIT-ASYNC (audit findings documented)

14. `PERFORMANCE-AUDIT-COMMS.md`
    - **Status**: Audit complete?
    - **Action**: DONE-PERFORMANCE-AUDIT-COMMS (audit findings documented)

15. `PERFORMANCE-AUDIT-FRAMEBUFFER.md`
    - **Status**: Audit complete?
    - **Action**: DONE-PERFORMANCE-AUDIT-FRAMEBUFFER (audit findings documented)

### Sprint Files
16. `SPRINT-00-foundation.md` ⚠️ DUPLICATE of TODO-SPRINT-00
    - **Status**: Sprint 0 complete per TODO.md
    - **Action**: Check if duplicate → DONE-SPRINT-00-foundation

### Story Files
17. `STORY-3-COMPLETION-SUMMARY.md`
    - **Status**: COMPLETION suggests done
    - **Action**: DONE-STORY-3-COMPLETION-SUMMARY

18. `STORY-3-IMPLEMENTATION-PLAN.md`
    - **Status**: If story 3 complete, plan is done
    - **Action**: DONE-STORY-3-IMPLEMENTATION-PLAN

19. `STORY-3-INTEGRATION-DIAGRAM.md`
    - **Status**: If story 3 complete, diagram is done
    - **Action**: DONE-STORY-3-INTEGRATION-DIAGRAM

### Stress Testing
20. `STRESS_TEST_DELIVERABLE.md`
    - **Status**: Deliverable tracking
    - **Action**: Check status → TODO or DONE

21. `STRESS_TESTING.md`
    - **Status**: Stress testing plan/results
    - **Action**: Check status → TODO or DONE

---

## Naming Collision Resolution

### BUG Number Reassignment
Current collision:
- `BUG-001-toolbox-detection.md` (unprefixed)
- `TODO-BUG-001-tui-waits-for-full-stream.md` (properly prefixed)

**Resolution**:
```bash
# Renumber toolbox-detection as BUG-004
git mv progress/BUG-001-toolbox-detection.md progress/TODO-BUG-004-toolbox-detection.md

# Keep TODO-BUG-001-tui-waits as is (already properly prefixed and numbered)
# (Will be moved to DONE-BUG-001 after QA verification)

# Renumber other BUGs to avoid future collisions:
git mv progress/BUG-002-toolbox-execution-command.md progress/TODO-BUG-005-toolbox-execution-command.md
git mv progress/BUG-003-tui-performance-regression.md progress/DONE-BUG-003-tui-performance-regression.md
# (BUG-003 likely obsolete, superseded by BUG-001-tui-waits)
```

### EPIC/SPRINT Duplicate Check
- `EPIC-001-TUI-reactive-overhaul.md` vs `TODO-EPIC-001-TUI-reactive-overhaul.md`
- `SPRINT-00-foundation.md` vs `TODO-SPRINT-00-foundation.md`

**Action**: Read both to see if they're duplicates or need merging

---

## Next Steps

1. Read each uncertain file to determine TODO vs DONE status
2. Execute git mv commands in order
3. Update file headers with obsolete markers where appropriate
4. Create TODO-QA-verify-xxxx for completed work needing verification
5. Add Parent/Sibling/Children navigation to all files
6. Commit changes with detailed message

---

## Commands to Execute

```bash
# Phase 1: Simple renames (known complete)
git mv progress/ADR-001-fresh-start-vs-migration.md progress/DONE-ADR-001-fresh-start-vs-migration.md
git mv progress/ANALYSIS-blocking-await-anti-pattern-triage.md progress/DONE-ANALYSIS-blocking-await-anti-pattern-triage.md
git mv progress/DELIVERABLES.txt progress/DONE-DELIVERABLES.txt
git mv progress/EPIC-002-REVIEW-SUMMARY.md progress/DONE-EPIC-002-REVIEW-SUMMARY.md
git mv progress/PERFORMANCE-AUDIT-ASYNC.md progress/DONE-PERFORMANCE-AUDIT-ASYNC.md
git mv progress/PERFORMANCE-AUDIT-COMMS.md progress/DONE-PERFORMANCE-AUDIT-COMMS.md
git mv progress/PERFORMANCE-AUDIT-FRAMEBUFFER.md progress/DONE-PERFORMANCE-AUDIT-FRAMEBUFFER.md
git mv progress/STORY-3-COMPLETION-SUMMARY.md progress/DONE-STORY-3-COMPLETION-SUMMARY.md
git mv progress/STORY-3-IMPLEMENTATION-PLAN.md progress/DONE-STORY-3-IMPLEMENTATION-PLAN.md
git mv progress/STORY-3-INTEGRATION-DIAGRAM.md progress/DONE-STORY-3-INTEGRATION-DIAGRAM.md

# Phase 2: BUG renumbering (resolve collisions)
git mv progress/BUG-001-toolbox-detection.md progress/TODO-BUG-004-toolbox-detection.md
git mv progress/BUG-002-toolbox-execution-command.md progress/TODO-BUG-005-toolbox-execution-command.md
git mv progress/BUG-003-tui-performance-regression.md progress/DONE-BUG-003-tui-performance-regression.md

# Phase 3: Check these manually before renaming
# - BUG-015, BUG-016
# - EPIC-001, EPIC-002, SPRINT-00 (duplicates?)
# - ODYSSEY, STRESS_TEST files
```
