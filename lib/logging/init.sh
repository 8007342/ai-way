#!/bin/bash
# ============================================================================
# lib/logging/init.sh - Logging Module Entry Point
#
# Initializes the logging system for internal diagnostics.
#
# Key Principle:
# - Logs are for PJ (Power Joe) to debug issues
# - AJ never sees log messages - they get clean UX output
# - Logs are stored in .logs/ (gitignored, local only)
#
# Usage:
#   log_debug "Verbose detail"      # Only when YOLLAYAH_DEBUG=1
#   log_info "Normal operation"     # Always logged
#   log_warn "Something unexpected" # Warnings
#   log_error "Something broke"     # Errors
#
# For AJ-facing output, use lib/ux/output.sh instead:
#   ux_info "Checking dependencies..."
#   ux_success "Ready!"
#
# Constitution Reference:
# - Law of Truth: Complete, honest logs for debugging
# - Four Protections: AJ sees clean output, not internal noise
# ============================================================================

# Prevent double-sourcing
[[ -n "$_YOLLAYAH_LOGGING_LOADED" ]] && return 0
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
