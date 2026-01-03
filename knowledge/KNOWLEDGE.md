# AI-Way Knowledge Base

**Purpose**: Immutable knowledge system documenting how we work, what we value, and who we are.

---

## Organization

This directory contains the **mostly static** knowledge that defines ai-way:

```
knowledge/
├── KNOWLEDGE.md (this file)          # Knowledge system index
├── project/                          # Project philosophy and vision
├── methodology/                      # How we work (TODO-driven, etc.)
├── principles/                       # Design principles we follow
├── requirements/                     # Hard requirements we enforce
├── anti-patterns/                    # Forbidden practices
├── team/                             # Agent specializations and associations
├── platform/                         # Platform-specific guides (Silverblue, etc.)
└── troubleshooting/                  # Operational guides
```

---

## Core Documents

### Project Philosophy
- **[project/AI-WAY.md](project/AI-WAY.md)** - Vision, mission, privacy-first principles
  - The Five Laws of Evolution
  - The Four Protections
  - Average Joe → Privacy Joe journey

### Methodology
- **[methodology/TODO-DRIVEN-METHODOLOGY.md](methodology/TODO-DRIVEN-METHODOLOGY.md)** - Our development process
  - TODO → DONE pattern (the sweet easter egg!)
  - Epic, Sprint, Story, Task definitions
  - Task guidelines and best practices

### Principles
Design principles that guide all development:
- **[principles/PRINCIPLE-efficiency.md](principles/PRINCIPLE-efficiency.md)** - The Three Laws of Async Efficiency
- **[principles/PRINCIPLE-data-flow.md](principles/PRINCIPLE-data-flow.md)** - Streams over copies philosophy

### Requirements
Hard requirements that MUST be enforced:
- **[requirements/REQUIRED-separation.md](requirements/REQUIRED-separation.md)** - TUI/Conductor separation laws

### Anti-Patterns
Forbidden practices that are caught by enforcement tests:
- **[anti-patterns/FORBIDDEN-inefficient-calculations.md](anti-patterns/FORBIDDEN-inefficient-calculations.md)** - Sleep/blocking/wasteful patterns

### Team Structure
Agent specializations for "have the team..." references:
- **[team/TEAM.md](team/TEAM.md)** - Overview of agent associations
- **[team/rust-ratatui-team.md](team/rust-ratatui-team.md)** - TUI and Conductor specialists
- **[team/llm-ollama-team.md](team/llm-ollama-team.md)** - LLM and Ollama specialists
- **[team/ux-security-team.md](team/ux-security-team.md)** - UX and Security (hacker) specialists

### Platform Guides
Platform-specific documentation:
- **[platform/TOOLBOX.md](platform/TOOLBOX.md)** - Fedora Silverblue toolbox usage

### Troubleshooting
Operational guides for common issues:
- **[troubleshooting/TROUBLESHOOTING.md](troubleshooting/TROUBLESHOOTING.md)** - Common problems and solutions

---

## Characteristics of Knowledge Files

1. **Mostly Immutable** - Changes are rare and carefully reviewed
2. **High Trust Only** - Architect and senior roles update these
3. **Reference Documentation** - Consulted during development
4. **Defines "How We Work"** - Methodology, principles, team structure

---

## Knowledge vs Progress

| knowledge/ (Static) | progress/ (Dynamic) |
|---------------------|---------------------|
| **How we work** | **What we're doing** |
| **What we value** | **Current state** |
| **Who we are** | **Incremental changes** |
| Updated rarely | Updated every sprint/session |
| Architect changes | Team changes |
| Methodology definition | Work tracking |

---

## Using the Knowledge Base

### When writing code:
- **Check principles/** - Follow async efficiency, data flow patterns
- **Check requirements/** - Ensure TUI/Conductor separation
- **Check anti-patterns/** - Avoid forbidden practices

### When planning work:
- **Check methodology/** - Follow TODO-driven process
- **Check team/** - Know which agents to involve

### When documenting philosophy:
- **Check project/** - Align with AI-Way vision and values

### When troubleshooting:
- **Check platform/** - Platform-specific guides
- **Check troubleshooting/** - Common issues and solutions

---

## Adding to Knowledge Base

**Rare and Deliberate**:
1. New principle discovered? → Document in principles/
2. New requirement established? → Document in requirements/
3. New anti-pattern identified? → Document in anti-patterns/
4. Methodology improvement? → Update methodology/
5. New agent specialization? → Update team/

**Process**:
- Discuss with Architect
- Draft the document
- Review for consistency with existing knowledge
- Commit with clear rationale
- Update this index if needed

---

## The Sweet Easter Egg 🎉

When a `TODO-xyz` file in progress/ is **100% complete**:
1. Move to `progress/completed/`
2. Rename to `DONE-xyz`

**Ultimate Goal**: `progress/TODO-AI-WAY.md` → `progress/completed/DONE-AI-WAY.md`

When that happens, ai-way ships! 🚀

---

**Remember**: This is the knowledge that guides our journey. Protect it, refine it, but change it deliberately.
