# TODO-BUG-009-Model-Creation-Impacts-Performance

**Status**: 🔴 CRITICAL - Active Investigation
**Created**: 2026-01-03
**Priority**: P0 - BLOCKING - Performance Regression
**Severity**: CRITICAL - GPU not being used for inference

---

## Problem Statement

**User Discovery**: Custom "yollayah" model runs on **CPU** instead of **GPU**, causing all the slowness we've been investigating.

### Evidence

```bash
# CPU (SLOW) - custom model
ollama run yollayah "tell me a haiku"
# Behavior: Slow, CPU inference

# CPU (SLOW) - large model (doesn't fit in VRAM)
ollama run llama3.1:70b "tell me a haiku"
# Behavior: Slow, CPU fallback (expected - model too large)

# GPU (FAST) - base model
ollama run llama3.1:8b "tell me a haiku"
# Behavior: Blazing fast, GPU inference
```

**Conclusion**: The "yollayah" model is being created incorrectly, forcing CPU fallback.

---

## Design Principle Violated

**PRINCIPLE**: "If a GPU is available, CPU should be left for secondary tasks, not for inference."

- ✅ **Correct**: GPU handles inference (compute-intensive)
- ✅ **Correct**: CPU handles orchestration, I/O, coordination
- ❌ **VIOLATED**: yollayah model using CPU for inference

---

## Investigation Checkpoints

### Checkpoint 1: Locate Model Creation Code ⏳

**Task**: Find where/how the "yollayah" model is created

**Files to Check**:
- [ ] `yollayah/lib/yollayah/personality.sh` - Model definition
- [ ] `yollayah/lib/ollama/service.sh` - Model creation/loading
- [ ] Search for `ollama create` commands
- [ ] Search for Modelfile generation

**Questions**:
1. Where is the yollayah model created?
2. What parameters are passed during creation?
3. Is there a Modelfile? What's in it?

---

### Checkpoint 2: Research Ollama Model Creation ✅ COMPLETE

**Task**: Understand Ollama's model creation parameters for GPU

**CRITICAL FINDINGS**:

1. **`num_gpu` is NOT a Modelfile PARAMETER**
   - It's a **runtime option** only (API/env var/interactive)
   - Cannot be set in Modelfile PARAMETER directives
   - Must use: `OLLAMA_NUM_GPU=999` or API options

2. **Custom models automatically inherit GPU from base**
   - No explicit configuration needed
   - llama3.2:3b uses GPU → yollayah should too

3. **Ollama auto-detects and uses GPU by default**
   - No manual configuration required
   - Automatically selects backend (CUDA/ROCm/Metal)

4. **All quantization formats support GPU**
   - Q4_K_M, Q8_0, Q5_K_M all work with GPU
   - SYSTEM prompt doesn't affect quantization

**The Real Issue**: VRAM exhaustion or model not staying loaded

**Sources**:
- Official Ollama Modelfile docs
- GitHub issues #1855, #3732
- Community guides and performance tuning docs

---

### Checkpoint 3: Diagnose Root Cause ⏳

**Task**: Determine WHY yollayah uses CPU instead of GPU

**Hypothesis Ranking** (after research):

1. **VRAM Exhaustion** (MOST LIKELY)
   ```
   llama3.2:3b base    ~2GB VRAM
   + SYSTEM prompt     ~50MB
   + Context (8192)    ~1-2GB
   + KV cache          ~500MB
   ─────────────────────────────
   Total:              ~4-5GB
   ```
   If GPU has limited VRAM or is shared with desktop, CPU fallback occurs.

2. **Model Not Preloaded** (LIKELY)
   - Base model stays in VRAM
   - Custom model unloads between calls
   - Each call loads from disk → CPU until VRAM space available

3. **OLLAMA_NUM_GPU Not Set** (POSSIBLE)
   - Runtime needs explicit `OLLAMA_NUM_GPU=999`
   - Modelfile alone insufficient

