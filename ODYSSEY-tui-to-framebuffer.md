# ODYSSEY: TUI Framebuffer Refactor

**Created**: 2026-01-03
**Status**: 🚢 IN PROGRESS
**Owner**: Claudia
**Tlatoani's Decree**: "Let's do a proper framebuffer. Keep it simple."

---

## 🎯 Mission

Replace lazy full-screen rendering with proper dirty-tracked framebuffer. **BUT**: Don't reinvent the wheel - leverage Ratatui's built-in `Buffer::diff()` instead of rebuilding ncurses.

**Critical Discovery**: We already have dirty tracking implemented (`dirty_tracker.rs`) and Ratatui has `Buffer::merge()` - we're just not using them! The problem is a **10,000+ cell clone loop** that bypasses Ratatui's optimizations.

---

## 📊 Current vs Target Performance

| Metric | Current | Target | Improvement |
|--------|---------|--------|-------------|
| **Cells scanned/frame** | 10,000+ | 50-500 | **20-200x** |
| **Cell clones/frame** | 10,000+ | 0 | **∞x** |
| **Text rewrapping** | Every frame | Only on change | **∞x** |
| **Idle CPU** | ~10% | <1% | **10x** |
| **Render latency** | 50-100ms | 5-10ms | **5-10x** |

---

## 🗺️ The Journey (4 Epics)

### EPIC 1: Quick Wins - Use Ratatui Properly ⚡ (1 DAY)
**Goal**: Replace cell cloning with `Buffer::merge()` - 2-3x perf boost immediately
**Status**: ✅ CODE COMPLETE, AWAITING USER TESTING

**Stories**:
1. ✅ Research TUI libraries (Ratatui, ncurses, etc) - DONE (30 mins)
2. ✅ Replace cell clone loop with `merge()` - DONE (10 mins)
3. ⏳ Measure performance improvement - PENDING (needs user testing)
4. ⏳ Remove redundant compositor output buffer - DEFERRED to EPIC-002

**Expected Impact**: 50-70% CPU reduction, 2-3x FPS boost
**Code Changed**: `tui/src/app.rs` lines 830-836 (10 lines → 4 lines)

---

### EPIC 2: Layer-Level Dirty Tracking 🎨 (WEEK 1)
**Goal**: Only re-composite layers that changed

**Stories**:
1. ⏳ Add `dirty_layers: HashSet<LayerId>` to Compositor
2. ⏳ Implement `mark_layer_dirty(id)` API
3. ⏳ Skip compositing if no dirty layers
4. ⏳ Track layer changes in conversation/input/status rendering
5. ⏳ Measure improvement with mostly-static UI

**Expected Impact**: Additional 50-80% reduction when UI is idle

---

### EPIC 3: Wire Up Existing DirtyTracker 🎭 (WEEK 2)
**Goal**: Use avatar's `DirtyTracker` for fine-grained animation optimization

**Stories**:
1. ⏳ Wire `DirtyTracker` into avatar rendering
2. ⏳ Mark only changed animation cells dirty
3. ⏳ Skip rendering unchanged avatar regions
4. ⏳ Test breathing animation CPU usage

**Expected Impact**: Avatar animations use <5% CPU (currently ~20%)

---

### EPIC 4: Incremental Message Rendering 📜 (WEEK 2-3)
**Goal**: Stop rebuilding entire conversation every frame

**Stories**:
1. ⏳ Cache wrapped lines per message
2. ⏳ Track last rendered message count
3. ⏳ Only wrap NEW messages
4. ⏳ Only update LAST message if streaming
5. ⏳ Invalidate cache on terminal resize
6. ⏳ Benchmark streaming performance

**Expected Impact**: 10-50x faster streaming rendering

---

## 📋 Epic Tracking

- **EPIC-001**: Quick Wins - Use Ratatui Properly → `TODO-epic-001-ratatui-quick-wins.md`
- **EPIC-002**: Layer-Level Dirty Tracking → `TODO-epic-002-layer-dirty-tracking.md`
- **EPIC-003**: Wire Up Existing DirtyTracker → `TODO-epic-003-avatar-dirty-tracker.md`
- **EPIC-004**: Incremental Message Rendering → `TODO-epic-004-incremental-messages.md`

---

## 🔬 Research Findings

### What We Learned

1. **Ratatui 0.29 has built-in `Buffer::diff()`**
   - Automatically tracks changes between frames
   - Writes only modified cells to terminal
   - We're bypassing it with manual cell cloning

2. **We already have dirty tracking** (`dirty_tracker.rs`)
   - Fully implemented with tests
   - Used by avatar animations
   - Not connected to main compositor

3. **Ncurses comparison is misleading**
   - Ncurses requires manual dirty tracking (C, low-level)
   - Ratatui handles it automatically (Rust, high-level)
   - We should use Ratatui's approach, not fight it

