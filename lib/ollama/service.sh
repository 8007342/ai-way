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

# Check if Ollama is installed
ollama_check_installed() {
    pj_check "ollama binary"
    pj_cmd "command -v ollama"
    if command_exists ollama; then
        pj_found "ollama at $(command -v ollama)"
        return 0
    else
        pj_missing "ollama"

        # Use ux_* for user-facing messages if available
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

    # Start Ollama serve in background
    # Set LD_LIBRARY_PATH to help Ollama find CUDA libraries on Fedora/Silverblue
    # See TODO-ollama-gpu.md for details
    pj_cmd "ollama serve (background)"
    LD_LIBRARY_PATH="/usr/lib:/usr/lib64:${LD_LIBRARY_PATH:-}" ollama serve > /dev/null 2>&1 &
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
