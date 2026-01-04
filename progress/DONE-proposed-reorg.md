# TODO: Proposed Project Reorganization

**Created**: 2026-01-03
**Priority**: P0 - Critical Infrastructure
**Status**: 🟢 IN PROGRESS - Partially Complete

---

## Progress Update (2026-01-03)

### ✅ Completed
1. **knowledge/ directory structure created** (previous work)
   - All subdirectories in place: methodology/, principles/, requirements/, anti-patterns/, team/, platform/, troubleshooting/
   - KNOWLEDGE.md index file created
   - Reference files moved from reference/ to knowledge/

2. **progress/ directory structure created** (previous work)
   - All subdirectories in place: active/, bugs/, completed/, audits/, design/, odysseys/, work-logs/, deliverables/
   - TODO-AI-WAY.md main tracker created

3. **facts/ directory structure created** (2026-01-03)
   - facts/design/ created for project-specific design knowledge
   - docs/yollayah-avatar-constraints.md moved to facts/design/yollayah-avatar-constraints.md

4. **Init TODO files created** (2026-01-03)
   - progress/active/TODO-sprites-init.md - Sprite animation system
   - progress/active/TODO-coherent-evolving-mood-system-init.md - Mood-based animations
   - progress/active/TODO-animation-cache-init.md - Multi-layer caching system
   - progress/active/TODO-bash-minimal-fallback.md - Bash fallback interface
   - progress/active/TODO-move-bash-fallback-to-bash-module.md - Code organization

### 🔄 In Progress
- Updating TODO-proposed-reorg.md to reflect progress (this update)
- Updating CLAUDE.md with facts/ structure

### ⏳ Pending
- Move code to src/ directory (future work, not breaking current functionality)
- Update yollayah.sh paths if src/ reorganization happens
- Update README.md with new structure documentation

### ✅ Success Criteria Met
- ✅ Root directory clean (only 3 .md files: README.md, CLAUDE.md, TODO-proposed-reorg.md)
- ✅ yollayah.sh immediately visible and works
- ✅ knowledge/ structure complete
- ✅ progress/ structure complete
- ✅ facts/ structure created
- ✅ KNOWLEDGE.md created as index
- ✅ TODO-AI-WAY.md created as main project tracker
- ⏳ CLAUDE.md update pending
- ⏳ src/ reorganization deferred (not breaking current builds)

---

## Executive Summary

The ai-way root directory has become heavily polluted with 60+ markdown files, making it difficult to navigate and understand the project structure. This proposal reorganizes the project into a clear, logical structure that:

1. **Preserves all work** - No content loss, files are moved and renamed
2. **Clarifies intent** - Root dir shows only entry points and core components
3. **Separates concerns** - Knowledge (immutable) vs Progress (iterative)
4. **Follows conventions** - Standard project layout with `src/` for code

---

## Current State (Root Directory)

**Total files in root**: 60+ markdown files + 10+ directories

### Directories:
- `agents/` - AI agent profiles and Constitution
- `conductor/` - Conductor Rust code
- `tui/` - TUI Rust code
- `lib/` - Bash modules for yollayah.sh
- `scripts/` - Build and setup scripts
- `reference/` - Design principles (4 files)
- `workflows/` - Methodology docs (3 files)
- `docs/` - Additional documentation
- `tests/` - Integration tests
- `.github/` - GitHub Actions
- `.integrity/`, `.state/`, `.logs/` - Runtime artifacts

### Markdown Files (60+):
- `TODO-*.md` (30+ files) - Active tasks, sprints, epics
- `BUG-*.md` (6 files) - Bug tracking
- `EPIC-*.md`, `STORY-*.md` - Project planning
- `DESIGN-*.md` (8 files) - Design documents
- `PERFORMANCE-AUDIT-*.md` (3 files) - Audit reports
- `WORK-SUMMARY-*.md` (1 file) - Session summaries
- `ODYSSEY-*.md`, `PRINCIPLES.md`, etc - Misc docs
- `README.md`, `CLAUDE.md`, `TOOLBOX.md`, etc - Documentation