4. **The real bottleneck** (`app.rs:835-844`)
   ```rust
   // 10,000+ cell clones per frame:
   for y in 0..area.height {
       for x in 0..area.width {
           buf[(x, y)] = output.content[idx].clone();  // ← REMOVE THIS
       }
   }

   // Replace with ONE line:
   frame.buffer_mut().merge(output);
   ```

### TUI Library Analysis

| Library | Dirty Tracking | Approach |
|---------|---------------|----------|
| **Ratatui 0.29** | Automatic `Buffer::diff()` | Redraw everything, framework optimizes |
| **Cursive** | Also uses Ratatui | Same approach |
| **Ncurses** | Manual regions (C) | Required in low-level C code |
| **Termion** | None | DIY everything |

**Decision**: Use Ratatui's `merge()` + layer-level tracking for best balance.

---

## 🏗️ Architecture

### Current (BAD)

```
Compositor → output buffer
    ↓
Cell-by-cell clone loop (10k+ clones)
    ↓
frame.buffer_mut()
    ↓
Ratatui's diff (but sees all cells as changed)
    ↓
Terminal
```

### Target (GOOD)

```
Compositor (tracks dirty layers)
    ↓
Only composite dirty layers
    ↓
frame.buffer_mut().merge(output)  ← ONE operation
    ↓
Ratatui's diff (sees minimal changes)
    ↓
Terminal
```

---

## 🎯 Success Criteria

### Phase 1 (Quick Wins) - DONE WHEN:
- ✅ Cell clone loop removed
- ✅ Using `Buffer::merge()` instead
- ✅ 2-3x FPS improvement measured
- ✅ No visual regressions

### Phase 2 (Layer Dirty Tracking) - DONE WHEN:
- ✅ Idle CPU < 2% (no input, no streaming)
- ✅ Only dirty layers re-composited
- ✅ Logging shows dirty layer counts

### Phase 3 (Avatar DirtyTracker) - DONE WHEN:
- ✅ Avatar CPU < 5% during breathing
- ✅ DirtyTracker integrated with rendering
- ✅ Only changed cells rendered

### Phase 4 (Incremental Messages) - DONE WHEN:
- ✅ Streaming feels instant (matches GPU speed)
- ✅ Only new/changed messages wrapped
- ✅ Cache invalidation works on resize

---

## 📈 Measurement Plan

**Before Each Epic**:
```bash
# Baseline CPU/FPS
cargo build --release
./target/release/yollayah-tui &
btop  # Watch yollayah-tui process

# Stress test
cargo test --test stress_test stress_test_rapid_token_streaming --release -- --nocapture
```

**After Each Epic**:
- Record FPS (visual observation + logs)
- Record CPU % (btop)
- Record memory (btop)
- Compare to baseline

---

## 🚧 Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| **Breaking avatar animations** | Use feature flag, test before/after |
| **Visual regressions** | Screenshot tests, manual QA |
| **Merge conflicts** | Work in feature branch `odyssey/framebuffer` |
| **Over-engineering** | Start with simplest solution (merge), add complexity only if needed |

---

## 📝 Notes & Learnings

### 2026-01-03 16:00 - Research Complete
- Discovered Ratatui's `Buffer::merge()` - we were doing it wrong!
- Found our own `DirtyTracker` implementation (ready to use)
- Identified the 10k cell clone loop as the smoking gun

**Key insight**: "Don't reinvent the wheel" means using Ratatui's features, not fighting them.

### 2026-01-03 19:30 - EPIC-001 Quick Win Complete! 🎉
- Replaced 10-line cell clone loop with 1-line `merge()` call
- Code reduction: 10 lines → 4 lines (60% less code)
- Build successful: 1.73s (release mode)
- **Ready for user testing** - needs `./yollayah.sh` run to verify

**Implementation**:
```rust
// BEFORE: 10,000+ cell clones per frame
for y in 0..area.height {
    for x in 0..area.width {
        buf[(x, y)] = output.content[idx].clone();
    }
}

// AFTER: ONE optimized operation
frame.buffer_mut().merge(output);
```

**Next**: User tests ./yollayah.sh, measures FPS/CPU improvement

---

## 🎭 The Tlatoani's Wisdom

> "We prefer simplicity, since this is only a SURFACE of the whole ai-way project, and it shouldn't take this long to build, it's sidetracking us from our main project: the rest of ai-way."

**Translation**: Keep It Simple, Stupid (KISS). Use Ratatui's tools, don't rebuild ncurses.

---

## 📚 References

- [Ratatui Rendering Under the Hood](https://ratatui.rs/concepts/rendering/under-the-hood/)
- [Buffer::diff() Documentation](https://docs.rs/ratatui/0.29.0/ratatui/buffer/struct.Buffer.html#method.diff)
- Our own `tui/src/avatar/dirty_tracker.rs` (already implemented!)
- BUG-003-tui-performance-regression.md (the investigation that led here)

---

**Next Step**: Create EPIC-001 TODO and START THE QUICK WIN! 🚀
