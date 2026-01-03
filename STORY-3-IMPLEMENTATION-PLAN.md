# STORY 3 Implementation Plan - Layer Dirty Tracking

**Status**: ⏸️ BLOCKED - Waiting for Developer 1 to complete STORY 1 & 2
**Date**: 2026-01-03
**Developer**: Developer 2

---

## Prerequisites

Before implementing STORY 3, Developer 1 must complete:

### ✅ STORY 1: Compositor Infrastructure
File: `/var/home/machiyotl/src/ai-way/tui/src/compositor/mod.rs`

**Required additions**:
```rust
use std::collections::{HashMap, HashSet};

pub struct Compositor {
    // ... existing fields ...

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

    pub fn clear_dirty(&mut self) {
        self.dirty_layers.clear();
    }
}
```

**Verification**: Run this command to verify:
```bash
grep -n "dirty_layers" /var/home/machiyotl/src/ai-way/tui/src/compositor/mod.rs
grep -n "mark_layer_dirty" /var/home/machiyotl/src/ai-way/tui/src/compositor/mod.rs
```

### ✅ STORY 2: Optimize composite()
File: `/var/home/machiyotl/src/ai-way/tui/src/compositor/mod.rs`

**Required changes**: The `composite()` method must skip rendering when no layers are dirty.

**Verification**: Check that `composite()` returns early if `dirty_layers.is_empty()`

---

## Implementation Details

Once prerequisites are met, implement STORY 3 in `/var/home/machiyotl/src/ai-way/tui/src/app.rs`:

### 1. render_conversation() - Line 842
**Location**: End of function, after line 996 (after the buffer rendering loop)
**Layer**: `self.layers.conversation`

**Add**:
```rust
fn render_conversation(&mut self) {
    // ... existing rendering code (lines 842-996) ...

    // Mark layer as needing re-composite
    self.compositor.mark_layer_dirty(self.layers.conversation);
}
```

**Rationale**: Conversation changes when new messages arrive or streaming updates occur.

---

### 2. render_tasks() - Line 1179
**Location**: End of function, after line 1196 (after task rendering)
**Layer**: `self.layers.tasks`

**Add**:
```rust
fn render_tasks(&mut self) {
    // ... existing task rendering code (lines 1179-1196) ...

    // Mark layer as needing re-composite
    self.compositor.mark_layer_dirty(self.layers.tasks);
}
```

**Rationale**: Task panel updates when tasks start, progress, or complete.

---

### 3. render_input() - Line 999
**Location**: End of function, after line 1064 (after input buffer rendering)
**Layer**: `self.layers.input`

**Add**:
```rust
fn render_input(&mut self) {
    // ... existing input rendering code (lines 999-1064) ...

    // Mark layer as needing re-composite
    self.compositor.mark_layer_dirty(self.layers.input);
}
```

**Rationale**: Input changes with every keystroke and cursor movement.

---

### 4. render_status() - Line 1067
**Location**: End of function, after line 1176 (after status bar rendering)
**Layer**: `self.layers.status`

**Add**:
```rust
fn render_status(&mut self) {
    // ... existing status rendering code (lines 1067-1176) ...

    // Mark layer as needing re-composite
    self.compositor.mark_layer_dirty(self.layers.status);
}
```

**Rationale**: Status updates with conductor state changes and task counts.

---

### 5. render_avatar() - Line 1246
**Location**: End of function, after line 1251 (after avatar rendering)
**Layer**: `self.layers.avatar`

**Add**:
```rust
fn render_avatar(&mut self) {
    if let Some(buf) = self.compositor.layer_buffer_mut(self.layers.avatar) {
        buf.reset();
        self.avatar.render(buf);
    }

    // Mark layer as needing re-composite
    self.compositor.mark_layer_dirty(self.layers.avatar);
}
```

**Rationale**: Avatar animates continuously, always needs re-compositing.

---

## Testing Plan

After implementation:

1. **Build verification**:
   ```bash
   cd /var/home/machiyotl/src/ai-way
   cargo build --workspace
   ```

2. **Manual testing**:
   ```bash
   ./yollayah.sh --test
   ```
   - Verify UI renders correctly
   - Type in input field - should see changes
   - Send a message - should see conversation update
   - Check avatar animates smoothly

3. **Check for regressions**:
   - No visual glitches
   - No missing content
   - Smooth animations
   - Responsive input

---

## Expected Impact

With dirty tracking in place:
- **Idle UI**: Only avatar layer marked dirty (animation)
- **Typing**: Input + avatar layers marked dirty
- **Streaming**: Conversation + avatar layers marked dirty
- **Full render**: All 5 layers marked dirty (rare)

This sets the foundation for STORY 4 (conditional rendering) which will add per-layer dirty flags to skip rendering unchanged layers entirely.

---

## Current Status

**BLOCKED**: Waiting for Developer 1 to implement:
- [ ] STORY 1: Compositor dirty tracking infrastructure
- [ ] STORY 2: Optimized composite() method

Once unblocked, this implementation should take ~30 minutes.
