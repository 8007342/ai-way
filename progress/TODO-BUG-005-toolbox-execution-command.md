# BUG-002: Toolbox Execution Uses Enter Instead of Run

**Severity**: 🔴 CRITICAL
**Status**: ✅ FIXED
**Discovered**: 2026-01-03 (User Report)
**Fixed**: 2026-01-03
**Reporter**: User (machiyotl)
**Fixed By**: Claude Code (Architect + Developer)

---

## Summary

The auto-enter wrapper used `toolbox enter` which spawns an interactive shell, instead of `toolbox run` which executes a command and exits. This caused users to get stuck in an empty shell prompt instead of the script continuing execution.

---

## Impact

**User Flow Affected**: Primary startup flow on Fedora Silverblue
**Severity**: CRITICAL - Completely blocks automated execution

**What the user experienced**:
1. Run `./yollayah.sh` from host
2. See "🔧 Entering ai-way toolbox container..."
3. Get dropped into interactive shell prompt (ai-way)
4. Script never re-executes inside container
5. `$INSIDE_TOOLBOX` is empty (script didn't run)
6. User confused about what to do next

---

## Root Cause

**Files**: `/var/home/machiyotl/src/ai-way/yollayah.sh`
**Lines**: 103, 133

**Incorrect Command**:
```bash
exec toolbox enter ai-way -- "$SCRIPT_DIR/yollayah.sh" "$@"
```

**Why it failed**:
- `toolbox enter [CONTAINER]` spawns an **interactive shell**
- Does **NOT** accept command arguments
- The `-- "$SCRIPT_DIR/yollayah.sh" "$@"` part is **silently ignored**
- User gets dropped into bash prompt instead of script executing
- This is the documented behavior of `toolbox enter`

**Key distinction**:
- `toolbox enter CONTAINER` → Interactive shell (for manual use)
- `toolbox run CONTAINER COMMAND` → Execute command and exit (for automation)

---

## Fix Applied

**Changed from**:
```bash
exec toolbox enter ai-way -- "$SCRIPT_DIR/yollayah.sh" "$@"
```

**Changed to**:
```bash
exec toolbox run -c ai-way "$SCRIPT_DIR/yollayah.sh" "$@"
```

**Why this works**:
- `toolbox run [OPTIONS] CONTAINER COMMAND` executes command in container
- `-c CONTAINER` explicitly specifies container name
- Command and arguments follow directly (no `--` separator needed)
- Executes script inside container and exits back to host prompt
- Uses `exec` to replace process (clean exit behavior)

---

## Verification Tests

All tests passed after fix:

### Test 1: Existing Toolbox Auto-Enter ✅
```bash
./yollayah.sh --help
```
**Result**:
- Shows "🔧 Entering ai-way toolbox container..."
- Help text displays correctly
- Returns to host prompt (NOT interactive shell)

### Test 2: Inside Toolbox (No Re-Entry) ✅
```bash
toolbox run -c ai-way /var/home/machiyotl/src/ai-way/yollayah.sh --help
```
**Result**:
- NO "Entering" message
- Help displays immediately
- Correct early return logic

### Test 3: Argument Preservation ✅
```bash
./yollayah.sh status
```
**Result**:
- `status` command preserved through wrapper
- Executes inside toolbox correctly
- Shows daemon status output

---

## Related Issues

- **BUG-001**: Toolbox detection grep pattern (separate issue, already fixed in line 79)
- **TODO-sprint-toolbox-5-bugfixes.md**: Sprint tracking this fix
- **TODO-epic-2026Q1-toolbox.md**: Parent epic (Sprint 5 added)

---

## Lessons Learned

1. **Read the manual**: `toolbox enter` vs `toolbox run` distinction is documented
2. **Test the happy path**: Should have manually tested `./yollayah.sh` before marking complete
3. **User testing is critical**: QA didn't catch this, user did
4. **Document command patterns**: Add to CLAUDE.md for future reference

---

## Files Modified

| File | Lines | Change |
|------|-------|--------|
| `yollayah.sh` | 103 | `toolbox enter ai-way --` → `toolbox run -c ai-way` |
| `yollayah.sh` | 133 | `toolbox enter ai-way --` → `toolbox run -c ai-way` |

---

**Created**: 2026-01-03
**Last Updated**: 2026-01-03
**Status**: RESOLVED ✅
**Next Action**: Update documentation examples in TODO files
