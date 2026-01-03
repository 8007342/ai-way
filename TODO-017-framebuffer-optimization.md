# TODO-017: Framebuffer Optimization Sprints

**Created**: 2026-01-03
**Updated**: 2026-01-03
**Priority**: P1 - High Impact Performance Work
**Status**: 🟢 IN PROGRESS - Sprints 1 & 2 Complete
**Related**: PERFORMANCE-AUDIT-FRAMEBUFFER.md

---

## Overview

Optimize TUI rendering pipeline to reduce CPU overhead by 40-60%. Based on comprehensive performance audit findings.

**Current State**: 5-layer compositor with full re-composition and text re-wrapping every frame
**Target State**: Efficient cached rendering with dirty tracking

---

## Sprint 1: Text Wrapping Cache ✅ COMPLETED

**Impact**: Saves 1200+ `textwrap::wrap()` calls per second
**Effort**: Medium (2-4 hours)
**Files**: `tui/src/display.rs`, `tui/src/app.rs`
**Status**: ✅ Implemented 2026-01-03

### Problem

`tui/src/app.rs:896` calls `textwrap::wrap()` for EVERY message EVERY frame:

```rust
// ❌ CURRENT: Re-wraps all messages every frame
for msg in &self.display.messages {
    let wrapped = textwrap::wrap(&content, width); // ← EXPENSIVE!
    for line in wrapped.iter() {
        all_lines.push(...);
    }
}
```

**Measurements**:
- 20 messages × 60 wraps/message = 1200 wraps/sec at 10 FPS
- Each wrap involves Unicode analysis, word breaking, allocation
- Width rarely changes (only on terminal resize)
- Content rarely changes (only on new messages)

### Solution

Add caching to `DisplayMessage`:

```rust
// display.rs
use std::cell::RefCell;

pub struct DisplayMessage {
    pub id: MessageId,
    pub role: DisplayRole,
    pub content: String,
    pub streaming: bool,
    pub metadata: Option<ResponseMetadata>,

    // ✅ NEW: Wrapping cache
    wrapped_cache: RefCell<Option<WrappedCache>>,
}

struct WrappedCache {
    width: usize,
    content_hash: u64,  // Hash of content for invalidation
    lines: Vec<String>,
}

impl DisplayMessage {
    pub fn get_wrapped(&self, width: usize) -> Vec<String> {
        let mut cache = self.wrapped_cache.borrow_mut();

        // Calculate content hash
        let content_hash = calculate_hash(&self.content);

        // Check if cache is valid
        if let Some(cached) = cache.as_ref() {
            if cached.width == width && cached.content_hash == content_hash {
                return cached.lines.clone();  // ✅ Cache hit!
            }
        }

        // Cache miss - re-wrap and store
        let wrapped = textwrap::wrap(&self.content, width)
            .iter()
            .map(|s| s.to_string())
            .collect();

        *cache = Some(WrappedCache {
            width,
            content_hash,
            lines: wrapped.clone(),
        });

        wrapped
    }

    pub fn invalidate_wrap_cache(&self) {
        *self.wrapped_cache.borrow_mut() = None;
    }
}
```

### Implementation Steps

1. Add `wrapped_cache: RefCell<Option<WrappedCache>>` to `DisplayMessage`
2. Add `get_wrapped(width)` method
3. Update `render_conversation()` to use `msg.get_wrapped(width)`
4. Invalidate cache when:
   - Content changes (`update_content()`)
   - Streaming updates
   - Terminal resizes

### Testing

```rust
#[test]
fn test_wrapping_cache() {
    let msg = DisplayMessage::new(...);

    let wrapped1 = msg.get_wrapped(80);
    let wrapped2 = msg.get_wrapped(80);

    // Second call should be cached (same pointer or hash match)
    assert_eq!(wrapped1, wrapped2);

    // Different width should re-wrap
    let wrapped3 = msg.get_wrapped(100);
    assert_ne!(wrapped1.len(), wrapped3.len());
}
```

### Expected Impact

- **Cache hit rate**: ~95% (width constant, most messages static)
- **CPU reduction**: ~15-20% for conversation rendering
- **Allocations**: -1000+ strings/sec

---

## Sprint 2: Conversation Dirty Tracking ✅ COMPLETED

**Impact**: 90% fewer re-renders when conversation unchanged
**Effort**: Small (1-2 hours)
**Files**: `tui/src/app.rs`
**Status**: ✅ Implemented 2026-01-03

### Problem

`render_conversation()` rebuilds `all_lines` from scratch every frame, even when nothing changed.

### Solution

Track when conversation actually changes:

