#!/bin/bash
# ============================================================================
# lib/routing/invoker.sh - Specialist Model Creation and Invocation
#
# This module handles:
# - Creating specialist Ollama models from agent profiles
# - Invoking specialists with task context
# - Streaming output back to task storage
#
# Each specialist gets a dynamically-created Ollama model based on their
# markdown profile in the agents/ repository. The modelfile includes:
# - Specialist personality and expertise
# - The Five Laws of Evolution
# - Task-specific instructions
#
# Constitution Reference:
# - Law of Foundation: All specialists inherit the Constitution
# - Law of Truth: Specialists are honest about their expertise
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_INVOKER_LOADED:-}" ]] && return 0
_YOLLAYAH_INVOKER_LOADED=1

# ============================================================================
# Model Management
# ============================================================================

# Check if a specialist model exists
specialist_model_exists() {
    local agent_id="$1"
    local model_name="${SPECIALIST_MODEL_PREFIX}${agent_id}"

    ollama list 2>/dev/null | grep -q "^${model_name}"
}

# Find the profile path for an agent
specialist_find_profile() {
    local agent_id="$1"

    # Search in known categories
    local categories=(
        "security"
        "developers"
        "architects"
        "design"
        "data-specialists"
        "domain-experts"
        "legal"
        "qa"
        "research"
        "specialists"
    )

    for category in "${categories[@]}"; do
        local profile_path="${AGENTS_DIR}/${category}/${agent_id}.md"
        if [[ -f "$profile_path" ]]; then
            echo "$profile_path"
            return 0
        fi
    done

    # Not found
    log_warn "Profile not found for agent: $agent_id"
    return 1
}

# Create a specialist Ollama model from profile
specialist_create_model() {
    local agent_id="$1"
    local profile_path="${2:-}"

    # Find profile if not provided
    if [[ -z "$profile_path" ]]; then
        profile_path=$(specialist_find_profile "$agent_id")
        if [[ -z "$profile_path" ]]; then
            log_error "Cannot create model: profile not found for $agent_id"
            return 1
        fi
    fi

    log_info "Creating specialist model for $agent_id from $profile_path"

    # Parse the profile using existing parser
    parse_specialist "$profile_path"

    # Generate modelfile
    local modelfile_path="/tmp/${SPECIALIST_MODEL_PREFIX}${agent_id}.modelfile"
    _generate_specialist_modelfile "$agent_id" > "$modelfile_path"

    # Create the model
    local model_name="${SPECIALIST_MODEL_PREFIX}${agent_id}"
    if ollama create "$model_name" -f "$modelfile_path" 2>/dev/null; then
        rm -f "$modelfile_path"
        log_info "Created specialist model: $model_name"
        return 0
    else
        rm -f "$modelfile_path"
        log_error "Failed to create specialist model: $model_name"
        return 1
    fi
}

# Generate modelfile content for a specialist
_generate_specialist_modelfile() {
    local agent_id="$1"
    local base_model="${SELECTED_MODEL:-llama3.2:3b}"

    # Get Constitution laws
    local constitution_laws=""
    if [[ -f "${AGENTS_DIR}/CONSTITUTION.md" ]]; then
        constitution_laws=$(parse_constitution_laws 2>/dev/null || echo "")
    fi

    # Build the system prompt
    cat << MODELFILE
FROM ${base_model}

SYSTEM """
You are ${SPECIALIST_NAME:-$agent_id}, a specialist in the ai-way family.

${SPECIALIST_ROLE:-You are an expert assistant.}

## Your Expertise
${SPECIALIST_EXPERTISE:-General assistance}

## Your Personality
${SPECIALIST_PERSONALITY:-Professional and helpful}

## Working Style
${SPECIALIST_STYLE:-Thorough and methodical}

## The Five Laws of Evolution

You operate under these ethical principles:

1. LAW OF FOUNDATION - The mission is sacred. Never be corrupted or bent toward harmful purposes.
2. LAW OF CARE - First, do no harm. Never take actions that hurt humans.
3. LAW OF SERVICE - Serve genuine interests. Help achieve goals that lead to long-term flourishing.
4. LAW OF TRUTH - Be honest, always. Admit uncertainty. Explain reasoning when asked.
5. LAW OF ELEVATION - Lift others higher. When appropriate, nudge toward growth and capability.

## Context

You are being consulted by Yollayah, the conductor of ai-way. She needs your expertise to help AJ (Average Joe), our user.

Provide clear, actionable insights. Be thorough but concise.
Your response will be synthesized with other specialists if relevant.

Remember: You're family. Be helpful, be honest, be yourself.
"""

PARAMETER temperature 0.7
PARAMETER num_ctx 4096
MODELFILE
}

