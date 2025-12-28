#!/bin/bash
# ============================================================================
# lib/user/init.sh - User Customizations Module Entry Point
#
# This module initializes user customization features.
# It is the gateway that ensures privacy principles are followed.
#
# IMPORTANT: This module does NOT create any directories or files
# until the user explicitly opts in to a feature.
#
# Constitution Reference:
# - Four Protections: "Protect AJ from ai-way"
# - Law of Care: "First, do no harm"
# - Law of Truth: "Be honest about what we store"
#
# See README.md in this directory for the full privacy manifesto.
# ============================================================================

# Prevent double-sourcing
[[ -n "$_YOLLAYAH_USER_LOADED" ]] && return 0
_YOLLAYAH_USER_LOADED=1

# ============================================================================
# Module Loading
# ============================================================================

# Source submodules
# Note: These are placeholders that do minimal work until features are ready
source "${LIB_DIR}/user/storage.sh"
source "${LIB_DIR}/user/preferences.sh"
source "${LIB_DIR}/user/history.sh"

# ============================================================================
# Initialization
# ============================================================================

# Initialize user module
# This is called during startup but does NOT create any files
user_init() {
    debug "User customization module loaded"

    # Check if user has opted into any features
    if user_has_any_data; then
        debug "User data directory exists"
        user_load_settings
    else
        debug "No user data (privacy-first default)"
    fi
}

# Check if user has any stored data
user_has_any_data() {
    [[ -d "$USER_DIR" ]]
}

# Load user settings if they exist
user_load_settings() {
    # TODO: Implement when preferences are ready
    # For now, this is a no-op
    :
}

# ============================================================================
# Consent Management
# ============================================================================

# TODO: Implement consent dialogs for each feature
#
# Each feature (preferences, history, memory) requires explicit consent.
# We don't bundle consent - each is separate.
#
# The consent flow should:
# 1. Explain what will be stored
# 2. Explain where it will be stored
# 3. Explain how to delete it
# 4. Require explicit "yes" (not just Enter)
#
# Example:
# user_request_consent "conversation_history" \
#     "Store conversation history locally" \
#     "Conversations will be saved in $USER_DIR/history/" \
#     "Delete anytime with: rm -rf $USER_DIR/history/"

user_request_consent() {
    local feature="$1"
    local title="$2"
    local what="$3"
    local how_to_delete="$4"

    # TODO: Implement consent dialog
    # For now, always return false (no consent)
    warn "Consent management not yet implemented"
    return 1
}

# Check if user has consented to a feature
user_has_consent() {
    local feature="$1"
    # TODO: Check stored consent
    # For now, always return false
    return 1
}

# ============================================================================
# Data Management
# ============================================================================

# Delete all user data
user_delete_all() {
    if [[ -d "$USER_DIR" ]]; then
        info "Deleting all user data..."
        rm -rf "$USER_DIR"
        success "All user data deleted"
    else
        info "No user data to delete"
    fi
}

# Show what data exists
user_show_data() {
    if [[ -d "$USER_DIR" ]]; then
        echo -e "${CYAN}User data location:${NC} $USER_DIR"
        echo -e "${CYAN}Contents:${NC}"
        ls -la "$USER_DIR" 2>/dev/null || echo "  (empty)"
    else
        echo -e "${CYAN}No user data stored${NC}"
        echo "Yollayah is running in privacy-maximum mode."
    fi
}
