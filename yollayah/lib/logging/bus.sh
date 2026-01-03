#!/bin/bash
# ============================================================================
# lib/logging/bus.sh - Category-Based Logging Bus
#
# Internal logging for debugging and diagnostics. These logs are for PJ
# (Power Joe/Jane) to inspect - AJ never sees them.
#
# Log Categories:
# - yollayah.log  : Main Yollayah operations and personality
# - ollama.log    : Ollama service, model management
# - agents.log    : Agent repository, routing, specialists
# - network.log   : Network operations, downloads, connectivity
# - ux.log        : User experience layer, terminal output
# - session.log   : Session metadata, startup/shutdown info
#
# Log Lifecycle:
# - By default, logs are EPHEMERAL (deleted on clean shutdown)
# - Set YOLLAYAH_PERSIST_LOGS=1 to keep logs with timestamps
# - When persisted, logs are tagged with launch time (YYYYMMDDHHMM)
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
[[ -n "${_YOLLAYAH_LOGGING_BUS_LOADED:-}" ]] && return 0
_YOLLAYAH_LOGGING_BUS_LOADED=1

# ============================================================================
# Configuration
# ============================================================================

# Log directory (in script dir, not /home)
readonly LOG_DIR="${SCRIPT_DIR}/.logs"

# Session identifier (launch timestamp for persistence)
readonly LOG_LAUNCH_TIME="$(date +%Y%m%d%H%M)"
readonly LOG_SESSION_ID="$$"

# Whether to persist logs (default: false = ephemeral)
LOG_PERSIST="${YOLLAYAH_PERSIST_LOGS:-false}"
[[ "$LOG_PERSIST" == "1" ]] && LOG_PERSIST="true"

# Log categories
readonly LOG_CATEGORIES=(
    "yollayah"   # Main operations
    "ollama"     # Ollama service
    "agents"     # Agent repository/routing
    "network"    # Network operations
    "ux"         # User experience
    "session"    # Session metadata
    "integrity"  # Security/integrity checks
)

# Log retention (days) for persisted logs
readonly LOG_RETENTION_DAYS="${YOLLAYAH_LOG_RETENTION:-7}"

# Log level (0=DEBUG, 1=INFO, 2=WARN, 3=ERROR, 4=FATAL)
_log_level=1
[[ "${YOLLAYAH_DEBUG:-}" == "1" ]] && _log_level=0

# Whether logging is enabled
_logging_enabled=true

# ============================================================================
# Log File Paths
# ============================================================================

# Get log file path for a category
# If persisting: category-YYYYMMDDHHMM.log
# If ephemeral:  category.log (overwritten each session)
_log_file_for_category() {
    local category="$1"
    if [[ "$LOG_PERSIST" == "true" ]]; then
        echo "${LOG_DIR}/${category}-${LOG_LAUNCH_TIME}.log"
    else
        echo "${LOG_DIR}/${category}.log"
    fi
}

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

    # Create/clear category log files
    for category in "${LOG_CATEGORIES[@]}"; do
        local log_file
        log_file=$(_log_file_for_category "$category")

        # For ephemeral logs, clear on each session start
        if [[ "$LOG_PERSIST" != "true" ]]; then
            : > "$log_file" 2>/dev/null || continue
        fi

        touch "$log_file" 2>/dev/null || continue
        chmod 600 "$log_file" 2>/dev/null || true
    done

    # Create README.md for PJ
    _log_create_readme

    # Write session header to session.log
    _log_write_session_header

    # Cleanup old persisted logs
    _log_cleanup_old
}

