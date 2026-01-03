# Yollayah TUI Loading Screen Design - Complete Package

**Task**: STORY 3 - Design Yollayah-Themed Loading UI
**Project**: TODO-tui-initialization.md
**Status**: Complete - Ready for Implementation
**Created**: 2026-01-03

---

## Quick Start

### For Decision Makers
**Read**: [LOADING-SCREEN-SUMMARY.txt](LOADING-SCREEN-SUMMARY.txt) (5 minutes)

**TL;DR**:
- Problem: Blank screen on TUI startup looks broken
- Solution: Show Yollayah axolotl with pulsing gills during loading
- Recommendation: **Option B** (2 hours, delight factor 8/10)
- Next: Review design, approve, implement

### For Designers
**Read**: [LOADING-SCREEN-MOCKUPS.txt](LOADING-SCREEN-MOCKUPS.txt) (10 minutes)

**Contains**:
- ASCII mockups of all 3 options
- Frame-by-frame animation details
- Color specifications
- Character positioning

### For Developers
**Read**: [DESIGN-loading-screen.md](DESIGN-loading-screen.md) (20 minutes)

**Contains**:
- Detailed technical analysis
- Color palette from `theme.rs`
- Sprite system integration
- Implementation checklist
- Performance targets

**Then**: Use [LOADING-SCREEN-CODE-TEMPLATE.rs](LOADING-SCREEN-CODE-TEMPLATE.rs) as a starting point

---

## Document Overview

### 1. LOADING-SCREEN-SUMMARY.txt
**Audience**: Executives, Project Managers, Quick Reviewers
**Length**: 2-3 minutes
**Format**: Text with clear sections

**Contains**:
- Problem statement
- 3 design options comparison table
- Recommendation with justification
- Success criteria
- Timeline estimate

**Use this to**: Get quick overview and make decision on which option

---

### 2. LOADING-SCREEN-MOCKUPS.txt
**Audience**: Designers, UX Specialists, Visual Reviewers
**Length**: 10 minutes
**Format**: ASCII art with detailed annotations

**Contains**:
- Full ASCII mockups of all 3 options on 80x24 terminal
- Frame-by-frame animation sequences
- Color values (RGB hex codes)
- Timeline of changes through animation
- Character positioning details
- Variant examples

**Use this to**:
- Visualize exactly how each option looks
- Understand animation timing
- Verify colors match brand palette
- Share with stakeholders for feedback

---

### 3. DESIGN-loading-screen.md
**Audience**: Developers, Technical Leads, Implementation Team
**Length**: 20-30 minutes
**Format**: Markdown with code examples

**Contains**:
- Theme analysis (colors, animation style)
- Technical details for all 3 options
- Detailed pros/cons for each option
- Recommendation rationale
- Implementation plan with checklist
- Code complexity estimates
- Integration points
- Success criteria
- Related files reference

**Use this to**:
- Understand all technical requirements
- Plan implementation approach
- Review code examples
- Set success criteria for testing

---

### 4. LOADING-SCREEN-CODE-TEMPLATE.rs
**Audience**: Rust Developers, Implementation Team
**Length**: Reference document (80-100 lines of actual code)
**Format**: Heavily commented Rust code template

**Contains**:
- Complete struct and enum definitions
- Method signatures with documentation
- Frame building code (with examples)
- Color breathing logic
- Rendering logic
- Usage example (integration in app.rs)
- Testing checklist
- Implementation notes
- Future enhancement ideas

**Use this to**:
- Start implementation immediately
- Copy-paste code structure
- Understand how to use existing sprite system
- Reference color names and breathing functions

**Note**: This is a template showing structure, not production code. Adapt as needed.

---

## Design Decision Matrix

| Aspect | Option A | Option B | Option C |
|--------|----------|----------|----------|
| **Appearance** | Text spinner | Pulsing axolotl | Swimming progress |
| **Code** | 40 lines | 150-200 lines | 250+ lines |
| **Time** | 1.5 hrs | 2 hrs | 3-4 hrs |
| **Delight** | 6/10 | 8/10 | 9/10 |
| **Reuses Code** | Minimal | Excellent | Medium |
| **Terminal Compat** | All | 80x24+ | 80x24+ |
| **CPU** | Low | Low | Medium |
| **Brand Personality** | None | High | High |
| **Upgradeable** | No | Yes→C | N/A |
| ****RECOMMENDATION** | ✗ | **✓ SELECT** | ✗ |

