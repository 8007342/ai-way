# TUI Launch Bug Investigation

**Date**: 2026-01-03
**Status**: RESOLVED
**Issue**: TUI doesn't launch - script silently exits after GPU message

## Root Cause

Incorrect TUI path in `yollayah/lib/ux/terminal.sh`:

```bash
# WRONG:
TUI_DIR="${SCRIPT_DIR}/tui"

# CORRECT:
TUI_DIR="${SCRIPT_DIR}/yollayah/core/surfaces/tui"
```

## Why Silent Exit?

1. `ux_tui_ensure_ready()` checks if `$TUI_DIR/src` exists  
2. With wrong path, directory doesn't exist
3. Function returns 1
4. With `set -e`, script exits immediately
5. No error shown - just silent return to prompt

## Fix

Changed line 437 in `yollayah/lib/ux/terminal.sh`:
```bash
TUI_DIR="${SCRIPT_DIR}/yollayah/core/surfaces/tui"
```

## Verification

After fix, script now:
- ✅ Finds TUI source directory
- ✅ Launches TUI in interactive terminal
- ✅ Falls back gracefully to bash if no TTY
- ✅ No more silent exits

## Debug Method

Systematic checkpoint debugging revealed exact failure point:
1. Checkpoints in `main()` → reached interface launch
2. Checkpoints in `ux_start_interface()` → stopped at `ux_tui_ensure_ready()`  
3. Investigated directory check → path mismatch discovered

## Lessons

1. `set -e` makes debugging harder - always return 0 explicitly
2. Path config must match actual directory structure
3. Document directory layout clearly (see CLAUDE.md)
4. Debug checkpoints with `echo >&2` are invaluable
