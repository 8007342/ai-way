# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with ai-way.

---

## Project Overview

**ai-way**: Privacy-first local AI appliance for Average Joe (AJ)

**Entry Point**: `./yollayah.sh` - The single, unequivocal entry point

**Platform**: Fedora Silverblue (with toolbox) - tested and developed
**Tech Stack**: Rust (TUI + Conductor) + Bash (yollayah.sh + lib modules)
**License**: AGPL-3.0

---

## Directory Structure

```
ai-way/
├── LICENSE              # AGPL-3.0
├── .gitignore           # Git ignore rules
├── README.md            # Project documentation
├── yollayah.sh          # SINGLE POINT OF ENTRY (wrapper script)
├── yollayah/            # EVERYTHING ELSE GOES HERE
│   ├── conductor/       # Conductor Rust project (with Cargo.toml)
│   ├── core/            # Core code components
│   │   └── surfaces/    # User-facing interfaces
│   │       ├── tui/     # TUI Rust project (with Cargo.toml)
│   │       │   └── TODO-TUI.md
│   │       └── bash/    # Bash fallback surface
│   │           ├── TODO-bash-minimal-fallback.md
│   │           └── TODO-move-bash-fallback-to-bash-module.md
│   ├── shared/          # Shared components across surfaces
│   │   └── yollayah/    # Yollayah-specific shared resources
│   │       ├── proto/   # Protobufs, animations, graphics
│   │       │   └── sprites/
│   │       │       └── TODO-sprites-init.md
│   │       ├── mood/    # Mood system for avatar animations
│   │       │   └── TODO-coherent-evolving-mood-system-init.md
│   │       └── cache/   # Animation caching
│   │           └── TODO-animation-cache-init.md
│   ├── lib/             # Bash modules
│   ├── scripts/         # Build and setup scripts
│   └── tests/           # Integration tests
├── knowledge/           # Immutable methodology knowledge
├── agents/              # AI agent profiles, Constitution (foreign tracked)
├── workdir/             # Runtime generated files (logs, cache)
│   ├── README.md        # "You can safely delete" message
│   └── logs/            # Runtime logs
├── progress/            # TODO-xxx and DONE-xxx tracking files (flat)
└── facts/               # Project-specific knowledge
    └── design/          # Design documents
```

**Key Principles**:
- **Root is clean**: Only entry point and top-level organization dirs
- **Rust is NOT top-level**: Rust projects live in `yollayah/conductor/` and `yollayah/core/surfaces/tui/`
- **Cargo files in components**: Each Rust project has its own Cargo.toml, NOT at root
- **progress/ is flat**: Only TODO-xxx.md or DONE-xxx.md files, no subdirs
- **TODO files WITH code**: Component-specific TODOs live with their components

---

## Foundational Principles

**Read `agents/CONSTITUTION.md` first** when working on ai-way. It defines the Five Laws of Evolution and Four Protections that govern all development decisions.

### Core Documents

| Document | Purpose |
|----------|---------|
| [`agents/CONSTITUTION.md`](agents/CONSTITUTION.md) | **Immutable ethical principles** - read first |
| [`knowledge/project/AI-WAY.md`](knowledge/project/AI-WAY.md) | Project philosophy, vision, values |
| [`knowledge/KNOWLEDGE.md`](knowledge/KNOWLEDGE.md) | Knowledge base index |
| [`progress/TODO-AI-WAY.md`](progress/TODO-AI-WAY.md) | Main project tracker (will become DONE-AI-WAY.md!) |

---

## Primary User: Average Joe (AJ)

All ai-way work serves AJ (defined in `agents/personas/average-joe.md`):
- Small business owner with minimal tech knowledge
- Needs privacy but doesn't understand implementation
- Expects apps to "just work" with zero configuration
- Gaming laptop with mid-range GPU

**Use the terminology dictionary** (`agents/ai-way-docs/terminology-dictionary.md`) for user-facing content. Never expose technical jargon to AJ.

---

## Development

### Build Commands

