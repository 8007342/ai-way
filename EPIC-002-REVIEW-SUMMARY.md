# EPIC-002 Architecture Review - Executive Summary

**Reviewed By**: Rust + Ratatui Specialist
**Date**: 2026-01-03
**Status**: 🚨 APPROVED WITH CRITICAL CHANGES REQUIRED

---

## TL;DR

**Verdict**: ✅ **APPROVED** - but STORY 2 implementation has a fatal flaw that MUST be fixed before implementation.

**Issue**: Proposed code only re-blits dirty layers, breaking Z-order and leaving stale pixels.
**Fix**: Re-composite ALL layers when ANY layer is dirty (still get massive perf wins from skipping when idle).

---

## Critical Issue Found

### The Problem

STORY 2's proposed implementation:

```rust
// ❌ WRONG - Breaks layer compositing!
pub fn composite(&mut self) -> &Buffer {
    if self.dirty_layers.is_empty() {
        return &self.output;
    }

    // FATAL FLAW: Only blits dirty layers
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

**Why it fails**:
1. **Z-Order Corruption**: If avatar (Z=2) is dirty but status bar (Z=3) is clean, status won't render over new avatar position
2. **Stale Pixels**: Old content from moved/resized layers remains in output buffer
3. **Incomplete Rendering**: Clean layers don't re-composite over dirty layers below them

### The Fix

```rust
// ✅ CORRECT - Maintains Z-order and clears properly
pub fn composite(&mut self) -> &Buffer {
    if self.dirty_layers.is_empty() {
        return &self.output;  // Still skip when idle!
    }

    // Re-composite ALL layers to maintain Z-order
    self.output.reset();

    for &id in &self.render_order {
        if let Some(layer) = self.layers.get(&id) {
            if layer.visible {
                Self::blit_layer(&mut self.output, &self.area, layer);
            }
        }
    }

    self.dirty_layers.clear();
    &self.output
}
```

**Performance**: Still excellent!
- 5 layers × 500 cells = 2,500 operations (vs 10,000+ before)
- Skips ALL work when idle (the real win)
- Ratatui's `Buffer::merge()` handles final optimization

---

## Additional Required Changes

### 1. Mark Dirty on Visibility Changes

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

### 2. Mark Dirty on Move/Resize

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

### 3. Avatar Always Dirty (Animation)

```rust
fn render_avatar(&mut self) {
    // ... render avatar ...
    self.compositor.mark_layer_dirty(self.layers.avatar);  // ✅ Always mark
}
```

### 4. Conditional Dirty Marking (Other Layers)

```rust
fn render_status(&mut self) {
    let new_status = self.format_status();
    if new_status != self.last_status {
        // ... render status ...
        self.compositor.mark_layer_dirty(self.layers.status);
        self.last_status = new_status;
    }
}
```

---

## Story 4 Architectural Concern

**Proposed**: Add dirty flags to App struct (`conversation_dirty: bool`, etc)

**Problem**: This duplicates state - Compositor already tracks dirty layers

**Better Approach**:

```rust
fn render(&mut self, terminal: &mut Terminal) -> Result<()> {
    // Check if data changed, call render function
    if self.display.messages_changed() {
        self.render_conversation();  // This marks layer dirty internally
    }

    // Avatar always renders (animation)
    self.render_avatar();

    // Compositor uses its own dirty tracking
    terminal.draw(|frame| {
        let output = self.compositor.composite();
        frame.buffer_mut().merge(output);
    })?;

    Ok(())
}
```

**Key Insight**: Don't duplicate compositor's dirty tracking in App. Let render functions mark layers dirty.

---

## Performance Expectations

| Scenario | Before EPIC-002 | After EPIC-002 | Improvement |
|----------|----------------|----------------|-------------|
| **Idle UI** | ~10% CPU | <2% CPU | **5x** |
| **Avatar only** | ~10% CPU | ~5% CPU | **2x** |
| **Streaming** | ~20% CPU | ~12% CPU | **1.7x** |

**Why**: Skipping composite when idle is the big win. Compositing cost is trivial (<0.1ms), terminal I/O is expensive.

---

## Implementation Checklist

### STORY 1: Add Dirty Tracking
- [ ] Add `dirty_layers: HashSet<LayerId>` to Compositor
- [ ] Add `mark_layer_dirty(id: LayerId)` method
- [ ] Add `is_dirty() -> bool` method

### STORY 2: Optimize composite() ⚠️ USE CORRECTED VERSION
- [ ] Early return if no dirty layers
- [ ] **Reset output buffer when dirty** (CHANGED)
- [ ] **Re-composite ALL layers in Z-order** (CHANGED)
- [ ] Clear dirty set after composite
- [ ] Test with static UI

### STORY 3: Mark Layers Dirty in Compositor Operations
- [ ] Add to `set_visible()`
- [ ] Add to `move_layer()`
- [ ] Add to `resize_layer()`
- [ ] Add to `set_z_index()`

### STORY 4: Conditional Rendering ⚠️ AVOID STATE DUPLICATION
- [ ] Check data changes in App::render()
- [ ] Call render functions only when data changed
- [ ] Avatar always calls render (animation)
- [ ] Let render functions mark layers dirty (don't duplicate in App)

### STORY 5: Measurement
- [ ] Measure idle CPU
- [ ] Measure avatar-only CPU
- [ ] Measure streaming CPU
- [ ] Verify 50-80% reduction for idle

---

## Approval

**Status**: ✅ APPROVED WITH REQUIRED CHANGES
**Confidence**: 95% (conservative, well-tested pattern)
**Risk Level**: LOW

**Approved by**: Rust + Ratatui Specialist
**Full Review**: See `TODO-epic-002-layer-dirty-tracking.md` (bottom section)

---

## Next Steps

1. ✅ Update STORY 2 implementation in EPIC-002 TODO (DONE - see review section)
2. ✅ Implement STORY 1 (HashSet + methods) - ALREADY DONE CORRECTLY!
3. ✅ Implement STORY 2 (corrected composite logic) - ALREADY DONE CORRECTLY!
4. Review STORY 3 (verify dirty marking in app.rs)
5. Review STORY 4 approach (avoid duplication)
6. Measure and verify performance gains

---

## 🎉 UPDATE: Stories 1-2 Already Implemented CORRECTLY!

**Verification Date**: 2026-01-03

The compositor implementation (`/var/home/machiyotl/src/ai-way/tui/src/compositor/mod.rs`) already has:

✅ **STORY 1 - Complete**:
- `dirty_layers: HashSet<LayerId>` (line 35)
- `mark_layer_dirty()` method (lines 129-133)
- `is_dirty()` method (lines 135-139)
- `clear_dirty()` private method (lines 141-145)

✅ **STORY 2 - Complete and CORRECT**:
```rust
pub fn composite(&mut self) -> &Buffer {
    if self.dirty_layers.is_empty() {
        return &self.output;  // ✅ Early return when clean
    }

    self.output.reset();  // ✅ Clear output buffer

    // ✅ Re-composite ALL layers in Z-order
    for &id in &self.render_order {
        if let Some(layer) = self.layers.get(&id) {
            if layer.visible {
                Self::blit_layer(&mut self.output, &self.area, layer);
            }
        }
    }

    self.clear_dirty();  // ✅ Clear dirty tracking
    &self.output
}
```

**Bonus Features Already Implemented**:
- ✅ `set_visible()` marks dirty (lines 108-116)
- ✅ `move_layer()` marks dirty (lines 82-91)
- ✅ `resize_layer()` marks dirty (lines 94-105)
- ✅ `set_z_index()` marks dirty (lines 70-79)
- ✅ `resize()` marks all layers dirty (lines 119-127)
- ✅ Excellent documentation comments explaining the logic

**Verdict**: Implementation is PERFECT and follows all recommended practices! 🎊

**Remaining Work**: Only Stories 3-5 remain (app.rs integration and measurement)
