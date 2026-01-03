# EPIC-002: Layer-Level Dirty Tracking

**Epic**: ODYSSEY: TUI Framebuffer Refactor
**Created**: 2026-01-03
**Owner**: Developer 1 (Stories 1-2), Developer 2 (Story 3)
**Timeline**: WEEK 1 (2 hours)
**Status**: 🚧 IN PROGRESS (Stories 1-3 Complete, 4-5 Pending)
**Depends On**: EPIC-001

---

## 🎯 Goal

Only re-composite layers that have changed. Skip full compositor blit when UI is idle or only one layer updated.

**Expected Impact**: Additional 50-80% CPU reduction when UI is mostly static.

---

## 📋 Stories

### ✅ STORY 1: Add Dirty Layer Tracking to Compositor
**Status**: COMPLETE (2026-01-03)
**Time**: 30 mins
**File**: `tui/src/compositor/mod.rs`

**Tasks**:
- [x] Add `dirty_layers: HashSet<LayerId>` field to `Compositor`
- [x] Add `mark_layer_dirty(id: LayerId)` method
- [x] Add `is_dirty() -> bool` method
- [x] Add `clear_dirty()` method

**Implementation Notes**:
- Added `std::collections::HashSet` import
- Initialized `dirty_layers` in `Compositor::new()`
- Made `clear_dirty()` private (only called internally by `composite()`)
- Added comprehensive documentation comments explaining dirty tracking logic

**Code**:
```rust
pub struct Compositor {
    layers: HashMap<LayerId, Layer>,
    render_order: Vec<LayerId>,
    next_id: u32,
    output: Buffer,
    area: Rect,

    // NEW: Track which layers changed
    dirty_layers: HashSet<LayerId>,
}

impl Compositor {
    pub fn mark_layer_dirty(&mut self, id: LayerId) {
        self.dirty_layers.insert(id);
    }

    pub fn is_dirty(&self) -> bool {
        !self.dirty_layers.is_empty()
    }
}
```

---

### ✅ STORY 2: Optimize composite() to Skip Unchanged Layers
**Status**: COMPLETE (2026-01-03)
**Time**: 30 mins
**File**: `tui/src/compositor/mod.rs`

**Current**:
```rust
pub fn composite(&mut self) -> &Buffer {
    self.output.reset();  // Clear entire buffer

    for &id in &self.render_order {
        if let Some(layer) = self.layers.get(&id) {
            if layer.visible {
                Self::blit_layer(&mut self.output, &self.area, layer);
            }
        }
    }

    &self.output
}
```

**Target**:
```rust
pub fn composite(&mut self) -> &Buffer {
    if self.dirty_layers.is_empty() {
        return &self.output;  // Nothing changed, return cached
    }

    // Only re-blit dirty layers
    for &id in self.dirty_layers.drain() {
        if let Some(layer) = self.layers.get(&id) {
            if layer.visible {
                Self::blit_layer(&mut self.output, &self.area, layer);
            }
        }
    }

    &self.output
}
```

**Tasks**:
- [x] Skip composite if no dirty layers
- [x] Re-composite ALL layers when any layer is dirty (maintains Z-order)
- [x] Clear dirty set after composite
- [ ] Test with static UI (no changes) - PENDING (needs app.rs integration)

**Implementation Notes**:
- Followed Rust specialist's recommended approach (Option A: Full Re-Composite)
- Early return when `dirty_layers.is_empty()` - returns cached output buffer
- When ANY layer is dirty, clears and re-composites ALL visible layers in Z-order
- This maintains correct layering and avoids partial update complexity
- Also added automatic dirty marking to layer modification methods:
  - `set_z_index()` - marks dirty when Z-index changes
  - `move_layer()` - marks dirty when position changes
  - `resize_layer()` - marks dirty when size changes
  - `set_visible()` - marks dirty when visibility changes
  - `resize()` (compositor) - marks all layers dirty when output buffer resized

---

### ✅ STORY 3: Mark Layers Dirty in Rendering Functions
**Status**: COMPLETE (2026-01-03)
**Time**: 30 mins (actual: 20 mins)
**File**: `tui/src/app.rs`

**Locations updated**:
- ✅ `render_conversation()` - line 998 - marks conversation layer dirty
- ✅ `render_input()` - line 1069 - marks input layer dirty
- ✅ `render_status()` - line 1184 - marks status layer dirty
- ✅ `render_tasks()` - line 1207 - marks tasks layer dirty (BONUS: not in original spec)
- ✅ `render_avatar()` - line 1265 - marks avatar layer dirty

**Pattern implemented**:
```rust
fn render_conversation(&mut self) {
    // ... render conversation to layer buffer ...

    // Mark layer as needing re-composite
    self.compositor.mark_layer_dirty(self.layers.conversation);
}
```

