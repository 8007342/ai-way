# TODO-STORY-002-Integration-Testing-Tools

**Status**: 🔴 ACTIVE - Development
**Created**: 2026-01-03
**Priority**: P0 - BLOCKING
**Owner**: Bash Expert, DevOps, QA
**Parent**: [TODO-EPIC-001-yollayah-gpu-model-generation.md](TODO-EPIC-001-yollayah-gpu-model-generation.md)

---

## Goal

**Build automated integration testing tools to verify GPU usage and provide diagnostic output.**

Extend yollayah.sh and yollayah-build-logs.sh with GPU verification and component-level verbosity controls for rapid diagnostics and CI/CD integration.

---

## Background

**Current Testing Limitations**:
- Manual GPU verification (watching btop/nvidia-smi)
- No automated pass/fail for GPU usage
- Limited diagnostic output
- Can't run quick GPU checks in CI/CD

**Need**:
- Automated GPU verification: `./yollayah.sh --test-gpu`
- Component-level verbosity: `--robot=tui=full:conductor=warn`
- Integration with build tools
- Fast diagnostic output for debugging

---

## Tasks

### Task 1: Implement --test-gpu Mode ⏳

**Goal**: Quick automated GPU verification

**File**: `yollayah.sh`

**Design**:
```bash
./yollayah.sh --test-gpu

# Flow:
# 1. Ensure ollama running
# 2. Create/verify yollayah model exists
# 3. Run test inference: "test"
# 4. Monitor GPU usage during inference
# 5. EXIT 0 if GPU used, EXIT 1 if CPU fallback
```

**Implementation**:
```bash
# In yollayah.sh, add flag handling
if [[ "$1" == "--test-gpu" ]]; then
    test_gpu_usage
    exit $?
fi

test_gpu_usage() {
    echo "Testing GPU usage for yollayah model..."

    # Start test inference in background
    ollama run yollayah "test" > /dev/null 2>&1 &
    local pid=$!

    # Wait for inference to start
    sleep 1

    # Check GPU usage
    if command -v nvidia-smi >/dev/null 2>&1; then
        # NVIDIA GPU
        if nvidia-smi | grep -q ollama; then
            echo "✅ GPU usage detected"
            kill $pid 2>/dev/null
            return 0
        else
            echo "❌ No GPU usage detected (CPU fallback)"
            kill $pid 2>/dev/null
            return 1
        fi
    else
        echo "⚠️  nvidia-smi not available, cannot verify GPU"
        kill $pid 2>/dev/null
        return 2
    fi
}
```

**Success Criteria**:
- [⏳] `--test-gpu` flag implemented
- [⏳] EXIT 0 when GPU used
- [⏳] EXIT 1 when CPU fallback
- [⏳] EXIT 2 when cannot verify
- [⏳] Clear output messages

---

### Task 2: Implement --robot Verbosity Flags ⏳

**Goal**: Component-level diagnostic output

**File**: `yollayah.sh`

**Syntax**:
```bash
./yollayah.sh --robot=component1=level:component2=level:...

# Examples:
--robot=tui=full:conductor=off:proto=warn
--robot=all=debug
--robot=ollama=trace:gpu=full
```

**Components**:
- `tui` - TUI surface logs
- `conductor` - Conductor core logs
- `ollama` - Ollama backend logs
- `proto` - Protocol/messaging logs
- `gpu` - GPU detection and diagnostics
- `bash` - Bash wrapper logs
- `all` - All components

**Levels**:
- `off` - No output
- `error` - Errors only
- `warn` - Warnings and errors
- `info` - Info, warnings, errors
- `debug` - Debug and above
- `trace` - Everything
- `full` - Alias for trace

**Implementation**:
```bash
# Parse --robot flag
parse_robot_flags() {
    local robot_spec="$1"

    # Split by colon
    IFS=':' read -ra components <<< "$robot_spec"

    for comp_spec in "${components[@]}"; do
        IFS='=' read -r component level <<< "$comp_spec"

        case "$component" in
            tui)
                export YOLLAYAH_TUI_LOG_LEVEL="$level"
                ;;
            conductor)
                export YOLLAYAH_CONDUCTOR_LOG_LEVEL="$level"
                ;;
            ollama)
                export YOLLAYAH_OLLAMA_LOG_LEVEL="$level"
                ;;
            all)
                export YOLLAYAH_LOG_LEVEL="$level"
                ;;
            # ... more components
        esac
    done
}

# Usage
if [[ "$1" == --robot=* ]]; then
    robot_spec="${1#--robot=}"
    parse_robot_flags "$robot_spec"
    shift
fi
```

**Success Criteria**:
- [⏳] --robot flag parsed correctly
- [⏳] Component-level environment variables set
- [⏳] Logging respects verbosity levels
- [⏳] Can combine multiple components
- [⏳] Clear documentation in --help

---

### Task 3: GPU Verification Script ⏳

**Goal**: Standalone GPU verification utility

**File**: `yollayah/scripts/verify-gpu.sh`

**Purpose**:
- Called by --test-gpu
- Used in CI/CD
- Can verify any model

