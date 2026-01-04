#!/bin/bash
# ============================================================================
# lib/ux/terminal.sh - Terminal User Interface
#
# This module handles:
# - Banner and branding
# - User prompts and input
# - Command handling (/quit, /clear, /help, etc.)
# - Conversation loop
# - Visual feedback
#
# UX Philosophy:
# - Clean, uncluttered interface
# - Yollayah's personality shines through
# - Commands are discoverable but not intrusive
# - Color enhances but doesn't overwhelm
#
# Output Architecture:
# - All display output uses ux_* functions from lib/ux/output.sh
# - Uses UX_* color constants for consistency
# - Never uses raw echo for formatted output
#
# Constitution Reference:
# - Law of Truth: Clear, honest interface
# - Law of Care: Pleasant, non-stressful experience
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_UX_TERMINAL_LOADED:-}" ]] && return 0
_YOLLAYAH_UX_TERMINAL_LOADED=1

# ============================================================================
# Banner and Branding
# ============================================================================

ux_print_banner() {
    echo -e "${UX_MAGENTA}"
    cat << 'BANNER'
  ╭─────────────────────────────────────────╮
  │                                         │
  │   🦎 Yollayah                           │
  │   "Heart that goes with you"            │
  │                                         │
  │   ai-way-lite                           │
  │   Local AI. Your data. Your rules.      │
  │                                         │
  ╰─────────────────────────────────────────╯
BANNER
    echo -e "${UX_NC}"
}

ux_print_separator() {
    ux_separator
}

# ============================================================================
# Ready Message
# ============================================================================

ux_print_ready() {
    ux_blank
    ux_print_separator
    ux_blank
    echo -e "${UX_WHITE}Yollayah is ready! Type your message and press Enter.${UX_NC}"
    echo -e "${UX_CYAN}Commands: /quit to exit, /clear to clear screen, /help for more${UX_NC}"
    ux_blank
    ux_print_separator
    ux_blank
}

# ============================================================================
# Help Display
# ============================================================================

ux_show_help() {
    ux_blank
    echo -e "${UX_CYAN}Commands:${UX_NC}"
    ux_item "/quit, /exit, /q  - Exit Yollayah"
    ux_item "/clear            - Clear the screen"
    ux_item "/mood             - Check Yollayah's mood"
    ux_item "/model            - Show current model"
    ux_item "/help             - Show this help"
    ux_blank
    echo -e "${UX_CYAN}Tips:${UX_NC}"
    ux_item "Just type naturally, Yollayah understands context"
    ux_item "Your conversations stay local and private"
    ux_item "Check out agents/ai-way-docs/ for more info"
    ux_blank
}

# ============================================================================
# Command Handling
# ============================================================================

