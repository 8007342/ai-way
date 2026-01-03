#!/bin/bash
# ============================================================================
# lib/routing/aggregator.sh - Result Collection and Synthesis
#
# This module handles:
# - Collecting outputs from completed specialist tasks
# - Detecting conflicts between specialists
# - Synthesizing a unified response through Yollayah
# - Formatting results for AJ
#
# Data Flow:
#   Task Outputs -> Collect -> Conflict Check -> Synthesize -> Response
#
# Constitution Reference:
# - Law of Truth: Honest synthesis, acknowledge conflicts
# - Law of Service: Present the most useful combined insight
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_AGGREGATOR_LOADED:-}" ]] && return 0
_YOLLAYAH_AGGREGATOR_LOADED=1

# ============================================================================
# Result Collection
# ============================================================================

# Collect results from multiple tasks
# Usage: aggregator_collect_results <task-id1> <task-id2> ...
# Returns: Combined results with metadata
aggregator_collect_results() {
    local task_ids=("$@")
    local results=""

    for task_id in "${task_ids[@]}"; do
        [[ -z "$task_id" ]] && continue

        local agent_id status output family_name
        agent_id=$(task_get_agent "$task_id")
        status=$(task_get_status "$task_id")
        output=$(task_get_output "$task_id")
        family_name=$(specialist_get_family_name "$agent_id")

        results+="━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
SPECIALIST: ${family_name} (${agent_id})
STATUS: ${status}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

${output}

"
    done

    echo "$results"
}

# Collect results as structured data (for programmatic use)
aggregator_collect_structured() {
    local task_ids=("$@")

    for task_id in "${task_ids[@]}"; do
        [[ -z "$task_id" ]] && continue

        local agent_id status output error description
        agent_id=$(task_get_agent "$task_id")
        status=$(task_get_status "$task_id")
        output=$(task_get_output "$task_id")
        error=$(task_get_error "$task_id")
        description=$(task_get_description "$task_id")

        cat << EOF
[RESULT]
task_id=${task_id}
agent=${agent_id}
family_name=$(specialist_get_family_name "$agent_id")
status=${status}
description=${description}
error=${error}
output_length=$(echo -n "$output" | wc -c)
[OUTPUT]
${output}
[/OUTPUT]
[/RESULT]

EOF
    done
}

# ============================================================================
# Conflict Detection
# ============================================================================

# Check if results contain potential conflicts
# Usage: aggregator_detect_conflicts "combined results"
# Returns: 0 if conflicts detected, 1 if no conflicts
aggregator_detect_conflicts() {
    local results="$1"

    # Simple heuristic: look for contradictory language patterns
    local conflict_patterns=(
        "however"
        "on the other hand"
        "alternatively"
        "disagree"
        "contrary"
        "but.*instead"
        "not.*but rather"
    )

    for pattern in "${conflict_patterns[@]}"; do
        if echo "$results" | grep -qiE "$pattern"; then
            return 0  # Conflict detected
        fi
    done

    return 1  # No conflicts
}

# Get conflict analysis from Yollayah
aggregator_analyze_conflicts() {
    local results="$1"

    local analysis_prompt="Analyze these specialist responses for conflicts or disagreements:

${results}

If there are conflicts:
1. Identify what they disagree about
2. Summarize each position briefly
3. Suggest which approach might be better and why

If no significant conflicts: Just say 'No major conflicts - specialists are aligned.'

Be concise."

    ollama run "$YOLLAYAH_MODEL_NAME" "$analysis_prompt" 2>/dev/null
}

# ============================================================================
# Result Synthesis
# ============================================================================

# Synthesize results into a unified Yollayah response
# Usage: aggregator_synthesize <original-query> <specialist-results> [original-yollayah-response]
# Returns: Synthesized response
aggregator_synthesize() {
    local original_query="$1"
    local specialist_results="$2"
    local original_response="${3:-}"

    # Count how many specialists responded
    local specialist_count
    specialist_count=$(echo "$specialist_results" | grep -c "^SPECIALIST:" || echo "0")

    # Build synthesis prompt
    local synthesis_prompt
    synthesis_prompt=$(cat << EOF
You are Yollayah, synthesizing results from your family of specialists.

## Original Question from AJ
${original_query}

## Your Initial Thoughts
${original_response}

## Specialist Responses
${specialist_results}

## Your Task
Synthesize these responses into a helpful, unified answer for AJ.

Guidelines:
1. Credit your family naturally (e.g., "Rita pointed out...", "According to Marco...")
2. Highlight the key insights from each specialist
3. If there are conflicts, acknowledge them and give your take
4. Use your personality - warm, real, playful sass when appropriate
5. End with clear, actionable advice

Remember: You're the conductor. You take their expertise and make it accessible for AJ.
EOF
)

    # Generate synthesis through Yollayah
    local synthesized
    synthesized=$(ollama run "$YOLLAYAH_MODEL_NAME" "$synthesis_prompt" 2>/dev/null)

    # Clean any lingering task commands from the synthesis
    synthesized=$(classifier_clean_all_commands "$synthesized")

    echo "$synthesized"
}

# ============================================================================
# Quick Synthesis (Single Specialist)
# ============================================================================

# When only one specialist responds, format their response
aggregator_synthesize_single() {
    local original_query="$1"
    local agent_id="$2"
    local specialist_output="$3"

    local family_name
    family_name=$(specialist_get_family_name "$agent_id")

    local synthesis_prompt
    synthesis_prompt=$(cat << EOF
You are Yollayah. You asked ${family_name} (${agent_id}) for help with this:

Question: ${original_query}

${family_name} said:
${specialist_output}

Now present this to AJ in your voice:
- Credit ${family_name} naturally
- Add your warm, helpful personality
- Make it actionable for AJ
- Keep it concise

Remember: You're translating expert advice for Average Joe.
EOF
)

    ollama run "$YOLLAYAH_MODEL_NAME" "$synthesis_prompt" 2>/dev/null
}

# ============================================================================
# Result Formatting
# ============================================================================

# Format results for display (without synthesis)
aggregator_format_results() {
    local task_ids=("$@")

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "           FAMILY CONSULTATION RESULTS           "
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""

    for task_id in "${task_ids[@]}"; do
        [[ -z "$task_id" ]] && continue

        local agent_id status output family_name
        agent_id=$(task_get_agent "$task_id")
        status=$(task_get_status "$task_id")
        output=$(task_get_output "$task_id")
        family_name=$(specialist_get_family_name "$agent_id")

        echo "┌─ ${family_name} (${agent_id})"
        echo "│  Status: ${status}"
        echo "├─────────────────────────────────────────────────"
        echo "$output" | sed 's/^/│  /'
        echo "└─────────────────────────────────────────────────"
        echo ""
    done
}

# ============================================================================
# Summary Generation
# ============================================================================

# Generate a brief summary of all results
aggregator_generate_summary() {
    local results="$1"

    local summary_prompt="Summarize these specialist responses in 2-3 bullet points:

${results}

Format:
• Key point 1
• Key point 2
• Key point 3 (if needed)

Be extremely concise."

    ollama run "$YOLLAYAH_MODEL_NAME" "$summary_prompt" 2>/dev/null
}
