#!/bin/bash
# ============================================================================
# lib/integrity/level-1-checksums.sh - Checksum Verification (DEFAULT)
#
# Level 1: The default integrity level
#
# What it does:
# - Verifies SHA-256 checksums of all script files
# - Compares against stored manifest (.integrity/checksums.sha256)
# - Fails fast if any file has been modified
#
# Trade-offs:
# - FAST: Local-only, no network required
# - OFFLINE: Works without internet
# - LIMITED: Only detects modification, not who modified
#
# The Chicken-and-Egg Problem:
# If an attacker modifies a script, they can also modify the checksums.
# This verification catches:
# - Accidental corruption (disk errors)
# - Unsophisticated tampering (script kiddie)
# - Post-download modifications
#
# It does NOT catch:
# - Attacker who modifies both script AND checksums
# - Repository compromise
#
# Constitution Reference:
# - Law of Truth: We're honest about these limitations
# - Law of Care: Better than nothing, catches real issues
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_INTEGRITY_CHECKSUMS_LOADED:-}" ]] && return 0
_YOLLAYAH_INTEGRITY_CHECKSUMS_LOADED=1

# ============================================================================
# Configuration
# ============================================================================

# Where checksums are stored
readonly INTEGRITY_CHECKSUMS_DIR="${SCRIPT_DIR}/workdir/.integrity"
readonly INTEGRITY_CHECKSUMS_FILE="${INTEGRITY_CHECKSUMS_DIR}/checksums.sha256"

# Files to verify (glob patterns)
readonly INTEGRITY_VERIFY_PATTERNS=(
    "yollayah.sh"
    "lib/*.sh"
    "lib/**/*.sh"
)

# ============================================================================
# Checksum Functions
# ============================================================================

# Generate SHA-256 checksum for a file
_integrity_checksum_file() {
    local file="$1"
    sha256sum "$file" 2>/dev/null | awk '{print $1}'
}

# Get stored checksum for a file
_integrity_stored_checksum() {
    local file="$1"
    local relative_path="${file#$SCRIPT_DIR/}"

    if [[ -f "$INTEGRITY_CHECKSUMS_FILE" ]]; then
        grep -E "^[a-f0-9]+  ${relative_path}$" "$INTEGRITY_CHECKSUMS_FILE" 2>/dev/null | awk '{print $1}'
    fi
}

# Verify a single file's checksum
_integrity_verify_file() {
    local file="$1"
    local expected actual

    expected=$(_integrity_stored_checksum "$file")
    actual=$(_integrity_checksum_file "$file")

    if [[ -z "$expected" ]]; then
        # No stored checksum - file is new or not tracked
        debug "No checksum for: ${file#$SCRIPT_DIR/}"
        return 0  # Don't fail on untracked files
    fi

    if [[ "$actual" != "$expected" ]]; then
        error "INTEGRITY FAILURE: ${file#$SCRIPT_DIR/}"
        error "  Expected: $expected"
        error "  Actual:   $actual"
        return 1
    fi

    debug "Verified: ${file#$SCRIPT_DIR/}"
    return 0
}

# ============================================================================
# Manifest Functions
# ============================================================================

# Check if checksums manifest exists
integrity_manifest_exists() {
    [[ -f "$INTEGRITY_CHECKSUMS_FILE" ]]
}

# Generate checksums manifest (for development use)
integrity_generate_manifest() {
    info "Generating checksums manifest..."

    ensure_dir "$INTEGRITY_CHECKSUMS_DIR"

    # Create new manifest
    local manifest_tmp="${INTEGRITY_CHECKSUMS_FILE}.tmp"
    : > "$manifest_tmp"

    # Find all files matching patterns
    local file
    for pattern in "${INTEGRITY_VERIFY_PATTERNS[@]}"; do
        while IFS= read -r -d '' file; do
            if [[ -f "$file" ]]; then
                local relative="${file#$SCRIPT_DIR/}"
                local checksum
                checksum=$(_integrity_checksum_file "$file")
                echo "$checksum  $relative" >> "$manifest_tmp"
            fi
        done < <(find "$SCRIPT_DIR" -path "$SCRIPT_DIR/$pattern" -type f -print0 2>/dev/null)
    done

    # Sort for consistent ordering
    sort -k2 "$manifest_tmp" > "$INTEGRITY_CHECKSUMS_FILE"
    rm -f "$manifest_tmp"

    local count
    count=$(wc -l < "$INTEGRITY_CHECKSUMS_FILE")
    success "Generated checksums for $count files"
    info "Manifest: $INTEGRITY_CHECKSUMS_FILE"
}

