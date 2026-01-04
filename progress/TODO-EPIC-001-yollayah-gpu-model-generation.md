# TODO-EPIC-001-Yollayah-GPU-Model-Generation

**Status**: 🔴 CRITICAL - Active Development
**Created**: 2026-01-03
**Priority**: P0 - BLOCKING - Performance Critical
**Owner**: Architect, LLM Specialist, Ollama Expert, GPU Specialist
**Policy**: ZERO TOLERANCE - Model must use GPU

---

## Problem Statement

**User Finding**: yollayah model runs **99% CPU, <1% GPU** while base llama3.1:8b runs **100% GPU**.

**Impact**: ALL inference is slow due to CPU execution instead of GPU acceleration.

**Verification** (via btop):
```bash
# Terminal 1: Monitor resources
btop

# Terminal 2: Test base model (GPU)
ollama run llama3.1:8b "tell me a haiku"
# Result: 100% GPU utilization ✅

# Terminal 3: Test yollayah (CPU)
ollama run yollayah "tell me a haiku"
# Result: 99% CPU, <1% GPU ❌
```

**Conclusion**: Model creation process is fundamentally broken for GPU.

---

## Navigation

**Parent**: [TODO-AI-WAY.md](TODO-AI-WAY.md)
**Related**:
- BUG-009: Model creation impacts performance
- BUG-007: Interactive mode slow
- EPIC: Reactive refactor (completed, but root cause was model)

