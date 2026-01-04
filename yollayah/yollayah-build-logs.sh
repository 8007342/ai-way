#!/bin/bash
# ============================================================================
# yollayah-build-log.sh - Build Diagnostics for ai-way
#
# Usage:
#   ./yollayah-build-log.sh [MODE]
#
# Modes:
#   (no args)              Build entire workspace (non-verbose)
#   --all                  Build entire workspace (non-verbose)
#   --all-verbose          Build entire workspace (verbose)
#   --surfaces             Build all surfaces (non-verbose)
#   --surfaces-verbose     Build all surfaces (verbose)
#   --tui                  Build TUI only (non-verbose)
#   --tui-verbose          Build TUI only (verbose)
#   --conductor            Build Conductor only (non-verbose)
#   --conductor-verbose    Build Conductor only (verbose)
#   --install-ollama       Test Ollama installation (non-verbose)
#   --install-ollama-verbose  Test Ollama installation (verbose)
#
# Output:
#   - Build output to stdout (real-time)
#   - Saved to build-log-TIMESTAMP.txt
#   - Uses yollayah.sh library functions for consistency
# ============================================================================

set -euo pipefail

# ============================================================================
# Toolbox Enforcement: MUST run on host
# ============================================================================

# This script MUST run on the host, not inside toolbox
# See: facts/tools/TOOLBOX.md - Category 1: Host-Only Scripts
if [[ -f /run/.toolboxenv ]]; then
    echo "ERROR: This script must run on the HOST, not inside toolbox" >&2
    echo "" >&2
    echo "Why: Build scripts need consistent paths and host Rust toolchain" >&2
    echo "" >&2
    echo "Exit toolbox first:" >&2
    echo "  exit" >&2
    echo "" >&2
    echo "Then run from host:" >&2
    echo "  ./yollayah/yollayah-build-logs.sh $@" >&2
    echo "" >&2
    echo "See: facts/tools/TOOLBOX.md for details" >&2
    exit 1
fi

# ============================================================================
# Bootstrap: Load yollayah.sh library
# ============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export SCRIPT_DIR

# Source yollayah.sh library modules directly
# Note: We skip toolbox auto-enter and main execution by sourcing modules individually

# Core utilities
source "${SCRIPT_DIR}/yollayah/lib/common.sh"

# Robot flag system (must be early for --robot parsing)
source "${SCRIPT_DIR}/yollayah/lib/common/robot.sh"

# Logging infrastructure
source "${SCRIPT_DIR}/yollayah/lib/logging/init.sh"

# UX output
source "${SCRIPT_DIR}/yollayah/lib/ux/output.sh"

# Ollama management
source "${SCRIPT_DIR}/yollayah/lib/ollama/service.sh"
source "${SCRIPT_DIR}/yollayah/lib/ollama/lifecycle.sh"
source "${SCRIPT_DIR}/yollayah/lib/ollama/model.sh"

# ============================================================================
# Configuration
# ============================================================================

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
# Output logs to workdir/logs/ to avoid polluting root
mkdir -p "$SCRIPT_DIR/workdir/logs"
LOG_FILE="$SCRIPT_DIR/workdir/logs/build-log-$TIMESTAMP.txt"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# ============================================================================
# Helper Functions
# ============================================================================

log() {
    echo -e "${BLUE}[BUILD]${NC} $*" | tee -a "$LOG_FILE"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $*" | tee -a "$LOG_FILE"
}

log_error() {
    echo -e "${RED}[✗]${NC} $*" | tee -a "$LOG_FILE"
}

log_warn() {
    echo -e "${YELLOW}[!]${NC} $*" | tee -a "$LOG_FILE"
}

log_section() {
    echo "" | tee -a "$LOG_FILE"
    echo -e "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}" | tee -a "$LOG_FILE"
    echo -e "${MAGENTA} $*${NC}" | tee -a "$LOG_FILE"
    echo -e "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}" | tee -a "$LOG_FILE"
    echo "" | tee -a "$LOG_FILE"
}

# ============================================================================
# Build Functions
# ============================================================================

