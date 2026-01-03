#!/bin/bash
# ============================================================================
# lib/routing/classifier.sh - Routing Command Parser
#
# Parses Yollayah's output for routing commands:
#   [yolla:task start <agent-id> "<description>"]
#   [yolla:task progress <task-id> <percent>]
#   [yolla:task done <task-id>]
#   [yolla:task fail <task-id> "<reason>"]
#
# The classifier extracts these commands from the response text and
# returns structured data for the task manager to process.
#
# Constitution Reference:
# - Law of Truth: Accurate parsing of Yollayah's routing decisions
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_CLASSIFIER_LOADED:-}" ]] && return 0
_YOLLAYAH_CLASSIFIER_LOADED=1

# ============================================================================
# Command Extraction
# ============================================================================

# Extract all task commands from a response
# Usage: classifier_extract_tasks "response text"
# Returns: One command per line
classifier_extract_tasks() {
    local response="$1"

    # Match [yolla:task ...] patterns
    # Captures: [yolla:task start agent-id "description"]
    echo "$response" | grep -oE '\[yolla:task[^\]]+\]' || true
}

# Extract all routing commands (broader category)
# Usage: classifier_extract_routing "response text"
# Returns: One command per line
classifier_extract_routing() {
    local response="$1"

    # Match any [yolla:...] patterns related to routing
    echo "$response" | grep -oE '\[yolla:(task|route)[^\]]+\]' || true
}

# ============================================================================
# Command Parsing
# ============================================================================

# Parse a task start command
# Usage: classifier_parse_task "[yolla:task start ethical-hacker \"Check for SQL injection\"]"
# Returns: "agent-id description" (space-separated)
classifier_parse_task() {
    local command="$1"

    # Extract agent ID and description
    # Pattern: [yolla:task start <agent-id> "<description>"]
    local agent_id description

    # Remove brackets and prefix
    local inner="${command#\[yolla:task }"
    inner="${inner%\]}"

    # Check for 'start' subcommand
    if [[ "$inner" =~ ^start[[:space:]] ]]; then
        inner="${inner#start }"

        # Extract agent-id (first word)
        agent_id="${inner%% *}"

        # Extract description (quoted string)
        if [[ "$inner" =~ \"([^\"]+)\" ]]; then
            description="${BASH_REMATCH[1]}"
        else
            # Fallback: everything after agent-id
            description="${inner#* }"
            description="${description#\"}"
            description="${description%\"}"
        fi

        echo "$agent_id $description"
        return 0
    fi

    # Not a start command
    return 1
}

# Parse a task progress command
# Usage: classifier_parse_progress "[yolla:task progress task-123 75]"
# Returns: "task-id percent"
classifier_parse_progress() {
    local command="$1"

    local inner="${command#\[yolla:task }"
    inner="${inner%\]}"

    if [[ "$inner" =~ ^progress[[:space:]] ]]; then
        inner="${inner#progress }"
        local task_id="${inner%% *}"
        local percent="${inner##* }"
        echo "$task_id $percent"
        return 0
    fi

    return 1
}

# Parse a task done command
# Usage: classifier_parse_done "[yolla:task done task-123]"
# Returns: "task-id"
classifier_parse_done() {
    local command="$1"

    local inner="${command#\[yolla:task }"
    inner="${inner%\]}"

    if [[ "$inner" =~ ^done[[:space:]] ]]; then
        local task_id="${inner#done }"
        echo "$task_id"
        return 0
    fi

    return 1
}

# Parse a task fail command
# Usage: classifier_parse_fail "[yolla:task fail task-123 \"reason\"]"
# Returns: "task-id reason"
classifier_parse_fail() {
    local command="$1"

    local inner="${command#\[yolla:task }"
    inner="${inner%\]}"

    if [[ "$inner" =~ ^fail[[:space:]] ]]; then
        inner="${inner#fail }"
        local task_id="${inner%% *}"
        local reason=""

        if [[ "$inner" =~ \"([^\"]+)\" ]]; then
            reason="${BASH_REMATCH[1]}"
        fi

        echo "$task_id $reason"
        return 0
    fi

    return 1
}

# ============================================================================
# Command Type Detection
# ============================================================================

# Get the type of a task command
# Returns: start|progress|done|fail|unknown
classifier_get_command_type() {
    local command="$1"

    if [[ "$command" =~ \[yolla:task[[:space:]]start ]]; then
        echo "start"
    elif [[ "$command" =~ \[yolla:task[[:space:]]progress ]]; then
        echo "progress"
    elif [[ "$command" =~ \[yolla:task[[:space:]]done ]]; then
        echo "done"
    elif [[ "$command" =~ \[yolla:task[[:space:]]fail ]]; then
        echo "fail"
    else
        echo "unknown"
    fi
}

# ============================================================================
# Response Cleaning
# ============================================================================

# Remove routing commands from response text
# Usage: classifier_clean_response "response with [yolla:task...] commands"
# Returns: Response with commands removed
classifier_clean_response() {
    local response="$1"

    # Remove [yolla:task ...] patterns
    echo "$response" | sed -E 's/\[yolla:task[^\]]+\]//g'
}

# Remove all yolla commands from response
# Usage: classifier_clean_all_commands "response with [yolla:...] commands"
# Returns: Clean response
classifier_clean_all_commands() {
    local response="$1"

    # Remove [yolla:...] patterns
    echo "$response" | sed -E 's/\[yolla:[^\]]+\]//g'
}

# ============================================================================
# Batch Processing
# ============================================================================

# Process all task commands from a response
# Usage: classifier_process_response "response"
# Outputs: JSON-like structured data for each command
classifier_process_response() {
    local response="$1"

    classifier_extract_tasks "$response" | while IFS= read -r cmd; do
        [[ -z "$cmd" ]] && continue

        local cmd_type
        cmd_type=$(classifier_get_command_type "$cmd")

        case "$cmd_type" in
            start)
                local parsed
                parsed=$(classifier_parse_task "$cmd")
                local agent_id="${parsed%% *}"
                local description="${parsed#* }"
                echo "type=start agent=$agent_id description=$description"
                ;;
            progress)
                local parsed
                parsed=$(classifier_parse_progress "$cmd")
                local task_id="${parsed%% *}"
                local percent="${parsed#* }"
                echo "type=progress task_id=$task_id percent=$percent"
                ;;
            done)
                local task_id
                task_id=$(classifier_parse_done "$cmd")
                echo "type=done task_id=$task_id"
                ;;
            fail)
                local parsed
                parsed=$(classifier_parse_fail "$cmd")
                local task_id="${parsed%% *}"
                local reason="${parsed#* }"
                echo "type=fail task_id=$task_id reason=$reason"
                ;;
            *)
                echo "type=unknown command=$cmd"
                ;;
        esac
    done
}
