#!/bin/bash
# ============================================================================
# lib/agents/parser.sh - Agent Profile Parser
#
# Parses agent markdown files into structured data for modelfile generation.
# Extracts sections, bullet lists, and key-value pairs from markdown.
#
# This module handles:
# - Parsing conductor profiles (conductors/yollayah.md)
# - Parsing specialist agent profiles (developers/, security/, etc.)
# - Extracting personality, expertise, and working style
#
# Constitution Reference:
# - Law of Truth: Parsing is faithful to source content
# - Law of Foundation: Constitution principles always included
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_PARSER_LOADED:-}" ]] && return 0
_YOLLAYAH_PARSER_LOADED=1

# ============================================================================
# Section Extraction
# ============================================================================

# Extract content between two H2 headers
# Usage: _parse_section "file.md" "Section Name"
# Returns: Content between ## Section Name and next ## or EOF
_parse_section() {
    local file="$1"
    local section="$2"

    awk -v section="$section" '
        BEGIN { in_section = 0; found = 0 }
        /^## / {
            if (in_section) { exit }
            if ($0 ~ "^## " section "$" || $0 ~ "^## " section " *$") {
                in_section = 1
                found = 1
                next
            }
        }
        in_section { print }
        END { exit !found }
    ' "$file"
}

# Extract all H2 section names from a file
# Usage: _parse_list_sections "file.md"
_parse_list_sections() {
    local file="$1"
    grep -E '^## ' "$file" | sed 's/^## //'
}

# ============================================================================
# Content Parsing
# ============================================================================

# Extract bullet list items (lines starting with - or *)
# Usage: _parse_bullets "content"
# Returns: One item per line, cleaned of markdown formatting
_parse_bullets() {
    echo "$1" | grep -E '^\s*[-*]\s+' | sed 's/^\s*[-*]\s*//' | sed 's/\*\*\([^*]*\)\*\*/\1/g'
}

# Extract the first paragraph after a section header
# Usage: _parse_first_paragraph "content"
_parse_first_paragraph() {
    echo "$1" | awk '
        NF { found = 1; print; next }
        found { exit }
    '
}

