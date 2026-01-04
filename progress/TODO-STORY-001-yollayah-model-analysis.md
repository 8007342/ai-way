# TODO-STORY-001-Yollayah-Model-Analysis

**Status**: 🔴 ACTIVE - Investigation
**Created**: 2026-01-03
**Priority**: P0 - BLOCKING
**Owner**: LLM Specialist, Ollama Expert
**Parent**: [TODO-EPIC-001-yollayah-gpu-model-generation.md](TODO-EPIC-001-yollayah-gpu-model-generation.md)

---

## Goal

**Understand exactly why yollayah model runs on CPU instead of GPU.**

Trace the complete model creation flow, compare with working base models, and identify the exact point where GPU inheritance breaks.

---

## Background

**Verified Behavior**:
- llama3.1:8b (base): 100% GPU ✅
- llama3.2:3b (base): 100% GPU ✅
- yollayah (custom): 99% CPU, <1% GPU ❌

**Current Creation Process**:
1. Generate Modelfile with SYSTEM prompt
2. Call `OLLAMA_NUM_GPU=999 ollama create yollayah -f modelfile`
3. Model created successfully
4. But runs on CPU!

---

## Tasks

### Task 1: Inspect Current yollayah Model ⏳

**Command**:
```bash
# Show complete model info
ollama show yollayah

# Show modelfile
ollama show yollayah --modelfile

# Show parameters
ollama show yollayah --parameters

# List all models with details
ollama list
```

**Document**:
- Base model reference
- SYSTEM prompt included?
- PARAMETER directives
- Model size
- Quantization format

---

### Task 2: Compare with Base Model ⏳

**Command**:
```bash
# Base model (works on GPU)
ollama show llama3.2:3b --modelfile
ollama show llama3.2:3b --parameters

# Compare outputs
diff <(ollama show llama3.2:3b) <(ollama show yollayah)
```

**Look for**:
- Differences in model structure
- Missing fields in yollayah
- Different quantization?
- GPU hints present in base but not custom?

---

### Task 3: Test Minimal SYSTEM Prompt ⏳

**Hypothesis**: Large SYSTEM prompt breaks GPU

**Test**:
```bash
# Create minimal test model
cat > /tmp/test-minimal.modelfile <<EOF
FROM llama3.2:3b

SYSTEM "You are a helpful assistant."

PARAMETER temperature 0.8
PARAMETER num_ctx 4096
EOF

# Create model
OLLAMA_NUM_GPU=999 ollama create test-minimal -f /tmp/test-minimal.modelfile

# Test GPU usage
ollama run test-minimal "haiku" &
# Monitor with btop in another terminal
```

**Expected**:
- If GPU works: SYSTEM prompt not the issue
- If CPU fallback: SYSTEM prompt size is the problem

---

### Task 4: Test WITHOUT SYSTEM Prompt ⏳

**Hypothesis**: ANY SYSTEM prompt breaks GPU

**Test**:
```bash
# Create model without SYSTEM
cat > /tmp/test-nosystem.modelfile <<EOF
FROM llama3.2:3b

PARAMETER temperature 0.8
PARAMETER num_ctx 4096
EOF

OLLAMA_NUM_GPU=999 ollama create test-nosystem -f /tmp/test-nosystem.modelfile

# Test GPU usage
ollama run test-nosystem "haiku" &
# Monitor with btop
```

**Expected**:
- If GPU works: SYSTEM prompt is breaking GPU
- If CPU fallback: Something else is wrong

---

### Task 5: Test Base Model Directly ⏳

**Verify base model actually uses GPU**:

```bash
# Clear all models from memory
ollama ps
# Stop any running

# Load base model fresh
ollama run llama3.2:3b "tell me a haiku" &

# Monitor GPU
btop
nvidia-smi
```

**Verify**:
- GPU usage 100%
- VRAM shows model loaded
- Fast inference

---

### Task 6: Trace Model Creation Flow ⏳

**Document the exact flow**:

1. **Entry point**: `yollayah/lib/yollayah/personality.sh:yollayah_create_model()`
2. **Modelfile generation**: `_generate_modelfile()` (lines 41-132)
3. **Wrapper call**: `ux_ollama_create()` in `lib/ux/output.sh`
4. **Actual creation**: `ollama create yollayah -f /tmp/yollayah.modelfile`

**Questions**:
- Does OLLAMA_NUM_GPU get passed correctly?
- Are there any intermediary steps?
- Is the model being copied or referenced?

---

### Task 7: Check Ollama Logs ⏳

**Enable Ollama debug logging**:

```bash
# Stop existing ollama
pkill ollama

# Start with debug logging
OLLAMA_DEBUG=1 ollama serve > /tmp/ollama-debug.log 2>&1 &

# Create yollayah model
ollama rm yollayah
./yollayah.sh --test

# Check logs
grep -i "gpu\|cuda\|layers" /tmp/ollama-debug.log
```

**Look for**:
- GPU detection messages
- Layer offload messages
- Warnings about CPU fallback
- Errors during model creation

---

### Task 8: Research Ollama GitHub Issues ⏳

**Search for similar problems**:

Keywords:
- "custom model CPU"
- "SYSTEM prompt GPU"
- "ollama create GPU"
- "model not using GPU"

**Document any known issues or workarounds**

---

## Hypotheses to Test

### Hypothesis 1: SYSTEM Prompt Size

**Theory**: Large SYSTEM prompt (~500 tokens) causes CPU fallback

**Test**: Task 3 (minimal SYSTEM) and Task 4 (no SYSTEM)

**If True**: Need to inject personality at runtime, not model creation

---

### Hypothesis 2: Model Copy vs Reference

**Theory**: `ollama create` copies model but loses GPU flags

**Test**: Compare model internals, check Ollama logs

**If True**: Need different creation method (LoRA adapter? Runtime prompt?)

---

### Hypothesis 3: OLLAMA_NUM_GPU Not Applied

**Theory**: Environment variable not passed through creation process

**Test**: Check Ollama debug logs for GPU layer messages

**If True**: Need to use API instead of CLI

---

### Hypothesis 4: Quantization Mismatch

**Theory**: Custom model uses different quantization that doesn't support GPU

**Test**: Compare quantization between base and custom

**If True**: Need to specify quantization during creation

---

### Hypothesis 5: Ollama Bug

**Theory**: Ollama has a bug with custom models + SYSTEM prompts + GPU

**Test**: Search GitHub issues, try different Ollama versions

**If True**: Need workaround or upstream fix

---

## Success Criteria

- [⏳] Know exact root cause of CPU fallback
- [⏳] Have reproducible minimal test case
- [⏳] Documented comparison: base vs custom model
- [⏳] Clear fix strategy identified
- [⏳] All hypotheses tested

---

## Deliverables

1. **Analysis Document**: Complete findings in this file
2. **Model Comparison**: diff output of base vs custom
3. **Test Results**: Results of Tasks 3-5
4. **Ollama Logs**: Debug output showing GPU behavior
5. **Fix Strategy**: Recommended approach based on findings

---

## Timeline

**Target**: Complete within 1 day
- Tasks 1-2: 30 minutes (inspection)
- Tasks 3-5: 1 hour (testing)
- Tasks 6-7: 30 minutes (tracing/logs)
- Task 8: 30 minutes (research)
- Analysis: 30 minutes (synthesis)

**Total**: ~3 hours of focused investigation

---

## Notes

This is the foundation for the entire EPIC. Without understanding the root cause, we can't fix it properly.

**Take the time to be thorough.** Document everything. Test methodically.
