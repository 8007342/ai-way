#!/bin/bash
# ============================================================================
# lib/ux/output.sh - Sanitized Output for AJ
#
# This module handles ALL user-facing output. AJ only sees output from
# ux_* functions - never raw echo or log_* functions.
#
# Design for Future Enhancement:
# - All output goes through this module
# - Easy to swap for ncurses, rich TUI, or GUI
# - Animation hooks for "thinking" states
# - Yollayah personality in feedback messages
#
# Future Vision:
# ┌──────────────────────────────────────────┐
# │  🦎 ~~~  (Yollayah swimming/thinking)   │
# │                                          │
# │  Checking dependencies...                │
# │  ████████░░░░░░░░░░░░  40%              │
# └──────────────────────────────────────────┘
#
# Constitution Reference:
# - Law of Care: Pleasant, non-stressful output
# - Law of Truth: Clear, honest status messages
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_UX_OUTPUT_LOADED:-}" ]] && return 0
_YOLLAYAH_UX_OUTPUT_LOADED=1

# ============================================================================
# Output Backend Configuration
# ============================================================================

# Output mode: "basic" (default), "color", "rich" (future: ncurses)
UX_MODE="${YOLLAYAH_UX_MODE:-color}"

# Whether to show spinners/animations
UX_ANIMATE="${YOLLAYAH_UX_ANIMATE:-true}"

# Quiet mode (minimal output)
UX_QUIET="${YOLLAYAH_QUIET:-false}"

# Debug mode - shows technical messages for PJ (Power Joe/Jane)
# When off, only Yollayah personality messages are shown to AJ
UX_DEBUG="${YOLLAYAH_DEBUG:-false}"
[[ "$UX_DEBUG" == "1" ]] && UX_DEBUG="true"

# ============================================================================
# Color Definitions (for color mode)
# ============================================================================

if [[ "$UX_MODE" == "color" ]] && [[ -t 1 ]]; then
    readonly UX_RED='\033[0;31m'
    readonly UX_GREEN='\033[0;32m'
    readonly UX_YELLOW='\033[0;33m'
    readonly UX_BLUE='\033[0;34m'
    readonly UX_MAGENTA='\033[0;35m'
    readonly UX_CYAN='\033[0;36m'
    readonly UX_WHITE='\033[1;37m'
    readonly UX_DIM='\033[2m'
    readonly UX_BOLD='\033[1m'
    readonly UX_NC='\033[0m'
else
    readonly UX_RED=''
    readonly UX_GREEN=''
    readonly UX_YELLOW=''
    readonly UX_BLUE=''
    readonly UX_MAGENTA=''
    readonly UX_CYAN=''
    readonly UX_WHITE=''
    readonly UX_DIM=''
    readonly UX_BOLD=''
    readonly UX_NC=''
fi

# ============================================================================
# Core Output Functions (AJ sees these)
# ============================================================================

# Informational message - HIDDEN unless debug mode
ux_info() {
    [[ "$UX_QUIET" == "true" ]] && return
    log_ux "INFO" "$1"
    [[ "$UX_DEBUG" != "true" ]] && return
    echo -e "${UX_CYAN}ℹ${UX_NC}  $1"
}

# Success message - HIDDEN unless debug mode
ux_success() {
    [[ "$UX_QUIET" == "true" ]] && return
    log_ux "INFO" "SUCCESS: $1"
    [[ "$UX_DEBUG" != "true" ]] && return
    echo -e "${UX_GREEN}✓${UX_NC}  $1"
}

# Warning message - always shown (important for AJ too)
ux_warn() {
    echo -e "${UX_YELLOW}⚠${UX_NC}  $1"
    log_ux "WARN" "$1"
}

# Error message - always shown (something went wrong)
ux_error() {
    echo -e "${UX_RED}✗${UX_NC}  $1" >&2
    log_ux "ERROR" "$1"
}

# ============================================================================
# PJ Debug Output (Power Joe/Jane - technical but accessible)
#
# These show ONLY when YOLLAYAH_DEBUG=1, providing helpful insight
# into what's happening under the hood. Messages are:
# - Short and sweet (one line)
# - Command-focused (show what's being run)
# - Informative but not overwhelming
# ============================================================================

# Show a command being run (for transparency)
pj_cmd() {
    [[ "$UX_DEBUG" != "true" ]] && return
    echo -e "${UX_DIM}→ Running:${UX_NC} ${UX_CYAN}$1${UX_NC}"
    log_ux "DEBUG" "CMD: $1"
}

# Show a check being performed
pj_check() {
    [[ "$UX_DEBUG" != "true" ]] && return
    echo -e "${UX_DIM}→ Checking:${UX_NC} $1"
    log_ux "DEBUG" "CHECK: $1"
}

# Show a result or finding
pj_result() {
    [[ "$UX_DEBUG" != "true" ]] && return
    echo -e "${UX_DIM}  └─${UX_NC} $1"
    log_ux "DEBUG" "RESULT: $1"
}

# Show a step in progress
pj_step() {
    [[ "$UX_DEBUG" != "true" ]] && return
    echo -e "${UX_BLUE}▸${UX_NC} $1"
    log_ux "DEBUG" "STEP: $1"
}

