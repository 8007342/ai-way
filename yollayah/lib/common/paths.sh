#!/usr/bin/env bash
# paths.sh - Path configuration for Yollayah
#
# Constitution Reference (Four Protections):
# "Protect AJ from third parties" - No data in standard locations
# All Yollayah data stays in the ai-way directory, not in /home

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_PATHS_LOADED:-}" ]] && return 0
_YOLLAYAH_PATHS_LOADED=1

# SCRIPT_DIR must be set by bootstrap before sourcing
if [[ -z "$SCRIPT_DIR" ]]; then
    echo "ERROR: SCRIPT_DIR must be set before sourcing paths.sh" >&2
    exit 1
fi

# Core paths
readonly LIB_DIR="${SCRIPT_DIR}/yollayah/lib"
readonly AGENTS_DIR="${SCRIPT_DIR}/agents"
readonly AGENTS_REPO="https://github.com/8007342/agents.git"

# Runtime state (gitignored, ephemeral)
readonly STATE_DIR="${SCRIPT_DIR}/workdir/state"
readonly STATE_FILE="${STATE_DIR}/ollama.state"

# Logs (gitignored, for PJ debugging)
readonly LOGS_DIR="${SCRIPT_DIR}/workdir/logs"

# User customizations (gitignored, persistent but local-only)
# See lib/user/README.md for privacy policy
readonly USER_DIR="${SCRIPT_DIR}/.user"
readonly USER_SETTINGS="${USER_DIR}/settings"
readonly USER_PREFERENCES="${USER_DIR}/preferences"

# Ensure state directory exists (runtime, ephemeral)
ensure_dir() {
    local dir="$1"
    [[ -d "$dir" ]] || mkdir -p "$dir"
}

ensure_dir "$STATE_DIR"

# Note: USER_DIR is NOT created here - that's handled by lib/user/init.sh
# with explicit consent. See lib/user/README.md for privacy policy.
