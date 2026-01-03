#!/bin/bash
# ============================================================================
# lib/ollama/service.sh - Ollama Service State Management
#
# This module handles:
# - Recording pre-Yollayah Ollama state
# - Starting Ollama if needed
# - Cleanup: restoring system to pre-Yollayah state
#
# Core Principle: LEAVE NO TRACE
# If Ollama wasn't running before Yollayah, stop it when we exit.
# If Ollama wasn't a systemd service, don't enable it.
# The system should look exactly as it did before Yollayah ran.
#
# Constitution Reference:
# - Law of Care: "First, do no harm" - don't mess with user's system
# - Four Protections: "Protect AJ from ai-way" - we clean up after ourselves
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_OLLAMA_SERVICE_LOADED:-}" ]] && return 0
_YOLLAYAH_OLLAMA_SERVICE_LOADED=1

# ============================================================================
# State Recording
# ============================================================================

# Record Ollama state BEFORE we touch anything
# This lets us restore to pre-Yollayah state on exit
ollama_record_state() {
    log_ollama "INFO" "Recording pre-Yollayah Ollama state"
    pj_step "Recording Ollama state (for cleanup later)"

    # Check if Ollama is currently responding
    pj_cmd "curl -s http://localhost:11434/api/tags"
    if ollama_is_running; then
        OLLAMA_WAS_RUNNING=true
        pj_result "Ollama already running"
        log_ollama "DEBUG" "Ollama was already running"
    else
        pj_result "Ollama not running yet"
    fi

    # Check systemd service state (Linux only)
    if command_exists systemctl; then
        pj_check "systemd service status"
        if systemctl list-unit-files ollama.service &> /dev/null 2>&1; then
            if systemctl is-enabled ollama.service &> /dev/null 2>&1; then
                OLLAMA_SERVICE_WAS_ENABLED=true
                pj_result "Service enabled (auto-starts on boot)"
                log_ollama "DEBUG" "Ollama service was enabled"
            fi
            if systemctl is-active ollama.service &> /dev/null 2>&1; then
                OLLAMA_SERVICE_WAS_ACTIVE=true
                pj_result "Service currently active"
                log_ollama "DEBUG" "Ollama service was active"
            fi
        else
            pj_result "No systemd service found"
        fi
    fi

    # Persist state to file (for crash recovery)
    ensure_dir "$STATE_DIR"
    cat > "$STATE_FILE" << EOF
OLLAMA_WAS_RUNNING=$OLLAMA_WAS_RUNNING
OLLAMA_SERVICE_WAS_ENABLED=$OLLAMA_SERVICE_WAS_ENABLED
OLLAMA_SERVICE_WAS_ACTIVE=$OLLAMA_SERVICE_WAS_ACTIVE
RECORDED_AT=$(date -Iseconds)
EOF
    log_ollama "INFO" "State recorded: was_running=$OLLAMA_WAS_RUNNING"
}

# ============================================================================
# Service Control
# ============================================================================

# Smart filter for ollama serve output (test verbose mode)
# Extracts only GPU/CUDA diagnostic lines, showing them in clean format
_ollama_filter_output() {
    # Read from stdin, filter and colorize key diagnostic lines
    while IFS= read -r line; do
        # Extract GPU discovery messages
        if [[ "$line" =~ "discovering available GPUs" ]]; then
            echo -e "${UX_CYAN}→ Ollama:${UX_NC} ${UX_DIM}Discovering GPUs...${UX_NC}"

        # Extract GPU compute info (the money shot!)
        elif [[ "$line" =~ "inference compute" ]]; then
            # Parse the structured log line for key details
            local gpu_name=$(echo "$line" | grep -oP 'name=\K[^ ]+')
            local gpu_desc=$(echo "$line" | grep -oP 'description="\K[^"]+')
            local gpu_total=$(echo "$line" | grep -oP 'total="\K[^"]+')
            local gpu_avail=$(echo "$line" | grep -oP 'available="\K[^"]+')
            local gpu_lib=$(echo "$line" | grep -oP 'library=\K[^ ]+')

            echo -e "${UX_GREEN}✓ Ollama:${UX_NC} GPU detected: ${UX_BOLD}$gpu_desc${UX_NC}"
            echo -e "${UX_DIM}  └─ Library: $gpu_lib, Total: $gpu_total, Available: $gpu_avail${UX_NC}"

        # Extract CUDA visibility warnings
        elif [[ "$line" =~ "user overrode visible devices" ]]; then
            local cuda_devices=$(echo "$line" | grep -oP 'CUDA_VISIBLE_DEVICES=\K[^ ]+')
            echo -e "${UX_YELLOW}⚠ Ollama:${UX_NC} ${UX_DIM}Using CUDA device: $cuda_devices${UX_NC}"

        # Extract listening message (confirms startup)
        elif [[ "$line" =~ "Listening on" ]]; then
            local addr=$(echo "$line" | grep -oP 'Listening on \K[^ ]+')
            local version=$(echo "$line" | grep -oP 'version \K[^ ]+' | tr -d ')')
            echo -e "${UX_GREEN}✓ Ollama:${UX_NC} Listening on $addr ${UX_DIM}(v$version)${UX_NC}"

        # Extract Vulkan messages
        elif [[ "$line" =~ "Vulkan" ]]; then
            if [[ "$line" =~ "disabled" ]]; then
                echo -e "${UX_DIM}  └─ Vulkan: disabled (using CUDA)${UX_NC}"
            fi

        # Extract runner start messages (interesting in test mode)
        elif [[ "$line" =~ "starting runner" ]]; then
            local port=$(echo "$line" | grep -oP 'port \K[0-9]+')
            echo -e "${UX_DIM}→ Ollama:${UX_NC} ${UX_DIM}Starting runner on port $port${UX_NC}"
        fi

        # All other lines are suppressed (that's the whole point!)
    done
}