**Problem**: Can't find yollayah.sh in the noise!

---

## Proposed Structure

### Root Directory (Clean, Discoverable)

```
ai-way/
├── agents/           # Core component - Agent profiles, Constitution
├── src/              # All source code
├── knowledge/        # Immutable knowledge base (methodology, principles)
├── progress/         # Iterative progress tracking (TODOs, bugs, work logs)
├── yollayah.sh       # SINGLE POINT OF ENTRY
├── yollayah-build-log.sh  # Verbose build diagnostics
├── README.md         # Comprehensive project overview
├── CLAUDE.md         # Claude Code instructions
├── LICENSE           # Apache 2.0 license
├── Cargo.toml        # Workspace manifest
└── Cargo.lock        # Dependency lock
```

**Rationale**:
- Only 4 directories + 6 files visible at root
- Clear separation: code (src), knowledge (static), progress (dynamic), agents (core)
- yollayah.sh immediately visible as entry point
- GitHub-friendly (README, LICENSE at top)

---

## Detailed Proposed Structure

### 1. `src/` - Source Code

```
src/
├── conductor/        # Moved from ./conductor
│   ├── core/
│   ├── daemon/
│   └── tests/        # Integration tests for conductor
├── tui/              # Moved from ./tui
│   ├── src/
│   ├── tests/        # Integration tests for TUI
│   └── Cargo.toml
├── lib/              # Moved from ./lib (bash modules)
│   ├── agents/
│   ├── integrity/
│   ├── logging/
│   └── ...
├── scripts/          # Moved from ./scripts
│   ├── install-hooks.sh
│   ├── verify-gpu-toolbox.sh
│   └── ...
└── tests/            # Moved from ./tests
    └── architectural-enforcement/
```

**Changes**:
- All Rust code under `src/`
- Bash modules under `src/lib/`
- Scripts under `src/scripts/`
- Tests colocated with their components

---

### 2. `knowledge/` - Immutable Knowledge Base

```
knowledge/
├── KNOWLEDGE.md                    # Index - describes the system
├── project/
│   └── AI-WAY.md                   # Project philosophy, vision, guidelines
├── methodology/
│   ├── TODO-DRIVEN-METHODOLOGY.md  # Moved from workflows/todo-driven-development.md
│   ├── EPIC-definition.md          # Moved from workflows/epic.md
│   ├── SPRINT-definition.md        # Moved from workflows/sprint.md
│   ├── TASK-definition.md          # New - task definition
│   ├── TASK-guidelines.md          # New - task best practices
│   └── DONE-easter-egg.md          # Documents TODO→DONE rename pattern
├── principles/
│   ├── PRINCIPLE-efficiency.md     # Moved from reference/
│   ├── PRINCIPLE-data-flow.md      # Moved from reference/
│   └── PRINCIPLE-*.md              # Future principles
├── requirements/
│   ├── REQUIRED-separation.md      # Moved from reference/
│   └── REQUIRED-*.md               # Future requirements
├── anti-patterns/
│   ├── FORBIDDEN-inefficient-calculations.md  # Moved from reference/
│   └── FORBIDDEN-*.md              # Future anti-patterns
└── team/
    ├── TEAM.md                     # Agent associations and specializations
    ├── rust-ratatui-team.md        # TUI and Conductor specialists
    ├── llm-ollama-team.md          # LLM and Ollama specialists
    └── ux-security-team.md         # UX and Security (hacker) specialists
```

**Characteristics**:
- **Mostly immutable** - Changes are rare and carefully reviewed
- **High trust only** - Architect and senior roles
- **Defines "how we work"** - Methodology, principles, team structure
- **Reference documentation** - Consulted during development

---

### 3. `progress/` - Iterative Progress Tracking

