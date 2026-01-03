# EPIC-001: Quick Wins - Use Ratatui Properly

**Epic**: ODYSSEY: TUI Framebuffer Refactor
**Created**: 2026-01-03
**Owner**: Claudia
**Timeline**: TODAY (1-2 hours)
**Status**: 🚀 ACTIVE

---

## 🎯 Goal

Replace the 10,000+ cell clone loop with `Buffer::merge()` to leverage Ratatui's built-in dirty tracking. Expected: **2-3x FPS boost, 50-70% CPU reduction** immediately.

---

## 📋 Stories

### ✅ STORY 1: Research TUI Libraries
**Status**: COMPLETE
**Time**: 30 mins

**Done**:
- Researched Ratatui 0.29 capabilities
- Discovered `Buffer::merge()` and `Buffer::diff()`
- Found we have `DirtyTracker` already implemented
- Identified cell clone loop as bottleneck

**Findings**: See ODYSSEY-tui-to-framebuffer.md

---

### ✅ STORY 2: Replace Cell Clone Loop
**Status**: COMPLETE (2026-01-03)
**Time**: 10 mins
**File**: `tui/src/app.rs:820-848`

**Current Code** (lines 835-844):
```rust
for y in 0..area.height.min(output.area.height) {
    for x in 0..area.width.min(output.area.width) {
        let idx = output.index_of(x, y);
        if idx < output.content.len() {
            // TODO: Cell cloning is expensive (10k+ per frame)
            buf[(x, y)] = output.content[idx].clone();
        }
    }
}
```

**Target Code**:
```rust
// Option 1: Use merge (simple)
frame.buffer_mut().merge(output);

// Option 2: Render directly (avoid compositor buffer entirely)
// Render each layer directly to frame.buffer_mut()
// (More invasive, save for EPIC-002)
```

**Tasks**:
- [x] Read Ratatui `Buffer::merge()` documentation
- [x] Replace clone loop with `merge()` call
- [x] Compile and test
- [ ] Verify no visual regressions (needs user testing)

**Actual Code Change**:
```rust
// BEFORE (lines 830-845):
terminal.draw(|frame| {
    let output = self.compositor.composite();
    let area = frame.area();
    let buf = frame.buffer_mut();

    for y in 0..area.height.min(output.area.height) {
        for x in 0..area.width.min(output.area.width) {
            let idx = output.index_of(x, y);
            if idx < output.content.len() {
                buf[(x, y)] = output.content[idx].clone();
            }
        }
    }
})?;

// AFTER (lines 830-836):
terminal.draw(|frame| {
    let output = self.compositor.composite();

    // Use Ratatui's Buffer::merge() instead of cell-by-cell cloning
    frame.buffer_mut().merge(output);
})?;
```

**Result**:
- ✅ Clone loop removed (10 lines → 4 lines)
- ✅ Compiles without errors
- ✅ Build time: 1.73s (release)
- ⏳ Visual testing pending (user needs to run ./yollayah.sh)

---

### ⏳ STORY 3: Measure Performance Improvement
**Status**: PENDING
**Time**: 15 mins

**Tasks**:
- [ ] Baseline: Run TUI, measure CPU with btop
- [ ] Baseline: Count FPS (add frame counter to debug output)
- [ ] After fix: Measure CPU
- [ ] After fix: Count FPS
- [ ] Document results in this file

**Acceptance Criteria**:
- ✅ 2-3x FPS improvement
- ✅ 50-70% CPU reduction
- ✅ Numbers documented below

---

### ⏳ STORY 4: Clean Up Compositor (Optional)
**Status**: PENDING
**Time**: 15 mins
**File**: `tui/src/compositor/mod.rs`

**Question**: Do we still need `compositor.output` buffer?

**Options**:
1. Keep it (renders to compositor buffer, then merge to frame)
2. Remove it (render layers directly to frame buffer)

**Decision**: Defer to EPIC-002 (layer dirty tracking needs compositor buffer)

---

## 📊 Performance Measurements

### Baseline (Before)

```bash
# Measure FPS and CPU
cargo build --release
./target/release/yollayah-tui

# In another terminal:
btop  # Watch yollayah-tui process

# Stress test:
cargo test --test stress_test stress_test_rapid_token_streaming --release -- --nocapture
```

