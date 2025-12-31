#!/bin/bash
# ============================================================================
# lib/routing/avatar_agent.sh - Autonomous Avatar Behavior Controller
#
# This module generates autonomous avatar behavior while background tasks run.
# Yollayah's avatar should feel alive - peeking at tasks, swimming around,
# reacting to completions, showing anticipation.
#
# Behavior Loop:
# 1. Check current state (active tasks, progress, etc.)
# 2. Generate appropriate avatar commands
# 3. Send commands to TUI via FIFO
# 4. Wait, repeat
#
# Avatar Commands Generated:
#   [yolla:move <x> <y>]     - Move to position
#   [yolla:move center/tl/tr/bl/br] - Move to named position
#   [yolla:mood <mood>]      - Set mood expression
#   [yolla:peek <direction>] - Peek at something
#   [yolla:swim]             - Swim around playfully
#   [yolla:wander]           - Roam freely
#   [yolla:react <reaction>] - React to event
#   [yolla:point <x> <y>]    - Point at location
#
# Constitution Reference:
# - Law of Care: Non-intrusive, delightful presence
# - Law of Elevation: Show engagement with AJ's work
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_AVATAR_AGENT_LOADED:-}" ]] && return 0
_YOLLAYAH_AVATAR_AGENT_LOADED=1

# ============================================================================
# Configuration
# ============================================================================

# Behavior tick interval (seconds)
AVATAR_BEHAVIOR_INTERVAL="${AVATAR_BEHAVIOR_INTERVAL:-4}"

# Minimum/maximum interval for variety
AVATAR_BEHAVIOR_MIN_INTERVAL=2
AVATAR_BEHAVIOR_MAX_INTERVAL=8

# Whether avatar behavior is enabled
AVATAR_BEHAVIOR_ENABLED="${AVATAR_BEHAVIOR_ENABLED:-true}"

# ============================================================================
# State Tracking
# ============================================================================

_avatar_last_task_count=0
_avatar_last_check_time=0
_avatar_idle_ticks=0

# ============================================================================
# Avatar Behavior Loop
# ============================================================================

# Start the avatar behavior loop
# Runs in background, generates commands while tasks are active
avatar_behavior_start() {
    [[ "$AVATAR_BEHAVIOR_ENABLED" != "true" ]] && return 0

    log_debug "Starting avatar behavior loop"

    # Ensure FIFO exists
    if [[ ! -p "$AVATAR_FIFO" ]]; then
        mkfifo "$AVATAR_FIFO" 2>/dev/null || true
    fi

    # Initial greeting when tasks start
    avatar_emit_command "[yolla:mood curious]"
    avatar_emit_command "[yolla:move tr]"

    # Main behavior loop
    while true; do
        avatar_behavior_tick

        # Variable interval for natural feel
        local interval=$((AVATAR_BEHAVIOR_MIN_INTERVAL + RANDOM % (AVATAR_BEHAVIOR_MAX_INTERVAL - AVATAR_BEHAVIOR_MIN_INTERVAL + 1)))
        sleep "$interval"
    done
}

# Single behavior tick
avatar_behavior_tick() {
    local active_tasks
    active_tasks=$(task_count_active)

    # Check for task state changes
    if [[ $active_tasks -ne $_avatar_last_task_count ]]; then
        if [[ $active_tasks -gt $_avatar_last_task_count ]]; then
            # New task started
            avatar_on_task_started
        elif [[ $active_tasks -lt $_avatar_last_task_count && $active_tasks -eq 0 ]]; then
            # All tasks completed
            avatar_on_all_tasks_done
        fi
        _avatar_last_task_count=$active_tasks
    fi

    # Generate behavior based on state
    if [[ $active_tasks -gt 0 ]]; then
        avatar_behavior_during_tasks "$active_tasks"
        _avatar_idle_ticks=0
    else
        ((_avatar_idle_ticks++))
        avatar_behavior_idle
    fi
}

# ============================================================================
# Context-Aware Behaviors
# ============================================================================

# Behavior when tasks are running
avatar_behavior_during_tasks() {
    local task_count="$1"

    # Get a random active task to potentially focus on
    local random_task
    random_task=$(task_list_active | shuf -n 1)

    local behavior=$((RANDOM % 10))

    case $behavior in
        0|1)
            # Peek at task panel
            avatar_emit_command "[yolla:peek right]"
            avatar_emit_command "[yolla:mood curious]"
            ;;
        2|3)
            # Move near task panel
            avatar_emit_command "[yolla:move 85 30]"
            avatar_emit_command "[yolla:mood thinking]"
            ;;
        4)
            # Check task progress
            if [[ -n "$random_task" ]]; then
                local progress
                progress=$(task_get_progress "$random_task")
                if [[ $progress -gt 75 ]]; then
                    avatar_emit_command "[yolla:mood excited]"
                    avatar_emit_command "[yolla:bounce]"
                fi
            fi
            ;;
        5|6)
            # Swim around casually
            avatar_emit_command "[yolla:swim]"
            avatar_emit_command "[yolla:mood playful]"
            ;;
        7)
            # Move to different corner
            local corners=("tl" "tr" "bl" "br")
            local corner="${corners[$((RANDOM % 4))]}"
            avatar_emit_command "[yolla:move $corner]"
            ;;
        8)
            # Wander freely
            avatar_emit_command "[yolla:wander]"
            ;;
        9)
            # Point at task panel
            avatar_emit_command "[yolla:point 90 20]"
            avatar_emit_command "[yolla:mood curious]"
            ;;
    esac
}