```bash
# Main entry point
./yollayah.sh              # Full experience
./yollayah.sh --test       # Fast startup for testing (qwen2:0.5b model)

# Build diagnostics
./yollayah/yollayah-build-logs.sh --all         # Full workspace with verbose logs
./yollayah/yollayah-build-logs.sh --tui         # TUI only
./yollayah/yollayah-build-logs.sh --conductor   # Conductor only

# Manual Rust builds
cd yollayah/core/surfaces/tui && cargo build --release
cd yollayah/conductor && cargo build --release
```

### Test Mode

Fast startup for development/testing:
```bash
./yollayah.sh --test
# - Uses qwen2:0.5b (352MB, fast inference)
# - Skips non-essential operations
# - Launches in < 5 seconds (< 30s first run with model download)
# - Shows verbose Ollama logs (GPU/CUDA initialization)
```

### Toolbox Mode (Fedora Silverblue)

On Silverblue, ai-way automatically runs inside a toolbox container:
```bash
# Auto-enter behavior (automatic)
./yollayah.sh              # Automatically enters ai-way toolbox

# Manual toolbox management
toolbox create ai-way      # Create container
toolbox enter ai-way       # Enter container
toolbox rm ai-way          # Remove container (clean uninstall)
```

**See**: `knowledge/platform/TOOLBOX.md` for details

---

## Architecture

### yollayah/ Directory Organization

