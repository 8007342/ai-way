# EPIC-001: TUI Reactive Architecture Overhaul

**Status**: 🔴 PLANNING - Architecture Evaluation Phase
**Created**: 2026-01-03
**Owner**: Architect + Rust/Async Specialist + Hacker + Crazy Intern
**Scope**: COMPLETE rewrite of TUI and Conductor async architecture
**Branch**: `feature/reactive-tui-overhaul`
**Target**: ai-way 0.2.0 (Breaking Changes Allowed)

---

## Executive Summary

**The Current TUI/Conductor architecture fundamentally violates async principles and causes memory leaks, thread pool conflicts, and poor performance.**

### The Problem

The codebase currently uses manual `.await` calls, `tokio::select!`, and `tokio::spawn` - treating async as "threading with extra steps". This causes:

1. **Memory Leaks** - Resources not properly managed by framework
2. **Thread Pool Exhaustion** - Manual threading conflicts with framework's thread pool
3. **Blocking Operations** - `.await` calls freeze the UI
4. **CPU/GPU Coordination Issues** - Manual parallelism breaks hardware scheduling
5. **No Backpressure** - Manual polling can't handle flow control

### The Solution

**Complete migration to reactive streams/observables pattern using tokio-stream and RxRust.**

- **Zero `.await` calls** - Framework manages ALL async coordination
- **Declarative composition** - Describe WHAT, not HOW
- **Framework-managed execution** - Let tokio/RxRust handle threading
- **Automatic backpressure** - Framework handles flow control
- **Clean resource management** - Framework manages cleanup

### Scope

**EVERYTHING with `.await` must be rewritten:**

- ✅ TUI event loop → Reactive stream pipeline
- ✅ Conductor message handling → Observable composition
- ✅ Token streaming → ReceiverStream with combinators
- ✅ Terminal event handling → TerminalEventStream
- ✅ Animation tick → IntervalStream
- ✅ All async fn → Stream-returning functions

**Estimated Effort**: 4-6 sprints (2-3 weeks per sprint)

---

## Principles Being Enforced

From `knowledge/principles/PRINCIPLE-efficiency.md`:

### STRICTLY FORBIDDEN

| Pattern | Status | Reason |
|---------|--------|--------|
| `.await` anywhere | ❌ **FORBIDDEN** | Manual async = thread management bugs |
| `.wait()` anywhere | ❌ **FORBIDDEN** | Blocking = defeats async |
| `tokio::select!` | ❌ **FORBIDDEN** | Manual coordination = memory leaks |
| `tokio::spawn` | ❌ **FORBIDDEN** | Manual threading = framework conflicts |
| `async fn` with manual loops | ❌ **FORBIDDEN** | Manual loops = no backpressure |

### REQUIRED

| Pattern | Status | Reason |
|---------|--------|--------|
| Reactive streams (tokio-stream) | ✅ **REQUIRED** | Framework manages everything |
| Observables (RxRust) | ✅ **PREFERRED** | Most powerful composition |
| Stream combinators (map, filter, etc) | ✅ **REQUIRED** | Declarative transformations |
| Framework-managed execution | ✅ **REQUIRED** | Let framework handle async |

---

## Architecture Comparison

### Current Architecture (WRONG)

```rust
// ❌ Manual async/await with blocking
pub async fn poll_streaming(&mut self) -> bool {
    match rx.recv().await {  // BLOCKS!
        Some(token) => { /* manual processing */ }
        None => false,
    }
}

// ❌ Manual event loop
while self.running {
    tokio::select! {  // Manual coordination
        event = event_stream.next() => { ... }
        _ = tick.tick() => { ... }
    }

    self.conductor.poll_streaming().await;  // BLOCKS!
    self.update();
    self.render(terminal)?;
}
```

**Problems**:
- Manual `.await` = fighting the framework
- Manual event loop = no framework optimization
- Blocking operations = UI freezes
- Memory leaks = poor resource management

### Target Architecture (CORRECT)

```rust
// ✅ Reactive stream composition (NO .await!)
pub fn create_event_pipeline() -> impl Stream<Item = AppEvent> {
    let token_stream = ReceiverStream::new(token_rx)
        .map(AppEvent::Token);

    let terminal_stream = TerminalEventStream::new()
        .map(AppEvent::Terminal);

    let tick_stream = IntervalStream::new(interval(Duration::from_millis(100)))
        .map(|_| AppEvent::Tick);

    // Framework merges streams (NO manual coordination!)
    select_all(vec![
        token_stream.boxed(),
        terminal_stream.boxed(),
        tick_stream.boxed(),
    ])
}

// ✅ Reactive event handler (NO .await, NO manual loop!)
pub fn run_app(mut app: App) {
    create_event_pipeline()
        .for_each(|event| {
            app.handle_event(event);
            app.update();
            app.render();
            futures::future::ready(())  // Framework manages execution
        })
        .run();  // Framework runs everything
}
```

