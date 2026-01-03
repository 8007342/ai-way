# TODO-sprint-toolbox-3: GPU Passthrough & Integration Testing

> Sprint 3 of Toolbox Integration Epic: Comprehensive testing of ollama in toolbox, GPU verification, performance benchmarking.
>
> **Created**: 2026-01-02
> **Last Updated**: 2026-01-02 (Sprint 3 - initial creation)
> **Owner**: QA + Backend Dev
> **Sprint Duration**: 2-3 hours
> **Priority**: HIGH
> **Epic**: TODO-epic-2026Q1-toolbox.md
> **Depends On**: Sprint 1 (auto-enter) + Sprint 2 (ollama install)

---

## Sprint Goal

Verify the toolbox integration works end-to-end with real testing. Confirm GPU passthrough, test model inference performance, verify OLLAMA_KEEP_ALIVE, and identify any issues before moving to Sprint 4 (polish).

---

## Success Criteria

- [ ] Fresh toolbox test passes (create → install → inference)
- [ ] GPU detected correctly in toolbox
- [ ] Inference performance meets requirements (< 2s with GPU)
- [ ] OLLAMA_KEEP_ALIVE verified (models stay loaded)
- [ ] No regressions from Sprints 1-2
- [ ] All critical bugs found and filed

---

## Tasks

### Phase 3.1: GPU Verification Script ✅ PENDING

**Owner**: QA + Hacker
**Files**: `scripts/verify-gpu-toolbox.sh` (new)

- [ ] **T3.1.1**: Create GPU verification script
  ```bash
  #!/bin/bash
  # Verify GPU passthrough in toolbox

  echo "=== GPU Verification in Toolbox ==="
  echo ""

  # Check if in toolbox
  if [[ ! -f /run/.toolboxenv ]]; then
      echo "❌ Not running in toolbox"
      echo "Run: toolbox enter ai-way"
      exit 1
  fi

  # Check GPU devices
  echo "1. Checking GPU devices..."
  if ls /dev/nvidia* &> /dev/null; then
      echo "✅ NVIDIA GPU devices found:"
      ls -la /dev/nvidia*
  else
      echo "❌ No NVIDIA GPU devices"
  fi

  # Check nvidia-smi
  echo ""
  echo "2. Checking nvidia-smi..."
  if command -v nvidia-smi &> /dev/null; then
      nvidia-smi --query-gpu=name,driver_version,memory.total --format=csv
  else
      echo "❌ nvidia-smi not available"
  fi

  # Check ollama GPU detection
  echo ""
  echo "3. Checking ollama GPU detection..."
  if command -v ollama &> /dev/null; then
      # This will show if ollama detects GPU
      echo "Running quick inference test..."
      time ollama run qwen2:0.5b "hi" 2>&1 | head -10
  else
      echo "❌ ollama not installed"
  fi

  echo ""
  echo "=== Verification Complete ==="
  ```

- [ ] **T3.1.2**: Make script executable and test
  ```bash
  chmod +x scripts/verify-gpu-toolbox.sh
  toolbox run -c ai-way ./scripts/verify-gpu-toolbox.sh
  ```

- [ ] **T3.1.3**: Add to documentation
  - Reference in TOOLBOX.md troubleshooting section
  - Add to CLAUDE.md testing commands

**Acceptance Criteria**:
- Script detects GPU devices correctly
- Shows nvidia-smi output
- Tests ollama can access GPU
- Clear pass/fail indicators

---

### Phase 3.2: Performance Benchmarking ✅ PENDING

**Owner**: QA
**Files**: Test manually, document in sprint TODO

- [ ] **T3.2.1**: Benchmark qwen2:0.5b inference speed
  ```bash
  # In toolbox, test inference performance
  toolbox enter ai-way

  # First run (cold start)
  time ollama run qwen2:0.5b "Write a haiku"

  # Second run (warm start)
  time ollama run qwen2:0.5b "Write another haiku"

  # Check GPU usage during inference
  # In another terminal:
  watch -n 1 nvidia-smi
  ```

