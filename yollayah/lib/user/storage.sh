#!/bin/bash
# ============================================================================
# lib/user/storage.sh - User Data Storage Abstraction
#
# This module provides the WHERE and HOW for user data storage.
# It enforces our privacy principles at the storage layer.
#
# Core Principle: EVERYTHING STAYS IN SCRIPT_DIR
#
# We do NOT use:
# - ~/.config/ (standard, easily searched)
# - ~/.local/ (standard, easily searched)
# - /tmp/ (shared, potentially logged)
# - Any XDG directories
#
# We DO use:
# - $SCRIPT_DIR/.user/ (non-standard, deleted with ai-way)
#
# Constitution Reference:
# - Four Protections: "Protect AJ from third parties"
# - Law of Care: "First, do no harm" - no data in obvious places
#
# See README.md for the full privacy manifesto.
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_USER_STORAGE_LOADED:-}" ]] && return 0
_YOLLAYAH_USER_STORAGE_LOADED=1

# ============================================================================
# Storage Paths
# ============================================================================

# All paths are defined in common.sh:
# - USER_DIR="${SCRIPT_DIR}/.user"
# - USER_SETTINGS="${USER_DIR}/settings"
# - USER_PREFERENCES="${USER_DIR}/preferences"

# Subdirectories (created on demand, not at startup)
readonly USER_HISTORY_DIR="${USER_DIR}/history"
readonly USER_MEMORY_DIR="${USER_DIR}/memory"

# ============================================================================
# Storage Operations
# ============================================================================

# Ensure user directory exists (only call when user opts in)
storage_ensure_user_dir() {
    if [[ ! -d "$USER_DIR" ]]; then
        debug "Creating user data directory: $USER_DIR"
        mkdir -p "$USER_DIR"
        chmod 700 "$USER_DIR"  # Owner-only access
    fi
}

# Write a value to a key-value store file
# Usage: storage_set "preferences" "sass_level" "high"
storage_set() {
    local store="$1"
    local key="$2"
    local value="$3"
    local store_file="${USER_DIR}/${store}"

    # TODO: Implement when preferences are ready
    # This is a placeholder that refuses to work
    #
    # Privacy consideration:
    # Even simple key-value storage can leak information.
    # Before implementing, we need to:
    # 1. Define what keys are allowed
    # 2. Ensure values don't contain sensitive data
    # 3. Consider if this enables fingerprinting

    warn "storage_set not yet implemented (privacy review pending)"
    return 1
}

# Read a value from a key-value store file
# Usage: storage_get "preferences" "sass_level"
storage_get() {
    local store="$1"
    local key="$2"
    local store_file="${USER_DIR}/${store}"

    # TODO: Implement when preferences are ready
    return 1
}

# Delete a key from a store
storage_delete() {
    local store="$1"
    local key="$2"

    # TODO: Implement
    return 1
}

# ============================================================================
# File Storage (for larger data like history)
# ============================================================================

# Write content to a file in user storage
# Usage: storage_write_file "history/2024-01-15.log" "content"
storage_write_file() {
    local path="$1"
    local content="$2"
    local full_path="${USER_DIR}/${path}"

    # TODO: Implement when history is ready
    #
    # Privacy considerations for file storage:
    # 1. Filenames can leak information (dates, session IDs)
    # 2. File metadata (mtime) can reveal usage patterns
    # 3. Content could contain anything - how do we handle sensitive data?
    #
    # Before implementing, we need:
    # - Clear policy on what can be stored
    # - Decision on encryption (see README.md for tradeoffs)
    # - Auto-deletion policy

    warn "storage_write_file not yet implemented (privacy review pending)"
    return 1
}

# Read a file from user storage
storage_read_file() {
    local path="$1"
    local full_path="${USER_DIR}/${path}"

    if [[ -f "$full_path" ]]; then
        cat "$full_path"
    else
        return 1
    fi
}

# ============================================================================
# Secure Deletion
# ============================================================================

# Securely delete a file (overwrite before unlinking)
# Note: This is best-effort. SSDs may retain data in wear-leveling.
storage_secure_delete() {
    local path="$1"
    local full_path="${USER_DIR}/${path}"

    if [[ -f "$full_path" ]]; then
        # Overwrite with random data
        if command_exists shred; then
            shred -u "$full_path"
        else
            # Fallback: overwrite with zeros then delete
            dd if=/dev/zero of="$full_path" bs=1k count=$(stat -f%z "$full_path" 2>/dev/null || stat -c%s "$full_path") 2>/dev/null
            rm -f "$full_path"
        fi
    fi
}

# ============================================================================
# Storage Info
# ============================================================================

# Get total size of user storage
storage_size() {
    if [[ -d "$USER_DIR" ]]; then
        du -sh "$USER_DIR" 2>/dev/null | cut -f1
    else
        echo "0"
    fi
}

# List all stored data categories
storage_list_categories() {
    if [[ -d "$USER_DIR" ]]; then
        ls -1 "$USER_DIR" 2>/dev/null
    fi
}