```rust
// app.rs
pub struct App {
    // ... existing fields ...

    // ✅ NEW: Dirty tracking
    conversation_dirty: bool,
    last_render_width: usize,
    cached_lines: Vec<LineMeta>,
}

impl App {
    fn render_conversation(&mut self) {
        let width = self.size.0.saturating_sub(2) as usize;

        // ✅ GOOD: Check if rebuild needed
        if !self.conversation_dirty && self.last_render_width == width {
            // Use cached lines, skip expensive rebuild
            self.render_cached_lines();
            return;
        }

        // Rebuild needed
        let mut all_lines = Vec::new();
        for msg in &self.display.messages {
            let wrapped = msg.get_wrapped(width); // Uses cache from Sprint 1
            // ... build lines ...
        }

        // Cache the result
        self.cached_lines = all_lines;
        self.last_render_width = width;
        self.conversation_dirty = false;

        self.render_cached_lines();
    }

    fn mark_conversation_dirty(&mut self) {
        self.conversation_dirty = true;
    }
}
```

### Mark Dirty When

- New message arrives
- Streaming token updates
- Message deleted
- Scroll offset changes (NO - scrolling uses same lines)
- Terminal resizes (YES - width changes)

### Expected Impact

- **Idle frames**: 90% reduction in CPU (nothing to rebuild)
- **Active streaming**: Still rebuilds only streaming message
- **Combined with Sprint 1**: 30-40% total CPU reduction

---

## Sprint 3: Optimize or Eliminate Compositor

**Impact**: 50-70% rendering CPU reduction
**Effort**: High (4-8 hours) - Architectural change
**Files**: `tui/src/compositor/*`, `tui/src/app.rs`

### Problem

The 5-layer compositor causes massive overhead:

```rust
// compositor/mod.rs:210
output.content[dst_idx] = src_cell.clone(); // ← 100,000 clones/sec!
```

**Why is this expensive?**
- 200×50 terminal = 10,000 cells
- 5 layers × 10,000 cells = 50,000 potential clones
- At 10 FPS = 500,000 clones/sec (actual: 100,000 due to partial visibility)
- Each clone is a `Cell` struct copy (small but adds up)

**Why does it re-composite ALL layers when ANY is dirty?**
- Line 168-174: When `dirty_layers` is non-empty, clears output and re-blits all visible layers
- Required to maintain correct z-ordering
- Comment acknowledges this: "We render ALL layers (not just dirty ones)"

### Option A: Partial Compositor Optimization (Medium effort)

**Keep compositor, optimize re-composition**:

```rust
// Instead of re-blitting ALL layers, track dirty REGIONS
pub struct Compositor {
    // ... existing fields ...
    dirty_regions: Vec<Rect>,  // ✅ NEW
}

impl Compositor {
    pub fn composite(&mut self) -> &Buffer {
        if self.dirty_regions.is_empty() {
            return &self.output;  // No changes
        }

        // Only blit layers in dirty regions
        for region in &self.dirty_regions {
            self.blit_region_from_layers(region);
        }

        self.dirty_regions.clear();
        &self.output
    }

    fn blit_region_from_layers(&mut self, region: &Rect) {
        // Clear region
        for y in region.y..(region.y + region.height) {
            for x in region.x..(region.x + region.width) {
                let idx = self.output.index_of(x, y);
                self.output.content[idx].reset();
            }
        }

        // Blit only layers that intersect this region, in z-order
        for &id in &self.render_order {
            if let Some(layer) = self.layers.get(&id) {
                if layer.visible && layer.intersects(region) {
                    self.blit_layer_region(&mut self.output, layer, region);
                }
            }
        }
    }
}
```

**Impact**: 40-60% CPU reduction, keeps architectural flexibility

### Option B: Eliminate Compositor Entirely (High effort, highest impact)

**Use Ratatui's Frame directly**:

Ratatui already has:
- Built-in dirty tracking via `Buffer::diff()`
- Efficient rendering of widgets
- Terminal-optimized double buffering

```rust
// app.rs - BEFORE (with compositor)
fn draw(&mut self, frame: &mut Frame) {
    // Render to layer buffers
    self.render_conversation();
    self.render_input();
    self.render_status();
    self.render_avatar();

    // Composite all layers
    let output = self.compositor.composite();

    // Blit to terminal
    frame.render_widget(BufferWidget(output), frame.size());
}

// app.rs - AFTER (direct rendering)
fn draw(&mut self, frame: &mut Frame) {
    let area = frame.size();

    // Render widgets directly in z-order
    // Ratatui handles dirty tracking and optimization internally

    // Background (z=0)
    self.render_conversation_widget(frame, conversation_area);

    // Foreground (z=10)
    self.render_input_widget(frame, input_area);
    self.render_status_widget(frame, status_area);
    self.render_tasks_widget(frame, tasks_area);

    // Top (z=50)
    self.render_avatar_widget(frame, avatar_bounds);

    // Ratatui's frame.render_widget() handles:
    // - Dirty tracking via Buffer::diff()
    // - Only sends changed cells to terminal
    // - Zero copies for unchanged regions
}
```