**Advantages**:
- No `.await` - framework manages async
- Declarative - describe data flow
- Automatic backpressure
- No memory leaks
- Framework-optimized execution

---

## Sprint Structure (By Complexity & Interdependence)

### Sprint 0: Foundation & Planning (1 week)
**Dependencies**: None
**Complexity**: LOW (Planning & Setup)

**Goals**:
- Create feature branch
- Evaluate fresh start vs incremental migration
- Set up RxRust + tokio-stream dependencies
- Create prototype reactive event pipeline
- Document migration patterns
- Disable integration tests on branch

**Deliverables**:
- Branch `feature/reactive-tui-overhaul` created
- Architecture decision document
- Prototype working with minimal reactive pipeline
- Migration guide for team

**Stories**:
- STORY-001: Create feature branch and disable tests
- STORY-002: Add RxRust and tokio-stream dependencies
- STORY-003: Prototype minimal reactive event pipeline
- STORY-004: Architecture evaluation (fresh start vs migration)

---

### Sprint 1: Core Reactive Infrastructure (2 weeks)
**Dependencies**: Sprint 0 complete
**Complexity**: MEDIUM (New patterns, foundational)

**Goals**:
- Build reactive stream wrappers for all event sources
- Create stream combinators for common operations
- Implement backpressure handling
- Build testing infrastructure for reactive code

**Deliverables**:
- `ReceiverStream` wrappers for all channels
- `IntervalStream` for animations/ticks
- `TerminalEventStream` for crossterm events
- Stream combinator utilities
- Reactive test harness

**Stories**:
- STORY-005: ReceiverStream wrapper for token channels
- STORY-006: IntervalStream for animation ticks
- STORY-007: TerminalEventStream for crossterm
- STORY-008: Stream combinator utilities (merge, filter, scan)
- STORY-009: Reactive testing infrastructure

**Key Files to Create**:
- `yollayah/core/surfaces/tui/src/reactive/mod.rs`
- `yollayah/core/surfaces/tui/src/reactive/streams.rs`
- `yollayah/core/surfaces/tui/src/reactive/combinators.rs`
- `yollayah/core/surfaces/tui/src/reactive/testing.rs`

---

### Sprint 2: TUI Event Loop Rewrite (2 weeks)
**Dependencies**: Sprint 1 complete
**Complexity**: HIGH (Core event loop, critical path)

**Goals**:
- Replace `app.rs` event loop with reactive pipeline
- Eliminate ALL `.await` calls in TUI
- Implement declarative event handling
- Verify UI responsiveness

**Deliverables**:
- TUI runs on pure reactive pipeline
- Zero `.await` calls in TUI codebase
- 10 FPS rendering maintained
- No UI freezing

**Stories**:
- STORY-010: Rewrite main event loop to reactive pipeline
- STORY-011: Convert event handlers to stream processors
- STORY-012: Eliminate blocking operations
- STORY-013: Performance validation (10 FPS, no freezes)

**Key Files to Rewrite**:
- `yollayah/core/surfaces/tui/src/app.rs` - COMPLETE REWRITE
- `yollayah/core/surfaces/tui/src/events.rs` - Reactive event types

**Success Criteria**:
- `grep -r "\.await" yollayah/core/surfaces/tui/src` returns 0 matches
- UI renders at 10 FPS consistently
- No blocking during model loading

---

### Sprint 3: Conductor Message Pipeline (2 weeks)
**Dependencies**: Sprint 1 complete (can run parallel with Sprint 2)
**Complexity**: HIGH (Core business logic, message routing)

**Goals**:
- Rewrite Conductor message handling to observables
- Eliminate `.await` in Conductor
- Implement reactive routing
- Handle backpressure for token streams

**Deliverables**:
- Conductor uses pure observable patterns
- Zero `.await` in Conductor
- Automatic backpressure handling
- Clean resource management

**Stories**:
- STORY-014: Convert Conductor message loop to observables
- STORY-015: Reactive token streaming pipeline
- STORY-016: Observable-based routing
- STORY-017: Backpressure implementation
- STORY-018: Resource cleanup validation

