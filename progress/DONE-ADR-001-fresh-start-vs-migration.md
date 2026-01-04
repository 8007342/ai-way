# ADR-001: Fresh Start vs Incremental Migration for TUI Reactive Overhaul

**Status**: PROPOSED
**Date**: 2026-01-03
**Decision Makers**: Architect + Rust/Async Specialist
**Related**: EPIC-001-TUI-reactive-overhaul.md, STORY-004 (Sprint 0)

---

## Context

The current TUI and Conductor architecture fundamentally violates async principles, causing:

1. **Memory leaks** - Resources not properly managed by reactive framework
2. **Thread pool exhaustion** - Manual threading conflicts with tokio's thread pool
3. **Blocking operations** - `.await` calls freeze the UI (see BUG-001)
4. **CPU/GPU coordination issues** - Manual parallelism breaks hardware scheduling
5. **No backpressure** - Manual polling can't handle flow control

**The Goal**: Complete migration to reactive streams/observables pattern (tokio-stream + RxRust)

**The Question**: Should we rewrite from scratch or migrate incrementally?

---

## Code Audit

### Current Codebase Size

| Component | Files | LoC | `.await` Count |
|-----------|-------|-----|----------------|
| **TUI** | 28 files | ~6,800 LoC | 49 instances |
| **Conductor** | 55 files | ~39,000 LoC | 442 instances |
| **Total** | 83 files | ~46,000 LoC | 491 instances |

### TUI Analysis (`yollayah/core/surfaces/tui/`)

**Key Files**:
- `app.rs` (1,390 LoC) - Main event loop with blocking `.await` calls
- `conductor_client.rs` (400+ LoC) - Wrapper around Conductor communication
- `display.rs` (500+ LoC) - Display state management (mostly salvageable)
- `avatar/*` (~5,000 LoC) - Animation and rendering (NOT async-related, salvageable)
- `compositor/*` (~300 LoC) - Layer composition (NOT async-related, salvageable)
- `theme.rs`, `widgets/*`, `tasks/*` (~800 LoC) - Pure rendering logic (salvageable)

**Salvageable Code** (~70%):
- ✅ Avatar rendering system (5,000 LoC) - Pure display logic, no async issues
- ✅ Compositor layer system (300 LoC) - Pure rendering, no async
- ✅ Display state types (500 LoC) - Data structures, minimal async
- ✅ Theme system (400 LoC) - Pure constants and styling
- ✅ Widgets and task rendering (400 LoC) - Pure rendering

**Must Rewrite** (~30%):
- ❌ `app.rs` event loop (lines 260-375) - Blocking `.await` in main loop
- ❌ `conductor_client.rs` (lines 200-400) - Async message handling
- ❌ Terminal event handling - Currently uses async stream with manual `.await`

**Critical Violations in TUI**:
```rust
// app.rs:351 - BLOCKS event loop for 2-5 seconds during model loading
self.conductor.poll_streaming().await;

// app.rs:287-342 - Manual tokio::select! (should use reactive streams)
tokio::select! {
    maybe_event = event_stream.next() => { ... }
    _ = tokio::time::sleep(Duration::from_millis(100)) => { ... }
}

// conductor_client.rs:380 - Propagates blocking from Conductor
pub async fn poll_streaming(&mut self) -> bool {
    self.conductor.poll_streaming().await  // Blocks!
}
```

### Conductor Analysis (`yollayah/conductor/core/`)

**Key Files**:
- `conductor.rs` (2,500+ LoC) - Core orchestration with 130 `.await` calls
- `backend/ollama.rs` (1,000+ LoC) - LLM backend with streaming
- `routing/*` (5,000+ LoC) - Multi-model routing with async coordination
- `transport/*` (3,000+ LoC) - IPC/networking (inherently async, some salvageable)
- `session.rs`, `avatar.rs`, `security.rs` (~5,000 LoC) - Business logic (mostly salvageable)

**Salvageable Code** (~60%):
- ✅ Business logic (session, avatar state, security) - Pure logic, no async patterns
- ✅ Data structures (messages, events, types) - No async
- ✅ Transport traits (interfaces) - Can adapt to reactive
- ✅ Routing policy logic - Decision-making code is pure

**Must Rewrite** (~40%):
- ❌ `conductor.rs::poll_streaming()` (lines 1022-1130) - Blocking channel recv
- ❌ `conductor.rs::send_message()` - Manual async orchestration
- ❌ `backend/ollama.rs` streaming task - Manual async spawn/channel handling
- ❌ `routing/router.rs` - Manual async coordination between models
- ❌ All manual `tokio::spawn`, `tokio::select!`, async function chains

