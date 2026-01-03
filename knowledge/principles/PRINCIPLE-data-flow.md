# PRINCIPLE: Data Flow - Streams Over Copies, Share Over Clone

**Status**: ✅ Core Architectural Principle (Philosophical Guidance)
**Applies To**: All components, especially high-concurrency paths
**Enforcement**: Code review, architectural discussion (NOT mechanical linting)

---

## Core Philosophy

**AI-Way will orchestrate multiple AI agents in parallel, handling large language model contexts, streaming responses, and coordinating complex workflows. When that happens, naive data copying will kill performance.**

**Think streams, not snapshots. Think shared ownership, not defensive copies.**

This is a **philosophical principle**, not a mechanical rule. Use judgment. The goal is awareness, not dogma.

---

## The Problem: Data Copying Under High Concurrency

### Scenario: Multi-Agent Orchestration (Future State)

Imagine ai-way coordinating 10 AI agents simultaneously:
- Each agent has a 4000-token conversation context (~16KB of text)
- Each agent streams 200 tokens/sec response
- Total throughput: 2000 tokens/sec, ~8KB/sec text
- System lifetime: Hours to days

**Naive Approach (Bad)**:
```rust
// ❌ TERRIBLE - Clones entire context for every agent
for agent in agents {
    let context = conversation.clone(); // 16KB × 10 = 160KB copied!
    agent.send_request(context).await;
}

// ❌ TERRIBLE - Copies every token through multiple layers
let token = llm.next_token().await;
let copied_token = token.clone(); // Copy 1
conductor.handle_token(copied_token.clone()).await; // Copy 2
surface.display_token(copied_token.clone()).await; // Copy 3
```

**Impact**:
- **Memory**: 160KB × 10 requests/sec = 1.6MB/sec allocation rate
- **CPU**: Copying overhead dominates actual work
- **Cache thrashing**: Large copies evict useful data from L1/L2 cache
- **GC pressure**: (Not Rust, but concept applies to allocator fragmentation)

---

## The Principles

### Principle 1: Stream Large Data, Don't Copy It

**For datasets > 1KB or infinite streams, use iterators/streams.**

**Bad - Materialized Copy**:
```rust
// ❌ BAD - Loads entire conversation into memory, copies it
async fn get_conversation_text(&self) -> String {
    let mut result = String::new();
    for msg in &self.messages {
        result.push_str(&msg.content); // Copies all content into one String
        result.push('\n');
    }
    result // Returns 100KB+ String
}

// Caller must clone to keep using it
let text = self.get_conversation_text().await;
send_to_agent(text.clone()).await; // Another copy!
```

**Good - Streaming Iterator**:
```rust
// ✅ GOOD - Iterator yields references, no copying
fn conversation_lines(&self) -> impl Iterator<Item = &str> + '_ {
    self.messages.iter().map(|msg| msg.content.as_str())
}

// Caller can consume directly without copying
for line in conversation.conversation_lines() {
    send_line_to_agent(line).await; // No copy, just references
}

// Or collect only if needed
let text: String = conversation.conversation_lines().collect();
```

**Best - Async Stream** (for infinite/network data):
```rust
use futures::stream::{Stream, StreamExt};

// ✅ BEST - Async stream for LLM tokens
async fn stream_tokens(&self) -> impl Stream<Item = Token> + '_ {
    futures::stream::unfold(self.state.clone(), |state| async move {
        match self.llm.next_token().await {
            Some(token) => Some((token, state)),
            None => None,
        }
    })
}

// Caller can process tokens as they arrive
let mut token_stream = agent.stream_tokens().await;
while let Some(token) = token_stream.next().await {
    display(token); // No buffering, no copying
}
```

---

### Principle 2: Share Ownership (Arc), Don't Clone Large Structs

**For read-heavy data shared across tasks, use `Arc<T>` (cheap pointer clone, not data clone).**

**Bad - Deep Copy on Every Share**:
```rust
// ❌ BAD - Clones 4KB conversation context for every agent
#[derive(Clone)]
struct ConversationContext {
    messages: Vec<Message>, // 4KB
    system_prompt: String,  // 1KB
}

async fn run_agents(&self, agents: &[Agent]) {
    for agent in agents {
        let context = self.context.clone(); // ← 5KB × 10 agents = 50KB!
        tokio::spawn(async move {
            agent.process(context).await;
        });
    }
}
```

**Good - Shared Ownership**:
```rust
// ✅ GOOD - Arc allows cheap pointer cloning
use std::sync::Arc;

struct ConversationContext {
    messages: Vec<Message>, // 4KB (stored once)
    system_prompt: String,  // 1KB
}

async fn run_agents(&self, agents: &[Agent]) {
    let shared_context = Arc::new(self.context.clone()); // Clone once

    for agent in agents {
        let context = Arc::clone(&shared_context); // ← Just 8 bytes (pointer)!
        tokio::spawn(async move {
            agent.process(&context).await; // Pass reference
        });
    }
}
```

