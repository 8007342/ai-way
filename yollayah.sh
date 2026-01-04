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
# Toolbox Integration (Fedora Silverblue)
# ============================================================================

# Detect if running inside toolbox container
# The /run/.toolboxenv file is created by toolbox when entering a container
if [[ -f /run/.toolboxenv ]]; then
    INSIDE_TOOLBOX=true
    # Extract container name from containerenv file (standard toolbox metadata)
    # Format: name="container-name"
    if [[ -f /run/.containerenv ]]; then
        TOOLBOX_NAME=$(grep -oP 'name="\K[^"]+' /run/.containerenv 2>/dev/null || echo "unknown")
    else
        TOOLBOX_NAME="unknown"
    fi
else
    INSIDE_TOOLBOX=false
    TOOLBOX_NAME=""
fi

# Check if toolbox command is available on system
# Toolbox is pre-installed on Fedora Silverblue but may not exist on other distros
if command -v toolbox &> /dev/null; then
    TOOLBOX_AVAILABLE=true
else
    TOOLBOX_AVAILABLE=false
fi

# Check if ai-way toolbox container exists
# Only check if toolbox command is available to avoid errors on other distros
# Use awk to check column 2 (container name) to avoid false positives
if [[ "$TOOLBOX_AVAILABLE" == "true" ]] && toolbox list 2>/dev/null | awk 'NR>1 && $2=="ai-way"' | grep -q .; then
    TOOLBOX_EXISTS=true
else
    TOOLBOX_EXISTS=false
fi

# Export variables for child processes (daemon, TUI)
export INSIDE_TOOLBOX TOOLBOX_NAME TOOLBOX_AVAILABLE TOOLBOX_EXISTS

# ============================================================================
# Toolbox Auto-Enter & Creation (Phase 1.2 & 1.3)
# ============================================================================

# Auto-enter ai-way toolbox if available (Silverblue)
_toolbox_auto_enter() {
    # Skip if already inside toolbox
    [[ "$INSIDE_TOOLBOX" == "true" ]] && return 0

    # Skip if toolbox not available (other distros)
    [[ "$TOOLBOX_AVAILABLE" != "true" ]] && return 0

    # If ai-way toolbox exists, enter it
    if [[ "$TOOLBOX_EXISTS" == "true" ]]; then
        echo "🔧 Entering ai-way toolbox container..."
        exec toolbox run -c ai-way "$SCRIPT_DIR/yollayah.sh" "$@"
    fi

    # Toolbox doesn't exist, return to trigger creation
    return 1
}

# Create ai-way toolbox if needed
_toolbox_create_and_enter() {
    # Skip if already in toolbox or toolbox not available
    [[ "$INSIDE_TOOLBOX" == "true" ]] && return 0
    [[ "$TOOLBOX_AVAILABLE" != "true" ]] && return 0

    # Check if already exists (shouldn't happen, but be safe)
    if [[ "$TOOLBOX_EXISTS" == "true" ]]; then
        return 0
    fi

    # Create ai-way toolbox
    echo ""
    echo "🚀 First-time setup: Creating ai-way toolbox container..."
    echo "   This provides clean dependency isolation on Silverblue."
    echo "   (One-time setup, takes ~30 seconds)"
    echo ""

    if toolbox create ai-way; then
        echo ""
        echo "✅ Toolbox created successfully!"
        echo "   Entering container..."
        echo ""
        exec toolbox run -c ai-way "$SCRIPT_DIR/yollayah.sh" "$@"
    else
        echo ""
        echo "❌ Failed to create toolbox container."
        echo "   Try manually: toolbox create ai-way"
        echo ""
        exit 1
    fi
}

# Try to auto-enter toolbox (exits via exec if successful)
_toolbox_auto_enter "$@" || _toolbox_create_and_enter "$@"

# ============================================================================
# Bootstrap: Load Modules
# ============================================================================

# Core utilities (paths, helper functions) - must be first
source "${SCRIPT_DIR}/yollayah/lib/common.sh"

