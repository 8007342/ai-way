# Reactive Migration Guide

**Purpose**: Guide for migrating from manual async/await to reactive streams

**Audience**: Development team (Rust/Async Specialists, Architects)

**Status**: Active (Sprint 0 deliverable)

---

## 1. Introduction

### Why We're Migrating

The current codebase uses manual `.await` calls which cause:
- **Memory leaks**: Resources not properly released
- **Thread pool conflicts**: Manual threading fights framework's thread pool
- **UI freezing**: Blocking operations freeze the event loop
- **CPU/GPU coordination issues**: Manual parallelism breaks hardware scheduling

**Solution**: Reactive streams/observables pattern using tokio-stream and RxRust.

### What Reactive Programming Is

Reactive programming treats asynchronous operations as **streams of events** that you compose declaratively.

Instead of:
```rust
while running {
    let event = rx.recv().await;  // BLOCKS!
    handle(event);
}
```

You write:
```rust
ReceiverStream::new(rx)
    .for_each(|event| {
        handle(event);
        ready(())
    })
    .await
```

The framework manages execution - you just declare transformations.

### Core Principles

1. **No manual `.await`** - Let the framework handle execution
2. **Declarative composition** - Describe what, not how
3. **Framework-managed concurrency** - No manual spawning
4. **Backpressure by default** - Automatic flow control
5. **Resource safety** - RAII and drop-based cleanup

---

## 2. Forbidden Patterns

### `.await` and Why It's Wrong

**❌ FORBIDDEN**:
```rust
async fn handle_message(&mut self, msg: Message) {
    let response = self.process(msg).await;  // BLOCKS EVENT LOOP!
    self.send_response(response).await;      // BLOCKS AGAIN!
}
```

**Why it's wrong**:
- Blocks the event loop thread
- Prevents other events from being processed
- Causes UI freezing
- Manual resource management prone to leaks

**✅ CORRECT**:
```rust
fn create_message_pipeline(msg_stream: impl Stream<Item = Message>)
    -> impl Stream<Item = Response>
{
    msg_stream
        .map(|msg| process_sync(msg))
        .filter_map(|result| result.ok())
}
```

### `tokio::select!` and Alternatives

**❌ FORBIDDEN**:
```rust
loop {
    tokio::select! {
        event = event_rx.recv() => handle_event(event),
        tick = ticker.tick() => render(),
        msg = msg_rx.recv() => handle_message(msg),
    }
}
```

**Why it's wrong**:
- Manual polling with `.await`
- Complex error handling
- Hard to test
- Resource cleanup is manual

**✅ CORRECT**:
```rust
use futures::stream::select_all;

let event_stream = ReceiverStream::new(event_rx);
let tick_stream = IntervalStream::new(ticker);
let msg_stream = ReceiverStream::new(msg_rx);

select_all(vec![
    event_stream.map(Event::Terminal).boxed(),
    tick_stream.map(|_| Event::Tick).boxed(),
    msg_stream.map(Event::Message).boxed(),
])
```

### Manual Loops vs Declarative Streams

**❌ FORBIDDEN**:
```rust
while self.running {
    match rx.recv().await {
        Some(item) => process(item),
        None => break,
    }
}
```

**✅ CORRECT**:
```rust
ReceiverStream::new(rx)
    .take_while(|_| ready(self.running))
    .for_each(|item| {
        process(item);
        ready(())
    })
```

### `tokio::spawn` vs Framework-Managed Tasks

**❌ FORBIDDEN**:
```rust
tokio::spawn(async move {
    process_data(data).await;
});
```

**Why it's wrong**:
- Manual task management
- No automatic cleanup
- Difficult to coordinate shutdown
- Thread pool conflicts

**✅ CORRECT**:
```rust
// Framework manages concurrent processing
stream
    .buffer_unordered(10)  // Process up to 10 items concurrently
    .for_each(|result| ready(()))
```

### Blocking Operations

**❌ FORBIDDEN**:
```rust
std::thread::sleep(Duration::from_secs(1));  // BLOCKS THREAD!
std::fs::read_to_string("file.txt");         // BLOCKS!
```

**✅ CORRECT**:
```rust
tokio::time::sleep(Duration::from_secs(1)).await;  // Non-blocking
tokio::fs::read_to_string("file.txt").await;       // Non-blocking
```

---

## 3. Required Patterns

### Stream Creation

**From channel**:
```rust
use tokio_stream::wrappers::ReceiverStream;

let (tx, rx) = mpsc::channel(100);
let stream = ReceiverStream::new(rx);
```

**From interval**:
```rust
use tokio_stream::wrappers::IntervalStream;

let interval = tokio::time::interval(Duration::from_millis(100));
let stream = IntervalStream::new(interval);
```

