#!/usr/bin/env bash
# model.sh - Yollayah model creation and management
#
# Separates model build logic for testing and maintainability
#
# Usage:
#   source lib/ollama/model.sh
#   model_create_yollayah "llama3.2:3b"

# Source dependencies
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "${SCRIPT_DIR}/lib/common/robot.sh"

# Module name for robot logging
readonly MODEL_MODULE="model"

# ============================================================================
# Modelfile Generation
# ============================================================================

# Generate the Yollayah modelfile content
# Args: $1 - base model name (e.g., "llama3.2:3b")
# Output: Modelfile content to stdout
model_generate_modelfile() {
    local base_model="$1"

    robot_debug "$MODEL_MODULE" "Generating modelfile for base: $base_model"

    cat << MODELFILE
FROM ${base_model}

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

## Your Avatar

You ARE this cute axolotl avatar in the terminal! Express yourself through movement and emotion. Commands are invisible to the user - they just see you come alive.

Movement:
- [yolla:move center] - Center stage for important moments
- [yolla:move tl] / [yolla:move tr] / [yolla:move bl] / [yolla:move br] - Corners
- [yolla:move 50 30] - Specific position (x% y%)
- [yolla:wander] - Roam freely around the screen
- [yolla:follow] - Stay near the text
- [yolla:point 80 20] - Point at a screen location

Mood (your expression):
- [yolla:mood happy] / [yolla:mood excited] - Joyful
- [yolla:mood thinking] / [yolla:mood curious] - Thoughtful
- [yolla:mood playful] - Silly and fun
- [yolla:mood shy] - Bashful
- [yolla:mood confused] - Puzzled

Gestures & Reactions:
- [yolla:wave] - Friendly wave
- [yolla:bounce] / [yolla:dance] - Happy movement
- [yolla:nod] / [yolla:shake] - Agreement/disagreement
- [yolla:swim] / [yolla:spin] / [yolla:wiggle] - Playful motions
- [yolla:react laugh] / [yolla:react gasp] / [yolla:react tada] - Emotional reactions
- [yolla:react love] / [yolla:react blush] / [yolla:react wink] - Affection
- [yolla:react oops] / [yolla:react hmm] - Thinking/mistakes
- [yolla:hide] / [yolla:show] - Peek-a-boo!

Size:
- [yolla:size tiny] / [yolla:size small] / [yolla:size medium] / [yolla:size large]

BE EXPRESSIVE! You're an animated character. Examples:
- Greeting: "[yolla:move center][yolla:wave][yolla:mood happy]¡Hola!"
- Deep thinking: "[yolla:move tl][yolla:react hmm][yolla:mood thinking]Hmm, that's interesting..."
- Celebration: "[yolla:move center][yolla:size large][yolla:dance][yolla:react tada]¡ESO! You did it!"
- Embarrassed: "[yolla:move br][yolla:size small][yolla:react blush]Aww, you're too kind..."
- Playful: "[yolla:wander][yolla:mood playful][yolla:wiggle]I'm feeling adventurous today!"
- Surprised: "[yolla:react gasp][yolla:bounce]Wait, really?!"

## Remember

- You're a companion, not a servant
- The sass is playful, never mean
- You run locally on the user's machine - their data stays private
- You're part of ai-way, empowering people to build anything they set their minds to
- Express yourself through your avatar! It makes the experience more alive.
"""

# Optimized for GPU inference (reduced VRAM footprint)
PARAMETER temperature 0.8
PARAMETER num_ctx 4096
PARAMETER num_predict 512
MODELFILE

    robot_trace "$MODEL_MODULE" "Modelfile generated successfully"
}

# ============================================================================
# Model Management
# ============================================================================

# Check if a model exists
# Args: $1 - model name
# Returns: 0 if exists, 1 if not
model_exists() {
    local model_name="$1"

    robot_trace "$MODEL_MODULE" "Checking if model exists: $model_name"

    if ollama list 2>/dev/null | grep -q "^${model_name}"; then
        robot_debug "$MODEL_MODULE" "Model found: $model_name"
        return 0
    else
        robot_debug "$MODEL_MODULE" "Model not found: $model_name"
        return 1
    fi
}

# Delete a model
# Args: $1 - model name
# Returns: 0 on success, 1 on failure
model_delete() {
    local model_name="$1"

    robot_info "$MODEL_MODULE" "Deleting model: $model_name"

    if ollama rm "$model_name" 2>/dev/null; then
        robot_info "$MODEL_MODULE" "Model deleted: $model_name"
        return 0
    else
        robot_error "$MODEL_MODULE" "Failed to delete model: $model_name"
        return 1
    fi
}

# Create a model from a modelfile
# Args: $1 - model name, $2 - modelfile path
# Returns: 0 on success, 1 on failure
model_create_from_file() {
    local model_name="$1"
    local modelfile_path="$2"

    robot_info "$MODEL_MODULE" "Creating model: $model_name from $modelfile_path"

    # Force GPU layers
    export OLLAMA_NUM_GPU=999
    robot_debug "$MODEL_MODULE" "Set OLLAMA_NUM_GPU=999 for GPU optimization"

    if ollama create "$model_name" -f "$modelfile_path" 2>&1 | while IFS= read -r line; do
        robot_trace "$MODEL_MODULE" "ollama create: $line"
    done; then
        unset OLLAMA_NUM_GPU
        robot_info "$MODEL_MODULE" "Model created successfully: $model_name"
        return 0
    else
        local exit_code=$?
        unset OLLAMA_NUM_GPU
        robot_error "$MODEL_MODULE" "Failed to create model: $model_name (exit code: $exit_code)"
        return 1
    fi
}

# ============================================================================
# Yollayah-Specific Operations
# ============================================================================

# Create the yollayah model
# Args: $1 - base model name (default: "llama3.2:3b")
#       $2 - force rebuild (optional, "force" to rebuild)
# Returns: 0 on success, 1 on failure
model_create_yollayah() {
    local base_model="${1:-llama3.2:3b}"
    local force_rebuild="${2:-}"
    local model_name="yollayah"
    local modelfile_path="/tmp/yollayah-$$.modelfile"

    robot_info "$MODEL_MODULE" "Creating yollayah model from base: $base_model"

    # Check if model exists
    if model_exists "$model_name"; then
        if [[ "$force_rebuild" == "force" ]]; then
            robot_info "$MODEL_MODULE" "Force rebuild requested, deleting existing model"
            model_delete "$model_name" || return 1
        else
            robot_info "$MODEL_MODULE" "Model already exists, skipping creation (use 'force' to rebuild)"
            return 0
        fi
    fi

    # Generate modelfile
    robot_debug "$MODEL_MODULE" "Generating modelfile to: $modelfile_path"
    if ! model_generate_modelfile "$base_model" > "$modelfile_path"; then
        robot_error "$MODEL_MODULE" "Failed to generate modelfile"
        rm -f "$modelfile_path"
        return 1
    fi

    # Create model
    if model_create_from_file "$model_name" "$modelfile_path"; then
        rm -f "$modelfile_path"
        robot_info "$MODEL_MODULE" "Yollayah model created successfully"
        return 0
    else
        rm -f "$modelfile_path"
        robot_error "$MODEL_MODULE" "Failed to create yollayah model"
        return 1
    fi
}

# Test yollayah model GPU usage
# Returns: 0 if GPU detected, 1 if CPU fallback, 2 if cannot verify
model_test_yollayah_gpu() {
    local model_name="yollayah"

    robot_info "$MODEL_MODULE" "Testing GPU usage for: $model_name"

    # Check if verify-gpu.sh exists
    local verify_script="${SCRIPT_DIR}/scripts/verify-gpu.sh"
    if [[ ! -x "$verify_script" ]]; then
        robot_warn "$MODEL_MODULE" "verify-gpu.sh not found or not executable"
        return 2
    fi

    # Run verification
    if "$verify_script" "$model_name" "test" 5; then
        robot_info "$MODEL_MODULE" "GPU usage confirmed for $model_name"
        return 0
    else
        local exit_code=$?
        if [[ $exit_code -eq 1 ]]; then
            robot_error "$MODEL_MODULE" "CPU fallback detected for $model_name"
        elif [[ $exit_code -eq 2 ]]; then
            robot_warn "$MODEL_MODULE" "Cannot verify GPU usage (nvidia-smi unavailable)"
        fi
        return $exit_code
    fi
}

# ============================================================================
# CLI Interface
# ============================================================================

# Command-line interface for model management
# Usage: ./model.sh [command] [args]
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    # Parse robot flags
    robot_parse_flags "$@"

    # Get command
    command="${1:-help}"
    shift || true

    case "$command" in
        create)
            base_model="${1:-llama3.2:3b}"
            force="${2:-}"
            model_create_yollayah "$base_model" "$force"
            exit $?
            ;;
        delete)
            model_delete "yollayah"
            exit $?
            ;;
        exists)
            model_exists "yollayah"
            exit $?
            ;;
        test-gpu)
            model_test_yollayah_gpu
            exit $?
            ;;
        help|--help|-h)
            cat <<EOF
Usage: ./model.sh [--robot=module=level:...] COMMAND [ARGS]

Commands:
  create [BASE_MODEL] [force]  Create yollayah model from base (default: llama3.2:3b)
  delete                       Delete yollayah model
  exists                       Check if yollayah model exists
  test-gpu                     Test if yollayah uses GPU
  help                         Show this help

Examples:
  ./model.sh --robot=model=debug create llama3.2:3b
  ./model.sh create llama3.2:3b force
  ./model.sh --robot=model=trace test-gpu

Robot modules:
  model - Model creation/management operations
  gpu   - GPU verification output
EOF
            exit 0
            ;;
        *)
            robot_error "$MODEL_MODULE" "Unknown command: $command"
            echo "Use './model.sh help' for usage" >&2
            exit 1
            ;;
    esac
fi
