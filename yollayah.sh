#!/bin/bash
# ============================================================================
# __   __    _ _                   _
# \ \ / /__ | | | __ _ _   _  __ _| |__
#  \ V / _ \| | |/ _` | | | |/ _` | '_ \
#   | | (_) | | | (_| | |_| | (_| | | | |
#   |_|\___/|_|_|\__,_|\__, |\__,_|_| |_|
#                      |___/
#
# ai-way-lite: Your local AI companion
# Just clone and run. That's it.
#
# This is the bootstrap script. It:
# 1. Sets up the environment
# 2. Sources the modular components
# 3. Manages the Conductor daemon
# 4. Runs the main entry point
#
# Commands:
#   yollayah.sh [start]   - Start daemon + TUI (default)
#   yollayah.sh daemon    - Start daemon only (background)
#   yollayah.sh connect   - Connect TUI to existing daemon
#   yollayah.sh stop      - Stop daemon gracefully
#   yollayah.sh restart   - Restart daemon
#   yollayah.sh status    - Show daemon status
#   yollayah.sh --help    - Show this help
#
# Environment Variables:
#   CONDUCTOR_SOCKET  - Unix socket path (default: XDG_RUNTIME_DIR/ai-way/conductor.sock)
#   CONDUCTOR_PID     - PID file path (default: XDG_RUNTIME_DIR/ai-way/conductor.pid)
#   AI_WAY_LOG        - Log level (trace, debug, info, warn, error)
#
# For module documentation, see lib/*/
# For privacy policy, see lib/user/README.md
# For ethical principles, see agents/CONSTITUTION.md (after first run)
# ============================================================================

set -e

# ============================================================================
# Bootstrap: Environment Setup
# ============================================================================

# Get script directory (all paths relative to this)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export SCRIPT_DIR

# ============================================================================
# Bootstrap: Load Modules
# ============================================================================

# Core utilities (paths, helper functions) - must be first
source "${SCRIPT_DIR}/lib/common.sh"

# Logging bus - loaded early so all modules can log
# Logs go to .logs/ for PJ debugging, not visible to AJ
source "${SCRIPT_DIR}/lib/logging/init.sh"

# UX output - loaded before modules that display to AJ
# All user-facing output goes through ux_* functions
source "${SCRIPT_DIR}/lib/ux/output.sh"

# Integrity verification - loaded early, runs environment sanitization immediately
# This cannot be bypassed - environment.sh always runs
source "${SCRIPT_DIR}/lib/integrity/init.sh"

# Ollama management
source "${SCRIPT_DIR}/lib/ollama/service.sh"
source "${SCRIPT_DIR}/lib/ollama/lifecycle.sh"

# Agent repository
source "${SCRIPT_DIR}/lib/agents/sync.sh"

# Yollayah personality
source "${SCRIPT_DIR}/lib/yollayah/personality.sh"

# Yollayah setup (first-run, dependency installation)
source "${SCRIPT_DIR}/lib/yollayah/setup.sh"

# Routing module (specialist agents, task management)
source "${SCRIPT_DIR}/lib/routing/init.sh"

# User experience
source "${SCRIPT_DIR}/lib/ux/terminal.sh"

# User customizations (privacy-first, mostly placeholders)
source "${SCRIPT_DIR}/lib/user/init.sh"

# ============================================================================
# Conductor Daemon Management (Phase 5.2)
# ============================================================================

# Environment variable defaults
: "${CONDUCTOR_SOCKET:=""}"
: "${CONDUCTOR_PID:=""}"
: "${AI_WAY_LOG:="info"}"

# Get resolved socket path (uses env var or defaults)
_get_socket_path() {
    if [[ -n "${CONDUCTOR_SOCKET}" ]]; then
        echo "$CONDUCTOR_SOCKET"
    else
        conductor_socket_path  # From lib/ux/terminal.sh
    fi
}

# Get resolved PID file path (uses env var or defaults)
_get_pid_path() {
    if [[ -n "${CONDUCTOR_PID}" ]]; then
        echo "$CONDUCTOR_PID"
    else
        conductor_pid_path  # From lib/ux/terminal.sh
    fi
}

# Check if daemon is running
# Returns 0 if running, 1 if not
check_daemon() {
    local socket pid_file pid

    socket="$(_get_socket_path)"
    pid_file="$(_get_pid_path)"

    # Check PID file first
    if [[ -f "$pid_file" ]]; then
        pid=$(cat "$pid_file" 2>/dev/null)
        if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
            # Process exists, check socket too
            if [[ -S "$socket" ]]; then
                return 0
            fi
        fi
    fi

    # Fallback: just check if socket exists
    [[ -S "$socket" ]]
}

