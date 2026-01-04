# Toolbox Architecture and Limitations

**Status**: ACTIVE
**Created**: 2026-01-03
**Platform**: Fedora Silverblue (immutable OS)

---

## Overview

ai-way uses toolbox containers for dependency isolation on Fedora Silverblue. This document defines **what runs where** and **why**.

---

## Architecture: Host vs Toolbox

### What Runs INSIDE Toolbox

**Runtime components** (require dependencies not on immutable host):

| Component | Why Inside Toolbox | Command |
|-----------|-------------------|---------|
| **Ollama** | Requires CUDA, model files, GPU passthrough | `toolbox run ollama serve` |
| **Conductor** | Runtime binary, needs Ollama API access | `toolbox run conductor-daemon` |
| **TUI** | Runtime binary, interactive terminal | `toolbox run yollayah-tui` |
| **yollayah.sh** | Main entry point, orchestrates runtime | `./yollayah.sh` (auto-enters) |

**Key principle**: Anything that **runs at user interaction time** goes in toolbox.

### What Runs OUTSIDE Toolbox (Host)

**Build and test infrastructure** (works with host toolchain):

| Component | Why Outside Toolbox | Command |
|-----------|---------------------|---------|
| **cargo build** | Uses host Rust toolchain | `cargo build --release` |
| **Integration tests** | Tests workspace structure on host | `./yollayah/tests/**/*.sh` |
| **Build scripts** | Compiles binaries, no runtime deps | `./yollayah/yollayah-build-logs.sh` |
| **Model tests** | Tests model creation (calls into toolbox) | `./yollayah/tests/model/*.sh` |

**Key principle**: Anything that **builds or tests code** runs on host.

---

## Critical Limitations

### 1. Build Scripts MUST Run on Host

**Problem**: Running build scripts inside toolbox causes path confusion.

**Symptoms**:
```bash
# Inside toolbox
./yollayah/yollayah-build-logs.sh
# ❌ May fail with path errors, environment variable conflicts
```

**Solution**: Always run build scripts from host:
```bash
# Outside toolbox (on host)
./yollayah/yollayah-build-logs.sh
# ✅ Works correctly
```

**Why**: Build scripts need consistent `SCRIPT_DIR` paths that match the repository structure on the host filesystem.

### 2. Runtime Components MUST Run in Toolbox

**Problem**: Running runtime components on host fails due to missing dependencies.

**Symptoms**:
```bash
# On host (outside toolbox)
ollama serve
# ❌ Command not found (not installed on immutable host)
```

**Solution**: Let yollayah.sh auto-enter toolbox:
```bash
./yollayah.sh
# ✅ Automatically enters toolbox, starts ollama, runs TUI
```

### 3. GPU Passthrough Only Works in Toolbox

**Problem**: GPU is not accessible on immutable host.

**Symptoms**:
```bash
# On host
nvidia-smi
# ❌ Command not found or permission denied
```

**Solution**: GPU operations must happen in toolbox:
```bash
toolbox run nvidia-smi
# ✅ Shows GPU
```

---

## Guidelines for Scripts

### Detection: Am I in Toolbox?

```bash
#!/usr/bin/env bash

# Check if running inside toolbox
is_in_toolbox() {
    [[ -f /run/.toolboxenv ]]
}

if is_in_toolbox; then
    echo "Running inside toolbox"
else
    echo "Running on host"
fi
```

### Enforcement: Build Scripts on Host

**Pattern**: Build scripts MUST detect and refuse to run in toolbox.

```bash
#!/usr/bin/env bash
# yollayah-build-logs.sh - Build diagnostics

# MUST run on host
if [[ -f /run/.toolboxenv ]]; then
    echo "ERROR: This script must run on the HOST, not inside toolbox" >&2
    echo "" >&2
    echo "Exit toolbox first:" >&2
    echo "  exit" >&2
    echo "" >&2
    echo "Then run from host:" >&2
    echo "  ./yollayah/yollayah-build-logs.sh" >&2
    exit 1
fi

# ... rest of script
```

### Enforcement: Runtime Scripts in Toolbox

**Pattern**: Runtime scripts should auto-enter toolbox if not already inside.

```bash
#!/usr/bin/env bash
# yollayah.sh - Main entry point

# Auto-enter toolbox if needed
if [[ ! -f /run/.toolboxenv ]] && command -v toolbox &>/dev/null; then
    echo "Entering ai-way toolbox..."
    exec toolbox run --directory "$PWD" bash "$0" "$@"
fi

# ... rest of script (now guaranteed to be in toolbox)
```

### Pattern: Call Toolbox Commands from Host

**Pattern**: Host scripts can call toolbox commands when needed.

```bash
#!/usr/bin/env bash
# Test script running on host

# Test GPU verification (must run in toolbox)
test_gpu() {
    if command -v toolbox &>/dev/null; then
        toolbox run bash -c "source yollayah/lib/ollama/model.sh && model_test_yollayah_gpu"
    else
        echo "Toolbox not available, skipping GPU test"
        return 2
    fi
}
```

