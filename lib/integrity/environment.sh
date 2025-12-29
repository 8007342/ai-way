#!/bin/bash
# ============================================================================
# lib/integrity/environment.sh - Environment Sanitization (ALWAYS ON)
#
# This module provides baseline protection against common attack vectors.
# It CANNOT be disabled - these are fundamental safety measures.
#
# What this protects against:
# - PATH manipulation attacks (malicious binaries in current dir)
# - LD_PRELOAD injection (malicious shared libraries)
# - Execution from suspicious locations (/tmp, world-writable dirs)
#
# What this does NOT protect against:
# - Script content tampering (see checksums.sh)
# - Sophisticated supply chain attacks
# - Root-level compromise
#
# Constitution Reference:
# - Law of Care: "First, do no harm" - basic hygiene is non-negotiable
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_INTEGRITY_ENV_LOADED:-}" ]] && return 0
_YOLLAYAH_INTEGRITY_ENV_LOADED=1

# ============================================================================
# PATH Sanitization
# ============================================================================

# Reset PATH to known-safe directories
# This prevents attacks where malicious binaries are placed in current dir
# or other early PATH locations
_integrity_sanitize_path() {
    # Standard safe PATH (no current directory, no user-writable locations first)
    export PATH="/usr/local/bin:/usr/bin:/bin:/usr/local/sbin:/usr/sbin:/sbin"

    # Add common tool locations that might be needed
    # Homebrew on macOS
    [[ -d "/opt/homebrew/bin" ]] && export PATH="/opt/homebrew/bin:$PATH"
    # Linuxbrew
    [[ -d "/home/linuxbrew/.linuxbrew/bin" ]] && export PATH="/home/linuxbrew/.linuxbrew/bin:$PATH"

    debug "PATH sanitized"
}

# ============================================================================
# Dynamic Linker Sanitization
# ============================================================================

# Clear dangerous environment variables that could inject malicious code
_integrity_sanitize_linker() {
    # LD_PRELOAD: Forces loading of arbitrary shared libraries
    unset LD_PRELOAD

    # LD_LIBRARY_PATH: Changes library search order
    unset LD_LIBRARY_PATH

    # LD_AUDIT: Can hook into dynamic linker
    unset LD_AUDIT

    # Other potentially dangerous LD_* variables
    unset LD_AOUT_LIBRARY_PATH
    unset LD_AOUT_PRELOAD
    unset LD_DEBUG
    unset LD_DEBUG_OUTPUT
    unset LD_DYNAMIC_WEAK
    unset LD_ORIGIN_PATH
    unset LD_PROFILE
    unset LD_PROFILE_OUTPUT
    unset LD_SHOW_AUXV

    # Python-specific
    unset PYTHONHOME
    unset PYTHONPATH

    debug "Linker environment sanitized"
}

# ============================================================================
# Execution Context Validation
# ============================================================================

# Refuse to run from suspicious locations
_integrity_validate_location() {
    local script_path="$1"
    local script_dir
    script_dir="$(cd "$(dirname "$script_path")" && pwd)"

    # Refuse to run from /tmp (commonly used by malware)
    if [[ "$script_dir" == /tmp* ]]; then
        error "SECURITY: Refusing to run from /tmp"
        error "Scripts in /tmp are a common malware vector."
        error "Please run from a permanent location."
        return 1
    fi

    # Refuse to run from /var/tmp
    if [[ "$script_dir" == /var/tmp* ]]; then
        error "SECURITY: Refusing to run from /var/tmp"
        return 1
    fi

    # Refuse to run from /dev/shm (RAM disk, often used by malware)
    if [[ "$script_dir" == /dev/shm* ]]; then
        error "SECURITY: Refusing to run from /dev/shm"
        return 1
    fi

    debug "Execution location validated: $script_dir"
    return 0
}

# Check file permissions aren't overly permissive
_integrity_validate_permissions() {
    local script_path="$1"

    # Check if script is world-writable but not owned by us
    if [[ -w "$script_path" && ! -O "$script_path" ]]; then
        error "SECURITY: Script is writable by others: $script_path"
        error "This could indicate tampering."
        error "Fix with: chmod go-w '$script_path'"
        return 1
    fi

    # Check if script directory is world-writable
    local script_dir
    script_dir="$(dirname "$script_path")"
    if [[ -w "$script_dir" && ! -O "$script_dir" ]]; then
        warn "SECURITY: Script directory is world-writable: $script_dir"
        warn "Consider restricting permissions."
    fi

    debug "Permissions validated for: $script_path"
    return 0
}

# ============================================================================
# Bash Safety Settings
# ============================================================================

# Enable strict bash settings for safer execution
_integrity_bash_strict() {
    # Already set in bootstrap, but reinforce here
    set -o errexit   # Exit on error
    set -o nounset   # Error on undefined variables
    set -o pipefail  # Pipe failures propagate

    # Disable history expansion (prevents accidents with !)
    set +o histexpand 2>/dev/null || true

    # Restrict umask (new files not world-readable)
    umask 077

    debug "Bash strict mode enabled"
}

# ============================================================================
# Main Entry Point
# ============================================================================

# Run all environment sanitization
# This is called automatically when module is sourced
integrity_sanitize_environment() {
    debug "Running environment sanitization..."

    _integrity_sanitize_path
    _integrity_sanitize_linker
    _integrity_bash_strict

    # Validate execution context (pass script path)
    if [[ -n "${BASH_SOURCE[0]:-}" ]]; then
        # Find the main script (the one that started execution)
        local main_script="${BASH_SOURCE[-1]}"
        _integrity_validate_location "$main_script" || return 1
        _integrity_validate_permissions "$main_script" || return 1
    fi

    debug "Environment sanitization complete"
    return 0
}

# Run sanitization immediately when sourced
# This cannot be skipped - it's fundamental safety
integrity_sanitize_environment || {
    echo "FATAL: Environment sanitization failed" >&2
    exit 1
}