# Show manifest contents
integrity_show_manifest() {
    if integrity_manifest_exists; then
        echo "Checksums manifest: $INTEGRITY_CHECKSUMS_FILE"
        echo "---"
        cat "$INTEGRITY_CHECKSUMS_FILE"
        echo "---"
        echo "Total: $(wc -l < "$INTEGRITY_CHECKSUMS_FILE") files"
    else
        warn "No checksums manifest found"
        echo "Generate with: YOLLAYAH_INTEGRITY_GENERATE=1 ./yollayah.sh"
    fi
}

# ============================================================================
# Main Verification Function
# ============================================================================

# Run Level 1 verification
integrity_verify_checksums() {
    info "Level 1: Checksum verification"

    # Check if manifest exists
    if ! integrity_manifest_exists; then
        warn "No checksums manifest found"
        warn "This is normal on first run or after updates"
        warn ""
        warn "To generate manifest (developers only):"
        warn "  YOLLAYAH_INTEGRITY_GENERATE=1 ./yollayah.sh"
        warn ""
        warn "Continuing without checksum verification..."
        return 0
    fi

    # Verify all tracked files
    local failed=0
    local verified=0
    local file relative

    while IFS= read -r line; do
        [[ -z "$line" ]] && continue

        # Parse line: "checksum  path"
        local stored_checksum="${line%% *}"
        relative="${line#*  }"
        file="$SCRIPT_DIR/$relative"

        if [[ ! -f "$file" ]]; then
            error "MISSING FILE: $relative"
            ((failed++))
            continue
        fi

        local actual_checksum
        actual_checksum=$(_integrity_checksum_file "$file")

        if [[ "$actual_checksum" != "$stored_checksum" ]]; then
            error "MODIFIED: $relative"
            error "  Expected: $stored_checksum"
            error "  Actual:   $actual_checksum"
            ((failed++))
        else
            ((verified++))
            debug "OK: $relative"
        fi
    done < "$INTEGRITY_CHECKSUMS_FILE"

    # Report results
    if [[ $failed -gt 0 ]]; then
        echo ""
        error "INTEGRITY CHECK FAILED"
        error "$failed file(s) failed verification, $verified passed"
        echo ""
        error "This could mean:"
        error "  1. Files were corrupted (disk error)"
        error "  2. Files were modified (intentional or malicious)"
        error "  3. You updated the code but not the checksums"
        echo ""
        error "Options:"
        error "  - Re-clone the repository"
        error "  - Run: git checkout -- . (discard local changes)"
        error "  - Set YOLLAYAH_SKIP_INTEGRITY=1 to bypass (not recommended)"
        echo ""
        return 1
    fi

    success "Verified $verified files"
    return 0
}

# ============================================================================
# Development Utilities
# ============================================================================

# Check if we should generate manifest instead of verify
integrity_should_generate() {
    [[ "${YOLLAYAH_INTEGRITY_GENERATE:-}" == "1" ]]
}

# Update checksums for specific files (after legitimate changes)
integrity_update_file() {
    local file="$1"

    if [[ ! -f "$file" ]]; then
        error "File not found: $file"
        return 1
    fi

    local relative="${file#$SCRIPT_DIR/}"
    local checksum
    checksum=$(_integrity_checksum_file "$file")

    # Remove old entry if exists
    if [[ -f "$INTEGRITY_CHECKSUMS_FILE" ]]; then
        grep -v "  ${relative}$" "$INTEGRITY_CHECKSUMS_FILE" > "${INTEGRITY_CHECKSUMS_FILE}.tmp" || true
        mv "${INTEGRITY_CHECKSUMS_FILE}.tmp" "$INTEGRITY_CHECKSUMS_FILE"
    fi

    # Add new entry
    ensure_dir "$INTEGRITY_CHECKSUMS_DIR"
    echo "$checksum  $relative" >> "$INTEGRITY_CHECKSUMS_FILE"

    # Re-sort
    sort -k2 "$INTEGRITY_CHECKSUMS_FILE" -o "$INTEGRITY_CHECKSUMS_FILE"

    success "Updated checksum for: $relative"
}