**Critical Violations in Conductor**:
```rust
// conductor.rs:1034 - ROOT CAUSE of BUG-001 (now fixed, but illustrates pattern)
match rx.recv().await {  // Was blocking, now try_recv()
    Some(token) => { ... }
}

// conductor.rs:900+ - Manual message sending with .await chains
pub async fn send_message(&mut self, content: String) -> Result<()> {
    self.validate_input(&content).await?;
    let response = self.backend.chat(request).await?;  // Manual async
    self.streaming_rx = Some(rx);
    self.set_state(ConductorState::Responding).await;
}

// backend/ollama.rs:181-243 - Manual tokio::spawn with channel
tokio::spawn(async move {
    while let Some(chunk) = stream.next().await {  // Manual iteration
        if tx.send(StreamingToken::Token(token)).await.is_err() {
            return;  // Manual error handling
        }
    }
});
```

### Patterns That Must Be Completely Rewritten

**None of these can be incrementally migrated - they require architectural redesign**:

1. **Event Loop Pattern** (app.rs:260-375)
   - Current: Manual `tokio::select!` with blocking calls
   - Required: Reactive stream pipeline with `select_all()`, `for_each()`

2. **Channel Polling Pattern** (conductor.rs:1022-1130)
   - Current: `rx.recv().await` or `rx.try_recv()` in loop
   - Required: `ReceiverStream::new(rx).map(Event::Token)` in pipeline

3. **Async Function Chains** (Throughout both codebases)
   - Current: `async fn` calling other `async fn` with `.await`
   - Required: Stream transformations with `map`, `filter`, `scan`

4. **Manual Spawning** (backend/ollama.rs:181)
   - Current: `tokio::spawn(async move { ... })`
   - Required: Framework-managed stream execution

5. **State Mutation During Async** (conductor.rs:900-1016)
   - Current: `self.state = X; self.backend.chat().await; self.state = Y;`
   - Required: Pure transformations with `scan()` for state accumulation

---

## Options Considered

### Option 1: Fresh Start (Complete Rewrite)

**Approach**:
1. Create new `app_reactive.rs` and `conductor_reactive.rs` from scratch
2. Port salvageable code (display logic, business logic) as needed
3. Build correct reactive architecture from day 1
4. Swap in new implementation when complete
5. Delete old code entirely

**Pros**:
- ✅ **Clean slate** - No legacy constraints, no fighting old patterns
- ✅ **Faster development** - Don't need to coordinate old/new code
- ✅ **Correct by default** - Reactive patterns enforced from start
- ✅ **Easier to enforce "no .await" rule** - No existing violations to work around
- ✅ **Optimal state management** - Design reactive state flow from scratch
- ✅ **Better testing** - Build tests for reactive behavior from start
- ✅ **No coordination overhead** - One coherent codebase, not mixed paradigms

**Cons**:
- ❌ **Higher upfront risk** - All-or-nothing approach
- ❌ **More initial code** - Need to write ~30-40% of codebase from scratch
- ❌ **Feature replication** - Need to ensure all features ported
- ❌ **Harder to validate incrementally** - Can't test in production until complete
- ❌ **Context switching** - Porting logic while learning reactive patterns

**Effort Estimate**:
- **TUI**: 2-3 weeks (rewrite event loop, port display logic)
- **Conductor**: 3-4 weeks (rewrite orchestration, port business logic)
- **Total**: 5-7 weeks (can parallelize TUI + Conductor work)

**Risk Level**: MEDIUM
- Mitigated by: Prototype in Sprint 0, salvaging 60-70% of code, clear target architecture

### Option 2: Incremental Migration

**Approach**:
1. Introduce reactive abstractions alongside existing code
2. Migrate file-by-file, converting async functions to streams
3. Maintain compatibility layer between old/new code
4. Gradually replace old code as new code proven
5. Delete old code once all features migrated

**Pros**:
- ✅ **Lower risk** - Validate each step before proceeding
- ✅ **Keep features working** - No big bang replacement
- ✅ **Can mix old/new** - Gradual transition during development
- ✅ **Easier rollback** - Can revert individual changes
- ✅ **Learn while migrating** - Reactive patterns learned incrementally