# Logging bus - loaded early so all modules can log
# Logs go to workdir/logs/ for PJ debugging, not visible to AJ
source "${SCRIPT_DIR}/yollayah/lib/logging/init.sh"

# UX output - loaded before modules that display to AJ
# All user-facing output goes through ux_* functions
source "${SCRIPT_DIR}/yollayah/lib/ux/output.sh"

# Integrity verification - loaded early, runs environment sanitization immediately
# This cannot be bypassed - environment.sh always runs
source "${SCRIPT_DIR}/yollayah/lib/integrity/init.sh"

# Ollama management
source "${SCRIPT_DIR}/yollayah/lib/ollama/service.sh"
source "${SCRIPT_DIR}/yollayah/lib/ollama/lifecycle.sh"

# Agent repository
source "${SCRIPT_DIR}/yollayah/lib/agents/sync.sh"

# Yollayah personality
source "${SCRIPT_DIR}/yollayah/lib/yollayah/personality.sh"

# Yollayah setup (first-run, dependency installation)
source "${SCRIPT_DIR}/yollayah/lib/yollayah/setup.sh"

# Routing module (specialist agents, task management)
source "${SCRIPT_DIR}/yollayah/lib/routing/init.sh"

# User experience
source "${SCRIPT_DIR}/yollayah/lib/ux/terminal.sh"

# User customizations (privacy-first, mostly placeholders)
source "${SCRIPT_DIR}/yollayah/lib/user/init.sh"

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
  test      Test mode: builds yollayah from qwen2:0.5b (~352MB, fast inference)
  daemon    Start daemon only (runs in background)
  connect   Connect TUI to existing daemon
  stop      Stop daemon gracefully
  restart   Restart daemon
  status    Show daemon status

Options:
  --help, -h    Show this help message
  --version     Show version

Environment Variables:
  CONDUCTOR_SOCKET            Unix socket path for daemon communication
  CONDUCTOR_PID               PID file path for daemon process
  AI_WAY_LOG                  Log level (trace, debug, info, warn, error)
  YOLLAYAH_TEST_MODEL         Override test mode model (default: qwen2:0.5b)
  YOLLAYAH_OLLAMA_KEEP_ALIVE  Model keep-alive duration (default: 24h, use -1 for forever)

Toolbox Mode (Fedora Silverblue):
  On Silverblue, ai-way automatically runs inside a toolbox container
  for better dependency isolation. The ai-way toolbox is created
  automatically on first run.

  To manually manage the toolbox:
    toolbox create ai-way    # Create container
    toolbox enter ai-way     # Enter container
    toolbox rm ai-way        # Remove container (clean uninstall)

