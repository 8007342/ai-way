#!/usr/bin/env bash
# state.sh - Global state variables for Yollayah
#
# This module provides shared state variables used across different modules.
# These are set by various modules and read by others.

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_STATE_LOADED:-}" ]] && return 0
_YOLLAYAH_STATE_LOADED=1

# ============================================================================
# Ollama State
#
# Set by: lib/ollama/service.sh
# Read by: yollayah.sh cleanup
# ============================================================================

OLLAMA_WAS_RUNNING=false
OLLAMA_SERVICE_WAS_ENABLED=false
OLLAMA_SERVICE_WAS_ACTIVE=false
WE_STARTED_OLLAMA=false

# ============================================================================
# Agents State
#
# Set by: lib/agents/sync.sh
# Read by: yollayah.sh for agent routing decisions
# ============================================================================

AGENTS_CHANGED=false

# ============================================================================
# Model State
#
# Set by: lib/ollama/lifecycle.sh
# Read by: lib/ollama/model.sh, yollayah.sh
# ============================================================================

SELECTED_MODEL=""
MODEL_NEEDS_UPDATE=false
