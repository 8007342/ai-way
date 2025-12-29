# Rich UX TODO

Tracking progress on the enhanced terminal UI for Yollayah.

## Research: TUI Libraries

### Recommendation: **Textual** (Python)

For our requirements, **Textual** is the best fit:
- Independent scrollable regions (built-in)
- CSS-like styling (grayed out text trivial)
- Absolute positioning (supported)
- Animation support (refresh cycles, reactive updates)
- Modern async architecture
- Works over SSH, in containers, everywhere

### Library Comparison

| Feature | ncurses | blessed | urwid | Textual | rich |
|---------|---------|---------|-------|---------|------|
| Independent scroll regions | Manual | Manual | Yes | Yes | No |
| Absolute positioning | Yes | Yes | Limited | Yes | No |
| Animation/refresh | Manual | Yes | Yes | Yes | Limited |
| Modern Python async | No | No | Partial | Yes | Yes |
| CSS-like styling | No | No | No | Yes | Yes |
| Mouse support | Manual | Yes | Yes | Yes | No |
| Complexity | High | Medium | Medium | Low | Low |

### Why Not Pure ncurses?

ncurses is C-based and while Python has a `curses` module, it's:
- Very low-level (manual window management)
- No widgets out of the box
- Error-prone for complex layouts
- Hard to maintain

Textual is built ON TOP of rich (same author), so we get:
- Rich text rendering
- Markdown support
- Syntax highlighting
- Plus full TUI capabilities

### Architecture Decision

```
yollayah.sh (bash)          # Bootstrap, system ops, ollama management
    │
    └──▶ surfaces/terminal/  # Python Textual app
         ├── app.py          # Main TUI application
         ├── widgets/        # Custom widgets
         │   ├── conversation.py   # Scrollable chat
         │   ├── pinned.py         # Pinned question header
         │   ├── status.py         # Glowing status indicator
         │   └── axolotl.py        # ASCII art animations
         └── styles/
             └── yollayah.tcss     # Textual CSS
```

The bash bootstrap handles:
- Ollama service management
- Integrity verification
- Environment setup
- Launching the Python surface

---

## Phase 1: Foundation

- [ ] Add Textual to dependencies (requirements.txt or pyproject.toml)
- [ ] Create surfaces/terminal/ directory structure
- [ ] Basic App class with placeholder layout
- [ ] Wire yollayah.sh to launch Python surface after setup
- [ ] Verify it works with existing ollama connection

## Phase 2: Core Layout

- [ ] Pinned header widget (shows current question, grayed styling)
- [ ] Main conversation scroll region
- [ ] Mouse wheel detection per-region
- [ ] Status bar at bottom

## Phase 3: Status Indicators

- [ ] Absolute positioned status icon (top-right corner)
- [ ] Glowing/pulsing effect for "thinking" state
- [ ] States: idle, thinking, waiting-for-input, error

## Phase 4: Axolotl ASCII Art

- [ ] Design blocky axolotl frames (8x8 or similar)
- [ ] Loading animation (axolotl swimming)
- [ ] Thinking animation (axolotl with thought bubble)
- [ ] Prompting animation (axolotl looking curious)
- [ ] Success animation (happy axolotl)
- [ ] Error animation (confused axolotl)

### Axolotl Art Concepts

```
Idle (simple):
  ◖●_●◗
   └┴┘

Swimming frame 1:        Swimming frame 2:
  ◖●_●◗ ~~~             ~~~ ◖●_●◗
   └┴┘                       └┴┘

Thinking:
    ?
  ◖●_●◗
   └┴┘

Happy:
  ◖^_^◗ !
   └┴┘
```

More detailed blocky version (for loading screen):
```
    ██████
   ██◖●_●◗██
  ████████████
   ██ ████ ██
    ~~    ~~
```

## Phase 5: Developer Mode (PJ)

- [ ] Split pane showing routing decisions
- [ ] Token count display
- [ ] Agent handoff visualization
- [ ] Context window usage meter
- [ ] Toggle with /debug command

## Phase 6: Polish

- [ ] Smooth scrolling
- [ ] Keyboard shortcuts (Ctrl+C graceful exit, etc.)
- [ ] History navigation (up/down arrows)
- [ ] Resize handling
- [ ] Color theme respects terminal palette

---

## Design Principles

1. **AJ sees simplicity**: Clean, minimal, no clutter
2. **PJ sees internals**: /debug reveals everything
3. **Yollayah floats free**: Not confined to boxes, personality shines
4. **Blocky consistency**: All ASCII art uses same blocky style
5. **Mouse-friendly**: Scroll regions intuitive, no instructions needed
6. **Keyboard-friendly**: Everything works without mouse too

---

## Dependencies

```
# requirements.txt additions
textual>=0.40.0
rich>=13.0.0
```

## References

- Textual docs: https://textual.textualize.io/
- Rich docs: https://rich.readthedocs.io/
- Textual CSS: https://textual.textualize.io/guide/CSS/
- ASCII art inspiration: https://www.asciiart.eu/

---

## Constitution Reference

- **Law of Care**: UI should be pleasant, not stressful
- **Law of Truth**: Status indicators are honest (thinking = actually thinking)
- **Law of Elevation**: PJ mode helps developers grow their understanding
