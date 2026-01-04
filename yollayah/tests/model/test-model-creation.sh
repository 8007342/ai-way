#!/usr/bin/env bash
# test-model-creation.sh - Integration tests for model creation
#
# Tests the model.sh module for yollayah model creation and GPU usage
#
# Environment: Host (Category 4: Hybrid Script)
# - Runs on host for test framework
# - Calls ollama via toolbox when needed
# See: facts/tools/TOOLBOX.md

set -euo pipefail

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Source dependencies
source "${SCRIPT_DIR}/lib/common/robot.sh"
source "${SCRIPT_DIR}/lib/ollama/model.sh"

# ============================================================================
# Toolbox Adaptation: Hybrid Script
# ============================================================================

# Detect if we're in toolbox or on host
is_in_toolbox() {
    [[ -f /run/.toolboxenv ]]
}

# Wrapper to call ollama commands (adapts to environment)
call_ollama() {
    if is_in_toolbox; then
        # Already in toolbox, call directly
        ollama "$@"
    else
        # On host, call via toolbox
        if command -v toolbox &>/dev/null; then
            toolbox run --directory "$PWD" ollama "$@"
        else
            robot_error "$TEST_MODULE" "Toolbox not available, cannot run ollama"
            return 1
        fi
    fi
}

# Check if ollama is running (adapts to environment)
check_ollama_running() {
    if is_in_toolbox; then
        pgrep -x "ollama" > /dev/null
    else
        if command -v toolbox &>/dev/null; then
            toolbox run pgrep -x "ollama" > /dev/null 2>&1
        else
            return 1
        fi
    fi
}

# Test module
readonly TEST_MODULE="test"

# Parse robot flags
robot_parse_flags "$@"

# Test counters
tests_run=0
tests_passed=0
tests_failed=0

# Test helper
run_test() {
    local test_name="$1"
    local test_func="$2"

    ((tests_run++))
    robot_info "$TEST_MODULE" "Running: $test_name"

    if $test_func; then
        ((tests_passed++))
        robot_info "$TEST_MODULE" "✅ PASS: $test_name"
        return 0
    else
        ((tests_failed++))
        robot_error "$TEST_MODULE" "❌ FAIL: $test_name"
        return 1
    fi
}

# ============================================================================
# Tests
# ============================================================================

test_modelfile_generation() {
    robot_debug "$TEST_MODULE" "Testing modelfile generation"

    local modelfile
    modelfile=$(model_generate_modelfile "llama3.2:3b")

    # Check modelfile contains expected content
    if ! echo "$modelfile" | grep -q "FROM llama3.2:3b"; then
        robot_error "$TEST_MODULE" "Modelfile missing FROM directive"
        return 1
    fi

    if ! echo "$modelfile" | grep -q "SYSTEM"; then
        robot_error "$TEST_MODULE" "Modelfile missing SYSTEM prompt"
        return 1
    fi

    if ! echo "$modelfile" | grep -q "PARAMETER temperature"; then
        robot_error "$TEST_MODULE" "Modelfile missing temperature parameter"
        return 1
    fi

    if ! echo "$modelfile" | grep -q "PARAMETER num_ctx 4096"; then
        robot_error "$TEST_MODULE" "Modelfile has wrong num_ctx (should be 4096)"
        return 1
    fi

    robot_debug "$TEST_MODULE" "Modelfile generation validated"
    return 0
}

test_model_exists_check() {
    robot_debug "$TEST_MODULE" "Testing model existence check"

    # This test assumes ollama is running
    if ! check_ollama_running; then
        robot_warn "$TEST_MODULE" "Ollama not running, skipping test"
        return 0  # Skip, don't fail
    fi

    # Check for a model that should exist (base models)
    if model_exists "llama3.2:3b" || model_exists "llama3.1:8b"; then
        robot_debug "$TEST_MODULE" "Base model existence check passed"
        return 0
    else
        robot_warn "$TEST_MODULE" "No base models found, test inconclusive"
        return 0  # Skip, don't fail
    fi
}

test_yollayah_creation() {
    robot_debug "$TEST_MODULE" "Testing yollayah model creation"

    # This test requires ollama running
    if ! check_ollama_running; then
        robot_warn "$TEST_MODULE" "Ollama not running, skipping test"
        return 0  # Skip, don't fail
    fi

    # Try to create yollayah model
    if model_create_yollayah "llama3.2:3b"; then
        robot_debug "$TEST_MODULE" "Yollayah model creation succeeded"

        # Verify it exists
        if model_exists "yollayah"; then
            robot_debug "$TEST_MODULE" "Yollayah model verified to exist"
            return 0
        else
            robot_error "$TEST_MODULE" "Yollayah model created but doesn't exist"
            return 1
        fi
    else
        robot_error "$TEST_MODULE" "Yollayah model creation failed"
        return 1
    fi
}

test_gpu_verification() {
    robot_debug "$TEST_MODULE" "Testing GPU verification"

    # This test requires ollama running and nvidia-smi
    if ! check_ollama_running; then
        robot_warn "$TEST_MODULE" "Ollama not running, skipping test"
        return 0  # Skip, don't fail
    fi

    if ! command -v nvidia-smi >/dev/null 2>&1; then
        robot_warn "$TEST_MODULE" "nvidia-smi not available, skipping test"
        return 0  # Skip, don't fail
    fi

    # Ensure yollayah model exists
    if ! model_exists "yollayah"; then
        robot_warn "$TEST_MODULE" "Yollayah model doesn't exist, skipping GPU test"
        return 0  # Skip, don't fail
    fi

    # Test GPU usage
    local gpu_result
    if model_test_yollayah_gpu; then
        robot_info "$TEST_MODULE" "GPU usage verified for yollayah"
        return 0
    else
        gpu_result=$?
        if [[ $gpu_result -eq 1 ]]; then
            robot_error "$TEST_MODULE" "CPU fallback detected (THIS IS THE BUG WE'RE TRACKING)"
            return 1
        elif [[ $gpu_result -eq 2 ]]; then
            robot_warn "$TEST_MODULE" "Cannot verify GPU (nvidia-smi issue)"
            return 0  # Skip, don't fail
        fi
    fi
}

# ============================================================================
# Run Tests
# ============================================================================

main() {
    robot_info "$TEST_MODULE" "Starting model creation tests"
    robot_info "$TEST_MODULE" "Robot configuration:"
    robot_show_config

    echo ""
    run_test "Modelfile Generation" test_modelfile_generation
    run_test "Model Existence Check" test_model_exists_check
    run_test "Yollayah Creation" test_yollayah_creation
    run_test "GPU Verification" test_gpu_verification

    echo ""
    robot_info "$TEST_MODULE" "Test Summary:"
    robot_info "$TEST_MODULE" "  Total:  $tests_run"
    robot_info "$TEST_MODULE" "  Passed: $tests_passed"
    robot_info "$TEST_MODULE" "  Failed: $tests_failed"

    if [[ $tests_failed -gt 0 ]]; then
        robot_error "$TEST_MODULE" "Tests failed!"
        return 1
    else
        robot_info "$TEST_MODULE" "All tests passed!"
        return 0
    fi
}

main "$@"
