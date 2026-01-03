# Troubleshooting Guide

## Table of Contents

- [TTY/Terminal Issues](#ttyterminal-issues)
- [Slow Model Responses](#slow-model-responses)

---

# TTY/Terminal Issues

## Error: "yollayah-tui requires a terminal (TTY)"

**Issue**: TUI fails to launch with error about missing TTY

**Full Error Message**:
```
❌ Error: yollayah-tui requires a terminal (TTY)

This usually means:
  • Running in a non-interactive environment (CI, container)
  • SSH without -t flag
  • Piped stdin/stdout
```

### Common Causes

#### 1. SSH Without TTY Allocation

**Symptoms**:
- Connecting via SSH
- Error appears immediately

**Fix**:
```bash
# Wrong: ssh without -t
ssh user@host ./yollayah.sh

# Right: ssh with -t flag
ssh -t user@host ./yollayah.sh
```

#### 2. Running in Toolbox (Fedora Silverblue)

**Symptoms**:
- Using `toolbox run` without proper flags
- Error about TTY device

**Fix**:
```bash
# Wrong: toolbox run without directory
toolbox run ./yollayah.sh

# Right: toolbox run with --directory
toolbox run --directory $PWD ./yollayah.sh

# Or: Enter toolbox first
toolbox enter ai-way
./yollayah.sh
```

#### 3. Piped Input/Output

**Symptoms**:
- Using pipes or redirection with TUI
- Running in background

**Fix**:
```bash
# Wrong: piping to TUI
echo "hello" | ./yollayah.sh

# Right: run interactively
./yollayah.sh

# Or: use script command as wrapper
script -c './yollayah.sh' /dev/null
```

#### 4. Running in CI/Automated Environment

**Symptoms**:
- GitHub Actions, GitLab CI, or similar
- No real terminal attached

**Fix**:
```bash
# For CI: Use test mode which gracefully falls back to bash interface
./yollayah.sh --test

# Or: Use smoke test that's CI-aware
./tests/smoke_test_tui.sh  # Skips TUI launch if no TTY
```

### Verify You Have a TTY

```bash
# Check if stdin is a terminal
[ -t 0 ] && echo "stdin is TTY" || echo "stdin NOT a TTY"

# Check if stdout is a terminal
[ -t 1 ] && echo "stdout is TTY" || echo "stdout NOT a TTY"

# Both must be TTY for TUI to work
```

### Fallback Options

If you can't get a TTY, yollayah.sh will automatically fall back to the bash interface:

```bash
# If TUI fails, this happens automatically:
# 1. TTY check fails
# 2. Error message displayed
# 3. Falls back to simple bash prompt
# 4. You can still chat with Yollayah!
```

**Bash interface**:
- No animated avatar
- Simple text prompt
- All functionality works
- Type `/quit` to exit

---

# Slow Model Responses

**Issue**: Test mode launches fast but responses are slow

## Quick Diagnostics

### 0. Use Verbose Test Mode (Easiest!)

```bash
# Run test mode with ALL logs visible
./yollayah.sh --test

# You'll see:
# - Ollama serve startup logs (GPU/CUDA detection)
# - Model loading logs
# - All verbose output from Ollama
```

This is the **fastest way to see what's happening** - all Ollama logs will be displayed on screen instead of hidden by the TUI.

### 1. Check Which Model is Actually Running

```bash
# See what model Ollama is using
ollama ps

# Expected output for test mode:
# NAME            ID              SIZE    PROCESSOR    UNTIL
# qwen2:0.5b      abc123...       352MB   100% GPU     4 minutes from now
```

**What to look for**:
- Is it actually using qwen2:0.5b (352MB)?
- Or did it fall back to a larger model?
- Is PROCESSOR showing "100% GPU" or "100% CPU"?

### 2. Test Model Speed Directly

```bash
# Benchmark the model directly (bypass TUI)
time ollama run qwen2:0.5b "Say hi" --verbose

# Good response time: < 2 seconds total
# Bad response time: > 5 seconds
```

**What to check**:
- Total time
- Time to first token
- Tokens per second in verbose output

### 3. Check GPU Usage

```bash
# Is Ollama actually using the GPU?
ollama ps

# Also check nvidia-smi
nvidia-smi

# Look for ollama process using GPU
# Should show memory usage on GPU 0
```

**If CPU instead of GPU**:
- Check `lib/ollama/service.sh` line 146 (LD_LIBRARY_PATH)
- Verify: `LD_LIBRARY_PATH="/usr/lib:/usr/lib64:${LD_LIBRARY_PATH:-}"`
- Restart Ollama: `pkill ollama && ollama serve`

### 4. Check System Resources

```bash
# CPU usage
top -b -n 1 | grep ollama

# Memory
free -h

# Disk I/O (model loading)
iostat -x 1 3
```

**Common issues**:
- High CPU usage → Not using GPU
- Low memory → Swapping
- High disk I/O → Model reloading each time

### 5. Verify Model is Loaded

```bash
# Check if model is actually in Ollama
ollama list | grep qwen2

# Expected:
# qwen2:0.5b    <hash>    352 MB    X minutes ago

# If not there, pull it:
ollama pull qwen2:0.5b
```

### 6. Test with Minimal Prompt

```bash
# Single token response (fastest possible)
time echo "1" | ollama run qwen2:0.5b

# Should be near-instant (< 1 second)
```

### 7. Check Ollama Logs

```bash
# If Ollama started by yollayah.sh, check logs
journalctl -u ollama -f

# Or if running in foreground:
pkill ollama
ollama serve  # Watch output
```

**Look for**:
- CUDA/GPU initialization messages
- Model loading time
- Inference performance warnings

### 8. Compare Test vs Normal Model

```bash
# Test model (should be fast)
time ollama run qwen2:0.5b "hi"

# Normal model (will be slower, but how much?)
time ollama run llama3.2:3b "hi"

# Ratio should be ~3-5x (size difference)
```

## Common Issues & Fixes

### Issue: Model Keeps Unloading (MOST COMMON SLOW RESPONSE CAUSE!)

**Symptoms**:
- First response after startup is normal speed
- Subsequent responses are VERY slow (10-30 seconds)
- Model seems to reload for each query
- GPU detected correctly but still slow

**Cause**: Ollama's default `keep_alive` setting (5 minutes) unloads the model from GPU memory. Each new request requires reloading the model into VRAM, which is slow.

**Fix**:
```bash
# Set OLLAMA_KEEP_ALIVE before starting ollama
export OLLAMA_KEEP_ALIVE=-1  # Keep model loaded forever
ollama serve &

# Or set to specific duration
export OLLAMA_KEEP_ALIVE=24h  # Keep for 24 hours
```

**Verify it's working**:
```bash
# Send first message
time ollama run qwen2:0.5b "hi"  # ~1-2 seconds

# Wait 10 seconds, send another
sleep 10
time ollama run qwen2:0.5b "hi"  # Should ALSO be ~1-2 seconds

# If second one is slow, model unloaded - OLLAMA_KEEP_ALIVE not set
```

**For yollayah.sh** (fix in progress):
- Currently NOT setting OLLAMA_KEEP_ALIVE
- Will be added to `lib/ollama/service.sh`
- See: `TODO-ollama-keep-alive.md`

### Issue: Using CPU Instead of GPU

**Symptoms**:
- `ollama ps` shows "100% CPU"
- Very slow responses (10-30 seconds)
- nvidia-smi shows no Ollama process

**Fix**:
```bash
# Stop Ollama
pkill ollama

# Start with GPU libraries
LD_LIBRARY_PATH="/usr/lib:/usr/lib64:${LD_LIBRARY_PATH:-}" ollama serve &

# Verify GPU usage
nvidia-smi
ollama ps
```

### Issue: Wrong Model Loaded

**Symptoms**:
- `ollama ps` shows different model
- Slow even though test mode

**Fix**:
```bash
# Check what TUI is actually using
echo $YOLLAYAH_MODEL

# Should be: qwen2:0.5b in test mode

# Force test model
YOLLAYAH_TEST_MODEL=qwen2:0.5b ./yollayah.sh --test
```

### Issue: Model Reloading Each Time

**Symptoms**:
- First response fast
- Subsequent responses slow
- High disk I/O

**Fix**:
```bash
# Keep model in memory
# Check Ollama's keep_alive setting
# Models should stay loaded for 5+ minutes by default

# Verify model stays loaded
ollama ps  # Check "UNTIL" column
```

### Issue: Swap/Low Memory

**Symptoms**:
- System feels sluggish
- `free -h` shows low available memory
- High swap usage

**Fix**:
```bash
# Check memory
free -h

# Kill other apps if needed
# Or upgrade RAM

# Test mode should use < 1GB
# Normal mode needs 4-6GB
```

## Expected Performance

### Test Mode (qwen2:0.5b on RTX A5000)

**GPU**:
- First token: < 500ms
- Subsequent tokens: ~20-50ms each
- Total for "Say hi": < 1 second

**CPU** (fallback):
- First token: 1-2 seconds
- Subsequent tokens: 100-200ms each
- Total for "Say hi": 2-5 seconds

### If Slower Than This

**Check in order**:
1. GPU actually being used?
2. Right model loaded?
3. System resources available?
4. Network issues? (shouldn't affect local Ollama)
5. Ollama version outdated?

## Debug Commands Cheat Sheet

```bash
# Quick diagnostics (run all)
ollama ps                           # What's running?
ollama list | grep qwen2           # Model installed?
nvidia-smi | grep ollama           # GPU usage?
echo $YOLLAYAH_MODEL               # What model does TUI think?
time ollama run qwen2:0.5b "hi"   # Actual speed?

# Deep diagnostics
top | grep ollama                  # CPU usage
free -h                            # Memory
iostat -x 1 3                      # Disk I/O
journalctl -u ollama -n 50         # Ollama logs

# Force GPU
pkill ollama
LD_LIBRARY_PATH="/usr/lib:/usr/lib64" ollama serve &
```

## Getting Help

**Gather this info**:
```bash
# System info
uname -a
nvidia-smi
free -h

# Ollama info
ollama --version
ollama ps
ollama list

# Model test
time ollama run qwen2:0.5b "test" --verbose
```

**Include in bug report**:
- Above diagnostic output
- What you expected vs what happened
- Whether GPU is detected (from nvidia-smi)
- Timing from `time ollama run`

## Quick Fixes to Try

```bash
# 1. Restart Ollama with GPU
pkill ollama
LD_LIBRARY_PATH="/usr/lib:/usr/lib64" ollama serve &

# 2. Reload test model
ollama rm qwen2:0.5b
ollama pull qwen2:0.5b

# 3. Test directly
ollama run qwen2:0.5b "hi"

# 4. Restart test mode
./yollayah.sh --test
```

## Still Slow?

**Last resort checks**:
- Ollama version: `ollama --version` (should be recent)
- CUDA installed: `nvidia-smi` (should show driver version)
- Model corrupted: `ollama pull qwen2:0.5b` (re-download)
- System overload: `top` (other processes hogging CPU/GPU?)
- Thermal throttling: `nvidia-smi` (check GPU temp, should be < 85°C)

---

**Most common issue**: Ollama using CPU instead of GPU. Fix with LD_LIBRARY_PATH.
