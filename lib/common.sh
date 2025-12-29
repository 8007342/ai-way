#!/bin/bash
# ============================================================================
# lib/common.sh - Shared utilities for Yollayah
#
# This module provides:
# - Path configuration (all relative to SCRIPT_DIR)
# - Global state variables
# - Utility functions
# - Legacy output functions (wrappers for ux_* and log_*)
#
# Output Architecture:
# - log_* functions → Write to .logs/ (for PJ debugging)
# - ux_*  functions → Display to terminal (for AJ)
# - info/success/warn/error → Legacy wrappers (use ux_* in new code)
#
# Constitution Reference:
# - Law of Truth: Logging is honest and transparent
# - Law of Foundation: Paths are predictable and secure
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_COMMON_LOADED:-}" ]] && return 0
_YOLLAYAH_COMMON_LOADED=1

# ============================================================================
# Path Configuration
#
# Constitution Reference (Four Protections):
# "Protect AJ from third parties" - No data in standard locations
# All Yollayah data stays in the ai-way directory, not in /home
# ============================================================================

# SCRIPT_DIR must be set by bootstrap before sourcing
if [[ -z "$SCRIPT_DIR" ]]; then
    echo "ERROR: SCRIPT_DIR must be set before sourcing common.sh" >&2
    exit 1
fi

# Core paths
readonly LIB_DIR="${SCRIPT_DIR}/lib"
readonly AGENTS_DIR="${SCRIPT_DIR}/agents"
readonly AGENTS_REPO="https://github.com/8007342/agents.git"

# Runtime state (gitignored, ephemeral)
readonly STATE_DIR="${SCRIPT_DIR}/.state"
readonly STATE_FILE="${STATE_DIR}/ollama.state"

# Logs (gitignored, for PJ debugging)
readonly LOGS_DIR="${SCRIPT_DIR}/.logs"

# User customizations (gitignored, persistent but local-only)
# See lib/user/README.md for privacy policy
readonly USER_DIR="${SCRIPT_DIR}/.user"
readonly USER_SETTINGS="${USER_DIR}/settings"
readonly USER_PREFERENCES="${USER_DIR}/preferences"

# ============================================================================
# Global State Variables
# ============================================================================

# Ollama state tracking (set by ollama/service.sh)
OLLAMA_WAS_RUNNING=false
OLLAMA_SERVICE_WAS_ENABLED=false
OLLAMA_SERVICE_WAS_ACTIVE=false
WE_STARTED_OLLAMA=false

# Agents state (set by agents/sync.sh)
AGENTS_CHANGED=false

# Model state (set by ollama/lifecycle.sh)
SELECTED_MODEL=""
MODEL_NEEDS_UPDATE=false

# ============================================================================
# Legacy Output Functions (Wrappers)
#
# These wrap ux_* and log_* functions for backward compatibility.
# NEW CODE SHOULD USE ux_* for display and log_* for logging directly.
#
# After modules are loaded:
# - info()    → ux_info() + log_info()
# - success() → ux_success() + log_info()
# - warn()    → ux_warn() + log_warn()
# - error()   → ux_error() + log_error()
# - debug()   → log_debug() only (never shown to AJ)
# ============================================================================

# Colors for early output (before ux module loads)
readonly _C_RED='\033[0;31m'
readonly _C_GREEN='\033[0;32m'
readonly _C_YELLOW='\033[0;33m'
readonly _C_BLUE='\033[0;34m'
readonly _C_MAGENTA='\033[0;35m'
readonly _C_CYAN='\033[0;36m'
readonly _C_WHITE='\033[1;37m'
readonly _C_NC='\033[0m'

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
debug() {
    if declare -f log_debug &>/dev/null; then
        log_debug "$1"
    fi
    # Only show to terminal if YOLLAYAH_DEBUG is set AND we're early in boot
    if [[ -n "${YOLLAYAH_DEBUG:-}" ]] && ! declare -f log_debug &>/dev/null; then
        echo -e "${_C_BLUE}[DEBUG]${_C_NC} $1"
    fi
}

# ============================================================================
# Utility Functions
# ============================================================================

# Ensure a directory exists
ensure_dir() {
    local dir="$1"
    [[ -d "$dir" ]] || mkdir -p "$dir"
}

# Check if a command exists
command_exists() {
    command -v "$1" &> /dev/null
}

# Check if Ollama API is responding
ollama_is_running() {
    curl -s http://localhost:11434/api/tags > /dev/null 2>&1
}

# Get current git hash of a repo
get_git_hash() {
    local repo_dir="$1"
    (cd "$repo_dir" && git rev-parse HEAD 2>/dev/null)
}

# ============================================================================
# Initialization
# ============================================================================

# Ensure state directory exists (runtime, ephemeral)
ensure_dir "$STATE_DIR"

# Note: USER_DIR is NOT created here - that's handled by lib/user/init.sh
# with explicit consent. See lib/user/README.md for privacy policy.
