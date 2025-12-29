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
# Sets SELECTED_MODEL global variable
model_select_best() {
    # Allow override via environment variable
    if [[ -n "${YOLLAYAH_MODEL:-}" ]]; then
        SELECTED_MODEL="$YOLLAYAH_MODEL"
        debug "Using model from YOLLAYAH_MODEL: $SELECTED_MODEL"
        return 0
    fi

    local vram_gb ram_gb
    vram_gb=$(detect_vram_gb)
    ram_gb=$(detect_ram_gb)

    debug "Detected: ${vram_gb}GB VRAM, ${ram_gb}GB RAM"

    # If no GPU, fall back to CPU inference with RAM check
    if [[ $vram_gb -eq 0 ]]; then
        if [[ $ram_gb -ge 16 ]]; then
            SELECTED_MODEL="llama3.2:3b"
            debug "No GPU, but ${ram_gb}GB RAM - using 3b model"
        else
            SELECTED_MODEL="llama3.2:1b"
            debug "No GPU, limited RAM - using 1b model"
        fi
        return 0
    fi

    # Select based on VRAM
    for tier in "${MODEL_TIERS[@]}"; do
        local min_vram="${tier%%:*}"
        local model="${tier#*:}"
        if [[ $vram_gb -ge $min_vram ]]; then
            SELECTED_MODEL="$model"
            debug "Selected $model for ${vram_gb}GB VRAM"
            return 0
        fi
    done

    # Fallback
    SELECTED_MODEL="$DEFAULT_MODEL"
}

# ============================================================================
# Model Management
# ============================================================================

# Check if a model is available locally
model_is_available() {
    local model="$1"
    ollama list 2>/dev/null | grep -q "^${model}"
}

# Pull a model (with progress)
model_pull() {
    local model="$1"

    if model_is_available "$model"; then
        success "Model $model available"
        return 0
    fi

    info "Pulling model $model..."
    warn "This may take a few minutes on first run..."

    if ollama pull "$model"; then
        success "Model $model ready"
        return 0
    else
        error "Failed to pull model $model"
        return 1
    fi
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
    debug "Model update checking not yet implemented"
    return 1
}

# Update a model to latest version
model_update() {
    local model="$1"
    info "Updating model $model..."
    ollama pull "$model"
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
