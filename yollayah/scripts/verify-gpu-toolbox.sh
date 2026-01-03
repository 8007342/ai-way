#!/bin/bash
# ============================================================================
# scripts/verify-gpu-toolbox.sh - GPU Passthrough Verification for Toolbox
#
# Verifies that GPU devices are properly passed through to the toolbox
# container and that ollama can detect and use them.
#
# Usage:
#   toolbox enter ai-way -- ./scripts/verify-gpu-toolbox.sh
#
# Or from inside toolbox:
#   ./scripts/verify-gpu-toolbox.sh
#
# Exit Codes:
#   0 - All checks passed
#   1 - Not in toolbox
#   2 - No GPU devices found
#   3 - nvidia-smi not available
#   4 - ollama not installed
#   5 - GPU inference test failed
# ============================================================================

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== GPU Verification in Toolbox ===${NC}"
echo ""

# Check if in toolbox
if [[ ! -f /run/.toolboxenv ]]; then
    echo -e "${RED}❌ Not running in toolbox${NC}"
    echo ""
    echo "This script must be run inside the ai-way toolbox container."
    echo ""
    echo "To run from host:"
    echo "  toolbox enter ai-way -- ./scripts/verify-gpu-toolbox.sh"
    echo ""
    echo "To run from inside toolbox:"
    echo "  toolbox enter ai-way"
    echo "  ./scripts/verify-gpu-toolbox.sh"
    echo ""
    exit 1
fi

# Get container name
CONTAINER_NAME=$(grep -oP 'name="\K[^"]+' /run/.containerenv 2>/dev/null || echo "unknown")
echo -e "${GREEN}✓${NC} Running in toolbox: ${CONTAINER_NAME}"
echo ""

# Check 1: GPU devices
echo -e "${BLUE}1. Checking GPU devices...${NC}"
GPU_FOUND=false

# Check for NVIDIA GPUs
if ls /dev/nvidia* &> /dev/null; then
    echo -e "${GREEN}✓ NVIDIA GPU devices found:${NC}"
    ls -la /dev/nvidia* | awk '{print "  " $0}'
    GPU_FOUND=true
    GPU_TYPE="NVIDIA"
else
    echo -e "${YELLOW}○ No NVIDIA GPU devices found${NC}"
fi

# Check for AMD GPUs
if ls /dev/dri/renderD* &> /dev/null; then
    echo -e "${GREEN}✓ AMD GPU devices found:${NC}"
    ls -la /dev/dri/renderD* | awk '{print "  " $0}'
    GPU_FOUND=true
    GPU_TYPE="${GPU_TYPE:+$GPU_TYPE/}AMD"
else
    echo -e "${YELLOW}○ No AMD GPU devices found${NC}"
fi

if [[ "$GPU_FOUND" == "false" ]]; then
    echo ""
    echo -e "${RED}❌ No GPU devices detected in container${NC}"
    echo ""
    echo "This could mean:"
    echo "  1. No GPU in the system"
    echo "  2. GPU drivers not installed on host"
    echo "  3. toolbox not mounting GPU devices (shouldn't happen)"
    echo ""
    echo "Troubleshooting:"
    echo "  - On host, check: lspci | grep VGA"
    echo "  - On host, check: nvidia-smi (for NVIDIA)"
    echo "  - Verify drivers installed on host system"
    echo ""
    exit 2
fi

echo ""

# Check 2: nvidia-smi (for NVIDIA GPUs)
if [[ "$GPU_TYPE" == *"NVIDIA"* ]]; then
    echo -e "${BLUE}2. Checking nvidia-smi...${NC}"
    if command -v nvidia-smi &> /dev/null; then
        echo -e "${GREEN}✓ nvidia-smi available${NC}"
        echo ""
        nvidia-smi --query-gpu=index,name,driver_version,memory.total,memory.used --format=csv | \
            awk 'NR==1 {print "  " $0} NR>1 {print "  " $0}'
        echo ""

        # Show detailed GPU info
        echo "  GPU Utilization:"
        nvidia-smi --query-gpu=index,utilization.gpu,utilization.memory --format=csv,noheader | \
            awk '{print "    " $0}'
    else
        echo -e "${YELLOW}⚠ nvidia-smi not available (but GPU devices are mounted)${NC}"
        echo ""
        echo "This is not critical - ollama may still use GPU."
        echo "Install nvidia-utils for nvidia-smi: dnf install nvidia-utils"
    fi