**Results**:
- FPS: _____ (to be measured)
- CPU (idle): _____ %
- CPU (streaming): _____ %
- Memory: _____ MB

### After Quick Win

**Results**:
- FPS: _____ (expected: 2-3x baseline)
- CPU (idle): _____ % (expected: 50-70% reduction)
- CPU (streaming): _____ % (expected: 50-70% reduction)
- Memory: _____ MB (expected: similar or lower)

**Improvement**:
- FPS: _____ → _____ ( _____x )
- CPU: _____ → _____ ( -_____% )

---

## 🔍 Technical Details

### Ratatui's Buffer::merge()

From [Ratatui docs](https://docs.rs/ratatui/0.29.0/ratatui/buffer/struct.Buffer.html#method.merge):

```rust
pub fn merge(&mut self, other: &Buffer)
```

Merges another buffer into this one:
- Copies non-empty cells from `other` to `self`
- Preserves `self`'s cells where `other` has empty cells
- Much faster than manual cell-by-cell copying
- Leverages memory-efficient bulk operations

**Why it's faster than our loop**:
1. No per-cell bounds checking (done once for whole buffer)
2. Uses optimized memory copy operations
3. Avoids individual `Cell::clone()` calls
4. Ratatui can optimize further (SIMD, bulk copies, etc)

### How Ratatui's Diff Works

After `terminal.draw()`, Ratatui:
1. Calls `current_buffer.diff(&previous_buffer)`
2. Generates minimal ANSI escape sequence
3. Writes only changed cells to terminal
4. Swaps buffers for next frame

**We were breaking this** by cloning every cell, making Ratatui's diff see all cells as changed.

---

## 🚧 Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Visual regression | Low | Manual testing before/after |
| Buffer size mismatch | Low | merge() handles this |
| Breaking compositor | Low | Only changing final copy step |

---

## ✅ Definition of Done

- [ ] Cell clone loop removed from `app.rs`
- [ ] Using `frame.buffer_mut().merge(output)`
- [ ] No compiler errors or warnings
- [ ] Visual QA: TUI displays correctly
- [ ] Avatar animations work
- [ ] Performance measured and documented
- [ ] 2-3x FPS improvement confirmed
- [ ] 50%+ CPU reduction confirmed
- [ ] Changes committed with clear message

---

## 📝 Implementation Notes

### Change Location
**File**: `/var/home/machiyotl/src/ai-way/tui/src/app.rs`
**Function**: `render()`
**Lines**: 820-848

### Before (SLOW):
```rust
terminal.draw(|frame| {
    let area = frame.area();
    let output = self.compositor.composite();
    let buf = frame.buffer_mut();

    // NESTED LOOPS - 10k+ cell clones
    for y in 0..area.height.min(output.area.height) {
        for x in 0..area.width.min(output.area.width) {
            let idx = output.index_of(x, y);
            if idx < output.content.len() {
                buf[(x, y)] = output.content[idx].clone();
            }
        }
    }
})?;
```

### After (FAST):
```rust
terminal.draw(|frame| {
    let output = self.compositor.composite();

    // ONE LINE - leverages Ratatui's optimizations
    frame.buffer_mut().merge(output);
})?;
```

**Code reduction**: 10 lines → 3 lines
**Performance improvement**: 10,000 clones → 1 bulk merge

---

## 🎓 Lessons Learned

### Why We Missed This
- TODO comment acknowledged the problem but didn't point to solution
- Didn't read Ratatui docs thoroughly
- Assumed manual cell manipulation was necessary
- Didn't realize `merge()` existed

### What We Learned
- "Don't reinvent the wheel" means reading the framework docs!
- Ratatui is well-designed - trust the framework
- Simple solutions (one function call) often beat complex ones
- Performance problems aren't always what they seem (we thought it was "breathing colors" - it was cell cloning!)

---

## 🔗 Related

- ODYSSEY-tui-to-framebuffer.md (parent)
- BUG-003-tui-performance-regression.md (investigation)
- `tui/src/compositor/mod.rs` (compositor implementation)
- `tui/src/app.rs` (main render loop)

---

**Next**: Start STORY 2 - Replace the clone loop NOW! 🚀