# Start the conductor daemon in background
# Returns 0 on success, 1 on failure
start_daemon() {
    local daemon_bin socket_path socket_dir pid_file max_wait=15
    local log_level="${AI_WAY_LOG:-info}"

    daemon_bin="$(conductor_daemon_binary)"
    socket_path="$(_get_socket_path)"
    pid_file="$(_get_pid_path)"
    socket_dir="$(dirname "$socket_path")"

    # Check if already running
    if check_daemon; then
        echo "Conductor daemon is already running"
        return 0
    fi

    # Check if daemon binary exists
    if [[ -z "$daemon_bin" ]] || [[ ! -x "$daemon_bin" ]]; then
        echo "Error: Conductor daemon binary not found" >&2
        echo "Build it with: cargo build --release -p conductor-daemon" >&2
        return 1
    fi

    # Ensure socket directory exists
    if [[ ! -d "$socket_dir" ]]; then
        mkdir -p "$socket_dir" || {
            echo "Error: Failed to create socket directory: $socket_dir" >&2
            return 1
        }
    fi

    # Remove stale socket if it exists
    if [[ -S "$socket_path" ]]; then
        rm -f "$socket_path"
    fi

    echo "Starting Conductor daemon..."

    # Start daemon with daemonize flag
    "$daemon_bin" \
        --socket-path "$socket_path" \
        --pid-file "$pid_file" \
        --log-level "$log_level" \
        --daemonize

    # Wait for socket to appear
    local waited=0
    while [[ ! -S "$socket_path" ]] && [[ $waited -lt $max_wait ]]; do
        sleep 0.5
        waited=$((waited + 1))
    done

    if [[ ! -S "$socket_path" ]]; then
        echo "Error: Conductor daemon failed to start (socket not created after ${max_wait}s)" >&2
        return 1
    fi

    echo "Conductor daemon started successfully"
    echo "  Socket: $socket_path"
    if [[ -f "$pid_file" ]]; then
        echo "  PID: $(cat "$pid_file")"
    fi
    return 0
}

# Stop the conductor daemon gracefully
stop_daemon() {
    local pid_file pid socket_path

    pid_file="$(_get_pid_path)"
    socket_path="$(_get_socket_path)"

    if [[ ! -f "$pid_file" ]]; then
        # No PID file, check if socket exists
        if [[ -S "$socket_path" ]]; then
            echo "Warning: Socket exists but no PID file found" >&2
            echo "Removing stale socket: $socket_path"
            rm -f "$socket_path"
        else
            echo "Conductor daemon is not running"
        fi
        return 0
    fi

    pid=$(cat "$pid_file" 2>/dev/null)
    if [[ -z "$pid" ]]; then
        echo "Error: PID file is empty" >&2
        rm -f "$pid_file"
        return 1
    fi

    if ! kill -0 "$pid" 2>/dev/null; then
        echo "Conductor daemon not running (stale PID file)"
        rm -f "$pid_file" "$socket_path"
        return 0
    fi

    echo "Stopping Conductor daemon (PID: $pid)..."
    kill -TERM "$pid" 2>/dev/null

    # Wait for process to exit
    local waited=0
    while kill -0 "$pid" 2>/dev/null && [[ $waited -lt 10 ]]; do
        sleep 0.5
        waited=$((waited + 1))
    done

    if kill -0 "$pid" 2>/dev/null; then
        echo "Warning: Daemon did not stop gracefully, sending SIGKILL"
        kill -9 "$pid" 2>/dev/null || true
    fi

    # Clean up files
    rm -f "$pid_file" "$socket_path" 2>/dev/null

    echo "Conductor daemon stopped"
    return 0
}

# Show daemon status
show_status() {
    local socket_path pid_file pid

    socket_path="$(_get_socket_path)"
    pid_file="$(_get_pid_path)"

    echo "Conductor Daemon Status"
    echo "========================"
    echo "Socket path: $socket_path"
    echo "PID file:    $pid_file"
    echo ""

    if [[ -f "$pid_file" ]]; then
        pid=$(cat "$pid_file" 2>/dev/null)
        if [[ -n "$pid" ]]; then
            if kill -0 "$pid" 2>/dev/null; then
                echo "Status: RUNNING (PID: $pid)"
            else
                echo "Status: NOT RUNNING (stale PID file)"
            fi
        else
            echo "Status: UNKNOWN (empty PID file)"
        fi
    else
        echo "Status: NOT RUNNING (no PID file)"
    fi

    if [[ -S "$socket_path" ]]; then
        echo "Socket: exists"
    else
        echo "Socket: not found"
    fi
}