---

## Implementation Roadmap

### Phase 1: Design Approval (This Task - STORY 3)
**Status**: ✅ COMPLETE

**Deliverables**:
- [x] LOADING-SCREEN-SUMMARY.txt (executive summary)
- [x] LOADING-SCREEN-MOCKUPS.txt (visual designs)
- [x] DESIGN-loading-screen.md (technical analysis)
- [x] LOADING-SCREEN-CODE-TEMPLATE.rs (code reference)
- [x] This index document

**Next**: Get team approval on Option B

### Phase 2: Implementation (STORY 4)
**Owner**: Backend Developer
**Estimated**: 2-3 hours
**File**: `tui/src/loading.rs` (new file)

**Tasks**:
- [ ] Review and approve design
- [ ] Implement LoadingScreen struct
- [ ] Build animation frames
- [ ] Implement tick() and render() methods
- [ ] Test on multiple terminal sizes
- [ ] Verify color breathing smooth

### Phase 3: Integration (STORY 4 continued)
**Owner**: Backend Developer
**Estimated**: 2 hours
**Files**: `tui/src/app.rs`

**Tasks**:
- [ ] Show LoadingScreen before Conductor connect
- [ ] Update phase as connection progresses
- [ ] Transition to main UI when ready
- [ ] Handle Ctrl+C during loading
- [ ] Test startup sequence

### Phase 4: Testing & Validation (STORY 6)
**Owner**: QA Specialist
**Estimated**: 2-4 hours
**Files**: `tui/tests/initialization_test.rs`

**Tests**:
- [ ] TUI shows loading screen immediately
- [ ] Loading screen handles Ctrl+C
- [ ] Transitions to main UI when ready
- [ ] Works on multiple terminal sizes

---

## Related Files

### In This Directory
- `TODO-tui-initialization.md` - Full project plan (STORY 3-7)
- `DESIGN-loading-screen.md` - Detailed technical design
- `LOADING-SCREEN-SUMMARY.txt` - Executive summary
- `LOADING-SCREEN-MOCKUPS.txt` - ASCII mockups
- `LOADING-SCREEN-CODE-TEMPLATE.rs` - Code template
- `LOADING-SCREEN-DESIGN-INDEX.md` - This file

### TUI Source Code
- `tui/src/main.rs` - Where LoadingScreen will be used
- `tui/src/app.rs` - App initialization (integration point)
- `tui/src/avatar/sprites.rs` - Frame, Animation, SpriteSheet types
- `tui/src/theme/mod.rs` - Color palette, breathing_color() function
- `tui/src/avatar/sizes.rs` - Example animations

### Project Context
- `agents/CONSTITUTION.md` - Core principles
- `agents/ai-way-docs/terminology-dictionary.md` - User-facing language

---

## Key Decisions Made

### 1. Option B (Pulsing Axolotl Head) Selected
**Reasoning**:
- Perfect balance between minimal and complex
- Reuses all existing sprite infrastructure
- Shows Yollayah personality immediately
- 2-hour implementation (reasonable scope)
- Can upgrade to Option C later without waste
- Breathing metaphor perfect (AI waking up)

### 2. Reuse Existing Sprite System
**Why**:
- Sprite building already proven in `avatar/sizes.rs`
- `breathing_color()` function already written in `theme.rs`
- Lower implementation risk
- Consistent with existing animation style
- No new dependencies

### 3. Target <100ms First Render
**Why**:
- Must appear before Conductor connection (typically 500-5000ms)
- User should never see blank screen
- 10fps animation (100ms per frame) is smooth enough

### 4. Tiny (6x2) Sprite Size
**Why**:
- Works on all terminal sizes (even 80x24 minimum)
- Still cute and recognizable
- Minimal CPU/memory overhead
- Easy to center on screen

---

## Color Palette Reference

**From `tui/src/theme/mod.rs`:**