# Handle a slash command
# Returns 0 if command was handled, 1 if not a command
ux_handle_command() {
    local input="$1"

    case "$input" in
        /quit|/exit|/q)
            ux_blank
            ux_yollayah "¡Hasta luego! Take care of yourself. 💜"
            ux_blank
            exit 0
            ;;
        /clear)
            clear
            ux_print_banner
            return 0
            ;;
        /mood)
            ux_yollayah "I'm feeling good! Ready to help. How about you?"
            ux_blank
            return 0
            ;;
        /model)
            ux_keyval "Current model" "$SELECTED_MODEL"
            ux_keyval "Hardware" "$(hardware_summary)"
            ux_blank
            return 0
            ;;
        /help)
            ux_show_help
            return 0
            ;;
        /debug)
            # Hidden command to toggle debug mode
            if [[ -n "${YOLLAYAH_DEBUG:-}" ]]; then
                unset YOLLAYAH_DEBUG
                ux_info "Debug mode disabled"
            else
                export YOLLAYAH_DEBUG=1
                ux_info "Debug mode enabled"
            fi
            ux_blank
            return 0
            ;;
        /*)
            # Unknown command
            ux_warn "Unknown command: $input"
            ux_print "Type /help for available commands"
            ux_blank
            return 0
            ;;
        *)
            # Not a command
            return 1
            ;;
    esac
}

# ============================================================================
# Conversation Loop
# ============================================================================

# Main conversation loop
ux_conversation_loop() {
    local model_name="$1"

    ux_print_ready

    while true; do
        # Prompt
        ux_prompt "You:"
        read -r user_input

        # Handle empty input
        if [[ -z "$user_input" ]]; then
            continue
        fi

        # Handle commands
        if ux_handle_command "$user_input"; then
            continue
        fi

        # Get response from Yollayah
        ux_blank
        echo -ne "${UX_MAGENTA}Yollayah:${UX_NC} "

        # Stream the response
        if ! ollama run "$model_name" "$user_input"; then
            echo ""
            ux_error "Failed to get response from model: $model_name"
            ux_info "Check if Ollama is running: ollama list"
            ux_info "Try: ollama run $model_name"
        fi

        ux_blank
        ux_blank
    done
}

# ============================================================================
# Progress and Feedback
# ============================================================================

ux_show_startup_progress() {
    pj_step "Yollayah startup sequence"
    ux_info "Checking dependencies..."
    ux_blank
}

ux_show_all_ready() {
    ux_blank
    ux_success "All systems ready!"

    # Show cute GPU message if one was detected
    if [[ -n "${DETECTED_GPU:-}" ]] && [[ "${DETECTED_VRAM_GB:-0}" -gt 0 ]]; then
        ux_yollayah "$(yollayah_gpu_flex "$DETECTED_GPU")"
    fi
}

# ============================================================================
# Conductor Daemon Support
# ============================================================================

# Path to the Conductor daemon binary
CONDUCTOR_DIR="${SCRIPT_DIR}/conductor"
CONDUCTOR_DAEMON="${CONDUCTOR_DIR}/target/release/conductor-daemon"
CONDUCTOR_DAEMON_DEBUG="${CONDUCTOR_DIR}/target/debug/conductor-daemon"

# Check if daemon mode is enabled via CONDUCTOR_TRANSPORT
# Returns 0 (true) if daemon mode is needed, 1 (false) otherwise
conductor_needs_daemon() {
    local transport="${CONDUCTOR_TRANSPORT:-inprocess}"
    transport=$(echo "$transport" | tr '[:upper:]' '[:lower:]')

    case "$transport" in
        unix|socket)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

# Get the path to the conductor daemon binary (prefer release, fall back to debug)
conductor_daemon_binary() {
    if [[ -x "$CONDUCTOR_DAEMON" ]]; then
        echo "$CONDUCTOR_DAEMON"
    elif [[ -x "$CONDUCTOR_DAEMON_DEBUG" ]]; then
        echo "$CONDUCTOR_DAEMON_DEBUG"
    fi
}

# Get the conductor socket path
conductor_socket_path() {
    if [[ -n "${CONDUCTOR_SOCKET:-}" ]]; then
        echo "$CONDUCTOR_SOCKET"
    elif [[ -n "${XDG_RUNTIME_DIR:-}" ]]; then
        echo "${XDG_RUNTIME_DIR}/ai-way/conductor.sock"
    else
        echo "/tmp/ai-way-$(id -u)/conductor.sock"
    fi
}

# Get the conductor PID file path
conductor_pid_path() {
    if [[ -n "${XDG_RUNTIME_DIR:-}" ]]; then
        echo "${XDG_RUNTIME_DIR}/ai-way/conductor.pid"
    else
        echo "/tmp/ai-way-$(id -u)/conductor.pid"
    fi
}

# Check if conductor daemon is running
conductor_is_running() {
    local pid_file
    pid_file="$(conductor_pid_path)"

    if [[ -f "$pid_file" ]]; then
        local pid
        pid=$(cat "$pid_file" 2>/dev/null)
        if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
            return 0
        fi
    fi

    # Also check if socket exists and is responsive
    local socket
    socket="$(conductor_socket_path)"
    [[ -S "$socket" ]]
}

# Ensure the conductor daemon is running
# Starts it if not already running, waits for socket to be ready
conductor_ensure_running() {
    local daemon_bin socket_path socket_dir max_wait=10

    daemon_bin="$(conductor_daemon_binary)"
    socket_path="$(conductor_socket_path)"
    socket_dir="$(dirname "$socket_path")"

    # Check if already running
    if conductor_is_running; then
        log_ux "DEBUG" "Conductor daemon already running"
        return 0
    fi

    # Check if daemon binary exists
    if [[ -z "$daemon_bin" ]]; then
        log_ux "ERROR" "Conductor daemon binary not found"
        ux_error "Conductor daemon not built. Run: cargo build --release -p conductor-daemon"
        return 1
    fi

    # Ensure socket directory exists
    if [[ ! -d "$socket_dir" ]]; then
        mkdir -p "$socket_dir" || {
            log_ux "ERROR" "Failed to create socket directory: $socket_dir"
            return 1
        }
    fi

    log_ux "INFO" "Starting Conductor daemon..."
    ux_info "Starting Conductor daemon..."

    # Start daemon in background
    "$daemon_bin" &
    local daemon_pid=$!

    # Wait for socket to appear
    local waited=0
    while [[ ! -S "$socket_path" ]] && [[ $waited -lt $max_wait ]]; do
        sleep 0.5
        waited=$((waited + 1))

        # Check if daemon died
        if ! kill -0 "$daemon_pid" 2>/dev/null; then
            log_ux "ERROR" "Conductor daemon exited unexpectedly"
            ux_error "Conductor daemon failed to start"
            return 1
        fi
    done

    if [[ ! -S "$socket_path" ]]; then
        log_ux "ERROR" "Conductor socket not created after ${max_wait}s"
        ux_error "Conductor daemon failed to create socket"
        kill "$daemon_pid" 2>/dev/null || true
        return 1
    fi

    log_ux "INFO" "Conductor daemon started (PID: $daemon_pid)"
    ux_success "Conductor daemon ready"
    return 0
}

# Stop the conductor daemon
conductor_stop() {
    local pid_file socket_path

    pid_file="$(conductor_pid_path)"
    socket_path="$(conductor_socket_path)"

    if [[ -f "$pid_file" ]]; then
        local pid
        pid=$(cat "$pid_file" 2>/dev/null)
        if [[ -n "$pid" ]]; then
            log_ux "INFO" "Stopping Conductor daemon (PID: $pid)"
            kill "$pid" 2>/dev/null || true
            rm -f "$pid_file"
        fi
    fi

    # Clean up socket if it exists
    rm -f "$socket_path" 2>/dev/null || true
}

# ============================================================================
# Rich Terminal UI (Rust TUI)
# ============================================================================

# Path to the Rust TUI binary
TUI_DIR="${SCRIPT_DIR}/tui"
TUI_BINARY="${TUI_DIR}/target/release/yollayah-tui"
TUI_BINARY_DEBUG="${TUI_DIR}/target/debug/yollayah-tui"

# Check if Rust TUI is available
ux_tui_available() {
    [[ -x "$TUI_BINARY" ]] || [[ -x "$TUI_BINARY_DEBUG" ]]
}

# Get the TUI binary path (prefer release, fall back to debug)
ux_tui_binary() {
    if [[ -x "$TUI_BINARY" ]]; then
        echo "$TUI_BINARY"
    elif [[ -x "$TUI_BINARY_DEBUG" ]]; then
        echo "$TUI_BINARY_DEBUG"
    fi
}

# Check if TUI needs to be rebuilt (sources newer than binary)
ux_tui_needs_rebuild() {
    pj_check "TUI binary status"
    local binary
    binary="$(ux_tui_binary)"

    # No binary at all? Definitely need to build
    if [[ -z "$binary" ]] || [[ ! -f "$binary" ]]; then
        log_ux "DEBUG" "TUI binary not found, needs build"
        pj_missing "TUI binary (will compile)"
        return 0
    fi
    pj_found "TUI binary at $binary"

    # Check if any Rust source file is newer than the binary
    pj_check "Source files vs binary"
    local newest_source
    newest_source=$(find "$TUI_DIR/src" -name "*.rs" -newer "$binary" 2>/dev/null | head -1)

    if [[ -n "$newest_source" ]]; then
        log_ux "INFO" "TUI source newer than binary: $newest_source"
        pj_result "Source changed: $newest_source (will recompile)"
        return 0
    fi

    # Check if Cargo.toml is newer (dependencies changed)
    if [[ "$TUI_DIR/Cargo.toml" -nt "$binary" ]]; then
        log_ux "INFO" "Cargo.toml newer than binary, rebuild needed"
        pj_result "Cargo.toml changed (will recompile)"
        return 0
    fi

    log_ux "DEBUG" "TUI binary is up to date"
    pj_result "Binary up to date, no rebuild needed"
    return 1
}

# Build the TUI (release mode)
ux_tui_build() {
    pj_step "Building TUI interface"

    # Check if Rust is available
    pj_check "Cargo availability"
    pj_cmd "command -v cargo"
    if ! command -v cargo &>/dev/null; then
        # Try sourcing cargo env
        pj_result "Not in PATH, checking ~/.cargo/env"
        if [[ -f "$HOME/.cargo/env" ]]; then
            pj_cmd "source $HOME/.cargo/env"
            source "$HOME/.cargo/env" 2>/dev/null || true
        fi
    fi

    if ! command -v cargo &>/dev/null; then
        log_ux "WARN" "Cargo not available, cannot build TUI"
        pj_missing "cargo (can't compile TUI)"
        return 1
    fi
    pj_found "cargo at $(command -v cargo)"

    log_ux "INFO" "Building TUI (this may take a moment on first build)..."
    ux_yollayah "$(yollayah_thinking) Getting my pretty face ready..."

    # Build in release mode for better performance
    pj_cmd "cargo build --release --manifest-path $TUI_DIR/Cargo.toml"
    pj_result "This compiles the Rust TUI (may take 1-2 min first time)"
    if ux_run_friendly "Building interface..." cargo build --release --manifest-path "$TUI_DIR/Cargo.toml" 2>&1; then
        log_ux "INFO" "TUI build successful"
        pj_result "Build successful"
        ux_success "Interface ready!"
        return 0
    else
        log_ux "ERROR" "TUI build failed"
        pj_result "Build failed (check cargo output)"
        ux_warn "Couldn't build the fancy interface, using simple mode"
        return 1
    fi
}

# Ensure TUI is built and up to date
ux_tui_ensure_ready() {
    pj_step "Checking TUI readiness"

    # Check if TUI directory exists
    pj_check "TUI source directory"
    if [[ ! -d "$TUI_DIR/src" ]]; then
        log_ux "DEBUG" "TUI source directory not found"
        pj_missing "$TUI_DIR/src (TUI not available)"
        return 1
    fi
    pj_found "$TUI_DIR/src"

    # Check if rebuild is needed
    if ux_tui_needs_rebuild; then
        ux_tui_build
        return $?
    fi

    return 0
}

# Launch the rich TUI
# Returns 0 if TUI launched and exited cleanly, 1 if not available
ux_launch_tui() {
    local model_name="$1"
    local tui_bin

    pj_step "Launching TUI"
    tui_bin="$(ux_tui_binary)"

    if [[ -z "$tui_bin" ]]; then
        log_ux "DEBUG" "TUI binary not found, falling back to bash prompt"
        pj_missing "TUI binary (using simple mode)"
        return 1
    fi
    pj_found "TUI at $tui_bin"

    # Ensure we have a TTY (TUI requires interactive terminal)
    pj_check "TTY availability"
    if ! [ -t 0 ] || ! [ -t 1 ]; then
        log_ux "ERROR" "No TTY available for TUI"
        pj_missing "TTY (interactive terminal required)"
        ux_error "TUI requires an interactive terminal"
        ux_info "Solutions:"
        ux_info "  • Run from interactive shell"
        ux_info "  • SSH with: ssh -t user@host"
        ux_info "  • Toolbox: toolbox run --directory \$PWD ./yollayah.sh"
        return 1
    fi
    pj_result "TTY available (stdin and stdout are terminals)"

    # If using daemon mode, ensure conductor is running first
    if conductor_needs_daemon; then
        log_ux "INFO" "Daemon mode enabled, ensuring Conductor is running"
        pj_step "Starting Conductor daemon (socket mode)"
        if ! conductor_ensure_running; then
            log_ux "ERROR" "Failed to start Conductor daemon"
            pj_result "Conductor failed to start"
            return 1
        fi
    fi

    log_ux "INFO" "Launching rich TUI: $tui_bin"
    pj_result "Launching: $tui_bin"

    # Export model info for TUI to use
    pj_step "Setting up TUI environment"
    export YOLLAYAH_MODEL="$model_name"
    export YOLLAYAH_OLLAMA_HOST="${OLLAMA_HOST:-localhost}"
    export YOLLAYAH_OLLAMA_PORT="${OLLAMA_PORT:-11434}"
    pj_result "Model: $model_name, Host: ${OLLAMA_HOST:-localhost}:${OLLAMA_PORT:-11434}"

    # Export paths for TUI shell integration (routing, task management)
    export YOLLAYAH_SCRIPT_DIR="$SCRIPT_DIR"
    export YOLLAYAH_STATE_DIR="$STATE_DIR"

    # Launch TUI (it takes over the terminal)
    "$tui_bin"
    local exit_code=$?

    # Note: We don't stop the daemon here - it may be serving other surfaces
    # Use 'conductor_stop' explicitly if needed

    return $exit_code
}

# Start UI - tries TUI first, falls back to bash
ux_start_interface() {
    local model_name="$1"
    pj_step "Starting user interface"

    # Ensure TUI is built and up to date (rebuilds if sources changed)
    ux_tui_ensure_ready

    if ux_tui_available; then
        log_ux "INFO" "Launching rich TUI interface"
        pj_result "Using rich TUI interface"

        # Try to launch TUI, fall back to bash if it fails (e.g., no TTY)
        if ! ux_launch_tui "$model_name"; then
            log_ux "WARN" "TUI launch failed, falling back to simple mode"
            pj_result "Falling back to simple bash interface"
            ux_conversation_loop "$model_name"
        fi
    else
        log_ux "INFO" "Starting conversation loop (bash interface)"
        pj_result "Using simple bash interface"
        ux_conversation_loop "$model_name"
    fi
}
