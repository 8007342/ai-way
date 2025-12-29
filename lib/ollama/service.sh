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
    debug "Recording pre-Yollayah Ollama state..."

    # Check if Ollama is currently responding
    if ollama_is_running; then
        OLLAMA_WAS_RUNNING=true
        debug "Ollama was already running"
    fi

    # Check systemd service state (Linux only)
    if command_exists systemctl; then
        if systemctl list-unit-files ollama.service &> /dev/null 2>&1; then
            if systemctl is-enabled ollama.service &> /dev/null 2>&1; then
                OLLAMA_SERVICE_WAS_ENABLED=true
                debug "Ollama service was enabled"
            fi
            if systemctl is-active ollama.service &> /dev/null 2>&1; then
                OLLAMA_SERVICE_WAS_ACTIVE=true
                debug "Ollama service was active"
            fi
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
}

# ============================================================================
# Service Control
# ============================================================================

# Check if Ollama is installed
ollama_check_installed() {
    if command_exists ollama; then
        success "Ollama found"
        return 0
    else
        error "Ollama not found"

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
    if ollama_is_running; then
        success "Ollama is running"
        return 0
    fi

    warn "Ollama is not running"
    info "Starting Ollama..."

    # Start Ollama serve in background
    ollama serve > /dev/null 2>&1 &
    WE_STARTED_OLLAMA=true

    # Wait for it to come up
    local attempts=0
    while ! ollama_is_running; do
        sleep 1
        ((attempts++))
        if [[ $attempts -ge 10 ]]; then
            error "Failed to start Ollama after 10 seconds"
            error "Please run 'ollama serve' manually"
            return 1
        fi
    done

    success "Ollama started (will stop on exit)"
}

# ============================================================================
# Cleanup - Restore Pre-Yollayah State
# ============================================================================

# Cleanup function - call this on exit
# Restores system to pre-Yollayah state
ollama_cleanup() {
    # Only clean up if we started Ollama
    if [[ "$WE_STARTED_OLLAMA" != "true" ]]; then
        debug "We didn't start Ollama, nothing to clean up"
        return 0
    fi

    info "Cleaning up..."

    # If Ollama wasn't running before, stop it
    if [[ "$OLLAMA_WAS_RUNNING" == "false" ]]; then
        debug "Stopping Ollama (wasn't running before)"

        # Kill the ollama serve process we started
        pkill -f "ollama serve" 2>/dev/null || true

        # Handle systemd service if applicable
        if command_exists systemctl; then
            # If service is now active but wasn't before, stop it
            if systemctl is-active ollama.service &> /dev/null 2>&1; then
                if [[ "$OLLAMA_SERVICE_WAS_ACTIVE" == "false" ]]; then
                    info "Stopping Ollama service..."
                    sudo systemctl stop ollama.service 2>/dev/null || true
                fi
            fi

            # If service is now enabled but wasn't before, disable it
            if systemctl is-enabled ollama.service &> /dev/null 2>&1; then
                if [[ "$OLLAMA_SERVICE_WAS_ENABLED" == "false" ]]; then
                    info "Disabling Ollama service auto-start..."
                    sudo systemctl disable ollama.service 2>/dev/null || true
                fi
            fi
        fi

        success "Ollama stopped (restored pre-Yollayah state)"
    else
        info "Leaving Ollama running (was already running)"
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
    exit $exit_code
}
