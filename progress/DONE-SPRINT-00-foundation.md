# SPRINT 00: Foundation & Planning

**EPIC**: EPIC-001-TUI-reactive-overhaul.md
**Status**: 🟡 READY TO START
**Duration**: 1 week
**Start Date**: TBD
**End Date**: TBD
**Complexity**: LOW (Planning & Setup)
**Dependencies**: None

---

## Sprint Goals

1. ✅ Create feature branch for reactive overhaul
2. ✅ Evaluate fresh start vs incremental migration approach
3. ✅ Set up RxRust + tokio-stream dependencies
4. ✅ Build prototype reactive event pipeline
5. ✅ Document migration patterns for team
6. ✅ Disable integration tests on branch (dev-only)

---

## Team Assignment

**All Hands** - This is a planning sprint requiring full team input

- **Architect**: Lead architecture evaluation, facilitate decisions
- **Rust/Async Specialist**: Research reactive patterns, build prototype
- **Hacker**: Risk assessment, identify edge cases and gotchas
- **Crazy Intern**: Fresh perspective, challenge assumptions

---

## Stories

### STORY-001: Create Feature Branch and Disable Tests
**Owner**: Hacker
**Effort**: 1 hour
**Complexity**: TRIVIAL

**Objective**: Set up isolated development environment

**Tasks**:
1. Create branch `feature/reactive-tui-overhaul` from `main`
2. Update CI config to skip integration tests on this branch
3. Add README to branch explaining it's under active development
4. Document branch merge criteria

**Acceptance Criteria**:
- Branch created and pushed
- CI runs unit tests only (no integration tests)
- README.md updated with branch status

**Files to Modify**:
- `.github/workflows/` or CI config
- `README.md` (add development branch notice)

---

### STORY-002: Add RxRust and tokio-stream Dependencies
**Owner**: Rust/Async Specialist
**Effort**: 2 hours
**Complexity**: LOW

**Objective**: Add reactive framework dependencies

**Tasks**:
1. Research latest stable versions of RxRust and tokio-stream
2. Add to `yollayah/core/surfaces/tui/Cargo.toml`
3. Add to `yollayah/conductor/core/Cargo.toml`
4. Verify builds succeed with new dependencies
5. Create minimal "hello world" reactive example

**Dependencies to Add**:
```toml
[dependencies]
# Reactive streams
tokio-stream = { version = "0.1", features = ["sync", "time"] }
futures = "0.3"

# Optional: RxRust (decision pending)
rxrust = { version = "1.0", optional = true }
```

**Acceptance Criteria**:
- Dependencies added to both TUI and Conductor
- Builds succeed
- Minimal reactive example compiles and runs

**Decision Point**: RxRust vs pure tokio-stream
- Document pros/cons in this story
- Make recommendation to Architect

---

### STORY-003: Prototype Minimal Reactive Event Pipeline
**Owner**: Rust/Async Specialist
**Effort**: 1 day
**Complexity**: MEDIUM

**Objective**: Prove reactive pattern works for our use case

**Tasks**:
1. Create `yollayah/core/surfaces/tui/examples/reactive_prototype.rs`
2. Implement minimal reactive pipeline:
   - Terminal event stream
   - Interval tick stream
   - Message receiver stream
3. Merge streams with `select_all`
4. Process events reactively (no `.await`)
5. Measure performance (CPU, memory, responsiveness)

**Prototype Requirements**:
- Zero `.await` calls in event processing
- 10 FPS tick rate maintained
- Handles 1000 events/sec without backpressure issues
- Clean shutdown

**Code Structure**:
```rust
// examples/reactive_prototype.rs

use tokio_stream::{StreamExt, wrappers::*};
use futures::stream::select_all;

fn create_event_pipeline() -> impl Stream<Item = Event> {
    let terminal_stream = /* terminal events */;
    let tick_stream = /* 10 FPS ticks */;
    let message_stream = /* simulated messages */;

    select_all(vec![
        terminal_stream.boxed(),
        tick_stream.boxed(),
        message_stream.boxed(),
    ])
}

fn main() {
    // NO .await in main event processing!
    create_event_pipeline()
        .for_each(|event| {
            handle_event(event);
            futures::future::ready(())
        })
        .run();
}
```

**Acceptance Criteria**:
- Prototype runs successfully
- Performance meets targets (documented)
- No `.await` in event handling
- Code demonstrates clear migration path

---

### STORY-004: Architecture Evaluation - Fresh Start vs Migration
**Owner**: Architect + Rust/Async Specialist
**Effort**: 2 days
**Complexity**: MEDIUM (Critical Decision)

**Objective**: Decide on rewrite strategy

**Evaluation Criteria**:

#### Option 1: Fresh Start (New TUI from Scratch)

**Pros**:
- ✅ Clean slate, no legacy constraints
- ✅ Faster initial development (no careful migration)
- ✅ Correct patterns from day 1
- ✅ Easier to enforce "no .await" rule
- ✅ Can redesign state management optimally