**Key Files to Rewrite**:
- `yollayah/conductor/core/src/conductor.rs` - COMPLETE REWRITE
- `yollayah/conductor/core/src/backend/ollama.rs` - Observable streaming
- `yollayah/conductor/core/src/routing/` - Reactive routing

**Success Criteria**:
- `grep -r "\.await" yollayah/conductor/core/src` returns 0 matches (excluding tests)
- Memory usage stays flat during streaming
- No CPU spikes from batch processing

---

### Sprint 4: Integration & Polish (2 weeks)
**Dependencies**: Sprint 2 AND Sprint 3 complete
**Complexity**: MEDIUM (Integration, edge cases)

**Goals**:
- Connect reactive TUI to reactive Conductor
- Handle error cases reactively
- Performance optimization
- Re-enable integration tests

**Deliverables**:
- Full reactive pipeline end-to-end
- Error handling via reactive streams
- Performance meets/exceeds targets
- Integration tests passing

**Stories**:
- STORY-019: Connect TUI and Conductor reactive pipelines
- STORY-020: Reactive error handling
- STORY-021: Performance profiling and optimization
- STORY-022: Re-enable and fix integration tests
- STORY-023: Documentation update

**Success Criteria**:
- All integration tests pass
- Performance budgets met (see PRINCIPLE-efficiency.md)
- Zero `.await` violations
- Memory leaks eliminated

---

### Sprint 5: Advanced Features & Optimization (2 weeks)
**Dependencies**: Sprint 4 complete
**Complexity**: LOW-MEDIUM (Enhancements, optimization)

**Goals**:
- Advanced reactive patterns (retry, debounce, throttle)
- Animation system optimization
- GPU/CPU coordination via reactive streams
- Stress testing

**Deliverables**:
- Advanced reactive operators
- Optimized animation pipeline
- Hardware coordination via streams
- Stress test suite passing

**Stories**:
- STORY-024: Retry/exponential backoff operators
- STORY-025: Debounce/throttle for user input
- STORY-026: Reactive animation system
- STORY-027: GPU/CPU coordination via observables
- STORY-028: Stress testing and chaos engineering

**Key Features**:
- Retry logic for network failures
- Input debouncing for performance
- Smooth animation via reactive streams
- Proper GPU scheduling

---

### Sprint 6: Production Readiness (1 week)
**Dependencies**: Sprint 5 complete
**Complexity**: LOW (Polish, documentation, merge)

**Goals**:
- Final performance validation
- Complete documentation
- Code review and approval
- Merge to main

**Deliverables**:
- Production-ready reactive architecture
- Complete migration guide
- Updated PRINCIPLE-efficiency.md (Law 1 rewrite)
- Merged to main

**Stories**:
- STORY-029: Final performance validation
- STORY-030: Complete reactive architecture documentation
- STORY-031: Team code review and approval
- STORY-032: Merge to main (breaking release)

**Success Criteria**:
- All tests pass on main
- Performance budgets met
- Zero `.await` violations
- Memory leaks eliminated
- Team sign-off

---

## Risk Assessment

### High Risk

1. **Learning Curve** - Team unfamiliar with reactive patterns
   - **Mitigation**: Sprint 0 prototype, pair programming, expert consultation

2. **Scope Creep** - "While we're at it" refactors
   - **Mitigation**: Strict scope control, feature freeze on branch

3. **Performance Regression** - Reactive overhead
   - **Mitigation**: Continuous performance testing, profiling

### Medium Risk

1. **Breaking Changes** - Existing code depends on current API
   - **Mitigation**: Version 0.2.0, breaking changes documented

2. **Testing Gaps** - Reactive code needs different testing approach
   - **Mitigation**: Sprint 1 builds reactive test infrastructure

### Low Risk

1. **Framework Bugs** - tokio-stream or RxRust issues
   - **Mitigation**: Well-established libraries, active communities

---

## Success Metrics

### Performance Targets

From `PRINCIPLE-efficiency.md`:

**TUI**:
- ✅ Idle CPU: < 0.1%
- ✅ Active CPU: < 5% during streaming
- ✅ Frame rate: 10 FPS consistent
- ✅ Memory: < 50 MB baseline, < 100 MB streaming
- ✅ Allocations: < 1000/sec rendering, < 100/sec idle

**Conductor**:
- ✅ Idle CPU: < 1%
- ✅ Memory: < 200 MB baseline
- ✅ Latency: < 1ms message passing

### Code Quality Targets

