#!/bin/bash
# ============================================================================
# lib/logging/init.sh - Logging Module Entry Point
#
# Initializes the category-based logging system for internal diagnostics.
#
# Key Principles:
# - Logs are for PJ (Power Joe/Jane) to debug issues
# - AJ never sees log messages - they get clean UX output
# - Logs are stored in .logs/ (gitignored, ephemeral by default)
# - Logs are deleted on clean shutdown unless YOLLAYAH_PERSIST_LOGS=1
#
# Log Categories (separate files):
#   yollayah.log  - Main operations, personality
#   ollama.log    - Ollama service, model management
#   agents.log    - Agent repository, routing
#   network.log   - Network operations, downloads
#   ux.log        - User experience layer
#   session.log   - Session metadata, startup/shutdown
#   integrity.log - Security checks
#
# Usage:
#   log_debug "Verbose detail"           # -> yollayah.log (YOLLAYAH_DEBUG=1)
#   log_info "Normal operation"          # -> yollayah.log
#   log_ollama "INFO" "Model selected"   # -> ollama.log
#   log_agents "INFO" "Syncing repo"     # -> agents.log
#   log_ux "INFO" "Banner displayed"     # -> ux.log
#
# For AJ-facing output, use lib/ux/output.sh instead:
#   ux_info "Checking dependencies..."
#   ux_success "Ready!"
#
# Environment Variables:
#   YOLLAYAH_PERSIST_LOGS=1  - Keep logs after shutdown (timestamped)
#   YOLLAYAH_DEBUG=1         - Enable DEBUG level logging
#   YOLLAYAH_LOG_RETENTION=7 - Days to keep persisted logs
#
# Constitution Reference:
# - Law of Truth: Complete, honest logs for debugging
# - Four Protections: AJ sees clean output, not internal noise
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_LOGGING_LOADED:-}" ]] && return 0
_YOLLAYAH_LOGGING_LOADED=1

# ============================================================================
# Load Submodules
# ============================================================================

source "${LIB_DIR}/logging/bus.sh"

# ============================================================================
# Convenience Wrapper
# ============================================================================

# Log and exit with error
die() {
    local message="$1"
    local exit_code="${2:-1}"

    log_fatal "$message"
    log_session_end "$exit_code"

    # Also show to user via UX (if available)
    if declare -f ux_error &>/dev/null; then
        ux_error "$message"
    else
        echo "FATAL: $message" >&2
    fi

    exit "$exit_code"
}

# ============================================================================
# Debug Helpers
# ============================================================================

# Log function entry (for tracing)
log_enter() {
    log_debug "-> ${FUNCNAME[1]}()" "${FUNCNAME[1]}"
}

# Log function exit
log_exit() {
    log_debug "<- ${FUNCNAME[1]}()" "${FUNCNAME[1]}"
}

# Log variable value
log_var() {
    local name="$1"
    local value="${!name}"
    log_debug "$name='$value'" "${FUNCNAME[1]}"
}

# ============================================================================
# Timing Helpers
# ============================================================================

# Start a timer
# Usage: local start=$(log_timer_start)
log_timer_start() {
    date +%s%3N
}

# End timer and log duration
# Usage: log_timer_end "$start" "operation_name"
log_timer_end() {
    local start="$1"
    local operation="$2"
    local end
    end=$(date +%s%3N)
    local duration=$((end - start))
    log_perf "$operation" "$duration"
}