Examples:
  yollayah.sh                           # Auto-enters toolbox (Silverblue)
  yollayah.sh --test                    # Test mode in toolbox
  toolbox enter ai-way                  # Manually enter toolbox
  YOLLAYAH_OLLAMA_KEEP_ALIVE=-1 \
    yollayah.sh                         # Keep models loaded forever
  yollayah.sh daemon                    # Start daemon only
  yollayah.sh connect                   # Connect to running daemon
  yollayah.sh status                    # Check if daemon is running
  AI_WAY_LOG=debug yollayah.sh          # Start with debug logging
  YOLLAYAH_TEST_MODEL=tinyllama:1.1b \
    yollayah.sh --test                  # Test mode with custom model
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
    # Skip pretty output in test verbose mode
    if [[ -z "${YOLLAYAH_TEST_VERBOSE:-}" ]]; then
        clear
        ux_print_banner
    fi

    # Verify script integrity FIRST (before any other operations)
    # Environment sanitization already ran when integrity module was sourced
    integrity_verify || exit 1

    # Skip pretty output in test verbose mode
    if [[ -z "${YOLLAYAH_TEST_VERBOSE:-}" ]]; then
        ux_show_startup_progress
    fi

    # Run first-time setup if needed (gracious sudo handling)
    setup_run || exit 1

    # Check Ollama is installed (should be after setup)
    ollama_check_installed || exit 1

    # Record pre-Yollayah state (for cleanup)
    ollama_record_state

    # Register cleanup handler
    ollama_register_cleanup

    # === SYNCHRONOUS BOOTSTRAP ===
    # Complete ALL setup before launching TUI to avoid terminal corruption
    # See TODO-architecture-terminal-ownership.md for design rationale

    # Ensure Ollama is running
    ollama_ensure_running || exit 1

    # Select best model for hardware (test mode uses tiny model)
    model_select_best

    # Test mode: minimal bootstrap, skip non-essential operations
    if [[ -n "${YOLLAYAH_TEST_MODE:-}" ]]; then
        if [[ -n "${YOLLAYAH_TEST_VERBOSE:-}" ]]; then
            echo ">>> Test mode: skipping non-essential operations"
            echo ">>> Ensuring model ready: $SELECTED_MODEL"
        else
            log_info "Test mode: skipping non-essential operations"
        fi

        # Pull test model if needed (small download, fast)
        model_ensure_ready || exit 1

        # Skip agents_sync, yollayah_create_model, routing_init, user_init
        if [[ -n "${YOLLAYAH_TEST_VERBOSE:-}" ]]; then
            echo ">>> Test mode ready! Using $SELECTED_MODEL"
        else
            ux_blank
            ux_success "Test mode ready! Using $SELECTED_MODEL"
        fi
    else
        # Normal mode: full bootstrap

        # Verify GPU usage (warn if GPU detected but Ollama not using it)
        verify_ollama_gpu_usage || true

        # Ensure model is ready (may take time on first run)
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
    fi

    # Bootstrap complete - NOW launch interface
    # In test+interactive mode, just exit successfully (integration test)
    if [[ "${YOLLAYAH_TEST_MODE:-}" == "1" && "${YOLLAYAH_INTERACTIVE:-}" == "1" ]]; then
        echo "✅ Integration test complete: All systems initialized successfully"
        echo "   Model: $YOLLAYAH_MODEL_NAME"
        echo "   Ready for interactive sessions"
        return 0
    fi

    # Use bash interface if --interactive flag is set, otherwise try TUI
    if [[ "${YOLLAYAH_INTERACTIVE:-}" == "1" ]]; then
        echo "🖥️  Interactive mode: Using simple bash interface"
        ux_conversation_loop "$YOLLAYAH_MODEL_NAME"
    else
        ux_start_interface "$YOLLAYAH_MODEL_NAME"
    fi
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
    test|--test)
        # Test mode: minimal bootstrap with tiny model + verbose logging
        export YOLLAYAH_TEST_MODE=1
        export YOLLAYAH_TEST_MODEL="${YOLLAYAH_TEST_MODEL:-qwen2:0.5b}"
        export YOLLAYAH_TEST_VERBOSE=1
        echo "🧪 TEST MODE - Verbose logging enabled"
        echo "Model: $YOLLAYAH_TEST_MODEL"
        echo "----------------------------------------"
        main "$@"
        ;;
    interactive|--interactive)
        # Interactive mode: Use simple bash interface instead of TUI
        export YOLLAYAH_INTERACTIVE=1
        echo "🖥️  INTERACTIVE MODE - Using bash interface (no TUI)"
        echo "Combine with --test for fast integration testing:"
        echo "  ./yollayah.sh --test --interactive"
        echo "----------------------------------------"
        main "$@"
        ;;
    test-interactive|--test-interactive)
        # Combined test+interactive mode: Fast integration test
        export YOLLAYAH_TEST_MODE=1
        export YOLLAYAH_INTERACTIVE=1
        export YOLLAYAH_TEST_MODEL="${YOLLAYAH_TEST_MODEL:-qwen2:0.5b}"
        echo "🧪 INTEGRATION TEST MODE"
        echo "Fast startup + initialization check (no UI launched)"
        echo "Model: $YOLLAYAH_TEST_MODEL"
        echo "----------------------------------------"
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
