# TODO-yollayah.sh-improvements

**Status**: 🟡 ACTIVE - Quality of Life Improvements
**Created**: 2026-01-03
**Priority**: P2 - Nice to Have
**Component**: yollayah.sh (entry point and fallback chat)

---

## Navigation

**Parent**: [TODO-AI-WAY.md](TODO-AI-WAY.md)
**Related**:
- `yollayah.sh` - Main entry point
- `lib/ux/fallback_chat.sh` - Fallback chat implementation

---

## Problem Statement

The `--interactive` mode and fallback chat work functionally, but lack quality-of-life features that would improve the user experience.

---

## Active Tasks

### 🔴 TASK-000: CRITICAL - Investigate slow --interactive performance

**Status**: BLOCKED - Awaiting user testing
**Priority**: P0 - BLOCKING
**Bug**: [TODO-BUG-007-interactive-mode-slow.md](TODO-BUG-007-interactive-mode-slow.md)

**Problem**: Direct `ollama run` is fast (GPU), but `./yollayah.sh --interactive` is slow, even though both use `ollama run`.

**This makes no sense** - investigating root cause.

**Next Steps**:
- User needs to run diagnostic tests (see BUG-007)
- Compare timing: direct ollama vs --interactive
- Verify OLLAMA_KEEP_ALIVE is working
- Check if same model being used

---

### ✅ TASK-001: Add Ctrl+C and Esc support to fallback chat

**Status**: ✅ COMPLETE
**Priority**: P1 - Should Have

**Problem**: User has no way to gracefully exit the fallback chat loop without killing the terminal.

**Solution**: Add signal handlers for Ctrl+C (SIGINT) and Esc key detection.

**Implementation**:
- Add trap for SIGINT in fallback_chat.sh
- Detect Esc key in read loop
- Display farewell message on exit
- Clean up properly before terminating

**Acceptance Criteria**:
- [ ] Ctrl+C exits gracefully with farewell message
- [ ] Esc key exits gracefully
- [ ] No orphaned processes after exit
- [ ] Terminal state restored properly

---

## Nice to Have Improvements

### Input Enhancements

**IDEA-001: Multi-line input support**
- Allow Shift+Enter or Alt+Enter for newlines
- Submit on bare Enter
- Visual indicator for multi-line mode

**IDEA-002: Input history with arrow keys**
- Up/Down arrows navigate previous prompts
- Persist history across sessions (~/.yollayah/history)
- Ctrl+R for reverse search

**IDEA-003: Tab completion**
- Complete common phrases
- Model names
- File paths when referencing code

**IDEA-004: Input editing**
- Ctrl+A/E for beginning/end of line
- Ctrl+K/U for kill line
- Ctrl+W for delete word

**IDEA-005: Paste detection**
- Auto-detect large pastes
- Confirm before sending
- Prevent accidental submission of clipboard

### Output Enhancements

**IDEA-006: Streaming indicators**
- Show "thinking..." animation
- Token/sec counter during streaming
- Progress bar for long responses

**IDEA-007: Markdown rendering**
- Syntax highlighting for code blocks
- Bold/italic formatting
- Clickable links (if terminal supports)

**IDEA-008: Code block extraction**
- Auto-detect code blocks in response
- Offer to save to file
- Copy to clipboard option

**IDEA-009: Response metadata**
- Show token count
- Model used
- Response time
- Temperature/settings used

**IDEA-010: Scroll support**
- Paginate long responses
- Less/more style navigation
- Search within responses

### Conversation Management

**IDEA-011: Save conversation**
- Export to markdown
- Save to timestamped file
- Resume previous conversations

**IDEA-012: Clear screen command**
- `/clear` or Ctrl+L
- Keep conversation history
- Just clear display

**IDEA-013: Conversation branching**
- Save checkpoint
- Try different prompts
- Return to checkpoint

**IDEA-014: Multi-turn context**
- Show conversation summary
- Indicate context window usage
- Warn when approaching limit

### Session Management

**IDEA-015: Model switching**
- `/model <name>` command
- List available models
- Show current model in prompt

**IDEA-016: Settings adjustment**
- `/temp <value>` for temperature
- `/system <prompt>` for system message
- `/max <tokens>` for max tokens