**Advantages**:
- Eliminates 100,000+ Cell clones
- Uses Ratatui's optimized dirty tracking
- Simpler architecture (less code to maintain)
- Natural widget-based rendering

**Disadvantages**:
- Requires refactoring all render functions
- Loses explicit layer management (rely on render order instead)
- More complex for advanced effects (but we don't use them)

**Impact**: 50-70% CPU reduction, cleaner architecture

### Recommendation

**Start with Option A** (partial optimization) if time-constrained.
**Migrate to Option B** (eliminate compositor) for long-term performance and maintainability.

---

## Sprint 4: Additional Optimizations (Optional)

### 4.1: Reduce String Allocations in render_conversation

**Current** (app.rs:899):
```rust
all_lines.push(LineMeta {
    text: line.to_string(),  // ← Allocation!
    // ...
});
```

**Optimized**:
```rust
// Use Cow<'a, str> or &str where possible
struct LineMeta<'a> {
    text: Cow<'a, str>,  // ✅ Zero-copy when possible
    // ...
}
```

**Impact**: -500 allocations/sec

### 4.2: Optimize Avatar Rendering

**Already Fixed in Previous Commit**: Eliminated 1000+ String allocations in avatar cell rendering

### 4.3: Scrollbar Caching

Similar to conversation cache, cache scrollbar rendering when scroll position unchanged.

---

## Performance Targets

**Before Optimizations**:
- Idle CPU: ~2-5% (even with dirty tracking)
- Active streaming CPU: ~10-15%
- Allocations/sec: ~3000

**After Sprint 1 + 2**:
- Idle CPU: ~0.5-1%
- Active streaming CPU: ~5-8%
- Allocations/sec: ~1500

**After Sprint 3 (Eliminate Compositor)**:
- Idle CPU: ~0.1-0.3%
- Active streaming CPU: ~3-5%
- Allocations/sec: ~500

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_wrapping_cache_invalidation() { ... }

#[test]
fn test_conversation_dirty_tracking() { ... }

#[test]
fn test_partial_compositor_update() { ... }
```

### Performance Regression Tests

Add to `tests/architectural-enforcement/`:

```rust
#[test]
fn test_no_unwrap_in_hot_paths() {
    // Ensure render paths don't have `textwrap::wrap` outside cache
}

#[test]
fn test_dirty_tracking_prevents_rebuilds() {
    // Verify conversation not rebuilt when unchanged
}
```

### Manual Testing

1. Profile idle CPU: `perf record ./yollayah.sh`
2. Measure streaming latency: Time from token arrival → screen render
3. Check memory usage: `valgrind --tool=massif`

---

## Implementation Order

1. **Sprint 1** (Text Wrapping Cache) - Highest ROI, isolated change
2. **Sprint 2** (Dirty Tracking) - Builds on Sprint 1, additive
3. **Sprint 3 Option A** (Partial Compositor) - If needed, medium effort
4. **Sprint 3 Option B** (Eliminate Compositor) - Best long-term, higher effort

---

## Acceptance Criteria

- [ ] Text wrapping cache implemented with >90% hit rate
- [ ] Conversation dirty tracking prevents unnecessary rebuilds
- [ ] Compositor optimized or eliminated (choice of Option A/B)
- [ ] Idle CPU < 0.5%
- [ ] Active streaming CPU < 8%
- [ ] All tests pass (unit + integration + smoke)
- [ ] Performance regression tests added
- [ ] Code committed with detailed metrics

---

## Related Documents

- **Audit**: PERFORMANCE-AUDIT-FRAMEBUFFER.md
- **Principles**: reference/PRINCIPLE-efficiency.md (Law 2: Lazy Init, Aggressive Caching)
- **Anti-patterns**: reference/FORBIDDEN-inefficient-calculations.md

---

## Notes

**Why not do all sprints now?**
These are architectural changes that require careful testing. Implementing incrementally allows:
- Validation of each optimization in isolation
- Easier rollback if issues arise
- Clear performance attribution per change

**Priority**: Complete Sprint 1+2 first (quick wins), then evaluate if Sprint 3 is needed based on profiling results.