check_environment() {
    log_section "Environment Check"

    log "Repository root: $SCRIPT_DIR"
    log "Build log: $LOG_FILE"
    log "Timestamp: $TIMESTAMP"
    echo "" | tee -a "$LOG_FILE"

    # Check Rust toolchain
    if command -v rustc &> /dev/null; then
        RUST_VERSION=$(rustc --version)
        log_success "Rust installed: $RUST_VERSION"
    else
        log_error "Rust not installed!"
        exit 1
    fi

    # Check Cargo
    if command -v cargo &> /dev/null; then
        CARGO_VERSION=$(cargo --version)
        log_success "Cargo installed: $CARGO_VERSION"
    else
        log_error "Cargo not installed!"
        exit 1
    fi

    # Note: No workspace Cargo.toml at root (by design - hard requirement)
    # Individual Rust projects are in yollayah/conductor/ and yollayah/core/surfaces/tui/

    echo "" | tee -a "$LOG_FILE"
}

clean_build() {
    log_section "Clean Build Artifacts"

    log "Cleaning TUI build artifacts..."
    (cd "$SCRIPT_DIR/yollayah/core/surfaces/tui" && cargo clean) 2>&1 | tee -a "$LOG_FILE" || {
        log_error "TUI cargo clean failed"
    }

    log "Cleaning Conductor build artifacts..."
    (cd "$SCRIPT_DIR/yollayah/conductor/daemon" && cargo clean) 2>&1 | tee -a "$LOG_FILE" || {
        log_error "Conductor cargo clean failed"
    }

    log_success "Build artifacts cleaned"
}

build_workspace() {
    local verbose="$1"
    log_section "Building All Rust Projects"

    # Note: No workspace Cargo.toml at root (by design)
    # Build each project individually
    log "Building conductor and TUI..."

    local build_failed=0

    # Build conductor first
    build_conductor "$verbose" || build_failed=1

    # Build TUI
    build_tui "$verbose" || build_failed=1

    if [[ $build_failed -eq 0 ]]; then
        log_success "All builds completed"
        return 0
    else
        log_error "One or more builds failed"
        return 1
    fi
}

build_tui() {
    local verbose="$1"
    log_section "Building TUI (yollayah-tui)"

    if [[ "$verbose" == "true" ]]; then
        log "Running: cd yollayah/core/surfaces/tui && cargo build --release --verbose"
        echo "" | tee -a "$LOG_FILE"
        (cd "$SCRIPT_DIR/yollayah/core/surfaces/tui" && cargo build --release --verbose) 2>&1 | tee -a "$LOG_FILE"
    else
        log "Running: cd yollayah/core/surfaces/tui && cargo build --release"
        echo "" | tee -a "$LOG_FILE"
        (cd "$SCRIPT_DIR/yollayah/core/surfaces/tui" && cargo build --release) 2>&1 | tee -a "$LOG_FILE"
    fi

    if [[ ${PIPESTATUS[0]} -eq 0 ]]; then
        log_success "TUI build completed"
        check_binary "yollayah/core/surfaces/tui/target/release/yollayah-tui" "TUI"
        return 0
    else
        log_error "TUI build failed"
        return 1
    fi
}

build_conductor() {
    local verbose="$1"
    log_section "Building Conductor (conductor-core + conductor-daemon)"

    if [[ "$verbose" == "true" ]]; then
        log "Running: cd yollayah/conductor/daemon && cargo build --release --verbose"
        echo "" | tee -a "$LOG_FILE"
        (cd "$SCRIPT_DIR/yollayah/conductor/daemon" && cargo build --release --verbose) 2>&1 | tee -a "$LOG_FILE"
    else
        log "Running: cd yollayah/conductor/daemon && cargo build --release"
        echo "" | tee -a "$LOG_FILE"
        (cd "$SCRIPT_DIR/yollayah/conductor/daemon" && cargo build --release) 2>&1 | tee -a "$LOG_FILE"
    fi

    if [[ ${PIPESTATUS[0]} -eq 0 ]]; then
        log_success "Conductor build completed"
        check_binary "yollayah/conductor/daemon/target/release/conductor-daemon" "Conductor Daemon"
        return 0
    else
        log_error "Conductor build failed"
        return 1
    fi
}

