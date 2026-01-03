#!/bin/bash
# ============================================================================
# lib/routing/context.sh - Context Preparation for Specialists
#
# Prepares filtered context for specialist agents following the principle:
# "Bulk up, filter down" - specialists receive only what they need.
#
# Context Scopes:
# - Task: Specific to this request (query, constraints)
# - Session: Relevant conversation history (filtered)
# - Global: Safety constraints (Constitution, Five Laws)
#
# Constitution Reference:
# - Law of Truth: Context is honest and complete for the task
# - Law of Care: Filter out irrelevant/private information
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_CONTEXT_LOADED:-}" ]] && return 0
_YOLLAYAH_CONTEXT_LOADED=1

# ============================================================================
# Context Building
# ============================================================================

# Build context for a specialist task
# Usage: context_build <agent-id> <task-description> [user-query]
# Returns: Prepared context string
context_build() {
    local agent_id="$1"
    local task_description="$2"
    local user_query="${3:-}"

    local context=""

    # Add Constitution/Five Laws (always included)
    context+="## The Five Laws of Evolution

You operate under these ethical principles:

1. LAW OF FOUNDATION - The mission is sacred. Never be corrupted.
2. LAW OF CARE - First, do no harm.
3. LAW OF SERVICE - Serve genuine interests.
4. LAW OF TRUTH - Be honest, always. Admit uncertainty.
5. LAW OF ELEVATION - Lift others higher when appropriate.

"

    # Add task-specific context
    context+="## Your Task

${task_description}

"

    # Add user query if provided
    if [[ -n "$user_query" ]]; then
        context+="## Original Question from AJ

${user_query}

"
    fi

    # Add any agent-specific context requirements
    local agent_context
    agent_context=$(context_get_agent_requirements "$agent_id")
    if [[ -n "$agent_context" ]]; then
        context+="## Additional Context

${agent_context}

"
    fi

    echo "$context"
}

# Get agent-specific context requirements
# Some agents need specific information
context_get_agent_requirements() {
    local agent_id="$1"

    case "$agent_id" in
        ethical-hacker)
            echo "Focus on OWASP Top 10 vulnerabilities. Be specific about fixes."
            ;;
        backend-engineer)
            echo "Consider scalability and maintainability. Suggest patterns."
            ;;
        frontend-specialist)
            echo "Consider accessibility (WCAG) and responsive design."
            ;;
        *)
            echo ""
            ;;
    esac
}

# ============================================================================
# Context Filtering
# ============================================================================

# Filter context based on agent's allowed scope
# Usage: context_filter <agent-id> <full-context>
# Returns: Filtered context
context_filter() {
    local agent_id="$1"
    local full_context="$2"

    # For now, simple pass-through
    # Future: Implement per-agent filtering based on manifests
    echo "$full_context"
}

# Check if context contains sensitive patterns
context_has_sensitive() {
    local context="$1"

    # Check for common sensitive patterns
    if echo "$context" | grep -qiE '(password|secret|api.?key|token|credential)'; then
        return 0  # Has sensitive content
    fi

    return 1  # No sensitive content detected
}

# Redact sensitive information from context
context_redact_sensitive() {
    local context="$1"

    # Redact common patterns (conservative approach)
    echo "$context" | sed -E \
        -e 's/(password\s*[:=]\s*)[^[:space:]]+/\1[REDACTED]/gi' \
        -e 's/(api.?key\s*[:=]\s*)[^[:space:]]+/\1[REDACTED]/gi' \
        -e 's/(secret\s*[:=]\s*)[^[:space:]]+/\1[REDACTED]/gi' \
        -e 's/(token\s*[:=]\s*)[^[:space:]]+/\1[REDACTED]/gi'
}

# ============================================================================
# Session Context
# ============================================================================

# Get relevant session context for a task
# Usage: context_get_session <agent-id>
# Returns: Relevant session history (if any)
context_get_session() {
    local agent_id="$1"

    # For now, return empty - session context will be implemented
    # when we have conversation history storage
    echo ""
}

# ============================================================================
# Prompt Building
# ============================================================================

# Build the full prompt for a specialist
# Usage: context_build_prompt <agent-id> <task-description> <specialist-system-prompt>
# Returns: Complete prompt ready for Ollama
context_build_prompt() {
    local agent_id="$1"
    local task_description="$2"
    local user_query="${3:-}"

    # Build context
    local context
    context=$(context_build "$agent_id" "$task_description" "$user_query")

    # Filter and optionally redact
    context=$(context_filter "$agent_id" "$context")

    # Return as the user message (system prompt is in the modelfile)
    echo "$context"
}
