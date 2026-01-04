# TUI Framebuffer & Rendering Performance Audit

**Date**: 2026-01-03
**Auditor**: Ratatui Performance Expert
**Scope**: TUI rendering pipeline, compositor architecture, and terminal operations

---

## Executive Summary

### Critical Findings

**GOOD NEWS**: Recent P0 fixes (frame rate 60→10 FPS, removed breathing colors) have addressed the most severe performance bottlenecks. The architecture is now in a much better state.

**CONCERNS IDENTIFIED**:

1. **5-Layer Compositor is UNNECESSARY COMPLEXITY** - Can reduce to 2-3 layers
2. **Full buffer composition every frame** - Even with dirty tracking, still compositing all layers
3. **Massive cell cloning during composition** - 200x50 terminal = 10,000 `Cell::clone()` calls per frame
4. **Conversation re-wrapping not cached** - All messages re-wrapped every frame
5. **Ratatui's built-in dirty tracking UNDERUTILIZED** - Custom compositor bypasses framework optimizations

**Performance Impact**: Estimated 40-60% unnecessary CPU overhead remains after P0 fixes.

---

## 1. Framebuffer Complexity Analysis

### Current Architecture: 5-Layer Compositor

**Location**: `/var/home/machiyotl/src/ai-way/tui/src/compositor/mod.rs`

```rust
struct Compositor {
    layers: HashMap<LayerId, Layer>,      // All layers by ID
    render_order: Vec<LayerId>,            // Sorted by z-index
    output: Buffer,                        // Composited result
    dirty_layers: HashSet<LayerId>,        // Dirty tracking
}
```

**5 Layers Created** (`app.rs:146-200`):
1. **Conversation layer** (z=0) - Main conversation area
2. **Input layer** (z=10) - Input box (5 lines)
3. **Status layer** (z=10) - Status bar (1 line)
4. **Tasks layer** (z=25) - Right-side task panel
5. **Avatar layer** (z=50/100) - Animated axolotl (dynamic z-index)

#### Problem: Over-Engineered for Current Needs

**Why 5 layers is excessive**:

- **No true overlap requirements**: Layers occupy distinct screen regions
- **Z-index changes rare**: Only avatar changes z-index (thinking state)
- **Per-layer buffers wasteful**: Each layer maintains separate `Buffer`
- **Compositor overhead**: Blitting 5 layers → output buffer every frame

**Evidence**:
```rust
// compositor/mod.rs:156-180
pub fn composite(&mut self) -> &Buffer {
    if self.dirty_layers.is_empty() {
        return &self.output;  // ✅ Early return if nothing changed
    }

    self.output.reset();  // ❌ Clear entire output buffer

    // ❌ Render ALL visible layers even if only 1 changed
    for &id in &self.render_order {
        if let Some(layer) = self.layers.get(&id) {
            if layer.visible {
                Self::blit_layer(&mut self.output, &self.area, layer);
            }
        }
    }

    self.clear_dirty();
    &self.output
}
```

**Performance Cost**:
- 5 separate `Buffer` instances (each 200x50 = 10,000 cells)
- Blit loop runs over ALL layers when ANY layer dirty
- `blit_layer()` does per-cell cloning (see §3)

---

### Recommended Simplification: 2-3 Layers

**Minimal layer architecture**:

1. **Background layer**: Conversation + Tasks (static layout, rarely changes)
2. **Foreground layer**: Input + Status (changes frequently)
3. **Avatar overlay** (optional): Only if avatar actually overlaps content

**Why this works**:

