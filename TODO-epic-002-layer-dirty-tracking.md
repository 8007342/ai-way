# EPIC-002: Layer-Level Dirty Tracking

**Epic**: ODYSSEY: TUI Framebuffer Refactor
**Created**: 2026-01-03
**Owner**: TBD
**Timeline**: WEEK 1 (2 hours)
**Status**: 📋 PLANNED
**Depends On**: EPIC-001

---

## 🎯 Goal

Only re-composite layers that have changed. Skip full compositor blit when UI is idle or only one layer updated.

**Expected Impact**: Additional 50-80% CPU reduction when UI is mostly static.

---

## 📋 Stories

### ⏳ STORY 1: Add Dirty Layer Tracking to Compositor
**Status**: PENDING
**Time**: 30 mins
**File**: `tui/src/compositor/mod.rs`

**Tasks**:
- [ ] Add `dirty_layers: HashSet<LayerId>` field to `Compositor`
- [ ] Add `mark_layer_dirty(id: LayerId)` method
- [ ] Add `is_dirty() -> bool` method
- [ ] Add `clear_dirty()` method

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

### ⏳ STORY 2: Optimize composite() to Skip Unchanged Layers
**Status**: PENDING
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
- [ ] Skip composite if no dirty layers
- [ ] Only blit dirty layers
- [ ] Clear dirty set after composite
- [ ] Test with static UI (no changes)

---

### ⏳ STORY 3: Mark Layers Dirty in Rendering Functions
**Status**: PENDING
**Time**: 30 mins
**File**: `tui/src/app.rs`

**Locations to update**:
- `render_conversation()` - mark conversation layer dirty
- `render_input()` - mark input layer dirty
- `render_status()` - mark status layer dirty
- `render_avatar()` - mark avatar layer dirty

**Pattern**:
```rust
fn render_conversation(&mut self) {
    // ... render conversation to layer buffer ...

    // Mark layer as needing re-composite
    self.compositor.mark_layer_dirty(self.layers.conversation);
}
```

**Tasks**:
- [ ] Add layer dirty marking to each render function
- [ ] Only mark if content actually changed
- [ ] Test that idle UI doesn't mark layers dirty

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