- ✅ Zero `.await` calls (enforced by grep in CI)
- ✅ Zero memory leaks (validated by valgrind/heaptrack)
- ✅ 100% test coverage for reactive pipelines
- ✅ All integration tests passing
- ✅ Architectural enforcement tests passing

### User Experience Targets

- ✅ No UI freezing (ever)
- ✅ Smooth streaming (tokens appear as generated)
- ✅ Responsive to user input (< 100ms)
- ✅ Graceful error handling
- ✅ Clean shutdown

---

## Decision Points

### Decision 1: Fresh Start vs Incremental Migration

**Question**: Rewrite from scratch or migrate incrementally?

**Options**:
1. **Fresh Start** - New TUI/Conductor from ground up
   - Pros: Clean slate, no legacy baggage, faster
   - Cons: Higher risk, more work upfront

2. **Incremental Migration** - Convert file by file
   - Pros: Lower risk, incremental validation
   - Cons: Complex coordination, legacy code interference

**Decision**: TBD in Sprint 0 (STORY-004)
**Owner**: Architect + Rust/Async Specialist

### Decision 2: RxRust vs Pure tokio-stream

**Question**: Which reactive framework to use?

**Options**:
1. **RxRust** - Full reactive extensions
   - Pros: Most powerful, rich operators, well-tested patterns
   - Cons: Heavier dependency, learning curve

2. **Pure tokio-stream** - Minimal, tokio-native
   - Pros: Lighter, integrates with tokio, simpler
   - Cons: Fewer operators, more manual composition

**Decision**: TBD in Sprint 0 (STORY-002)
**Owner**: Rust/Async Specialist

**Recommendation**: Start with RxRust for power, can always simplify later

---

## Team Assignments

### Sprint 0: All Hands
- **Architect**: Architecture evaluation, decision facilitation
- **Rust/Async Specialist**: Reactive patterns research, prototype
- **Hacker**: Risk assessment, edge case identification
- **Crazy Intern**: Fresh perspective, "what if we just..." questions

### Sprint 1-3: Parallel Teams
- **Team A (TUI)**: Architect + Crazy Intern
  - Focus: TUI event loop rewrite
- **Team B (Conductor)**: Rust/Async Specialist + Hacker
  - Focus: Conductor reactive pipeline

### Sprint 4-6: Integration
- **All Hands**: Integration, testing, polish, review

---

## Related Documents

**Principles**:
- `knowledge/principles/PRINCIPLE-efficiency.md` - The law we're enforcing
- `knowledge/requirements/REQUIRED-separation.md` - TUI/Conductor boundaries

**Bug Tracking**:
- `progress/TODO-BUG-001-tui-waits-for-full-stream.md` - The bug that triggered this
- `progress/ANALYSIS-blocking-await-anti-pattern-triage.md` - Codebase audit

**Sprints** (To Be Created):
- `progress/SPRINT-00-foundation.md`
- `progress/SPRINT-01-reactive-infrastructure.md`
- `progress/SPRINT-02-tui-rewrite.md`
- `progress/SPRINT-03-conductor-rewrite.md`
- `progress/SPRINT-04-integration.md`
- `progress/SPRINT-05-advanced-features.md`
- `progress/SPRINT-06-production.md`

---

## Timeline

**Total Estimated Duration**: 12-14 weeks

| Sprint | Duration | End Date (Est) | Milestone |
|--------|----------|----------------|-----------|
| Sprint 0 | 1 week | Week 1 | Architecture decided, branch created |
| Sprint 1 | 2 weeks | Week 3 | Reactive infrastructure ready |
| Sprint 2 | 2 weeks | Week 5 | TUI reactive (parallel with Sprint 3) |
| Sprint 3 | 2 weeks | Week 5 | Conductor reactive (parallel with Sprint 2) |
| Sprint 4 | 2 weeks | Week 7 | Integration complete |
| Sprint 5 | 2 weeks | Week 9 | Advanced features done |
| Sprint 6 | 1 week | Week 10 | Merged to main |

**Target Release**: ai-way 0.2.0 (Breaking Changes)

---

## Next Actions

1. **Architect + Specialist**: Evaluate fresh start vs migration (Sprint 0, Story 4)
2. **Create Sprint documents**: Generate detailed sprint plans
3. **Create Story documents**: Break down each story into tasks
4. **Create TODO tasks**: Actionable work items for Sprint 0
5. **Create feature branch**: `feature/reactive-tui-overhaul`
6. **Begin Sprint 0**: Prototype and architecture evaluation

---

**Status**: 🔴 PLANNING - Awaiting architecture evaluation and sprint document creation

**Last Updated**: 2026-01-03
