# TODO: Bash Minimal Fallback Interface

**Created**: 2026-01-03
**Priority**: P1 - Critical Feature
**Status**: 🔵 PROPOSED - Awaiting Planning

---

## Overview

As the TUI becomes more complex and potentially unstable during development, create a minimal bash-based fallback interface that provides core functionality with zero dependencies beyond bash. This ensures Average Joe (AJ) always has a working interface, even if the Rust TUI crashes or fails to compile.

---

## Problem Statement

### Current Situation
- TUI is the only interface
- If TUI crashes or fails, entire application is unusable
- Development iterations can break TUI temporarily
- No graceful degradation path

### Risk Scenario
> "AJ runs yollayah.sh, TUI crashes, sees error message, thinks ai-way is broken, gives up."

### Desired State
- Bash fallback activates automatically on TUI failure
- Provides basic chat functionality
- Includes UX polish (colors, activity indicators, minimal "animations")
- Clear messaging: "Using fallback mode, full experience available when [condition]"

---

## Goals

1. **Automatic fallback** - Detect TUI failure, switch to bash mode
2. **Core functionality** - Chat works, Ollama integration works
3. **UX polish** - Not just plain text, use terminal capabilities
4. **Clear communication** - Explain why fallback mode, how to get full experience
5. **Exit to full mode** - Easy way to retry TUI when available

---

## Scope

### In Scope (Minimal Viable Fallback)
- ✅ Text-based chat interface
- ✅ Ollama integration (reuse existing lib/ollama modules)
- ✅ Color output (ANSI escape codes)
- ✅ Activity indicators (spinner, progress dots)
- ✅ Conversation history (in-session)
- ✅ Graceful exit
- ✅ Clear status messages

### Out of Scope (Not Essential)
- ❌ Avatar animations (requires TUI/graphics)
- ❌ Multi-pane layout
- ❌ Mouse interaction
- ❌ Advanced rendering
- ❌ Persistence across sessions (save/load history)

### Nice to Have (If Time Permits)
- 🟡 Simple "animations" (rotating spinner, pulsing dots)
- 🟡 Keyboard shortcuts (Ctrl+C to exit, Ctrl+L to clear)
- 🟡 Basic Markdown rendering (bold, italic, code blocks)
- 🟡 Color-coded message types (user, assistant, system)

---

## UX Design

### Welcome Screen
```
╔════════════════════════════════════════════════╗
║  🦎 Yollayah - Privacy-First AI Assistant     ║
║                                                ║
║  Running in: FALLBACK MODE                     ║
║  Why: TUI component unavailable                ║
║                                                ║
║  ✅ Chat works                                 ║
║  ✅ Privacy protected                          ║
║  ⚠️  Limited visual features                   ║
║                                                ║
║  To enable full experience:                    ║
║    cargo build --package yollayah-tui         ║
║    ./yollayah.sh                               ║
╚════════════════════════════════════════════════╝

Type your message below (or 'exit' to quit):
>
```

### Chat Interaction
```
You: Hello!

🦎 Thinking... ⠋

🦎 Yollayah: Hi there! I'm Yollayah, your privacy-first AI
assistant running locally on your machine. How can I help
you today?

You: What's the weather like?

🦎 Thinking... ⠸

🦎 Yollayah: I don't have access to real-time weather data,
but I can help you find that information. Would you like me
to suggest some privacy-respecting weather services?

You: _
```

### Activity Indicators
```bash
# Spinner (8 frames, rotating)
⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧

# Dots (pulsing, 4 frames)
.   ..  ... ..  .

# Progress bar (for long operations)
[████░░░░░░] 40%
```

---

## Terminal Minimalism Features

Per UX expert guidance, implement minimal but polished terminal UX:

