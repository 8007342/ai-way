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
# Bootstrap: Load yollayah.sh library
# ============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export SCRIPT_DIR

# Source yollayah.sh library modules directly
# Note: We skip toolbox auto-enter and main execution by sourcing modules individually

# Core utilities
source "${SCRIPT_DIR}/lib/common.sh"

# Logging infrastructure
source "${SCRIPT_DIR}/lib/logging/init.sh"

# UX output
source "${SCRIPT_DIR}/lib/ux/output.sh"

# Ollama management
source "${SCRIPT_DIR}/lib/ollama/service.sh"
source "${SCRIPT_DIR}/lib/ollama/lifecycle.sh"

# ============================================================================
# Configuration
# ============================================================================

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="$SCRIPT_DIR/build-log-$TIMESTAMP.txt"

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

    # Check workspace
    if [[ -f "$SCRIPT_DIR/Cargo.toml" ]]; then
        log_success "Workspace manifest found"
    else
        log_error "Cargo.toml not found!"
        exit 1
    fi

    echo "" | tee -a "$LOG_FILE"
}

clean_build() {
    log_section "Clean Build Artifacts"

    log "Running cargo clean..."
    cargo clean 2>&1 | tee -a "$LOG_FILE" || {
        log_error "cargo clean failed"
        return 1
    }

    log_success "Build artifacts cleaned"
}

build_workspace() {
    local verbose="$1"
    log_section "Building Entire Workspace"

    if [[ "$verbose" == "true" ]]; then
        log "Running: cargo build --workspace --all-features --verbose"
        echo "" | tee -a "$LOG_FILE"
        cargo build --workspace --all-features --verbose 2>&1 | tee -a "$LOG_FILE"
    else
        log "Running: cargo build --workspace --all-features"
        echo "" | tee -a "$LOG_FILE"
        cargo build --workspace --all-features 2>&1 | tee -a "$LOG_FILE"
    fi

    if [[ ${PIPESTATUS[0]} -eq 0 ]]; then
        log_success "Workspace build completed"
        return 0
    else
        log_error "Workspace build failed"
        return 1
    fi
}

build_tui() {
    local verbose="$1"
    log_section "Building TUI (yollayah-tui)"

    if [[ "$verbose" == "true" ]]; then
        log "Running: cargo build --package yollayah-tui --release --verbose"
        echo "" | tee -a "$LOG_FILE"
        cargo build --package yollayah-tui --release --verbose 2>&1 | tee -a "$LOG_FILE"
    else
        log "Running: cargo build --package yollayah-tui --release"
        echo "" | tee -a "$LOG_FILE"
        cargo build --package yollayah-tui --release 2>&1 | tee -a "$LOG_FILE"
    fi

    if [[ ${PIPESTATUS[0]} -eq 0 ]]; then
        log_success "TUI build completed"
        check_binary "target/release/yollayah-tui" "TUI"
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
        log "Running: cargo build --package conductor-core --package conductor-daemon --release --verbose"
        echo "" | tee -a "$LOG_FILE"
        cargo build --package conductor-core --package conductor-daemon --release --verbose 2>&1 | tee -a "$LOG_FILE"
    else
        log "Running: cargo build --package conductor-core --package conductor-daemon --release"
        echo "" | tee -a "$LOG_FILE"
        cargo build --package conductor-core --package conductor-daemon --release 2>&1 | tee -a "$LOG_FILE"
    fi

    if [[ ${PIPESTATUS[0]} -eq 0 ]]; then
        log_success "Conductor build completed"
        check_binary "target/release/conductor-daemon" "Conductor Daemon"
        return 0
    else
        log_error "Conductor build failed"
        return 1
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
    if [[ -x "$SCRIPT_DIR/target/release/yollayah-tui" ]]; then
        log "Testing TUI binary (version check)..."
        if "$SCRIPT_DIR/target/release/yollayah-tui" --version 2>&1 | tee -a "$LOG_FILE"; then
            log_success "TUI binary responds to --version"
        else
            log_warn "TUI binary does not support --version"
        fi
    fi

    # Test conductor-daemon binary
    if [[ -x "$SCRIPT_DIR/target/release/conductor-daemon" ]]; then
        log "Testing Conductor binary (version check)..."
        if "$SCRIPT_DIR/target/release/conductor-daemon" --version 2>&1 | tee -a "$LOG_FILE"; then
            log_success "Conductor binary responds to --version"
        else
            log_warn "Conductor binary does not support --version"
        fi
    fi
}

analyze_log() {
    log_section "Build Analysis"

    local error_count=$(grep -c "error:" "$LOG_FILE" 2>/dev/null || echo "0")
    local warning_count=$(grep -c "warning:" "$LOG_FILE" 2>/dev/null || echo "0")

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
Usage: $0 [MODE]

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

Examples:
  $0                     # Build everything (non-verbose)
  $0 --tui-verbose       # Build TUI with verbose output
  $0 --install-ollama    # Test Ollama installation

Output:
  - Real-time output to stdout
  - Full log saved to build-log-TIMESTAMP.txt
EOF
}

# ============================================================================
# Main Execution
# ============================================================================

main() {
    local mode="${1:---all}"
    local verbose="false"
    local build_failed=0

    # Parse mode and verbose flag
    case "$mode" in
        --help|-h)
            show_usage
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
    esac

    # Run smoke tests only for build modes
    if [[ "$mode" != "ollama" ]] && [[ $build_failed -eq 0 ]]; then
        run_smoke_tests
    fi

    # Analyze log only for build modes
    if [[ "$mode" != "ollama" ]]; then
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