```
progress/
├── TODO-AI-WAY.md                   # Main project tracker (will become DONE-AI-WAY.md)
├── active/                          # Active work
│   ├── TODO-017-framebuffer-optimization.md
│   ├── TODO-next.md
│   ├── TODO-main.md                 # Current sprint planning
│   ├── EPIC-*.md                    # Active epics
│   └── STORY-*.md                   # Active stories
├── bugs/                            # Bug tracking
│   ├── BUG-015-sleep-in-polling-loops.md  # (RESOLVED)
│   ├── BUG-016-config-test-failure.md
│   └── BUG-*.md                     # All bug reports
├── completed/                       # Completed work
│   ├── DONE-epic-001-ratatui-quick-wins.md  # Example when complete
│   └── DONE-*.md                    # Completed TODOs
├── audits/                          # Performance and architecture audits
│   ├── PERFORMANCE-AUDIT-ASYNC.md
│   ├── PERFORMANCE-AUDIT-FRAMEBUFFER.md
│   ├── PERFORMANCE-AUDIT-COMMS.md
│   └── ARCHITECTURE-REVIEW-*.md
├── design/                          # Design explorations
│   ├── DESIGN-loading-screen.md
│   ├── DESIGN-palette-rotation.md
│   └── DESIGN-*.md
├── odysseys/                        # Long-term architectural journeys
│   ├── ODYSSEY-tui-to-framebuffer.md
│   └── ODYSSEY-*.md
└── work-logs/                       # Session summaries
    ├── WORK-SUMMARY-2026-01-03.md
    └── WORK-SUMMARY-*.md
```

**Characteristics**:
- **Highly dynamic** - Changes every sprint/session
- **Tracks progress** - Current state of work
- **Easter egg**: When TODO-xxx is complete → move to `completed/` and rename to `DONE-xxx`
- **Organized by type** - active, bugs, completed, audits, design, work logs

---

### 4. `agents/` - Agent Profiles (No Change)

```
agents/
├── CONSTITUTION.md
├── personas/
├── clients/
├── dangers/
└── ...
```

**Rationale**: Core component, meant to be discoverable by AJ → PJ

---

## Special Files and Directories

### Where do these go?

| File/Dir | Current Location | Proposed Location | Rationale |
|----------|------------------|-------------------|-----------|
| **Build artifacts** |
| `.integrity/` | Root | Root | Runtime artifact, hidden |
| `.state/` | Root | Root | Runtime artifact, hidden |
| `.logs/` | Root | Root | Runtime artifact, hidden |
| `target/` | Root | Root | Cargo build output, in .gitignore |
| **GitHub integration** |
| `.github/` | Root | Root | GitHub Actions, must be at root |
| **Documentation** |
| `docs/` | Root | `knowledge/docs/` or delete if redundant | TBD - need to review contents |
| `TOOLBOX.md` | Root | `knowledge/platform/TOOLBOX.md` | Platform-specific docs |
| `TROUBLESHOOTING.md` | Root | `knowledge/troubleshooting/` | Operational guides |
| **Core project files** |
| `Cargo.toml` | Root | Root | Workspace manifest, must be at root |
| `Cargo.lock` | Root | Root | Dependency lock, must be at root |
| `LICENSE` | Root | Root | License file, best at root |
| `deps.yaml` | Root | Root or `.github/` | Renovate dependency config |
| **Generated files** |
| `DELIVERABLES.txt` | Root | `progress/deliverables/` | Progress tracking |
| `*.rs` code examples | Root | `progress/design/examples/` | Design artifacts |
| `*.txt` mockups | Root | `progress/design/mockups/` | Design artifacts |

---

## Open Questions (Need User Decision)

### 1. **docs/ directory contents**
**Current**: Unclear what's in docs/
**Question**: Should we keep docs/ or merge into knowledge/? Need to review contents.

