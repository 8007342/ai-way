#!/bin/bash
# ============================================================================
# lib/user/history.sh - Conversation History (PLACEHOLDER)
#
# STATUS: NOT IMPLEMENTED - Privacy review required
#
# This module will handle conversation history storage.
# This is the HIGHEST RISK feature in the user module.
#
# CRITICAL PRIVACY CONCERNS:
#
# 1. CONTENT SENSITIVITY
#    Conversations contain:
#    - Business secrets ("draft an NDA for...")
#    - Personal information ("my SSN is...")
#    - Health information ("I've been feeling...")
#    - Financial data ("my bank account...")
#    - Queries that reveal intent ("how to hide...")
#
#    If this data leaks, the harm is SEVERE and IRREVERSIBLE.
#
#    Reference: agents/dangers/DATA_LEAKS.md
#
# 2. METADATA EXPOSURE
#    Even without content, metadata reveals:
#    - When AJ uses Yollayah (timing patterns)
#    - How often (usage patterns)
#    - What topics (category inference)
#    - Session patterns (work schedule)
#
#    Reference: agents/dangers/AGENT_FINGERPRINTING.md
#
# 3. ENCRYPTION DILEMMA
#    Should we encrypt history?
#
#    Pros:
#    - Protected if machine is stolen
#    - Protected from casual snooping
#
#    Cons:
#    - Key management is HARD
#    - Password = single point of failure
#    - AJ forgets password = data loss
#    - Key in memory = still vulnerable
#    - False sense of security
#
#    Reference: CONSTITUTION.md - Law of Care
#
# 4. RETENTION POLICY
#    How long should history be kept?
#    - Forever? (maximum value, maximum risk)
#    - 30 days? (balance)
#    - Session only? (minimal risk)
#    - Manual delete? (AJ might forget)
#
# 5. SELECTIVE DELETION
#    Can AJ say "forget this conversation"?
#    - Technical challenge: model may have learned
#    - UX challenge: which parts?
#    - Trust challenge: did it really forget?
#
# Constitution References:
# - Law of Care: "First, do no harm" - don't create dossiers
# - Law of Truth: "Be honest" - about what's stored and risks
# - Four Protections: All of them. This is the riskiest feature.
#
# See README.md for the full privacy manifesto.
# See agents/dangers/ for threat analysis.
# ============================================================================

# Prevent double-sourcing
[[ -n "$_YOLLAYAH_USER_HISTORY_LOADED" ]] && return 0
_YOLLAYAH_USER_HISTORY_LOADED=1

# ============================================================================
# History Configuration
# ============================================================================

# TODO: Define history policy after privacy review
#
# Questions to answer:
# 1. Is history opt-in or opt-out?
#    ANSWER: Must be OPT-IN with explicit consent
#
# 2. What is stored?
#    PROPOSED: User message + Yollayah response (minimal)
#    NOT STORED: Routing decisions, model internals, timing
#
# 3. How is it organized?
#    PROPOSED: One file per session, dated
#    CONCERN: Dates reveal usage patterns
#
# 4. Format?
#    PROPOSED: Plain text (searchable, no special tools)
#    ALTERNATIVE: JSON (structured, but needs tooling)

# ============================================================================
# History Operations (STUBS)
# ============================================================================

# Save a conversation turn
history_save_turn() {
    local user_message="$1"
    local assistant_response="$2"

    # TODO: Implement after privacy review
    #
    # Implementation must:
    # 1. Check if history is enabled (consent)
    # 2. Sanitize content (remove obvious PII?)
    # 3. Write to session file
    # 4. NOT log timing metadata
    #
    # CRITICAL: This function may be called every turn.
    # If it fails, it must fail SILENTLY (don't break conversation)

    # Currently: do nothing (most private)
    return 0
}

# Get conversation history for context
history_get_recent() {
    local count="${1:-5}"

    # TODO: Implement after privacy review
    #
    # This is used to provide context to the model.
    # Privacy concern: The model sees this data.
    # If history contains sensitive info from previous sessions,
    # we're feeding it back into the model.

    # Currently: return nothing
    echo ""
}

# Delete current session's history
history_delete_session() {
    # TODO: Implement
    warn "History not yet implemented"
    return 1
}

# Delete all history
history_delete_all() {
    if [[ -d "$USER_HISTORY_DIR" ]]; then
        info "Deleting all conversation history..."
        rm -rf "$USER_HISTORY_DIR"
        success "History deleted"
    else
        info "No history to delete"
    fi
}

# ============================================================================
# Session Management (PLANNED)
# ============================================================================

# TODO: Session concept for history
#
# A session is a continuous conversation.
# Sessions help organize history and enable features like:
# - "Continue yesterday's conversation"
# - "Delete just this conversation"
#
# Privacy concern: Session IDs are identifiers
# They shouldn't be predictable or sequential

# history_start_session() {
#     # Generate session ID
#     # Create session file
# }

# history_end_session() {
#     # Mark session complete
#     # Apply retention policy
# }

# ============================================================================
# Encryption (DEFERRED)
# ============================================================================

# TODO: Decide on encryption after privacy review
#
# Options under consideration:
#
# 1. NO ENCRYPTION
#    - Simplest, no key management
#    - Risk: Physical access = full access
#    - Mitigated by: Non-standard location, OS encryption
#
# 2. PASSWORD-BASED ENCRYPTION
#    - AJ sets password, we derive key
#    - Risk: AJ forgets password = data loss
#    - Risk: Weak passwords
#    - Risk: Password entry fatigue → saved password → vulnerable
#
# 3. HARDWARE KEY (TPM, Secure Enclave)
#    - Best protection, transparent to AJ
#    - Risk: Not available on all hardware
#    - Risk: Tied to this machine (no portability)
#
# 4. EPHEMERAL ENCRYPTION
#    - Key generated per session, never stored
#    - History readable only during session
#    - On restart: history is encrypted garbage
#    - Like session-only but with crash recovery
#
# Current decision: DEFER
# We need more research on what AJ actually wants.

# history_encrypt() { ... }
# history_decrypt() { ... }

# ============================================================================
# Export (PLANNED, HIGH RISK)
# ============================================================================

# TODO: Consider export functionality
#
# Use cases:
# - AJ wants to back up conversations
# - AJ switching machines
# - AJ wants to analyze their usage
#
# Privacy concerns:
# - Export file is an exfiltration target
# - Export could be triggered by malicious prompt
# - Export format could leak metadata
#
# Proposed safeguards:
# - Require explicit terminal command (not conversational)
# - Show preview of what will be exported
# - Require second confirmation
# - Include deletion instructions in export

# history_export() { ... }

# ============================================================================
# Search (PLANNED, MEDIUM RISK)
# ============================================================================

# TODO: Consider search functionality
#
# Use case: "When did I ask about X?"
#
# Privacy concern:
# - Search queries reveal what AJ is looking for
# - Search results show historical data
# - Could be triggered by malicious prompt
#
# Proposed approach:
# - Local-only search (no embedding models)
# - Simple grep-like matching
# - Results shown directly, not to model

# history_search() { ... }