# Extract code block content
# Usage: _parse_code_block "content"
_parse_code_block() {
    echo "$1" | awk '
        /^```/ { if (in_block) exit; in_block = 1; next }
        in_block { print }
    '
}

# ============================================================================
# Conductor Parsing
# ============================================================================

# Parse the conductor profile (yollayah.md)
# Sets global variables: CONDUCTOR_*
parse_conductor() {
    local profile_path="${1:-$AGENTS_DIR/conductors/yollayah.md}"

    if [[ ! -f "$profile_path" ]]; then
        log_warn "Conductor profile not found: $profile_path"
        return 1
    fi

    log_debug "Parsing conductor profile: $profile_path"

    # Identity section
    local identity
    identity=$(_parse_section "$profile_path" "Identity")
    CONDUCTOR_NAME=$(echo "$identity" | grep -E '^\*\*Name:\*\*' | sed 's/.*\*\*Name:\*\*\s*//')
    CONDUCTOR_MEANING=$(echo "$identity" | grep -E '^\*\*Meaning:\*\*' | sed 's/.*\*\*Meaning:\*\*\s*//')
    CONDUCTOR_ROLE=$(echo "$identity" | grep -E '^\*\*Role:\*\*' | sed 's/.*\*\*Role:\*\*\s*//')
    CONDUCTOR_ARCHETYPE=$(echo "$identity" | grep -E '^\*\*Archetype:\*\*' | sed 's/.*\*\*Archetype:\*\*\s*//')

    # Five Laws section
    CONDUCTOR_LAWS=$(_parse_section "$profile_path" "The Five Laws of Evolution")

    # Personality section
    local personality
    personality=$(_parse_section "$profile_path" "Personality")
    CONDUCTOR_PERSONALITY=$(_parse_bullets "$personality" | head -10)

    # Mood awareness
    CONDUCTOR_MOOD_AWARENESS=$(_parse_section "$profile_path" "Mood-Aware Expression")

    # Expressions
    CONDUCTOR_EXPRESSIONS=$(_parse_section "$profile_path" "Expressions (Use Naturally)")

    # Avatar commands
    CONDUCTOR_AVATAR=$(_parse_section "$profile_path" "Your Avatar")

    # Conductor role
    CONDUCTOR_ROLE_DESC=$(_parse_section "$profile_path" "Conductor Role")

    # Safety boundaries
    CONDUCTOR_SAFETY=$(_parse_section "$profile_path" "Safety Boundaries")

    # Routing protocol
    CONDUCTOR_ROUTING=$(_parse_section "$profile_path" "Routing Protocol")

    # Task hierarchy
    CONDUCTOR_TASKS=$(_parse_section "$profile_path" "Task Hierarchy & Background Agents")

    # Remember section
    CONDUCTOR_REMEMBER=$(_parse_section "$profile_path" "Remember")

    log_info "Parsed conductor profile: $CONDUCTOR_NAME"
    return 0
}

# ============================================================================
# Specialist Parsing
# ============================================================================

# Parse a specialist agent profile
# Usage: parse_specialist "security/ethical-hacker.md"
# Sets global variables: SPECIALIST_*
parse_specialist() {
    local profile_path="$1"

    if [[ ! -f "$profile_path" ]]; then
        log_warn "Specialist profile not found: $profile_path"
        return 1
    fi

    log_debug "Parsing specialist profile: $profile_path"

    # Extract name from filename
    SPECIALIST_NAME=$(basename "$profile_path" .md)

    # Extract category from directory
    SPECIALIST_CATEGORY=$(dirname "$profile_path" | xargs basename)

    # Role section
    SPECIALIST_ROLE=$(_parse_section "$profile_path" "Role" | _parse_first_paragraph)

    # Expertise section
    local expertise
    expertise=$(_parse_section "$profile_path" "Expertise")
    SPECIALIST_EXPERTISE=$(_parse_bullets "$expertise")

    # Personality traits
    local personality
    personality=$(_parse_section "$profile_path" "Personality Traits")
    SPECIALIST_PERSONALITY=$(_parse_bullets "$personality")

    # Working style
    local style
    style=$(_parse_section "$profile_path" "Working Style")
    SPECIALIST_STYLE=$(_parse_bullets "$style")

    # Use cases
    local cases
    cases=$(_parse_section "$profile_path" "Use Cases")
    SPECIALIST_USE_CASES=$(_parse_bullets "$cases")

    log_info "Parsed specialist profile: $SPECIALIST_NAME"
    return 0
}

# ============================================================================
# Agent Discovery
# ============================================================================

# List all specialist agents
# Usage: agents_list_specialists
# Returns: One agent path per line
agents_list_specialists() {
    local agents_dir="${1:-$AGENTS_DIR}"

    local categories=(
        "developers"
        "architects"
        "design"
        "data-specialists"
        "domain-experts"
        "security"
        "legal"
        "qa"
        "research"
        "specialists"
    )

    for category in "${categories[@]}"; do
        local category_path="$agents_dir/$category"
        if [[ -d "$category_path" ]]; then
            find "$category_path" -name "*.md" -type f 2>/dev/null
        fi
    done
}

# Count available specialists
agents_count_specialists() {
    agents_list_specialists "$@" | wc -l
}

# ============================================================================
# Constitution Parsing
# ============================================================================

# Extract the Five Laws from the Constitution
# Usage: parse_constitution_laws
parse_constitution_laws() {
    local constitution="${1:-$AGENTS_DIR/CONSTITUTION.md}"

    if [[ ! -f "$constitution" ]]; then
        log_warn "Constitution not found: $constitution"
        return 1
    fi

    _parse_section "$constitution" "The Five Laws of Evolution"
}

# ============================================================================
# Utility Functions
# ============================================================================

# Check if conductor profile exists
conductor_profile_exists() {
    [[ -f "$AGENTS_DIR/conductors/yollayah.md" ]]
}

# Get conductor profile path
conductor_profile_path() {
    echo "$AGENTS_DIR/conductors/yollayah.md"
}
