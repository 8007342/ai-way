# TODO-BUG-008-Remove-Ollama-Overrides

**Status**: ✅ FIXED
**Created**: 2026-01-03
**Priority**: P0 - CRITICAL - Performance Regression
**Severity**: CRITICAL - Breaking GPU acceleration

---

## Problem Statement

**User Report**: "--interactive launches quickly, but responses take a while to load and stream slowly. Direct `ollama run` with 8B params runs immediately, one after another."

**Root Cause**: We were overriding `LD_LIBRARY_PATH` when starting `ollama serve`, interfering with Ollama's ability to properly use GPU libraries.

---

## Investigation

### Symptoms

1. ✅ Startup fast (warmup fix worked)
2. ❌ Responses slow in --interactive mode
3. ✅ Direct `ollama run` from terminal = instant (GPU working)
4. ❌ Same model through yollayah = slow

**Conclusion**: Our custom environment setup was breaking Ollama's GPU access.

### Root Cause Found

**Location**: `yollayah/lib/ollama/service.sh:332-334`

**Problematic Code**:
```bash
# WRONG - Overriding library paths
LD_LIBRARY_PATH="/usr/lib:/usr/lib64:${LD_LIBRARY_PATH:-}" ollama serve 2>&1 | _ollama_filter_output &
LD_LIBRARY_PATH="/usr/lib:/usr/lib64:${LD_LIBRARY_PATH:-}" ollama serve > /dev/null 2>&1 &
```

**Why This Breaks**:
- Ollama already knows how to find its CUDA/ROCm libraries
- Forcing `/usr/lib:/usr/lib64` can cause library conflicts
- May prevent Ollama from finding correct GPU libraries
- Results in CPU fallback or degraded performance

---

## Fix Applied

### Changes

**File**: `yollayah/lib/ollama/service.sh`

**Before** (BROKEN):
```bash
LD_LIBRARY_PATH="/usr/lib:/usr/lib64:${LD_LIBRARY_PATH:-}" ollama serve > /dev/null 2>&1 &
log_ollama "INFO" "Started ollama serve (PID: $pid) with CUDA library path"
```

**After** (FIXED):
```bash
ollama serve > /dev/null 2>&1 &
log_ollama "INFO" "Started ollama serve (PID: $pid) using default configuration"
```

### What We Kept

- ✅ `OLLAMA_KEEP_ALIVE=24h` - CRITICAL for performance
- ✅ GPU detection for informational/verbose messages
- ✅ Filtered output in verbose mode (GPU diagnostics)
- ✅ Background process management
- ✅ API health check / wait for ready

### What We Removed

- ❌ `LD_LIBRARY_PATH` override
- ❌ Custom library path injection
- ❌ "CUDA library path" messaging (misleading)

---

## Philosophy Change

**Old Approach**: "We know better, configure everything"
- Set custom library paths
- Override Ollama's defaults
- Try to help Ollama find libraries

**New Approach**: "Trust Ollama, use defaults"
- Let Ollama handle its own library detection
- Only configure what we need (KEEP_ALIVE)
- Don't fight the tool's built-in logic

**Principle**: "Configuration should enhance, not replace, built-in functionality"

---

## Testing

### Before Fix

```bash
./yollayah.sh --interactive
# Type: "Write a story"
# Result: Slow response, sluggish streaming
# Behavior: Feels like CPU, not GPU
```

### After Fix

```bash
./yollayah.sh --interactive
# Type: "Write a story"
# Expected: Instant response, smooth streaming
# Expected: Matches direct `ollama run` performance
```

### Verification

```bash
# Should be identical performance:
ollama run llama3.2:3b "test"     # Direct
./yollayah.sh --interactive       # Through yollayah
# Type: "test"

# Both should be instant with GPU
```

---

## Related Issues

**Similar Philosophy**:
- BUG-007: --interactive performance (parent issue)
- EPIC Reactive Refactor: Remove blocking, trust async

**Pattern**: When performance is bad, check if we're fighting the tool

---

## Success Criteria

- [x] Removed `LD_LIBRARY_PATH` override
- [x] Ollama uses default configuration
- [x] Kept `OLLAMA_KEEP_ALIVE` setting
- [x] Kept GPU detection for verbose output
- [x] Code is simpler and cleaner
- [⚠️] Performance matches direct ollama (needs user testing)

---

## Lessons Learned

1. **Don't override unless necessary**: Ollama knows its environment better than we do
2. **Test against baseline**: Direct `ollama run` should match our wrapped version
3. **Simplicity wins**: Fewer environment tweaks = fewer chances to break things
4. **Trust the tool**: Well-designed tools have good defaults

---

**The golden rule**: If it works directly but breaks through our wrapper, we're doing something wrong.
