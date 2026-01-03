#!/bin/bash
# ============================================================================
# lib/integrity/level-0-git.sh - Git Pull Verification (PARANOID MODE)
#
# Level 0: The most paranoid integrity level
#
# What it does:
# - Fetches latest from origin on every startup
# - Compares local files to remote
# - Optionally resets to match remote exactly
#
# Trade-offs:
# - SLOW: Requires network on every startup
# - EXPENSIVE: Git fetch/pull operations
# - INFLEXIBLE: Can't run offline
# - DESTRUCTIVE: May overwrite local modifications
#
# When to use:
# - You don't trust your local filesystem
# - You want guaranteed fresh code every run
# - You have fast, reliable internet
# - You're not making local modifications
#
# Constitution Reference:
# - Law of Care: Maximum protection at cost of convenience
# - Law of Truth: Honest about the performance cost
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_INTEGRITY_GIT_LOADED:-}" ]] && return 0
_YOLLAYAH_INTEGRITY_GIT_LOADED=1

# ============================================================================
# Configuration
# ============================================================================

# Remote to verify against (default: origin)
readonly INTEGRITY_GIT_REMOTE="${INTEGRITY_GIT_REMOTE:-origin}"

# Branch to verify against (default: main)
readonly INTEGRITY_GIT_BRANCH="${INTEGRITY_GIT_BRANCH:-main}"

# Whether to auto-reset on mismatch (default: prompt)
# Values: "auto", "prompt", "warn"
readonly INTEGRITY_GIT_ACTION="${INTEGRITY_GIT_ACTION:-prompt}"

# ============================================================================
# Git Verification Functions
# ============================================================================

# Check if we're in a git repository
_integrity_git_is_repo() {
    git -C "$SCRIPT_DIR" rev-parse --is-inside-work-tree &>/dev/null
}

# Fetch latest from remote
_integrity_git_fetch() {
    info "Fetching latest from $INTEGRITY_GIT_REMOTE..."

    if ! git -C "$SCRIPT_DIR" fetch "$INTEGRITY_GIT_REMOTE" "$INTEGRITY_GIT_BRANCH" --quiet 2>/dev/null; then
        warn "Failed to fetch from remote (offline?)"
        return 1
    fi

    return 0
}

# Get local HEAD hash
_integrity_git_local_hash() {
    git -C "$SCRIPT_DIR" rev-parse HEAD 2>/dev/null
}

# Get remote HEAD hash
_integrity_git_remote_hash() {
    git -C "$SCRIPT_DIR" rev-parse "$INTEGRITY_GIT_REMOTE/$INTEGRITY_GIT_BRANCH" 2>/dev/null
}

# Check if there are local modifications
_integrity_git_has_local_changes() {
    ! git -C "$SCRIPT_DIR" diff --quiet HEAD 2>/dev/null
}

# Check if local is behind remote
_integrity_git_is_behind() {
    local local_hash remote_hash
    local_hash=$(_integrity_git_local_hash)
    remote_hash=$(_integrity_git_remote_hash)

    [[ "$local_hash" != "$remote_hash" ]]
}

# Get number of commits behind
_integrity_git_commits_behind() {
    git -C "$SCRIPT_DIR" rev-list --count HEAD.."$INTEGRITY_GIT_REMOTE/$INTEGRITY_GIT_BRANCH" 2>/dev/null || echo "?"
}

# ============================================================================
# Reset Functions
# ============================================================================

# Hard reset to remote (destructive!)
_integrity_git_reset_hard() {
    warn "Resetting to $INTEGRITY_GIT_REMOTE/$INTEGRITY_GIT_BRANCH..."

    # Stash any local changes first (just in case)
    git -C "$SCRIPT_DIR" stash --quiet 2>/dev/null || true

    # Hard reset
    if git -C "$SCRIPT_DIR" reset --hard "$INTEGRITY_GIT_REMOTE/$INTEGRITY_GIT_BRANCH" --quiet 2>/dev/null; then
        success "Reset complete"
        return 0
    else
        error "Failed to reset"
        return 1
    fi
}

# Prompt user about reset
_integrity_git_prompt_reset() {
    local commits_behind
    commits_behind=$(_integrity_git_commits_behind)

    echo ""
    warn "Local code differs from remote ($commits_behind commits behind)"
    echo ""
    echo "Options:"
    echo "  [r] Reset to remote (recommended - ensures clean code)"
    echo "  [c] Continue anyway (risk: may run modified code)"
    echo "  [q] Quit"
    echo ""

    read -p "Your choice [r/c/q]: " -n 1 -r choice
    echo ""

    case "$choice" in
        r|R)
            _integrity_git_reset_hard
            return $?
            ;;
        c|C)
            warn "Continuing with potentially modified code"
            warn "Set YOLLAYAH_INTEGRITY_LEVEL=1 to use checksum verification instead"
            return 0
            ;;
        q|Q|*)
            info "Exiting"
            return 1
            ;;
    esac
}

# ============================================================================
# Main Verification Function
# ============================================================================

# Run Level 0 verification
integrity_verify_git() {
    info "Level 0: Git-based verification (paranoid mode)"

    # Check if we're in a git repo
    if ! _integrity_git_is_repo; then
        error "Not a git repository: $SCRIPT_DIR"
        error "Level 0 requires running from a git clone"
        error "Use YOLLAYAH_INTEGRITY_LEVEL=1 for checksum verification instead"
        return 1
    fi

    # Fetch from remote
    if ! _integrity_git_fetch; then
        warn "Cannot verify against remote (network unavailable)"
        warn "Falling back to local-only checks"

        # Check for local modifications at least
        if _integrity_git_has_local_changes; then
            warn "Local modifications detected"
            warn "Cannot verify integrity without network"

            if [[ "$INTEGRITY_GIT_ACTION" == "auto" ]]; then
                error "Auto-reset requested but network unavailable"
                return 1
            fi
        fi

        return 0
    fi

    # Check if we're in sync with remote
    if ! _integrity_git_is_behind; then
        # Check for uncommitted local changes
        if _integrity_git_has_local_changes; then
            warn "Uncommitted local modifications detected"

            case "$INTEGRITY_GIT_ACTION" in
                auto)
                    _integrity_git_reset_hard || return 1
                    ;;
                prompt)
                    _integrity_git_prompt_reset || return 1
                    ;;
                warn)
                    warn "Continuing with local modifications"
                    ;;
            esac
        else
            success "Code verified: matches $INTEGRITY_GIT_REMOTE/$INTEGRITY_GIT_BRANCH"
        fi
        return 0
    fi

    # We're behind remote
    local commits_behind
    commits_behind=$(_integrity_git_commits_behind)
    warn "Local code is $commits_behind commits behind remote"

    case "$INTEGRITY_GIT_ACTION" in
        auto)
            _integrity_git_reset_hard || return 1
            ;;
        prompt)
            _integrity_git_prompt_reset || return 1
            ;;
        warn)
            warn "Continuing with outdated code"
            warn "Run 'git pull' to update"
            ;;
    esac

    return 0
}