### Colors (ANSI 256-color palette)
- **User messages**: Cyan (#00D7FF)
- **Assistant messages**: Green (#00FF87)
- **System messages**: Yellow (#FFD700)
- **Errors**: Red (#FF5555)
- **Prompts**: Magenta (#FF79C6)

### Animations
- **Spinner**: Rotate through Braille patterns during thinking
- **Pulsing prompt**: Fade in/out effect on input prompt
- **Loading dots**: Animate dots for Ollama loading

### Box Drawing
Use Unicode box-drawing characters for structure:
```
┌─────────────────┐
│ System Message  │
└─────────────────┘
```

### Text Formatting
- **Bold**: `\033[1m`
- **Italic**: `\033[3m` (if terminal supports)
- **Underline**: `\033[4m`

---

## Technical Architecture

### Module Organization
```
lib/
├── bash-fallback/        # NEW MODULE
│   ├── init.sh           # Initialize fallback mode
│   ├── chat.sh           # Chat loop
│   ├── ui.sh             # UX functions (colors, spinners)
│   ├── input.sh          # User input handling
│   └── renderer.sh       # Message rendering
├── ollama/               # EXISTING - reuse
├── logging/              # EXISTING - reuse
└── ...
```

### Entry Point (yollayah.sh modification)
```bash
# Try to launch TUI
if ! launch_tui; then
    log_warn "TUI unavailable, switching to fallback mode"
    source lib/bash-fallback/init.sh
    bash_fallback_main
fi
```

---

## Implementation Steps

### Phase 1: Basic Chat (Core)
1. Create `lib/bash-fallback/` module structure
2. Implement basic chat loop (read user input → send to Ollama → display response)
3. Reuse existing `lib/ollama/` integration
4. Test with Ollama backend

### Phase 2: UX Polish
1. Add color output for messages
2. Implement spinner during Ollama processing
3. Add welcome screen with fallback mode explanation
4. Clear status messages and error handling

### Phase 3: Terminal Minimalism
1. Implement activity indicators (spinner, dots)
2. Add box drawing for structure
3. Basic text formatting (bold, italic)
4. Polish input prompt (color, cursor positioning)

### Phase 4: Integration & Testing
1. Update yollayah.sh to detect TUI failure
2. Add automatic fallback switching
3. Test failure scenarios (TUI crash, missing binary, etc.)
4. User testing with AJ persona

---

## Fallback Trigger Conditions

When to activate fallback mode:

1. **TUI binary missing**: `tui/target/release/yollayah-tui` doesn't exist
2. **TUI compilation failed**: Build error, binary not executable
3. **TUI crashes on startup**: Exit code != 0 within first 5 seconds
4. **User flag**: `./yollayah.sh --fallback` (manual override)
5. **Terminal incompatible**: No color support, too small, etc.

---

## Configuration

### Environment Variables
- `YOLLAYAH_FALLBACK=1` - Force fallback mode
- `YOLLAYAH_FALLBACK_COLORS=0` - Disable colors (basic terminals)
- `YOLLAYAH_FALLBACK_ANIMATIONS=0` - Disable spinners/animations

### Terminal Requirements
- **Minimum**: 80x24 characters
- **Recommended**: 100x30 characters
- **Color**: 256-color support (fallback to 16-color)

---

## Error Handling

### Graceful Degradation Levels
1. **Full fallback mode** - Colors, animations, box drawing
2. **Basic fallback mode** - No colors, no animations, plain text
3. **Emergency mode** - Minimal output, just chat working

### Error Messages
```
ERROR: Ollama is not running.

Please start Ollama:
  systemctl start ollama  (systemd)
  or
  ollama serve            (manual)

Then restart yollayah.sh
```

---

## Testing Strategy

### Manual Tests
1. Delete TUI binary, verify fallback activates
2. Kill TUI mid-session, verify fallback catches it
3. Test on basic terminal (no color support)
4. Test on small terminal (80x24)
5. Test with slow Ollama responses (verify spinner)

### User Acceptance Tests (AJ Persona)
1. AJ runs yollayah.sh with broken TUI
2. Sees clear message about fallback mode
3. Can chat successfully
4. Understands how to get full experience
5. Not confused or frustrated

---

## Dependencies

- **Blocks**: None
- **Blocked by**: None (can implement independently)
- **Enables**: Reliable fallback when TUI is unstable
- **Related**: TODO-move-bash-fallback-to-bash-module.md (code organization)

---

## Acceptance Criteria

- ✅ Fallback mode activates automatically on TUI failure
- ✅ Basic chat works (user input → Ollama → response)
- ✅ Colors used for message differentiation
- ✅ Spinner/activity indicator during processing
- ✅ Welcome screen explains fallback mode
- ✅ Clear instructions to enable full experience
- ✅ Graceful exit (Ctrl+C, 'exit' command)
- ✅ Works on basic terminals (80x24, 16-color)
- ✅ Code organized in `lib/bash-fallback/` module

---

## Related Documents

- **Architecture**: `knowledge/requirements/REQUIRED-separation.md` - TUI/Conductor separation
- **UX**: Check with UX expert for terminal minimalism guidelines
- **Code Org**: `progress/active/TODO-move-bash-fallback-to-bash-module.md` - Module structure

---

## Notes

- **Keep it simple** - This is a fallback, not a full alternative interface
- **Focus on AJ** - Must be usable by non-technical users
- **Graceful degradation** - Work on any terminal, even basic ones
- **Clear communication** - Don't hide that it's fallback mode, explain why
- **Path to full experience** - Always show how to get TUI working

---

## Open Questions

1. **Should fallback mode have conversation persistence?**
   - Save chat history to file?
   - **Decision**: Out of scope for init, can add later

2. **How to handle TUI recovering mid-session?**
   - Prompt user to switch to TUI?
   - **Proposed**: Stay in fallback for session, mention TUI available on restart

3. **Should we support rich text rendering (Markdown)?**
   - Code blocks, lists, bold, italic?
   - **Decision**: Nice to have, implement if time permits

---

**Next Steps**: Review design with UX expert, create lib/bash-fallback/ module structure, implement Phase 1 (basic chat).
