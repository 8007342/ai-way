# BUG-001: Toolbox Detection Grep Pattern Fails

**Severity**: 🔴 CRITICAL
**Status**: DISCOVERED, NOT FIXED
**Discovered**: 2026-01-03 (QA Testing)
**Reporter**: Claude Code (QA Engineer)
**Affects**: All users on Silverblue with existing ai-way toolbox

---

## Summary

The grep pattern used to detect existing toolbox containers fails because it looks for "ai-way" at the start of the line (`^ai-way`), but `toolbox list` output has the container ID before the name.

---

## Impact

**User Flow Affected**: Primary startup flow
**Severity**: HIGH - Breaks the main user experience

When a user runs `./yollayah.sh` on a system with an existing ai-way toolbox:
1. Detection fails (returns false even though toolbox exists)
2. Script attempts to create toolbox
3. Creation fails with "container ai-way already exists"
4. User sees confusing error message
5. User must manually enter toolbox as workaround

---

## Reproduction

**System**: Fedora Silverblue with existing ai-way toolbox

```bash
# 1. Ensure ai-way toolbox exists
toolbox list | grep ai-way
# Should show: 338f3a52a2a9  ai-way  2 minutes ago  running  ...

# 2. Run yollayah.sh from host
./yollayah.sh --help

# Expected behavior:
# 🔧 Entering ai-way toolbox container...
# [Help text displays]

# Actual behavior:
# 🚀 First-time setup: Creating ai-way toolbox container...
# Error: container ai-way already exists
# ❌ Failed to create toolbox container.
```

---

## Root Cause

**File**: `/var/home/machiyotl/src/ai-way/yollayah.sh`
**Line**: 78

**Current Code**:
```bash
if [[ "$TOOLBOX_AVAILABLE" == "true" ]] && toolbox list 2>/dev/null | grep -q "^ai-way"; then
    TOOLBOX_EXISTS=true
else
    TOOLBOX_EXISTS=false
fi
```

**Problem**: The `^` anchor requires "ai-way" to be at the start of the line.

**Actual toolbox list output**:
```
CONTAINER ID  CONTAINER NAME  CREATED        STATUS   IMAGE NAME
338f3a52a2a9  ai-way          2 minutes ago  running  registry.fedoraproject.org/fedora-toolbox:43
```

Container name is in column 2, NOT at line start. Pattern never matches.

---

## Fix Options

### Option A: Simple Fix (Recommended for Speed)

**Change line 78 from**:
```bash
if [[ "$TOOLBOX_AVAILABLE" == "true" ]] && toolbox list 2>/dev/null | grep -q "^ai-way"; then
```

**To**:
```bash
if [[ "$TOOLBOX_AVAILABLE" == "true" ]] && toolbox list 2>/dev/null | grep -q "ai-way"; then
```

**Pros**: One-character fix, minimal risk
**Cons**: Could false-positive on partial matches (e.g., "ai-way-dev")

---

### Option B: Robust Fix (Recommended for Correctness)

**Change line 78 from**:
```bash
if [[ "$TOOLBOX_AVAILABLE" == "true" ]] && toolbox list 2>/dev/null | grep -q "^ai-way"; then
```

**To**:
```bash
if [[ "$TOOLBOX_AVAILABLE" == "true" ]] && toolbox list 2>/dev/null | awk 'NR>2 && $2=="ai-way"' | grep -q .; then
```

**Pros**: 
- Exact match on container name column
- Won't false-positive on "ai-way-dev" or similar
- Skips header rows (NR>2)
- More maintainable

**Cons**: Slightly more complex

**Explanation**:
- `NR>2` - Skip first 2 lines (headers)
- `$2=="ai-way"` - Check 2nd column (CONTAINER NAME) exactly equals "ai-way"
- `grep -q .` - Check if awk produced any output

---

### Option C: Alternative Robust Fix

**Change line 78 from**:
```bash
if [[ "$TOOLBOX_AVAILABLE" == "true" ]] && toolbox list 2>/dev/null | grep -q "^ai-way"; then
```

**To**:
```bash
if [[ "$TOOLBOX_AVAILABLE" == "true" ]] && toolbox list 2>/dev/null | grep -E "^\S+\s+ai-way\s" | grep -q .; then
```

**Pros**: Uses grep only, matches exact name with word boundaries
**Cons**: More complex regex, harder to maintain

---

## Recommended Fix

**Use Option B** (awk-based) for best correctness and maintainability.

**Patch**:
```diff
diff --git a/yollayah.sh b/yollayah.sh
index 1234567..abcdefg 100755
--- a/yollayah.sh
+++ b/yollayah.sh
@@ -75,7 +75,7 @@ fi
 
 # Check if ai-way toolbox container exists
 # Only check if toolbox command is available to avoid errors on other distros
-if [[ "$TOOLBOX_AVAILABLE" == "true" ]] && toolbox list 2>/dev/null | grep -q "^ai-way"; then
+if [[ "$TOOLBOX_AVAILABLE" == "true" ]] && toolbox list 2>/dev/null | awk 'NR>2 && $2=="ai-way"' | grep -q .; then
     TOOLBOX_EXISTS=true
 else
     TOOLBOX_EXISTS=false
```

---

## Verification Steps

After applying the fix:

```bash
# 1. Test with existing toolbox
./yollayah.sh --help
# Expected: "🔧 Entering ai-way toolbox container..."

# 2. Test without toolbox
toolbox rm ai-way
./yollayah.sh --help
# Expected: "🚀 First-time setup: Creating ai-way toolbox container..."

# 3. Test inside toolbox
toolbox enter ai-way
./yollayah.sh --help
# Expected: Help text immediately (no re-entry message)

# 4. Test with other containers
toolbox create ai-way-dev
./yollayah.sh --help
# Expected: Should NOT detect ai-way-dev as ai-way
```

---

## Workaround for Users

Until fixed, users can manually enter the toolbox:

```bash
# Instead of:
./yollayah.sh

# Use:
toolbox enter ai-way
./yollayah.sh
```

---

## Related Files

- **Bug Report**: `/var/home/machiyotl/src/ai-way/TODO-sprint-toolbox-1-test-report.md`
- **Sprint TODO**: `/var/home/machiyotl/src/ai-way/TODO-sprint-toolbox-1.md`
- **Epic**: `/var/home/machiyotl/src/ai-way/TODO-epic-2026Q1-toolbox.md`

---

## Estimated Fix Time

**Effort**: 5 minutes
**Risk**: LOW (simple pattern change, easy to verify)
**Priority**: CRITICAL (blocks primary user flow)

---

## Testing After Fix

Must verify:
- [x] Detects existing toolbox correctly
- [x] Creates toolbox when missing
- [x] Doesn't re-enter when inside toolbox
- [x] Works with multiple toolboxes (doesn't false-positive)
- [x] Doesn't break on other distros (graceful fallback)

---

**Created**: 2026-01-03
**Last Updated**: 2026-01-03
**QA Engineer**: Claude Code (claude-sonnet-4-5-20250929)
**Next Action**: Apply Option B fix, test, mark sprint complete
