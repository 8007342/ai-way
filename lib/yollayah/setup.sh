#!/bin/bash
# ============================================================================
# lib/yollayah/setup.sh - First-Run Setup with Yollayah Personality
#
# Handles installation of dependencies (Ollama, Rust, etc.) with a friendly,
# non-scary Yollayah personality. AJ shouldn't be intimidated by system prompts.
#
# Key Principles:
# - Explain what's happening in plain language
# - Warn about sudo prompts graciously
# - Make the scary system text feel safe
# - Only happens once (or when updates needed)
#
# Constitution Reference:
# - Law of Care: Don't stress AJ out
# - Law of Truth: Be honest about what's being installed
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_SETUP_LOADED:-}" ]] && return 0
_YOLLAYAH_SETUP_LOADED=1

# ============================================================================
# Setup State
# ============================================================================

readonly SETUP_STATE_FILE="${STATE_DIR}/setup.done"

# Check if first-run setup is needed
setup_needed() {
    [[ ! -f "$SETUP_STATE_FILE" ]]
}

# Mark setup as complete
setup_mark_done() {
    date -Iseconds > "$SETUP_STATE_FILE"
}

# ============================================================================
# Dependency Checks
# ============================================================================

# Check if Ollama is installed
setup_has_ollama() {
    pj_check "Ollama installation"
    pj_cmd "command -v ollama"
    if command -v ollama &> /dev/null; then
        pj_found "ollama at $(command -v ollama)"
        return 0
    fi
    pj_missing "ollama"
    return 1
}

# Check if Rust/Cargo is installed
setup_has_rust() {
    pj_check "Rust toolchain"

    # First check if cargo is already in PATH
    pj_cmd "command -v cargo"
    if command -v cargo &> /dev/null; then
        pj_found "cargo at $(command -v cargo)"
        return 0
    fi

    # Check if rustup installed cargo but it's not in PATH yet
    pj_check "$HOME/.cargo/bin/cargo"
    if [[ -f "$HOME/.cargo/bin/cargo" ]]; then
        pj_found "$HOME/.cargo/bin/cargo (not in PATH)"
        # Source the env to add it to PATH for this session
        pj_cmd "source $HOME/.cargo/env"
        source "$HOME/.cargo/env" 2>/dev/null || true
        return 0
    fi

    # Check common alternative locations
    pj_check "Alternative rustup locations"
    if [[ -f "$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo" ]]; then
        pj_found "cargo in rustup toolchains"
        return 0
    fi

    pj_missing "Rust/Cargo"
    return 1
}

# Check if all dependencies are present
setup_has_all_dependencies() {
    setup_has_ollama && setup_has_rust
}

# ============================================================================
# Gracious Sudo Warning (only when actually needed!)
# ============================================================================

