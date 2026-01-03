# TODO-ollama-gpu: Ollama GPU Detection Issues

**Created**: 2026-01-02
**Status**: ACTIVE
**Priority**: HIGH
**Owner**: Hacker + DevOps

---

## Problem Statement

Ollama is **running on CPU instead of GPU** despite:
- NVIDIA RTX A5000 (24GB VRAM) detected correctly
- nvidia-smi working properly (driver version 580.119.02)
- CUDA libraries present in ldconfig
- Hardware detection script correctly identifies GPU

**Impact**: 10-100x slower inference performance. User paid for GPU but getting CPU-only performance.

---

## Environment Details

### Host System
- **OS**: Fedora Silverblue (immutable ostree-based)
- **GPU**: NVIDIA RTX A5000 (24GB VRAM)
- **Driver**: 580.119.02 (verified via nvidia-smi)
- **CUDA**: Libraries present (/usr/lib64/libcuda.so)

### Ollama Installation Attempts

1. **Attempt 1**: Installed in ostree (system-level)
   - Result: Runs on CPU

2. **Attempt 2**: Installed via official script (`curl ... | sh`)
   - Result: Runs on CPU
   - Both attempts done AFTER nvidia-smi was working

### Current Detection Output

From yollayah.sh startup:
```
▸ Detecting GPU hardware
→ Checking: NVIDIA GPU
✓ Found: nvidia-smi
→ Running: nvidia-smi --query-gpu=name,memory.total,driver_version
  └─ GPU: NVIDIA RTX A5000, 24564 MiB, 580.119.02
→ Checking: NVIDIA CUDA availability
✓ Found: /usr/lib64/libcuda.so
  └─ CUDA: available
▸ GPU detection summary
  └─ Using: NVIDIA GPU acceleration

⚠  GPU detected but Ollama running on CPU - performance will be slower
ℹ  To fix: reinstall Ollama with 'curl -fsSL https://ollama.com/install.sh | sh'
```

**Our script detects GPU correctly**, but Ollama itself doesn't use it.

---

## Root Cause Hypotheses

### H1: Ollama Binary Not Built with GPU Support
- Downloaded binary might be CPU-only variant
- Check: `ldd $(which ollama)` to see if it links against CUDA

### H2: LD_LIBRARY_PATH Not Set
- Ollama can't find CUDA libraries at runtime
- Silverblue's immutable design might affect library paths
- Check: Environment variables when ollama runs

### H3: Container/Sandbox Isolation (Silverblue-specific)
- Distrobox jail might not expose GPU to applications
- Flatpak/toolbox containers need special GPU passthrough
- Check: Can other GPU apps access the GPU in distrobox?

### H4: Driver/CUDA Version Mismatch
- Driver 580.119.02 might not be compatible with Ollama's CUDA version
- Check: CUDA version Ollama was built against

### H5: systemd Service vs Manual Start
- GPU device permissions differ between systemd and user session
- Check: Does `ollama run` vs `ollama serve` behave differently?

---

## Investigation Tasks

### Phase 1: Diagnosis (Sprint 10)

- [ ] **G1.1**: Check Ollama binary GPU support
  ```bash
  ldd $(which ollama) | grep -i cuda
  ldd $(which ollama) | grep -i nvidia
  file $(which ollama)
  ```

- [ ] **G1.2**: Check Ollama environment at runtime
  ```bash
  # While ollama is running:
  cat /proc/$(pgrep ollama)/environ | tr '\0' '\n' | grep -i cuda
  cat /proc/$(pgrep ollama)/environ | tr '\0' '\n' | grep -i ld_library
  ```

- [ ] **G1.3**: Test GPU access in distrobox
  ```bash
  # Can other apps see the GPU?
  nvidia-smi
  glxinfo | grep -i nvidia
  vulkaninfo | grep -i nvidia
  ```

- [ ] **G1.4**: Check Ollama version and build info
  ```bash
  ollama --version
  ollama show llama3.1:70b --modelfile 2>&1 | head -20
  ```

- [ ] **G1.5**: Test manual GPU specification
  ```bash
  # Try forcing GPU:
  CUDA_VISIBLE_DEVICES=0 ollama serve
  # or
  HSA_OVERRIDE_GFX_VERSION=11.0.0 ollama serve  # AMD
  ```

### Phase 2: Fixes (Sprint 10) - IN PROGRESS

**Solution**: Add `/usr/lib` to library search path when starting Ollama

#### Option A: LD_LIBRARY_PATH (Quick Fix) - ✓ IMPLEMENTED

- [x] **G2.1**: Modify `lib/ollama/service.sh` to set LD_LIBRARY_PATH
  - Modified lib/ollama/service.sh:144
  - Added: `LD_LIBRARY_PATH="/usr/lib:/usr/lib64:${LD_LIBRARY_PATH:-}"`
  - Ollama now finds CUDA libraries at startup