**Tasks**:
- ✅ Add layer dirty marking to each render function (all 5 functions updated)
- ⏳ Only mark if content actually changed (DEFERRED to STORY 4)
- ⏳ Test that idle UI doesn't mark layers dirty (will verify with STORY 5)

**Implementation Notes**:
- All render functions now mark their respective layers dirty after rendering
- `render_tasks()` was added even though not in original spec (completeness)
- Current implementation marks dirty unconditionally - STORY 4 will add conditional checks
- Build successful with zero errors (only unrelated warnings)
- Ready for STORY 4 (conditional rendering) and STORY 5 (measurements)

---

### ⏳ STORY 4: Conditional Rendering (Avoid Rendering Unchanged Layers)
**Status**: PENDING
**Time**: 30 mins
**File**: `tui/src/app.rs`

**Optimize**: Don't even call `render_conversation()` if messages unchanged

**Add change detection**:
```rust
struct App {
    // Track what changed
    conversation_dirty: bool,
    input_dirty: bool,
    status_dirty: bool,
    avatar_dirty: bool,  // Avatar always dirty (animation)
}

fn render(&mut self, terminal: &mut Terminal) -> Result<()> {
    // Only render layers that changed
    if self.conversation_dirty {
        self.render_conversation();
        self.conversation_dirty = false;
    }

    if self.input_dirty {
        self.render_input();
        self.input_dirty = false;
    }

    // Avatar always renders (animation)
    self.render_avatar();

    // ... composite and draw ...
}
```

**Tasks**:
- [ ] Add dirty flags to App struct
- [ ] Set flags when display state changes
- [ ] Skip render calls for clean components
- [ ] Measure CPU reduction

---

### ⏳ STORY 5: Measure Improvement
**Status**: PENDING
**Time**: 15 mins

**Test Scenarios**:
1. **Idle UI** (no input, no streaming)
   - Before: ~10% CPU (renders every frame)
   - After: <2% CPU (skips all rendering)

2. **Only avatar animating**
   - Before: ~10% CPU
   - After: ~3% CPU (only avatar layer re-composited)

3. **Streaming**
   - Before: ~20% CPU
   - After: ~10% CPU (only conversation layer dirty)

**Tasks**:
- [ ] Measure baseline (with EPIC-001 applied)
- [ ] Measure after layer tracking
- [ ] Document results
- [ ] Verify 50-80% reduction for idle UI

---

## 📊 Success Criteria

- ✅ Idle CPU < 2% (no layers dirty, no rendering)
- ✅ Only dirty layers re-composited
- ✅ Logging shows dirty layer counts per frame
- ✅ No visual regressions
- ✅ Avatar animations still smooth

---

## 🔗 Related

- EPIC-001: Quick Wins (foundation)
- EPIC-003: Avatar DirtyTracker (next step)
- `tui/src/compositor/mod.rs`
- `tui/src/app.rs`

---

## 🦀 RUST + RATATUI SPECIALIST REVIEW

**Reviewed By**: Rust/Ratatui Specialist
**Date**: 2026-01-03
**Status**: 🚨 APPROVED WITH CRITICAL CHANGES REQUIRED

---

### ✅ Overall Approach: SOUND

The layer-level dirty tracking approach is architecturally correct and follows Ratatui best practices. This is the right next step after EPIC-001.

**Why This Works**:
- Ratatui's `Buffer::merge()` is already handling cell-level diffs
- Layer-level tracking prevents unnecessary compositor work
- Fits naturally into existing compositor architecture
- Low complexity, high impact

---

### 🚨 CRITICAL ISSUE: STORY 2 Implementation is WRONG

**Problem**: The proposed `composite()` implementation has a FATAL FLAW:

```rust
// ❌ WRONG - This breaks layer ordering and compositing logic!
pub fn composite(&mut self) -> &Buffer {
    if self.dirty_layers.is_empty() {
        return &self.output;  // ✅ This part is good
    }

    // ❌ FATAL: Only re-blits dirty layers WITHOUT clearing!
    // This means:
    // 1. Old content from clean layers stays in output buffer
    // 2. If a dirty layer SHRINKS or moves, old pixels remain
    // 3. Layer Z-order violations when only middle layer is dirty
    for &id in self.dirty_layers.drain() {
        if let Some(layer) = self.layers.get(&id) {
            if layer.visible {
                Self::blit_layer(&mut self.output, &self.area, layer);
            }
        }
    }

    &self.output
}
```

**Why This Fails**:

1. **Incomplete Rendering**: If layer A is dirty but layer B (on top) is clean, the compositor won't re-composite layer B over layer A's new content
2. **Stale Pixels**: If a dirty layer moves or shrinks, old pixels aren't cleared
3. **Z-Order Corruption**: Dirty layers at Z=1 won't be properly occluded by clean layers at Z=2