**Cons**:
- ❌ **Complex coordination** - Old and new code must coexist
- ❌ **Legacy interference** - Hard to enforce reactive principles during transition
- ❌ **Slower overall** - Coordination overhead slows progress
- ❌ **Risk of stopping midway** - "Good enough" syndrome before completion
- ❌ **Two codebases** - Maintain both paradigms simultaneously
- ❌ **Hard to test** - Mixed async/reactive patterns are hard to reason about
- ❌ **Technical debt accumulates** - Adaptation layer complexity grows

**Effort Estimate**:
- **TUI**: 3-4 weeks (incremental conversion + coordination)
- **Conductor**: 5-7 weeks (incremental conversion + coordination)
- **Total**: 8-11 weeks (coordination overhead prevents full parallelization)

**Risk Level**: HIGH
- Coordination complexity
- Risk of incomplete migration
- Hard to enforce principles during transition
- Two mental models active simultaneously

---

## Decision

**RECOMMENDATION: Fresh Start (Option 1)**

### Rationale

#### 1. Salvageable Code is Substantial (60-70%)

The majority of the codebase is **pure logic with no async patterns**:
- Avatar rendering system (5,000 LoC)
- Display state management (500 LoC)
- Compositor and widgets (700 LoC)
- Business logic (session, security, routing policy) (5,000 LoC)

**Fresh start doesn't mean rewriting everything** - it means rewriting the 30-40% that's fundamentally broken and porting the 60-70% that's clean.

#### 2. Async Patterns Cannot Be Incrementally Fixed

The violations are **architectural**, not tactical:

```rust
// ❌ This cannot be "fixed" - it must be REPLACED
while self.running {
    tokio::select! { ... }
    self.conductor.poll_streaming().await;  // Blocks!
    self.render()?;
}

// ✅ With this
create_event_pipeline()
    .for_each(|event| {
        app.handle_event(event);
        app.render();
        futures::future::ready(())
    })
    .run();
```

**You can't have both in the same codebase.** They're mutually exclusive paradigms.

#### 3. Incremental Migration Has Fatal Coordination Overhead

**The complexity isn't in the code - it's in the coordination**:

- How do you mix `tokio::select!` with reactive streams?
- How do you maintain a compatibility layer between blocking `.await` and `ReceiverStream`?
- How do you test code that's half-reactive, half-manual-async?
- How do you prevent developers from using the "old way" during transition?

**This overhead costs more time than writing clean code from scratch.**

#### 4. We Already Have a Working Bug Fix (Proof of Concept)

**BUG-001 was fixed in 5 minutes** by changing ONE line:
```rust
// FROM: match rx.recv().await {
// TO:   match rx.try_recv() {
```

This proves two things:
1. The fixes are **simple once you know the pattern**
2. But **finding all violations is hard** (491 `.await` calls to audit!)

**Fresh start means we find ZERO violations** because we never write them.

#### 5. Team is Small and Focused

We're not coordinating across 50 engineers. We have:
- **Architect + Rust/Async Specialist** (Team A) - TUI
- **Rust/Async Specialist + Hacker** (Team B) - Conductor

**Two small teams can build clean code faster than coordinating a complex migration.**

#### 6. Prototype Proves Viability (Sprint 0)

Sprint 0 delivers:
- Reactive prototype demonstrating the pattern
- Migration guide showing before/after
- Performance validation

**By the time we decide to commit, we've already proven it works.**

---

## Consequences

### Positive

1. **Clean Architecture** - Reactive patterns enforced from day 1
2. **Faster Development** - No coordination overhead, just build
3. **Better Testing** - Reactive code is easier to test (pure transformations)
4. **No Technical Debt** - No adaptation layer, no "legacy mode"
5. **Team Learning** - Fresh start forces deep understanding of reactive patterns
6. **Performance Guarantees** - Hit efficiency targets immediately (no gradual improvement)

### Negative

1. **Higher Initial Risk** - All-or-nothing approach
2. **Feature Replication** - Must ensure all existing features ported
3. **Requires Discipline** - No shortcuts to "just make it work"
4. **Context Switching** - Porting logic while learning reactive patterns

### Mitigations

1. **Sprint 0 Prototype** - Prove viability before committing
2. **Salvage Plan** - Clear list of what to port (60-70% of code)
3. **Parallel Development** - TUI and Conductor can be built simultaneously
4. **Integration Tests** - Re-enable tests to validate feature parity
5. **Migration Guide** - Document common patterns for team reference

---

## Implementation Plan

