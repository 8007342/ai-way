# TODO-103: Build Reactive TUI Prototype

**EPIC**: EPIC-001-TUI-reactive-overhaul.md
**Sprint**: SPRINT-00-foundation.md
**Story**: STORY-003
**Owner**: Rust/Async Specialist
**Status**: 🟡 READY TO START
**Effort**: 1 day
**Priority**: P0 - CRITICAL (validates approach)
**Depends On**: TODO-102 (dependencies added)

---

## Objective

Build working prototype of reactive TUI event pipeline to validate the approach before committing to full rewrite.

---

## Prototype Requirements

**Must demonstrate**:
- ✅ Zero `.await` calls in event processing
- ✅ Multiple stream sources merged correctly
- ✅ 10 FPS tick rate maintained
- ✅ Handles high event throughput (1000/sec)
- ✅ Clean shutdown
- ✅ Backpressure handling
- ✅ Performance meets targets

**Stream sources**:
1. Terminal events (keyboard, mouse)
2. Interval ticks (10 FPS animation)
3. Message stream (simulated conductor messages)
4. Shutdown signal

---

## Implementation

Create `yollayah/core/surfaces/tui/examples/reactive_prototype.rs`:

```rust
//! Full reactive TUI prototype
//!
//! This prototype demonstrates:
//! - Real terminal event handling (crossterm)
//! - Multiple merged event streams
//! - 10 FPS rendering loop
//! - Message processing (simulated conductor)
//! - Clean shutdown
//! - Performance monitoring
//!
//! NO .await calls in event processing!

use crossterm::{
    event::{Event as CrosstermEvent, EventStream, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use futures::stream::{self, StreamExt};
use tokio_stream::wrappers::IntervalStream;
use std::time::{Duration, Instant};
use std::io::Write;

#[derive(Debug)]
enum AppEvent {
    Terminal(CrosstermEvent),
    Tick,
    Message(String),
    Shutdown,
}

struct AppState {
    running: bool,
    frame_count: u64,
    message_count: u64,
    event_count: u64,
    start_time: Instant,
}

impl AppState {
    fn new() -> Self {
        Self {
            running: true,
            frame_count: 0,
            message_count: 0,
            event_count: 0,
            start_time: Instant::now(),
        }
    }

    fn handle_event(&mut self, event: AppEvent) {
        self.event_count += 1;

        match event {
            AppEvent::Terminal(CrosstermEvent::Key(key)) => {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    println!("\n\nShutdown requested by user");
                    self.running = false;
                }
            }
            AppEvent::Tick => {
                self.frame_count += 1;
            }
            AppEvent::Message(msg) => {
                self.message_count += 1;
                println!("Message {}: {}", self.message_count, msg);
            }
            AppEvent::Shutdown => {
                println!("\n\nShutdown signal received");
                self.running = false;
            }
            _ => {}
        }
    }

    fn render(&self) {
        if self.frame_count % 10 == 0 {  // Print stats every second
            let elapsed = self.start_time.elapsed();
            let fps = self.frame_count as f64 / elapsed.as_secs_f64();
            let events_per_sec = self.event_count as f64 / elapsed.as_secs_f64();

            print!("\r[Frame: {} | FPS: {:.1} | Events: {} | Evt/sec: {:.0} | Messages: {}]",
                self.frame_count,
                fps,
                self.event_count,
                events_per_sec,
                self.message_count
            );
            std::io::stdout().flush().unwrap();
        }
    }
}

fn create_terminal_stream() -> impl Stream<Item = AppEvent> {
    EventStream::new()
        .map(|result| {
            result
                .map(AppEvent::Terminal)
                .unwrap_or_else(|_| AppEvent::Message("Terminal error".to_string()))
        })
}

fn create_tick_stream() -> impl Stream<Item = AppEvent> {
    IntervalStream::new(tokio::time::interval(Duration::from_millis(100)))
        .map(|_| AppEvent::Tick)
}

fn create_message_stream() -> impl Stream<Item = AppEvent> {
    // Simulate high-throughput message stream (100 msgs/sec)
    IntervalStream::new(tokio::time::interval(Duration::from_millis(10)))
        .take(1000)  // 1000 messages total (10 seconds)
        .enumerate()
        .map(|(i, _)| AppEvent::Message(format!("Token_{}", i)))
}

fn create_shutdown_stream() -> impl Stream<Item = AppEvent> {
    // Shutdown after 10 seconds
    stream::once(async {
        tokio::time::sleep(Duration::from_secs(10)).await;
        AppEvent::Shutdown
    })
}

fn create_event_pipeline() -> impl Stream<Item = AppEvent> {
    use futures::stream::select_all;

    select_all(vec![
        Box::pin(create_terminal_stream()) as Pin<Box<dyn Stream<Item = AppEvent>>>,
        Box::pin(create_tick_stream()),
        Box::pin(create_message_stream()),
        Box::pin(create_shutdown_stream()),
    ])
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    enable_raw_mode()?;
    println!("Reactive TUI Prototype");
    println!("======================");
    println!("Press 'q' or ESC to quit, or wait 10 seconds for auto-shutdown");
    println!();

    // Create app state
    let mut app = AppState::new();

    // Create reactive event pipeline
    let mut event_stream = create_event_pipeline();

    // Main event loop - NO .await in event processing!
    while app.running {
        if let Some(event) = event_stream.next().await {
            app.handle_event(event);
            app.render();
        } else {
            break;  // Stream exhausted
        }
    }

    // Cleanup
    disable_raw_mode()?;

    // Print final stats
    let elapsed = app.start_time.elapsed();
    println!("\n\nPrototype Statistics:");
    println!("====================");
    println!("Total runtime: {:.2}s", elapsed.as_secs_f64());
    println!("Total frames: {}", app.frame_count);
    println!("Average FPS: {:.1}", app.frame_count as f64 / elapsed.as_secs_f64());
    println!("Total events: {}", app.event_count);
    println!("Events/sec: {:.0}", app.event_count as f64 / elapsed.as_secs_f64());
    println!("Messages processed: {}", app.message_count);

    Ok(())
}
```

