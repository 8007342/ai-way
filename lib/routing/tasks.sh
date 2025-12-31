#!/bin/bash
# ============================================================================
# lib/routing/tasks.sh - Task Lifecycle Management
#
# Manages background specialist tasks:
# - Create tasks with unique IDs
# - Track status (pending, running, done, failed)
# - Monitor progress (0-100%)
# - Store outputs for aggregation
#
# Task State Structure:
#   .state/tasks/task-{timestamp}-{pid}-{rand}/
#       id              # Task UUID
#       agent           # Agent ID (e.g., "ethical-hacker")
#       description     # What the task is doing
#       status          # pending|running|done|failed
#       progress        # 0-100
#       started_at      # ISO timestamp
#       completed_at    # ISO timestamp (when done)
#       output          # Raw specialist output
#       error           # Error message if failed
#
# Constitution Reference:
# - Law of Truth: Accurate status reporting
# - Law of Care: Clean up resources, don't leak state
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_TASKS_LOADED:-}" ]] && return 0
_YOLLAYAH_TASKS_LOADED=1

# ============================================================================
# Task Creation
# ============================================================================

# Create a new task
# Usage: task_create <agent-id> <description> [context]
# Returns: task-id
task_create() {
    local agent_id="$1"
    local description="$2"
    local context="${3:-}"

    # Generate unique task ID
    local task_id="task-$(date +%s)-$$-$RANDOM"
    local task_dir="${TASKS_DIR}/${task_id}"

    # Create task directory
    mkdir -p "$task_dir"

    # Write task metadata
    echo "$task_id" > "${task_dir}/id"
    echo "$agent_id" > "${task_dir}/agent"
    echo "$description" > "${task_dir}/description"
    echo "pending" > "${task_dir}/status"
    echo "0" > "${task_dir}/progress"
    date -Iseconds > "${task_dir}/started_at"

    # Store context if provided
    if [[ -n "$context" ]]; then
        echo "$context" > "${task_dir}/context"
    fi

    # Initialize empty output
    touch "${task_dir}/output"

    log_debug "Created task $task_id for agent $agent_id"
    echo "$task_id"
}

# ============================================================================
# Task Execution
# ============================================================================

# Start a task (launches specialist in background)
# Usage: task_start <task-id>
task_start() {
    local task_id="$1"
    local task_dir="${TASKS_DIR}/${task_id}"

    if [[ ! -d "$task_dir" ]]; then
        log_error "Task not found: $task_id"
        return 1
    fi

    # Update status
    echo "running" > "${task_dir}/status"

    # Read task details
    local agent_id description context
    agent_id=$(cat "${task_dir}/agent")
    description=$(cat "${task_dir}/description")
    context=$(cat "${task_dir}/context" 2>/dev/null || echo "")

    log_info "Starting task $task_id with agent $agent_id"

    # Invoke the specialist (this runs the actual inference)
    specialist_invoke "$agent_id" "$description" "$context" "$task_id"
    local exit_code=$?

    if [[ $exit_code -eq 0 ]]; then
        echo "done" > "${task_dir}/status"
        echo "100" > "${task_dir}/progress"
        date -Iseconds > "${task_dir}/completed_at"
        log_info "Task $task_id completed successfully"
    else
        echo "failed" > "${task_dir}/status"
        echo "Specialist invocation failed with code $exit_code" > "${task_dir}/error"
        date -Iseconds > "${task_dir}/completed_at"
        log_error "Task $task_id failed"
    fi

    return $exit_code
}

# ============================================================================
# Task Status & Progress
# ============================================================================

# Get task status
# Returns: pending|running|done|failed|unknown
task_get_status() {
    local task_id="$1"
    local status_file="${TASKS_DIR}/${task_id}/status"

    if [[ -f "$status_file" ]]; then
        cat "$status_file"
    else
        echo "unknown"
    fi
}

# Get task progress (0-100)
task_get_progress() {
    local task_id="$1"
    local progress_file="${TASKS_DIR}/${task_id}/progress"

    if [[ -f "$progress_file" ]]; then
        cat "$progress_file"
    else
        echo "0"
    fi
}

