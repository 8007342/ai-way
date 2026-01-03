# TODO-TUI: Central TUI Task Tracking

**Created**: 2026-01-03
**Component**: TUI Surface (Rust + Ratatui)
**Status**: 🟢 ACTIVE

---

## Overview

Central tracking point for all TUI-related tasks, features, and improvements. This file lives WITH the TUI code and tracks component-specific work.

---

## Active Tasks

### High Priority
- [ ] **Framebuffer optimization** (see progress/TODO-017-framebuffer-optimization.md)
  - Reduce unnecessary redraws
  - Implement dirty tracking for layers
  - Target: 60fps stable

- [ ] **Avatar animation integration** (tracked in sprites/mood/cache dirs)
  - Sprite rendering system
  - Mood-based animation selection
  - Aggressive caching layer

### Medium Priority
- [ ] **Loading screen improvements**
  - Animated spinner
  - Progress indicators
  - "Almost there" messaging for AJ

- [ ] **Color palette rotation**
  - Dynamic theme switching
  - Mood-influenced colors
  - Accessibility considerations

### Low Priority
- [ ] **Keyboard shortcuts**
  - Ctrl+C graceful exit
  - Ctrl+L clear screen
  - Document for AJ

---

## Completed

- [x] **Initial TUI implementation** - Basic Ratatui integration
- [x] **Terminal ownership model** - Proper startup/shutdown
- [x] **Message rendering** - Chat interface working

---

## Related TODOs

- **progress/TODO-017-framebuffer-optimization.md** - Performance work
- **progress/TODO-epic-2026Q1-avatar-animation.md** - Avatar roadmap
- **yollayah/shared/yollayah/proto/sprites/TODO-sprites-init.md** - Sprite system
- **yollayah/shared/yollayah/mood/TODO-coherent-evolving-mood-system-init.md** - Mood system
- **yollayah/shared/yollayah/cache/TODO-animation-cache-init.md** - Caching layer

---

## Notes

- TUI is a **surface**, not the core application
- Must remain async and non-blocking
- Conductor owns state, TUI is just a view
- Fallback to bash surface if TUI fails (see bash/TODO-bash-minimal-fallback.md)

---

**Update this file as TUI work progresses. When major milestones complete, update the Completed section.**