# Show usage help
show_usage() {
    cat << 'EOF'
Usage: yollayah.sh [COMMAND]

Commands:
  start     Start daemon and TUI (default if no command given)
  daemon    Start daemon only (runs in background)
  connect   Connect TUI to existing daemon
  stop      Stop daemon gracefully
  restart   Restart daemon
  status    Show daemon status

Options:
  --help, -h    Show this help message
  --version     Show version

Environment Variables:
  CONDUCTOR_SOCKET  Unix socket path for daemon communication
  CONDUCTOR_PID     PID file path for daemon process
  AI_WAY_LOG        Log level (trace, debug, info, warn, error)

Examples:
  yollayah.sh                  # Start normally (daemon + TUI)
  yollayah.sh daemon           # Start daemon only
  yollayah.sh connect          # Connect to running daemon
  yollayah.sh status           # Check if daemon is running
  AI_WAY_LOG=debug yollayah.sh # Start with debug logging
EOF
}

# Connect TUI to existing daemon (TUI only, no setup)
connect_tui() {
    local socket_path tui_bin

    socket_path="$(_get_socket_path)"

    # Check daemon is running
    if ! check_daemon; then
        echo "Error: Conductor daemon is not running" >&2
        echo "Start it with: $0 daemon" >&2
        echo "Or run: $0 start (to start both daemon and TUI)" >&2
        return 1
    fi

    tui_bin="$(ux_tui_binary)"
    if [[ -z "$tui_bin" ]] || [[ ! -x "$tui_bin" ]]; then
        echo "Error: TUI binary not found or not built" >&2
        echo "Run: cargo build --release -p yollayah-tui" >&2
        return 1
    fi

    echo "Connecting to Conductor daemon..."
    echo "  Socket: $socket_path"

    # Export for TUI
    export CONDUCTOR_SOCKET="$socket_path"
    export CONDUCTOR_TRANSPORT="unix"

    # Launch TUI
    exec "$tui_bin"
}

# ============================================================================
# Main Entry Point
# ============================================================================

main() {
    clear
    ux_print_banner

    # Verify script integrity FIRST (before any other operations)
    # Environment sanitization already ran when integrity module was sourced
    integrity_verify || exit 1

    ux_show_startup_progress

    # Run first-time setup if needed (gracious sudo handling)
    setup_run || exit 1

    # Check Ollama is installed (should be after setup)
    ollama_check_installed || exit 1

    # Record pre-Yollayah state (for cleanup)
    ollama_record_state

    # Register cleanup handler
    ollama_register_cleanup

    # Ensure Ollama is running
    ollama_ensure_running || exit 1

    # Select and pull best model for hardware
    model_select_best

    # Verify GPU usage (warn if GPU detected but Ollama not using it)
    verify_ollama_gpu_usage || true  # Don't fail, just warn

    model_ensure_ready || exit 1

    # Sync agents repository (the breadcrumb to YOU.md)
    agents_sync

    # Create Yollayah personality model
    yollayah_create_model || exit 1

    # Initialize routing module (specialist task management)
    routing_init

    # Initialize user module (no-op if no data)
    user_init

    # Ready!
    ux_show_all_ready

    # Start the interface (TUI if available, else bash prompt)
    ux_start_interface "$YOLLAYAH_MODEL_NAME"
}

# ============================================================================
# Command Line Parsing and Dispatch
# ============================================================================

# Parse command and dispatch
case "${1:-start}" in
    start|"")
        # Default: full startup (daemon + TUI)
        main "$@"
        ;;
    daemon|--daemon)
        # Start daemon only
        start_daemon
        exit $?
        ;;
    connect|--tui)
        # Connect TUI to existing daemon
        connect_tui
        exit $?
        ;;
    stop)
        # Stop daemon
        stop_daemon
        exit $?
        ;;
    restart)
        # Restart daemon
        stop_daemon
        sleep 1
        start_daemon
        exit $?
        ;;
    status)
        # Show status
        show_status
        exit $?
        ;;
    --help|-h|help)
        # Show help
        show_usage
        exit 0
        ;;
    --version)
        echo "yollayah.sh (ai-way-lite)"
        echo "Conductor daemon management script"
        exit 0
        ;;
    *)
        echo "Error: Unknown command '$1'" >&2
        echo "Run '$0 --help' for usage information" >&2
        exit 1
        ;;
esac
