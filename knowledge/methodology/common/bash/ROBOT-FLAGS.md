# ROBOT-FLAGS Methodology

**Category**: Common Bash Conventions
**Status**: REQUIRED for all ai-way bash scripts
**Created**: 2026-01-03

---

## Principle

**All bash scripts in the ai-way project MUST support the `--robot` flag for configurable module-level verbosity.**

This enables:
- Fine-grained debugging control
- Automated testing with selective output
- CI/CD diagnostics
- Integration testing without noise

---

## Syntax

```bash
./script.sh --robot=module1=level:module2=level:...
```

**Components**:
- `module`: Name of the logical module (e.g., `tui`, `conductor`, `ollama`, `gpu`)
- `level`: Verbosity level (see below)
- `:`: Separator between module specifications

**Example**:
```bash
./yollayah.sh --robot=tui=debug:conductor=warn:ollama=trace:gpu=off
```

This sets:
- `tui` module to `debug` level
- `conductor` module to `warn` level
- `ollama` module to `trace` level
- `gpu` module to `off` (no output)

---

## Log Levels

From least to most verbose:

| Level | Numeric | Description | Use Case |
|-------|---------|-------------|----------|
| `off` | 0 | No output | Silence noisy modules |
| `error` | 1 | Errors only | Production, critical issues |
| `warn` | 2 | Warnings + errors | Default for non-critical modules |
| `info` | 3 | Info + above | Default global level |
| `debug` | 4 | Debug + above | Development, troubleshooting |
| `trace` | 5 | Everything | Deep debugging |
| `full` | 5 | Alias for `trace` | Convenience |

**Filtering**: A message at level `warn` will only print if the module's level is `warn` or higher (`info`, `debug`, `trace`).

---

## Implementation

### 1. Source the Utility

Every script must source the robot utility:

```bash
#!/usr/bin/env bash

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source robot flag system
source "${SCRIPT_DIR}/lib/common/robot.sh"

# Parse flags early
robot_parse_flags "$@"
```

### 2. Define Modules

Each script defines its modules. Common modules:

- `tui` - Terminal UI output
- `conductor` - Conductor core logic
- `ollama` - Ollama backend operations
- `gpu` - GPU detection and diagnostics
- `model` - Model creation and management
- `build` - Build process output
- `test` - Test execution
- `bash` - General bash wrapper logic

**Custom modules** are allowed and encouraged.

### 3. Log Messages

Use the convenience functions:

```bash
# Error (level 1)
robot_error "module_name" "Something went wrong: $error_msg"

# Warning (level 2)
robot_warn "module_name" "Deprecated function called"

# Info (level 3)
robot_info "module_name" "Starting operation..."

# Debug (level 4)
robot_debug "module_name" "Variable value: $var"

# Trace (level 5)
robot_trace "module_name" "Entering function: ${FUNCNAME[0]}"
```

**Output format**:
```
[module_name] LEVEL: message
```

Example:
```
[ollama] INFO: Starting ollama serve
[gpu] DEBUG: Detected NVIDIA GPU: RTX 3090
[tui] TRACE: Rendering frame 42
```

### 4. Conditional Logging

For expensive operations, check before logging:

```bash
if robot_should_log "module_name" "debug"; then
    local expensive_data=$(generate_debug_info)
    robot_debug "module_name" "Debug data: $expensive_data"
fi
```

---

## Standard Modules

All ai-way scripts should recognize these standard modules:

| Module | Purpose |
|--------|---------|
| `tui` | TUI surface output |
| `conductor` | Conductor core |
| `ollama` | Ollama backend |
| `gpu` | GPU detection/diagnostics |
| `model` | Model creation/management |
| `bash` | Bash wrapper logic |
| `build` | Build process |
| `test` | Testing output |
| `all` | Global override (sets all modules) |

---

## Passthrough to Subcommands

When calling subcommands, pass the --robot flag through:

```bash
# Get the robot flag
robot_flag=$(robot_get_flag "$@")

# Pass to subcommand
if [[ -n "$robot_flag" ]]; then
    ./subcommand.sh "$robot_flag" other args
else
    ./subcommand.sh other args
fi
```

Or use the utility:

```bash
# Strip robot flag and get cleaned args
cleaned_args=$(robot_strip_flag "$@")

# Pass robot flag separately
robot_flag=$(robot_get_flag "$@")

./subcommand.sh $robot_flag $cleaned_args
```

---

## Examples

### Example 1: Debugging TUI Issues

```bash
./yollayah.sh --robot=tui=trace:conductor=off:ollama=off
```

Shows all TUI output, silences conductor and ollama.

### Example 2: Integration Testing

```bash
./yollayah-build-logs.sh --robot=build=info:test=debug:model=trace
```

Build at info level, tests at debug, model creation at trace.

### Example 3: GPU Diagnostics

```bash
./yollayah.sh --test-gpu --robot=gpu=trace:ollama=debug:all=off
```

Only GPU and Ollama output, everything else silent.

### Example 4: Production Mode

```bash
./yollayah.sh --robot=all=error
```

Only errors printed across all modules.

---

## Integration with Existing Logging

Scripts with existing logging systems should map to robot levels:

```bash
# Old style
log_info "message"

# New style
robot_info "module_name" "message"

# Compatibility layer
log_info() {
    robot_info "bash" "$@"
}
```

---

## Testing

Scripts SHOULD provide `--robot-test` mode to verify flag handling:

```bash
if [[ "$1" == "--robot-test" ]]; then
    robot_parse_flags "$@"
    robot_show_config

    robot_error "test" "Error message"
    robot_warn "test" "Warning message"
    robot_info "test" "Info message"
    robot_debug "test" "Debug message"
    robot_trace "test" "Trace message"

    exit 0
fi
```

**Test**:
```bash
./script.sh --robot-test --robot=test=debug
```

**Expected output**:
```
Robot Configuration:
  Global level: info
  test: debug
[test] ERROR: Error message
[test] WARN: Warning message
[test] INFO: Info message
[test] DEBUG: Debug message
```

(No trace message since test=debug, not trace)

---

## Requirements

### MUST

- ✅ All bash scripts MUST support `--robot` flag
- ✅ All scripts MUST source `lib/common/robot.sh`
- ✅ All scripts MUST call `robot_parse_flags "$@"` early
- ✅ All logging MUST use `robot_*` functions

### SHOULD

- ⚠️ Scripts SHOULD define standard modules where applicable
- ⚠️ Scripts SHOULD pass `--robot` to subcommands
- ⚠️ Scripts SHOULD provide `--robot-test` mode

### MAY

- 💡 Scripts MAY define custom modules
- 💡 Scripts MAY provide module aliases
- 💡 Scripts MAY add color coding to output

---

## Benefits

1. **Debugging**: Turn on verbose output only for the module you're debugging
2. **Testing**: Silence noisy modules during integration tests
3. **CI/CD**: Automated tests can control output verbosity
4. **Performance**: Conditional logging prevents expensive operations when not needed
5. **Consistency**: Standardized across all ai-way bash scripts

---

## See Also

- `yollayah/lib/common/robot.sh` - Implementation
- `PRINCIPLE-efficiency.md` - Lazy evaluation, minimal overhead
- `TODO-EPIC-001` - GPU model generation (uses robot flags extensively)

---

**The golden rule**: If it outputs to stderr/stdout, it should use robot flags.