- [ ] **T3.2.2**: Compare toolbox vs host performance
  - Run same test on host (if ollama available)
  - Compare timings
  - Should be nearly identical (container overhead minimal)

- [ ] **T3.2.3**: Document performance baselines
  - Record: Time to first token
  - Record: Tokens per second
  - Record: Total inference time
  - Compare to Sprint 2 expectations (< 2s goal)

**Acceptance Criteria**:
- Inference < 2s with GPU for qwen2:0.5b
- GPU utilization visible during inference
- Performance matches host (if tested)

---

### Phase 3.3: Integration Testing ✅ PENDING

**Owner**: QA + Backend Dev
**Files**: Manual testing, document results

- [ ] **T3.3.1**: Fresh toolbox full flow test
  ```bash
  # Remove existing toolbox
  toolbox rm ai-way -f

  # Run yollayah.sh (should create toolbox, install ollama, launch)
  ./yollayah.sh --test

  # Verify:
  # - Toolbox created
  # - Ollama installed
  # - TUI launched
  # - Can send message and get response
  ```

- [ ] **T3.3.2**: Test OLLAMA_KEEP_ALIVE model persistence
  ```bash
  # In toolbox, start ollama and load model
  toolbox enter ai-way
  ollama run qwen2:0.5b "hi"

  # Check model is loaded
  ollama ps
  # Should show: qwen2:0.5b loaded, UNTIL = 24 hours from now

  # Wait 10 minutes
  sleep 600

  # Check model still loaded
  ollama ps
  # Should STILL show qwen2:0.5b loaded

  # Run inference again (should be fast, no reload)
  time ollama run qwen2:0.5b "hi"
  # Should be < 2s (model stayed in memory)
  ```

- [ ] **T3.3.3**: Test all CLI flags work through toolbox
  ```bash
  # From host
  ./yollayah.sh --help       # Should enter toolbox, show help
  ./yollayah.sh --test       # Should enter toolbox, run test mode
  ./yollayah.sh --version    # Should work

  # Verify environment variables pass through
  YOLLAYAH_DEBUG=1 ./yollayah.sh --test
  # Should show debug output
  ```

- [ ] **T3.3.4**: Test avatar animations work in toolbox
  ```bash
  ./yollayah.sh --test
  # Verify:
  # - Yollayah avatar appears
  # - Animations are smooth (no lag)
  # - No visual artifacts
  ```

**Acceptance Criteria**:
- Fresh toolbox setup works end-to-end
- OLLAMA_KEEP_ALIVE verified (models stay 24h)
- All CLI flags work correctly
- TUI functions normally in toolbox

---

### Phase 3.4: LD_LIBRARY_PATH Optimization Test ✅ PENDING

**Owner**: Hacker
**Files**: `lib/ollama/service.sh`

- [ ] **T3.4.1**: Test ollama WITHOUT LD_LIBRARY_PATH in toolbox
  ```bash
  # Temporarily disable LD_LIBRARY_PATH in lib/ollama/service.sh
  # Line 332-334, comment out LD_LIBRARY_PATH export

  # Test if ollama still detects GPU
  toolbox enter ai-way
  pkill ollama
  ollama serve &

  # Watch logs for CUDA initialization
  # Check if GPU detected without manual library path
  ```

- [ ] **T3.4.2**: Document findings
  - Does GPU work without LD_LIBRARY_PATH in toolbox? (Y/N)
  - If YES: Remove LD_LIBRARY_PATH for toolbox, keep for host
  - If NO: Keep LD_LIBRARY_PATH, document why needed

- [ ] **T3.4.3**: Update code if LD_LIBRARY_PATH not needed
  ```bash
  # If not needed, change to:
  if [[ ! -f /run/.toolboxenv ]]; then
      # Only on host
      LD_LIBRARY_PATH="/usr/lib:/usr/lib64:${LD_LIBRARY_PATH:-}"
  fi
  ```

