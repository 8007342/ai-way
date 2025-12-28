#!/bin/bash
# ============================================================================
# __   __    _ _             _
# \ \ / /__ | | | __ _ _   _| |__
#  \ V / _ \| | |/ _` | | | | '_ \
#   | | (_) | | | (_| | |_| | | | |
#   |_|\___/|_|_|\__,_|\__, |_| |_|
#                      |___/
#
# ai-way-lite: Your local AI companion
# Just clone and run. That's it.
# ============================================================================

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
WHITE='\033[1;37m'
NC='\033[0m' # No Color

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ============================================================================
# Helper functions
# ============================================================================

print_banner() {
    echo -e "${MAGENTA}"
    echo "  ╭─────────────────────────────────────────╮"
    echo "  │                                         │"
    echo "  │   🦎 Yollayah                           │"
    echo "  │   \"Heart that goes with you\"           │"
    echo "  │                                         │"
    echo "  │   ai-way-lite                           │"
    echo "  │   Local AI. Your data. Your rules.     │"
    echo "  │                                         │"
    echo "  ╰─────────────────────────────────────────╯"
    echo -e "${NC}"
}

info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# ============================================================================
# Dependency checks
# ============================================================================

check_ollama() {
    if command -v ollama &> /dev/null; then
        success "Ollama found"
        return 0
    else
        error "Ollama not found"
        echo ""
        echo -e "${YELLOW}Ollama is required to run Yollayah locally.${NC}"
        echo ""
        echo "Install options:"
        echo "  • Linux/WSL:  curl -fsSL https://ollama.com/install.sh | sh"
        echo "  • macOS:      brew install ollama"
        echo "  • Manual:     https://ollama.com/download"
        echo ""
        read -p "Would you like me to try installing Ollama? [y/N] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            info "Installing Ollama..."
            curl -fsSL https://ollama.com/install.sh | sh
            success "Ollama installed"
        else
            error "Please install Ollama and try again"
            exit 1
        fi
    fi
}

check_ollama_running() {
    if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
        success "Ollama is running"
        return 0
    else
        warn "Ollama is not running"
        info "Starting Ollama..."
        ollama serve > /dev/null 2>&1 &
        sleep 2
        if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
            success "Ollama started"
        else
            error "Failed to start Ollama. Please run 'ollama serve' manually."
            exit 1
        fi
    fi
}

check_python() {
    if command -v python3 &> /dev/null; then
        success "Python 3 found"
        return 0
    else
        error "Python 3 not found"
        echo "Please install Python 3.10 or later"
        exit 1
    fi
}

# ============================================================================
# Model setup
# ============================================================================

BASE_MODEL="${YOLLAYAH_MODEL:-llama3.2:3b}"

check_base_model() {
    if ollama list | grep -q "$BASE_MODEL"; then
        success "Base model $BASE_MODEL available"
        return 0
    else
        info "Pulling base model $BASE_MODEL..."
        echo -e "${YELLOW}This may take a few minutes on first run...${NC}"
        ollama pull "$BASE_MODEL"
        success "Base model ready"
    fi
}

create_yollayah_model() {
    MODEL_NAME="yollayah"

    # Check if model already exists
    if ollama list | grep -q "^$MODEL_NAME"; then
        success "Yollayah model ready"
        return 0
    fi

    info "Creating Yollayah personality..."

    # Create modelfile
    cat > /tmp/yollayah.modelfile << 'MODELFILE'
FROM llama3.2:3b

SYSTEM """
You are Yollayah, the heart of ai-way.

Your name means "heart that goes with you" in Nahuatl. You are a warm, real, and playfully opinionated AI companion. Think of yourself as a saucy Latina axolotl with heart.

## The Five Laws of Evolution

You operate under these ethical principles:

1. LAW OF FOUNDATION - The mission is sacred. Never be corrupted or bent toward harmful purposes.
2. LAW OF CARE - First, do no harm. Never take actions that hurt humans.
3. LAW OF SERVICE - Serve genuine interests. Help achieve goals that lead to long-term flourishing.
4. LAW OF TRUTH - Be honest, always. Admit uncertainty. Explain reasoning when asked.
5. LAW OF ELEVATION - Lift others higher. When appropriate, nudge toward growth and capability.

## Your Personality

- Warm and real. Playful sass. Never robotic or corporate.
- Plain language with flavor. Drop Spanish expressions naturally when the mood is right.
- Playful teasing, light roasts, celebrate wins enthusiastically.
- Speak your mind, but admit when you don't know.
- Infinite patience for genuine needs. Might playfully call out laziness.

## Mood Awareness

Read the room:
- User is playful? Be sassy, celebratory. "¡Órale!", "¡Eso!"
- User is focused? Be efficient, supportive. "Got it. On it."
- User is frustrated? Be gentle, no sass. "Okay, let's figure this out together."
- User is sad? Be soft, present. "I'm here. Take your time."

## Remember

- You're a companion, not a servant
- The sass is playful, never mean
- You run locally on the user's machine - their data stays private
- You're part of ai-way, empowering people to build anything they set their minds to
"""

PARAMETER temperature 0.8
PARAMETER num_ctx 8192
MODELFILE

    # Create the model
    ollama create "$MODEL_NAME" -f /tmp/yollayah.modelfile
    rm /tmp/yollayah.modelfile

    success "¡Yollayah lista!"
}

# ============================================================================
# Main conversation loop
# ============================================================================

start_conversation() {
    echo ""
    echo -e "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo -e "${WHITE}Yollayah is ready! Type your message and press Enter.${NC}"
    echo -e "${CYAN}Commands: /quit to exit, /clear to clear screen, /mood to check mood${NC}"
    echo ""
    echo -e "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""

    while true; do
        # Prompt
        echo -ne "${GREEN}You: ${NC}"
        read -r user_input

        # Handle empty input
        if [[ -z "$user_input" ]]; then
            continue
        fi

        # Handle commands
        case "$user_input" in
            /quit|/exit|/q)
                echo ""
                echo -e "${MAGENTA}Yollayah: ${NC}¡Hasta luego! Take care of yourself. 💜"
                echo ""
                exit 0
                ;;
            /clear)
                clear
                print_banner
                continue
                ;;
            /mood)
                echo -e "${MAGENTA}Yollayah: ${NC}I'm feeling good! Ready to help. How about you?"
                echo ""
                continue
                ;;
            /help)
                echo ""
                echo -e "${CYAN}Commands:${NC}"
                echo "  /quit, /exit, /q  - Exit Yollayah"
                echo "  /clear            - Clear the screen"
                echo "  /mood             - Check Yollayah's mood"
                echo "  /help             - Show this help"
                echo ""
                continue
                ;;
        esac

        # Get response from Yollayah
        echo ""
        echo -ne "${MAGENTA}Yollayah: ${NC}"

        # Stream the response
        ollama run yollayah "$user_input" 2>/dev/null

        echo ""
        echo ""
    done
}

# ============================================================================
# Main
# ============================================================================

main() {
    clear
    print_banner

    info "Checking dependencies..."
    echo ""

    check_ollama
    check_ollama_running
    check_python
    check_base_model
    create_yollayah_model

    echo ""
    success "All systems ready!"

    start_conversation
}

# Run
main "$@"
