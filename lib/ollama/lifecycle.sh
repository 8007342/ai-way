#!/bin/bash
# ============================================================================
# lib/ollama/lifecycle.sh - Model Lifecycle Management
#
# This module handles:
# - Hardware detection (GPU, VRAM, RAM)
# - Model selection based on hardware
# - Pulling models
# - Updating models when newer versions available
#
# Model Selection Philosophy:
# We want to give AJ the best experience their hardware can handle.
# A gaming laptop with 8GB VRAM can run bigger models than we default to.
# But we start conservative and let hardware detection optimize.
#
# Constitution Reference:
# - Law of Service: "Serve genuine interests" - best model for their hardware
# - Law of Care: "First, do no harm" - don't OOM their system
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_OLLAMA_LIFECYCLE_LOADED:-}" ]] && return 0
_YOLLAYAH_OLLAMA_LIFECYCLE_LOADED=1

# ============================================================================
# Model Configuration
# ============================================================================

# Default model (conservative, works on most hardware)
readonly DEFAULT_MODEL="llama3.2:3b"

# Model tiers based on VRAM
# Format: "min_vram_gb:model_name"
readonly MODEL_TIERS=(
    "16:llama3.1:70b"     # 16GB+ VRAM: Full power
    "12:llama3.1:8b"      # 12GB+ VRAM: Great quality
    "8:llama3.2:3b"       # 8GB+ VRAM: Good balance
    "6:llama3.2:3b"       # 6GB+ VRAM: Default
    "4:llama3.2:1b"       # 4GB+ VRAM: Lightweight
    "0:llama3.2:1b"       # Fallback: CPU inference
)

# ============================================================================
# Hardware Detection
# ============================================================================

# Check if NVIDIA CUDA is available for Ollama
# Returns 0 if CUDA appears functional, 1 otherwise
check_nvidia_cuda_available() {
    # Check if nvidia-smi works
    if ! command_exists nvidia-smi; then
        log_ollama "DEBUG" "nvidia-smi not found"
        return 1
    fi

    # Check if nvidia-smi can query the GPU
    if ! nvidia-smi --query-gpu=name --format=csv,noheader &>/dev/null; then
        log_ollama "WARN" "nvidia-smi found but cannot query GPU"
        return 1
    fi

    # Check for CUDA libraries
    local cuda_found=false
    if [[ -d "/usr/local/cuda" ]] || [[ -d "/usr/lib/cuda" ]]; then
        cuda_found=true
    fi
    if ldconfig -p 2>/dev/null | grep -q libcuda; then
        cuda_found=true
    fi
    if [[ -f "/usr/lib64/libcuda.so" ]] || [[ -f "/usr/lib/x86_64-linux-gnu/libcuda.so" ]]; then
        cuda_found=true
    fi

    if [[ "$cuda_found" == "false" ]]; then
        log_ollama "WARN" "NVIDIA GPU found but CUDA libraries not detected"
    fi

    return 0
}

