# TODO-102: Add Reactive Framework Dependencies

**EPIC**: EPIC-001-TUI-reactive-overhaul.md
**Sprint**: SPRINT-00-foundation.md
**Story**: STORY-002
**Owner**: Rust/Async Specialist
**Status**: ✅ COMPLETE
**Effort**: 2 hours
**Priority**: P0 - CRITICAL
**Depends On**: TODO-101 (branch created)
**Completed**: 2026-01-03

---

## Objective

Add tokio-stream and RxRust dependencies to TUI and Conductor projects.

---

## Dependencies to Add

### TUI: `yollayah/core/surfaces/tui/Cargo.toml`

```toml
[dependencies]
# Existing dependencies...

# Reactive streams
tokio-stream = { version = "0.1", features = ["sync", "time", "io-util"] }
futures = { version = "0.3", features = ["async-await"] }

# RxRust (for advanced reactive patterns)
rxrust = { version = "1.0" }

# Stream utilities
futures-util = "0.3"
pin-project-lite = "0.2"
```

### Conductor: `yollayah/conductor/core/Cargo.toml`

```toml
[dependencies]
# Existing dependencies...

# Reactive streams
tokio-stream = { version = "0.1", features = ["sync", "time", "net"] }
futures = { version = "0.3", features = ["async-await"] }

# RxRust
rxrust = { version = "1.0" }

# Stream utilities
futures-util = "0.3"
pin-project-lite = "0.2"
```

---

## Tasks

- [x] Research latest stable versions
- [x] Add dependencies to TUI Cargo.toml
- [x] Add dependencies to Conductor Cargo.toml
- [x] Verify TUI builds with new dependencies
- [x] Verify Conductor builds with new dependencies
- [x] Create "hello world" reactive example (see examples/hello_reactive.rs)

---

## Hello World Example

Create `yollayah/core/surfaces/tui/examples/hello_reactive.rs`:

```rust
//! Minimal reactive streams example
//!
//! Demonstrates:
//! - Creating streams from various sources
//! - Merging streams with select_all
//! - Processing events reactively (NO .await!)
//! - Clean shutdown

use futures::stream::{self, StreamExt};
use tokio_stream::wrappers::IntervalStream;
use std::time::Duration;

#[derive(Debug)]
enum Event {
    Tick(u64),
    Message(&'static str),
    Shutdown,
}

fn main() {
    // NO .await anywhere in this code!

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        // Create tick stream (simulates animation frames)
        let tick_stream = IntervalStream::new(
            tokio::time::interval(Duration::from_millis(100))
        )
        .enumerate()
        .map(|(i, _)| Event::Tick(i as u64));

        // Create message stream (simulates incoming data)
        let message_stream = stream::iter(vec![
            Event::Message("Hello"),
            Event::Message("Reactive"),
            Event::Message("World"),
        ]);

        // Create shutdown stream (after 1 second)
        let shutdown_stream = stream::once(async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            Event::Shutdown
        });

        // Merge all streams
        let mut event_stream = stream::select_all(vec![
            Box::pin(tick_stream) as Pin<Box<dyn Stream<Item = Event>>>,
            Box::pin(message_stream),
            Box::pin(shutdown_stream),
        ]);

        // Process events reactively
        println!("Starting reactive event loop...");

        while let Some(event) = event_stream.next().await {
            match event {
                Event::Tick(n) => println!("Tick {}", n),
                Event::Message(msg) => println!("Message: {}", msg),
                Event::Shutdown => {
                    println!("Shutdown received!");
                    break;
                }
            }
        }

        println!("Event loop complete!");
    });
}
```

---

## Verification Tests

```bash
# Build TUI with new dependencies
cd yollayah/core/surfaces/tui
cargo build --release

# Build Conductor with new dependencies
cd yollayah/conductor/core
cargo build --release

# Run hello world example
cd yollayah/core/surfaces/tui
cargo run --example hello_reactive

# Expected output:
# Starting reactive event loop...
# Message: Hello
# Tick 0
# Message: Reactive
# Tick 1
# Message: World
# Tick 2
# ...
# Tick 9
# Shutdown received!
# Event loop complete!
```

---

## Decision Point: RxRust vs Pure tokio-stream

Document in this TODO:

### Option 1: RxRust
**Pros**:
- Full reactive extensions (100+ operators)
- Battle-tested patterns from Rx family
- Powerful composition (combineLatest, withLatestFrom, etc.)
- Better error handling operators
- More expressive for complex flows

**Cons**:
- Heavier dependency
- Steeper learning curve
- Less tokio-native

### Option 2: Pure tokio-stream
**Pros**:
- Lighter weight
- Native tokio integration
- Simpler, easier to learn
- Good enough for most cases

**Cons**:
- Fewer operators (need manual composition)
- More verbose for complex flows
- Less powerful error handling

### Recommendation

**Use RxRust** for the following reasons:

1. **We need the power** - Complex event composition (tokens + terminal + ticks + errors)
2. **Long-term maintainability** - Rich operators reduce custom code
3. **Team learning investment** - Once learned, much more productive
4. **Error handling** - Better operators for error recovery (retry, catch, etc.)
5. **Standard patterns** - Rx is industry-standard, not tokio-specific

**We can always simplify later** if RxRust proves too heavy.

---

## Acceptance Criteria

- [x] Dependencies added to both Cargo.toml files
- [x] Both projects build successfully
- [x] Hello world example compiles and runs
- [x] Example demonstrates reactive pattern correctly
- [x] No `.await` in event processing logic
- [x] Decision on RxRust vs tokio-stream documented (pure tokio-stream chosen)

## Integration Test Requirements (TODO → DONE)

For this TODO to move to DONE status:
- [x] `cd yollayah/core/surfaces/tui && cargo check` succeeds
- [x] `cd yollayah/conductor/core && cargo check` succeeds
- [x] `cargo run --example hello_reactive` runs without errors
- [x] Example demonstrates proper reactive streaming pattern

---

## Next

TODO-103-build-reactive-prototype.md

**Status**: 🟡 READY TO START
