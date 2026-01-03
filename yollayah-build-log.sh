#!/bin/bash
# ============================================================================
# yollayah-build-log.sh - Verbose Build Diagnostics for ai-way
#
# Usage:
#   ./yollayah-build-log.sh [--tui|--conductor|--surfaces|--all]
#
# Flags:
#   --tui         Build TUI with verbose output
#   --conductor   Build Conductor with verbose output
#   --surfaces    Build all surfaces (currently just TUI)
#   --all         Build entire workspace (default)
#
# Output:
#   - Full build output to stdout (real-time)
#   - Saved to build-log-TIMESTAMP.txt
#   - Highlights errors and warnings
#   - Checks binary existence
#   - Basic smoke tests
# ============================================================================

set -euo pipefail

# ============================================================================
# Configuration
# ============================================================================

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="$REPO_ROOT/build-log-$TIMESTAMP.txt"

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

    log "Repository root: $REPO_ROOT"
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
    if [[ -f "$REPO_ROOT/Cargo.toml" ]]; then
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
    log_section "Building Entire Workspace"

    log "Running: cargo build --workspace --all-features --verbose"
    echo "" | tee -a "$LOG_FILE"

    if cargo build --workspace --all-features --verbose 2>&1 | tee -a "$LOG_FILE"; then
        log_success "Workspace build completed"
        return 0
    else
        log_error "Workspace build failed"
        return 1
    fi
}

build_tui() {
    log_section "Building TUI (yollayah-tui)"

    log "Running: cargo build --package yollayah-tui --release --verbose"
    echo "" | tee -a "$LOG_FILE"

    if cargo build --package yollayah-tui --release --verbose 2>&1 | tee -a "$LOG_FILE"; then
        log_success "TUI build completed"
        check_binary "target/release/yollayah-tui" "TUI"
        return 0
    else
        log_error "TUI build failed"
        return 1
    fi
}

build_conductor() {
    log_section "Building Conductor (conductor-core + conductor-daemon)"

    log "Running: cargo build --package conductor-core --package conductor-daemon --release --verbose"
    echo "" | tee -a "$LOG_FILE"

    if cargo build --package conductor-core --package conductor-daemon --release --verbose 2>&1 | tee -a "$LOG_FILE"; then
        log_success "Conductor build completed"
        check_binary "target/release/conductor-daemon" "Conductor Daemon"
        return 0
    else
        log_error "Conductor build failed"
        return 1
    fi
}

check_binary() {
    local binary_path="$1"
    local component_name="$2"

    log "Checking binary: $binary_path"

    if [[ -f "$REPO_ROOT/$binary_path" ]]; then
        local size=$(du -h "$REPO_ROOT/$binary_path" | cut -f1)
        log_success "$component_name binary exists ($size)"

        if [[ -x "$REPO_ROOT/$binary_path" ]]; then
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
    if [[ -x "$REPO_ROOT/target/release/yollayah-tui" ]]; then
        log "Testing TUI binary (version check)..."
        if "$REPO_ROOT/target/release/yollayah-tui" --version 2>&1 | tee -a "$LOG_FILE"; then
            log_success "TUI binary responds to --version"
        else
            log_warn "TUI binary does not support --version"
        fi
    fi

    # Test conductor-daemon binary
    if [[ -x "$REPO_ROOT/target/release/conductor-daemon" ]]; then
        log "Testing Conductor binary (version check)..."
        if "$REPO_ROOT/target/release/conductor-daemon" --version 2>&1 | tee -a "$LOG_FILE"; then
            log_success "Conductor binary responds to --version"
        else
            log_warn "Conductor binary does not support --version"
        fi
    fi
}

run_unit_tests() {
    log_section "Unit Tests"

    log "Running: cargo test --workspace --lib --verbose"
    echo "" | tee -a "$LOG_FILE"

    if cargo test --workspace --lib --verbose 2>&1 | tee -a "$LOG_FILE"; then
        log_success "Unit tests passed"
        return 0
    else
        log_error "Unit tests failed"
        return 1
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
        log "Errors:"
        grep "error:" "$LOG_FILE" | tee -a "$LOG_FILE"
    fi

    if [[ $warning_count -gt 0 ]]; then
        log_warn "Build completed with $warning_count warning(s)"
    fi
}

# ============================================================================
# Main Execution
# ============================================================================

main() {
    local mode="${1:---all}"

    echo -e "${CYAN}" | tee -a "$LOG_FILE"
    echo "╔═══════════════════════════════════════════════════════════════════════╗" | tee -a "$LOG_FILE"
    echo "║                  Yollayah Build Diagnostics                           ║" | tee -a "$LOG_FILE"
    echo "║                      Verbose Build Log                                ║" | tee -a "$LOG_FILE"
    echo "╚═══════════════════════════════════════════════════════════════════════╝" | tee -a "$LOG_FILE"
    echo -e "${NC}" | tee -a "$LOG_FILE"

    check_environment

    local build_failed=0

    case "$mode" in
        --tui)
            log "Mode: TUI only"
            clean_build
            build_tui || build_failed=1
            ;;
        --conductor)
            log "Mode: Conductor only"
            clean_build
            build_conductor || build_failed=1
            ;;
        --surfaces)
            log "Mode: All surfaces (currently just TUI)"
            clean_build
            build_tui || build_failed=1
            ;;
        --all)
            log "Mode: Full workspace"
            clean_build
            build_workspace || build_failed=1
            ;;
        *)
            log_error "Unknown mode: $mode"
            echo ""
            echo "Usage: $0 [--tui|--conductor|--surfaces|--all]"
            exit 1
            ;;
    esac

    if [[ $build_failed -eq 0 ]]; then
        run_smoke_tests
        run_unit_tests || true  # Don't fail on test failures
    fi

    analyze_log

    log_section "Build Complete"
    log "Full build log saved to: $LOG_FILE"

    if [[ $build_failed -eq 0 ]]; then
        log_success "Build completed successfully!"
        exit 0
    else
        log_error "Build failed. Check log for details: $LOG_FILE"
        exit 1
    fi
}

# ============================================================================
# Execute
# ============================================================================

main "$@"