---

## Script Classification

### Category 1: Host-Only Scripts

**MUST refuse to run in toolbox:**

- `yollayah/yollayah-build-logs.sh` - Build diagnostics
- `yollayah/tests/architectural-enforcement/*.sh` - Integration tests
- Any script that does `cargo build`, `cargo test`, file structure validation

**Enforcement**: Add toolbox detection at the top:
```bash
if [[ -f /run/.toolboxenv ]]; then
    echo "ERROR: Must run on host. Exit toolbox first." >&2
    exit 1
fi
```

### Category 2: Toolbox-Only Scripts

**MUST auto-enter toolbox if needed:**

- `yollayah.sh` - Main entry point
- Scripts that call `ollama` commands directly
- Scripts that need GPU access

**Enforcement**: Add auto-enter at the top:
```bash
if [[ ! -f /run/.toolboxenv ]] && command -v toolbox &>/dev/null; then
    exec toolbox run --directory "$PWD" bash "$0" "$@"
fi
```

### Category 3: Flexible Scripts

**Can run in either environment:**

- `yollayah/lib/common/robot.sh` - Pure bash utilities
- `yollayah/lib/logging/*.sh` - Logging utilities
- Scripts that only do file I/O, no external commands

**Enforcement**: No special handling needed.

### Category 4: Hybrid Scripts

**Run on host, but call into toolbox when needed:**

- `yollayah/tests/model/test-model-creation.sh` - Runs on host, calls ollama via toolbox
- `yollayah/scripts/verify-gpu.sh` - May need to detect environment

**Enforcement**: Detect environment and adapt:
```bash
call_ollama() {
    if [[ -f /run/.toolboxenv ]]; then
        # Already in toolbox, call directly
        ollama "$@"
    else
        # On host, call via toolbox
        toolbox run ollama "$@"
    fi
}
```

---

## Common Scenarios

### Scenario 1: Developer Building from Host

```bash
# On host (outside toolbox)
./yollayah/yollayah-build-logs.sh --all
# ✅ Builds conductor and TUI using host cargo

# Binary now exists at:
# yollayah/conductor/daemon/target/release/conductor-daemon
# yollayah/core/surfaces/tui/target/release/yollayah-tui
```

### Scenario 2: Developer Running TUI

```bash
# On host
./yollayah.sh
# ✅ Auto-enters toolbox
# ✅ Starts ollama
# ✅ Launches TUI
```

### Scenario 3: Testing Model Creation

```bash
# On host
./yollayah/tests/model/test-model-creation.sh
# ✅ Runs test framework on host
# ✅ Calls ollama commands via toolbox when needed
```

### Scenario 4: CI/CD Pipeline

```bash
# CI runs on host (no toolbox)
./yollayah/yollayah-build-logs.sh --all
# ✅ Builds successfully

# Runtime tests need toolbox
if command -v toolbox &>/dev/null; then
    toolbox create ai-way || true
    toolbox run --directory "$PWD" ./yollayah.sh --test
fi
```

---

## Troubleshooting

### Error: "This script must run on the HOST"

**Cause**: You're running a build script inside toolbox.

**Solution**:
```bash
exit  # Exit toolbox
./yollayah/yollayah-build-logs.sh  # Run on host
```

### Error: "ollama: command not found"

**Cause**: Trying to run ollama on host (outside toolbox).

**Solution**:
```bash
./yollayah.sh  # Auto-enters toolbox
# OR
toolbox run ollama serve
```

### Error: Path confusion (/yollayah/yollayah/...)

**Cause**: `SCRIPT_DIR` being set incorrectly, often due to running in wrong environment.

**Solution**: Ensure build scripts run on host, runtime scripts run in toolbox.

### TUI Smoke Test Fails with "requires a terminal (TTY)"

**Cause**: TUI is designed for interactive use, can't run in non-TTY environments.

**Solution**: This is expected behavior. TUI smoke tests should be skipped in non-interactive environments.

---

## Implementation Checklist

When creating a new script, ask:

1. **Does it call `cargo build/test`?** → Host-only (Category 1)
2. **Does it call `ollama` or need GPU?** → Toolbox-only (Category 2)
3. **Does it only do file I/O?** → Flexible (Category 3)
4. **Does it coordinate between host/toolbox?** → Hybrid (Category 4)

Then add appropriate enforcement:
- Category 1: Add toolbox detection and exit
- Category 2: Add auto-enter toolbox
- Category 3: No special handling
- Category 4: Add environment detection and adaptation

---

## Related Documents

- `knowledge/platform/TOOLBOX.md` - General toolbox usage guide (user-facing)
- `CLAUDE.md` - Build commands and toolbox overview
- `yollayah.sh` - Reference implementation of auto-enter pattern

---

**The Golden Rule**: Build on host. Run in toolbox. Test smartly.
