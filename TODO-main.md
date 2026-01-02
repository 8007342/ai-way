# TODO Tracking Index

Master index for all TODO tracking documents. Check here first to find pending work.

## Active Epics (2026-Q1)

| Epic | Status | Description |
|------|--------|-------------|
| [TODO-epic-2026Q1-avatar-animation.md](TODO-epic-2026Q1-avatar-animation.md) | **EXECUTION** | Avatar evolution, sprite generation, TUI animation loop |
| [TODO-epic-2026Q1-multi-surface.md](TODO-epic-2026Q1-multi-surface.md) | **EXECUTION** | Daemon architecture, multi-connection support, transport security |

## Active Refactors

| Document | Status | Description |
|----------|--------|-------------|
| [TODO-conductor-ux-split.md](TODO-conductor-ux-split.md) | **IN PROGRESS** | TUI/Conductor separation - see E-2026Q1-multi-surface |
| [TODO-avatar-animation-system.md](TODO-avatar-animation-system.md) | **IN PROGRESS** | Avatar animation details - see E-2026Q1-avatar-animation |
| [TODO-meta-agent-conductor-interactions.md](TODO-meta-agent-conductor-interactions.md) | **ACTIVE** | Meta-agent orchestration, agent delegation, avatar control |
| [TODO-implementation-plan.md](TODO-implementation-plan.md) | **ACTIVE** | Detailed implementation plan with dependencies |
| [TODO-qa-testing.md](TODO-qa-testing.md) | **PENDING** | Unit tests, integration tests, pre-commit hooks |
| [TODO-integration-testing.md](TODO-integration-testing.md) | **ACTIVE** | Integration test suite documentation |

## Enhancement Tracking

| Document | Status | Description |
|----------|--------|-------------|
| [TODO-accessibility.md](TODO-accessibility.md) | **ACTIVE** | Accessibility roadmap (WCAG compliance, TTS, high contrast) |
| ~~TODO-rich-ux.md~~ | SUPERSEDED | Original UX plan (now Rust-based, not Python Textual) - file removed |
| [agents/TODO.md](agents/TODO.md) | ONGOING | Agent profiles and future enhancements |
| [TODO-documentation.md](TODO-documentation.md) | ROLLING | Documentation updates needed for user docs |

## Specialist Reviews (2026-01-01)

Four specialist agents reviewed the project architecture:

| Specialist | Focus | Key Findings |
|------------|-------|--------------|
| Solutions Architect | Architecture gaps | Daemon binary needed, multi-surface refactor required |
| UX Designer | Multi-surface UX | ContentType, VoiceState, LayoutHint messages needed |
| Ethical Hacker | Security hardening | Auth tokens, session isolation, frame integrity |
| Mad Scientist | Edge cases & chaos | 5 critical edge cases, chaos test infrastructure needed |

Full findings integrated into [TODO-conductor-ux-split.md](TODO-conductor-ux-split.md).

## Avatar Architecture Refactor (2026-01-01)

Architect and UX specialist reviewed avatar system for extensibility:

| Area | Finding | Resolution |
|------|---------|------------|
| Animation System | Dual competing systems (state machine + engine) | Unified trait-based animation system |
| Frame Timing | Hard-coded ms in sprites | Frame-rate independent timing abstraction |
| Caching | None (recomputed every frame) | LRU cache with eviction |
| Alpha/Blending | Not supported | ColoredCell enhanced for future blending |
| Transitions | Instant state snaps | Smooth transition animations planned |
| Surface Portability | TUI-specific timing | Beat-based semantic timing format |

Key principle: **Meta-agent drives avatar entirely, surface is thin rendering layer.**

## TUI UX Enhancements (2026-01-01)

Deep usability review by UX Specialist, Architect, and Ethical Hacker:

### Quick Wins (Implementing)
| Feature | Status | Impact |
|---------|--------|--------|
| Scroll gradient indicators | PENDING | High - discoverability |
| Input history (Up/Down) | DONE | High - efficiency |
| Breathing color effect | PENDING | High - perceived aliveness |
| Micro-interactions | PENDING | Medium - tactile feedback |

### Planned Enhancements
| Feature | Effort | Impact |
|---------|--------|--------|
| Autocomplete (trie + context) | 3-4 hours | High |
| Focus management (Tab nav) | 3-5 hours | High |
| High contrast mode | 2-3 hours | Critical (a11y) |
| Reduced motion support | 1-2 hours | High (a11y) |
| RTL language support | Medium | Medium |
| Small screen layouts | Medium | Medium |

### Accessibility Status
- **Current WCAG compliance**: ~25% AA
- **Critical gaps**: Screen reader, keyboard nav, high contrast, colorblind
- **See**: [TODO-accessibility.md](TODO-accessibility.md) for full roadmap

## Principles

- Each major refactor/feature gets its own TODO-*.md file
- Mark items done in the relevant file, not this index
- Use feature creep items to capture scope expansion - don't block main refactor
- Commit stable checkpoints frequently to the refactor branch

---

**Last Updated**: 2026-01-02 (Epic planning session - added E-2026Q1-avatar-animation, E-2026Q1-multi-surface)
