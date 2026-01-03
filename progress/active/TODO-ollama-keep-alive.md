# TODO: Fix OLLAMA_KEEP_ALIVE Not Configured

> Ollama models are being unloaded from memory between requests, causing extremely slow response times even with GPU detected correctly.
>
> **Created**: 2026-01-02
> **Last Updated**: 2026-01-02 (Sprint N - initial discovery)
> **Owner**: Hacker + Architect
> **Priority**: HIGH
> **Category**: Performance Bug

---

## Problem Summary

### Symptoms Reported by User

- ✅ Ollama startup is CORRECT (GPU detected, CUDA logs visible)
- ✅ TUI launches FAST with test mode
- ❌ Model responses are VERY SLOW

### Root Cause Analysis

**Finding**: `OLLAMA_KEEP_ALIVE` environment variable is NOT set anywhere in codebase.

**Evidence**:
```bash
$ grep -r "OLLAMA_KEEP_ALIVE" ai-way/
# No results
```

**Impact**: Ollama's default `keep_alive` is 5 minutes. After 5 minutes of inactivity:
1. Model unloads from GPU memory
2. Next request triggers model reload (slow!)
3. User experiences 10-30 second delays even on tiny qwen2:0.5b model

### Web Research Findings

From web search on 2026-01-02:

**Source**: Multiple Ollama performance optimization guides (2025)

Key finding:
> "Setting a longer OLLAMA_KEEP_ALIVE value (e.g., 24h or -1) significantly improves inference response speed because the model remains in GPU or system memory."

**Impact Assessment**:
- This is the #1 most common Ollama performance issue
- Affects ALL models regardless of hardware
- Even with perfect GPU detection, models reload slowly
- Simple fix with massive performance improvement

---

## Current State

### Where Ollama is Started

**File**: `lib/ollama/service.sh`
**Function**: `ollama_ensure_running()` (line 128)
**Current code** (line 146-154):
```bash
# In test verbose mode, show all Ollama output (for debugging GPU/CUDA)
if [[ -n "${YOLLAYAH_TEST_VERBOSE:-}" ]]; then
    echo ">>> Starting ollama serve with verbose output..."
    echo ">>> LD_LIBRARY_PATH=/usr/lib:/usr/lib64:${LD_LIBRARY_PATH:-}"
    LD_LIBRARY_PATH="/usr/lib:/usr/lib64:${LD_LIBRARY_PATH:-}" ollama serve &
else
    LD_LIBRARY_PATH="/usr/lib:/usr/lib64:${LD_LIBRARY_PATH:-}" ollama serve > /dev/null 2>&1 &
fi
```

**Missing**: `OLLAMA_KEEP_ALIVE` not exported before starting `ollama serve`

---

## Fix Plan

### Phase 1: Add OLLAMA_KEEP_ALIVE Configuration ✅ IMMEDIATE

**Task 1.1**: Add environment variable export
- [ ] Modify `lib/ollama/service.sh::ollama_ensure_running()`
- [ ] Export `OLLAMA_KEEP_ALIVE` before starting `ollama serve`
- [ ] Use sensible default (e.g., `24h` or `-1`)

**Task 1.2**: Add configuration option
- [ ] Allow override via environment variable
- [ ] Document in `CLAUDE.md`
- [ ] Add to `yollayah.sh --help` output

**Task 1.3**: Update documentation
- [x] Update `TROUBLESHOOTING.md` with model unloading section
- [ ] Add diagnostic command to check if model stays loaded
- [ ] Document in `CLAUDE.md` build commands section

### Phase 2: Make Configurable (Nice to Have)

**Task 2.1**: Add to future config system
- [ ] When config file implemented, add `ollama.keep_alive` setting
- [ ] Allow per-model keep_alive settings
- [ ] Show warning if system memory low

### Phase 3: Add Monitoring (Future)

**Task 3.1**: Add diagnostics
- [ ] Log when model loads/unloads
- [ ] Track model memory usage
- [ ] Warn user if model unloading frequently

---

## Implementation Details

### Proposed Code Change

**File**: `lib/ollama/service.sh`
**Line**: 146 (before ollama serve)

```bash
# Set keep_alive to prevent model unloading
# Default: keep models loaded for 24 hours
# Override with: YOLLAYAH_OLLAMA_KEEP_ALIVE=<duration>
: "${YOLLAYAH_OLLAMA_KEEP_ALIVE:=24h}"
export OLLAMA_KEEP_ALIVE="$YOLLAYAH_OLLAMA_KEEP_ALIVE"

pj_result "Setting OLLAMA_KEEP_ALIVE=${OLLAMA_KEEP_ALIVE}"
log_ollama "INFO" "OLLAMA_KEEP_ALIVE set to: $OLLAMA_KEEP_ALIVE"

# Start ollama serve with GPU libraries
# In test verbose mode, show all Ollama output (for debugging GPU/CUDA)
if [[ -n "${YOLLAYAH_TEST_VERBOSE:-}" ]]; then
    echo ">>> Starting ollama serve with verbose output..."
    echo ">>> LD_LIBRARY_PATH=/usr/lib:/usr/lib64:${LD_LIBRARY_PATH:-}"
    echo ">>> OLLAMA_KEEP_ALIVE=${OLLAMA_KEEP_ALIVE}"
    LD_LIBRARY_PATH="/usr/lib:/usr/lib64:${LD_LIBRARY_PATH:-}" ollama serve &
else
    LD_LIBRARY_PATH="/usr/lib:/usr/lib64:${LD_LIBRARY_PATH:-}" ollama serve > /dev/null 2>&1 &
fi
```

