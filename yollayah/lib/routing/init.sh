#!/bin/bash
# ============================================================================
# lib/routing/init.sh - Routing Module Entry Point
#
# This module enables Yollayah to route queries to specialist agents (la familia).
#
# Key Components:
# - classifier.sh  : Parse routing commands from Yollayah's output
# - invoker.sh     : Create and run specialist Ollama models
# - context.sh     : Prepare context for specialists
# - tasks.sh       : Task lifecycle management
# - aggregator.sh  : Collect and synthesize results
# - avatar_agent.sh: Autonomous avatar behavior during tasks
#
# Architecture:
#   AJ Query -> Yollayah -> [yolla:task] commands -> Specialists -> Aggregation -> Response
#
# Constitution Reference:
# - Law of Service: Route to the best specialist for AJ's needs
# - Law of Truth: Be honest about what each specialist provides
# - Law of Elevation: Help AJ learn through expert perspectives
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_ROUTING_LOADED:-}" ]] && return 0
_YOLLAYAH_ROUTING_LOADED=1

# ============================================================================
# Configuration
# ============================================================================

# Tasks state directory
readonly TASKS_DIR="${STATE_DIR}/tasks"

# Avatar command FIFO for async behavior
readonly AVATAR_FIFO="${STATE_DIR}/avatar_commands.fifo"

# Specialist model prefix
readonly SPECIALIST_MODEL_PREFIX="specialist-"

# Maximum concurrent tasks (can be overridden, but design for unbounded)
ROUTING_MAX_CONCURRENT_TASKS="${ROUTING_MAX_CONCURRENT_TASKS:-10}"

# ============================================================================
# Load Submodules
# ============================================================================

source "${LIB_DIR}/routing/tasks.sh"
source "${LIB_DIR}/routing/context.sh"
source "${LIB_DIR}/routing/invoker.sh"
source "${LIB_DIR}/routing/classifier.sh"
source "${LIB_DIR}/routing/aggregator.sh"
source "${LIB_DIR}/routing/avatar_agent.sh"

# ============================================================================
# Initialization
# ============================================================================

routing_init() {
    log_info "Initializing routing module"

    # Ensure tasks directory exists
    ensure_dir "$TASKS_DIR"

    # Create avatar FIFO if needed
    if [[ ! -p "$AVATAR_FIFO" ]]; then
        mkfifo "$AVATAR_FIFO" 2>/dev/null || true
    fi

    # Clean up old tasks from previous sessions
    task_cleanup_old

    log_info "Routing module initialized"
}

# ============================================================================
# Main Routing Entry Point
# ============================================================================

# Process a query through the routing system
# Usage: routing_process_query "user query" "yollayah response"
# Returns: Final synthesized response
routing_process_query() {
    local user_query="$1"
    local yollayah_response="$2"

    log_debug "Processing query through routing system"

    # Extract task commands from Yollayah's response
    local task_commands
    task_commands=$(classifier_extract_tasks "$yollayah_response")

    if [[ -z "$task_commands" ]]; then
        # No routing needed, return original response
        log_debug "No routing commands found, returning direct response"
        echo "$yollayah_response"
        return 0
    fi

    log_info "Found routing commands, spawning specialists"

    # Parse and spawn tasks
    local task_ids=()
    while IFS= read -r cmd; do
        [[ -z "$cmd" ]] && continue

        local agent_id description
        read -r agent_id description <<< "$(classifier_parse_task "$cmd")"

        if [[ -n "$agent_id" ]]; then
            local task_id
            task_id=$(task_create "$agent_id" "$description")
            task_start "$task_id" &
            task_ids+=("$task_id")
            log_info "Started task $task_id for $agent_id"
        fi
    done <<< "$task_commands"

    # Start avatar behavior loop if we have tasks
    if [[ ${#task_ids[@]} -gt 0 ]]; then
        avatar_behavior_start &
        local avatar_pid=$!
    fi

    # Wait for all tasks to complete
    routing_wait_for_tasks "${task_ids[@]}"

    # Stop avatar behavior
    if [[ -n "${avatar_pid:-}" ]]; then
        kill "$avatar_pid" 2>/dev/null || true
    fi

    # Aggregate results
    local results
    results=$(aggregator_collect_results "${task_ids[@]}")

    # Synthesize final response
    local final_response
    final_response=$(aggregator_synthesize "$user_query" "$results" "$yollayah_response")

    echo "$final_response"
}

# Wait for all tasks to complete with timeout
routing_wait_for_tasks() {
    local task_ids=("$@")
    local timeout=300  # 5 minutes max
    local start_time
    start_time=$(date +%s)

    while true; do
        local all_done=true

        for task_id in "${task_ids[@]}"; do
            local status
            status=$(task_get_status "$task_id")
            if [[ "$status" == "running" || "$status" == "pending" ]]; then
                all_done=false
                break
            fi
        done

        if [[ "$all_done" == "true" ]]; then
            log_debug "All tasks completed"
            return 0
        fi

        # Check timeout
        local elapsed=$(($(date +%s) - start_time))
        if [[ $elapsed -gt $timeout ]]; then
            log_warn "Task timeout reached after ${elapsed}s"
            return 1
        fi

        sleep 0.5
    done
}

# ============================================================================
# Specialist Discovery
# ============================================================================

# Get list of available specialists with their family names
routing_list_specialists() {
    # Use parser from agents module
    agents_list_specialists | while read -r profile_path; do
        [[ -z "$profile_path" ]] && continue
        local agent_id
        agent_id=$(basename "$profile_path" .md)
        echo "$agent_id"
    done
}

# Check if a specialist is available
routing_specialist_exists() {
    local agent_id="$1"
    local profile_path
    profile_path=$(specialist_find_profile "$agent_id")
    [[ -f "$profile_path" ]]
}
