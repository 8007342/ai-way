#!/bin/bash
# ============================================================================
# lib/integrity/init.sh - Integrity Module Entry Point
#
# This module orchestrates script integrity verification.
#
# Integrity Levels:
#   0 - Git Pull (paranoid): Fetch from remote every time
#   1 - Checksums (default): Verify SHA-256 hashes locally
#   2 - Signatures (future): Cryptographic signature verification
#   disabled - Skip all integrity checks (PJ mode)
#
# Environment Variables:
#   YOLLAYAH_INTEGRITY_LEVEL  - Set integrity level (0, 1, 2, disabled)
#   YOLLAYAH_SKIP_INTEGRITY   - Set to 1 to skip (alias for disabled)
#   YOLLAYAH_INTEGRITY_GENERATE - Set to 1 to generate checksums
#
# Constitution Reference:
# - Law of Care: Default to safety (Level 1)
# - Law of Truth: Honest about what each level protects
# - Four Protections: PJ can disable if informed
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_INTEGRITY_LOADED:-}" ]] && return 0
_YOLLAYAH_INTEGRITY_LOADED=1

# ============================================================================
# Load Submodules
# ============================================================================

# Environment sanitization (always loaded, always runs)
source "${LIB_DIR}/integrity/environment.sh"

# Level-specific modules (loaded but not run until called)
source "${LIB_DIR}/integrity/level-0-git.sh"
source "${LIB_DIR}/integrity/level-1-checksums.sh"
source "${LIB_DIR}/integrity/level-2-signatures.sh"

# ============================================================================
# Configuration
# ============================================================================

# Default integrity level
readonly INTEGRITY_DEFAULT_LEVEL=1

# Determine integrity level from environment
_integrity_get_level() {
    # YOLLAYAH_SKIP_INTEGRITY is an alias for disabled
    if [[ "${YOLLAYAH_SKIP_INTEGRITY:-}" == "1" ]]; then
        echo "disabled"
        return
    fi

    # Check explicit level setting
    local level="${YOLLAYAH_INTEGRITY_LEVEL:-$INTEGRITY_DEFAULT_LEVEL}"

    # Validate level
    case "$level" in
        0|1|2|disabled)
            echo "$level"
            ;;
        *)
            warn "Invalid integrity level: $level"
            warn "Valid levels: 0, 1, 2, disabled"
            warn "Using default: $INTEGRITY_DEFAULT_LEVEL"
            echo "$INTEGRITY_DEFAULT_LEVEL"
            ;;
    esac
}

# ============================================================================
# Main Entry Point
# ============================================================================

# Run integrity verification based on configured level
integrity_verify() {
    local level
    level=$(_integrity_get_level)
    log_integrity "INFO" "Starting integrity verification (level=$level)"

    # Handle manifest generation mode (developers only)
    if integrity_should_generate; then
        log_integrity "INFO" "Generating integrity manifest (developer mode)"
        info "Generating integrity manifest..."
        integrity_generate_manifest
        info "Manifest generated. Exiting."
        info "Commit .integrity/ to version control."
        exit 0
    fi

    # Handle disabled mode
    if [[ "$level" == "disabled" ]]; then
        log_integrity "WARN" "Integrity checks DISABLED by user"
        warn "=========================================="
        warn "  INTEGRITY CHECKS DISABLED"
        warn "=========================================="
        warn ""
        warn "You have disabled integrity verification."
        warn "This is not recommended for normal use."
        warn ""
        warn "Reasons to disable:"
        warn "  - You're developing/modifying the scripts"
        warn "  - You're debugging an integrity issue"
        warn "  - You understand and accept the risks"
        warn ""
        warn "To re-enable:"
        warn "  unset YOLLAYAH_SKIP_INTEGRITY"
        warn "  unset YOLLAYAH_INTEGRITY_LEVEL"
        warn ""
        return 0
    fi

    # Run appropriate verification level
    case "$level" in
        0)
            log_integrity "INFO" "Running git-based verification (level 0)"
            integrity_verify_git
            ;;
        1)
            log_integrity "INFO" "Running checksum verification (level 1)"
            integrity_verify_checksums
            ;;
        2)
            log_integrity "INFO" "Running signature verification (level 2)"
            integrity_verify_signatures
            ;;
    esac

    log_integrity "INFO" "Integrity verification complete"
}

# ============================================================================
# Utility Functions
# ============================================================================

