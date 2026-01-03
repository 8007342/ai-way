#!/bin/bash
# Run all stress tests for ai-way TUI/Conductor

set -e

echo "=========================================="
echo "ai-way Stress Test Suite"
echo "=========================================="
echo ""
echo "This will run 5 comprehensive stress tests:"
echo "  1. 1M Short Messages (~45s)"
echo "  2. 1M Medium Messages (~100s)"
echo "  3. 1K Long Messages (~3s)"
echo "  4. Rapid Token Streaming (~15s)"
echo "  5. Backpressure Handling (~12s)"
echo ""
echo "Total time: ~3-5 minutes"
echo ""
echo "Press Ctrl+C to cancel, or wait 5 seconds to continue..."
sleep 5

cd tui

echo ""
echo "Building tests in release mode..."
cargo test --test stress_test --release --no-run

echo ""
echo "=========================================="
echo "Running stress tests..."
echo "=========================================="
echo ""

cargo test --test stress_test --release -- --nocapture

echo ""
echo "=========================================="
echo "All stress tests completed!"
echo "=========================================="
echo ""
echo "See documentation:"
echo "  - STRESS_TESTING.md - Overview and results"
echo "  - tui/tests/STRESS_TEST_GUIDE.md - Detailed guide"
echo "  - tui/tests/STRESS_TEST_RESULTS.md - Example outputs"
echo ""