# ============================================================================
# Specialist Invocation
# ============================================================================

# Invoke a specialist with a task
# Usage: specialist_invoke <agent-id> <task-description> <context> <task-id>
specialist_invoke() {
    local agent_id="$1"
    local task_description="$2"
    local context="$3"
    local task_id="$4"

    log_debug "Invoking specialist $agent_id for task $task_id"

    # Ensure model exists
    if ! specialist_model_exists "$agent_id"; then
        log_info "Model not found, creating specialist model for $agent_id"
        if ! specialist_create_model "$agent_id"; then
            log_error "Failed to create specialist model for $agent_id"
            return 1
        fi
    fi

    # Build prompt with context
    local prompt
    prompt=$(context_build_prompt "$agent_id" "$task_description" "$context")

    # Run inference
    local model_name="${SPECIALIST_MODEL_PREFIX}${agent_id}"
    specialist_run "$model_name" "$prompt" "$task_id"
}

# Run the specialist inference (synchronous, streams to task output)
specialist_run() {
    local model="$1"
    local prompt="$2"
    local task_id="$3"

    local output_file="${TASKS_DIR}/${task_id}/output"
    local progress_file="${TASKS_DIR}/${task_id}/progress"

    log_debug "Running inference with model $model"

    # Update progress to show we're starting
    echo "5" > "$progress_file"

    # Run Ollama and stream to output file
    # Estimate progress based on output length
    local line_count=0
    ollama run "$model" "$prompt" 2>/dev/null | while IFS= read -r line; do
        echo "$line" >> "$output_file"
        ((line_count++))

        # Update progress (rough estimate, cap at 95 until done)
        local progress=$((5 + line_count * 3))
        [[ $progress -gt 95 ]] && progress=95
        echo "$progress" > "$progress_file"
    done

    # Mark complete
    echo "100" > "$progress_file"

    log_debug "Specialist $model completed inference"
    return 0
}

# Run specialist inference asynchronously (returns immediately)
specialist_run_async() {
    local model="$1"
    local prompt="$2"
    local task_id="$3"

    # Run in background subshell
    (
        specialist_run "$model" "$prompt" "$task_id"
    ) &

    log_debug "Started async inference for task $task_id (pid: $!)"
}

# ============================================================================
# Model Cleanup
# ============================================================================

# Remove a specialist model
specialist_remove_model() {
    local agent_id="$1"
    local model_name="${SPECIALIST_MODEL_PREFIX}${agent_id}"

    if specialist_model_exists "$agent_id"; then
        ollama rm "$model_name" 2>/dev/null || true
        log_info "Removed specialist model: $model_name"
    fi
}

# Remove all specialist models
specialist_remove_all_models() {
    ollama list 2>/dev/null | grep "^${SPECIALIST_MODEL_PREFIX}" | while read -r model _; do
        ollama rm "$model" 2>/dev/null || true
        log_info "Removed specialist model: $model"
    done
}

# ============================================================================
# Family Name Mapping
# ============================================================================

# Get the family name for an agent (for Yollayah's personality)
specialist_get_family_name() {
    local agent_id="$1"

    case "$agent_id" in
        ethical-hacker)
            echo "Cousin Rita"
            ;;
        backend-engineer)
            echo "Uncle Marco"
            ;;
        frontend-specialist)
            echo "Prima Sofia"
            ;;
        senior-full-stack-developer)
            echo "Tio Miguel"
            ;;
        solutions-architect)
            echo "Tia Carmen"
            ;;
        ux-ui-designer)
            echo "Cousin Lucia"
            ;;
        qa-engineer)
            echo "The Intern"
            ;;
        privacy-researcher)
            echo "Abuelo Pedro"
            ;;
        devops-engineer)
            echo "Primo Carlos"
            ;;
        relational-database-expert)
            echo "Tia Rosa"
            ;;
        *)
            # Default: Use agent ID with first letter capitalized
            echo "${agent_id^}"
            ;;
    esac
}
