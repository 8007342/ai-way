# BUG-016: Config Test Failure in test_parse_partial_toml

**Created**: 2026-01-03
**Severity**: 🟡 **HIGH**
**Priority**: P1 - Should Fix Soon
**Status**: 🔴 IDENTIFIED - Blocking Pre-Commit Hook

---

## Problem Statement

The test `config::tests::test_parse_partial_toml` is failing with an assertion mismatch. This test failure is pre-existing and unrelated to recent performance work, but it blocks the pre-commit hook.

**Impact**:
- Blocks all commits that use the pre-commit hook
- Must use `git commit --no-verify` to bypass
- May indicate config parsing regression

---

## Test Failure Details

**Location**: `conductor/core/src/config/mod.rs:785` (test function)

**Error**:
```
---- config::tests::test_parse_partial_toml stdout ----
assertion `left == right` failed
  left: 3000
 right: 7500
```

**Test Name**: `test_parse_partial_toml`

---

## Root Cause Analysis

**Status**: Not yet investigated

**Possible Causes**:
1. Default value changed but test not updated
2. Config parsing logic changed
3. TOML parsing behavior changed
4. Test itself has incorrect expectations

**Next Steps**:
1. Read the test code to understand what it's testing
2. Identify which config field is producing 3000 vs 7500
3. Determine if 3000 or 7500 is the correct value
4. Fix either the code or the test

---

## Required Fixes

### Investigation Required

1. **Read test code** (`conductor/core/src/config/mod.rs:785`)
2. **Identify the field** being tested (likely a numeric config value)
3. **Trace the default value** in config structs
4. **Determine correct behavior** (is 3000 or 7500 right?)
5. **Fix accordingly**

---

## Acceptance Criteria

- [ ] Test `config::tests::test_parse_partial_toml` passes
- [ ] Pre-commit hook succeeds without `--no-verify`
- [ ] Config parsing works correctly
- [ ] All other config tests still pass

---

## Related Documents

- **Test File**: `conductor/core/src/config/mod.rs`
- **Blocking**: Pre-commit hook in git hooks

---

## Timeline

- **Identified**: 2026-01-03
- **Fix Target**: Sprint 9
- **Blocking**: Pre-commit hook (workaround: `--no-verify`)

**This is a HIGH priority bug that should be fixed to restore normal git workflow.**