# Update task progress
task_set_progress() {
    local task_id="$1"
    local progress="$2"

    # Clamp to 0-100
    [[ $progress -lt 0 ]] && progress=0
    [[ $progress -gt 100 ]] && progress=100

    echo "$progress" > "${TASKS_DIR}/${task_id}/progress"
}

# ============================================================================
# Task Output
# ============================================================================

# Get task output
task_get_output() {
    local task_id="$1"
    local output_file="${TASKS_DIR}/${task_id}/output"

    if [[ -f "$output_file" ]]; then
        cat "$output_file"
    else
        echo ""
    fi
}

# Append to task output (used during streaming)
task_append_output() {
    local task_id="$1"
    local content="$2"

    echo "$content" >> "${TASKS_DIR}/${task_id}/output"
}

# Get task error message
task_get_error() {
    local task_id="$1"
    local error_file="${TASKS_DIR}/${task_id}/error"

    if [[ -f "$error_file" ]]; then
        cat "$error_file"
    else
        echo ""
    fi
}

# ============================================================================
# Task Metadata
# ============================================================================

# Get agent ID for a task
task_get_agent() {
    local task_id="$1"
    cat "${TASKS_DIR}/${task_id}/agent" 2>/dev/null || echo ""
}

# Get task description
task_get_description() {
    local task_id="$1"
    cat "${TASKS_DIR}/${task_id}/description" 2>/dev/null || echo ""
}

# Get when task started
task_get_started_at() {
    local task_id="$1"
    cat "${TASKS_DIR}/${task_id}/started_at" 2>/dev/null || echo ""
}

# Get when task completed
task_get_completed_at() {
    local task_id="$1"
    cat "${TASKS_DIR}/${task_id}/completed_at" 2>/dev/null || echo ""
}

# ============================================================================
# Task Listing
# ============================================================================

# List all active tasks (pending or running)
task_list_active() {
    for task_dir in "${TASKS_DIR}"/task-*; do
        [[ -d "$task_dir" ]] || continue

        local status
        status=$(cat "${task_dir}/status" 2>/dev/null)

        if [[ "$status" == "running" || "$status" == "pending" ]]; then
            basename "$task_dir"
        fi
    done
}

# List all tasks (any status)
task_list_all() {
    for task_dir in "${TASKS_DIR}"/task-*; do
        [[ -d "$task_dir" ]] || continue
        basename "$task_dir"
    done
}

# Count active tasks
task_count_active() {
    task_list_active | wc -l
}

# ============================================================================
# Task Cleanup
# ============================================================================

# Remove old completed tasks (older than 1 hour by default)
task_cleanup_old() {
    local max_age_minutes="${1:-60}"

    find "${TASKS_DIR}" -maxdepth 1 -type d -name "task-*" -mmin "+${max_age_minutes}" -exec rm -rf {} \; 2>/dev/null || true

    log_debug "Cleaned up tasks older than ${max_age_minutes} minutes"
}

# Remove a specific task
task_remove() {
    local task_id="$1"
    local task_dir="${TASKS_DIR}/${task_id}"

    if [[ -d "$task_dir" ]]; then
        rm -rf "$task_dir"
        log_debug "Removed task $task_id"
    fi
}

# ============================================================================
# Task Info (for TUI display)
# ============================================================================

# Get task info as JSON-like structure for TUI
task_get_info() {
    local task_id="$1"
    local task_dir="${TASKS_DIR}/${task_id}"

    if [[ ! -d "$task_dir" ]]; then
        echo "{}"
        return 1
    fi

    local agent status progress description
    agent=$(cat "${task_dir}/agent" 2>/dev/null || echo "")
    status=$(cat "${task_dir}/status" 2>/dev/null || echo "unknown")
    progress=$(cat "${task_dir}/progress" 2>/dev/null || echo "0")
    description=$(cat "${task_dir}/description" 2>/dev/null || echo "")

    # Simple key-value format (not JSON, just readable)
    cat << EOF
id=$task_id
agent=$agent
status=$status
progress=$progress
description=$description
EOF
}

# Get all active tasks info (for TUI batch read)
task_get_all_active_info() {
    for task_id in $(task_list_active); do
        task_get_info "$task_id"
        echo "---"
    done
}
