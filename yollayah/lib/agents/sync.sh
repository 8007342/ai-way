#!/bin/bash
# ============================================================================
# lib/agents/sync.sh - Agent Repository Management
#
# This module handles:
# - Cloning the agents repository on first run
# - Pulling updates on subsequent runs
# - Detecting changes to trigger model rebuilds
#
# The agents repository contains:
# - Agent profiles (personality, expertise, working style)
# - The Constitution (Five Laws of Evolution)
# - Documentation including the YOU.md easter egg
#
# Constitution Reference:
# - Law of Elevation: "Lift others higher" - agents help AJ grow
# - Law of Foundation: "The mission is sacred" - Constitution is embedded
#
# Privacy Note:
# The agents/ directory is the ONE trace Yollayah leaves behind.
# This is intentional - it's how the curious discover ai-way-docs/YOU.md
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_AGENTS_LOADED:-}" ]] && return 0
_YOLLAYAH_AGENTS_LOADED=1

# Load parser submodule
source "${LIB_DIR}/agents/parser.sh"

# ============================================================================
# Agent Repository Sync
# ============================================================================

# Sync the agents repository (clone or update)
# Sets AGENTS_CHANGED=true if there were updates
agents_sync() {
    log_agents "INFO" "Starting agents sync"
    pj_step "Syncing agents repository"
    pj_check "$AGENTS_DIR"

    if [[ -d "$AGENTS_DIR" ]]; then
        pj_found "Agents repo"
        ux_success "Agents repo found"
        _agents_update
    else
        pj_missing "Agents repo (will clone)"
        _agents_clone
    fi
}

# Clone the agents repository (first run)
_agents_clone() {
    log_agents "INFO" "Cloning agents repo from $AGENTS_REPO"
    pj_step "Cloning agents repository"
    ux_info "Cloning agents repo..."

    pj_check "git"
    if ! command_exists git; then
        log_agents "ERROR" "Git not installed"
        pj_missing "git"
        ux_error "Git is required but not installed"
        return 1
    fi
    pj_found "git at $(command -v git)"

    pj_cmd "git clone $AGENTS_REPO $AGENTS_DIR (30s timeout)"
    if timeout 30 git clone --quiet "$AGENTS_REPO" "$AGENTS_DIR"; then
        AGENTS_CHANGED=true
        log_agents "INFO" "Agents repo cloned successfully"
        pj_result "Clone successful"
        ux_success "Agents repo cloned"

        # Hint about the easter egg (subtle)
        log_agents "DEBUG" "Agents include ai-way-docs/ with documentation"
    else
        log_agents "ERROR" "Failed to clone agents repository"
        pj_result "Clone failed"
        ux_error "Failed to clone agents repository"
        return 1
    fi
}

# Update the agents repository (subsequent runs)
_agents_update() {
    pj_step "Checking for agent updates"

    # Only update if it's a git repo
    pj_check "$AGENTS_DIR/.git"
    if [[ ! -d "$AGENTS_DIR/.git" ]]; then
        log_agents "WARN" "Agents directory exists but is not a git repo"
        pj_result "Not a git repo, skipping update"
        ux_warn "Agents directory exists but is not a git repo"
        return 0
    fi

    log_agents "INFO" "Checking for agent updates"
    ux_info "Checking for agent updates..."

    local before_hash after_hash
    before_hash=$(get_git_hash "$AGENTS_DIR")
    pj_result "Current: ${before_hash:0:7}"

    # Pull quietly with timeout, don't fail if offline
    pj_cmd "git pull (in $AGENTS_DIR, 15s timeout)"
    if (cd "$AGENTS_DIR" && timeout 15 git pull --quiet 2>/dev/null); then
        after_hash=$(get_git_hash "$AGENTS_DIR")

        if [[ "$before_hash" != "$after_hash" ]]; then
            AGENTS_CHANGED=true
            log_agents "INFO" "Agents updated: ${before_hash:0:7} -> ${after_hash:0:7}"
            pj_result "Updated: ${before_hash:0:7} → ${after_hash:0:7}"
            ux_success "Agents updated! (${before_hash:0:7} -> ${after_hash:0:7})"
        else
            log_agents "DEBUG" "Agents already up to date"
            pj_result "Already up to date"
        fi
    else
        log_agents "WARN" "Could not update agents (network issue?)"
        pj_result "Pull failed (network issue?)"
        ux_warn "Could not update agents (offline?)"
    fi
}

# Check if Constitution exists (sanity check)
agents_has_constitution() {
    [[ -f "$AGENTS_DIR/CONSTITUTION.md" ]]
}

# Get path to Constitution
agents_constitution_path() {
    echo "$AGENTS_DIR/CONSTITUTION.md"
}

# ============================================================================
# Future: Agent Profile Parsing
# ============================================================================

# TODO: Parse agent profiles for full ai-way runtime
# For now, Yollayah-lite uses a hardcoded personality.
# When ai-way-full is ready, this module will:
# - Parse markdown agent profiles
# - Extract personality, expertise, working style
# - Feed into modelfile generation
# - Support the full 19-agent routing system

# agents_list() {
#     # List all available agent profiles
# }

# agents_parse_profile() {
#     # Parse a single agent markdown file into structured data
# }

# agents_get_conductor_config() {
#     # Get Yollayah's conductor configuration
# }