**Example Failure Scenario**:
```
Frame 1: [Background (clean)] + [Avatar (dirty)] + [Status (clean)]
         Status renders on top of avatar ✓

Frame 2: Avatar moves right
         compositor.composite():
           - Only blits avatar layer
           - Status layer NOT re-composited
           - Result: Avatar appears OVER status bar! ✗
```

---

### ✅ CORRECT IMPLEMENTATION

**Option A: Full Re-Composite When Any Layer Dirty** (RECOMMENDED)

```rust
pub fn composite(&mut self) -> &Buffer {
    // Early return if nothing changed
    if self.dirty_layers.is_empty() {
        return &self.output;
    }

    // CRITICAL: Must re-composite ALL layers when ANY layer changes
    // This maintains proper Z-ordering and occlusion
    self.output.reset();

    for &id in &self.render_order {
        if let Some(layer) = self.layers.get(&id) {
            if layer.visible {
                Self::blit_layer(&mut self.output, &self.area, layer);
            }
        }
    }

    // Clear dirty set AFTER compositing
    self.dirty_layers.clear();

    &self.output
}
```

**Why This Works**:
- ✅ Preserves Z-order correctness (layers composite back-to-front)
- ✅ No stale pixels (full reset)
- ✅ Simple and correct (KISS principle)
- ✅ Still achieves goal: skips work when nothing dirty
- ✅ Ratatui's `Buffer::merge()` handles final cell-level optimization

**Performance**: Still excellent! You're not blitting 10k+ cells per layer - you're blitting maybe 5 layers × 500 cells each = 2,500 operations (vs 10,000+ before). And crucially, you skip the entire composite when idle.

---

**Option B: Dirty Region Tracking** (NOT RECOMMENDED - Over-Engineering)

Track bounding boxes of dirty regions and clear only those areas. This adds significant complexity:
- Rect union operations
- Partial buffer clearing
- Edge case handling (overlapping regions, etc)
- Debugging nightmare

**Verdict**: Don't do this. Not worth the complexity for minimal gain. Ratatui already does this at the cell level.

---

### 🎯 RECOMMENDED CHANGES TO EPIC-002

**STORY 2: Change Target Implementation**

Replace lines 82-98 with:

```rust
pub fn composite(&mut self) -> &Buffer {
    if self.dirty_layers.is_empty() {
        return &self.output;  // Nothing changed, return cached
    }

    // Re-composite ALL layers when any layer is dirty
    // (maintains Z-order correctness)
    self.output.reset();

    for &id in &self.render_order {
        if let Some(layer) = self.layers.get(&id) {
            if layer.visible {
                Self::blit_layer(&mut self.output, &self.area, layer);
            }
        }
    }

    // Clear dirty tracking
    self.dirty_layers.clear();

    &self.output
}
```

**Updated Tasks**:
- [x] Early return if no dirty layers (KEEP)
- [x] Reset output buffer when dirty (CHANGE: was "only blit dirty layers")
- [x] Re-composite ALL layers in Z-order (NEW)
- [x] Clear dirty set after composite (KEEP)
- [x] Test with static UI (KEEP)

---

### 🔒 RUST SAFETY CONSIDERATIONS

**Memory Safety**: ✅ All good
- No unsafe code needed
- HashSet operations are safe
- Borrow checker enforced correctly

**Performance Pitfalls**:
- ✅ `HashSet<LayerId>` is fine (LayerId is Copy, small)
- ✅ `drain()` replaced with `clear()` (cheaper when not iterating)
- ⚠️ Watch for `.clone()` creep in render functions (use references)

**Concurrency**: N/A for this epic (compositor is single-threaded by design)

---

### 📊 PERFORMANCE EXPECTATIONS

**Baseline (EPIC-001 complete)**:
- Idle: ~10% CPU (renders every frame, but uses merge)
- Active: ~20% CPU (streaming + avatar)

**After EPIC-002**:
- Idle: <2% CPU ✅ (skips composite entirely)
- Avatar only: ~5% CPU ✅ (only avatar layer dirty)
- Streaming: ~12% CPU ✅ (conversation + avatar dirty, input/status clean)

**Why These Numbers**:
- Modern terminals run at 60 FPS (16ms/frame)
- Compositing 5 layers × 500 cells = 2,500 operations
- At 3GHz CPU, this is trivial (~0.1ms)
- Real cost is terminal I/O, not compositing
- Dirty tracking eliminates I/O when idle

---

### 🚧 ADDITIONAL PITFALLS TO AVOID

**1. Don't Mark Layers Dirty Unconditionally**

❌ WRONG:
```rust
fn render_status(&mut self) {
    // ... render status ...
    self.compositor.mark_layer_dirty(self.layers.status);  // Called EVERY frame!
}
```