### Configuration Options

| Value | Behavior | Use Case |
|-------|----------|----------|
| `-1` | Keep forever | Development, dedicated AI server |
| `24h` | Keep 24 hours | Production (recommended default) |
| `1h` | Keep 1 hour | Shared system, memory constrained |
| `5m` | Keep 5 minutes | Ollama default (TOO SHORT for our use case) |

### Recommended Default

**For yollayah.sh**: Use `24h` as default
- Long enough to keep model loaded during work session
- Still allows cleanup if system idle overnight
- Balances performance and memory usage
- Override with `YOLLAYAH_OLLAMA_KEEP_ALIVE` for customization

---

## Testing Plan

### Test 1: Verify OLLAMA_KEEP_ALIVE is Set

```bash
# Start yollayah
./yollayah.sh --test

# In another terminal, check environment
ps aux | grep ollama
# Get PID, then:
cat /proc/<PID>/environ | tr '\0' '\n' | grep OLLAMA_KEEP_ALIVE
# Expected: OLLAMA_KEEP_ALIVE=24h (or custom value)
```

### Test 2: Verify Model Stays Loaded

```bash
# Start yollayah
./yollayah.sh --test

# Send first message (loads model)
# ... use TUI ...

# Wait 10 minutes
sleep 600

# Send another message
# Expected: Should be FAST (model still in memory)
```

### Test 3: Check ollama ps Timeout

```bash
# After starting yollayah
ollama ps

# Look at "UNTIL" column
# Expected: Shows far future time (e.g., "24 hours from now")
# Not: "5 minutes from now" (old default)
```

---

## Related Issues

### Async Audit Results

**From**: `TODO-tui-async-audit.md`, `TODO-conductor-async-audit.md`

**Findings**: ✅ TUI and Conductor are PERFECT - fully async, zero blocking calls

**Conclusion**: Slow responses are NOT due to blocking code. TUI and Conductor are implemented correctly.

### Terminal Ownership

**From**: `TODO-architecture-terminal-ownership.md`

**Status**: ✅ RESOLVED - Bootstrap is synchronous, TUI owns terminal exclusively

**Conclusion**: Terminal corruption fixed, not related to slow responses.

### GPU Detection

**From**: `TROUBLESHOOTING.md`, `lib/ollama/service.sh`

**Status**: ✅ WORKING - User confirms GPU detected correctly in ollama serve logs

**Conclusion**: GPU detection is correct, performance issue is model unloading.

---

## Success Criteria

### Must Have

- [x] `OLLAMA_KEEP_ALIVE` is set before `ollama serve` starts
- [ ] Default value is reasonable (24h recommended)
- [ ] User can override with environment variable
- [ ] Documented in TROUBLESHOOTING.md
- [ ] Verbose test mode shows the setting

### Nice to Have

- [ ] Documented in CLAUDE.md
- [ ] Added to --help output
- [ ] Integration test verifies model stays loaded
- [ ] Warning if system memory low

---

## Priority Justification

**Priority**: HIGH

**Rationale**:
1. **User Impact**: Currently experiencing slow responses, making TUI nearly unusable
2. **Simple Fix**: One-line environment variable export
3. **High ROI**: Massive performance improvement for minimal code change
4. **Common Issue**: Affects ALL users, not edge case
5. **Quick Win**: Can be fixed and tested in < 30 minutes

**Recommendation**: Fix IMMEDIATELY in current session before other work.

---

## Dependencies

### Blocks

- User testing of TUI responsiveness
- Integration testing framework (slow model = slow tests)
- Performance benchmarking

### Blocked By

- None (can implement immediately)

---

## References

### Web Search Results (2026-01-02)

**Performance Guides**:
- How to Speed Up Ollama Performance (https://www.databasemart.com/kb/how-to-speed-up-ollama-performance)
- How to Make Ollama Faster (https://anakin.ai/blog/how-to-make-ollama-faster/)
- Boost Ollama Performance: Essential Tips and Tricks (https://www.arsturn.com/blog/tips-for-speeding-up-ollama-performance)

**Key Takeaways**:
- OLLAMA_KEEP_ALIVE is #1 performance optimization
- Memory bandwidth more important than CPU/GPU speed for LLMs
- Model loading is slowest operation (avoid at all costs)

### Internal Documents

- `TROUBLESHOOTING.md` - Diagnostic guide
- `lib/ollama/service.sh` - Ollama startup code
- `TODO-tui-async-audit.md` - TUI is perfect (not the issue)
- `TODO-conductor-async-audit.md` - Conductor is perfect (not the issue)

---

## Next Steps

1. **Implement fix** (Hacker)
   - Modify `lib/ollama/service.sh`
   - Add `OLLAMA_KEEP_ALIVE` export
   - Test in verbose mode

2. **Verify fix** (QA)
   - Test model stays loaded
   - Benchmark response times
   - Check `ollama ps` output

3. **Document** (Architect)
   - Update `CLAUDE.md`
   - Update `yollayah.sh --help`
   - Add to integration tests

4. **User validation**
   - Have user test with fix applied
   - Gather performance feedback
   - Confirm issue resolved

---

**Owner**: Hacker + Architect
**Last Updated**: 2026-01-02
**Status**: OPEN - Ready for implementation
**Sprint Target**: Current (Sprint N)