build_yollayah_model() {
    local verbose="$1"
    log_section "Building Yollayah Model"

    # Check if Ollama is running
    if ! pgrep -x "ollama" > /dev/null; then
        log_warn "Ollama not running, attempting to start..."
        ollama serve > /dev/null 2>&1 &
        sleep 2

        # Verify it started
        if ! pgrep -x "ollama" > /dev/null; then
            log_error "Failed to start Ollama"
            return 1
        fi
    fi

    log "Creating yollayah model from llama3.2:3b..."

    if [[ "$verbose" == "true" ]]; then
        # Verbose mode: use robot flags for detailed output
        if ROBOT_MODEL_LEVEL=debug model_create_yollayah "llama3.2:3b" "force" 2>&1 | tee -a "$LOG_FILE"; then
            log_success "Yollayah model created successfully"
            return 0
        else
            log_error "Yollayah model creation failed"
            return 1
        fi
    else
        # Non-verbose: capture output to log only
        if ROBOT_MODEL_LEVEL=info model_create_yollayah "llama3.2:3b" "force" >> "$LOG_FILE" 2>&1; then
            log_success "Yollayah model created successfully"
            return 0
        else
            log_error "Yollayah model creation failed"
            return 1
        fi
    fi
}

test_yollayah_gpu() {
    log_section "Testing Yollayah GPU Usage"

    # Check if nvidia-smi available
    if ! command -v nvidia-smi &> /dev/null; then
        log_warn "nvidia-smi not available, cannot test GPU"
        return 2
    fi

    # Check if yollayah model exists
    if ! model_exists "yollayah"; then
        log_warn "Yollayah model not found, skipping GPU test"
        log "Run: $0 --model to create the model first"
        return 2
    fi

    log "Running GPU verification test..."

    # Use robot flags for detailed GPU output
    if ROBOT_MODEL_LEVEL=info ROBOT_GPU_LEVEL=debug model_test_yollayah_gpu 2>&1 | tee -a "$LOG_FILE"; then
        log_success "GPU usage confirmed for yollayah model"
        return 0
    else
        local exit_code=$?
        if [[ $exit_code -eq 1 ]]; then
            log_error "CPU fallback detected (yollayah not using GPU)"
            log_warn "This is the bug we're investigating in EPIC-001"
        elif [[ $exit_code -eq 2 ]]; then
            log_warn "Cannot verify GPU usage"
        fi
        return $exit_code
    fi
}

test_ollama_install() {
    local verbose="$1"
    log_section "Testing Ollama Installation"

    # Check if Ollama is installed
    if command -v ollama &> /dev/null; then
        OLLAMA_VERSION=$(ollama --version 2>&1 || echo "unknown")
        log_success "Ollama installed: $OLLAMA_VERSION"
    else
        log_warn "Ollama not installed"
        log "Would install with: curl -fsSL https://ollama.com/install.sh | sh"
        return 1
    fi

    # Test Ollama service
    log "Testing Ollama service..."

    if [[ "$verbose" == "true" ]]; then
        # Verbose mode: show full output
        if ollama list 2>&1 | tee -a "$LOG_FILE"; then
            log_success "Ollama service responding"
        else
            log_warn "Ollama service not responding (may need to start)"
        fi
    else
        # Non-verbose: just check if it works
        if ollama list &> /dev/null; then
            log_success "Ollama service responding"
        else
            log_warn "Ollama service not responding (may need to start)"
        fi
    fi

    # Test GPU detection (uses yollayah.sh library function)
    log "Testing GPU detection..."

    # Check for GPU detection functions
    if command -v nvidia-smi &> /dev/null; then
        log "GPU: NVIDIA"
        if [[ "$verbose" == "true" ]]; then
            nvidia-smi --query-gpu=name,memory.total --format=csv,noheader 2>&1 | tee -a "$LOG_FILE" || true
        else
            GPU_INFO=$(nvidia-smi --query-gpu=name,memory.total --format=csv,noheader 2>/dev/null || echo "unknown")
            log "  $GPU_INFO"
        fi
    elif command -v rocm-smi &> /dev/null; then
        log "GPU: AMD ROCm"
        if [[ "$verbose" == "true" ]]; then
            rocm-smi 2>&1 | tee -a "$LOG_FILE" || true
        fi
    else
        log_warn "No GPU detected (NVIDIA or AMD ROCm)"
    fi

    return 0
}

check_binary() {
    local binary_path="$1"
    local component_name="$2"

    log "Checking binary: $binary_path"

    if [[ -f "$SCRIPT_DIR/$binary_path" ]]; then
        local size=$(du -h "$SCRIPT_DIR/$binary_path" | cut -f1)
        log_success "$component_name binary exists ($size)"

        if [[ -x "$SCRIPT_DIR/$binary_path" ]]; then
            log_success "$component_name binary is executable"
        else
            log_warn "$component_name binary is not executable"
        fi
    else
        log_error "$component_name binary not found at $binary_path"
    fi
}