**Cons**:
- ❌ Higher upfront risk (all or nothing)
- ❌ More code to write initially
- ❌ Need to replicate existing features
- ❌ Harder to validate incrementally

**Effort Estimate**: 4 weeks (Sprints 1-2 combined)

#### Option 2: Incremental Migration

**Pros**:
- ✅ Lower risk (validate each step)
- ✅ Keep existing features working
- ✅ Can mix old/new during transition
- ✅ Easier rollback if needed

**Cons**:
- ❌ Complex coordination between old/new code
- ❌ Legacy code interference
- ❌ Harder to enforce principles during transition
- ❌ Slower overall progress
- ❌ Risk of "good enough" stopping midway

**Effort Estimate**: 6 weeks (more coordination overhead)

#### Evaluation Tasks

1. **Code Audit**:
   - How much current TUI code is salvageable?
   - Which patterns can be adapted vs must be rewritten?
   - What state/logic can be preserved?

2. **Risk Analysis**:
   - What's the blast radius of fresh start?
   - What's the complexity of incremental migration?
   - Where are the landmines in each approach?

3. **Prototype Comparison**:
   - Build minimal TUI both ways
   - Compare code quality, maintainability
   - Measure development velocity

4. **Team Input**:
   - Hacker: Security/edge case perspective
   - Crazy Intern: "What if..." scenarios
   - Specialist: Technical feasibility

**Deliverable**: Architecture Decision Record (ADR)

Document in `progress/ADR-001-fresh-start-vs-migration.md`:
- Decision made
- Rationale
- Trade-offs considered
- Implementation plan

**Acceptance Criteria**:
- ADR document created
- Decision approved by Architect
- Team aligned on approach
- Clear implementation plan

---

### STORY-005: Document Migration Patterns
**Owner**: Rust/Async Specialist
**Effort**: 1 day
**Complexity**: LOW

**Objective**: Create migration guide for team

**Tasks**:
1. Document forbidden patterns (`.await`, `tokio::select!`, etc.)
2. Document required patterns (reactive streams, observables)
3. Create before/after examples for common cases
4. Write testing guide for reactive code
5. Create troubleshooting guide

**Migration Guide Sections**:

1. **Forbidden Patterns** (with explanations):
   - `.await` and why it's wrong
   - `tokio::select!` and alternatives
   - Manual loops vs declarative streams
   - `tokio::spawn` vs framework-managed tasks

2. **Required Patterns** (with examples):
   - ReceiverStream for channels
   - IntervalStream for ticks
   - Custom streams for event sources
   - Stream combinators (map, filter, merge, scan)
   - Error handling in streams
   - Backpressure management

3. **Common Conversions**:
   - Event loop → Event pipeline
   - Polling function → Stream processor
   - Async fn → Stream-returning fn
   - Message handler → Observable

4. **Testing Reactive Code**:
   - Stream testing patterns
   - Mocking event sources
   - Performance validation
   - Memory leak detection

**Deliverable**: `knowledge/migration/reactive-migration-guide.md`

**Acceptance Criteria**:
- Guide covers all forbidden patterns
- Guide shows how to convert each pattern
- Examples compile and run
- Team reviews and approves

---

## Sprint Deliverables

### Code

- ✅ Feature branch `feature/reactive-tui-overhaul` created
- ✅ RxRust + tokio-stream dependencies added
- ✅ Reactive prototype (`examples/reactive_prototype.rs`)
- ✅ Prototype performance validated

### Documentation

- ✅ `progress/ADR-001-fresh-start-vs-migration.md` - Architecture decision
- ✅ `knowledge/migration/reactive-migration-guide.md` - Team guide
- ✅ Branch README explaining development status

### Decisions

- ✅ Fresh start vs migration approach chosen
- ✅ RxRust vs tokio-stream decision made
- ✅ Implementation plan approved

---

## Success Criteria

- [ ] Feature branch created and CI configured
- [ ] Reactive dependencies added and building
- [ ] Prototype demonstrates viability of reactive approach
- [ ] Performance targets met in prototype
- [ ] Architecture decision made and documented
- [ ] Migration guide created and team-approved
- [ ] All team members aligned on approach

---

## Risks & Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Team unfamiliar with reactive patterns | HIGH | HIGH | Prototype + guide in Sprint 0 |
| Prototype doesn't meet performance | HIGH | LOW | Multiple framework options (RxRust/tokio-stream) |
| Decision paralysis on approach | MEDIUM | MEDIUM | Time-box evaluation to 2 days max |
| Scope creep in planning | LOW | MEDIUM | Strict sprint boundary enforcement |

---

## Next Sprint

**SPRINT-01-reactive-infrastructure.md**

**Depends On**:
- Sprint 0 complete
- Architecture decision made
- Dependencies added

**Goals**:
- Build reactive stream wrappers
- Create stream combinators
- Implement backpressure handling
- Build testing infrastructure

---

**Status**: 🟡 READY TO START
**Blocked By**: Nothing
**Blocking**: All subsequent sprints

**Last Updated**: 2026-01-03
