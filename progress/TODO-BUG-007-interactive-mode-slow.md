# TODO-BUG-007-Interactive-Mode-Slow

**Status**: 🔴 CRITICAL - Active Investigation
**Created**: 2026-01-03
**Priority**: P0 - BLOCKING UX
**Severity**: CRITICAL - Performance regression

---

## Navigation

**Parent**: [TODO-yollayah.sh-improvements.md](TODO-yollayah.sh-improvements.md)
**Related**:
- `yollayah/lib/ux/terminal.sh` - Fallback chat implementation
- `yollayah/lib/ollama/service.sh` - Ollama server management

---

## Problem Statement

**User Report**: "How come --interactive responds slowly but ollama run returns in the gpu?"

**Observation**:
- ✅ Direct `ollama run <model> "prompt"` from terminal: **FAST** (GPU, instant)
- ❌ `./yollayah.sh --interactive` (which calls `ollama run`): **SLOW**

This is illogical - both should use the same ollama server with the same OLLAMA_KEEP_ALIVE settings.

---

## Investigation Questions

### Q1: Is OLLAMA_KEEP_ALIVE Actually Being Used?

**Expected**:
- yollayah.sh starts `ollama serve` with `OLLAMA_KEEP_ALIVE=24h`
- Model stays loaded in VRAM
- All `ollama run` calls (including from --interactive) should be fast

**Check**:
```bash
# When yollayah.sh is running, check ollama process:
ps aux | grep "ollama serve"
# Get PID, then:
cat /proc/<PID>/environ | tr '\0' '\n' | grep OLLAMA_KEEP_ALIVE
# Expected: OLLAMA_KEEP_ALIVE=24h
```

### Q2: Are We Testing the Same Model?

**Direct ollama**:
```bash
ollama run llama3.2:3b "test"  # Which model?
```

**--interactive mode**:
```bash
./yollayah.sh --interactive
# Uses $SELECTED_MODEL - what is it?
```

**Verify**: Both must use same model for fair comparison

### Q3: Is Model Already Loaded?

**Hypothesis**: User runs direct `ollama run` AFTER model is already warm (loaded from previous use).
Then runs `--interactive` when model is cold (unloaded).

**Test**:
```bash
# Cold start test - kill ollama first
pkill ollama
time ollama run llama3.2:3b "hi"  # First run (cold)
time ollama run llama3.2:3b "hi"  # Second run (warm) - should be FAST

# Now test --interactive
pkill ollama
./yollayah.sh --interactive
# Type prompt, observe speed
```

### Q4: Is There Bash Overhead in the Loop?

**Current implementation** (terminal.sh:216):
```bash
ollama run "$model_name" "$user_input"
```

**Possible issues**:
- Fork/exec overhead per message?
- Environment variables not inherited?
- Stdout buffering?

**Test**: Add timing to conversation loop
```bash
start=$(date +%s%3N)
ollama run "$model_name" "$user_input"
end=$(date +%s%3N)
echo "Response time: $((end - start))ms"
```

### Q5: Are We Using Different Ollama Instances?

**Check**:
```bash
# When --interactive is running:
ps aux | grep ollama
# Should see ONE ollama serve process

# When direct ollama run:
ps aux | grep ollama
# Should see SAME ollama serve process
```

**Hypothesis**: Maybe --interactive starts its own ollama instance?

---

## Possible Root Causes

### HYPOTHESIS A: Model Not Staying Loaded

**Symptom**: Each message takes 3-7 seconds (GPU model load time)

**Cause**: OLLAMA_KEEP_ALIVE not working

**Evidence Needed**:
```bash
# After first message in --interactive:
ollama ps  # Should show model loaded
# Wait 10 seconds
ollama ps  # Should STILL show model loaded
```

**Fix if true**:
- Debug why OLLAMA_KEEP_ALIVE isn't working
- Check if environment variable is actually set
- Verify ollama version supports keep_alive

### HYPOTHESIS B: Wrong Model Being Used

**Symptom**: Slow because model is too large for VRAM

**Cause**: --interactive using different (larger) model than direct ollama

**Evidence Needed**:
```bash
# In --interactive, type:
/model
# Compare with direct: ollama ps
```

**Fix if true**:
- Ensure model selection is correct
- Add model name to prompt: `You (llama3.2:3b):`

### HYPOTHESIS C: Bash Overhead

**Symptom**: Delay before streaming starts, then fast streaming

**Cause**: Fork/exec overhead, environment setup

**Evidence Needed**:
- Add timing logs around `ollama run` call
- Compare with direct `time ollama run`

