#!/bin/bash
# ============================================================================
# lib/ux/terminal.sh - Terminal User Interface
#
# This module handles:
# - Banner and branding
# - User prompts and input
# - Command handling (/quit, /clear, /help, etc.)
# - Conversation loop
# - Visual feedback
#
# UX Philosophy:
# - Clean, uncluttered interface
# - Yollayah's personality shines through
# - Commands are discoverable but not intrusive
# - Color enhances but doesn't overwhelm
#
# Output Architecture:
# - All display output uses ux_* functions from lib/ux/output.sh
# - Uses UX_* color constants for consistency
# - Never uses raw echo for formatted output
#
# Constitution Reference:
# - Law of Truth: Clear, honest interface
# - Law of Care: Pleasant, non-stressful experience
# ============================================================================

# Prevent double-sourcing
[[ -n "$_YOLLAYAH_UX_TERMINAL_LOADED" ]] && return 0
_YOLLAYAH_UX_TERMINAL_LOADED=1

# ============================================================================
# Banner and Branding
# ============================================================================

ux_print_banner() {
    echo -e "${UX_MAGENTA}"
    cat << 'BANNER'
  ╭─────────────────────────────────────────╮
  │                                         │
  │   🦎 Yollayah                           │
  │   "Heart that goes with you"            │
  │                                         │
  │   ai-way-lite                           │
  │   Local AI. Your data. Your rules.      │
  │                                         │
  ╰─────────────────────────────────────────╯
BANNER
    echo -e "${UX_NC}"
}

ux_print_separator() {
    ux_separator
}

# ============================================================================
# Ready Message
# ============================================================================

ux_print_ready() {
    ux_blank
    ux_print_separator
    ux_blank
    echo -e "${UX_WHITE}Yollayah is ready! Type your message and press Enter.${UX_NC}"
    echo -e "${UX_CYAN}Commands: /quit to exit, /clear to clear screen, /help for more${UX_NC}"
    ux_blank
    ux_print_separator
    ux_blank
}

# ============================================================================
# Help Display
# ============================================================================

ux_show_help() {
    ux_blank
    echo -e "${UX_CYAN}Commands:${UX_NC}"
    ux_item "/quit, /exit, /q  - Exit Yollayah"
    ux_item "/clear            - Clear the screen"
    ux_item "/mood             - Check Yollayah's mood"
    ux_item "/model            - Show current model"
    ux_item "/help             - Show this help"
    ux_blank
    echo -e "${UX_CYAN}Tips:${UX_NC}"
    ux_item "Just type naturally, Yollayah understands context"
    ux_item "Your conversations stay local and private"
    ux_item "Check out agents/ai-way-docs/ for more info"
    ux_blank
}

# ============================================================================
# Command Handling
# ============================================================================

# Handle a slash command
# Returns 0 if command was handled, 1 if not a command
ux_handle_command() {
    local input="$1"

    case "$input" in
        /quit|/exit|/q)
            ux_blank
            ux_yollayah "¡Hasta luego! Take care of yourself. 💜"
            ux_blank
            exit 0
            ;;
        /clear)
            clear
            ux_print_banner
            return 0
            ;;
        /mood)
            ux_yollayah "I'm feeling good! Ready to help. How about you?"
            ux_blank
            return 0
            ;;
        /model)
            ux_keyval "Current model" "$SELECTED_MODEL"
            ux_keyval "Hardware" "$(hardware_summary)"
            ux_blank
            return 0
            ;;
        /help)
            ux_show_help
            return 0
            ;;
        /debug)
            # Hidden command to toggle debug mode
            if [[ -n "$YOLLAYAH_DEBUG" ]]; then
                unset YOLLAYAH_DEBUG
                ux_info "Debug mode disabled"
            else
                export YOLLAYAH_DEBUG=1
                ux_info "Debug mode enabled"
            fi
            ux_blank
            return 0
            ;;
        /*)
            # Unknown command
            ux_warn "Unknown command: $input"
            ux_print "Type /help for available commands"
            ux_blank
            return 0
            ;;
        *)
            # Not a command
            return 1
            ;;
    esac
}

# ============================================================================
# Conversation Loop
# ============================================================================

# Main conversation loop
ux_conversation_loop() {
    local model_name="$1"

    ux_print_ready

    while true; do
        # Prompt
        ux_prompt "You:"
        read -r user_input

        # Handle empty input
        if [[ -z "$user_input" ]]; then
            continue
        fi

        # Handle commands
        if ux_handle_command "$user_input"; then
            continue
        fi

        # Get response from Yollayah
        ux_blank
        echo -ne "${UX_MAGENTA}Yollayah:${UX_NC} "

        # Stream the response
        ollama run "$model_name" "$user_input" 2>/dev/null

        ux_blank
        ux_blank
    done
}

# ============================================================================
# Progress and Feedback
# ============================================================================

ux_show_startup_progress() {
    ux_info "Checking dependencies..."
    ux_blank
}

ux_show_all_ready() {
    ux_blank
    ux_success "All systems ready!"
}

# ============================================================================
# Future: Rich Terminal UI
# ============================================================================

# TODO: When we add richer terminal UI features:
# - Markdown rendering
# - Code syntax highlighting
# - Progress bars for model downloads
# - Split panes for dev mode (show routing)
# - History navigation with arrow keys

# For now, we keep it simple and let bash do the work.
# Rich features can come later via Python surfaces.