# Check what GPU Ollama is actually using
# This queries Ollama's reported capabilities
check_ollama_gpu_status() {
    if ! ollama_is_running; then
        return 1
    fi

    # Try to get Ollama's GPU info via the API
    local ollama_info
    ollama_info=$(curl -s http://localhost:11434/api/tags 2>/dev/null)

    # Check nvidia-smi for active GPU processes
    if command_exists nvidia-smi; then
        local ollama_on_gpu
        ollama_on_gpu=$(nvidia-smi --query-compute-apps=process_name --format=csv,noheader 2>/dev/null | grep -i ollama || true)
        if [[ -n "$ollama_on_gpu" ]]; then
            log_ollama "INFO" "Ollama is running on GPU (confirmed via nvidia-smi)"
            echo "gpu"
            return 0
        fi
    fi

    # If we have VRAM but Ollama isn't showing on GPU, it's likely CPU
    echo "unknown"
    return 0
}

# Diagnose GPU setup and return status
# Sets OLLAMA_GPU_STATUS: "gpu", "cpu", "unknown"
diagnose_gpu_setup() {
    log_ollama "INFO" "Diagnosing GPU setup..."

    local has_nvidia=false
    local has_amd=false
    local nvidia_works=false

    # Check NVIDIA
    if command_exists nvidia-smi; then
        has_nvidia=true
        log_ollama "DEBUG" "nvidia-smi found"

        local nvidia_output
        nvidia_output=$(nvidia-smi --query-gpu=name,memory.total,driver_version --format=csv,noheader 2>&1)
        if [[ $? -eq 0 ]]; then
            nvidia_works=true
            log_ollama "INFO" "NVIDIA GPU detected: $nvidia_output"

            # Check CUDA
            if check_nvidia_cuda_available; then
                log_ollama "INFO" "CUDA appears available"
            else
                log_ollama "WARN" "CUDA may not be properly configured"
            fi
        else
            log_ollama "WARN" "nvidia-smi failed: $nvidia_output"
        fi
    fi

    # Check AMD ROCm
    if command_exists rocm-smi; then
        has_amd=true
        log_ollama "DEBUG" "rocm-smi found"
        local rocm_output
        rocm_output=$(rocm-smi --showproductname 2>&1 || true)
        log_ollama "INFO" "AMD ROCm detected: $rocm_output"
    fi

    # Summary
    if [[ "$nvidia_works" == "true" ]]; then
        OLLAMA_GPU_STATUS="nvidia"
    elif [[ "$has_amd" == "true" ]]; then
        OLLAMA_GPU_STATUS="amd"
    else
        OLLAMA_GPU_STATUS="cpu"
        log_ollama "INFO" "No GPU acceleration detected, will use CPU"
    fi
}

# Verify Ollama is using GPU after it's running
# Call this after ollama_ensure_running to verify GPU usage
verify_ollama_gpu_usage() {
    # Only check if we detected a GPU
    if [[ "${OLLAMA_GPU_STATUS:-cpu}" == "cpu" ]]; then
        return 0
    fi

    # Give Ollama a moment to initialize
    sleep 1

    # Check if Ollama appears in GPU processes
    if command_exists nvidia-smi; then
        local gpu_processes
        gpu_processes=$(nvidia-smi --query-compute-apps=process_name,used_memory --format=csv,noheader 2>/dev/null || true)

        if echo "$gpu_processes" | grep -qi "ollama"; then
            log_ollama "INFO" "Verified: Ollama is using NVIDIA GPU"
            return 0
        else
            # GPU detected but Ollama not using it - this is the problem!
            log_ollama "WARN" "NVIDIA GPU detected but Ollama is NOT using it!"
            log_ollama "WARN" "GPU processes: ${gpu_processes:-none}"
            log_ollama "WARN" "Ollama may need to be reinstalled with GPU support"

            # Show warning to user
            ux_warn "GPU detected but Ollama running on CPU - performance will be slower"
            ux_info "To fix: reinstall Ollama with 'curl -fsSL https://ollama.com/install.sh | sh'"
            ux_info "Make sure NVIDIA drivers and CUDA are properly installed"
            return 1
        fi
    fi

    return 0
}

# Show GPU troubleshooting tips
show_gpu_troubleshooting() {
    ux_blank
    ux_separator
    ux_yollayah "$(yollayah_interjection) Hmm, looks like I'm running on CPU instead of your GPU..."
    ux_blank

    cat << 'TIPS'
    ╭──────────────────────────────────────────────────────────╮
    │  🔧  GPU Troubleshooting Tips                            │
    ├──────────────────────────────────────────────────────────┤
    │                                                          │
    │  For NVIDIA GPUs:                                        │
    │  1. Check drivers: nvidia-smi                            │
    │  2. Reinstall Ollama (it auto-detects CUDA):             │
    │     curl -fsSL https://ollama.com/install.sh | sh        │
    │  3. Restart Ollama after driver updates                  │
    │                                                          │
    │  For AMD GPUs:                                           │
    │  1. Install ROCm: https://rocm.docs.amd.com              │
    │  2. Reinstall Ollama after ROCm setup                    │
    │                                                          │
    │  Quick check:                                            │
    │  - Run: YOLLAYAH_DEBUG=1 ./yollayah.sh                   │
    │  - Check .logs/ollama.log for GPU detection info         │
    │                                                          │
    ╰──────────────────────────────────────────────────────────╯
TIPS

    ux_blank
    ux_separator
}

# Detect GPU name (for display purposes)
# Returns empty string if no GPU detected
detect_gpu_name() {
    local gpu_name=""

    # Try nvidia-smi first (NVIDIA GPUs)
    if command_exists nvidia-smi; then
        gpu_name=$(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null | head -1)
    fi

    # Try rocm-smi (AMD GPUs)
    if [[ -z "$gpu_name" ]] && command_exists rocm-smi; then
        gpu_name=$(rocm-smi --showproductname 2>/dev/null | grep "Card series" | head -1 | sed 's/.*: //')
    fi

    # Fallback: try lspci for any GPU
    if [[ -z "$gpu_name" ]] && command_exists lspci; then
        gpu_name=$(lspci 2>/dev/null | grep -i 'vga\|3d\|display' | head -1 | sed 's/.*: //' | cut -d'(' -f1 | xargs)
    fi

    echo "$gpu_name"
}

# Detect VRAM (GPU memory) in GB
# Returns 0 if no GPU detected
detect_vram_gb() {
    local vram_mb=0

    # Try nvidia-smi first (NVIDIA GPUs)
    if command_exists nvidia-smi; then
        vram_mb=$(nvidia-smi --query-gpu=memory.total --format=csv,noheader,nounits 2>/dev/null | head -1)
    fi

    # Try rocm-smi (AMD GPUs)
    if [[ $vram_mb -eq 0 ]] && command_exists rocm-smi; then
        # AMD reports in bytes, convert to MB
        local vram_bytes
        vram_bytes=$(rocm-smi --showmeminfo vram --csv 2>/dev/null | tail -1 | cut -d',' -f2)
        if [[ -n "$vram_bytes" ]]; then
            vram_mb=$((vram_bytes / 1024 / 1024))
        fi
    fi

    # Convert MB to GB
    echo $((vram_mb / 1024))
}

# Detect system RAM in GB
detect_ram_gb() {
    local ram_kb
    if [[ -f /proc/meminfo ]]; then
        ram_kb=$(grep MemTotal /proc/meminfo | awk '{print $2}')
        echo $((ram_kb / 1024 / 1024))
    elif command_exists sysctl; then
        # macOS
        local ram_bytes
        ram_bytes=$(sysctl -n hw.memsize 2>/dev/null)
        echo $((ram_bytes / 1024 / 1024 / 1024))
    else
        echo 8  # Assume 8GB if we can't detect
    fi
}

# ============================================================================
# Model Selection
# ============================================================================

# Select best model for available hardware
# Sets SELECTED_MODEL, DETECTED_GPU, and HARDWARE_TIER global variables
# HARDWARE_TIER: "powerful" (12GB+ VRAM), "decent" (6-11GB), "modest" (< 6GB or CPU)
model_select_best() {
    # Allow override via environment variable
    if [[ -n "${YOLLAYAH_MODEL:-}" ]]; then
        SELECTED_MODEL="$YOLLAYAH_MODEL"
        HARDWARE_TIER="custom"
        log_ollama "INFO" "Using model from YOLLAYAH_MODEL: $SELECTED_MODEL"
        return 0
    fi

    # Run GPU diagnostics first
    diagnose_gpu_setup

    local vram_gb ram_gb
    vram_gb=$(detect_vram_gb)
    ram_gb=$(detect_ram_gb)
    DETECTED_GPU=$(detect_gpu_name)
    DETECTED_VRAM_GB=$vram_gb

    log_ollama "INFO" "Hardware detected: ${vram_gb}GB VRAM, ${ram_gb}GB RAM, GPU: ${DETECTED_GPU:-none}"
    log_ollama "INFO" "GPU status: ${OLLAMA_GPU_STATUS:-unknown}"

    # If no GPU, fall back to CPU inference with RAM check
    if [[ $vram_gb -eq 0 ]]; then
        HARDWARE_TIER="modest"
        if [[ $ram_gb -ge 16 ]]; then
            SELECTED_MODEL="llama3.2:3b"
            log_ollama "INFO" "No GPU, but ${ram_gb}GB RAM - using 3b model"
        else
            SELECTED_MODEL="llama3.2:1b"
            log_ollama "INFO" "No GPU, limited RAM - using 1b model"
        fi
        return 0
    fi

    # Determine hardware tier based on VRAM
    if [[ $vram_gb -ge 12 ]]; then
        HARDWARE_TIER="powerful"
    elif [[ $vram_gb -ge 6 ]]; then
        HARDWARE_TIER="decent"
    else
        HARDWARE_TIER="modest"
    fi

    # Select based on VRAM
    for tier in "${MODEL_TIERS[@]}"; do
        local min_vram="${tier%%:*}"
        local model="${tier#*:}"
        if [[ $vram_gb -ge $min_vram ]]; then
            SELECTED_MODEL="$model"
            log_ollama "INFO" "Selected $model for ${vram_gb}GB VRAM (tier: $HARDWARE_TIER)"
            return 0
        fi
    done

    # Fallback
    SELECTED_MODEL="$DEFAULT_MODEL"
    HARDWARE_TIER="modest"
}

# ============================================================================
# Model Management
# ============================================================================

# Check if a model is available locally
model_is_available() {
    local model="$1"
    ollama list 2>/dev/null | grep -q "^${model}"
}

# Pull a model (with friendly progress - no scary hashes!)
model_pull() {
    local model="$1"

    if model_is_available "$model"; then
        ux_yollayah "$(yollayah_celebration) Brain's already here!"
        return 0
    fi

    # Show hardware-aware message before pulling
    case "${HARDWARE_TIER:-modest}" in
        powerful)
            ux_yollayah "$(yollayah_powerful_hardware "$model")"
            ;;
        decent)
            ux_yollayah "$(yollayah_thinking) Getting the brain ready: $model"
            ;;
        modest)
            ux_yollayah "$(yollayah_modest_hardware "$model")"
            ;;
    esac

    # Use the friendly wrapper that hides scary hashes (skip intro, we already announced)
    ux_ollama_pull "$model" true
}