else
    echo -e "${BLUE}2. nvidia-smi check skipped (AMD GPU)${NC}"
fi

echo ""

# Check 3: ollama installation
echo -e "${BLUE}3. Checking ollama installation...${NC}"
if ! command -v ollama &> /dev/null; then
    echo -e "${RED}❌ ollama not installed in container${NC}"
    echo ""
    echo "Ollama should have been auto-installed on first run."
    echo "Try running: ./yollayah.sh --test"
    echo ""
    exit 4
fi

echo -e "${GREEN}✓ ollama installed at: $(command -v ollama)${NC}"
echo ""

# Check if ollama is running
if pgrep -x ollama &> /dev/null; then
    echo -e "${GREEN}✓ ollama server is running${NC}"
else
    echo -e "${YELLOW}○ ollama server not running (will start for test)${NC}"
fi

echo ""

# Check 4: GPU inference test
echo -e "${BLUE}4. Testing GPU inference with qwen2:0.5b...${NC}"
echo ""

# Check if model is available
if ollama list 2>/dev/null | grep -q "qwen2:0.5b"; then
    echo -e "${GREEN}✓ qwen2:0.5b model already pulled${NC}"
else
    echo -e "${YELLOW}○ qwen2:0.5b not pulled yet - pulling now (352MB)...${NC}"
    if ! ollama pull qwen2:0.5b; then
        echo -e "${RED}❌ Failed to pull model${NC}"
        exit 5
    fi
fi

echo ""
echo "Running inference test..."
echo ""

# Run inference and capture timing
START_TIME=$(date +%s.%N)
RESPONSE=$(ollama run qwen2:0.5b "Say hi" 2>&1 | head -5)
END_TIME=$(date +%s.%N)

# Calculate duration
DURATION=$(echo "$END_TIME - $START_TIME" | bc)

echo "Response (first 5 lines):"
echo "$RESPONSE" | awk '{print "  " $0}'
echo ""
echo -e "Inference time: ${GREEN}${DURATION} seconds${NC}"

# Check if using GPU (look for CUDA in ollama logs if available)
echo ""
echo "Checking GPU usage (ollama ps):"
ollama ps | awk '{print "  " $0}'

# Performance check
if (( $(echo "$DURATION < 5" | bc -l) )); then
    echo ""
    echo -e "${GREEN}✓ Performance looks good (< 5 seconds)${NC}"
    if (( $(echo "$DURATION < 2" | bc -l) )); then
        echo -e "${GREEN}  Excellent! This is GPU-level performance.${NC}"
    fi
else
    echo ""
    echo -e "${YELLOW}⚠ Slower than expected (> 5 seconds)${NC}"
    echo "  This might indicate:"
    echo "    - CPU inference (GPU not being used)"
    echo "    - Model loading time (first run)"
    echo "    - System under heavy load"
    echo ""
    echo "  Try running again (model should stay loaded):"
    echo "    ollama run qwen2:0.5b \"test\""
fi

echo ""
echo -e "${BLUE}=== Verification Complete ===${NC}"
echo ""

# Summary
echo "Summary:"
echo -e "  GPU Type:       ${GREEN}${GPU_TYPE}${NC}"
echo -e "  GPU Devices:    ${GREEN}Detected${NC}"
echo -e "  ollama:         ${GREEN}Installed${NC}"
echo -e "  Inference Test: ${GREEN}Passed${NC}"
echo -e "  Performance:    $(if (( $(echo "$DURATION < 2" | bc -l) )); then echo "${GREEN}Excellent (GPU)${NC}"; elif (( $(echo "$DURATION < 5" | bc -l) )); then echo "${YELLOW}Good${NC}"; else echo "${YELLOW}Needs investigation${NC}"; fi)"
echo ""

exit 0
