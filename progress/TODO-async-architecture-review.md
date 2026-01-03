# TODO-async-architecture-review: Async/Non-Blocking Architecture Audit

**Created**: 2026-01-02
**Status**: IN PROGRESS
**Priority**: HIGH (Core Architecture)
**Owner**: Architect + Hacker + Crazy Intern

---

## Mission Statement

**HARD REQUIREMENTS**:
1. **Conductor**: Must be ENTIRELY async and non-blocking
2. **TUI**: Must be ENTIRELY async and non-blocking
3. **All Surfaces**: Must be async and non-blocking
4. **Performance**: Prime requirement for responsiveness

**Philosophy**: Heavy parallelism, all moving parts fully non-blocking.

---

## Scope

### ✅ Exempt from Async (Synchronous OK)

- **yollayah.sh**: Bootstrap wrapper script
  - Simple sequential initialization
  - Runs once, exits
  - Hands off to async runtime (TUI/Conductor)

### ❗ MUST BE ASYNC (Hard Requirements)

- **Conductor** (`conductor/`)
  - All operations async/await
  - No blocking I/O
  - Concurrent model handling
  - Parallel request processing

- **TUI** (`tui/`)
  - Event loop fully async
  - UI updates non-blocking
  - Network calls async
  - No thread blocking

- **All Surfaces** (future)
  - Web interface
  - API endpoints
  - Any client surface

---

## Review Checklist

### Conductor Review

- [ ] **A1**: Review conductor-core async patterns
  - [ ] Check tokio runtime usage
  - [ ] Verify no blocking calls in async context
  - [ ] Check for `.await` on all I/O
  - [ ] Review model loading (should be async)
  - [ ] Review Ollama API calls (should be async)

- [ ] **A2**: Review conductor-daemon async patterns
  - [ ] Check socket handling (async I/O)
  - [ ] Verify event loop non-blocking
  - [ ] Check message handling async
  - [ ] Review error handling (no blocking)

- [ ] **A3**: Identify blocking code in Conductor
  - [ ] File I/O operations
  - [ ] Network requests
  - [ ] Database operations (if any)
  - [ ] CPU-intensive tasks (should use spawn_blocking)

- [ ] **A4**: Performance bottlenecks
  - [ ] Sequential operations that could be parallel
  - [ ] Missing concurrency opportunities
  - [ ] Synchronization primitives (locks, mutexes)

### TUI Review

- [ ] **T1**: Review TUI async patterns
  - [ ] Event loop structure
  - [ ] Crossterm event handling (async)
  - [ ] Conductor client calls (async)
  - [ ] UI rendering pipeline

- [ ] **T2**: Identify blocking code in TUI
  - [ ] Terminal I/O operations
  - [ ] Event processing
  - [ ] Avatar rendering
  - [ ] Message sending/receiving

- [ ] **T3**: Check responsiveness
  - [ ] UI updates during long operations
  - [ ] Keyboard input responsiveness
  - [ ] Loading states for async operations

### Documentation Review

- [ ] **D1**: Update CLAUDE.md with async philosophy
- [ ] **D2**: Update architecture docs with async requirements
- [ ] **D3**: Add async patterns guide for contributors
- [ ] **D4**: Document blocking vs non-blocking decisions

### Testing Infrastructure

- [ ] **I1**: Integration testing framework (Epic)
- [ ] **I2**: Test hooks in build lifecycle
- [ ] **I3**: Async test patterns
- [ ] **I4**: Performance benchmarks for async operations

---

## Async Patterns - Best Practices

### ✅ DO

```rust
// Async I/O
async fn load_model(name: &str) -> Result<Model> {
    let response = ollama_client.pull(name).await?;
    Ok(response)
}

// Concurrent operations
let (model_a, model_b) = tokio::join!(
    load_model("llama3.2:3b"),
    load_model("qwen2:0.5b")
);

// Background tasks
tokio::spawn(async move {
    // Long-running background work
});

// CPU-intensive work (offload to thread pool)
tokio::task::spawn_blocking(|| {
    // Heavy computation
});
```

### ❌ DON'T

```rust
// Blocking I/O in async context
async fn bad_example() {
    std::fs::read_to_string("file.txt"); // ❌ BLOCKING!
}

// Synchronous sleep in async
async fn bad_sleep() {
    std::thread::sleep(Duration::from_secs(1)); // ❌ BLOCKS RUNTIME!
}

// Sequential when could be concurrent
async fn bad_sequential() {
    let a = fetch_a().await; // ❌ Could run in parallel
    let b = fetch_b().await;
}
```

