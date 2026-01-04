#!/usr/bin/env bash
# ============================================================================
# lib/common.sh - Shared utilities for Yollayah
#
# This is a compatibility shim that sources all lib/common/ modules.
# NEW CODE SHOULD source specific modules from lib/common/ directly.
#
# Architecture:
# - lib/common/paths.sh   - Path configuration
# - lib/common/state.sh   - Global state variables
# - lib/common/output.sh  - Legacy output function wrappers
# - lib/common/utils.sh   - Utility functions
# - lib/common/robot.sh   - Robot flag system (--robot=module=level:...)
#
# Output Architecture:
# - log_* functions → Write to .logs/ (for PJ debugging)
# - ux_*  functions → Display to terminal (for AJ, hidden unless YOLLAYAH_DEBUG=1)
# - pj_*  functions → Debug display for PJ (only shown when YOLLAYAH_DEBUG=1)
# - info/success/warn/error → Legacy wrappers (use ux_* in new code)
#
# Debug Mode (YOLLAYAH_DEBUG=1):
# - pj_step()   → Show a step in progress
# - pj_cmd()    → Show command being run
# - pj_check()  → Show what's being checked
# - pj_result() → Show result of check/command
# - pj_found()  → Show something was found
# - pj_missing() → Show something wasn't found
#
# Constitution Reference:
# - Law of Truth: Logging is honest and transparent
# - Law of Foundation: Paths are predictable and secure
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_COMMON_LOADED:-}" ]] && return 0
_YOLLAYAH_COMMON_LOADED=1

# SCRIPT_DIR must be set by bootstrap before sourcing
if [[ -z "$SCRIPT_DIR" ]]; then
    echo "ERROR: SCRIPT_DIR must be set before sourcing common.sh" >&2
    exit 1
fi

# Source all common modules
readonly _COMMON_DIR="${SCRIPT_DIR}/yollayah/lib/common"

source "${_COMMON_DIR}/paths.sh"
source "${_COMMON_DIR}/state.sh"
source "${_COMMON_DIR}/output.sh"
source "${_COMMON_DIR}/utils.sh"
source "${_COMMON_DIR}/robot.sh"