### 2. **scripts/ location**
**Option A**: `src/scripts/` (proposed above)
**Option B**: Root `scripts/` (discoverable for AJ)
**Question**: Should scripts be at root for discoverability or in src/?

### 3. **lib/ bash modules**
**Option A**: `src/lib/` (proposed above - code is code)
**Option B**: Root `lib/` (yollayah.sh sources from ./lib)
**Question**: Bash modules are code, but yollayah.sh expects ./lib. Move or keep?

### 4. **yollayah-build-log.sh location**
**Option A**: Root (next to yollayah.sh)
**Option B**: `src/scripts/`
**Question**: Build script for diagnostics - visible at root or in scripts/?

### 5. **Test organization**
**Current**: `tests/architectural-enforcement/` at root
**Proposed**: `src/tests/architectural-enforcement/`
**Question**: Keep tests at root or move to src/?

### 6. **PRINCIPLES.md**
**Current**: Root `PRINCIPLES.md` (older file)
**Proposed**: Delete (superseded by knowledge/principles/*)
**Question**: Delete or preserve somewhere?

### 7. **Completed epics/stories**
**Current**: No DONE-* files yet
**Proposed**: When TODO-xyz completes → move to progress/completed/ and rename to DONE-xyz
**Question**: Confirm this easter egg pattern?

---

## Migration Plan

### Phase 1: Create New Structure (No Breaking Changes)
1. Create `knowledge/`, `progress/`, `src/` directories
2. Create subdirectories and index files
3. **DO NOT DELETE OR MOVE** anything yet

### Phase 2: Move Files (Preserve Git History)
1. Use `git mv` to preserve history
2. Move in batches:
   - Batch 1: reference/ → knowledge/principles/, knowledge/requirements/
   - Batch 2: workflows/ → knowledge/methodology/
   - Batch 3: TODO-*.md → progress/active/
   - Batch 4: BUG-*.md → progress/bugs/
   - Batch 5: DESIGN-*.md → progress/design/
   - Batch 6: Audits → progress/audits/
   - Batch 7: Code → src/
3. Update internal references in files

### Phase 3: Update Tooling
1. Update yollayah.sh to reference new paths
2. Update .git/hooks/pre-commit
3. Update CLAUDE.md with new structure
4. Update README.md with new structure

### Phase 4: Test and Validate
1. Run full build: `cargo build --workspace`
2. Test yollayah.sh startup
3. Run integration tests
4. Verify all references updated

---

## New Files to Create

### 1. `knowledge/KNOWLEDGE.md`
Index file describing:
- The knowledge system organization
- Methodology vs Principles vs Requirements
- Team structure and agent associations
- How to use this knowledge base

### 2. `knowledge/project/AI-WAY.md`
Project philosophy:
- Vision and mission
- Privacy-first principles (Five Laws of Evolution, Four Protections)
- Average Joe (AJ) persona and journey to Privacy Joe (PJ)
- Long-term roadmap and values

### 3. `knowledge/team/TEAM.md`
Agent associations:
- "Have the Rust and Ratatui experts review" → rust-ratatui-team
- "Have the LLM specialists optimize" → llm-ollama-team
- "Have the UX and security team validate" → ux-security-team

### 4. `knowledge/methodology/DONE-easter-egg.md`
Documents the sweet pattern:
- When TODO-xyz is 100% complete → rename to DONE-xyz
- Move to progress/completed/
- Ultimate goal: TODO-AI-WAY.md → DONE-AI-WAY.md (project complete!)

### 5. `progress/TODO-AI-WAY.md`
Main project tracker:
- Overall project status
- Major milestones
- References to active epics/stories
- Will become DONE-AI-WAY.md when project ships!

### 6. `yollayah-build-log.sh`
Verbose build script:
```bash
#!/bin/bash
# Verbose build with diagnostics
# Usage: ./yollayah-build-log.sh [--tui|--conductor|--surfaces|--all]
```

Flags:
- `--tui` - Build TUI with verbose output
- `--conductor` - Build Conductor with verbose output
- `--surfaces` - Build all surfaces (currently just TUI)
- `--all` or no flag - Build entire workspace

Features:
- Capture full build output
- Save to build-log-TIMESTAMP.txt
- Highlight errors and warnings
- Check binary existence
- Test execution (basic smoke test)

---

## README.md Updates

### Add Sections:
1. **Prerequisites**
   - Fedora Silverblue (tested/developed platform)
   - toolbox (preinstalled on Silverblue)
   - Rust toolchain
   - Ollama (auto-installed by yollayah.sh)

2. **⚠️ Warning Section**
   ```markdown
   ## ⚠️ EXPERIMENTAL SOFTWARE - USE AT YOUR OWN RISK

   This is early-stage development software. Expect:
   - **Breaking changes** without notice
   - **Bugs and crashes** - this is breaky-breaky stuff!
   - **Performance issues** during optimization
   - **Incomplete features** - work in progress

   **NOT RECOMMENDED FOR PRODUCTION USE**

   If you're brave enough to try it, welcome aboard!
   We appreciate bug reports and contributions.
   ```

3. **Platform Support**
   - Primary: Fedora Silverblue (with toolbox)
   - Tested: Fedora Workstation
   - Experimental: Other Linux distros
   - Not supported: Windows, macOS (yet)

4. **Quick Start**
   ```bash
   ./yollayah.sh           # Full experience
   ./yollayah.sh --test    # Fast startup for testing
   ```

5. **Project Structure** (reference new organization)

---

## Benefits of Reorganization

### For Developers
- ✅ **Easy navigation** - Find files by purpose (knowledge vs progress)
- ✅ **Clear intent** - src/ means code, knowledge/ means reference
- ✅ **Standard layout** - Follows Rust/open-source conventions
- ✅ **Git history preserved** - `git mv` maintains file history

### For Average Joe (AJ)
- ✅ **Single entry point** - yollayah.sh is obvious
- ✅ **agents/ discoverable** - Core component visible at root
- ✅ **Progressive disclosure** - Can explore src/ → agents/ journey

### For Claude and AI Assistants
- ✅ **Clear context** - knowledge/ = static, progress/ = dynamic
- ✅ **Team references** - "Have the Rust team..." → knowledge/team/
- ✅ **Methodology** - Clear TODO-driven process in knowledge/methodology/

### For Project Health
- ✅ **Sustainable growth** - Progress files grow, knowledge files stabilize
- ✅ **Easter egg motivation** - Working toward DONE-AI-WAY.md!
- ✅ **Quality gates** - High-trust changes to knowledge/, anyone to progress/

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| **Break yollayah.sh** | Update paths before moving files, test after each batch |
| **Lose git history** | Use `git mv`, not delete+recreate |
| **Internal link rot** | Grep for old paths, update systematically |
| **Confuse contributors** | Update CLAUDE.md, README.md, add KNOWLEDGE.md index |
| **Merge conflicts** | Do reorganization in dedicated session, clear communication |

---

## Success Criteria

- ✅ Root directory has < 12 visible items (4 dirs + 6 files + hidden)
- ✅ yollayah.sh immediately visible and works
- ✅ All tests pass after reorganization
- ✅ CLAUDE.md updated with new structure
- ✅ README.md updated with warnings and new structure
- ✅ Git history preserved for all moved files
- ✅ yollayah-build-log.sh created and tested
- ✅ TODO-AI-WAY.md created as main project tracker

---

## Next Steps

1. **User review and approval** of this proposal
2. **Answer open questions** (docs/, scripts/, lib/ locations)
3. **Create yollayah-build-log.sh** first (for diagnosing current issue)
4. **Execute migration plan** in phases
5. **Update documentation** (README, CLAUDE.md)
6. **Test and validate** all changes

---

**Status**: Awaiting user approval and decisions on open questions.