# Ensure the selected model is ready
model_ensure_ready() {
    # Select best model if not already set
    if [[ -z "$SELECTED_MODEL" ]]; then
        model_select_best
    fi

    model_pull "$SELECTED_MODEL"
}

# ============================================================================
# Model Updates
# ============================================================================

# Check if a model has updates available
# TODO: Implement when Ollama provides update checking API
model_check_updates() {
    local model="$1"
    # Currently Ollama doesn't have a clean way to check for updates
    # without pulling. For now, we skip this.
    log_ollama "DEBUG" "Model update checking not yet implemented"
    return 1
}

# Update a model to latest version
model_update() {
    local model="$1"
    ux_yollayah "$(yollayah_thinking) Checking for brain updates..."
    ux_run_friendly "Updating ${model}..." ollama pull "$model"
}

# ============================================================================
# Hardware Info (for display)
# ============================================================================

# Get human-readable hardware summary
hardware_summary() {
    local vram_gb ram_gb
    vram_gb=$(detect_vram_gb)
    ram_gb=$(detect_ram_gb)

    if [[ $vram_gb -gt 0 ]]; then
        echo "${vram_gb}GB VRAM, ${ram_gb}GB RAM"
    else
        echo "${ram_gb}GB RAM (CPU inference)"
    fi
}