| Name | Hex | RGB | Use |
|------|-----|-----|-----|
| AXOLOTL_BODY | #FFB6C1 | 255,182,193 | Body/head |
| AXOLOTL_GILLS | #FF7F7F | 255,127,127 | Gills (dim state) |
| AXOLOTL_GILLS_HIGHLIGHT | #FFA0A0 | 255,160,160 | Gills (bright state) |
| AXOLOTL_EYES | #282828 | 40,40,40 | Eyes |
| YOLLAYAH_MAGENTA | #FF8CFF | 255,140,255 | Text (bright) |
| DIM_GRAY | #646464 | 100,100,100 | Text (dim) |

**Animation**:
- Gills oscillate between AXOLOTL_GILLS (dim) and AXOLOTL_GILLS_HIGHLIGHT (bright)
- Text oscillates between DIM_GRAY and YOLLAYAH_MAGENTA
- Both use 1000ms breathing cycle (smooth, not jarring)

---

## Success Criteria Checklist

After implementation, verify:

- [ ] Loading screen appears within 100ms of startup
- [ ] No blank screen visible at any point
- [ ] Axolotl sprite visible and centered
- [ ] Gills have smooth color breathing (no flicker)
- [ ] Eyes blink and show expressions
- [ ] Status messages rotate every 2 seconds
- [ ] "Press Ctrl+C to cancel" is clear
- [ ] Works on 80x24 terminal (minimum size)
- [ ] Works on 120x40 terminal (typical size)
- [ ] CPU usage minimal (<1% on idle system)
- [ ] Memory usage <500KB
- [ ] Code is clean and maintainable
- [ ] Can be extended to Option C later

---

## Timeline Summary

| Phase | Task | Time | Status |
|-------|------|------|--------|
| 1 | Design approval | 1-2 hrs | ✅ Complete |
| 2 | Implement LoadingScreen | 2 hrs | ⏳ Pending |
| 3 | Integrate with app | 2 hrs | ⏳ Pending |
| 4 | Test & verify | 2 hrs | ⏳ Pending |
| **Total** | | **7-8 hrs** | **In Progress** |

---

## Questions? Reviews?

### For Design Feedback
Review: `LOADING-SCREEN-MOCKUPS.txt`

Are the visuals clear? Do the colors match expectations? Is the animation smooth-looking?

### For Technical Feasibility
Review: `DESIGN-loading-screen.md` section "Technical Details"

Does the implementation approach make sense? Are there concerns about using sprites.rs?

### For Implementation Timeline
Review: `LOADING-SCREEN-CODE-TEMPLATE.rs` section "IMPLEMENTATION NOTES"

Does the code structure work with your app architecture? Any blockers?

---

## Next Action

1. **Review** all four design documents (30 minutes total)
2. **Discuss** with team:
   - Is Option B the right choice?
   - Any concerns about implementation approach?
   - Any requirements I missed?
3. **Approve** design and get sign-off
4. **Implement** using LOADING-SCREEN-CODE-TEMPLATE.rs as starting point
5. **Test** on multiple terminal sizes
6. **Integrate** with app startup sequence (STORY 4)

---

**Designer**: UX Specialist
**Approved By**: [Pending]
**Implementation Owner**: [To be assigned]
**Date Completed**: 2026-01-03

**File Structure**:
```
ai-way/
├── LOADING-SCREEN-DESIGN-INDEX.md (this file - navigation)
├── LOADING-SCREEN-SUMMARY.txt (executive summary - 5 min read)
├── LOADING-SCREEN-MOCKUPS.txt (visual designs - 10 min read)
├── DESIGN-loading-screen.md (technical details - 20 min read)
├── LOADING-SCREEN-CODE-TEMPLATE.rs (code reference - implementation)
├── TODO-tui-initialization.md (full project plan)
└── tui/
    └── src/
        ├── loading.rs (to be created - production code)
        ├── app.rs (integration point)
        ├── main.rs (startup entry point)
        ├── avatar/sprites.rs (sprite infrastructure)
        └── theme/mod.rs (color palette)
```

---

*Design Document Package v1.0 - Complete and Ready for Review*
