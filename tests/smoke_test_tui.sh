#!/bin/bash
# ============================================================================
# TUI Binary Smoke Test
#
# Verifies that:
# 1. TUI binary builds successfully
# 2. Binary exists at expected location
# 3. Binary can launch (if TTY available)
#
# Exit codes:
#   0 - All checks pass
#   1 - Build or binary missing (FAIL)
#   0 - Build succeeds but can't test launch (no TTY) - SKIP with warning
# ============================================================================

set -e

# Get script directory (repo root is parent)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

echo "=== TUI Binary Smoke Test ==="
echo ""

# ============================================================================
# Test 1: Build TUI
# ============================================================================

echo "Test 1: Building TUI..."
cd "$REPO_ROOT"

if cargo build --release -p yollayah-tui 2>&1 | tail -20; then
    echo "✅ PASS: TUI build succeeded"
else
    echo "❌ FAIL: TUI build failed"
    exit 1
fi

echo ""

# ============================================================================
# Test 2: Verify Binary Exists
# ============================================================================

echo "Test 2: Verifying binary exists..."

if [[ ! -f "$REPO_ROOT/tui/target/release/yollayah-tui" ]]; then
    echo "❌ FAIL: TUI binary not found at tui/target/release/yollayah-tui"
    exit 1
fi

echo "✅ PASS: TUI binary exists"
echo ""

# ============================================================================
# Test 3: Runtime Launch (Requires TTY)
# ============================================================================

echo "Test 3: Testing binary launch..."

# Check if we have a TTY for runtime test
if ! [ -t 0 ] || ! [ -t 1 ]; then
    echo "⏭️  SKIP: No TTY available (CI or non-interactive environment)"
    echo "   Binary builds successfully, but can't test execution without TTY"
    echo ""
    echo "=== Smoke Test Complete (Build OK, Runtime Skipped) ==="
    exit 0
fi

# Try to launch TUI (with timeout and immediate exit)
echo "Attempting TUI launch (will auto-exit)..."

# Use timeout to kill after 3 seconds, and send Ctrl+C to gracefully exit
timeout 3 "$REPO_ROOT/tui/target/release/yollayah-tui" 2>&1 | head -10 &
TUI_PID=$!

# Give it a moment to start
sleep 1

# Check if still running
if ps -p $TUI_PID > /dev/null 2>&1; then
    echo "✅ PASS: TUI started successfully (killing test instance)"
    kill $TUI_PID 2>/dev/null || true
    wait $TUI_PID 2>/dev/null || true
else
    # Check exit code - timeout returns 124, normal exit is 0
    wait $TUI_PID
    exit_code=$?

    if [[ $exit_code -eq 124 ]]; then
        echo "✅ PASS: TUI ran and was killed by timeout"
    elif [[ $exit_code -eq 0 ]]; then
        echo "✅ PASS: TUI exited cleanly"
    else
        echo "❌ FAIL: TUI exited with error code $exit_code"
        exit 1
    fi
fi

echo ""
echo "=== Smoke Test Complete (All Tests Passed) ==="
