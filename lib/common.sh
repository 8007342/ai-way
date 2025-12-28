#!/bin/bash
# ============================================================================
# lib/common.sh - Shared utilities for Yollayah
#
# This module provides:
# - Terminal colors and formatting
# - Logging functions (info, success, warn, error)
# - Path configuration (all relative to SCRIPT_DIR)
# - Global state variables
# - Utility functions
#
# Constitution Reference:
# - Law of Truth: Logging is honest and transparent
# - Law of Foundation: Paths are predictable and secure
# ============================================================================

# Prevent double-sourcing
[[ -n "$_YOLLAYAH_COMMON_LOADED" ]] && return 0
_YOLLAYAH_COMMON_LOADED=1

# ============================================================================
# Terminal Colors
# ============================================================================

readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[0;33m'
readonly BLUE='\033[0;34m'
readonly MAGENTA='\033[0;35m'
readonly CYAN='\033[0;36m'
readonly WHITE='\033[1;37m'
readonly NC='\033[0m'  # No Color

# ============================================================================
# Paths - Everything relative to script directory
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
# Logging Functions
#
# Constitution Reference:
# - Law of Truth: "Be honest, always"
# - These functions provide clear, honest feedback to AJ
# ============================================================================

info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

debug() {
    # Only show debug messages if YOLLAYAH_DEBUG is set
    [[ -n "$YOLLAYAH_DEBUG" ]] && echo -e "${BLUE}[DEBUG]${NC} $1"
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