# Behavior when idle (no tasks)
avatar_behavior_idle() {
    # Less active when idle
    if [[ $((_avatar_idle_ticks % 3)) -ne 0 ]]; then
        return
    fi

    local behavior=$((RANDOM % 6))

    case $behavior in
        0)
            # Gentle wander
            avatar_emit_command "[yolla:wander]"
            ;;
        1)
            # Return to center
            avatar_emit_command "[yolla:move center]"
            avatar_emit_command "[yolla:mood happy]"
            ;;
        2)
            # Playful wiggle
            avatar_emit_command "[yolla:wiggle]"
            ;;
        3)
            # Look around
            avatar_emit_command "[yolla:peek left]"
            sleep 1
            avatar_emit_command "[yolla:peek right]"
            ;;
        4|5)
            # Just chill
            avatar_emit_command "[yolla:mood content]"
            ;;
    esac
}

# ============================================================================
# Event Reactions
# ============================================================================

# React when a new task starts
avatar_on_task_started() {
    log_debug "Avatar reacting to task start"

    avatar_emit_command "[yolla:react hmm]"
    avatar_emit_command "[yolla:move tr]"
    avatar_emit_command "[yolla:mood curious]"
}

# React when all tasks complete
avatar_on_all_tasks_done() {
    log_debug "Avatar reacting to all tasks done"

    avatar_emit_command "[yolla:move center]"
    avatar_emit_command "[yolla:react tada]"
    avatar_emit_command "[yolla:mood excited]"
    avatar_emit_command "[yolla:dance]"
}

# React to a specific task completing
avatar_on_task_complete() {
    local task_id="$1"
    local agent_id
    agent_id=$(task_get_agent "$task_id")

    avatar_emit_command "[yolla:react love]"
    avatar_emit_command "[yolla:celebrate task $task_id]"
}

# React to a task failing
avatar_on_task_failed() {
    local task_id="$1"

    avatar_emit_command "[yolla:mood confused]"
    avatar_emit_command "[yolla:react oops]"
}

# ============================================================================
# Command Emission
# ============================================================================

# Send a command to the TUI
avatar_emit_command() {
    local command="$1"

    # Write to FIFO if it exists
    if [[ -p "$AVATAR_FIFO" ]]; then
        echo "$command" > "$AVATAR_FIFO" 2>/dev/null &
    fi

    # Also log for debugging
    log_debug "Avatar command: $command"
}

# Emit multiple commands with delay
avatar_emit_sequence() {
    local delay="${1:-0.5}"
    shift

    for cmd in "$@"; do
        avatar_emit_command "$cmd"
        sleep "$delay"
    done
}

# ============================================================================
# LLM-Driven Behavior (Optional, for richer behavior)
# ============================================================================

# Generate behavior using LLM (more creative but slower)
avatar_generate_behavior_llm() {
    local active_tasks="$1"
    local task_info=""

    # Get info about active tasks
    for task_id in $(task_list_active | head -3); do
        local agent progress
        agent=$(task_get_agent "$task_id")
        progress=$(task_get_progress "$task_id")
        task_info+="- ${agent}: ${progress}%\n"
    done

    local prompt="You are controlling Yollayah's avatar during background task execution.

Current state:
- Active tasks: ${active_tasks}
- Task details:
${task_info}

Generate 1-2 avatar commands to make Yollayah feel alive while waiting.
Output ONLY commands like:
[yolla:move 30 50]
[yolla:mood curious]
[yolla:peek right]

Keep it subtle and natural. Don't overdo it."

    # Quick inference with small context
    ollama run "$YOLLAYAH_MODEL_NAME" "$prompt" 2>/dev/null | head -3
}

# ============================================================================
# Behavior Control
# ============================================================================

# Stop avatar behavior (kill the loop)
avatar_behavior_stop() {
    # The parent process should kill the background loop
    log_debug "Avatar behavior stop requested"
}

# Enable/disable avatar behavior
avatar_behavior_enable() {
    AVATAR_BEHAVIOR_ENABLED=true
}

avatar_behavior_disable() {
    AVATAR_BEHAVIOR_ENABLED=false
}

# Set behavior intensity
avatar_set_intensity() {
    local intensity="$1"  # low, medium, high

    case "$intensity" in
        low)
            AVATAR_BEHAVIOR_MIN_INTERVAL=6
            AVATAR_BEHAVIOR_MAX_INTERVAL=12
            ;;
        medium)
            AVATAR_BEHAVIOR_MIN_INTERVAL=3
            AVATAR_BEHAVIOR_MAX_INTERVAL=7
            ;;
        high)
            AVATAR_BEHAVIOR_MIN_INTERVAL=1
            AVATAR_BEHAVIOR_MAX_INTERVAL=4
            ;;
    esac
}
