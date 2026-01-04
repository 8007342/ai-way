#!/bin/bash
# ============================================================================
# run-integration-tests.sh - Integration Test Runner for ai-way
#
# Usage:
#   ./yollayah/scripts/run-integration-tests.sh [MODE]
#
# Modes:
#   --small    Quick smoke tests only (~30s)
#   --medium   Important integration tests (~2-5min)
#   --all      All tests except long-running ones (~5-10min, default)
#   --full     Everything including stress/chaos tests (~15-30min)
#
# Exit Codes:
#   0 - All tests passed
#   1 - One or more tests failed
# ============================================================================

set -euo pipefail

# ============================================================================
# Setup
# ============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export SCRIPT_DIR

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Test tracking
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0
FAILED_TESTS=()

# ============================================================================
# Helper Functions
# ============================================================================

log_section() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN} $*${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

log_test() {
    echo -e "${BLUE}[TEST]${NC} $*"
}

log_pass() {
    echo -e "${GREEN}[✓]${NC} $*"
}

log_fail() {
    echo -e "${RED}[✗]${NC} $*"
}

log_skip() {
    echo -e "${YELLOW}[⏭]${NC} $*"
}

# Run a test and track results
run_test() {
    local test_name="$1"
    local test_cmd="$2"

    ((TESTS_RUN++))
    log_test "$test_name"

    if eval "$test_cmd" &> /dev/null; then
        ((TESTS_PASSED++))
        log_pass "$test_name"
        return 0
    else
        ((TESTS_FAILED++))
        FAILED_TESTS+=("$test_name")
        log_fail "$test_name"
        return 1
    fi
}

# ============================================================================
# Test Categories
# ============================================================================

run_smoke_tests() {
    log_section "Smoke Tests (Quick Validation)"

    # TUI smoke test
    if [[ -x "$SCRIPT_DIR/tests/smoke_test_tui.sh" ]]; then
        run_test "TUI Smoke Test" "$SCRIPT_DIR/tests/smoke_test_tui.sh"
    else
        log_skip "TUI smoke test not found"
    fi

    # Architectural enforcement (CRITICAL - always run)
    run_test "Architectural Enforcement" \
        "cd $SCRIPT_DIR/tests/architectural-enforcement && cargo test --quiet"
}

run_unit_tests() {
    log_section "Unit Tests"

    # TUI unit tests
    run_test "TUI Unit Tests" \
        "cd $SCRIPT_DIR/core/surfaces/tui && cargo test --lib --quiet"

    # Conductor core unit tests
    run_test "Conductor Core Unit Tests" \
        "cd $SCRIPT_DIR/conductor/core && cargo test --lib --quiet"
}

run_integration_tests() {
    log_section "Integration Tests"

    # TUI integration tests (fast ones)
    run_test "TUI Integration Tests" \
        "cd $SCRIPT_DIR/core/surfaces/tui && cargo test --test integration_test --quiet"

    # Conductor integration tests
    run_test "Conductor Integration Tests" \
        "cd $SCRIPT_DIR/conductor/core && cargo test --test integration_tests --quiet"
}

run_performance_tests() {
    log_section "Performance Tests"

    # TUI CPU performance
    run_test "TUI CPU Performance Tests" \
        "cd $SCRIPT_DIR/core/surfaces/tui && cargo test --test cpu_performance_test --quiet"

    # Conductor routing performance
    run_test "Conductor Routing Performance" \
        "cd $SCRIPT_DIR/conductor/core && cargo test --test routing_performance_tests --quiet"
}

run_stress_tests() {
    log_section "Stress Tests (Long-Running)"

    log_test "TUI Stress Tests (this may take several minutes...)"
    if cd "$SCRIPT_DIR/core/surfaces/tui" && cargo test --test stress_test --quiet; then
        ((TESTS_PASSED++))
        log_pass "TUI Stress Tests"
    else
        ((TESTS_FAILED++))
        FAILED_TESTS+=("TUI Stress Tests")
        log_fail "TUI Stress Tests"
    fi
    ((TESTS_RUN++))

    log_test "Conductor Chaos Tests (this may take several minutes...)"
    if cd "$SCRIPT_DIR/conductor/core" && cargo test --test chaos_tests --quiet; then
        ((TESTS_PASSED++))
        log_pass "Conductor Chaos Tests"
    else
        ((TESTS_FAILED++))
        FAILED_TESTS+=("Conductor Chaos Tests")
        log_fail "Conductor Chaos Tests"
    fi
    ((TESTS_RUN++))

    # TUI multi-model integration
    log_test "TUI Multi-Model Integration (this may take several minutes...)"
    if cd "$SCRIPT_DIR/core/surfaces/tui" && cargo test --test multi_model_integration_test --quiet; then
        ((TESTS_PASSED++))
        log_pass "TUI Multi-Model Integration"
    else
        ((TESTS_FAILED++))
        FAILED_TESTS+=("TUI Multi-Model Integration")
        log_fail "TUI Multi-Model Integration"
    fi
    ((TESTS_RUN++))
}

# ============================================================================
# Test Suites
# ============================================================================

run_small_suite() {
    log_section "Test Suite: SMALL (Quick validation ~30s)"
    run_smoke_tests
}

run_medium_suite() {
    log_section "Test Suite: MEDIUM (Important tests ~2-5min)"
    run_smoke_tests
    run_unit_tests
    run_integration_tests
}

run_all_suite() {
    log_section "Test Suite: ALL (Everything except long-running ~5-10min)"
    run_smoke_tests
    run_unit_tests
    run_integration_tests
    run_performance_tests
}

run_full_suite() {
    log_section "Test Suite: FULL (Complete test coverage ~15-30min)"
    run_smoke_tests
    run_unit_tests
    run_integration_tests
    run_performance_tests
    run_stress_tests
}

# ============================================================================
# Summary Report
# ============================================================================

print_summary() {
    echo ""
    log_section "Test Summary"

    echo -e "${BLUE}Total Tests:${NC}   $TESTS_RUN"
    echo -e "${GREEN}Passed:${NC}        $TESTS_PASSED"
    echo -e "${RED}Failed:${NC}        $TESTS_FAILED"

    if [[ $TESTS_FAILED -gt 0 ]]; then
        echo ""
        echo -e "${RED}Failed Tests:${NC}"
        for test in "${FAILED_TESTS[@]}"; do
            echo -e "  ${RED}✗${NC} $test"
        done
        echo ""
        echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo -e "${RED} TESTS FAILED${NC}"
        echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        return 1
    else
        echo ""
        echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo -e "${GREEN} ALL TESTS PASSED ✓${NC}"
        echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        return 0
    fi
}

show_usage() {
    cat << EOF
Usage: $0 [MODE]

Test Modes:
  --small    Quick smoke tests only (~30s)
             - Smoke tests
             - Architectural enforcement

  --medium   Important integration tests (~2-5min)
             - Smoke tests
             - Unit tests
             - Integration tests

  --all      All tests except long-running ones (~5-10min, default)
             - Smoke tests
             - Unit tests
             - Integration tests
             - Performance tests

  --full     Everything including stress/chaos tests (~15-30min)
             - All of the above
             - Stress tests
             - Chaos tests
             - Multi-model integration

Examples:
  $0                  # Run --all (default)
  $0 --small          # Quick validation
  $0 --medium         # Pre-commit testing
  $0 --full           # Before major releases

Exit Codes:
  0 - All tests passed
  1 - One or more tests failed
EOF
}

# ============================================================================
# Main
# ============================================================================

main() {
    local mode="${1:---all}"

    # Header
    echo -e "${CYAN}"
    echo "╔═══════════════════════════════════════════════════════════════════════╗"
    echo "║              ai-way Integration Test Runner                           ║"
    echo "╚═══════════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"

    case "$mode" in
        --small)
            run_small_suite
            ;;
        --medium)
            run_medium_suite
            ;;
        --all)
            run_all_suite
            ;;
        --full)
            run_full_suite
            ;;
        --help|-h)
            show_usage
            exit 0
            ;;
        *)
            echo -e "${RED}Error: Unknown mode '$mode'${NC}"
            echo ""
            show_usage
            exit 1
            ;;
    esac

    print_summary
}

# ============================================================================
# Execute
# ============================================================================

main "$@"