**Design**:
```bash
#!/usr/bin/env bash
# verify-gpu.sh - Verify Ollama model uses GPU

set -euo pipefail

MODEL="${1:-yollayah}"
PROMPT="${2:-test}"
TIMEOUT="${3:-5}"

echo "Verifying GPU usage for model: $MODEL"

# Start inference in background
ollama run "$MODEL" "$PROMPT" > /dev/null 2>&1 &
pid=$!

# Monitor GPU for TIMEOUT seconds
sleep 1
gpu_detected=false

for i in $(seq 1 "$TIMEOUT"); do
    if nvidia-smi | grep -q ollama; then
        gpu_detected=true
        break
    fi
    sleep 1
done

# Clean up
kill $pid 2>/dev/null || true

# Report results
if [[ "$gpu_detected" == "true" ]]; then
    echo "✅ GPU usage confirmed"
    exit 0
else
    echo "❌ GPU not used (CPU fallback)"
    exit 1
fi
```

**Success Criteria**:
- [⏳] Standalone script created
- [⏳] Works with any model
- [⏳] Configurable timeout
- [⏳] Clear exit codes
- [⏳] Used by --test-gpu

---

### Task 4: Extend yollayah-build-logs.sh ⏳

**Goal**: Add GPU verification to build script

**File**: `yollayah/yollayah-build-logs.sh`

**Add flags**:
```bash
./yollayah-build-logs.sh --verify-gpu
./yollayah-build-logs.sh --all --verify-gpu
./yollayah-build-logs.sh --robot=tui=debug:conductor=off
```

**Implementation**:
```bash
# Add flag handling
VERIFY_GPU=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --verify-gpu)
            VERIFY_GPU=true
            shift
            ;;
        --robot=*)
            ROBOT_FLAGS="${1#--robot=}"
            shift
            ;;
        # ... existing flags
    esac
done

# After build, verify GPU if requested
if [[ "$VERIFY_GPU" == "true" ]]; then
    echo "Running GPU verification..."
    if ./scripts/verify-gpu.sh yollayah; then
        echo "✅ GPU verification passed"
    else
        echo "❌ GPU verification failed"
        exit 1
    fi
fi
```

**Success Criteria**:
- [⏳] --verify-gpu flag implemented
- [⏳] Runs after successful build
- [⏳] Fails build if GPU not used
- [⏳] Integrates with --robot flags

---

### Task 5: Integration Test Suite ⏳

**Goal**: Comprehensive automated tests

**File**: `yollayah/tests/integration/test-gpu-usage.sh`

**Tests**:
```bash
#!/usr/bin/env bash
# Integration test: GPU usage

test_base_model_gpu() {
    echo "Test: Base model uses GPU"
    ./scripts/verify-gpu.sh llama3.2:3b "test"
}

test_yollayah_model_gpu() {
    echo "Test: Yollayah model uses GPU"
    ./scripts/verify-gpu.sh yollayah "test"
}

test_gpu_persistence() {
    echo "Test: GPU usage persists across calls"
    for i in {1..5}; do
        ./scripts/verify-gpu.sh yollayah "test $i"
    done
}

# Run all tests
test_base_model_gpu
test_yollayah_model_gpu
test_gpu_persistence

echo "✅ All GPU tests passed"
```

**Success Criteria**:
- [⏳] Test suite created
- [⏳] Tests base model GPU usage
- [⏳] Tests yollayah GPU usage
- [⏳] Tests GPU persistence
- [⏳] Can run in CI/CD

---

### Task 6: Documentation ⏳

**Goal**: Update docs with new tooling

**Files to Update**:
- `README.md` - Add --test-gpu and --robot to usage
- `CLAUDE.md` - Document integration testing
- `yollayah.sh --help` - Show new flags

**Documentation**:
```markdown
## Integration Testing

### GPU Verification

Test if yollayah model uses GPU:
```bash
./yollayah.sh --test-gpu
```

Exit codes:
- 0: GPU used ✅
- 1: CPU fallback ❌
- 2: Cannot verify ⚠️

### Diagnostic Verbosity

Control component-level logging:
```bash
./yollayah.sh --robot=tui=full:conductor=off:ollama=debug
```

Components: tui, conductor, ollama, proto, gpu, bash, all
Levels: off, error, warn, info, debug, trace, full
```

**Success Criteria**:
- [⏳] README updated
- [⏳] CLAUDE.md updated
- [⏳] --help shows new flags
- [⏳] Examples provided

---

## Implementation Order

1. **Task 3**: GPU verification script (foundation)
2. **Task 1**: --test-gpu mode (uses Task 3)
3. **Task 2**: --robot verbosity flags
4. **Task 4**: Extend yollayah-build-logs.sh
5. **Task 5**: Integration test suite
6. **Task 6**: Documentation

---

## Success Criteria

**All Must Pass**:
- [⏳] `./yollayah.sh --test-gpu` verifies GPU usage automatically
- [⏳] `--robot=component=level` controls verbosity
- [⏳] `./yollayah-build-logs.sh --verify-gpu` fails on CPU fallback
- [⏳] Integration tests pass in CI/CD
- [⏳] Documentation complete
- [⏳] Exit codes consistent and documented

---

## Timeline

**Target**: Complete within 1 day

- Task 3: 1 hour (verification script)
- Task 1: 1 hour (--test-gpu mode)
- Task 2: 2 hours (--robot flags)
- Task 4: 30 min (build script extension)
- Task 5: 1 hour (integration tests)
- Task 6: 30 min (documentation)

**Total**: ~6 hours

---

## Notes

**These tools are critical for ongoing development.**

Once built, they'll enable:
- Fast iteration on GPU fixes
- Automated regression detection
- CI/CD GPU verification
- Quick diagnostics when things break

**Invest the time to build them properly.**