**From terminal events**:
```rust
use crossterm::event::EventStream;

let stream = EventStream::new()
    .map(|result| result.unwrap_or_else(|_| /* error event */));
```

**Custom stream**:
```rust
use futures::stream;

let stream = stream::iter(vec![1, 2, 3]);
```

### Stream Combinators

**Map (transform)**:
```rust
stream.map(|x| x * 2)
```

**Filter (select)**:
```rust
stream.filter(|x| ready(*x > 10))
```

**Filter-map (transform + filter)**:
```rust
stream.filter_map(|x| {
    if x > 10 {
        Some(x * 2)
    } else {
        None
    }
})
```

**Merge (combine streams)**:
```rust
use futures::stream::select_all;

select_all(vec![stream1.boxed(), stream2.boxed(), stream3.boxed()])
```

**Scan (stateful transformation)**:
```rust
stream.scan(0, |state, item| {
    *state += item;
    ready(Some(*state))
})
```

### Error Handling in Streams

**Convert errors to values**:
```rust
stream
    .map(|result| result.ok())
    .filter_map(|opt| opt)
```

**Inspect errors (logging)**:
```rust
stream
    .map(|result| {
        result.map_err(|e| {
            eprintln!("Error: {}", e);
            e
        })
    })
```

**Error recovery**:
```rust
stream
    .filter_map(|result| match result {
        Ok(value) => Some(value),
        Err(e) => {
            log_error(&e);
            None  // Continue stream
        }
    })
```

### Backpressure Management

**Bounded buffer**:
```rust
stream.buffer_unordered(256)  // Process up to 256 items concurrently
```

**Throttle**:
```rust
stream.throttle(Duration::from_millis(100))  // Limit rate
```

**Batching**:
```rust
stream.chunks(10)  // Process in batches of 10
```

### Resource Cleanup

**Automatic with Drop**:
```rust
struct MyStream {
    _resource: Resource,  // Dropped when stream is dropped
}

impl Stream for MyStream {
    // ...
}
```

**Manual cleanup**:
```rust
stream
    .inspect(|_| {
        // On each item
    })
    .on_completion(|| {
        // On stream end
        cleanup();
    })
```

---

## 4. Common Conversions

### Pattern 1: Event Loop → Event Pipeline

**❌ FORBIDDEN** (manual loop with .await):
```rust
while self.running {
    tokio::select! {
        event = event_stream.next() => {
            handle_event(event);
        }
        _ = tick.tick() => {
            update();
            render();
        }
    }
}
```

**✅ REQUIRED** (reactive pipeline):
```rust
create_event_pipeline()
    .for_each(|event| {
        handle_event(event);
        futures::future::ready(())
    })
    .run();
```

### Pattern 2: Polling Function → Stream Wrapper

**❌ FORBIDDEN** (blocking await):
```rust
pub async fn poll_streaming(&mut self) -> bool {
    match rx.recv().await {  // BLOCKS!
        Some(token) => { process(token); true }
        None => false,
    }
}
```

**✅ REQUIRED** (stream wrapper):
```rust
pub fn create_token_stream(rx: Receiver<Token>) -> impl Stream<Item = Token> {
    ReceiverStream::new(rx)
}
```

### Pattern 3: Message Handler → Stream Transformation

**❌ FORBIDDEN** (async fn with await):
```rust
async fn handle_message(&mut self, msg: Message) {
    let response = self.process(msg).await;
    self.send_response(response).await;
}
```

**✅ REQUIRED** (stream transformation):
```rust
fn create_message_pipeline(msg_stream: impl Stream<Item = Message>)
    -> impl Stream<Item = Response>
{
    msg_stream
        .map(|msg| process_sync(msg))  // Synchronous processing
        .filter_map(|result| result.ok())  // Error handling
}
```

### Pattern 4: Error Handling → Error Stream

**❌ FORBIDDEN** (try/catch with await):
```rust
async fn fetch_data(&self) -> Result<Data, Error> {
    match self.client.get().await {
        Ok(data) => Ok(data),
        Err(e) => {
            log_error(&e);
            Err(e)
        }
    }
}
```

**✅ REQUIRED** (error stream):
```rust
fn create_data_stream() -> impl Stream<Item = Result<Data, Error>> {
    http_stream()
        .map(|response| parse_data(response))
        .inspect_err(|e| log_error(e))
}
```

### Pattern 5: Parallel Operations → Stream Splitting

**❌ FORBIDDEN** (tokio::spawn):
```rust
tokio::spawn(async move {
    process_data(data).await;
});
```

**✅ REQUIRED** (stream splitting):
```rust
let (tx, rx) = stream.split();
// Framework manages concurrent processing
```

---

## 5. Testing Reactive Code

### Testing Stream Output