- **conductor/**: Independent Rust project, owns conversation state
- **core/surfaces/**: User interfaces (TUI, bash fallback)
  - **tui/**: Ratatui-based TUI, async, responsive
  - **bash/**: Fallback interface when TUI fails
- **shared/yollayah/**: Shared resources (proto, mood, cache)
- **lib/**: Bash modules sourced by yollayah.sh
- **scripts/**: Build and setup utilities
- **tests/**: Integration and architectural enforcement tests

### Async/Non-Blocking Philosophy

**HARD REQUIREMENTS**:
- **Conductor**: Fully async (concurrent models, parallel requests)
- **TUI**: Fully async (responsive, non-blocking UI)
- **All Surfaces**: Must be async
- **yollayah.sh**: Bootstrap wrapper - simple sync is OK

**Core Principles**:
1. **No Sleep, Only Wait on Async I/O** - Never poll, never sleep
2. **No Blocking I/O** - Use tokio::fs, tokio::net, not std::fs, std::net
3. **Surfaces Are Thin Clients** - Negligible performance impact

**See**: `knowledge/principles/PRINCIPLE-efficiency.md` for details

### TUI/Conductor Separation

**The Rule**: TUI ≠ Conductor

- No direct dependencies between TUI and Conductor
- Conductor compiles without TUI dependency
- Communication via messages only
- State belongs to Conductor, not surfaces
- Swappable surfaces (TUI, web, CLI, headless)

**See**: `knowledge/requirements/REQUIRED-separation.md` for details

---

## Knowledge Base

### `knowledge/` - Immutable Knowledge

**Characteristics**:
- Mostly static, changes are rare
- High-trust updates only (Architect role)
- Defines "how we work"
- De-yollayah-ized, de-ai-way-ized (general methodology)

**Structure**:
```
knowledge/
├── project/          # Project philosophy (AI-WAY.md)
├── methodology/      # TODO-driven development, DONE easter egg
├── principles/       # PRINCIPLE-efficiency, PRINCIPLE-data-flow
├── requirements/     # REQUIRED-separation
├── anti-patterns/    # FORBIDDEN-inefficient-calculations
├── team/             # Agent specializations and associations
├── platform/         # Platform-specific guides (TOOLBOX.md)
└── troubleshooting/  # Operational guides
```

### `facts/` - Project-Specific Knowledge

**Characteristics**:
- Domain-specific to ai-way (not general methodology)
- Design constraints, technical decisions
- Can change more frequently than knowledge/

**Structure**:
```
facts/
└── design/           # Design documents and constraints
    └── yollayah-avatar-constraints.md
```

### `progress/` - Iterative Progress

**Characteristics**:
- Highly dynamic, changes every sprint/session
- **FLAT STRUCTURE**: Only TODO-xxx.md or DONE-xxx.md files
- No subdirectories like "active", "completed", "bugs"
- All tracking files at top level

**Naming Conventions**:
- `TODO-xxx.md` - Active work
- `DONE-xxx.md` - Completed work (renamed from TODO-xxx.md)
- `EPIC-xxx.md`, `STORY-xxx.md`, `BUG-xxx.md` - Specific types
- `PERFORMANCE-AUDIT-xxx.md`, `ODYSSEY-xxx.md` - Analysis

**Component TODOs**: Some TODO files live WITH their components:
- `yollayah/core/surfaces/tui/TODO-TUI.md` - TUI central tracking
- `yollayah/core/surfaces/bash/TODO-bash-minimal-fallback.md`
- `yollayah/shared/yollayah/proto/sprites/TODO-sprites-init.md`
- `yollayah/shared/yollayah/mood/TODO-coherent-evolving-mood-system-init.md`
- `yollayah/shared/yollayah/cache/TODO-animation-cache-init.md`

---

## TODO-Driven Development

We use an iterative, tracked approach:

1. **EPICs** - Major features (weeks/months)
2. **Sprints** - Time-boxed work (days/weeks)
3. **Stories** - User-facing features (hours/days)
4. **Tasks** - Individual work items (minutes/hours)

**The Sweet Easter Egg** 🎉:
When a `TODO-xyz` is 100% complete:
1. Rename to `DONE-xyz` IN PLACE
2. Update references in related files

**Ultimate Goal**: `TODO-AI-WAY.md` → `DONE-AI-WAY.md` (when ai-way ships!)

**See**: `knowledge/methodology/TODO-DRIVEN-METHODOLOGY.md` for details

---

## Team Structure

When you need expertise, reference the appropriate team:

| Team | When to Use |
|------|-------------|
| **Rust & Ratatui** | "Have the Rust team review...", TUI/Conductor code |
| **LLM & Ollama** | "Have the LLM specialists optimize...", backend integration |
| **UX & Security** | "Have the UX team validate...", "Have the hacker review..." |
| **Architect** | Architecture decisions, principle updates |

**See**: `knowledge/team/TEAM.md` for details

---

## Architectural Enforcement

**Pre-commit hooks** run integration tests that enforce:
- ✅ No sleep() calls in production code
- ✅ No blocking I/O in async code
- ✅ All workspace tests pass

**Tests**: `yollayah/tests/architectural-enforcement/`

**Skip for .md-only changes** (performance optimization)

---

## Common Tasks

### Commit Changes

```bash
git add <files>
git commit -m "Short description

Detailed changes:
- [bullet points]

Related: [link to TODO/BUG]

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

### Create a New TODO

For progress tracking:
```bash
# Create in progress/ (flat)
vim progress/TODO-###-description.md
```

For component tracking:
```bash
# Create WITH the component
vim yollayah/core/surfaces/tui/TODO-new-feature.md
```

### Complete a TODO

```bash
# When 100% complete, rename IN PLACE:
git mv progress/TODO-xyz.md progress/DONE-xyz.md
git commit -m "Complete xyz - rename TODO to DONE 🎉"
```

---

## Threat Research

**Read `agents/dangers/` before working on ai-way security features.**

Key documents:
- `AGENT_FINGERPRINTING.md` - Behavioral identification risks
- `DATA_LEAKS.md` - Exfiltration vectors
- `CORRELATION_ATTACKS.md` - De-anonymization through linking
- `SUPPLY_CHAIN_RISKS.md` - Model and dependency security
- `THE_HUMAN_FACTOR.md` - Social engineering, user error

Core insight: *"We cannot make AJ invisible. We can make attacks expensive."*

---

## Sandboxed Claude Code Setup

This workspace runs inside a distrobox jail on Fedora Silverblue:
- Container home is `~/src` only
- Immutable host system (ostree)
- SELinux enforcing

**See**: `agents/clients/CLAUDE.md` for full sandboxing configuration

---

## Getting Help

- `/help` - Get help with Claude Code
- **Feedback**: Report issues at https://github.com/anthropics/claude-code/issues
- **Project Issues**: File in ai-way repository

---

**Remember**: This is about building the AI that AJ can trust. Privacy is not a feature. It's the promise.
