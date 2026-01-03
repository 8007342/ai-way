# EPIC-003: Wire Up Existing DirtyTracker

**Epic**: ODYSSEY: TUI Framebuffer Refactor
**Created**: 2026-01-03
**Owner**: TBD
**Timeline**: WEEK 2 (2 hours)
**Status**: 📋 PLANNED
**Depends On**: EPIC-002

---

## 🎯 Goal

Use the already-implemented `DirtyTracker` for avatar animations. Only render cells that changed, skip unchanged avatar regions.

**Expected Impact**: Avatar CPU < 5% (currently ~20% with breathing)

---

## 📋 Stories

### ⏳ STORY 1: Review Existing DirtyTracker
**Status**: PENDING
**Time**: 15 mins
**File**: `tui/src/avatar/dirty_tracker.rs`

**What we already have**:
- ✅ `DirtyTracker` struct with `HashSet<(u16, u16)>`
- ✅ `mark_dirty(x, y, width, height)` - mark rectangular regions
- ✅ `get_dirty_rects()` - return optimized bounding boxes
- ✅ `clear_dirty()` - reset after rendering
- ✅ Row-based and vertical rect merging
- ✅ Full test coverage

**Tasks**:
- [ ] Read dirty_tracker.rs implementation
- [ ] Understand how it merges dirty cells into rects
- [ ] Check existing integration with avatar (lines 373-403)

---

### ⏳ STORY 2: Add DirtyTracker to Avatar Struct
**Status**: PENDING
**Time**: 15 mins
**File**: `tui/src/avatar/mod.rs`

**Current**:
```rust
pub struct Avatar {
    bounds: Rect,
    position: (u16, u16),
    size: AvatarSize,
    engine: AnimationEngine,
    // ... other fields
}
```

**Target**:
```rust
pub struct Avatar {
    bounds: Rect,
    position: (u16, u16),
    size: AvatarSize,
    engine: AnimationEngine,

    // NEW: Track which cells changed
    dirty_tracker: DirtyTracker,
}

impl Avatar {
    pub fn new(...) -> Self {
        Self {
            // ...
            dirty_tracker: DirtyTracker::new(),
        }
    }
}
```

**Tasks**:
- [ ] Add dirty_tracker field
- [ ] Initialize in new()
- [ ] Update all constructors

---

### ⏳ STORY 3: Mark Dirty Cells During Animation Update
**Status**: PENDING
**Time**: 30 mins
**File**: `tui/src/avatar/mod.rs`

**Pattern**: Mark cells dirty when animation frame changes

```rust
impl Avatar {
    pub fn update(&mut self, delta: Duration) {
        let old_frame = self.engine.current_frame;
        let old_position = self.position;

        // Update animation
        self.engine.update(delta, self.size);
        self.update_position(delta);

        // NEW: Mark changed regions dirty
        if old_frame != self.engine.current_frame {
            // Frame changed - mark entire sprite dirty
            let (width, height) = self.size.dimensions();
            self.dirty_tracker.mark_dirty(
                self.position.0,
                self.position.1,
                width,
                height
            );
        }

        if old_position != self.position {
            // Position changed - mark old and new positions dirty
            let (width, height) = self.size.dimensions();
            self.dirty_tracker.mark_dirty(old_position.0, old_position.1, width, height);
            self.dirty_tracker.mark_dirty(self.position.0, self.position.1, width, height);
        }
    }
}
```

**Tasks**:
- [ ] Track previous frame/position
- [ ] Compare before/after update
- [ ] Mark only changed regions dirty
- [ ] Test with breathing animation

---

### ⏳ STORY 4: Render Only Dirty Regions
**Status**: PENDING
**Time**: 45 mins
**File**: `tui/src/avatar/mod.rs`

**Current** (renders entire sprite):
```rust
pub fn render(&self, buf: &mut Buffer) -> Result<(), AvatarError> {
    let frame = self.engine.current_frame(self.size)?;

    for (row_idx, row) in frame.cells.iter().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            // Render every cell
            if !cell.is_empty() {
                let x = self.position.0 + col_idx as u16;
                let y = self.position.1 + row_idx as u16;
                // ...
            }
        }
    }

    Ok(())
}
```

**Target** (renders only dirty rects):
```rust
pub fn render(&mut self, buf: &mut Buffer) -> Result<(), AvatarError> {
    let frame = self.engine.current_frame(self.size)?;
    let dirty_rects = self.dirty_tracker.get_dirty_rects();

    if dirty_rects.is_empty() {
        return Ok(());  // Nothing changed!
    }

    // Render only dirty regions
    for rect in dirty_rects {
        for y in rect.y..(rect.y + rect.height) {
            for x in rect.x..(rect.x + rect.width) {
                // Render cells within dirty rect only
                let row_idx = (y - self.position.1) as usize;
                let col_idx = (x - self.position.0) as usize;

                if row_idx < frame.cells.len() && col_idx < frame.cells[row_idx].len() {
                    let cell = &frame.cells[row_idx][col_idx];
                    if !cell.is_empty() {
                        // ... render cell ...
                    }
                }
            }
        }
    }

    self.dirty_tracker.clear_dirty();
    Ok(())
}
```

**Tasks**:
- [ ] Get dirty rects from tracker
- [ ] Skip render if no dirty rects
- [ ] Iterate only dirty regions
- [ ] Clear dirty after render
- [ ] Verify visual correctness

---

### ⏳ STORY 5: Measure Avatar CPU Usage
**Status**: PENDING
**Time**: 15 mins

**Test Scenarios**:
1. **Breathing animation** (subtle color changes)
   - Before: ~5-10 cells change per frame
   - After: Only render those 5-10 cells
   - Expected CPU: <5%

2. **Position change** (avatar wandering)
   - Before: Render entire sprite at old and new position
   - After: Dirty tracker marks both positions
   - Expected: 2x sprite size cells rendered (optimal)

3. **Frame change** (animation transition)
   - Before: Render entire sprite
   - After: Mark entire sprite dirty (necessary)
   - Expected: Same as before (no optimization possible)

**Tasks**:
- [ ] Measure CPU with breathing only
- [ ] Measure CPU with wandering
- [ ] Measure CPU with frame transitions
- [ ] Document results

---

## 📊 Success Criteria

- ✅ Avatar CPU < 5% during breathing animation
- ✅ DirtyTracker integrated with rendering
- ✅ Only changed cells rendered
- ✅ Dirty tracker cleared each frame
- ✅ Avatar animations smooth and correct

---

## 🔍 Technical Details

### How DirtyTracker Works

From `dirty_tracker.rs`:

1. **Mark cells dirty**: `mark_dirty(x, y, width, height)`
   - Adds cells to `HashSet<(u16, u16)>`
   - Tracks rectangular region

2. **Optimize to rects**: `get_dirty_rects()`
   - Merges adjacent cells into rectangles
   - Row-based optimization (scan horizontally)
   - Vertical merging (combine rows into taller rects)
   - Returns minimal set of `Rect`

3. **Clear**: `clear_dirty()`
   - Empties the dirty cell set
   - Call after rendering

**Why it's efficient**:
- HashSet lookups: O(1)
- Rect merging: O(n) where n = dirty cells (small)
- Rendering: O(m) where m = area of dirty rects (much smaller than full sprite)

---

## 🔗 Related

- EPIC-002: Layer Dirty Tracking (parallel approach)
- `tui/src/avatar/dirty_tracker.rs` (implementation)
- `tui/src/avatar/mod.rs` (avatar rendering)
- `tui/src/avatar/animation.rs` (frame updates)