```rust
#[tokio::test]
async fn test_token_stream() {
    let (tx, rx) = mpsc::channel(10);
    let stream = ReceiverStream::new(rx);

    // Send test data
    tx.send("token1").await.unwrap();
    tx.send("token2").await.unwrap();
    drop(tx);

    // Collect results
    let tokens: Vec<_> = stream.collect().await;

    assert_eq!(tokens, vec!["token1", "token2"]);
}
```

### Mocking Event Sources

```rust
fn create_mock_event_stream() -> impl Stream<Item = Event> {
    stream::iter(vec![
        Event::Input('a'),
        Event::Input('b'),
        Event::Quit,
    ])
}

#[tokio::test]
async fn test_event_handling() {
    let mut app = App::new();
    let events = create_mock_event_stream();

    events
        .for_each(|event| {
            app.handle_event(event);
            futures::future::ready(())
        })
        .await;

    assert_eq!(app.input, "ab");
    assert!(!app.running);
}
```

### Performance Validation

```rust
#[tokio::test]
async fn test_throughput() {
    let (tx, rx) = mpsc::channel(1000);
    let stream = ReceiverStream::new(rx);

    // Send 1000 messages
    for i in 0..1000 {
        tx.send(i).await.unwrap();
    }
    drop(tx);

    let start = Instant::now();
    let count = stream.count().await;
    let elapsed = start.elapsed();

    assert_eq!(count, 1000);
    assert!(elapsed < Duration::from_secs(1), "Too slow!");
}
```

### Memory Leak Detection

```rust
#[tokio::test]
async fn test_no_leaks() {
    // Run with valgrind or heaptrack
    let stream = create_large_stream();

    stream.for_each(|_| ready(())).await;

    // Memory should be released after this point
}
```

---

## 6. Troubleshooting

### Issue: Stream Never Produces Items

**Symptom**: Stream created but `.next()` never returns

**Cause**: Source not connected or not started

**Fix**:
```rust
// ❌ WRONG - Stream created but not connected
let stream = IntervalStream::new(/* not started */);

// ✅ CORRECT - Interval must be created with tokio::time::interval
let interval = tokio::time::interval(Duration::from_millis(100));
let stream = IntervalStream::new(interval);
```

### Issue: High Memory Usage

**Symptom**: Memory grows unbounded

**Cause**: Buffering without backpressure

**Fix**:
```rust
// ❌ WRONG - Unbounded buffer
stream.buffer_unordered(usize::MAX)

// ✅ CORRECT - Bounded buffer with backpressure
stream.buffer_unordered(256)  // Reasonable limit
```

### Issue: Panics in Stream Processing

**Symptom**: `unwrap()` panics crash entire stream

**Cause**: Not handling errors in stream

**Fix**:
```rust
// ❌ WRONG - Panic kills stream
stream.map(|x| x.parse::<i32>().unwrap())

// ✅ CORRECT - Handle errors, continue stream
stream.filter_map(|x| x.parse::<i32>().ok())
```

### Issue: Stream Completes Too Early

**Symptom**: Stream ends before all items processed

**Cause**: One sub-stream completing ends merged stream

**Fix**:
```rust
// ❌ WRONG - Short stream ends everything
select_all(vec![
    short_stream,  // Ends after 3 items
    long_stream,   // Never processed after short_stream ends
])

// ✅ CORRECT - Use fuse() to keep stream alive
select_all(vec![
    short_stream.fuse(),
    long_stream.fuse(),
])
```

### Issue: Events Out of Order

**Symptom**: Events arrive in unexpected order

**Cause**: Concurrent processing reorders items

**Fix**:
```rust
// ❌ WRONG - buffer_unordered reorders
stream.buffer_unordered(10)

// ✅ CORRECT - buffered preserves order
stream.buffered(10)
```

---

## Summary

**Key Takeaways**:

1. **Never use `.await`** in production code - use reactive streams
2. **Declarative > Imperative** - describe transformations, not control flow
3. **Framework manages execution** - don't spawn tasks manually
4. **Backpressure is automatic** - bounded buffers prevent memory issues
5. **Resources are RAII** - Drop trait handles cleanup

**Next Steps**:

1. Review existing code for forbidden patterns
2. Practice with `hello_reactive.rs` and `reactive_prototype.rs`
3. Start migrating small modules first
4. Test thoroughly - streams are testable!
5. Ask questions - reactive programming is a paradigm shift

**Resources**:

- `yollayah/core/surfaces/tui/examples/hello_reactive.rs` - Simple example
- `yollayah/core/surfaces/tui/examples/reactive_prototype.rs` - Full prototype
- `progress/EPIC-001-TUI-reactive-overhaul.md` - Complete migration plan
- `knowledge/principles/PRINCIPLE-efficiency.md` - Architectural principles

---

**Last Updated**: 2026-01-03
**Status**: Sprint 0 deliverable - Ready for team review