**Advanced - Arc + Mutex for Mutable Shared State**:
```rust
// ✅ GOOD - Shared mutable state (when needed)
use tokio::sync::RwLock;

struct SharedState {
    context: Arc<RwLock<ConversationContext>>,
}

// Many readers, few writers
async fn read_context(&self) -> String {
    let guard = self.context.read().await;
    guard.messages[0].content.clone() // Only clone what you need
}

async fn update_context(&self, msg: Message) {
    let mut guard = self.context.write().await;
    guard.messages.push(msg); // Modify in place
}
```

---

### Principle 3: Use Cow (Copy-on-Write) for Conditional Cloning

**When data is usually read but occasionally modified, use `Cow<T>`.**

**Bad - Always Clones**:
```rust
// ❌ BAD - Always clones, even when no modification needed
fn process_message(msg: &str) -> String {
    let mut processed = msg.to_string(); // Always clones
    if msg.contains("ERROR") {
        processed = format!("[!] {}", processed);
    }
    processed
}
```

**Good - Clone Only When Needed**:
```rust
use std::borrow::Cow;

// ✅ GOOD - Zero-copy when no modification
fn process_message(msg: &str) -> Cow<str> {
    if msg.contains("ERROR") {
        Cow::Owned(format!("[!] {}", msg)) // Clone only when modifying
    } else {
        Cow::Borrowed(msg) // No clone, just reference
    }
}

// Usage
let result = process_message("Hello"); // No allocation!
let error = process_message("ERROR: fail"); // Allocates only when needed
```

---

### Principle 4: Avoid Intermediate Allocations (Iterator Chains)

**Use iterator combinators instead of building temporary vectors.**

**Bad - Multiple Intermediate Vectors**:
```rust
// ❌ BAD - Creates 3 temporary vectors!
fn get_important_messages(&self) -> Vec<String> {
    let all_msgs: Vec<_> = self.messages.iter().collect(); // Temp vec 1
    let filtered: Vec<_> = all_msgs.iter()
        .filter(|m| m.important)
        .collect(); // Temp vec 2

    filtered.iter()
        .map(|m| m.content.clone())
        .collect() // Final vec 3
}
```

**Good - Single Pass, Single Allocation**:
```rust
// ✅ GOOD - Zero intermediate allocations, one final collect
fn get_important_messages(&self) -> Vec<String> {
    self.messages.iter()
        .filter(|m| m.important)
        .map(|m| m.content.clone())
        .collect() // Only one allocation
}

// ✅ EVEN BETTER - Return iterator, let caller decide
fn important_messages(&self) -> impl Iterator<Item = &Message> + '_ {
    self.messages.iter().filter(|m| m.important)
}
```

---

### Principle 5: Zero-Copy Deserialization (When Possible)

**For large payloads, deserialize in-place or use zero-copy formats.**

**Context**: ai-way will receive large JSON/protobuf messages from LLMs and agents.

**Naive Approach**:
```rust
// ❌ SUBOPTIMAL - Deserializes into owned Strings
#[derive(Deserialize)]
struct LlmResponse {
    content: String,      // Allocates + copies from JSON
    model: String,        // Allocates + copies
    metadata: HashMap<String, String>, // Many allocations
}
```

**Zero-Copy Approach** (where applicable):
```rust
// ✅ BETTER - Borrows from original buffer (serde zero-copy)
use serde::Deserialize;

#[derive(Deserialize)]
struct LlmResponse<'a> {
    #[serde(borrow)]
    content: &'a str,     // No allocation, borrows from JSON buffer
    #[serde(borrow)]
    model: &'a str,       // No allocation
    // Note: Only works if you don't need to own the data
}

// Parse once, use references
let json_bytes = receive_from_network().await;
let response: LlmResponse = serde_json::from_slice(&json_bytes)?;
process_content(response.content); // No copy!
```

**When to Clone**: If data needs to outlive the buffer, clone selectively:
```rust
// ✅ GOOD - Clone only what needs to persist
struct PersistedResponse {
    content: String, // Cloned because we store it
    model: String,   // Cloned
}

impl<'a> From<LlmResponse<'a>> for PersistedResponse {
    fn from(resp: LlmResponse<'a>) -> Self {
        Self {
            content: resp.content.to_string(), // Clone here, explicitly
            model: resp.model.to_string(),
        }
    }
}
```

---

## When to Copy (Exceptions)

**Not all copying is bad. Copy when:**

1. **Data is small** (< 100 bytes): `Copy` trait is fine for primitives, small structs
2. **Ownership simplifies logic**: Sometimes a clone prevents lifetime hell
3. **Mutation is needed**: Can't mutate through `Arc` without locking
4. **Performance doesn't matter**: Initialization code, CLI parsing, one-time setup
5. **Clarity over performance**: If zero-copy makes code unreadable, copy

**Example - Small Data is Fine to Copy**:
```rust
// ✅ OK - Token is tiny (16 bytes), Copy is appropriate
#[derive(Copy, Clone)]
struct Token {
    id: u64,
    timestamp: u64,
}

// Copying is cheaper than Arc indirection for tiny types
let token = Token { id: 1, timestamp: now() };
send_token(token); // Copy is fine here
```