**Acceptance Criteria**:
- Tested whether LD_LIBRARY_PATH needed in toolbox
- Code updated based on findings
- Documentation reflects the decision

---

## Testing Checklist

### Integration Tests

- [ ] **Test 1**: Fresh Silverblue system (VM recommended)
  - Clone ai-way repo
  - Run `./yollayah.sh --test`
  - Expected: Creates toolbox, installs ollama, launches TUI
  - Verify: Can send message, get response

- [ ] **Test 2**: Existing toolbox, no ollama
  - `toolbox create ai-way` (manual)
  - `./yollayah.sh --test`
  - Expected: Enters toolbox, installs ollama, launches TUI

- [ ] **Test 3**: Existing toolbox, ollama installed
  - Run `./yollayah.sh --test` twice
  - Expected: Second run should be faster (no install)

- [ ] **Test 4**: GPU stress test
  - Run large model (llama3.2:3b) if system can handle
  - Monitor GPU usage with nvidia-smi
  - Verify no memory leaks over time

- [ ] **Test 5**: Multiple models
  - Load qwen2:0.5b, get response
  - Load llama3.2:1b, get response
  - Check both stay loaded (ollama ps)
  - Verify OLLAMA_KEEP_ALIVE applies to all

### Regression Tests

- [ ] **Sprint 1**: Auto-enter still works
  - From host: `./yollayah.sh --help`
  - Should auto-enter toolbox seamlessly

- [ ] **Sprint 1**: Toolbox creation still works
  - Remove toolbox: `toolbox rm ai-way -f`
  - Run: `./yollayah.sh --test`
  - Should create and enter toolbox

- [ ] **Sprint 2**: Ollama install still works
  - Fresh toolbox with no ollama
  - Should auto-install on first run

### Performance Tests

- [ ] Measure startup time (fresh toolbox)
  - Expected: < 3 minutes (create 30s + install 90s + launch 2s)

- [ ] Measure startup time (existing toolbox)
  - Expected: < 5 seconds (enter 1s + launch 2s)

- [ ] Measure inference time (GPU)
  - qwen2:0.5b "hi": < 2 seconds
  - llama3.2:1b "hi": < 3 seconds

- [ ] Measure inference time (CPU, if tested)
  - Should be 5-10x slower than GPU

---

## Bug Tracking

### Bugs Found

_Document any bugs discovered during testing:_

**BUG-XXX**: [Title]
- **Severity**: HIGH/MEDIUM/LOW
- **Location**: file.sh:line
- **Description**: What's wrong
- **Reproduction**: Steps to reproduce
- **Fix**: How to fix (if known)

---

## Performance Baselines

_Record actual measurements:_

| Metric | Target | Actual | Pass/Fail |
|--------|--------|--------|-----------|
| Fresh toolbox setup | < 3 min | TBD | TBD |
| Existing toolbox startup | < 5 sec | TBD | TBD |
| qwen2:0.5b inference (GPU) | < 2 sec | TBD | TBD |
| OLLAMA_KEEP_ALIVE duration | 24 hours | TBD | TBD |
| GPU detection | 100% | TBD | TBD |

---

## Definition of Done

- [ ] All tasks marked complete
- [ ] All integration tests passed
- [ ] All performance baselines met
- [ ] All bugs documented (if any)
- [ ] No regressions from Sprints 1-2
- [ ] GPU verification script created
- [ ] LD_LIBRARY_PATH decision made and documented
- [ ] Sprint retrospective completed

---

## Sprint Retrospective

_Fill after sprint completion:_

**Completed**: TBD
**Status**: TBD

**What went well**:
-

**What could be improved**:
-

**Bugs found**:
-

**Performance results**:
-

**LD_LIBRARY_PATH findings**:
-

**Action items for Sprint 4**:
-

---

**Owner**: QA + Backend Dev
**Last Updated**: 2026-01-02
**Status**: READY TO START
**Sprint Target**: Complete in 2-3 hours
