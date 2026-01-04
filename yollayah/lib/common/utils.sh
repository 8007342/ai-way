#!/usr/bin/env bash
# utils.sh - Shared utility functions for Yollayah
#
# Provides common helper functions used across different modules.

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_UTILS_LOADED:-}" ]] && return 0
_YOLLAYAH_UTILS_LOADED=1

# ============================================================================
# Directory Management
# ============================================================================

# Ensure a directory exists
ensure_dir() {
    local dir="$1"
    [[ -d "$dir" ]] || mkdir -p "$dir"
}

# ============================================================================
# Command Checking
# ============================================================================

# Check if a command exists
command_exists() {
    command -v "$1" &> /dev/null
}

# ============================================================================
# Ollama Utilities
# ============================================================================

# Check if Ollama API is responding
ollama_is_running() {
    curl -s http://localhost:11434/api/tags > /dev/null 2>&1
}

# ============================================================================
# Git Utilities
# ============================================================================

# Get current git hash of a repo
get_git_hash() {
    local repo_dir="$1"
    (cd "$repo_dir" && git rev-parse HEAD 2>/dev/null)
}