# Check if Ollama is installed
ollama_check_installed() {
    # Phase 2.1: Toolbox-aware detection
    local in_toolbox=false
    if [[ -f /run/.toolboxenv ]]; then
        in_toolbox=true
        pj_check "ollama binary (inside toolbox container)"
        log_ollama "DEBUG" "Running inside toolbox - checking for container ollama"
    else
        pj_check "ollama binary"
        log_ollama "DEBUG" "Running on host - checking for ollama"
    fi

    pj_cmd "command -v ollama"
    if command_exists ollama; then
        pj_found "ollama at $(command -v ollama)"
        log_ollama "INFO" "Ollama found at: $(command -v ollama)"
        return 0
    else
        pj_missing "ollama"
        log_ollama "WARN" "Ollama not found in PATH"

        # Phase 2.2: Auto-install in toolbox
        if [[ "$in_toolbox" == "true" ]]; then
            log_ollama "INFO" "Inside toolbox - attempting auto-install"

            # Use ux_* for user-facing messages
            if declare -f ux_blank &>/dev/null; then
                ux_blank
                ux_info "Installing ollama in toolbox container (one-time setup)..."
                ux_print "This will take about 1-2 minutes."
                ux_blank
            else
                echo ""
                echo "Installing ollama in toolbox container (one-time setup)..."
                echo "This will take about 1-2 minutes."
                echo ""
            fi

            pj_step "Installing ollama via official script"
            pj_cmd "curl -fsSL https://ollama.com/install.sh | sh"

            # No sudo needed in toolbox!
            if curl -fsSL https://ollama.com/install.sh | sh; then
                log_ollama "INFO" "Ollama installed successfully in toolbox"
                pj_result "Installation complete"

                # Verify installation
                if command_exists ollama; then
                    if declare -f ux_success &>/dev/null; then
                        ux_success "Ollama installed successfully in container"
                        ux_blank
                    else
                        echo ""
                        echo "Ollama installed successfully in container"
                        echo ""
                    fi
                    return 0
                else
                    log_ollama "ERROR" "Ollama installed but not found in PATH"
                    if declare -f ux_error &>/dev/null; then
                        ux_error "Installation completed but ollama not found in PATH"
                    fi
                    return 1
                fi
            else
                log_ollama "ERROR" "Failed to install ollama in toolbox"
                if declare -f ux_error &>/dev/null; then
                    ux_blank
                    ux_error "Failed to install ollama in toolbox"
                    ux_error "Try manually: curl -fsSL https://ollama.com/install.sh | sh"
                    ux_blank
                else
                    echo ""
                    echo "Failed to install ollama in toolbox"
                    echo "Try manually: curl -fsSL https://ollama.com/install.sh | sh"
                    echo ""
                fi
                return 1
            fi
        fi

        # Not in toolbox - show install instructions and prompt
        if declare -f ux_blank &>/dev/null; then
            ux_blank
            ux_warn "Ollama is required to run Yollayah locally."
            ux_blank
            ux_print "Install options:"
            ux_item "Linux/WSL:  curl -fsSL https://ollama.com/install.sh | sh"
            ux_item "macOS:      brew install ollama"
            ux_item "Manual:     https://ollama.com/download"
            ux_blank
        else
            echo ""
            echo "Ollama is required to run Yollayah locally."
            echo ""
            echo "Install options:"
            echo "  Linux/WSL:  curl -fsSL https://ollama.com/install.sh | sh"
            echo "  macOS:      brew install ollama"
            echo "  Manual:     https://ollama.com/download"
            echo ""
        fi

        read -p "Would you like me to try installing Ollama? [y/N] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            info "Installing Ollama..."
            if curl -fsSL https://ollama.com/install.sh | sh; then
                success "Ollama installed"
                return 0
            else
                error "Installation failed"
                return 1
            fi
        else
            error "Please install Ollama and try again"
            return 1
        fi
    fi
}

