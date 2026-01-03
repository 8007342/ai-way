# EPIC-004: Incremental Message Rendering

**Epic**: ODYSSEY: TUI Framebuffer Refactor
**Created**: 2026-01-03
**Owner**: TBD
**Timeline**: WEEK 2-3 (4 hours)
**Status**: 📋 PLANNED
**Depends On**: EPIC-001

---

## 🎯 Goal

Stop rebuilding the entire conversation every frame. Cache wrapped lines, only process new/changed messages, update streaming message incrementally.

**Expected Impact**: 10-50x faster streaming rendering (process delta, not full history)

---

## 📋 Stories

### ⏳ STORY 1: Add Wrapped Line Cache to DisplayMessage
**Status**: PENDING
**Time**: 30 mins
**File**: `tui/src/display.rs`

**Current**:
```rust
pub struct DisplayMessage {
    pub id: MessageId,
    pub role: DisplayRole,
    pub content: String,
    pub streaming: bool,
    pub metadata: Option<ResponseMetadata>,
}
```

**Target**:
```rust
pub struct DisplayMessage {
    pub id: MessageId,
    pub role: DisplayRole,
    pub content: String,
    pub streaming: bool,
    pub metadata: Option<ResponseMetadata>,

    // NEW: Cache wrapped lines
    cached_wrapped: Option<CachedWrapping>,
    content_hash: u64,  // Detect content changes
}

struct CachedWrapping {
    width: usize,        // Terminal width when wrapped
    lines: Vec<String>,  // Wrapped lines
}

impl DisplayMessage {
    pub fn get_wrapped_lines(&mut self, width: usize) -> &[String] {
        let current_hash = calculate_hash(&self.content);

        // Check cache validity
        if let Some(ref cached) = self.cached_wrapped {
            if cached.width == width && self.content_hash == current_hash {
                return &cached.lines;  // Cache hit!
            }
        }

        // Cache miss - wrap and cache
        let lines = textwrap::wrap(&self.content, width)
            .iter()
            .map(|s| s.to_string())
            .collect();

        self.cached_wrapped = Some(CachedWrapping { width, lines });
        self.content_hash = current_hash;

        &self.cached_wrapped.as_ref().unwrap().lines
    }
}
```

**Tasks**:
- [ ] Add cached_wrapped and content_hash fields
- [ ] Implement get_wrapped_lines() with cache check
- [ ] Use FNV hash or similar for content_hash
- [ ] Test cache hit/miss behavior

---

### ⏳ STORY 2: Track Last Rendered State in App
**Status**: PENDING
**Time**: 30 mins
**File**: `tui/src/app.rs`

**Add conversation cache**:
```rust
struct App {
    // Existing fields...

    // NEW: Track what we last rendered
    conversation_cache: ConversationCache,
}

struct ConversationCache {
    /// Number of messages last rendered
    message_count: usize,

    /// Length of last streaming message
    streaming_len: usize,

    /// Terminal width when last rendered
    terminal_width: usize,

    /// Pre-built wrapped lines (ready to render)
    wrapped_lines: Vec<LineMeta>,
}

impl ConversationCache {
    fn invalidate(&mut self) {
        self.wrapped_lines.clear();
    }

    fn is_valid(&self, current_count: usize, current_width: usize) -> bool {
        self.message_count == current_count
            && self.terminal_width == current_width
    }
}
```

**Tasks**:
- [ ] Add ConversationCache struct
- [ ] Add cache field to App
- [ ] Initialize in App::new()
- [ ] Add invalidation method

---

### ⏳ STORY 3: Implement Incremental render_conversation()
**Status**: PENDING
**Time**: 2 hours
**File**: `tui/src/app.rs` (lines 850-1005)

**Current** (rebuilds everything):
```rust
fn render_conversation(&mut self) {
    let mut all_lines: Vec<LineMeta> = Vec::new();

    // PROBLEM: Processes ALL messages every frame
    for msg in &self.display.messages {
        let wrapped = textwrap::wrap(&content, width);
        for line in wrapped {
            all_lines.push(LineMeta { ... });
        }
    }

    // ... render visible_lines ...
}
```

**Target** (incremental):
```rust
fn render_conversation(&mut self) {
    let current_count = self.display.messages.len();
    let terminal_width = self.size.0 as usize;

    // Check if we can use cache
    if self.conversation_cache.is_valid(current_count, terminal_width) {
        // Cache hit! Skip wrapping entirely
        return self.render_cached_conversation();
    }

    // Determine what changed
    let last_count = self.conversation_cache.message_count;

    if current_count > last_count {
        // NEW MESSAGES: Only process new ones
        self.append_new_messages(last_count, current_count, terminal_width);
    } else if let Some(streaming_id) = &self.display.streaming_id {
        // STREAMING: Only update last message
        self.update_streaming_message(terminal_width);
    } else {
        // Full rebuild (terminal resized or messages deleted)
        self.rebuild_conversation(terminal_width);
    }

    // Render from cache
    self.render_cached_conversation();
}

fn append_new_messages(&mut self, start: usize, end: usize, width: usize) {
    for msg in &mut self.display.messages[start..end] {
        let wrapped = msg.get_wrapped_lines(width);  // Uses cache!
        for line in wrapped {
            self.conversation_cache.wrapped_lines.push(LineMeta { ... });
        }
    }

    self.conversation_cache.message_count = end;
}

fn update_streaming_message(&mut self, width: usize) {
    // Remove old streaming lines from cache
    let last_msg = self.display.messages.last_mut().unwrap();

    // Get newly wrapped lines (cache will be invalid due to content change)
    let wrapped = last_msg.get_wrapped_lines(width);

    // Replace last message's lines in cache
    // ... (implementation details)
}
```