- [x] **G2.2**: Test with manual start
  - Tested: `LD_LIBRARY_PATH=/usr/lib:/usr/lib64 ollama serve`
  - Result: ✓ SUCCESS - Ollama detected GPU!
  - Log shows: `library=CUDA name=CUDA0 description="NVIDIA RTX A5000" total="24.0 GiB"`

- [x] **G2.3**: Verify GPU is actually used
  - Confirmed GPU detection in Ollama logs
  - nvidia-smi shows Ollama process can access GPU
  - 22.8 GiB of 24.0 GiB VRAM available

#### Option B: System-wide ldconfig (NOT Recommended)

- [ ] **G2.4**: ~~Add `/usr/lib` to ldconfig search path~~
  **REJECTED for Silverblue**: rpm-ostree immutable filesystem
  - Would require ostree overlay (violates immutability)
  - Not appropriate for ai-way-lite (PJ/Tlatoani expects zero config)
  - Runtime vars are the Silverblue-native solution

#### Option C: Symlinks (NOT Recommended)

- [ ] **G2.5**: ~~Create symlinks in standard location~~
  **REJECTED for Silverblue**: Same reason as Option B
  - Requires sudo and filesystem modifications
  - Violates Silverblue's immutable design
  - toolbx containers shouldn't modify host system

**DECISION**: Option A (LD_LIBRARY_PATH) is the **correct solution** for Silverblue/toolbx:
- ✓ No system modifications (respects immutability)
- ✓ Runtime-only configuration (proper for containers)
- ✓ Zero user configuration (PJ/Tlatoani just runs it)
- ✓ Self-contained within ai-way (Law of Care: "do no harm")

### Phase 3: Prevention (Sprint 11+)

- [ ] **G3.1**: Add GPU verification to startup
  - Don't just detect GPU, verify Ollama is USING it
  - Parse `ollama ps` or API output to confirm GPU usage

- [ ] **G3.2**: Document Silverblue-specific setup
  - Create docs/silverblue-setup.md
  - Include GPU passthrough instructions
  - Add to CLAUDE.md

- [ ] **G3.3**: Add GPU health check
  - Periodic verification that GPU is being used
  - Alert user if GPU drops to CPU
  - Auto-fix if possible (restart with correct env vars)

---

## Detection Code Analysis

### Current GPU Detection Logic

**File**: `lib/ollama/lifecycle.sh`

**Lines 139-207**: `diagnose_gpu_setup()` function
- Correctly detects nvidia-smi
- Correctly detects CUDA libraries
- Correctly identifies GPU model and VRAM

**Lines 103-135**: `check_ollama_gpu_status()` function
- Queries Ollama API for GPU usage
- **This is where it detects CPU-only mode**
- Line 114: `curl -s http://localhost:11434/api/tags`

**Problem**: Our detection is good, but Ollama itself doesn't see/use the GPU.

---

## Monitoring Strategy

Since this is a recurring pain point, add monitoring:

### Detection Script
```bash
#!/bin/bash
# check-ollama-gpu.sh

# 1. Verify nvidia-smi works
nvidia-smi &>/dev/null || { echo "ERROR: nvidia-smi failed"; exit 1; }

# 2. Verify Ollama is running
ollama list &>/dev/null || { echo "ERROR: Ollama not running"; exit 1; }

# 3. Check Ollama's GPU usage
# Parse ollama ps output or API response
# Look for GPU device assignment

# 4. Report status
echo "GPU Status: OK/DEGRADED/FAILED"
```

### Integration
- Run check during yollayah.sh startup
- Add to TODO-integration-testing.md
- Run periodically (every 5 min?) while TUI is active

---

## Related Issues

- Upstream Ollama GPU detection: https://github.com/ollama/ollama/issues (search)
- Silverblue GPU passthrough: https://docs.fedoraproject.org/en-US/fedora-silverblue/
- NVIDIA Container Toolkit: https://github.com/NVIDIA/nvidia-container-toolkit

---

## Questions

### Active

| ID | Question | Owner | Status |
|----|----------|-------|--------|
| Q1 | Does distrobox need special flags for GPU? | DevOps | Investigating |
| Q2 | Which Ollama binary variant is installed? | DevOps | Investigating |
| Q3 | What CUDA version does Ollama require? | DevOps | Investigating |
| Q4 | Can we build Ollama from source on Silverblue? | DevOps | Pending |

### Resolved

*None yet*

---

## Workarounds (Temporary)

Until fixed, user can:
1. Use smaller models that fit in RAM (slower but functional)
2. Manually start Ollama with GPU env vars
3. Use Ollama in a standard Fedora VM (not Silverblue)

**NOT ACCEPTABLE** for production. This must be fixed.

---

**Owner**: Hacker + DevOps
**Last Updated**: 2026-01-02 (Initial investigation)
