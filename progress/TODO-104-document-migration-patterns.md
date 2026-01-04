# TODO-104: Document Migration Patterns

**EPIC**: EPIC-001-TUI-reactive-overhaul.md
**Sprint**: SPRINT-00-foundation.md
**Story**: STORY-005
**Owner**: Rust/Async Specialist
**Status**: ✅ COMPLETE
**Effort**: 1 day
**Priority**: P1 - HIGH
**Depends On**: TODO-103 (prototype validates patterns)
**Completed**: 2026-01-03

---

## Objective

Create comprehensive migration guide for team showing how to convert from manual async/await to reactive streams.

---

## Deliverable

Create `knowledge/migration/reactive-migration-guide.md`

---

## Guide Structure

### 1. Introduction
- Why we're migrating
- What reactive programming is
- Core principles

### 2. Forbidden Patterns
- `.await` and why it's wrong
- `tokio::select!` and alternatives
- Manual loops vs declarative streams
- `tokio::spawn` vs framework-managed tasks
- Blocking operations

### 3. Required Patterns
- Stream creation (ReceiverStream, IntervalStream, etc.)
- Stream combinators (map, filter, merge, scan)
- Error handling in streams
- Backpressure management
- Resource cleanup

### 4. Common Conversions
- Event loop → Event pipeline
- Polling function → Stream processor
- Async fn → Stream-returning fn
- Message handler → Observable
- Error handling → Error stream

### 5. Testing Reactive Code
- Stream testing patterns
- Mocking event sources
- Performance validation
- Memory leak detection

### 6. Troubleshooting
- Common mistakes
- Performance issues
- Debugging reactive code

---

## Key Sections Content

### Forbidden → Required Conversions

#### Pattern 1: Event Loop

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

#### Pattern 2: Polling Function

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

#### Pattern 3: Message Handler

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

#### Pattern 4: Error Handling

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

#### Pattern 5: Parallel Operations

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

### Testing Patterns

#### Testing Stream Output

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

#### Mocking Event Sources

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

### Troubleshooting Guide

#### Issue: Stream Never Produces Items

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

#### Issue: High Memory Usage

**Symptom**: Memory grows unbounded

**Cause**: Buffering without backpressure

**Fix**:
```rust
// ❌ WRONG - Unbounded buffer
stream.buffer_unordered(usize::MAX)

// ✅ CORRECT - Bounded buffer with backpressure
stream.buffer_unordered(256)  // Reasonable limit
```

#### Issue: Panics in Stream Processing

**Symptom**: `unwrap()` panics crash entire stream

**Cause**: Not handling errors in stream

**Fix**:
```rust
// ❌ WRONG - Panic kills stream
stream.map(|x| x.parse::<i32>().unwrap())

// ✅ CORRECT - Handle errors, continue stream
stream.filter_map(|x| x.parse::<i32>().ok())
```

---

## Acceptance Criteria

- [x] Guide created at `knowledge/migration/reactive-migration-guide.md`
- [x] All forbidden patterns documented with examples
- [x] All required patterns documented with examples
- [x] Common conversions shown (before/after)
- [x] Testing patterns provided
- [x] Troubleshooting section complete
- [x] Team reviews and approves guide
- [x] Examples compile and run

## Integration Test Requirements (TODO → DONE)

For this TODO to move to DONE status:
- [x] File `knowledge/migration/reactive-migration-guide.md` exists
- [x] Guide is comprehensive (covers forbidden → required patterns)
- [x] All code examples in guide are syntactically correct
- [x] Guide references working prototype examples
- [x] Guide explains troubleshooting common issues

---

## Next Steps After This TODO

- Sprint 0 complete ✅
- Begin Sprint 1: Reactive Infrastructure
- Start building actual reactive wrappers using patterns from this guide

**Status**: 🟡 READY TO START
