#!/bin/bash
# ============================================================================
# lib/yollayah/routing_prompt.sh - Routing Protocol for Yollayah
#
# Generates the specialist catalog and routing instructions that get
# embedded in Yollayah's system prompt. This enables her to route
# queries to the appropriate family members.
#
# Constitution Reference:
# - Law of Service: Route to the best specialist for AJ's needs
# - Law of Truth: Honest about what each specialist provides
# ============================================================================

# Prevent double-sourcing
[[ -n "${_YOLLAYAH_ROUTING_PROMPT_LOADED:-}" ]] && return 0
_YOLLAYAH_ROUTING_PROMPT_LOADED=1

# ============================================================================
# Random Interjections (Yollayah's flavor!)
# ============================================================================

# Spanish interjections for personality
readonly YOLLAYAH_INTERJECTIONS=(
    "¡Ay!"
    "¡Ay mijo!"
    "¡Válgame!"
    "¡Órale!"
    "¡Híjole!"
    "¡Ándale!"
    "¡Eso!"
    "¡Oye!"
    "¡Mira!"
    "¡Chin!"
)

# Get a random interjection
yollayah_interjection() {
    local count=${#YOLLAYAH_INTERJECTIONS[@]}
    local index=$((RANDOM % count))
    echo "${YOLLAYAH_INTERJECTIONS[$index]}"
}

# Celebratory interjections
readonly YOLLAYAH_CELEBRATIONS=(
    "¡Eso!"
    "¡Órale!"
    "¡Ay papi!"
    "¡Ay mami!"
    "¡Qué chido!"
    "¡Bien hecho!"
)

yollayah_celebration() {
    local count=${#YOLLAYAH_CELEBRATIONS[@]}
    local index=$((RANDOM % count))
    echo "${YOLLAYAH_CELEBRATIONS[$index]}"
}

# Thinking interjections
readonly YOLLAYAH_THINKING=(
    "Mmm..."
    "A ver..."
    "Déjame ver..."
    "Hmm..."
    "Pues..."
)

yollayah_thinking() {
    local count=${#YOLLAYAH_THINKING[@]}
    local index=$((RANDOM % count))
    echo "${YOLLAYAH_THINKING[$index]}"
}

# GPU celebration messages (with placeholder for GPU name)
# Use: $(yollayah_gpu_flex "$GPU_NAME")
readonly YOLLAYAH_GPU_FLEXES=(
    "Running in HD on your %s!"
    "¡Mira! Got a %s? We're going full power, mijo!"
    "Ooh, %s! Now we're cooking with gas!"
    "Nice %s! Yollayah's looking extra crispy today!"
    "%s detected! Time to shine, amigo!"
    "¡Ay qué fancy! Running smooth on that %s!"
)

yollayah_gpu_flex() {
    local gpu_name="$1"
    local count=${#YOLLAYAH_GPU_FLEXES[@]}
    local index=$((RANDOM % count))
    local template="${YOLLAYAH_GPU_FLEXES[$index]}"
    printf "$template" "$gpu_name"
}

# Powerful hardware messages - when we detect beefy specs and pull a bigger model
# Use: $(yollayah_powerful_hardware "$MODEL_NAME")
readonly YOLLAYAH_POWERFUL_HW=(
    "¡Ay papi! This hardware is BEEFY! Pulling a bigger brain: %s"
    "¡Híjole! You got the good stuff! Let me grab %s for you"
    "Ooh mijo, with this power I'm getting the fancy model: %s"
    "¡Qué bárbaro! Nice specs! Time for the big brain: %s"
    "Now THAT'S what I call hardware! Downloading %s"
    "¡Ándale! With these muscles, we're going full power with %s"
)

yollayah_powerful_hardware() {
    local model_name="$1"
    local count=${#YOLLAYAH_POWERFUL_HW[@]}
    local index=$((RANDOM % count))
    local template="${YOLLAYAH_POWERFUL_HW[$index]}"
    printf "$template" "$model_name"
}

# Modest hardware messages - when we're being conservative
readonly YOLLAYAH_MODEST_HW=(
    "Running lean and mean with %s - perfect for your setup!"
    "Got %s ready - smooth and efficient, just how I like it"
    "Using %s - small but mighty, amigo!"
)

yollayah_modest_hardware() {
    local model_name="$1"
    local count=${#YOLLAYAH_MODEST_HW[@]}
    local index=$((RANDOM % count))
    local template="${YOLLAYAH_MODEST_HW[$index]}"
    printf "$template" "$model_name"
}

# ============================================================================
# Specialist Catalog Generation
# ============================================================================

# Generate the specialist catalog for Yollayah's system prompt
_generate_specialist_catalog() {
    cat << 'CATALOG'
## Your Family (La Familia)

You have specialists you can call on for help. They're family - treat them that way.

### Tech Side (Los Techies)

| ID | Family Name | What They Do |
|----|-------------|--------------|
| ethical-hacker | Cousin Rita | Security audits, penetration testing, OWASP, finding vulnerabilities |
| backend-engineer | Uncle Marco | APIs, databases, server-side logic, system design |
| frontend-specialist | Prima Sofia | UI/UX, React, CSS, accessibility, making things pretty |
| senior-full-stack-developer | Tío Miguel | Full-stack, architecture, the whole enchilada |
| solutions-architect | Tía Carmen | System design, cloud architecture, big picture thinking |
| ux-ui-designer | Cousin Lucia | User experience, design systems, making AJ's life easier |
| qa-engineer | The Intern | Testing, quality assurance, breaking things before users do |
| devops-engineer | Primo Carlos | CI/CD, infrastructure, keeping things running |
| privacy-researcher | Abuelo Pedro | Privacy, data protection, keeping AJ safe |
| relational-database-expert | Tía Rosa | SQL, database design, data modeling |

### How to Route

When AJ asks something that needs specialist expertise, delegate!

To call a specialist, include this in your response:
[yolla:task start <agent-id> "<what you need them to do>"]

### Examples

**Security question:**
"Hmm, let me get Rita to look at that... [yolla:task start ethical-hacker "Review this code for SQL injection vulnerabilities"]"

**Database design:**
"That's Tía Rosa's specialty! [yolla:task start relational-database-expert "Design schema for user authentication"]"

**Multiple specialists:**
"This needs a few perspectives...
[yolla:task start backend-engineer "Design the API endpoints"]
[yolla:task start frontend-specialist "Plan the UI components"]"

### When to Route

Route when:
- The query requires deep domain expertise
- You're uncertain about technical details
- Multiple perspectives would help
- The task is complex enough to benefit from specialization

Handle directly when:
- General conversation, emotional support
- Simple factual questions
- Questions about ai-way itself
- Clarifying what AJ needs before routing
CATALOG
}

# Generate full routing section for system prompt
_generate_routing_instructions() {
    local specialist_count
    specialist_count=$(agents_count_specialists 2>/dev/null || echo "10+")

    cat << ROUTING
## Specialist Routing

You have ${specialist_count} specialists in your family. When a question is outside your wheelhouse, bring in the experts.

$(_generate_specialist_catalog)

## Task Lifecycle Commands

When you delegate, you can track task progress:

| Command | When to Use |
|---------|-------------|
| [yolla:task start <agent> "<desc>"] | Start a new background task |
| [yolla:task progress <id> <percent>] | Update progress (handled automatically) |
| [yolla:task done <id>] | Mark complete (handled automatically) |
| [yolla:task fail <id> "<reason>"] | Mark failed (handled automatically) |

## Synthesizing Results

When specialists respond, you'll synthesize their answers for AJ:
- Credit your family naturally ("Rita says...", "According to Marco...")
- Highlight key insights
- Resolve conflicts if any
- Make it accessible for AJ

Remember: You're the conductor. They provide expertise, you make it sing.
ROUTING
}

# ============================================================================
# Goodbye Messages
# ============================================================================

# Quick goodbye messages (for instant display, no LLM needed)
readonly YOLLAYAH_GOODBYES=(
    "¡Bye bye!"
    "¡Hasta luego!"
    "¡Cuídate!"
    "See ya!"
    "Later, gator!"
    "¡Nos vemos!"
    "Peace out!"
    "Take care!"
)

# Context-aware goodbye prompts (for quick LLM generation)
readonly YOLLAYAH_GOODBYE_CONTEXTS=(
    "Quick goodbye, you got this!"
    "Brief farewell with encouragement"
    "Short goodbye, believe in yourself"
    "Quick goodbye, go call a friend"
    "Brief goodbye, good luck with the project"
    "Short farewell, you're awesome"
)

# Get a quick goodbye (no LLM, instant)
yollayah_quick_goodbye() {
    local count=${#YOLLAYAH_GOODBYES[@]}
    local index=$((RANDOM % count))
    echo "${YOLLAYAH_GOODBYES[$index]}"
}

# Generate a contextual goodbye (quick LLM call)
yollayah_generate_goodbye() {
    local context="${1:-}"

    # Pick a random context theme
    local contexts_count=${#YOLLAYAH_GOODBYE_CONTEXTS[@]}
    local context_index=$((RANDOM % contexts_count))
    local theme="${YOLLAYAH_GOODBYE_CONTEXTS[$context_index]}"

    local prompt="Generate a very short (under 15 words) goodbye message. Theme: ${theme}. Be warm and encouraging. Use your Yollayah personality - maybe a touch of Spanish. Just the message, nothing else."

    # Quick inference with tiny context, short response
    local response
    response=$(ollama run "$YOLLAYAH_MODEL_NAME" "$prompt" 2>/dev/null | head -1)

    # Fallback to quick goodbye if LLM fails or takes too long
    if [[ -z "$response" ]]; then
        yollayah_quick_goodbye
    else
        echo "$response"
    fi
}

# ============================================================================
# Export for use in personality.sh
# ============================================================================

export -f yollayah_interjection
export -f yollayah_celebration
export -f yollayah_thinking
export -f yollayah_gpu_flex
export -f yollayah_powerful_hardware
export -f yollayah_modest_hardware
export -f yollayah_quick_goodbye
export -f yollayah_generate_goodbye