# Show detection/discovery
pj_found() {
    [[ "$UX_DEBUG" != "true" ]] && return
    echo -e "${UX_GREEN}✓${UX_NC} Found: $1"
    log_ux "DEBUG" "FOUND: $1"
}

# Show something wasn't found (not an error, just info)
pj_missing() {
    [[ "$UX_DEBUG" != "true" ]] && return
    echo -e "${UX_YELLOW}○${UX_NC} Not found: $1"
    log_ux "DEBUG" "MISSING: $1"
}

# Plain message (no prefix)
ux_print() {
    echo -e "$1"
}

# Blank line
ux_blank() {
    echo ""
}

# ============================================================================
# Yollayah Personality Messages
# ============================================================================

# Yollayah speaking (main character voice)
ux_yollayah() {
    local message="$1"
    echo -e "${UX_MAGENTA}Yollayah:${UX_NC} $message"
}

# Yollayah thinking (for async operations)
# Future: This will show animated axolotl
ux_yollayah_thinking() {
    local context="${1:-thinking}"

    # For now, simple message
    # Future: Animated axolotl swimming
    echo -ne "${UX_MAGENTA}Yollayah:${UX_NC} ${UX_DIM}*${context}*${UX_NC} "

    log_ux "DEBUG" "Yollayah thinking: $context"
}

# Clear thinking indicator
ux_yollayah_done_thinking() {
    # Clear the thinking line (carriage return, clear to end)
    echo -ne "\r\033[K"
}

# ============================================================================
# Progress Indicators
# ============================================================================

# Simple spinner characters
readonly UX_SPINNER_CHARS='⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏'
_ux_spinner_idx=0