**Tasks**:
- [ ] Implement cache validity check
- [ ] Implement append_new_messages()
- [ ] Implement update_streaming_message()
- [ ] Implement rebuild_conversation()
- [ ] Test with new messages
- [ ] Test with streaming
- [ ] Test with terminal resize

---

### ⏳ STORY 4: Handle Terminal Resize
**Status**: PENDING
**Time**: 30 mins
**File**: `tui/src/app.rs`

**When terminal resizes**:
1. Invalidate all cached wrapping (width changed)
2. Force full rebuild next frame
3. Re-wrap all messages at new width

**Implementation**:
```rust
fn handle_resize(&mut self, new_width: u16, new_height: u16) {
    if (new_width, new_height) != self.size {
        // Invalidate conversation cache
        self.conversation_cache.invalidate();

        // Invalidate per-message caches
        for msg in &mut self.display.messages {
            msg.cached_wrapped = None;  // Force re-wrap
        }

        self.size = (new_width, new_height);
    }
}
```

**Tasks**:
- [ ] Detect terminal resize events
- [ ] Invalidate conversation cache
- [ ] Invalidate per-message caches
- [ ] Verify re-wrap at new width

---

### ⏳ STORY 5: Optimize LineMeta Allocation
**Status**: PENDING
**Time**: 30 mins
**File**: `tui/src/app.rs`

**Current**: Creates new Vec and LineMeta structs every frame

**Optimization**: Reuse allocations

```rust
struct ConversationCache {
    // ... other fields ...

    /// Reusable buffer (avoid Vec allocations)
    wrapped_lines: Vec<LineMeta>,
}

impl ConversationCache {
    fn append_lines(&mut self, lines: impl Iterator<Item = LineMeta>) {
        // Extend existing Vec (reuses capacity)
        self.wrapped_lines.extend(lines);
    }

    fn clear_for_rebuild(&mut self) {
        // Clear but keep capacity
        self.wrapped_lines.clear();
    }
}
```

**Tasks**:
- [ ] Reuse Vec capacity instead of allocating new
- [ ] Profile heap allocations before/after
- [ ] Measure reduction in GC pressure

---

### ⏳ STORY 6: Measure Streaming Performance
**Status**: PENDING
**Time**: 30 mins

**Test Scenarios**:

1. **New message arrives**
   - Before: Re-wrap all 100 messages
   - After: Wrap only 1 new message
   - Expected: 100x faster

2. **Streaming token arrives**
   - Before: Re-wrap all messages including streaming one
   - After: Re-wrap only streaming message
   - Expected: 10-50x faster (depends on message count)

3. **Idle (no changes)**
   - Before: Re-wrap everything (wasted work)
   - After: Skip wrapping entirely
   - Expected: 0ms vs 10-20ms

**Metrics**:
```bash
# Stress test
cargo test --test stress_test stress_test_rapid_token_streaming --release -- --nocapture

# Measure tokens/sec before and after
```

**Tasks**:
- [ ] Baseline: tokens/sec with current implementation
- [ ] After: tokens/sec with incremental rendering
- [ ] Document 10-50x improvement
- [ ] Verify streaming feels instant

---

## 📊 Success Criteria

- ✅ Streaming feels instant (matches GPU speed)
- ✅ Only new/changed messages wrapped
- ✅ Cache invalidation works on resize
- ✅ 10-50x performance improvement measured
- ✅ Memory usage bounded (caches don't grow unbounded)
- ✅ No visual regressions

---

## 🔍 Technical Details

### Why Current Approach is Slow

**Lines 850-930 in `app.rs`**:
```rust
// EVERY FRAME, for EVERY MESSAGE:
for msg in &self.display.messages {
    let wrapped = textwrap::wrap(&content, width);  // ← EXPENSIVE
    for line in wrapped {
        all_lines.push(LineMeta {
            text: line.to_string(),  // ← HEAP ALLOCATION
            // ...
        });
    }
}
```

**At 10 FPS with 50 messages**:
- 50 messages × 10 wraps/sec = **500 text wrapping operations/second**
- 50 messages × 5 lines/msg × 10 FPS = **2,500 string allocations/second**

### How Incremental Rendering Helps

**With cache**:
- New message: Wrap 1 message (not 50)
- Streaming: Re-wrap 1 message (not 50)
- Idle: Wrap 0 messages (not 50)

**Math**:
- 10 FPS × 0 wraps = **0 operations/second when idle**
- 1 new message/minute × 1 wrap = **0.016 wraps/second**
- Streaming: 1 re-wrap/frame = **10 wraps/second** (vs 500)

**Result**: 50-500x reduction in text wrapping operations.

---

## 🔗 Related

- EPIC-001: Quick Wins (foundation)
- BUG-003-tui-performance-regression.md (root cause analysis)
- `tui/src/app.rs` lines 850-1005 (render_conversation)
- `tui/src/display.rs` (DisplayMessage struct)