---

## Findings (Initial Review)

### Conductor

**Status**: ⏳ PENDING REVIEW

**Files to Review**:
- `conductor/core/src/lib.rs`
- `conductor/core/src/conductor.rs`
- `conductor/daemon/src/main.rs`
- `conductor/protocol/src/lib.rs`

**Known Issues**: TBD

### TUI

**Status**: ⏳ PENDING REVIEW

**Files to Review**:
- `tui/src/app.rs`
- `tui/src/conductor_client.rs`
- `tui/src/events/mod.rs`
- `tui/src/main.rs`

**Known Good**:
- Uses `tokio::select!` for event handling (app.rs:260)
- Async event stream (crossterm EventStream)
- Warmup disabled for responsiveness (conductor_client.rs:112)

**Known Issues**: TBD

---

## Action Items

### Immediate (Sprint Current)

1. **Review Conductor Core**
   - File: `TODO-conductor-async-audit.md`
   - Owner: Architect + Hacker
   - Goal: Identify all blocking code

2. **Review TUI**
   - File: `TODO-tui-async-audit.md`
   - Owner: Architect + Hacker
   - Goal: Verify full async implementation

3. **Document Async Philosophy**
   - Update CLAUDE.md
   - Create async patterns guide
   - Update architecture docs

### Short-term (Next Sprint)

4. **Fix Blocking Code**
   - Create tasks for each blocking operation found
   - Prioritize by impact
   - Implement async versions

5. **Performance Testing**
   - Add async benchmarks
   - Test under load
   - Profile for bottlenecks

### Long-term (Epic)

6. **Integration Testing Framework**
   - See `TODO-epic-integration-testing.md`
   - Use `--test` mode for fast tests
   - Add pre-merge validation

---

## Integration Testing Strategy

### Test Modes

**Fast Tests** (< 1 minute):
- Use `./yollayah.sh --test` (tiny model)
- Unit tests
- Basic integration tests
- Run on every commit

**Full Tests** (< 10 minutes):
- Use `./yollayah.sh` (full model)
- Comprehensive integration tests
- End-to-end scenarios
- Run before merge to main

**Heavy Tests** (< 30 minutes):
- Load testing
- Performance benchmarks
- Multi-model scenarios
- Run nightly / on release

### Build Lifecycle Hooks

```bash
# Pre-commit
.git/hooks/pre-commit
└─> cargo test --workspace
└─> ./yollayah.sh --test (basic smoke test)

# Pre-push
.git/hooks/pre-push
└─> cargo test --workspace
└─> ./yollayah.sh --test (full integration)
└─> Async audit checks

# Pre-merge (CI/CD)
.github/workflows/pr.yml
└─> Full test suite
└─> Performance benchmarks
└─> Integration tests with normal mode
```

---

## Success Criteria

### Conductor

- ✅ All I/O operations use `.await`
- ✅ No `std::fs` or blocking calls in async context
- ✅ CPU work uses `spawn_blocking`
- ✅ Concurrent model handling demonstrated
- ✅ Performance benchmarks meet targets

### TUI

- ✅ Event loop never blocks
- ✅ UI updates during long operations
- ✅ Keyboard input always responsive (< 16ms)
- ✅ All network calls async
- ✅ Avatar rendering non-blocking

### Documentation

- ✅ Async philosophy documented
- ✅ Patterns guide for contributors
- ✅ Architecture diagrams show async flow
- ✅ Examples of correct async usage

### Testing

- ✅ Fast tests (< 1 min) in pre-commit
- ✅ Full tests before merge
- ✅ Heavy tests in CI/CD
- ✅ Performance regression detection

---

## Related Documents

- `TODO-architecture-terminal-ownership.md` - Terminal ownership (sync bootstrap)
- `TODO-epic-integration-testing.md` - Full testing framework (to be created)
- `TODO-conductor-async-audit.md` - Detailed conductor review (to be created)
- `TODO-tui-async-audit.md` - Detailed TUI review (to be created)

---

## Team Assignments

**Architect**:
- Overall async strategy
- Performance requirements
- Architecture documentation

**Hacker**:
- Code review (find blocking calls)
- Implement async versions
- Performance optimization

**Crazy Intern**:
- Test all the things!
- Break async assumptions
- Find edge cases

---

**Owner**: Architect + Hacker + Crazy Intern
**Last Updated**: 2026-01-02
**Status**: IN PROGRESS (Initial setup complete, reviews pending)