# Create README explaining logs
_log_create_readme() {
    local readme="${LOG_DIR}/README.md"
    cat > "$readme" << 'EOF'
# Yollayah Logs Directory

This directory contains diagnostic logs for debugging Yollayah.
These are for **PJ (Power Joe/Jane)** - technical users who need to troubleshoot issues.

## ⚠️ Ephemeral by Default

**These logs are deleted on clean shutdown!**

To persist logs across sessions, set before running:
```bash
export YOLLAYAH_PERSIST_LOGS=1
./yollayah.sh
```

When persisted, logs are tagged with launch timestamp (YYYYMMDDHHMM).

## Log Files

| File | Description |
|------|-------------|
| `yollayah.log` | Main Yollayah operations, personality, model creation |
| `ollama.log` | Ollama service management, model pulls, lifecycle |
| `agents.log` | Agent repository sync, routing, specialist delegation |
| `network.log` | Network operations, downloads, API calls |
| `ux.log` | User experience layer, terminal output, prompts |
| `session.log` | Session metadata, startup/shutdown, timing |
| `integrity.log` | Security checks, environment sanitization |

## Log Format

Each line follows this format:
```
[HH:MM:SS.mmm] LEVEL [source] message
```

- **HH:MM:SS.mmm**: Timestamp with milliseconds
- **LEVEL**: DEBUG, INFO, WARN, ERROR, FATAL, PERF, SEC
- **source**: Function or module that logged the message
- **message**: The log content

## Log Levels

| Level | When Used |
|-------|-----------|
| DEBUG | Verbose details (only when YOLLAYAH_DEBUG=1) |
| INFO | Normal operations, checkpoints |
| WARN | Unexpected but recoverable situations |
| ERROR | Something went wrong |
| FATAL | Unrecoverable error, will exit |
| PERF | Performance timing measurements |
| SEC | Security-related events |

## Filtering Logs

```bash
# Show only errors and warnings
grep -E '\[(ERROR|WARN)\]' yollayah.log

# Show Ollama operations timeline
cat ollama.log

# Show performance metrics
grep PERF session.log

# Follow logs in real-time (while Yollayah runs)
tail -f yollayah.log
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `YOLLAYAH_PERSIST_LOGS` | `false` | Set to `1` to keep logs after shutdown |
| `YOLLAYAH_DEBUG` | `false` | Set to `1` to enable DEBUG level logging |
| `YOLLAYAH_LOG_RETENTION` | `7` | Days to keep persisted logs |

## For AJ Users

If you're not a developer, you don't need to look at these files!
Yollayah shows you friendly messages in the terminal.
These logs are only useful for diagnosing technical problems.

---
*This README is auto-generated. Logs in this directory are ephemeral unless YOLLAYAH_PERSIST_LOGS=1*
EOF
    chmod 644 "$readme" 2>/dev/null || true
}

# Write session header to session.log
_log_write_session_header() {
    local session_log
    session_log=$(_log_file_for_category "session")

    # Detect basic hardware info for header
    local gpu_info="none"
    if command -v nvidia-smi &>/dev/null; then
        gpu_info=$(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null | head -1 || echo "nvidia (query failed)")
    elif command -v rocm-smi &>/dev/null; then
        gpu_info="AMD ROCm"
    elif command -v lspci &>/dev/null; then
        gpu_info=$(lspci 2>/dev/null | grep -i 'vga\|3d' | head -1 | sed 's/.*: //' | cut -d'(' -f1 || echo "unknown")
    fi

    {
        echo "========================================"
        echo "Yollayah Session Log"
        echo "Started: $(date -Iseconds)"
        echo "Launch Time: $LOG_LAUNCH_TIME"
        echo "PID: $LOG_SESSION_ID"
        echo "Persist Logs: $LOG_PERSIST"
        echo "Debug Mode: ${YOLLAYAH_DEBUG:-0}"
        echo "Script Dir: $SCRIPT_DIR"
        echo "GPU: $gpu_info"
        echo "========================================"
        echo ""
    } >> "$session_log"
}

# Cleanup logs older than retention period
_log_cleanup_old() {
    if [[ -d "$LOG_DIR" ]] && [[ "$LOG_PERSIST" == "true" ]]; then
        # Only clean up timestamped logs (persisted ones)
        for category in "${LOG_CATEGORIES[@]}"; do
            find "$LOG_DIR" -name "${category}-*.log" -type f -mtime +"$LOG_RETENTION_DAYS" -delete 2>/dev/null || true
        done
    fi
}

# ============================================================================
# Core Logging Function
# ============================================================================

# Write to category log file
# Usage: _log_write CATEGORY LEVEL "message" [source]
_log_write() {
    [[ "$_logging_enabled" != "true" ]] && return 0

    local category="$1"
    local level="$2"
    local message="$3"
    local source="${4:-${FUNCNAME[3]:-main}}"
    local timestamp
    timestamp=$(date +%H:%M:%S.%3N)

    local log_file
    log_file=$(_log_file_for_category "$category")

    # Format: [TIME] LEVEL [source] message
    printf "[%s] %-5s [%s] %s\n" "$timestamp" "$level" "$source" "$message" >> "$log_file" 2>/dev/null
}

# ============================================================================
# Public Logging Functions
# ============================================================================

# Generic log with category
# Usage: log_to CATEGORY LEVEL "message" [source]
log_to() {
    local category="$1"
    local level="$2"
    local message="$3"
    local source="${4:-}"

    case "$level" in
        DEBUG) [[ $_log_level -le 0 ]] && _log_write "$category" "$level" "$message" "$source" ;;
        INFO)  [[ $_log_level -le 1 ]] && _log_write "$category" "$level" "$message" "$source" ;;
        WARN)  [[ $_log_level -le 2 ]] && _log_write "$category" "$level" "$message" "$source" ;;
        ERROR) [[ $_log_level -le 3 ]] && _log_write "$category" "$level" "$message" "$source" ;;
        FATAL) _log_write "$category" "$level" "$message" "$source" ;;
        *)     _log_write "$category" "$level" "$message" "$source" ;;
    esac
    return 0
}

# ---- Standard level-based logging (defaults to yollayah category) ----

log_debug() {
    [[ $_log_level -le 0 ]] && _log_write "yollayah" "DEBUG" "$1" "${2:-}"
    return 0
}

log_info() {
    [[ $_log_level -le 1 ]] && _log_write "yollayah" "INFO" "$1" "${2:-}"
    return 0
}

log_warn() {
    [[ $_log_level -le 2 ]] && _log_write "yollayah" "WARN" "$1" "${2:-}"
    return 0
}

log_error() {
    [[ $_log_level -le 3 ]] && _log_write "yollayah" "ERROR" "$1" "${2:-}"
    return 0
}

log_fatal() {
    _log_write "yollayah" "FATAL" "$1" "${2:-}"
}

# ---- Category-specific convenience functions ----

# Ollama logging
log_ollama() {
    log_to "ollama" "$1" "$2" "${3:-}"
}

# Agents/routing logging
log_agents() {
    log_to "agents" "$1" "$2" "${3:-}"
}

# Network logging
log_network() {
    log_to "network" "$1" "$2" "${3:-}"
}

# UX logging
log_ux() {
    log_to "ux" "$1" "$2" "${3:-}"
}

# Session logging
log_session() {
    log_to "session" "$1" "$2" "${3:-}"
}

# Integrity/security logging
log_integrity() {
    log_to "integrity" "$1" "$2" "${3:-}"
}

# ============================================================================
# Structured Logging
# ============================================================================

# Log with key-value pairs (for machine parsing)
# Usage: log_structured CATEGORY LEVEL "event_name" key1=value1 key2=value2
log_structured() {
    [[ "$_logging_enabled" != "true" ]] && return 0

    local category="$1"
    local level="$2"
    local event="$3"
    shift 3

    local timestamp
    timestamp=$(date -Iseconds)
    local pairs="$*"

    local log_file
    log_file=$(_log_file_for_category "$category")

    # JSON-ish format for easy parsing
    printf '{"ts":"%s","level":"%s","event":"%s",%s}\n' \
        "$timestamp" "$level" "$event" \
        "$(echo "$pairs" | sed 's/\([^=]*\)=\([^ ]*\)/"\1":"\2"/g' | tr ' ' ',')" \
        >> "$log_file" 2>/dev/null
}

# Performance logging (goes to session.log)
log_perf() {
    local operation="$1"
    local duration_ms="$2"
    log_structured "session" "PERF" "$operation" duration_ms="$duration_ms"
}

# Security logging (goes to integrity.log)
log_security() {
    local event="$1"
    local details="$2"
    _log_write "integrity" "SEC" "$event: $details" "security"
}

# ============================================================================
# Session Management
# ============================================================================

# Log session end
log_session_end() {
    local exit_code="${1:-0}"
    local session_log
    session_log=$(_log_file_for_category "session")

    {
        echo ""
        echo "========================================"
        echo "Session ended: $(date -Iseconds)"
        echo "Exit code: $exit_code"
        echo "Duration: $(( $(date +%s) - ${_log_start_time:-$(date +%s)} ))s"
        echo "========================================"
    } >> "$session_log"
}

# Clean up logs on shutdown (if not persisting)
log_cleanup_on_exit() {
    local exit_code="${1:-0}"

    # Log session end first
    log_session_end "$exit_code"

    # Delete logs if not persisting
    if [[ "$LOG_PERSIST" != "true" ]]; then
        # Keep README but delete log files
        for category in "${LOG_CATEGORIES[@]}"; do
            rm -f "${LOG_DIR}/${category}.log" 2>/dev/null || true
        done
    fi
}

# ============================================================================
# Utility Functions
# ============================================================================

# Get log file path for a category
log_get_file() {
    local category="${1:-yollayah}"
    _log_file_for_category "$category"
}

# Get log directory
log_get_dir() {
    echo "$LOG_DIR"
}

# Tail a category log
log_tail() {
    local category="${1:-yollayah}"
    local lines="${2:-50}"
    local log_file
    log_file=$(_log_file_for_category "$category")
    tail -n "$lines" "$log_file" 2>/dev/null
}

# List all log files
log_list() {
    ls -la "$LOG_DIR"/*.log 2>/dev/null
}

# Check if logs will be persisted
log_is_persistent() {
    [[ "$LOG_PERSIST" == "true" ]]
}

# Get launch timestamp
log_get_launch_time() {
    echo "$LOG_LAUNCH_TIME"
}

# ============================================================================
# Initialize on source
# ============================================================================

_log_start_time=$(date +%s)
_log_init
log_session "INFO" "Logging initialized (persist=$LOG_PERSIST)" "bus"