**Diagnostic Commands**:
```bash
# While running: ollama run yollayah "test"
nvidia-smi  # Should show GPU activity
ollama ps   # Should show "100% GPU" or split

# Check VRAM availability
nvidia-smi --query-gpu=memory.free --format=csv

# Test with forced GPU
OLLAMA_NUM_GPU=999 ollama run yollayah "test"
```

---

### Checkpoint 4: Implement Fix ⏳

**Task**: Update model creation to ensure GPU usage

**Fix Strategy** (based on research):

1. **Reduce Context Window** (4096 instead of 8192)
   - Lowers VRAM requirement
   - Still plenty for conversations
   - Leaves headroom for GPU

2. **Add Runtime GPU Forcing**
   - Set `OLLAMA_NUM_GPU=999` during creation
   - Ensures maximum GPU layer offload
   - Works even if auto-detection flaky

3. **Optimize PARAMETER directives**
   - Add `num_predict 512` (limit output tokens)
   - Keep temperature and system prompt
   - Remove any invalid parameters

4. **Add GPU Verification**
   - Test model immediately after creation
   - Check nvidia-smi for GPU usage
   - Fail loudly if CPU fallback detected

**Updated Modelfile**:
```modelfile
FROM llama3.2:3b

SYSTEM """
[Yollayah personality - ~500 tokens]
"""

# Optimized for GPU (reduced VRAM usage)
PARAMETER temperature 0.8
PARAMETER num_ctx 4096
PARAMETER num_predict 512
```

**Updated Creation Code**:
```bash
# Force GPU layers during creation
OLLAMA_NUM_GPU=999 ollama create yollayah -f "$modelfile_path"

# Verify GPU usage immediately
ollama run yollayah "hi" &
sleep 2
if ! nvidia-smi | grep -q "ollama"; then
    log_error "WARNING: yollayah not using GPU!"
fi
```

---

### Checkpoint 5: Verify GPU Usage ⏳

**Task**: Confirm yollayah runs on GPU after fix

**Verification Steps**:

1. **Monitor GPU during inference**:
   ```bash
   # Terminal 1
   watch -n 0.5 nvidia-smi

   # Terminal 2
   time ollama run yollayah "tell me a haiku"
   ```

2. **Check ollama process status**:
   ```bash
   ollama ps
   # Expected: "100% GPU" or GPU percentage > 50%
   ```

3. **Performance comparison**:
   ```bash
   # Should be similar speeds (both GPU)
   time ollama run llama3.2:3b "haiku"
   time ollama run yollayah "haiku"
   ```

**Success Criteria**:
- [⏳] nvidia-smi shows GPU usage spike during inference
- [⏳] VRAM increases in nvidia-smi (model loaded)
- [⏳] Performance matches base model (~1-2 seconds for haiku)
- [⏳] ollama ps reports GPU usage
- [⏳] CPU usage < 10% during inference

---

### Checkpoint 6: Update Model Creation Code ⏳

**Task**: Ensure yollayah.sh creates models correctly

**Changes Needed**:
- [ ] Add GPU parameters to model creation
- [ ] Verify base model is GPU-compatible
- [ ] Add validation: check if model uses GPU after creation
- [ ] Document model creation parameters

---

### Checkpoint 7: Verification ⏳

**Task**: Confirm yollayah uses GPU

**Tests**:
```bash
# Terminal 1: Monitor GPU
watch -n 0.5 nvidia-smi

# Terminal 2: Run yollayah
ollama run yollayah "tell me a haiku"

# Expected: GPU utilization spike in nvidia-smi
# Expected: Fast inference (< 1 second for haiku)
```

**Success Criteria**:
- [⏳] nvidia-smi shows GPU usage during yollayah inference
- [⏳] Performance matches llama3.1:8b (base model)
- [⏳] CPU usage is minimal (< 10%)
- [⏳] VRAM shows model loaded (check nvidia-smi memory)

