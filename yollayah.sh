#!/bin/bash
# ============================================================================
# __   __    _ _                   _
# \ \ / /__ | | | __ _ _   _  __ _| |__
#  \ V / _ \| | |/ _` | | | |/ _` | '_ \
#   | | (_) | | | (_| | |_| | (_| | | | |
#   |_|\___/|_|_|\__,_|\__, |\__,_|_| |_|
#                      |___/
#
# ai-way-lite: Your local AI companion
# Just clone and run. That's it.
#
# This is the bootstrap script. It:
# 1. Sets up the environment
# 2. Sources the modular components
# 3. Runs the main entry point
#
# For module documentation, see lib/*/
# For privacy policy, see lib/user/README.md
# For ethical principles, see agents/CONSTITUTION.md (after first run)
# ============================================================================

set -e

# ============================================================================
# Bootstrap: Environment Setup
# ============================================================================

# Get script directory (all paths relative to this)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export SCRIPT_DIR

# ============================================================================
# Bootstrap: Load Modules
# ============================================================================

# Core utilities (colors, logging, paths) - must be first
source "${SCRIPT_DIR}/lib/common.sh"

# Integrity verification - loaded early, runs environment sanitization immediately
# This cannot be bypassed - environment.sh always runs
source "${SCRIPT_DIR}/lib/integrity/init.sh"

# Ollama management
source "${SCRIPT_DIR}/lib/ollama/service.sh"
source "${SCRIPT_DIR}/lib/ollama/lifecycle.sh"

# Agent repository
source "${SCRIPT_DIR}/lib/agents/sync.sh"

# Yollayah personality
source "${SCRIPT_DIR}/lib/yollayah/personality.sh"

# User experience
source "${SCRIPT_DIR}/lib/ux/terminal.sh"

# User customizations (privacy-first, mostly placeholders)
source "${SCRIPT_DIR}/lib/user/init.sh"

# ============================================================================
# Main Entry Point
# ============================================================================

main() {
    clear
    ux_print_banner

    # Verify script integrity FIRST (before any other operations)
    # Environment sanitization already ran when integrity module was sourced
    integrity_verify || exit 1

    ux_show_startup_progress

    # Check Ollama is installed
    ollama_check_installed || exit 1

    # Record pre-Yollayah state (for cleanup)
    ollama_record_state

    # Register cleanup handler
    ollama_register_cleanup

    # Ensure Ollama is running
    ollama_ensure_running || exit 1

    # Select and pull best model for hardware
    model_select_best
    model_ensure_ready || exit 1

    # Sync agents repository (the breadcrumb to YOU.md)
    agents_sync

    # Create Yollayah personality model
    yollayah_create_model || exit 1

    # Initialize user module (no-op if no data)
    user_init

    # Ready!
    ux_show_all_ready

    # Start the conversation
    ux_conversation_loop "$YOLLAYAH_MODEL_NAME"
}

# ============================================================================
# Run
# ============================================================================

main "$@"