**IDEA-017: Help system**
- `/help` or `?` for commands
- Show keyboard shortcuts
- Display tips periodically

**IDEA-018: Session statistics**
- Total tokens used
- Models used
- Time elapsed
- Cost estimate (if using paid API)

### Error Handling

**IDEA-019: Better error messages**
- Suggest solutions for common errors
- Show debug info on request
- Offer to restart conductor

**IDEA-020: Automatic retry**
- Retry on network errors
- Exponential backoff
- User confirmation for retries

**IDEA-021: Graceful degradation**
- Fall back to smaller model on OOM
- Reduce context on token limit
- Warn before failing

### Performance

**IDEA-022: Response caching**
- Cache identical prompts
- Show "(cached)" indicator
- Clear cache command

**IDEA-023: Lazy loading**
- Load conductor on first prompt
- Not on startup
- Faster initial launch

**IDEA-024: Background preloading**
- Warm up model in background
- After first response
- Ready for next prompt instantly

### User Experience

**IDEA-025: Welcome message**
- Show tips on first launch
- Explain commands
- Show example prompts

**IDEA-026: Prompt suggestions**
- Show random example on empty input
- Context-aware suggestions
- Based on conversation history

**IDEA-027: Typing indicators**
- Show when conductor is processing
- Animated ellipsis
- Cancel with Ctrl+C

**IDEA-028: Color themes**
- Light/dark mode toggle
- Custom color schemes
- Respect terminal colors

**IDEA-029: Sound effects (optional)**
- Notification on response complete
- Error sound
- Configurable/disable

**IDEA-030: Accessibility**
- Screen reader support
- High contrast mode
- Font size adjustment

### Integration

**IDEA-031: Pipe support**
- `echo "prompt" | ./yollayah.sh --interactive`
- Read from stdin
- Output to stdout for scripting

**IDEA-032: File input**
- `./yollayah.sh --interactive < prompts.txt`
- Process batch prompts
- Output results

**IDEA-033: Environment variables**
- `YOLLAYAH_MODEL` override
- `YOLLAYAH_TEMP` override
- `YOLLAYAH_PROMPT` for scripting

**IDEA-034: JSON output mode**
- `--json` flag
- Structured output
- For automation/scripting

### Testing

**IDEA-035: Replay mode**
- Record session to file
- Replay for testing
- Verify outputs match

**IDEA-036: Mock mode**
- Use fake responses
- No actual LLM calls
- For UI testing

**IDEA-037: Benchmark mode**
- Measure token/sec
- Test different models
- Compare performance

### Documentation

**IDEA-038: Inline help**
- `/help <command>` for details
- Show examples
- Link to docs

**IDEA-039: Tutorial mode**
- Interactive walkthrough
- Show features one by one
- Practice commands

**IDEA-040: Tips and tricks**
- Show random tip on startup
- Daily tip rotation
- User can disable

---

## Implementation Priority

### P0 - Critical (Blocking UX)
- TASK-001: Ctrl+C and Esc support (✅ In Progress)

### P1 - Should Have (Next Sprint)
- IDEA-002: Input history
- IDEA-007: Markdown rendering
- IDEA-015: Model switching
- IDEA-017: Help system

### P2 - Nice to Have (Future)
- All other IDEA-* items

### P3 - Polish (Anytime)
- Color themes
- Sound effects
- Accessibility features

---

## Success Criteria

### Usability
- User can exit gracefully (Ctrl+C, Esc)
- User can navigate history (arrow keys)
- User can edit input (readline shortcuts)
- User can get help (/help)

### Polish
- Responses feel instant (streaming)
- Errors are helpful (suggest solutions)
- Interface is beautiful (colors, formatting)
- Features are discoverable (hints, tips)

---

## Notes

- Focus on **terminal-native** experience (not trying to be a web UI)
- **Simplicity first** - don't over-engineer
- **Progressive enhancement** - each feature should be optional
- **Respect user's terminal** - colors, settings, capabilities

---

## Related Issues

- BUG-006: Streaming now reactive (should improve response feel)
- TODO-bash-minimal-fallback.md: Core fallback implementation

---

**Remember**: yollayah.sh is the **first impression**. Make it delightful! ✨