run_smoke_tests() {
    log_section "Smoke Tests"

    # Test TUI binary
    if [[ -x "$SCRIPT_DIR/yollayah/core/surfaces/tui/target/release/yollayah-tui" ]]; then
        log "Testing TUI binary (version check)..."
        if "$SCRIPT_DIR/yollayah/core/surfaces/tui/target/release/yollayah-tui" --version 2>&1 | tee -a "$LOG_FILE"; then
            log_success "TUI binary responds to --version"
        else
            log_warn "TUI binary does not support --version"
        fi
    fi

    # Test conductor-daemon binary
    if [[ -x "$SCRIPT_DIR/yollayah/conductor/daemon/target/release/conductor-daemon" ]]; then
        log "Testing Conductor binary (version check)..."
        if "$SCRIPT_DIR/yollayah/conductor/daemon/target/release/conductor-daemon" --version 2>&1 | tee -a "$LOG_FILE"; then
            log_success "Conductor binary responds to --version"
        else
            log_warn "Conductor binary does not support --version"
        fi
    fi
}

analyze_log() {
    log_section "Build Analysis"

    local error_count=$(grep -c "error:" "$LOG_FILE" 2>/dev/null || true)
    local warning_count=$(grep -c "warning:" "$LOG_FILE" 2>/dev/null || true)
    # grep -c returns 0 when no matches, but exit code 1, so we use || true to ignore exit code
    error_count=${error_count:-0}
    warning_count=${warning_count:-0}

    echo "" | tee -a "$LOG_FILE"
    log "Error count: $error_count"
    log "Warning count: $warning_count"
    echo "" | tee -a "$LOG_FILE"

    if [[ $error_count -gt 0 ]]; then
        log_error "Build completed with $error_count error(s)"
        echo "" | tee -a "$LOG_FILE"
        log "First 10 errors:"
        grep "error:" "$LOG_FILE" | head -10 | tee -a "$LOG_FILE"
    fi

    if [[ $warning_count -gt 0 ]]; then
        log_warn "Build completed with $warning_count warning(s)"
    fi
}

show_usage() {
    cat << EOF
Usage: $0 [--robot=module=level:...] [MODE]

Modes:
  (no args)              Build entire workspace (non-verbose)
  --all                  Build entire workspace (non-verbose)
  --all-verbose          Build entire workspace (verbose)
  --surfaces             Build all surfaces (non-verbose)
  --surfaces-verbose     Build all surfaces (verbose)
  --tui                  Build TUI only (non-verbose)
  --tui-verbose          Build TUI only (verbose)
  --conductor            Build Conductor only (non-verbose)
  --conductor-verbose    Build Conductor only (verbose)
  --install-ollama       Test Ollama installation (non-verbose)
  --install-ollama-verbose  Test Ollama installation (verbose)
  --model                Build yollayah model (non-verbose)
  --model-verbose        Build yollayah model (verbose)
  --test-model-gpu       Test yollayah GPU usage
  --robot-test           Show robot configuration and exit

Robot Flags (module-level verbosity control):
  --robot=module=level:module=level:...

  Modules:
    model    - Model creation and management
    gpu      - GPU verification and diagnostics
    build    - Build process output
    test     - Test execution
    ollama   - Ollama backend operations
    all      - Global override (sets all modules)

  Levels:
    off      - No output
    error    - Errors only
    warn     - Warnings + errors
    info     - Info + above (default)
    debug    - Debug + above
    trace    - Everything (most verbose)
    full     - Alias for trace

Examples:
  $0                                    # Build everything (non-verbose)
  $0 --tui-verbose                      # Build TUI with verbose output
  $0 --install-ollama                   # Test Ollama installation
  $0 --model                            # Build yollayah model
  $0 --model-verbose                    # Build yollayah model (verbose)
  $0 --test-model-gpu                   # Test GPU usage
  $0 --robot=model=debug --model        # Build model with debug output
  $0 --robot=model=trace:gpu=debug --test-model-gpu  # Test GPU with verbose logging
  $0 --robot-test                       # Show robot flag configuration

Output:
  - Real-time output to stdout
  - Full log saved to build-log-TIMESTAMP.txt

See: knowledge/methodology/common/bash/ROBOT-FLAGS.md for details
EOF
}