**Children** (Stories):
- [TODO-STORY-001-yollayah-model-analysis.md](#) - Deep dive into model generation
- [TODO-STORY-002-integration-testing-tools.md](#) - Build diagnostic tooling
- [TODO-STORY-003-gpu-model-creation-fix.md](#) - Implement GPU fix

---

## Current State

### What We Know

**Location**: Model created in `yollayah/lib/yollayah/personality.sh`

**Current Modelfile** (lines 44-132):
```modelfile
FROM llama3.2:3b

SYSTEM """
[~500 tokens of Yollayah personality]
"""

PARAMETER temperature 0.8
PARAMETER num_ctx 4096
PARAMETER num_predict 512
```

**Current Creation** (`lib/ux/output.sh:409-428`):
```bash
export OLLAMA_NUM_GPU=999
ollama create yollayah -f "$modelfile"
```

**What We Tried** (BUG-009):
- ✅ Reduced context window: 8192 → 4096
- ✅ Added num_predict limit: 512
- ✅ Added OLLAMA_NUM_GPU=999 during creation
- ❌ **Result**: Still runs on CPU!

### What Works vs What Doesn't

| Model | GPU Usage | Status |
|-------|-----------|--------|
| llama3.1:8b | 100% | ✅ WORKS |
| llama3.2:3b | 100% | ✅ WORKS |
| yollayah (custom) | <1% | ❌ BROKEN |

**Key Insight**: Base models work, custom model doesn't. Something in our creation process breaks GPU.

---

## Research Findings (So Far)

### From BUG-009 Investigation

1. **num_gpu is runtime-only** - Cannot be set in Modelfile
2. **Models should inherit GPU** - Custom models inherit from base
3. **Ollama auto-detects GPU** - No manual config needed
4. **Quantization irrelevant** - All formats support GPU
5. **VRAM exhaustion theory** - Disproven (4096 context should fit)

### New Hypothesis

**Previous theory**: VRAM exhaustion → CPU fallback
**Current reality**: Something else is breaking GPU inheritance

**Possible causes**:
1. Ollama bug with custom models + SYSTEM prompts
2. Model layers not being copied correctly
3. GPU flag lost during model creation
4. Base model not actually in VRAM when creating custom model
5. Toolbox GPU passthrough issue (but base models work fine)

---

## Requirements for GPU Execution

### Passing Criteria

**MUST achieve 100% GPU utilization** (or >90% with minor CPU overhead):

- [⏳] yollayah model uses GPU during inference (verified via btop/nvidia-smi)
- [⏳] Performance matches base model llama3.1:8b
- [⏳] CPU usage < 10% during inference
- [⏳] VRAM shows model loaded (~2-3GB)
- [⏳] First token latency < 200ms (GPU speed)
- [⏳] Subsequent tokens: 50-100 tokens/sec (GPU speed)

### Design Principles

1. **"GPU for inference, CPU for orchestration"**
   - Compute-intensive work (inference) → GPU
   - I/O and coordination → CPU

2. **"Test like production"**
   - Integration tests must verify GPU usage
   - Automated checks in CI/CD
   - No manual verification required

3. **"Fail fast, fail loud"**
   - If model creation results in CPU fallback, ERROR immediately
   - Don't silently accept degraded performance

---

## Implementation Plan

### Story 1: Deep Analysis of Model Generation ⏳

**File**: `TODO-STORY-001-yollayah-model-analysis.md`

**Goals**:
1. Trace exact model creation flow (code + Ollama internals)
2. Compare yollayah creation vs base model structure
3. Identify where GPU inheritance breaks
4. Reproduce issue in minimal test case

**Tasks**:
- [ ] Document complete model creation flow
- [ ] Inspect yollayah model internals (`ollama show --modelfile`)
- [ ] Compare with working base model
- [ ] Test minimal SYSTEM prompt (does it break GPU?)
- [ ] Test without SYSTEM prompt (does GPU work?)

**Success Criteria**: Know exactly why GPU fails

---

### Story 2: Integration Testing Tools 🔴 CRITICAL

**File**: `TODO-STORY-002-integration-testing-tools.md`

**Goals**:
1. Extend `--test` mode to verify GPU usage
2. Add verbosity flags for diagnostic output
3. Build automated GPU verification
4. Create integration test suite

**Tasks**:
- [ ] Add `--test-gpu` mode to yollayah.sh
- [ ] Implement `--robot` flag with component verbosity
- [ ] Build GPU verification script
- [ ] Add to yollayah-build-logs.sh
- [ ] Create integration test: "model uses GPU"

**Success Criteria**: Automated GPU verification in CI/CD

---

### Story 3: GPU Model Creation Fix ⏳

**File**: `TODO-STORY-003-gpu-model-creation-fix.md`

**Goals**:
1. Implement fix based on Story 1 findings
2. Verify GPU usage after fix
3. Add runtime verification
4. Document solution

**Tasks**:
- [ ] TBD (depends on Story 1 findings)
- [ ] Implement fix
- [ ] Add GPU verification to model creation
- [ ] Test across different GPUs/systems
- [ ] Document workarounds if needed

**Success Criteria**: yollayah uses GPU 100%

---

## Team Analysis Required

### Architect

**Questions**:
1. Should custom models be created differently?
2. Is there a better approach than `ollama create`?
3. Should we use LoRA adapters instead of full model copy?
4. What's the correct architecture for personality injection?

**Deliverable**: System design for GPU-compatible model creation

---

### LLM Specialist

**Questions**:
1. How do SYSTEM prompts affect model GPU placement?
2. Do large SYSTEM prompts cause issues?
3. Should personality be injected at runtime vs model creation?
4. Are there Ollama limitations with custom models?

**Deliverable**: LLM-specific requirements for GPU

---

### Ollama Expert

**Questions**:
1. What's the difference between base models and custom models internally?
2. Does `ollama create` preserve GPU flags?
3. Are there known issues with SYSTEM prompts and GPU?
4. Do we need to use different Ollama API endpoints?

**Deliverable**: Ollama internals documentation

---

### GPU Specialist

**Questions**:
1. How to verify GPU usage programmatically?
2. What's the difference in GPU usage between models?
3. Could this be a CUDA/driver issue?
4. Are there Ollama GPU diagnostics we can use?

**Deliverable**: GPU verification tooling

---

## Integration Testing Tools

### Goal: Automated GPU Verification

**Extend yollayah.sh**:
```bash
./yollayah.sh --test-gpu
# - Creates yollayah model
# - Runs test inference
# - Verifies GPU usage via nvidia-smi
# - EXIT 0 if GPU, EXIT 1 if CPU
```

**Add verbosity control**:
```bash
./yollayah.sh --robot=tui=full:conductor=off:proto=warn
# Component-level verbosity:
# - tui=full (all TUI logs)
# - conductor=off (no conductor logs)
# - proto=warn (only warnings from proto)
```

**Extend yollayah-build-logs.sh**:
```bash
./yollayah/yollayah-build-logs.sh --verify-gpu
# - Builds workspace
# - Runs GPU integration test
# - Reports GPU usage metrics
```

---

## Checkpoints (TODO-driven Methodology)

### Checkpoint 1: Create EPIC Structure ✅

- [x] Create EPIC-001-yollayah-gpu-model-generation.md
- [x] Define problem statement
- [x] Document current state
- [x] List requirements

### Checkpoint 2: Team Analysis ⏳

- [ ] Get Architect input on system design
- [ ] Get LLM Specialist input on SYSTEM prompts
- [ ] Get Ollama Expert input on model internals
- [ ] Get GPU Specialist input on verification
- [ ] Synthesize findings into Story 1

### Checkpoint 3: Create Stories ⏳

- [ ] Write TODO-STORY-001-yollayah-model-analysis.md
- [ ] Write TODO-STORY-002-integration-testing-tools.md
- [ ] Write TODO-STORY-003-gpu-model-creation-fix.md
- [ ] Link stories to EPIC

### Checkpoint 4: Build Integration Tools ⏳

- [ ] Implement --test-gpu mode
- [ ] Implement --robot verbosity flags
- [ ] Add GPU verification to yollayah-build-logs.sh
- [ ] Test tooling in CI/CD

### Checkpoint 5: Deep Model Analysis ⏳

- [ ] Trace model creation flow
- [ ] Compare yollayah vs base models
- [ ] Identify GPU breakage point
- [ ] Document findings

### Checkpoint 6: Implement Fix ⏳

- [ ] Apply fix based on analysis
- [ ] Verify GPU usage
- [ ] Run integration tests
- [ ] Document solution

### Checkpoint 7: QA & Documentation ⏳

- [ ] Run full test suite
- [ ] Verify across different systems
- [ ] Update documentation
- [ ] Mark EPIC as DONE

---

## Passing Goals

### Phase 1: Understanding (This Week)

**Goal**: Know exactly why GPU fails

- [ ] Complete model analysis (Story 1)
- [ ] Identify root cause
- [ ] Have clear fix strategy

### Phase 2: Tooling (This Week)

**Goal**: Automated GPU verification

- [ ] --test-gpu mode working
- [ ] --robot verbosity implemented
- [ ] Integration tests passing

### Phase 3: Fix (This Week)

**Goal**: yollayah runs on GPU

- [ ] GPU usage 100% (or >90%)
- [ ] Performance matches base model
- [ ] All tests passing

### Final Acceptance Criteria

**MUST ALL BE TRUE**:

1. ✅ yollayah model uses GPU (btop/nvidia-smi confirm)
2. ✅ Performance matches llama3.1:8b base model
3. ✅ Automated tests verify GPU usage
4. ✅ CPU usage < 10% during inference
5. ✅ Solution documented
6. ✅ Works across different systems (CI/CD passes)

---

## Related Issues

- **BUG-009**: Model creation impacts performance (initial investigation)
- **BUG-007**: Interactive mode slow (symptom, not cause)
- **BUG-008**: LD_LIBRARY_PATH override (red herring)

---

## Notes

**This is the actual root cause of all performance issues.**

Every optimization we did (reactive streaming, warmup elimination, bash simplification) was correct and valuable, but didn't address the fundamental problem: **the model runs on CPU instead of GPU**.

**Zero Tolerance Policy**: This EPIC blocks all other work. The model MUST use GPU before we can claim acceptable performance.

---

## Success Metrics

**Before (Current State)**:
- yollayah inference: ~10 tokens/sec (CPU)
- First token: ~2-5 seconds
- CPU usage: 99%
- GPU usage: <1%

**After (Target State)**:
- yollayah inference: 50-100 tokens/sec (GPU)
- First token: < 200ms
- CPU usage: < 10%
- GPU usage: > 90%

**Improvement**: **10x faster inference**