# Show a friendly warning before sudo prompts
# Only call this for things that ACTUALLY need sudo!
setup_warn_sudo() {
    local what_for="$1"

    ux_blank
    ux_separator

    # Random interjection for personality
    local interj
    interj=$(yollayah_interjection)

    ux_yollayah "${interj} Quick heads up!"
    ux_blank

    cat << 'BANNER'
    ╭──────────────────────────────────────────────────────╮
    │                                                      │
    │   🔐  Your computer's gonna ask for permission!      │
    │                                                      │
    │   Totally normal - it just wants to make sure        │
    │   YOU are okay with me installing stuff.             │
    │                                                      │
    │   This only happens once. Pinky promise.             │
    │                                                      │
    ╰──────────────────────────────────────────────────────╯
BANNER

    ux_blank
    ux_yollayah "What I'm installing: ${what_for}"
    ux_yollayah "Type your password when asked (it stays invisible, that's normal!)"
    ux_blank

    # Sassy punchline
    local punchlines=(
        "Computers are so dramatic, right? You got this!"
        "Think of it like your computer saying 'pretty please?'"
        "After this, smooth sailing. I got you."
        "One password and we're golden, amigo!"
    )
    local punchline_idx=$((RANDOM % ${#punchlines[@]}))
    ux_yollayah "${punchlines[$punchline_idx]}"

    ux_blank
    ux_separator
    ux_blank

    # Give AJ a moment to read
    sleep 2
}

# ============================================================================
# Dependency Installation
# ============================================================================

# Install Ollama (needs sudo - this is the only one that does!)
setup_install_ollama() {
    pj_step "Checking Ollama installation"
    if setup_has_ollama; then
        log_ollama "INFO" "Ollama already installed, skipping installation"
        pj_result "Already installed, skipping"
        return 0
    fi

    log_ollama "INFO" "Ollama not found, starting installation"
    pj_step "Installing Ollama"

    ux_yollayah "$(yollayah_thinking) Getting the AI brain ready..."

    # THIS actually needs sudo, so warn graciously
    setup_warn_sudo "the AI brain (Ollama) - it's what makes me smart!"

    # Download installer to temp file first (so we can run it with friendly wrapper)
    local installer_script
    installer_script=$(mktemp)
    pj_result "Temp script: $installer_script"

    ux_blank
    pj_cmd "curl -fsSL https://ollama.com/install.sh"
    if ! curl -fsSL https://ollama.com/install.sh -o "$installer_script" 2>/dev/null; then
        rm -f "$installer_script"
        pj_result "Download failed"
        ux_yollayah "$(yollayah_interjection) Couldn't download. Internet okay?"
        return 1
    fi
    pj_result "Downloaded installer script"

    # Run installer with friendly spinner (hides scary output)
    pj_cmd "sudo sh $installer_script"
    if ux_run_friendly "Installing Ollama..." sudo sh "$installer_script"; then
        rm -f "$installer_script"
        pj_result "Installation successful"
        ux_yollayah "$(yollayah_celebration) Brain installed! I can think now!"
        return 0
    else
        rm -f "$installer_script"
        pj_result "Installation failed"
        ux_yollayah "$(yollayah_interjection) Hmm, that didn't work. Check the logs?"
        return 1
    fi
}

# Install Rust (NO sudo needed - just goes in your home folder!)
setup_install_rust() {
    pj_step "Checking Rust installation"
    if setup_has_rust; then
        log_info "Rust already installed, skipping installation"
        pj_result "Already installed, skipping"
        ux_success "Rust toolchain found"
        return 0
    fi

    log_info "Rust not found, starting installation"
    pj_step "Installing Rust toolchain"
    ux_yollayah "$(yollayah_thinking) Getting some tools for the pretty interface..."
    ux_yollayah "This one's easy - no password needed!"
    ux_blank

    # Download installer to temp file first
    local installer_script
    installer_script=$(mktemp)
    pj_result "Temp script: $installer_script"

    pj_cmd "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs"
    if ! curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o "$installer_script" 2>/dev/null; then
        rm -f "$installer_script"
        pj_result "Download failed"
        ux_yollayah "$(yollayah_interjection) Couldn't download. Internet okay?"
        return 1
    fi
    pj_result "Downloaded rustup installer"

    # Run with friendly wrapper (hides scary output)
    pj_cmd "sh $installer_script -y"
    if ux_run_friendly "Installing Rust..." sh "$installer_script" -y; then
        rm -f "$installer_script"
        pj_result "Installation successful"
        # Source cargo env
        pj_cmd "source $HOME/.cargo/env"
        source "$HOME/.cargo/env" 2>/dev/null || true
        pj_result "Rust now available at $(command -v cargo 2>/dev/null || echo '$HOME/.cargo/bin/cargo')"
        ux_yollayah "$(yollayah_celebration) Got it! Now I can look pretty."
        return 0
    else
        rm -f "$installer_script"
        pj_result "Installation failed"
        ux_yollayah "$(yollayah_interjection) That didn't work. Internet hiccup maybe?"
        return 1
    fi
}

# ============================================================================
# Main Setup Flow
# ============================================================================

# Run the first-time setup (only does work if actually needed!)
setup_run() {
    log_session "INFO" "Running setup check"
    pj_step "Running first-time setup check"

    # Already done AND have everything? Skip entirely!
    pj_check "Previous setup state"
    if ! setup_needed && setup_has_all_dependencies; then
        log_session "INFO" "Setup already complete and all deps present"
        pj_result "Setup complete, all dependencies present"
        return 0
    fi

    # Check what's missing
    pj_step "Checking dependencies"
    local missing_ollama=false
    local missing_rust=false

    if ! setup_has_ollama; then
        missing_ollama=true
        log_session "INFO" "Ollama not found, will install"
    else
        log_session "DEBUG" "Ollama found"
    fi

    if ! setup_has_rust; then
        missing_rust=true
        log_session "INFO" "Rust not found, will install"
    else
        log_session "DEBUG" "Rust found"
    fi

    # Everything's already there? Just mark done and bounce
    if [[ "$missing_ollama" == "false" && "$missing_rust" == "false" ]]; then
        log_session "INFO" "All dependencies present, marking setup done"
        pj_result "All dependencies present"
        setup_mark_done
        return 0
    fi

    # Okay, we actually need to install something
    pj_step "Installing missing dependencies"
    [[ "$missing_ollama" == "true" ]] && pj_result "Will install: Ollama"
    [[ "$missing_rust" == "true" ]] && pj_result "Will install: Rust"

    ux_blank
    ux_yollayah "$(yollayah_interjection) First time? Let me get myself ready!"
    ux_blank

    # Install what's missing (Ollama first since it needs sudo)
    if [[ "$missing_ollama" == "true" ]]; then
        setup_install_ollama || return 1
        ux_blank
    fi

    if [[ "$missing_rust" == "true" ]]; then
        setup_install_rust || return 1
        ux_blank
    fi

    # Mark setup complete
    pj_step "Marking setup complete"
    setup_mark_done
    pj_result "Setup state saved"

    ux_yollayah "$(yollayah_celebration) All done! That wasn't so bad, right?"
    ux_blank

    return 0
}

# ============================================================================
# Update Check
# ============================================================================

# Check if any dependencies need updating
setup_check_updates() {
    # Future: Check for Ollama/Rust updates
    # For now, just return success
    return 0
}
