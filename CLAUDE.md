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
├── agents/           # AI agent profiles and Constitution (core component)
├── knowledge/        # Immutable knowledge base (methodology, principles, team)
├── progress/         # Iterative progress tracking (TODOs, bugs, work logs)
├── src/              # All source code (FUTURE - not yet moved)
│   ├── conductor/    # Conductor Rust code
│   ├── tui/          # TUI Rust code
│   ├── lib/          # Bash modules
│   └── tests/        # Integration tests
├── yollayah.sh       # Single entry point
├── yollayah-build-log.sh  # Verbose build diagnostics
├── README.md         # Comprehensive project overview
├── CLAUDE.md         # This file
├── LICENSE           # AGPL-3.0
└── Cargo.toml        # Workspace manifest
```

**Note**: Code is still at root level (conductor/, tui/, lib/, tests/) - future reorganization will move to src/.

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
./yollayah-build-log.sh --all         # Full workspace with verbose logs
./yollayah-build-log.sh --tui         # TUI only
./yollayah-build-log.sh --conductor   # Conductor only
./yollayah-build-log.sh --surfaces    # All surfaces (currently just TUI)

# Manual Rust builds
cargo build --workspace
cargo test --workspace
cargo build --package yollayah-tui --release
cargo build --package conductor-core --release
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

### `progress/` - Iterative Progress

**Characteristics**:
- Highly dynamic, changes every sprint/session
- Tracks current state of work

**Structure**:
```
progress/
├── TODO-AI-WAY.md       # Main project tracker (will become DONE-AI-WAY.md!)
├── active/              # Active TODOs, EPICs, Stories
├── bugs/                # Bug tracking
├── completed/           # Completed work (TODO → DONE renames)
├── audits/              # Performance and architecture audits
├── design/              # Design explorations
├── odysseys/            # Long-term architectural journeys
└── work-logs/           # Session summaries
```

---

## TODO-Driven Development

We use an iterative, tracked approach:

1. **EPICs** - Major features (weeks/months)
2. **Sprints** - Time-boxed work (days/weeks)
3. **Stories** - User-facing features (hours/days)
4. **Tasks** - Individual work items (minutes/hours)

**The Sweet Easter Egg** 🎉:
When a `TODO-xyz` is 100% complete:
1. Move to `progress/completed/`
2. Rename to `DONE-xyz`

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

**Tests**: `tests/architectural-enforcement/`

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

```bash
# Create in progress/active/
vim progress/active/TODO-###-description.md

# Follow TODO template (see methodology/)
# Reference from TODO-AI-WAY.md
```

### Complete a TODO

```bash
# When 100% complete:
git mv progress/active/TODO-xyz.md progress/completed/DONE-xyz.md
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