### Phase 1: Foundation (Sprint 0 - Week 1)
- [x] Create feature branch `feature/reactive-tui-overhaul`
- [ ] Build reactive prototype (STORY-003)
- [ ] Document salvageable vs must-rewrite code
- [ ] Create migration guide for team

### Phase 2: Infrastructure (Sprint 1 - Weeks 2-3)
- [ ] Build reactive stream wrappers (ReceiverStream, IntervalStream, TerminalEventStream)
- [ ] Create stream combinator utilities
- [ ] Build testing infrastructure for reactive code

### Phase 3: Parallel Development (Sprints 2-3 - Weeks 4-7)

**Team A (TUI)**:
- [ ] Rewrite `app_reactive.rs` with reactive event loop
- [ ] Port display logic from `display.rs`
- [ ] Port avatar system (no changes needed, just import)
- [ ] Port compositor (no changes needed, just import)

**Team B (Conductor)**:
- [ ] Rewrite `conductor_reactive.rs` with observable patterns
- [ ] Port business logic (session, avatar state, security)
- [ ] Rewrite backend streaming as reactive pipeline
- [ ] Rewrite routing as observable composition

### Phase 4: Integration (Sprint 4 - Weeks 8-9)
- [ ] Connect reactive TUI to reactive Conductor
- [ ] Port error handling to reactive streams
- [ ] Performance validation and optimization
- [ ] Re-enable integration tests

### Phase 5: Production (Sprints 5-6 - Weeks 10-12)
- [ ] Advanced reactive operators (retry, debounce, throttle)
- [ ] Stress testing and chaos engineering
- [ ] Final performance validation
- [ ] Merge to main (breaking release: ai-way 0.2.0)

---

## Success Criteria

### Code Quality
- [ ] Zero `.await` calls in production code (enforced by grep in CI)
- [ ] Zero memory leaks (validated by valgrind/heaptrack)
- [ ] 100% test coverage for reactive pipelines
- [ ] All integration tests passing

### Performance Targets (PRINCIPLE-efficiency.md)
- [ ] TUI idle CPU: < 0.1%
- [ ] TUI active CPU: < 5% during streaming
- [ ] TUI frame rate: 10 FPS consistent
- [ ] Conductor idle CPU: < 1%
- [ ] Memory: < 50 MB TUI baseline, < 200 MB Conductor baseline
- [ ] Message passing latency: < 1ms

### User Experience
- [ ] No UI freezing (ever)
- [ ] Smooth streaming (tokens appear as generated)
- [ ] Responsive to user input (< 100ms)
- [ ] Graceful error handling
- [ ] Clean shutdown

---

## Alternatives Considered

### Hybrid Approach (Partial Fresh Start)
- Fresh start for TUI (~6,800 LoC)
- Incremental migration for Conductor (~39,000 LoC)

**Rejected because**:
- Conductor is where most violations are (442 `.await` calls vs 49)
- Coordination overhead still exists
- Doesn't solve the "two paradigms" problem

### Gradual Feature Freeze
- Freeze new features during migration
- Focus entirely on reactive conversion

**Rejected because**:
- Doesn't reduce coordination complexity
- Just slows down overall progress
- Fresh start is already focused (no new features)

---

## References

**Documentation**:
- `EPIC-001-TUI-reactive-overhaul.md` - Full overhaul plan
- `SPRINT-00-foundation.md` - This sprint's goals
- `knowledge/principles/PRINCIPLE-efficiency.md` - Reactive principles
- `progress/TODO-BUG-001-tui-waits-for-full-stream.md` - Bug that triggered this

**Codebase Files**:
- `/var/home/machiyotl/src/ai-way/yollayah/core/surfaces/tui/src/app.rs`
- `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/conductor.rs`
- `/var/home/machiyotl/src/ai-way/yollayah/conductor/core/src/backend/ollama.rs`

**External Resources**:
- tokio-stream documentation: https://docs.rs/tokio-stream
- RxRust documentation: https://github.com/ReactiveX/RxRust
- Reactive Streams specification: http://www.reactive-streams.org/

---

## Approval

**Status**: PROPOSED - Awaiting team review

**Sign-off Required**:
- [ ] Architect - Architecture evaluation complete
- [ ] Rust/Async Specialist - Technical feasibility validated
- [ ] Hacker - Risk assessment complete
- [ ] Crazy Intern - Fresh perspective provided

**Next Steps**:
1. Team review and discussion
2. Finalize decision
3. Update EPIC-001 with chosen approach
4. Begin Sprint 1 (reactive infrastructure)

---

**Last Updated**: 2026-01-03