---

## Hypothesis: Why CPU Fallback?

### Possible Causes

1. **Missing GPU parameters in Modelfile**
   - Modelfile doesn't specify GPU layers
   - Defaults to CPU-only inference

2. **Quantization mismatch**
   - Wrong quantization format
   - Some formats don't support GPU

3. **Base model not loaded in GPU**
   - Base model (llama3.2:3b) not in VRAM
   - Custom model can't use GPU without base in VRAM

4. **Ollama create doesn't preserve GPU settings**
   - Bug in ollama create?
   - Need explicit parameters during creation

5. **Model too large after adding system prompt**
   - System prompt + context makes it exceed VRAM
   - Forces CPU fallback

---

## Current yollayah Model Definition

**Location**: `yollayah/lib/yollayah/personality.sh` (lines 44-90)

```bash
# Generate modelfile (preliminary check, will investigate)
cat > "$modelfile_path" <<EOF
FROM llama3.2:3b

SYSTEM """
You are Yollayah, the heart of ai-way...
[~500 tokens of personality]
"""
EOF

# Create model
ollama create yollayah -f "$modelfile_path"
```

**Missing from current definition** (hypothesis):
- ❌ GPU layer configuration
- ❌ Device placement hints
- ❌ Runtime parameters for GPU
- ❌ Quantization settings

---

## Research: Ollama Model Creation Best Practices

### Documentation to Review

1. **Ollama Modelfile Syntax**:
   - `FROM` directive - base model reference
   - `SYSTEM` directive - system prompt
   - `PARAMETER` directive - runtime settings
   - `ADAPTER` directive - LoRA adapters

2. **GPU Configuration**:
   - Environment variables (CUDA_VISIBLE_DEVICES, etc.)
   - Runtime parameters (num_gpu, gpu_layers)
   - Memory management (OLLAMA_MAX_LOADED_MODELS)

3. **Model Inheritance**:
   - Do custom models inherit GPU settings from base?
   - Or do they need explicit configuration?

---

## Expected Fix

**Hypothesis**: Add GPU parameters to Modelfile

```modelfile
FROM llama3.2:3b

# GPU Configuration
PARAMETER num_gpu 1
PARAMETER num_thread 4

SYSTEM """
You are Yollayah, the heart of ai-way...
"""
```

**Or**: Use ollama create flags

```bash
ollama create yollayah -f Modelfile --num-gpu 1
```

---

## Next Steps

1. **Immediate**: Checkpoint 1 - Find model creation code
2. **Research**: Checkpoint 2 - Ollama GPU parameters
3. **Analysis**: Checkpoint 3 - Inspect current yollayah model
4. **Compare**: Checkpoint 4 - Diff with working model
5. **Fix**: Checkpoint 5 - Rebuild with GPU support
6. **Verify**: Checkpoint 7 - Confirm GPU usage

---

## Related Issues

- **BUG-007**: --interactive mode slow (parent issue - this is root cause!)
- **BUG-008**: LD_LIBRARY_PATH override removed
- **EPIC**: Reactive refactor (performance investigation)
- **PERFORMANCE-AUDIT**: Streaming analysis (missed this!)

---

## Success Criteria

- [⏳] yollayah model uses GPU (nvidia-smi confirms)
- [⏳] Performance matches llama3.1:8b base model
- [⏳] CPU usage < 10% during inference
- [⏳] VRAM usage visible in nvidia-smi
- [⏳] Inference time < 1 second for haiku generation
- [⏳] Model creation code updated with GPU parameters
- [⏳] Documentation includes GPU configuration

---

## Lessons to Learn

1. **Always verify device placement** - Don't assume models use GPU
2. **Test custom models thoroughly** - Compare against base models
3. **Monitor nvidia-smi during testing** - Visual confirmation required
4. **Document model creation parameters** - Critical for reproducibility

---

**The golden rule**: Custom models must preserve GPU capabilities of base models.