---

## Practical Guidelines

### For Large Conversation Contexts

**DO**:
- Store conversation in `Arc<Vec<Message>>` for sharing across agents
- Use iterators to build prompts, avoid `String::new() + push_str()` loops
- Stream tokens from LLM, don't buffer entire response

**DON'T**:
- Clone entire conversation for each agent request
- Build giant strings by concatenating messages
- Allocate intermediate vectors when iterators would work

### For Streaming LLM Responses

**DO**:
- Use `Stream<Item = Token>` for incremental processing
- Forward tokens directly to surfaces without buffering
- Use channels with appropriate buffer sizes (100-256 for bursts)

**DON'T**:
- Collect all tokens into `Vec<Token>` before processing
- Clone tokens through multiple layers (pass references or move)
- Buffer entire response in memory "just in case"

### For Agent Coordination (Future)

**DO**:
- Share read-only config via `Arc<Config>`
- Use `Arc<RwLock<T>>` for shared mutable state (with read-heavy pattern)
- Pass task descriptions by reference, not value

**DON'T**:
- Clone agent configurations for every task spawn
- Serialize/deserialize data just to pass between tasks
- Create defensive copies "to be safe"

---

## Mental Model: Think Like a Stream Processor

**Bad Mental Model** (Batch Processing):
```
1. Load all data into memory
2. Process entire dataset
3. Output results
```

**Good Mental Model** (Stream Processing):
```
1. Receive data chunk
2. Process immediately
3. Forward to next stage
4. Repeat (bounded memory)
```

**Example - Conversation Export**:

**Batch Approach** (Bad):
```rust
// ❌ BAD - Loads everything, processes in batch
async fn export_conversation(&self) -> String {
    let mut output = String::new(); // Unbounded growth!

    for msg in &self.messages { // Loads all messages
        output.push_str(&format!("{}: {}\n", msg.role, msg.content));
    }

    output // Returns 100KB+ String
}
```

**Streaming Approach** (Good):
```rust
// ✅ GOOD - Processes incrementally, writes to output stream
async fn export_conversation<W: AsyncWrite>(&self, writer: &mut W) -> Result<()> {
    for msg in &self.messages {
        writer.write_all(format!("{}: {}\n", msg.role, msg.content).as_bytes()).await?;
        // Message content is dropped after writing, bounded memory
    }
    Ok(())
}

// Usage - writes directly to file without buffering entire conversation
let mut file = tokio::fs::File::create("export.txt").await?;
conductor.export_conversation(&mut file).await?;
```

---

## Observability: Measure, Don't Guess

**Use profiling to validate assumptions:**

1. **Memory Profiling**:
   ```bash
   # Valgrind massif - track allocations
   valgrind --tool=massif ./yollayah.sh

   # Heaptrack - more detailed
   heaptrack ./yollayah.sh
   ```

2. **Allocation Tracking**:
   ```rust
   // Count allocations in hot paths (debug builds)
   #[global_allocator]
   static ALLOC: dhat::Alloc = dhat::Alloc;
   ```

3. **Flamegraphs**:
   ```bash
   # CPU time - find cloning hotspots
   cargo flamegraph
   ```

**If profiling shows cloning is < 5% of CPU time, it's probably fine.**

---

## Summary Table

| Pattern | Use When | Example |
|---------|----------|---------|
| **Iterator** | Large datasets, transformations | `messages.iter().map(...)` |
| **Stream** | Infinite/async data | `Stream<Item = Token>` |
| **Arc** | Shared read-only data | `Arc<Config>` across tasks |
| **Cow** | Conditional mutation | `Cow<str>` for strings |
| **Zero-copy** | Large deserializations | `#[serde(borrow)]` |
| **Clone** | Small data (< 100B) | `Copy` trait for primitives |

---

## Future Considerations

When ai-way reaches production with multi-agent orchestration:

1. **Agent Context Sharing**:
   - 10+ agents × 4KB context = 40KB if copied, 8 bytes if Arc
   - Use `Arc<ConversationContext>` from day one

2. **LLM Response Streaming**:
   - 200 tokens/sec × 10 agents = 2000 tokens/sec
   - Stream processing is mandatory, not optional

3. **Parallel Task Execution**:
   - 100+ concurrent tasks is realistic
   - Shared state via `Arc<RwLock>`, not cloning

4. **Large Model Contexts**:
   - GPT-4: 128K tokens = ~500KB text
   - Streaming + zero-copy patterns essential

---

## Remember

**This is philosophy, not dogma:**
- Measure before optimizing
- Clarity > premature optimization
- But **be aware** of copying large data in hot paths
- Design for streams when data is naturally streaming (LLM responses, logs, events)
- Share read-heavy data via Arc, don't clone

**The goal**: When ai-way coordinates 50 AI agents with 100K token contexts and 10K events/sec, performance is bounded by LLM inference, not data copying overhead.
