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
    echo -e "${MAGENTA}"
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
    echo -e "${NC}"
}

ux_print_separator() {
    echo -e "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

# ============================================================================
# Ready Message
# ============================================================================

ux_print_ready() {
    echo ""
    ux_print_separator
    echo ""
    echo -e "${WHITE}Yollayah is ready! Type your message and press Enter.${NC}"
    echo -e "${CYAN}Commands: /quit to exit, /clear to clear screen, /help for more${NC}"
    echo ""
    ux_print_separator
    echo ""
}

# ============================================================================
# Help Display
# ============================================================================

ux_show_help() {
    echo ""
    echo -e "${CYAN}Commands:${NC}"
    echo "  /quit, /exit, /q  - Exit Yollayah"
    echo "  /clear            - Clear the screen"
    echo "  /mood             - Check Yollayah's mood"
    echo "  /model            - Show current model"
    echo "  /help             - Show this help"
    echo ""
    echo -e "${CYAN}Tips:${NC}"
    echo "  - Just type naturally, Yollayah understands context"
    echo "  - Your conversations stay local and private"
    echo "  - Check out agents/ai-way-docs/ for more info"
    echo ""
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
            echo ""
            echo -e "${MAGENTA}Yollayah: ${NC}¡Hasta luego! Take care of yourself. 💜"
            echo ""
            exit 0
            ;;
        /clear)
            clear
            ux_print_banner
            return 0
            ;;
        /mood)
            echo -e "${MAGENTA}Yollayah: ${NC}I'm feeling good! Ready to help. How about you?"
            echo ""
            return 0
            ;;
        /model)
            echo -e "${CYAN}Current model:${NC} $SELECTED_MODEL"
            echo -e "${CYAN}Hardware:${NC} $(hardware_summary)"
            echo ""
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
                echo -e "${CYAN}Debug mode disabled${NC}"
            else
                export YOLLAYAH_DEBUG=1
                echo -e "${CYAN}Debug mode enabled${NC}"
            fi
            echo ""
            return 0
            ;;
        /*)
            # Unknown command
            echo -e "${YELLOW}Unknown command: $input${NC}"
            echo -e "Type /help for available commands"
            echo ""
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
        echo -ne "${GREEN}You: ${NC}"
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
        echo ""
        echo -ne "${MAGENTA}Yollayah: ${NC}"

        # Stream the response
        ollama run "$model_name" "$user_input" 2>/dev/null

        echo ""
        echo ""
    done
}

# ============================================================================
# Progress and Feedback
# ============================================================================

ux_show_startup_progress() {
    info "Checking dependencies..."
    echo ""
}

ux_show_all_ready() {
    echo ""
    success "All systems ready!"
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
