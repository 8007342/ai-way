#!/usr/bin/env bash
# output.sh - Legacy output functions (wrappers)
#
# These wrap ux_* and log_* functions for backward compatibility.
# NEW CODE SHOULD USE ux_* for display and log_* for logging directly.
#
# Output Architecture:
# - log_* functions → Write to .logs/ (for PJ debugging)
# - ux_*  functions → Display to terminal (for AJ, hidden unless YOLLAYAH_DEBUG=1)
# - pj_*  functions → Debug display for PJ (only shown when YOLLAYAH_DEBUG=1)
# - info/success/warn/error → Legacy wrappers (use ux_* in new code)
#
# Debug Mode (YOLLAYAH_DEBUG=1):
# - pj_step()   → Show a step in progress
# - pj_cmd()    → Show command being run
# - pj_check()  → Show what's being checked
# - pj_result() → Show result of check/command
# - pj_found()  → Show something was found
# - pj_missing() → Show something wasn't found
#
# Constitution Reference:
# - Law of Truth: Logging is honest and transparent

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_OUTPUT_LOADED:-}" ]] && return 0
_YOLLAYAH_OUTPUT_LOADED=1

# ============================================================================
# Early Colors (before ux module loads)
# ============================================================================

readonly _C_RED='\033[0;31m'
readonly _C_GREEN='\033[0;32m'
readonly _C_YELLOW='\033[0;33m'
readonly _C_BLUE='\033[0;34m'
readonly _C_MAGENTA='\033[0;35m'
readonly _C_CYAN='\033[0;36m'
readonly _C_WHITE='\033[1;37m'
readonly _C_NC='\033[0m'

# ============================================================================
# Legacy Wrapper Functions
# ============================================================================

info() {
    if declare -f ux_info &>/dev/null; then
        ux_info "$1"
    else
        # Early output before ux module loads
        echo -e "${_C_CYAN}[INFO]${_C_NC} $1"
    fi
}

success() {
    if declare -f ux_success &>/dev/null; then
        ux_success "$1"
    else
        echo -e "${_C_GREEN}[OK]${_C_NC} $1"
    fi
}

warn() {
    if declare -f ux_warn &>/dev/null; then
        ux_warn "$1"
    else
        echo -e "${_C_YELLOW}[WARN]${_C_NC} $1"
    fi
}

error() {
    if declare -f ux_error &>/dev/null; then
        ux_error "$1"
    else
        echo -e "${_C_RED}[ERROR]${_C_NC} $1" >&2
    fi
}

# Debug only goes to log, never displayed to AJ
# When YOLLAYAH_DEBUG=1, use pj_* functions for user-facing debug output
debug() {
    if declare -f log_debug &>/dev/null; then
        log_debug "$1"
    fi
    # Only show to terminal if YOLLAYAH_DEBUG is set AND we're early in boot
    # After ux module loads, pj_* functions handle debug display
    if [[ -n "${YOLLAYAH_DEBUG:-}" ]] && ! declare -f log_debug &>/dev/null; then
        echo -e "${_C_BLUE}[DEBUG]${_C_NC} $1"
    fi
}