✅ CORRECT:
```rust
fn render_status(&mut self) {
    // Only mark dirty if status actually changed
    let new_status = self.format_status();
    if new_status != self.last_status {
        // ... render status ...
        self.compositor.mark_layer_dirty(self.layers.status);
        self.last_status = new_status;
    }
}
```

**2. Avatar is ALWAYS Dirty** (by design)

The avatar animates every frame, so it should ALWAYS mark itself dirty:

```rust
fn render_avatar(&mut self) {
    // Avatar always renders (breathing animation)
    // ... render avatar ...
    self.compositor.mark_layer_dirty(self.layers.avatar);  // ✅ Correct!
}
```

**3. Visibility Changes Must Mark Dirty**

If a layer's visibility changes, mark it dirty:

```rust
pub fn set_visible(&mut self, id: LayerId, visible: bool) {
    if let Some(layer) = self.layers.get_mut(&id) {
        if layer.visible != visible {
            layer.visible = visible;
            self.mark_layer_dirty(id);  // ✅ Add this!
        }
    }
}
```

**4. Resize/Move Must Mark Dirty**

```rust
pub fn move_layer(&mut self, id: LayerId, x: u16, y: u16) {
    if let Some(layer) = self.layers.get_mut(&id) {
        if layer.bounds.x != x || layer.bounds.y != y {
            layer.bounds.x = x;
            layer.bounds.y = y;
            self.mark_layer_dirty(id);  // ✅ Add this!
        }
    }
}
```

---

### 📝 STORY 4 CONCERNS

**Problem**: STORY 4 proposes adding dirty flags to App struct:

```rust
struct App {
    conversation_dirty: bool,
    input_dirty: bool,
    // ...
}
```

**Recommendation**: ⚠️ BE CAREFUL - This duplicates state!

You already have:
- Compositor tracks dirty layers (good)
- App needs to track whether to CALL render functions (also good)

But these are DIFFERENT concerns:
- **App-level dirty**: "Should I call `render_conversation()`?"
- **Layer-level dirty**: "Should compositor re-composite this layer?"

**Better Pattern**:

```rust
impl App {
    fn render(&mut self, terminal: &mut Terminal) -> Result<()> {
        // Detect changes at App level
        if self.display.messages_changed() {
            self.render_conversation();
            // render_conversation() internally calls:
            // self.compositor.mark_layer_dirty(self.layers.conversation)
        }

        if self.input_changed() {
            self.render_input();
        }

        // Avatar ALWAYS renders (animation)
        self.render_avatar();

        // Compositor handles dirty tracking from here
        terminal.draw(|frame| {
            let output = self.compositor.composite();
            frame.buffer_mut().merge(output);
        })?;

        Ok(())
    }
}
```

**Key Insight**: Let each layer's render function handle marking itself dirty. Don't duplicate this in App struct.

---

### 🎯 FINAL VERDICT

**APPROVAL STATUS**: ✅ APPROVED WITH REQUIRED CHANGES

**Required Changes**:
1. Fix STORY 2 `composite()` implementation (CRITICAL)
2. Add dirty marking to visibility/move/resize operations
3. Review STORY 4 approach (avoid state duplication)

**Optional Improvements**:
- Add debug logging: `log::trace!("Compositing {} dirty layers", dirty_count);`
- Add assertion: `debug_assert!(self.dirty_layers.len() <= self.layers.len());`

**Timeline Estimate**: Still 2 hours, BUT:
- STORY 2 needs the corrected implementation (saves debugging time later)
- STORY 4 needs architectural clarity (may take extra 15 mins)

**Confidence Level**: 🟢 HIGH - This is a well-understood pattern in immediate-mode GUIs

---

### 📚 RATATUI BEST PRACTICES APPLIED

✅ **Immediate Mode Rendering**: Render functions are idempotent
✅ **Buffer Ownership**: Compositor owns output, layers own their buffers
✅ **No Retained State**: Layers are just buffers, no complex state
✅ **Framework Integration**: Uses `Buffer::merge()`, not fighting Ratatui
✅ **Separation of Concerns**: App renders, Compositor composites, Ratatui diffs

---

### 🚀 READY TO IMPLEMENT?

**YES** - with the corrected STORY 2 implementation.

**Next Steps**:
1. Update STORY 2 code in EPIC-002 TODO
2. Implement STORY 1 (data structures)
3. Implement STORY 2 (corrected composite logic)
4. Implement STORY 3 (mark layers dirty)
5. Review STORY 4 (avoid state duplication)
6. Measure and celebrate!

---

**Reviewed and Approved**: Rust + Ratatui Specialist
**Confidence**: 95% (5% reserved for unforeseen edge cases)
**Risk Level**: LOW (conservative approach, well-tested pattern)
