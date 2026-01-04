#!/usr/bin/env bash
# verify-gpu.sh - Verify Ollama model uses GPU
#
# Usage: ./verify-gpu.sh [MODEL] [PROMPT] [TIMEOUT]
#
# Exit codes:
#   0 - GPU usage detected (success)
#   1 - CPU fallback detected (failure)
#   2 - Cannot verify (nvidia-smi not available)

set -euo pipefail

# Configuration
MODEL="${1:-yollayah}"
PROMPT="${2:-test}"
TIMEOUT="${3:-5}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "Verifying GPU usage for model: $MODEL"

# Check if nvidia-smi available
if ! command -v nvidia-smi >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  nvidia-smi not available, cannot verify GPU${NC}"
    exit 2
fi

# Check if ollama is running
if ! pgrep -x "ollama" > /dev/null; then
    echo -e "${RED}❌ Ollama not running${NC}"
    exit 1
fi

# Start inference in background
echo "Starting inference: ollama run $MODEL \"$PROMPT\""
ollama run "$MODEL" "$PROMPT" > /dev/null 2>&1 &
inference_pid=$!

# Wait for inference to actually start
sleep 1

# Monitor GPU for TIMEOUT seconds
gpu_detected=false
gpu_memory_used=false

echo "Monitoring GPU for $TIMEOUT seconds..."

for i in $(seq 1 "$TIMEOUT"); do
    # Check if ollama process is using GPU
    if nvidia-smi --query-compute-apps=pid,process_name,used_memory --format=csv,noheader 2>/dev/null | grep -q "ollama"; then
        gpu_detected=true
        gpu_memory_used=true
        echo -e "${GREEN}  GPU activity detected at ${i}s${NC}"
        break
    fi

    # Fallback: check if any GPU utilization happening
    if nvidia-smi --query-gpu=utilization.gpu --format=csv,noheader,nounits | awk '$1 > 5 {exit 0} {exit 1}'; then
        gpu_detected=true
        echo -e "${YELLOW}  GPU utilization detected at ${i}s (>5%)${NC}"
    fi

    sleep 1
done

# Clean up inference process
kill $inference_pid 2>/dev/null || true
wait $inference_pid 2>/dev/null || true

# Report results
echo ""
if [[ "$gpu_detected" == "true" ]]; then
    if [[ "$gpu_memory_used" == "true" ]]; then
        echo -e "${GREEN}✅ GPU usage confirmed (ollama process in GPU memory)${NC}"
        exit 0
    else
        echo -e "${YELLOW}⚠️  GPU utilization detected but ollama not in compute apps${NC}"
        echo -e "${YELLOW}   This might indicate partial GPU usage${NC}"
        exit 0
    fi
else
    echo -e "${RED}❌ No GPU usage detected (CPU fallback)${NC}"

    # Show helpful debug info
    echo ""
    echo "Debug information:"
    echo "  Model: $MODEL"
    echo "  Timeout: $TIMEOUT seconds"
    echo ""
    echo "Try:"
    echo "  1. Run 'nvidia-smi' to check GPU is available"
    echo "  2. Test with base model: ./verify-gpu.sh llama3.2:3b"
    echo "  3. Check Ollama logs for GPU detection"
    echo "  4. Increase timeout: ./verify-gpu.sh $MODEL \"$PROMPT\" 10"

    exit 1
fi
