#!/usr/bin/env bash
# robot.sh - Standardized --robot flag system for configurable module verbosity
#
# Usage:
#   source lib/common/robot.sh
#   robot_parse_flags "$@"
#
# Example:
#   ./script.sh --robot=tui=info:conductor=warn:ollama=debug:gpu=off
#
# Modules set ROBOT_<MODULE>_LEVEL environment variables:
#   ROBOT_TUI_LEVEL=info
#   ROBOT_CONDUCTOR_LEVEL=warn
#   ROBOT_OLLAMA_LEVEL=debug
#   ROBOT_GPU_LEVEL=off

# Log levels (ascending verbosity)
declare -A ROBOT_LOG_LEVELS=(
    [off]=0
    [error]=1
    [warn]=2
    [info]=3
    [debug]=4
    [trace]=5
    [full]=5  # Alias for trace
)

# Global log level (default: info)
ROBOT_GLOBAL_LEVEL="${ROBOT_GLOBAL_LEVEL:-info}"

# Parse --robot=module=level:module=level:... flags
robot_parse_flags() {
    local robot_spec=""

    # Extract --robot flag from arguments
    for arg in "$@"; do
        if [[ "$arg" == --robot=* ]]; then
            robot_spec="${arg#--robot=}"
            break
        fi
    done

    # If no --robot flag, use default global level
    if [[ -z "$robot_spec" ]]; then
        export ROBOT_GLOBAL_LEVEL
        return 0
    fi

    # Parse module:level pairs
    IFS=':' read -ra modules <<< "$robot_spec"

    for module_spec in "${modules[@]}"; do
        IFS='=' read -r module level <<< "$module_spec"

        # Validate level
        if [[ -z "${ROBOT_LOG_LEVELS[$level]}" ]]; then
            echo "Warning: Invalid log level '$level' for module '$module', using 'info'" >&2
            level="info"
        fi

        # Set module-specific level
        local var_name="ROBOT_${module^^}_LEVEL"
        export "${var_name}=${level}"
    done

    export ROBOT_GLOBAL_LEVEL
}

# Check if a log message should be printed
# Usage: robot_should_log MODULE LEVEL && echo "message"
robot_should_log() {
    local module="$1"
    local level="$2"

    # Get module-specific level or fall back to global
    local var_name="ROBOT_${module^^}_LEVEL"
    local module_level="${!var_name:-$ROBOT_GLOBAL_LEVEL}"

    # Get numeric values
    local module_level_num="${ROBOT_LOG_LEVELS[$module_level]:-3}"
    local message_level_num="${ROBOT_LOG_LEVELS[$level]:-3}"

    # Log if message level <= module level
    [[ $message_level_num -le $module_level_num ]]
}

# Log a message if module verbosity allows
# Usage: robot_log MODULE LEVEL "message"
robot_log() {
    local module="$1"
    local level="$2"
    shift 2
    local message="$*"

    if robot_should_log "$module" "$level"; then
        # Format: [MODULE] LEVEL: message
        local level_upper="${level^^}"
        echo "[${module}] ${level_upper}: ${message}" >&2
    fi
}

# Convenience functions for common log levels
robot_error() {
    robot_log "$1" "error" "${@:2}"
}

robot_warn() {
    robot_log "$1" "warn" "${@:2}"
}

robot_info() {
    robot_log "$1" "info" "${@:2}"
}

robot_debug() {
    robot_log "$1" "debug" "${@:2}"
}

robot_trace() {
    robot_log "$1" "trace" "${@:2}"
}

# Show current robot configuration
robot_show_config() {
    echo "Robot Configuration:" >&2
    echo "  Global level: ${ROBOT_GLOBAL_LEVEL}" >&2

    # Show all ROBOT_*_LEVEL variables
    for var in $(compgen -e | grep '^ROBOT_.*_LEVEL$' | sort); do
        local module="${var#ROBOT_}"
        module="${module%_LEVEL}"
        echo "  ${module,,}: ${!var}" >&2
    done
}

# Get --robot flag from arguments (for passing to subcommands)
robot_get_flag() {
    for arg in "$@"; do
        if [[ "$arg" == --robot=* ]]; then
            echo "$arg"
            return 0
        fi
    done
    return 1
}

# Remove --robot flag from arguments (returns cleaned args)
robot_strip_flag() {
    local args=()
    for arg in "$@"; do
        if [[ "$arg" != --robot=* ]]; then
            args+=("$arg")
        fi
    done
    echo "${args[@]}"
}