---

## Performance Targets

**Must meet these targets** (from PRINCIPLE-efficiency.md):

| Metric | Target | Notes |
|--------|--------|-------|
| Frame rate | 10 FPS | Consistent, no drops |
| Idle CPU | < 0.1% | When no events |
| Active CPU | < 5% | During message processing |
| Memory | < 50 MB | Baseline |
| Event throughput | 1000/sec | No backpressure issues |
| Latency | < 100ms | User input to render |

---

## Testing Procedure

```bash
# Build prototype
cd yollayah/core/surfaces/tui
cargo build --release --example reactive_prototype

# Run prototype
cargo run --release --example reactive_prototype

# Expected behavior:
# 1. Displays stats in real-time (frame count, FPS, events/sec)
# 2. Processes ~100 messages/sec smoothly
# 3. Maintains 10 FPS rendering
# 4. Responds to 'q' or ESC immediately (< 100ms)
# 5. Auto-shuts down after 10 seconds
# 6. Prints final statistics

# Expected output (approximately):
# [Frame: 100 | FPS: 10.0 | Events: 1100 | Evt/sec: 110 | Messages: 1000]
#
# Prototype Statistics:
# ====================
# Total runtime: 10.00s
# Total frames: 100
# Average FPS: 10.0
# Total events: 1100
# Events/sec: 110
# Messages processed: 1000
```

---

## Performance Validation

Run with performance monitoring:

```bash
# CPU profiling
cargo flamegraph --example reactive_prototype

# Memory profiling
heaptrack cargo run --release --example reactive_prototype

# Check for leaks
valgrind --leak-check=full \
    ./target/release/examples/reactive_prototype
```

**Validation checklist**:
- [ ] FPS stays at 10.0 (±0.1)
- [ ] No frame drops during high message throughput
- [ ] CPU usage < 5% during processing
- [ ] Memory usage stays flat (no leaks)
- [ ] No allocations in hot loop (validated by flamegraph)
- [ ] Clean shutdown with no leaks (validated by valgrind)

---

## Success Criteria

- [ ] Prototype compiles and runs
- [ ] All performance targets met
- [ ] Zero `.await` in event processing code
- [ ] Handles 1000 events/sec without backpressure issues
- [ ] Clean shutdown
- [ ] No memory leaks
- [ ] Team reviews code and approves pattern

---

## Lessons Learned

Document in this TODO after running prototype:

### What Worked Well
- (to be filled in after testing)

### What Was Challenging
- (to be filled in after testing)

### What Needs Adjustment
- (to be filled in after testing)

### Pattern Recommendations
- (to be filled in after testing)

---

## Next

TODO-104-document-migration-patterns.md

**Status**: 🟡 READY TO START
