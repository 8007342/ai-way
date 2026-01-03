# TODO: Sprites System - Initial Implementation

**Created**: 2026-01-03
**Priority**: P2 - Feature Development
**Status**: 🔵 PROPOSED - Awaiting Planning

---

## Overview

Create a minimal sprite animation system to test the graphics pipeline for Yollayah's animated avatar. This is the foundational work for mood-based animations and the evolved personality system.

---

## Goals

1. **Define sprite format** - Choose format (PNG sequence, sprite sheets, animated GIF, etc.)
2. **Create test sprites** - Generate 2-3 simple animations for testing
3. **Implement sprite loader** - Read and parse sprite files
4. **Test rendering pipeline** - Verify sprites display correctly in TUI

---

## Scope

### In Scope
- Basic sprite file format selection
- Minimal test animations (3 frames each)
- Simple sprite loader in Rust
- Terminal rendering verification

### Out of Scope
- Complex animations (save for mood system)
- Sprite editor/tooling
- Performance optimization
- Caching layer (see TODO-animation-cache-init.md)

---

## Technical Considerations

### Format Options
1. **PNG Sequence** - Individual PNG files per frame
   - ✅ Simple to generate and modify
   - ✅ No additional parsing needed
   - ❌ Many small files

2. **Sprite Sheet** - Single image with frames in grid
   - ✅ Single file per animation
   - ✅ Efficient for disk usage
   - ❌ Requires additional parsing logic

3. **Animated Format** - GIF, APNG, WebP
   - ✅ Self-contained animation
   - ❌ Complex parsing requirements
   - ❌ Limited by terminal capabilities

**Recommendation**: Start with PNG sequence for simplicity, migrate to sprite sheets if needed.

### Terminal Rendering

Terminal graphics options:
1. **ASCII Art** - Convert sprite to ASCII characters
2. **Block Characters** - Use Unicode box drawing
3. **True Color** - Use 24-bit RGB terminal escape codes
4. **Sixel Graphics** - Terminal graphics protocol (limited support)
5. **Kitty Graphics Protocol** - Modern terminal graphics (limited support)

**Recommendation**: Start with true color block characters for widest compatibility.

---

## Test Animations

### Animation 1: Idle Breathing
- 3 frames showing subtle axolotl breathing motion
- Loop duration: ~2 seconds
- Purpose: Default "waiting" state

### Animation 2: Thinking
- 3 frames showing thoughtful expression
- Loop duration: ~1.5 seconds
- Purpose: Active processing indicator

### Animation 3: Happy
- 3 frames showing excited/happy expression
- Loop duration: ~1 second
- Purpose: Successful completion state

---

## Implementation Steps

1. **Research & Decision** (1-2 hours)
   - Research terminal graphics capabilities
   - Decide on sprite format
   - Document format specification

2. **Test Asset Creation** (2-3 hours)
   - Create 3 simple test animations
   - Export in chosen format
   - Verify file structure

3. **Sprite Loader** (3-4 hours)
   - Implement file reading
   - Parse sprite format
   - Basic error handling

4. **TUI Integration** (2-3 hours)
   - Add sprite rendering to TUI
   - Test in different terminal emulators
   - Verify frame timing

5. **Documentation** (1 hour)
   - Document sprite format specification
   - Add usage examples
   - Create developer guide

---

## Dependencies

- **Blocks**: None (foundational work)
- **Blocked by**: None
- **Enables**:
  - TODO-coherent-evolving-mood-system-init.md (mood-based animations)
  - TODO-animation-cache-init.md (performance optimization)

---

## Acceptance Criteria

- ✅ Sprite format is documented and chosen
- ✅ 3 test animations created and loadable
- ✅ Sprites render correctly in terminal
- ✅ Frame timing works as expected
- ✅ Developer documentation exists

---

## Related Documents

- **Design**: `facts/design/yollayah-avatar-constraints.md` - Avatar design constraints
- **Epic**: `progress/active/TODO-epic-2026Q1-avatar-animation.md` - Avatar animation roadmap
- **Future**: `progress/active/TODO-coherent-evolving-mood-system-init.md` - Mood system

---

## Notes

- Keep it simple! This is exploratory work to validate the pipeline
- Don't optimize prematurely - performance comes later with caching
- Focus on developer experience - easy to add new sprites
- Consider AJ's terminal capabilities (basic terminals, not bleeding edge)

---

**Next Steps**: Review this proposal, assign to developer, move to TODO-epic-2026Q1-avatar-animation.md when ready to implement.