# Show current integrity configuration
integrity_status() {
    local level
    level=$(_integrity_get_level)

    # Use ux_* functions if available, fall back to raw echo
    if declare -f ux_section &>/dev/null; then
        ux_section "Integrity Configuration"
        ux_keyval "Current level" "$level"
        ux_blank
    else
        echo ""
        echo "Integrity Configuration"
        echo "========================"
        echo ""
        echo "Current level: $level"
        echo ""
    fi

    case "$level" in
        0)
            echo "Level 0: Git Pull (Paranoid Mode)"
            echo "  - Fetches from remote on every startup"
            echo "  - Requires network connection"
            echo "  - Resets local changes to match remote"
            echo "  - Slowest but most thorough"
            ;;
        1)
            echo "Level 1: Checksums (Default)"
            echo "  - Verifies SHA-256 hashes locally"
            echo "  - Works offline"
            echo "  - Fast verification"
            echo "  - Catches corruption and simple tampering"
            ;;
        2)
            echo "Level 2: Signatures (Future)"
            echo "  - Cryptographic signature verification"
            echo "  - NOT YET IMPLEMENTED"
            echo "  - Falls back to Level 1"
            ;;
        disabled)
            echo "DISABLED: No integrity verification"
            echo "  - Scripts run without verification"
            echo "  - Not recommended for normal use"
            ;;
    esac

    echo ""
    echo "Environment variables:"
    echo "  YOLLAYAH_INTEGRITY_LEVEL=${YOLLAYAH_INTEGRITY_LEVEL:-<not set>}"
    echo "  YOLLAYAH_SKIP_INTEGRITY=${YOLLAYAH_SKIP_INTEGRITY:-<not set>}"
    echo ""

    # Show manifest status for Level 1
    if [[ "$level" == "1" ]] || [[ "$level" == "2" ]]; then
        if integrity_manifest_exists; then
            echo "Checksums manifest: $(wc -l < "$INTEGRITY_CHECKSUMS_FILE") files tracked"
        else
            echo "Checksums manifest: NOT FOUND"
            echo "  Generate with: YOLLAYAH_INTEGRITY_GENERATE=1 ./yollayah.sh"
        fi
    fi

    echo ""
}

# Show help for integrity module
integrity_help() {
    cat << 'EOF'

INTEGRITY MODULE
================

Yollayah verifies script integrity before running to protect against
tampering and corruption.

LEVELS
------

  Level 0: Git Pull (Paranoid)
    Every startup fetches from GitHub and compares.
    Use when: You don't trust local filesystem.
    Set: YOLLAYAH_INTEGRITY_LEVEL=0

  Level 1: Checksums (Default)
    Verifies SHA-256 hashes of all script files.
    Use when: Normal operation, want offline support.
    Set: YOLLAYAH_INTEGRITY_LEVEL=1 (or don't set anything)

  Level 2: Signatures (Future)
    Cryptographic signature verification.
    Status: NOT YET IMPLEMENTED

  Disabled: Skip All Checks
    Runs scripts without any verification.
    Use when: Developing, debugging, or you understand the risks.
    Set: YOLLAYAH_SKIP_INTEGRITY=1

COMMANDS
--------

  Show status:
    YOLLAYAH_INTEGRITY_STATUS=1 ./yollayah.sh

  Generate checksums (developers):
    YOLLAYAH_INTEGRITY_GENERATE=1 ./yollayah.sh

  Skip verification (PJ mode):
    YOLLAYAH_SKIP_INTEGRITY=1 ./yollayah.sh

  Use paranoid mode:
    YOLLAYAH_INTEGRITY_LEVEL=0 ./yollayah.sh

THREAT MODEL
------------

What this protects against:
  ✓ Disk corruption (bit rot, bad sectors)
  ✓ Incomplete downloads
  ✓ PATH/LD_PRELOAD injection attacks
  ✓ Unsophisticated local tampering

What this does NOT protect against:
  ✗ Repository compromise (attacker modifies source)
  ✗ Sophisticated supply chain attacks
  ✗ Physical access / root compromise

For full details, see:
  agents/ai-way-docs/script-integrity-adr.md

EOF
}

# ============================================================================
# Special Modes
# ============================================================================

# Check if status display was requested
integrity_should_show_status() {
    [[ "${YOLLAYAH_INTEGRITY_STATUS:-}" == "1" ]]
}

# Handle special modes before main execution
_integrity_handle_special_modes() {
    if integrity_should_show_status; then
        integrity_status
        exit 0
    fi

    if integrity_should_generate; then
        # Will be handled in integrity_verify
        return
    fi
}

# Run special mode handling when sourced
_integrity_handle_special_modes