# ============================================================================
# Main Execution
# ============================================================================

main() {
    # Parse robot flags first
    robot_parse_flags "$@"

    local mode="${1:---all}"
    local verbose="false"
    local build_failed=0

    # Strip robot flag from args if present
    if [[ "$mode" == --robot=* ]]; then
        shift
        mode="${1:---all}"
    fi

    # Parse mode and verbose flag
    case "$mode" in
        --help|-h)
            show_usage
            exit 0
            ;;
        --robot-test)
            robot_show_config
            exit 0
            ;;
        --all|"")
            mode="all"
            verbose="false"
            ;;
        --all-verbose)
            mode="all"
            verbose="true"
            ;;
        --model)
            mode="model"
            verbose="false"
            ;;
        --model-verbose)
            mode="model"
            verbose="true"
            ;;
        --test-model-gpu)
            mode="test-gpu"
            verbose="false"
            ;;
        --surfaces)
            mode="surfaces"
            verbose="false"
            ;;
        --surfaces-verbose)
            mode="surfaces"
            verbose="true"
            ;;
        --tui)
            mode="tui"
            verbose="false"
            ;;
        --tui-verbose)
            mode="tui"
            verbose="true"
            ;;
        --conductor)
            mode="conductor"
            verbose="false"
            ;;
        --conductor-verbose)
            mode="conductor"
            verbose="true"
            ;;
        --install-ollama)
            mode="ollama"
            verbose="false"
            ;;
        --install-ollama-verbose)
            mode="ollama"
            verbose="true"
            ;;
        *)
            log_error "Unknown mode: $mode"
            echo ""
            show_usage
            exit 1
            ;;
    esac

    # Header
    echo -e "${CYAN}" | tee -a "$LOG_FILE"
    echo "╔═══════════════════════════════════════════════════════════════════════╗" | tee -a "$LOG_FILE"
    echo "║                  Yollayah Build Diagnostics                           ║" | tee -a "$LOG_FILE"
    echo "║                  Mode: $mode (verbose: $verbose)                      ║" | tee -a "$LOG_FILE"
    echo "╚═══════════════════════════════════════════════════════════════════════╝" | tee -a "$LOG_FILE"
    echo -e "${NC}" | tee -a "$LOG_FILE"

    check_environment

    # Execute based on mode
    case "$mode" in
        all)
            log "Mode: Full workspace build (verbose: $verbose)"
            clean_build
            build_workspace "$verbose" || build_failed=1
            ;;
        surfaces)
            log "Mode: All surfaces (currently just TUI) (verbose: $verbose)"
            clean_build
            build_tui "$verbose" || build_failed=1
            ;;
        tui)
            log "Mode: TUI only (verbose: $verbose)"
            clean_build
            build_tui "$verbose" || build_failed=1
            ;;
        conductor)
            log "Mode: Conductor only (verbose: $verbose)"
            clean_build
            build_conductor "$verbose" || build_failed=1
            ;;
        ollama)
            log "Mode: Ollama installation test (verbose: $verbose)"
            test_ollama_install "$verbose" || build_failed=1
            ;;
        model)
            log "Mode: Build yollayah model (verbose: $verbose)"
            build_yollayah_model "$verbose" || build_failed=1
            ;;
        test-gpu)
            log "Mode: Test yollayah GPU usage"
            test_yollayah_gpu || build_failed=1
            ;;
    esac

    # Run smoke tests only for build modes
    if [[ "$mode" != "ollama" ]] && [[ "$mode" != "model" ]] && [[ "$mode" != "test-gpu" ]] && [[ $build_failed -eq 0 ]]; then
        run_smoke_tests
    fi

    # Analyze log only for build modes
    if [[ "$mode" != "ollama" ]] && [[ "$mode" != "model" ]] && [[ "$mode" != "test-gpu" ]]; then
        analyze_log
    fi

    log_section "Complete"
    log "Full log saved to: $LOG_FILE"

    if [[ $build_failed -eq 0 ]]; then
        log_success "Operation completed successfully!"
        exit 0
    else
        log_error "Operation failed. Check log for details: $LOG_FILE"
        exit 1
    fi
}

# ============================================================================
# Execute
# ============================================================================

main "$@"
