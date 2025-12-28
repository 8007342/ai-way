#!/bin/bash
# ============================================================================
# lib/logging/bus.sh - Logging Bus
#
# Internal logging for debugging and diagnostics. These logs are for PJ
# (Power Joe) to inspect - AJ never sees them.
#
# Log files are stored in .logs/ directory (gitignored, local only).
#
# Separation of concerns:
# - log_*  functions → Write to .logs/ files (PJ debugging)
# - ux_*   functions → Display to terminal (AJ interaction)
#
# Log Levels:
# - DEBUG: Verbose internal details (only when YOLLAYAH_DEBUG=1)
# - INFO:  Normal operational messages
# - WARN:  Something unexpected but not critical
# - ERROR: Something went wrong
# - FATAL: Unrecoverable error
#
# Constitution Reference:
# - Law of Truth: Logs are honest and complete
# - Four Protections: AJ doesn't see internal noise
# ============================================================================

# Prevent double-sourcing
[[ -n "$_YOLLAYAH_LOGGING_BUS_LOADED" ]] && return 0
_YOLLAYAH_LOGGING_BUS_LOADED=1

# ============================================================================
# Configuration
# ============================================================================

# Log directory (in script dir, not /home)
readonly LOG_DIR="${SCRIPT_DIR}/.logs"

# Current session log file
readonly LOG_SESSION_ID="$(date +%Y%m%d-%H%M%S)-$$"
readonly LOG_FILE="${LOG_DIR}/yollayah-${LOG_SESSION_ID}.log"

# Log retention (days) - auto-cleanup old logs
readonly LOG_RETENTION_DAYS="${YOLLAYAH_LOG_RETENTION:-7}"

# Log level (0=DEBUG, 1=INFO, 2=WARN, 3=ERROR, 4=FATAL)
# Default: INFO (skip debug unless YOLLAYAH_DEBUG=1)
_log_level=1
[[ "${YOLLAYAH_DEBUG:-}" == "1" ]] && _log_level=0

# Whether logging is enabled
_logging_enabled=true

# ============================================================================
# Initialization
# ============================================================================

# Initialize logging system
_log_init() {
    # Create log directory
    mkdir -p "$LOG_DIR" 2>/dev/null || {
        _logging_enabled=false
        return 1
    }

    # Set permissions (owner only)
    chmod 700 "$LOG_DIR" 2>/dev/null || true

    # Create session log file
    touch "$LOG_FILE" 2>/dev/null || {
        _logging_enabled=false
        return 1
    }

    # Write session header
    {
        echo "========================================"
        echo "Yollayah Session Log"
        echo "Started: $(date -Iseconds)"
        echo "PID: $$"
        echo "Script: ${BASH_SOURCE[-1]}"
        echo "========================================"
        echo ""
    } >> "$LOG_FILE"

    # Cleanup old logs
    _log_cleanup_old
}

# Cleanup logs older than retention period
_log_cleanup_old() {
    if [[ -d "$LOG_DIR" ]]; then
        find "$LOG_DIR" -name "yollayah-*.log" -type f -mtime +"$LOG_RETENTION_DAYS" -delete 2>/dev/null || true
    fi
}

# ============================================================================
# Core Logging Function
# ============================================================================

# Write to log file
# Usage: _log_write LEVEL "message" [source]
_log_write() {
    [[ "$_logging_enabled" != "true" ]] && return 0

    local level="$1"
    local message="$2"
    local source="${3:-${FUNCNAME[2]:-main}}"
    local timestamp
    timestamp=$(date +%H:%M:%S.%3N)

    # Format: [TIME] LEVEL [source] message
    printf "[%s] %-5s [%s] %s\n" "$timestamp" "$level" "$source" "$message" >> "$LOG_FILE"
}

# ============================================================================
# Public Logging Functions
# ============================================================================

# Debug log (verbose, only when YOLLAYAH_DEBUG=1)
log_debug() {
    [[ $_log_level -le 0 ]] && _log_write "DEBUG" "$1" "${2:-}"
}

# Info log (normal operations)
log_info() {
    [[ $_log_level -le 1 ]] && _log_write "INFO" "$1" "${2:-}"
}

# Warning log (unexpected but recoverable)
log_warn() {
    [[ $_log_level -le 2 ]] && _log_write "WARN" "$1" "${2:-}"
}

# Error log (something went wrong)
log_error() {
    [[ $_log_level -le 3 ]] && _log_write "ERROR" "$1" "${2:-}"
}

# Fatal log (unrecoverable, will exit)
log_fatal() {
    _log_write "FATAL" "$1" "${2:-}"
}

# ============================================================================
# Structured Logging
# ============================================================================

# Log with key-value pairs (for machine parsing)
# Usage: log_structured INFO "event_name" key1=value1 key2=value2
log_structured() {
    [[ "$_logging_enabled" != "true" ]] && return 0

    local level="$1"
    local event="$2"
    shift 2

    local timestamp
    timestamp=$(date -Iseconds)
    local pairs="$*"

    # JSON-ish format for easy parsing
    printf '{"ts":"%s","level":"%s","event":"%s",%s}\n' \
        "$timestamp" "$level" "$event" \
        "$(echo "$pairs" | sed 's/\([^=]*\)=\([^ ]*\)/"\1":"\2"/g' | tr ' ' ',')" \
        >> "$LOG_FILE"
}

# ============================================================================
# Log Categories (for filtering)
# ============================================================================

# Module-specific logging
log_module() {
    local module="$1"
    local level="$2"
    local message="$3"
    _log_write "$level" "$message" "$module"
}

# Performance logging
log_perf() {
    local operation="$1"
    local duration_ms="$2"
    log_structured "PERF" "$operation" duration_ms="$duration_ms"
}

# Security logging
log_security() {
    local event="$1"
    local details="$2"
    _log_write "SEC" "$event: $details" "security"
}

# ============================================================================
# Session Management
# ============================================================================

# Log session end
log_session_end() {
    local exit_code="${1:-0}"

    {
        echo ""
        echo "========================================"
        echo "Session ended: $(date -Iseconds)"
        echo "Exit code: $exit_code"
        echo "========================================"
    } >> "$LOG_FILE"
}

# Get current log file path (for PJ)
log_get_file() {
    echo "$LOG_FILE"
}

# Get log directory
log_get_dir() {
    echo "$LOG_DIR"
}

# ============================================================================
# Utility Functions
# ============================================================================

# Tail the current log (for debugging)
log_tail() {
    local lines="${1:-50}"
    tail -n "$lines" "$LOG_FILE" 2>/dev/null
}

# Show all session logs
log_list_sessions() {
    ls -lt "$LOG_DIR"/yollayah-*.log 2>/dev/null | head -20
}

# ============================================================================
# Initialize on source
# ============================================================================

_log_init
log_info "Logging initialized" "bus"