**Fix if true**:
- Minimize bash overhead
- Use builtin commands where possible
- Profile conversation loop

### HYPOTHESIS D: Ollama Server Not Running

**Symptom**: Each call starts ollama server (slow!)

**Cause**: ollama_ensure_running() not working correctly

**Evidence Needed**:
```bash
# Before running --interactive:
pgrep -f "ollama serve"  # Should return PID
# If empty, ollama not running!
```

**Fix if true**:
- Debug ollama_ensure_running()
- Ensure server starts and stays running
- Add health check before conversation loop

### HYPOTHESIS E: Network/Socket Overhead

**Symptom**: Consistent ~100-500ms delay per request

**Cause**: ollama client connects via HTTP localhost

**Evidence Needed**:
- strace ollama run to see syscalls
- Check if multiple connects happening

**Fix if true**:
- Not much we can do (ollama architecture)
- But should be same for direct ollama run!

---

## Debugging Plan

### Phase 1: Data Collection (User)

Ask user to run:

```bash
# Test 1: Verify ollama is running
./yollayah.sh --interactive &
sleep 5
ps aux | grep "ollama serve"
cat /proc/<PID>/environ | tr '\0' '\n' | grep OLLAMA_KEEP_ALIVE
# Expected: OLLAMA_KEEP_ALIVE=24h

# Test 2: Check model persistence
# In --interactive, send message
ollama ps  # Should show model loaded
sleep 30
ollama ps  # Should STILL show model loaded (if KEEP_ALIVE works)

# Test 3: Compare timing
pkill ollama
time ollama run llama3.2:3b "hi"  # Direct (cold start)
time ollama run llama3.2:3b "hi"  # Direct (warm)

pkill ollama
./yollayah.sh --interactive
# Type "hi" and observe time to first token
# Should match "warm" time from direct ollama
```

### Phase 2: Add Instrumentation

Modify terminal.sh to log timing:

```bash
ux_conversation_loop() {
    # ... existing code ...

    while true; do
        # ... read input ...

        # TIMING: Start
        local req_start=$(date +%s%3N)

        # Stream the response
        if ! ollama run "$model_name" "$user_input"; then
            echo ""
            ux_error "Failed to get response"
        fi

        # TIMING: End
        local req_end=$(date +%s%3N)
        local req_duration=$((req_end - req_start))

        ux_info "⏱️  Response time: ${req_duration}ms"
        ux_blank
    done
}
```

### Phase 3: Root Cause Analysis

Based on timing data:
- **<1000ms**: Normal, using GPU with warm model ✅
- **1000-3000ms**: Possible network/bash overhead ⚠️
- **3000-7000ms**: Model reloading from disk (KEEP_ALIVE broken) ❌
- **>7000ms**: CPU inference or huge model ❌

---

## Expected Behavior

Both should be identical:

| Operation | Expected Time | Notes |
|-----------|---------------|-------|
| First message (cold start) | 3-7s | Model load to GPU |
| Second message (warm) | <1s | Model already in VRAM |
| Subsequent messages | <1s | Model stays loaded (KEEP_ALIVE) |

**--interactive should match direct `ollama run` performance exactly.**

---

## Temporary Workarounds

Until fixed:

1. **Pre-warm model**:
   ```bash
   ollama run llama3.2:3b "hi"  # Warm up
   ./yollayah.sh --interactive  # Now fast
   ```

2. **Use TUI instead** (reactive streaming fixed):
   ```bash
   ./yollayah.sh  # Not --interactive
   ```

3. **Manually set OLLAMA_KEEP_ALIVE**:
   ```bash
   export OLLAMA_KEEP_ALIVE=-1
   ollama serve &
   ./yollayah.sh --interactive
   ```

---

## Success Criteria

- [ ] --interactive first message: <1s (if model already loaded)
- [ ] --interactive subsequent messages: <1s
- [ ] Performance matches direct `ollama run`
- [ ] Model stays loaded between messages
- [ ] No visible delay before streaming starts

---

## Next Steps

1. **USER**: Run debugging tests (Phase 1)
2. **TEAM**: Analyze timing data
3. **HACKER**: Add instrumentation (Phase 2)
4. **TEAM**: Identify root cause
5. **HACKER**: Implement fix
6. **QA**: Verify performance matches direct ollama

---

## Related Issues

- TODO-ollama-keep-alive.md: OLLAMA_KEEP_ALIVE configuration (should be working)
- TODO-BUG-006: Reactive streaming fix (TUI only, not --interactive)
- TODO-yollayah.sh-improvements.md: QoL improvements for interactive mode

---

**This is blocking. --interactive mode MUST be as fast as direct ollama run.**