# Ensure Ollama is running (start if needed)
ollama_ensure_running() {
    pj_step "Ensuring Ollama is running"
    pj_cmd "curl -s http://localhost:11434/api/tags"

    if ollama_is_running; then
        log_ollama "INFO" "Ollama already running"
        pj_result "Ollama API responding on port 11434"
        ux_success "Ollama is running"
        return 0
    fi

    log_ollama "WARN" "Ollama not running, starting..."
    pj_result "Ollama not responding, will start it"

    # Phase 2.3: GPU device detection in toolbox
    if [[ -f /run/.toolboxenv ]]; then
        pj_step "GPU detection (inside toolbox container)"
        log_ollama "DEBUG" "Checking for GPU devices in toolbox"

        local gpu_found=false
        local gpu_devices=""

        # Check for NVIDIA GPUs
        if ls /dev/nvidia* &> /dev/null; then
            gpu_devices=$(ls /dev/nvidia* 2>/dev/null | tr '\n' ' ')
            pj_found "NVIDIA GPU devices: $gpu_devices"
            log_ollama "INFO" "NVIDIA GPU devices found: $gpu_devices"
            gpu_found=true
        fi

        # Check for AMD GPUs
        if ls /dev/dri/renderD* &> /dev/null; then
            local amd_devices=$(ls /dev/dri/renderD* 2>/dev/null | tr '\n' ' ')
            pj_found "AMD GPU devices: $amd_devices"
            log_ollama "INFO" "AMD GPU devices found: $amd_devices"
            gpu_found=true
        fi

        if [[ "$gpu_found" == "false" ]]; then
            pj_missing "No GPU devices found in container"
            log_ollama "WARN" "No GPU devices detected - will use CPU inference (slow)"
            if declare -f ux_warn &>/dev/null; then
                ux_warn "GPU not detected - will use CPU inference (slow)"
            fi
        fi

        # In verbose mode, show additional GPU info
        if [[ -n "${YOLLAYAH_DEBUG:-}" ]]; then
            pj_check "GPU driver information"
            if command_exists nvidia-smi; then
                pj_cmd "nvidia-smi --query-gpu=name,driver_version --format=csv,noheader"
                local gpu_info=$(nvidia-smi --query-gpu=name,driver_version --format=csv,noheader 2>/dev/null || echo "N/A")
                pj_found "$gpu_info"
                log_ollama "DEBUG" "GPU info: $gpu_info"
            else
                pj_missing "nvidia-smi not available"
            fi
        fi
    fi

    # Start Ollama serve in background
    # Set LD_LIBRARY_PATH to help Ollama find CUDA libraries on Fedora/Silverblue
    # NOTE: In toolbox, this may not be needed but kept for compatibility
    # See TODO-ollama-gpu.md for details
    pj_cmd "ollama serve (background)"

    # Set OLLAMA_KEEP_ALIVE to prevent model unloading (CRITICAL for performance!)
    # Default: keep models loaded for 24 hours - prevents slow model reloading
    # Override with: YOLLAYAH_OLLAMA_KEEP_ALIVE=<duration>
    # See: TODO-ollama-keep-alive.md for rationale
    : "${YOLLAYAH_OLLAMA_KEEP_ALIVE:=24h}"
    export OLLAMA_KEEP_ALIVE="$YOLLAYAH_OLLAMA_KEEP_ALIVE"
    pj_result "OLLAMA_KEEP_ALIVE=${OLLAMA_KEEP_ALIVE} (models stay in memory)"
    log_ollama "INFO" "OLLAMA_KEEP_ALIVE set to: $OLLAMA_KEEP_ALIVE"

    # In test verbose mode, show filtered Ollama output (GPU/CUDA diagnostics only)
    if [[ -n "${YOLLAYAH_TEST_VERBOSE:-}" ]]; then
        echo -e "${UX_CYAN}→${UX_NC} Starting ollama serve with smart diagnostic filtering..."
        echo -e "${UX_DIM}  (showing GPU/CUDA messages only)${UX_NC}"
        echo -e "${UX_DIM}  OLLAMA_KEEP_ALIVE=${OLLAMA_KEEP_ALIVE}${UX_NC}"
        ux_blank

        # Start ollama serve with filtered output
        # stderr goes to our filter, process runs in background
        LD_LIBRARY_PATH="/usr/lib:/usr/lib64:${LD_LIBRARY_PATH:-}" ollama serve 2>&1 | _ollama_filter_output &
    else
        LD_LIBRARY_PATH="/usr/lib:/usr/lib64:${LD_LIBRARY_PATH:-}" ollama serve > /dev/null 2>&1 &
    fi

    local pid=$!
    WE_STARTED_OLLAMA=true
    log_ollama "INFO" "Started ollama serve (PID: $pid) with CUDA library path"
    pj_result "Started ollama serve (PID: $pid)"

    # Wait for it to come up
    pj_step "Waiting for Ollama API..."
    local attempts=0
    while ! ollama_is_running; do
        sleep 1
        ((attempts++))
        pj_result "Attempt $attempts/10..."
        if [[ $attempts -ge 10 ]]; then
            log_ollama "ERROR" "Failed to start Ollama after 10 seconds"
            ux_error "Failed to start Ollama after 10 seconds"
            ux_error "Try running 'ollama serve' manually"
            return 1
        fi
    done

    log_ollama "INFO" "Ollama started successfully"
    pj_result "Ollama API now responding"
    ux_success "Ollama started (will stop on exit)"
}