# Show spinner frame (call repeatedly)
ux_spinner_tick() {
    local char="${UX_SPINNER_CHARS:_ux_spinner_idx:1}"
    echo -ne "\r${UX_CYAN}${char}${UX_NC} $1"
    _ux_spinner_idx=$(( (_ux_spinner_idx + 1) % ${#UX_SPINNER_CHARS} ))
}

# Clear spinner
ux_spinner_clear() {
    echo -ne "\r\033[K"
}

# Progress bar
# Usage: ux_progress "Downloading..." 45 100
ux_progress() {
    local label="$1"
    local current="$2"
    local total="$3"
    local width=30

    local percent=$((current * 100 / total))
    local filled=$((current * width / total))
    local empty=$((width - filled))

    local bar=""
    for ((i=0; i<filled; i++)); do bar+="█"; done
    for ((i=0; i<empty; i++)); do bar+="░"; done

    echo -ne "\r${label} ${bar} ${percent}%"

    if [[ $current -ge $total ]]; then
        echo ""  # Newline when complete
    fi
}

# ============================================================================
# Structured Output (for multi-line displays)
# ============================================================================

# Section header
ux_section() {
    local title="$1"
    ux_blank
    echo -e "${UX_BOLD}${title}${UX_NC}"
    echo -e "${UX_DIM}$(printf '─%.0s' {1..40})${UX_NC}"
}

# Key-value pair
ux_keyval() {
    local key="$1"
    local value="$2"
    printf "  ${UX_DIM}%s:${UX_NC} %s\n" "$key" "$value"
}

# List item
ux_item() {
    local item="$1"
    echo -e "  • $item"
}

# ============================================================================
# Special Displays
# ============================================================================

# Separator line
ux_separator() {
    echo -e "${UX_MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${UX_NC}"
}

# Box around text
ux_box() {
    local text="$1"
    local width=50

    echo -e "${UX_MAGENTA}╭$(printf '─%.0s' $(seq 1 $width))╮${UX_NC}"
    echo -e "${UX_MAGENTA}│${UX_NC} $(printf "%-$((width-2))s" "$text") ${UX_MAGENTA}│${UX_NC}"
    echo -e "${UX_MAGENTA}╰$(printf '─%.0s' $(seq 1 $width))╯${UX_NC}"
}

# ============================================================================
# Prompts and Input
# ============================================================================

# User input prompt
ux_prompt() {
    local prompt="$1"
    echo -ne "${UX_GREEN}${prompt}${UX_NC} "
}

# Yes/No confirmation
# Returns 0 for yes, 1 for no
ux_confirm() {
    local question="$1"
    local default="${2:-n}"  # default: no

    local prompt
    if [[ "$default" == "y" ]]; then
        prompt="[Y/n]"
    else
        prompt="[y/N]"
    fi

    echo -ne "${question} ${prompt} "
    read -r -n 1 answer
    echo ""

    case "${answer:-$default}" in
        [Yy]) return 0 ;;
        *) return 1 ;;
    esac
}

# ============================================================================
# Yollayah-Friendly Command Wrappers
# ============================================================================

# Run a command with a cute spinner, hiding scary output
# Usage: ux_run_friendly "Getting things ready..." command arg1 arg2
ux_run_friendly() {
    local message="$1"
    shift
    local cmd=("$@")

    # Yollayah phrases to cycle through while waiting
    local phrases=(
        "Almost there..."
        "Still working on it..."
        "Doing the thing..."
        "Making progress..."
        "Just a bit more..."
        "Getting closer..."
    )
    local phrase_idx=0

    # Start the command in background, capturing output to temp file
    local temp_out
    temp_out=$(mktemp)
    "${cmd[@]}" > "$temp_out" 2>&1 &
    local cmd_pid=$!

    # Show spinner while command runs
    local spin_idx=0
    local tick=0
    while kill -0 "$cmd_pid" 2>/dev/null; do
        local char="${UX_SPINNER_CHARS:spin_idx:1}"

        # Change phrase every ~3 seconds (30 ticks at 0.1s)
        if [[ $((tick % 30)) -eq 0 ]] && [[ $tick -gt 0 ]]; then
            phrase_idx=$(( (phrase_idx + 1) % ${#phrases[@]} ))
        fi

        echo -ne "\r${UX_MAGENTA}${char}${UX_NC} ${message} ${UX_DIM}${phrases[$phrase_idx]}${UX_NC}  "
        spin_idx=$(( (spin_idx + 1) % ${#UX_SPINNER_CHARS} ))
        tick=$((tick + 1))
        sleep 0.1
    done

    # Get exit code
    wait "$cmd_pid"
    local exit_code=$?

    # Clear spinner line
    echo -ne "\r\033[K"

    # Clean up
    rm -f "$temp_out"

    return $exit_code
}

# Run ollama pull with cute progress
# Usage: ux_ollama_pull "llama3.2:3b" [skip_intro]
# If skip_intro is "true", skips the intro message (caller handles it)
ux_ollama_pull() {
    local model="$1"
    local skip_intro="${2:-false}"

    # In test verbose mode, show raw ollama output
    if [[ -n "${YOLLAYAH_TEST_VERBOSE:-}" ]]; then
        echo ">>> Pulling model: $model"
        ollama pull "$model"
        return $?
    fi

    # Cute intro message (skipped if caller already announced)
    if [[ "$skip_intro" != "true" ]]; then
        ux_yollayah "$(yollayah_thinking) Downloading my brain... this might take a minute!"
    fi
    ux_blank

    # Run ollama pull with spinner, hiding scary hashes
    if ux_run_friendly "Fetching ${model}..." ollama pull "$model"; then
        ux_yollayah "$(yollayah_celebration) Got it! Brain downloaded."
        return 0
    else
        ux_yollayah "$(yollayah_interjection) Download failed. Internet okay?"
        return 1
    fi
}

# Run ollama create with cute progress and GPU optimization
# Usage: ux_ollama_create "yollayah" "/tmp/modelfile"
ux_ollama_create() {
    local model_name="$1"
    local modelfile="$2"

    ux_yollayah "$(yollayah_thinking) Putting myself together..."
    ux_blank

    # Force GPU layer offload for maximum performance
    # OLLAMA_NUM_GPU=999 tells Ollama to use all available GPU layers
    export OLLAMA_NUM_GPU=999

    # Run ollama create with spinner, hiding scary hashes
    if ux_run_friendly "Building ${model_name}..." ollama create "$model_name" -f "$modelfile"; then
        unset OLLAMA_NUM_GPU
        return 0
    else
        unset OLLAMA_NUM_GPU
        return 1
    fi
}

# ============================================================================
# Future: Animation Hooks
# ============================================================================

# Placeholder for future ncurses/animation integration
#
# When we implement rich TUI:
# - ux_yollayah_thinking → Animated axolotl swimming
# - ux_progress → Smooth progress bar
# - ux_spinner_tick → Fluid animation
#
# The axolotl animation frames might look like:
#
# Frame 1:    Frame 2:    Frame 3:
#   🦎~~~      ~🦎~~       ~~🦎~
#
# Or ASCII art version:
#
#    .---.
#   ( o o )~~~
#    `---'
#

# Animation state (for future use)
_ux_animation_running=false
_ux_animation_pid=""

# Start background animation (future)
ux_animation_start() {
    local animation_type="$1"
    log_debug "Animation start: $animation_type (not implemented)" "ux"
    # TODO: Launch background process for animation
}

# Stop background animation (future)
ux_animation_stop() {
    log_debug "Animation stop (not implemented)" "ux"
    # TODO: Kill animation process, restore cursor
}

# ============================================================================
# Mode Detection
# ============================================================================

# Check if we're in a terminal that supports colors
ux_supports_color() {
    [[ -t 1 ]] && [[ "${TERM:-dumb}" != "dumb" ]]
}

# Check if we're in a terminal that supports unicode
ux_supports_unicode() {
    [[ "${LANG:-}" == *UTF-8* ]] || [[ "${LC_ALL:-}" == *UTF-8* ]]
}

# Check terminal width
ux_terminal_width() {
    tput cols 2>/dev/null || echo 80
}

# ============================================================================
# Initialization
# ============================================================================

# Auto-detect best output mode
_ux_init() {
    if ! ux_supports_color; then
        UX_MODE="basic"
    fi

    log_ux "DEBUG" "UX initialized: mode=$UX_MODE"
}

_ux_init