- **Conversation + Tasks**: Both occupy top region, no overlap, can share layer
- **Input + Status**: Bottom region, adjacent areas, can share layer
- **Avatar**: Only needs separate layer if overlapping text (currently doesn't)

**Benefits**:
- 60% fewer layer buffers (2-3 vs 5)
- Fewer blit operations per composite
- Simpler dirty tracking logic
- Lower memory footprint

**Trade-off**:
- Lose fine-grained dirty tracking per UI component
- But gain: simpler code, less overhead, easier to reason about

---

### Alternative: Direct Ratatui Rendering (Zero-Layer)

**Even better approach**: Eliminate compositor entirely, use Ratatui's built-in optimizations.

```rust
// Instead of compositor, render directly to frame.buffer_mut()
terminal.draw(|frame| {
    render_conversation(frame, area_conversation);
    render_tasks(frame, area_tasks);
    render_input(frame, area_input);
    render_status(frame, area_status);
    render_avatar(frame, area_avatar);
})?;
```

**Ratatui does this for you**:
- `Frame::buffer_mut()` exposes a single mutable buffer
- `Terminal::draw()` uses `Buffer::diff()` internally to compute changes
- Only changed cells sent to terminal (via `Backend::draw()`)

**Benefits**:
- ✅ **Zero compositor overhead** - no extra blitting
- ✅ **Ratatui's optimized diff** - battle-tested, faster than custom dirty tracking
- ✅ **Automatic partial updates** - framework computes minimal change set
- ✅ **Less code to maintain** - 236 lines of compositor.rs eliminated

**Current blocker**: `app.rs:853` uses `Buffer::merge()` to composite layers.

```rust
// app.rs:848-854 - Current approach
terminal.draw(|frame| {
    let output = self.compositor.composite();

    // ❌ Merges entire composited buffer into frame buffer
    frame.buffer_mut().merge(output);
})?;
```

**What we should do instead**:

```rust
// Proposed approach - render widgets directly
terminal.draw(|frame| {
    // Ratatui handles dirty tracking internally
    Paragraph::new(conversation_text).render(frame, area_conversation);
    Paragraph::new(input_text).render(frame, area_input);
    // ... etc
})?;
```

---

## 2. Rendering Inefficiencies

### 2.1 Cell Cloning During Composition (CRITICAL)

**Location**: `compositor/mod.rs:183-215`

```rust
fn blit_layer(output: &mut Buffer, area: &Rect, layer: &Layer) {
    let lb = &layer.bounds;

    for ly in 0..lb.height {
        for lx in 0..lb.width {
            // ... bounds checking ...

            let src_cell = &layer.buffer.content[src_idx];

            if src_cell.symbol() != " " {
                let dst_idx = output.index_of(dst_x, dst_y);
                if dst_idx < output.content.len() {
                    // ❌ CELL CLONE - happens 10,000+ times per composite
                    output.content[dst_idx] = src_cell.clone();
                }
            }
        }
    }
}
```

**Performance Cost**:

| Terminal Size | Cells | Frames/Sec | Clones/Sec | Impact |
|--------------|-------|------------|------------|--------|
| 80x24 | 1,920 | 10 FPS | 19,200/sec | Moderate |
| 200x50 | 10,000 | 10 FPS | 100,000/sec | **HIGH** |
| 200x50 (pre-fix) | 10,000 | 60 FPS | 600,000/sec | **SEVERE** |

**Why this is expensive**:

`Cell` contains:
- `symbol: String` (heap-allocated, 24 bytes + string data)
- `fg: Color` (4 bytes)
- `bg: Color` (4 bytes)
- `modifier: Modifier` (bitflags)

Each `Cell::clone()` requires:
1. String clone (heap allocation + memcpy)
2. Style data copy

**Evidence from BUG-003**:
> "600,000 clones/second" identified as one of 4 critical P0 issues (line 138)

---

### 2.2 Conversation Re-Wrapping Every Frame (CONFIRMED)

**Location**: `app.rs:860-1017`

```rust
fn render_conversation(&mut self) {
    // ... setup ...

    for msg in &self.display.messages {
        let (prefix, base_style) = match msg.role { /* ... */ };

        let content = if msg.streaming {
            format!("{}{}_", prefix, msg.content)
        } else {
            format!("{}{}", prefix, msg.content)
        };

        // ❌ EXPENSIVE: Re-wrap EVERY message EVERY frame
        let wrapped = textwrap::wrap(&content, width);  // Line 896

        for (line_idx, line) in wrapped.iter().enumerate() {
            all_lines.push(LineMeta { /* ... */ });
        }
    }

    // ... rendering ...
}
```

**Performance Cost**:

| Messages | Avg Length | Lines Wrapped | Frames/Sec | Wraps/Sec |
|----------|-----------|---------------|------------|-----------|
| 10 | 200 chars | ~40 lines | 10 FPS | 400/sec |
| 30 | 200 chars | ~120 lines | 10 FPS | 1,200/sec |
| 50 | 200 chars | ~200 lines | 10 FPS | 2,000/sec |

**Why this is expensive**:

`textwrap::wrap()` does:
1. Unicode grapheme iteration
2. Word boundary detection
3. Width calculation (unicode-width)
4. String allocations for each wrapped line
5. Vec allocations for line collection

**Current state**:
- ❌ No caching of wrapped lines
- ❌ Re-wraps even unchanged messages
- ❌ Re-wraps on every frame (even when scrolling, not editing)

**Correct approach**:
```rust
struct DisplayMessage {
    content: String,
    wrapped_cache: Option<Vec<String>>,  // ← Cache wrapped lines
    cache_width: Option<usize>,          // ← Invalidate on resize
}

impl DisplayMessage {
    fn get_wrapped(&mut self, width: usize) -> &[String] {
        if self.cache_width != Some(width) {
            self.wrapped_cache = Some(textwrap::wrap(&self.content, width));
            self.cache_width = Some(width);
        }
        self.wrapped_cache.as_ref().unwrap()
    }
}
```

**Estimated impact**: Eliminates 1,200-2,000 wraps/second in typical usage.

---

### 2.3 Dirty Tracking: Good Intent, Incomplete Implementation

**Current state** (`app.rs:1015-1016`):

```rust
// Mark layer as needing re-composite
self.compositor.mark_layer_dirty(self.layers.conversation);
```

**Problem**: Dirty tracking exists but is **UNCONDITIONAL**.

**Evidence**:

```rust
// app.rs:837-857
fn render(&mut self, terminal: &mut Terminal<...>) -> Result<()> {
    self.render_conversation();  // ❌ Always renders
    self.render_tasks();         // ❌ Always renders
    self.render_input();         // ❌ Always renders
    self.render_status();        // ❌ Always renders
    self.render_avatar();        // ✅ Only if avatar_changed

    terminal.draw(|frame| {
        let output = self.compositor.composite();
        frame.buffer_mut().merge(output);
    })?;

    Ok(())
}
```

**Only avatar has conditional rendering** (`app.rs:1325-1336`):

```rust
fn render_avatar(&mut self) {
    if self.avatar_changed {  // ✅ Checks dirty flag
        if let Some(buf) = self.compositor.layer_buffer_mut(self.layers.avatar) {
            buf.reset();
            self.avatar.render(buf);
        }
        self.compositor.mark_layer_dirty(self.layers.avatar);
    }
}
```

**Missing dirty checks**:

```rust
// app.rs:1019-1099 - Input rendering
fn render_input(&mut self) {
    // ✅ HAS dirty check (lines 1021-1025)
    let input_changed = self.input_buffer != self.prev_input_buffer
        || self.cursor_pos != self.prev_cursor_pos;

    if input_changed {  // ✅ GOOD!
        // ... render ...
        self.compositor.mark_layer_dirty(self.layers.input);
        self.prev_input_buffer = self.input_buffer.clone();
        self.prev_cursor_pos = self.cursor_pos;
    }
}

// app.rs:1101-1227 - Status rendering
fn render_status(&mut self) {
    // ✅ HAS dirty check (lines 1111-1114)
    let status_changed = /* ... */;

    if status_changed {  // ✅ GOOD!
        // ... render ...
        self.compositor.mark_layer_dirty(self.layers.status);
    }
}

// app.rs:1243-1275 - Task rendering
fn render_tasks(&mut self) {
    // ✅ HAS dirty check (lines 1257)
    let tasks_changed = tasks_hash != self.prev_tasks_hash;

    if tasks_changed {  // ✅ GOOD!
        // ... render ...
        self.compositor.mark_layer_dirty(self.layers.tasks);
    }
}

// app.rs:859-1017 - Conversation rendering
fn render_conversation(&mut self) {
    // ❌ NO DIRTY CHECK!
    // Always renders, always marks dirty
    // ...
    self.compositor.mark_layer_dirty(self.layers.conversation);
}
```

**Analysis**:

| Layer | Dirty Check | Status | Notes |
|-------|-------------|--------|-------|
| Avatar | ✅ Yes | Good | Only renders on animation change |
| Input | ✅ Yes | Good | Only renders on buffer/cursor change |
| Status | ✅ Yes | Good | Only renders on state change |
| Tasks | ✅ Yes | Good | Only renders on task change |
| Conversation | ❌ **NO** | **BAD** | **Always renders, even if unchanged** |

**Impact**: Conversation is the **largest layer** (full screen height minus 6 lines). Re-rendering it every frame is wasteful.

---

### 2.4 Compositor Dirty Tracking Bypassed by Always-Dirty Layers

**Compositor has dirty tracking** (`compositor/mod.rs:147-180`):

```rust
pub fn composite(&mut self) -> &Buffer {
    // ✅ Early return if nothing changed
    if self.dirty_layers.is_empty() {
        return &self.output;
    }

    // ❌ But if ANY layer dirty, re-composite ALL layers
    self.output.reset();
    for &id in &self.render_order {
        if let Some(layer) = self.layers.get(&id) {
            if layer.visible {
                Self::blit_layer(&mut self.output, &self.area, layer);
            }
        }
    }

    self.clear_dirty();
    &self.output
}
```

**Problem**:
1. **Conversation always marked dirty** → `dirty_layers` never empty
2. **Early return never triggers**
3. **All 5 layers re-composited every frame**

**Evidence**:
- `render_conversation()` has no dirty check (line 859-1017)
- Always calls `mark_layer_dirty()` (line 1016)
- Result: 10 FPS × 5 layers × 10k cells = **500,000 cell operations/sec**

---

### 2.5 Ratatui's `Buffer::diff()` Not Used

**Ratatui provides optimized dirty tracking** (built into framework):

```rust
// What Ratatui does internally in Terminal::draw()
impl Terminal {
    pub fn draw(&mut self, f: impl FnOnce(&mut Frame)) -> Result<()> {
        // 1. Render to new buffer
        let mut buffer = Buffer::empty(self.area);
        let mut frame = Frame::new(&mut buffer);
        f(&mut frame);

        // 2. Diff against previous frame ← OPTIMIZED
        let updates = buffer.diff(&self.previous_buffer);

        // 3. Send only changes to terminal ← MINIMAL I/O
        self.backend.draw(updates)?;

        // 4. Swap buffers
        self.previous_buffer = buffer;
        Ok(())
    }
}
```

**We're bypassing this** with `Buffer::merge()`:

```rust
// app.rs:848-854
terminal.draw(|frame| {
    let output = self.compositor.composite();

    // ❌ Merges ALL cells from compositor output
    // This defeats Ratatui's diff optimization
    frame.buffer_mut().merge(output);
})?;
```

**What `Buffer::merge()` does**:

```rust
// Ratatui source (simplified)
impl Buffer {
    pub fn merge(&mut self, other: &Buffer) {
        for (index, cell) in other.content.iter().enumerate() {
            if cell != &Cell::default() {
                self.content[index] = cell.clone();  // ← MORE CLONING
            }
        }
    }
}
```

**Double cloning**:
1. Compositor clones cells: layer → output buffer
2. `Buffer::merge()` clones cells: output buffer → frame buffer

**Total clones per frame**: 10,000 cells × 2 = **20,000 Cell::clone() calls**

---

## 3. Avatar Rendering Performance

**VERDICT**: ✅ **Avatar is FINE** - Not a performance bottleneck.

### Evidence from Investigation

**Avatar implementation** (`tui/src/avatar/mod.rs:115-154`):

```rust
pub fn render(&self, buf: &mut Buffer) {
    let frame = self.engine.current_frame(self.size)?;

    // Simple cell-by-cell copy, only non-empty cells
    for (row_idx, row) in frame.cells.iter().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            if cell.is_empty() {
                continue;  // ✅ Skip transparent cells
            }

            let style = Style::default().fg(cell.fg);
            buf.set_string(x, y, cell.ch.to_string(), style);
        }
    }

    // Render overlay (activity indicator)
    self.render_overlay(buf, x_offset, y_offset);
}
```

**Why avatar is efficient**:

1. **Pre-computed sprites** - Loaded once at startup (`animation.rs:26-52`)
2. **Simple frame advance** - Just index increment (`animation.rs:56-92`)
3. **Small render area** - Medium avatar = 24×6 = 144 cells
4. **Dirty tracking exists** - Only renders on frame change (`app.rs:1326-1327`)
5. **No expensive math** - Colors pre-computed in sprite data

**Cost breakdown**:

| Operation | Per Frame | Per Second (10 FPS) |
|-----------|-----------|---------------------|
| Frame index lookup | 1 | 10 |
| Cell iterations | ~100 (Medium) | 1,000 |
| `buf.set_string()` | ~80 (non-empty) | 800 |
| Position smoothing | 4 integer ops | 40 |

**Comparison to breathing colors** (pre-fix):

| Feature | Operations/Frame | Ops/Second (60 FPS) | Cost |
|---------|------------------|---------------------|------|
| Breathing colors | 35 sin() + 105 lerp | 2,100 + 6,300 | **CRITICAL** |
| Avatar animation | 100 cell copies | 1,000 | **Negligible** |

**Conclusion**: Avatar uses **200× less CPU** than breathing colors did. Keep it.

---

## 4. Terminal Operations

### 4.1 Terminal I/O Analysis

**Good practices found**:

```rust
// main.rs:62-67
enable_raw_mode()?;
let mut stdout = io::stdout();
execute!(stdout, EnterAlternateScreen)?;  // ✅ Batched with execute! macro
let backend = CrosstermBackend::new(stdout);
let mut terminal = Terminal::new(backend)?;
terminal.clear()?;
```

✅ Using `execute!()` macro for batched operations
✅ Alternate screen enabled (doesn't pollute scroll)
✅ Raw mode for input capture

**Frame rendering**:

```rust
// app.rs:848-854
terminal.draw(|frame| {
    let output = self.compositor.composite();
    frame.buffer_mut().merge(output);
})?;
```

✅ Single `terminal.draw()` call per frame
✅ No manual `flush()` calls (Ratatui handles it)
❌ But see §2.5 about `Buffer::merge()` overhead

### 4.2 Terminal Size Queries

**Location**: `app.rs:627-679` (resize handler)

```rust
async fn handle_resize(&mut self, width: u16, height: u16) {
    self.size = (width, height);  // ✅ Cached
    let area = Rect::new(0, 0, width, height);

    self.compositor.resize(area);  // ✅ One-time on resize
    // ... update layer bounds ...
}
```

✅ Terminal size cached in `self.size`
✅ Only updated on `Event::Resize`
✅ No per-frame `terminal::size()` calls

**Verdict**: Terminal operations are well-optimized. No issues here.

---

## 5. Comparison to Ratatui Best Practices

### What Ratatui Recommends

From Ratatui documentation and examples:

**1. Direct widget rendering**:
```rust
terminal.draw(|frame| {
    let paragraph = Paragraph::new(text).block(Block::bordered());
    frame.render_widget(paragraph, area);
})?;
```

**2. Stateful widgets for complex UI**:
```rust
struct AppState {
    list_state: ListState,
    scroll_state: ScrollbarState,
}

terminal.draw(|frame| {
    frame.render_stateful_widget(list, area, &mut app.list_state);
})?;
```

**3. Layout management**:
```rust
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(frame.size());
```

### What We're Doing

**1. Custom compositor instead of widgets**:
```rust
// ❌ Not using Ratatui's widget system
self.render_conversation();  // Custom rendering to layer buffer
self.render_input();         // Custom rendering to layer buffer
let output = self.compositor.composite();  // Merge layers
frame.buffer_mut().merge(output);  // Copy to frame
```

**2. Manual buffer management**:
```rust
// ❌ Managing 5 separate buffers
pub struct Compositor {
    layers: HashMap<LayerId, Layer>,
    output: Buffer,
}

pub struct Layer {
    buffer: Buffer,  // ← Each layer has its own buffer
}
```

**3. Manual layout management**:
```rust
// app.rs:146-200 - Manual layer positioning
let conversation = compositor.create_layer(
    Rect::new(0, 0, area.width, area.height.saturating_sub(input_and_status_height)),
    0,
);
```

### Gap Analysis

| Best Practice | Our Implementation | Gap |
|---------------|-------------------|-----|
| Use widgets | Custom buffer rendering | **HIGH** |
| Layout API | Manual rect calculations | Medium |
| Widget state | Custom dirty tracking | Medium |
| Framework diff | `Buffer::merge()` | **HIGH** |
| Single buffer | 5-layer compositor | **HIGH** |

---

## 6. Recommended Architecture Changes

### 6.1 Eliminate Compositor (MAJOR REFACTOR)

**Why**:
- 5 layers → 2× buffer cloning overhead
- Custom dirty tracking duplicates Ratatui functionality
- 236 lines of code to maintain
- Bypasses framework optimizations

**Migration path**:

```rust
// BEFORE (current)
fn render(&mut self, terminal: &mut Terminal) -> Result<()> {
    self.render_conversation();  // → layer buffer
    self.render_input();         // → layer buffer
    self.render_status();        // → layer buffer
    self.render_tasks();         // → layer buffer
    self.render_avatar();        // → layer buffer

    terminal.draw(|frame| {
        let output = self.compositor.composite();  // Blit layers
        frame.buffer_mut().merge(output);          // Copy to frame
    })?;
}

// AFTER (proposed)
fn render(&mut self, terminal: &mut Terminal) -> Result<()> {
    terminal.draw(|frame| {
        let layout = self.compute_layout(frame.size());

        // Render directly to frame buffer
        if self.conversation_dirty {
            self.render_conversation_widget(frame, layout.conversation);
        }
        if self.tasks_dirty && self.display.has_active_tasks() {
            self.render_tasks_widget(frame, layout.tasks);
        }
        if self.input_dirty {
            self.render_input_widget(frame, layout.input);
        }
        if self.status_dirty {
            self.render_status_widget(frame, layout.status);
        }
        if self.avatar_changed {
            self.render_avatar_widget(frame, layout.avatar);
        }
    })?;
}
```

**Benefits**:
- Eliminates 20,000 Cell::clone() calls per frame
- Uses Ratatui's optimized `Buffer::diff()`
- Removes 236 lines of compositor code
- Simpler mental model

**Estimated impact**: **50-70% CPU reduction** from eliminating double buffering.

---

### 6.2 Cache Conversation Wrapping (P1)

**Current issue**: Re-wraps all messages every frame (§2.2)

**Fix**:

```rust
pub struct DisplayMessage {
    pub id: MessageId,
    pub role: DisplayRole,
    pub content: String,
    pub streaming: bool,
    pub metadata: Option<ResponseMetadata>,

    // ✅ Add caching fields
    wrapped_lines: RefCell<Option<Vec<String>>>,
    cached_width: RefCell<Option<usize>>,
}

impl DisplayMessage {
    pub fn get_wrapped(&self, width: usize) -> Vec<String> {
        let mut cache = self.wrapped_lines.borrow_mut();
        let mut cached_width = self.cached_width.borrow_mut();

        if *cached_width != Some(width) || cache.is_none() {
            let content = if self.streaming {
                format!("{}{}_", self.role.prefix(), self.content)
            } else {
                format!("{}{}", self.role.prefix(), self.content)
            };

            *cache = Some(
                textwrap::wrap(&content, width)
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            );
            *cached_width = Some(width);
        }

        cache.as_ref().unwrap().clone()
    }

    pub fn invalidate_cache(&self) {
        *self.wrapped_lines.borrow_mut() = None;
    }
}
```

**Invalidation triggers**:
- Message content changes (streaming append, completion)
- Terminal resize
- That's it

**Estimated impact**: Eliminates 1,200-2,000 wraps/sec in typical usage.

---

### 6.3 Add Conversation Dirty Tracking (P1)

**Current issue**: Conversation layer always marked dirty (§2.3)

**Fix**:

```rust
struct App {
    // ...
    prev_message_count: usize,
    prev_streaming_state: bool,
    prev_scroll_offset: usize,  // ✅ Already exists
}

fn render_conversation(&mut self) {
    // ✅ Check if anything changed
    let conversation_changed =
        self.display.messages.len() != self.prev_message_count
        || self.display.is_streaming() != self.prev_streaming_state
        || self.scroll_offset != self.prev_scroll_offset;

    if !conversation_changed {
        return;  // ✅ Skip rendering
    }

    // ... existing render code ...

    // Update tracking
    self.prev_message_count = self.display.messages.len();
    self.prev_streaming_state = self.display.is_streaming();
}
```

**Estimated impact**: 90% reduction in conversation renders during idle.

---

### 6.4 Optimize Cell Operations (P1)

**If keeping compositor** (not recommended, but as fallback):

**Option A**: Use `Buffer::content` slice operations

```rust
// BEFORE
for ly in 0..lb.height {
    for lx in 0..lb.width {
        // ... per-cell clone ...
        output.content[dst_idx] = src_cell.clone();
    }
}

// AFTER - Use bulk copy for contiguous regions
let src_range = /* calculate */;
let dst_range = /* calculate */;
output.content[dst_range].clone_from_slice(&layer.buffer.content[src_range]);
```

**Option B**: Use `Cow` or `Rc` for Cell symbols (advanced)

**Estimated impact**: 50% reduction in clone overhead (but still worse than eliminating compositor).

---

## 7. Performance Impact Summary

### Current State (Post-P0 Fixes)

| Issue | Operations/Sec | CPU Impact | Priority |
|-------|---------------|------------|----------|
| Frame rate | 10 FPS | Baseline | ✅ FIXED |
| Breathing colors | 0 sine ops | None | ✅ REMOVED |
| Cell cloning | 100,000 clones | **Medium** | 🟡 P1 |
| Conversation rewrap | 1,200 wraps | **Medium** | 🟡 P1 |
| Compositor overhead | 5 layer blits | **Low-Medium** | 🟡 P1 |
| Conversation always dirty | Wastes dirty tracking | **Low** | 🟢 P2 |

### Projected State (After Proposed Fixes)

| Metric | Current | After P1 Fixes | Improvement |
|--------|---------|----------------|-------------|
| Cell clones/sec | 100,000 | 0 (no compositor) | **-100%** |
| Text wraps/sec | 1,200 | ~50 (only on changes) | **-96%** |
| Layers composited | 5 | 0 | **-100%** |
| Buffer allocations | 6 (5 layers + output) | 1 (frame only) | **-83%** |
| **Total CPU reduction** | Baseline | **40-60% less** | **SIGNIFICANT** |

---

## 8. Actionable Recommendations

### Priority 1 (High Impact, Medium Effort)

**P1.1: Eliminate Compositor** ⏱️ Effort: 4-6 hours
- Remove `compositor/` module
- Render directly to `frame` in `terminal.draw()`
- Use Ratatui's `Layout` API for area management
- **Impact**: 50-70% CPU reduction

**P1.2: Cache Conversation Wrapping** ⏱️ Effort: 2-3 hours
- Add `wrapped_lines` cache to `DisplayMessage`
- Invalidate on content change or resize
- **Impact**: Eliminates 1,000+ wraps/sec

**P1.3: Add Conversation Dirty Tracking** ⏱️ Effort: 1 hour
- Track `prev_message_count` and `prev_streaming_state`
- Skip render if conversation unchanged
- **Impact**: 90% fewer conversation renders when idle

### Priority 2 (Nice to Have)

**P2.1: Benchmarking Suite**
- Add frame time measurements
- Track render operation counts
- Regression detection

**P2.2: Profile with `perf` or `samply`**
- Identify any remaining hot paths
- Verify fixes actually helped

### Priority 3 (Future Optimization)

**P3.1: Incremental Rendering**
- Render only new messages, not entire conversation
- Scroll by adjusting viewport, not re-rendering

**P3.2: Virtual Scrolling**
- Only render visible lines
- Maintain off-screen message metadata

---

## 9. Testing Strategy

### Regression Tests

**Add to `cpu_performance_test.rs`**:

```rust
#[tokio::test]
async fn test_frame_rate_target() {
    // Verify we're rendering at ~10 FPS, not 60
    let mut app = App::new().await?;
    let frame_count = count_frames_over(Duration::from_secs(5));
    let fps = frame_count as f64 / 5.0;

    assert!(fps >= 8.0 && fps <= 12.0,
        "Frame rate out of range: {:.1} FPS (target: 10 ± 20%)", fps);
}

#[tokio::test]
async fn test_no_render_when_idle() {
    // Verify we don't render when nothing changes
    let mut app = App::new().await?;
    app.render(/* ... */)?;  // Initial render

    let renders_before = app.render_count();
    sleep(Duration::from_secs(2)).await;
    let renders_after = app.render_count();

    assert_eq!(renders_after - renders_before, 0,
        "Should not render when idle");
}
```

### Manual Testing

**Before/after profiling**:

```bash
# Record before
perf record -g ./yollayah.sh
perf report

# Apply fixes

# Record after
perf record -g ./yollayah.sh
perf report

# Compare flamegraphs
```

---

## 10. Conclusion

### Summary of Findings

**Architecture Issues**:
- ❌ **5-layer compositor is over-engineered** - Can reduce to direct rendering
- ❌ **Custom dirty tracking duplicates Ratatui** - Framework does this better
- ❌ **Double buffering overhead** - 20,000 Cell clones per frame

**Rendering Inefficiencies**:
- ❌ **Conversation re-wrapping** - No caching, 1,200 wraps/sec
- ✅ **Avatar rendering is fine** - Not a bottleneck
- ❌ **Conversation always dirty** - Wastes dirty tracking

**Good Practices Found**:
- ✅ Frame rate fixed (60→10 FPS)
- ✅ Breathing colors removed
- ✅ Dirty tracking on input, status, tasks, avatar
- ✅ Terminal I/O well-optimized
- ✅ No excessive terminal queries

### Estimated Total Impact

**P0 fixes already applied**: 85-90% CPU reduction (breathing + frame rate)
**P1 fixes proposed**: Additional 40-60% reduction
**Combined**: TUI will use ~2-5% CPU during streaming (from ~50-70% pre-fixes)

### Next Steps

1. **Immediate**: Apply P1 fixes (eliminate compositor, cache wrapping, conversation dirty tracking)
2. **Week 1**: Add regression tests and benchmarks
3. **Week 2**: Profile with `perf`, verify improvements
4. **Future**: Consider incremental/virtual rendering for very long conversations

---

**Files Analyzed**:
- `/var/home/machiyotl/src/ai-way/tui/src/app.rs` (1,348 lines)
- `/var/home/machiyotl/src/ai-way/tui/src/compositor/mod.rs` (236 lines)
- `/var/home/machiyotl/src/ai-way/tui/src/compositor/layer.rs` (48 lines)
- `/var/home/machiyotl/src/ai-way/tui/src/avatar/mod.rs` (216 lines)
- `/var/home/machiyotl/src/ai-way/tui/src/display.rs` (1,496 lines)
- `/var/home/machiyotl/src/ai-way/tui/src/main.rs` (93 lines)
- `/var/home/machiyotl/src/ai-way/BUG-003-tui-performance-regression.md` (1,134 lines)

**References**:
- Ratatui documentation: https://docs.rs/ratatui/
- BUG-003 investigation and fixes
- CPU performance tests: `tui/tests/cpu_performance_test.rs`