# ============================================================================
# Cleanup - Restore Pre-Yollayah State
# ============================================================================

# Cleanup function - call this on exit
# Restores system to pre-Yollayah state
ollama_cleanup() {
    # Only clean up if we started Ollama
    if [[ "$WE_STARTED_OLLAMA" != "true" ]]; then
        log_ollama "DEBUG" "We didn't start Ollama, nothing to clean up"
        pj_step "Cleanup: we didn't start Ollama, nothing to do"
        return 0
    fi

    log_ollama "INFO" "Cleaning up Ollama..."
    pj_step "Cleaning up Ollama..."

    # If Ollama wasn't running before, stop it
    if [[ "$OLLAMA_WAS_RUNNING" == "false" ]]; then
        log_ollama "INFO" "Stopping Ollama (wasn't running before)"
        pj_result "Stopping Ollama (wasn't running before we started)"

        # Kill the ollama serve process we started
        pj_cmd "pkill -f 'ollama serve'"
        pkill -f "ollama serve" 2>/dev/null || true

        # Handle systemd service if applicable
        if command_exists systemctl; then
            # If service is now active but wasn't before, stop it
            if systemctl is-active ollama.service &> /dev/null 2>&1; then
                if [[ "$OLLAMA_SERVICE_WAS_ACTIVE" == "false" ]]; then
                    log_ollama "INFO" "Stopping Ollama systemd service"
                    pj_cmd "systemctl stop ollama.service"
                    sudo systemctl stop ollama.service 2>/dev/null || true
                fi
            fi

            # If service is now enabled but wasn't before, disable it
            if systemctl is-enabled ollama.service &> /dev/null 2>&1; then
                if [[ "$OLLAMA_SERVICE_WAS_ENABLED" == "false" ]]; then
                    log_ollama "INFO" "Disabling Ollama service auto-start"
                    pj_cmd "systemctl disable ollama.service"
                    sudo systemctl disable ollama.service 2>/dev/null || true
                fi
            fi
        fi

        log_ollama "INFO" "Ollama stopped, restored pre-Yollayah state"
        pj_result "Restored system to pre-Yollayah state"
    else
        log_ollama "INFO" "Leaving Ollama running (was already running)"
        pj_result "Leaving Ollama running (was already running)"
    fi

    # Clean up state file
    rm -f "$STATE_FILE" 2>/dev/null || true
}

# ============================================================================
# Trap Setup
# ============================================================================

# Register cleanup handler
# Called by bootstrap after sourcing this module
ollama_register_cleanup() {
    trap '_yollayah_exit_handler' EXIT INT TERM
}

# Internal exit handler
_yollayah_exit_handler() {
    local exit_code=$?
    echo ""
    ollama_cleanup

    # Clean up logs (deletes if not persisting)
    if declare -f log_cleanup_on_exit &>/dev/null; then
        log_cleanup_on_exit "$exit_code"
    fi

    exit $exit_code
}
