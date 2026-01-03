# ai-way

Privacy-first local AI appliance. Your AI, your data, your rules.

---

## ⚠️ EXPERIMENTAL SOFTWARE - USE AT YOUR OWN RISK

This is early-stage development software - breaky-breaky stuff!

**Expect:**
- 🔥 **Breaking changes** without notice
- 💥 **Bugs and crashes** - this is unstable
- 🐌 **Performance issues** during ongoing optimization
- 🚧 **Incomplete features** - work in progress
- 📝 **Evolving documentation** - updated regularly

**NOT RECOMMENDED FOR PRODUCTION USE**

**If you're brave enough to try it, welcome aboard!** We appreciate bug reports and contributions.

---

> **Current Phase**: Foundation (80% complete) - Local AI chat with privacy, zero configuration

## Prerequisites

**Platform**:
- **Fedora Silverblue** (recommended - tested and developed on this platform)
- **toolbox** (preinstalled on Silverblue)
- Other Linux distros supported but not as extensively tested

**Required**:
- bash (4.0+)
- curl
- git

**Auto-installed by yollayah.sh**:
- Ollama (AI model runtime)
- Rust/Cargo (for TUI)

---

## Recommended Setup (Fedora Silverblue)

If you're on Fedora Silverblue, ai-way automatically uses toolbox containers for clean dependency isolation. This is the recommended setup:

```bash
git clone https://github.com/8007342/ai-way.git
cd ai-way
./yollayah.sh
```

First run takes ~2-3 minutes:
1. Creates an isolated toolbox container (30 seconds)
2. Installs Ollama inside the container (1-2 minutes)
3. Downloads and loads an optimized model for your GPU
4. Launches the TUI with Yollayah

Subsequent runs take ~5 seconds. Everything is automatic - no manual setup required.

**Benefits**: Complete isolation, clean uninstall (`toolbox rm ai-way`), GPU passthrough works automatically.

See [`knowledge/platform/TOOLBOX.md`](knowledge/platform/TOOLBOX.md) for details and troubleshooting.

## Quick Start (Other Systems)

On non-Silverblue systems (Ubuntu, Fedora Workstation, etc.):

```bash
git clone https://github.com/8007342/ai-way.git
cd ai-way
./yollayah.sh
```

Yollayah will:
1. Check and install missing dependencies (Ollama, Rust)
2. Pull a model optimized for your hardware (~2-8GB depending on GPU)
3. Create their personality from the conductor profile
4. Start the TUI (if available) or fall back to terminal chat

## Dependencies

### Required (Must Be Pre-installed)

These must be installed on your system before running Yollayah:

| Dependency | Why Needed | Install |
|------------|-----------|---------|
| **bash** (4.0+) | Shell runtime | Usually pre-installed |
| **curl** | Download installers | `apt install curl` / `dnf install curl` |
| **git** | Clone agents repository | `apt install git` / `dnf install git` |

### Auto-Installed

Yollayah will automatically install these if missing:

| Dependency | Why Needed | Install Method |
|------------|-----------|----------------|
| **Ollama** | AI model runtime | Official installer (requires sudo) |
| **Rust/Cargo** | Build TUI interface | rustup.rs (no sudo, installs to ~/.cargo) |

### Optional (For Enhanced Features)

| Dependency | Why Needed | Install |
|------------|-----------|---------|
| **nvidia-smi** | NVIDIA GPU detection | Comes with NVIDIA drivers |
| **rocm-smi** | AMD GPU detection | Part of ROCm toolkit |
| **lspci** | Fallback GPU detection | `apt install pciutils` |
| **systemctl** | Service management | Part of systemd (most Linux distros) |

### GPU Support

For GPU acceleration (highly recommended for performance):

**NVIDIA GPUs:**
- Install NVIDIA drivers with CUDA support
- Reinstall Ollama after driver installation: `curl -fsSL https://ollama.com/install.sh | sh`

**AMD GPUs:**
- Install ROCm: https://rocm.docs.amd.com
- Reinstall Ollama after ROCm setup

Yollayah will detect your GPU and select an appropriate model size automatically.

## Environment Variables

```bash
YOLLAYAH_MODEL=llama3.2:3b    # Override model selection
YOLLAYAH_DEBUG=1              # Enable debug output (see what's happening under the hood)
YOLLAYAH_PERSIST_LOGS=1       # Keep logs after shutdown (default: deleted)
YOLLAYAH_SKIP_INTEGRITY=1     # Skip checksum verification (dev mode)
```

### Debug Mode

When `YOLLAYAH_DEBUG=1` is set, Yollayah shows informative messages about what's happening:

```bash
YOLLAYAH_DEBUG=1 ./yollayah.sh
```

You'll see output like:
```
▸ Detecting GPU hardware
→ Running: nvidia-smi --query-gpu=name,memory.total
✓ Found: NVIDIA GeForce RTX 3080, 10240 MiB
▸ Selecting best model for your hardware
  └─ Tier: powerful (10GB VRAM)
  └─ Selected: llama3.1:8b
```

This is useful for understanding hardware detection, model selection, and troubleshooting.

## Meet Yollayah

Yollayah ("heart that goes with you" in Nahuatl) is your AI companion - a saucy Latina axolotl with heart. They're warm, real, and playfully opinionated.

### The Expert Family

Yollayah has a family of specialist agents they consult for deep expertise:

| Agent | Family Name | Specialty |
|-------|-------------|-----------|
| ethical-hacker | Cousin Rita | Security audits, penetration testing |
| backend-engineer | Uncle Marco | APIs, server-side logic |
| frontend-specialist | Prima Sofia | UI implementation, accessibility |
| solutions-architect | Tia Carmen | System design, architecture |
| qa-engineer | The Intern | Testing, quality assurance |
| ux-designer | Cousin Lucia | User experience, design |
| privacy-researcher | Abuelo Pedro | Data privacy, fingerprinting |
| compliance-lawyer | Tio Javier | GDPR, legal compliance |

When you ask something complex, Yollayah might say: "Hold up - let me check with my cousin Rita, she's the security expert in the family..."

## Project Structure

ai-way is organized for clarity and discoverability:

```
ai-way/
├── agents/                  # AI agent profiles and Constitution (discoverable for AJ→PJ journey)
├── knowledge/               # Immutable knowledge base (methodology, principles, team structure)
│   ├── project/             # AI-WAY.md - project philosophy
│   ├── methodology/         # TODO-driven development, DONE easter egg
│   ├── principles/          # Async efficiency, data flow patterns
│   ├── requirements/        # TUI/Conductor separation
│   ├── anti-patterns/       # Forbidden practices (sleep, blocking I/O)
│   ├── team/                # Agent specializations
│   ├── platform/            # TOOLBOX.md and platform guides
│   └── troubleshooting/     # Common issues and solutions
├── progress/                # Iterative progress tracking (dynamic, changes every sprint)
│   ├── TODO-AI-WAY.md       # Main project tracker (will become DONE-AI-WAY.md when we ship!)
│   ├── active/              # Active TODOs, EPICs, Stories
│   ├── bugs/                # Bug tracking
│   ├── completed/           # Completed work (TODO → DONE renames)
│   ├── audits/              # Performance and architecture audits
│   ├── design/              # Design explorations
│   └── work-logs/           # Session summaries
├── conductor/               # Conductor Rust code (orchestration engine)
├── tui/                     # TUI Rust code (animated axolotl interface)
├── lib/                     # Bash modules for yollayah.sh
├── tests/                   # Integration tests (architectural enforcement)
├── yollayah.sh              # Single entry point (zero configuration)
├── yollayah-build-log.sh    # Verbose build diagnostics
└── README.md                # This file
```

**The Sweet Easter Egg** 🎉: When a `TODO-xyz` is 100% complete, it's renamed to `DONE-xyz` and moved to `progress/completed/`. Ultimate goal: `TODO-AI-WAY.md` → `DONE-AI-WAY.md` (when ai-way ships!)

**See**: [`knowledge/KNOWLEDGE.md`](knowledge/KNOWLEDGE.md) for knowledge base details

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         yollayah.sh                              │
│  Bootstrap: loads modules, starts TUI or terminal chat          │
├─────────────────────────────────────────────────────────────────┤
│  lib/                                                           │
│  ├── agents/      Agent sync, profile parsing                   │
│  ├── integrity/   Script verification, checksums                │
│  ├── logging/     Category-based logs (for PJ debugging)        │
│  ├── ollama/      Model management, GPU detection, lifecycle    │
│  ├── routing/     Specialist task delegation                    │
│  ├── user/        Preferences, history (privacy-first)          │
│  ├── ux/          Terminal output, TUI launcher                 │
│  └── yollayah/    Personality, setup, modelfile generation      │
├─────────────────────────────────────────────────────────────────┤
│  conductor/       Async orchestration engine (Rust)             │
│  tui/             Terminal UI with animated axolotl (Rust)      │
│  agents/          Conductor + specialist profiles (synced)      │
│  .logs/           Ephemeral logs (deleted on shutdown)          │
│  .integrity/      Checksum manifest for verification            │
└─────────────────────────────────────────────────────────────────┘
```

## The TUI

The Rust TUI features:
- Animated axolotl avatar that moves, reacts, and expresses emotions
- Avatar controlled by `[yolla:...]` commands in Yollayah's responses
- Background task display for concurrent agent work
- Streaming responses with live avatar interaction

## Privacy First

- **Local-only**: All inference runs on your machine
- **Ephemeral**: Session data and logs destroyed on close
- **Offline**: Works without internet (after initial setup)
- **No telemetry**: Zero data collection

## Logging

Logs are stored in `.logs/` and are **deleted on clean shutdown** by default.

To persist logs for debugging:
```bash
YOLLAYAH_PERSIST_LOGS=1 ./yollayah.sh
```

Log categories: `yollayah.log`, `ollama.log`, `agents.log`, `ux.log`, `session.log`

See `.logs/README.md` for details on log format and filtering.

## Agent Profiles

Agent personalities, expertise, and working styles are defined in the
[agents](https://github.com/8007342/agents) repository. The Constitution
(Five Laws of Evolution) is embedded in every agent.

See `agents/conductors/yollayah.md` for the conductor profile.

## Development

```bash
# Install git hooks (auto-updates checksums on commit)
./scripts/install-hooks.sh

# Run with debug logging
YOLLAYAH_DEBUG=1 ./yollayah.sh

# Generate integrity checksums (after modifying .sh files)
YOLLAYAH_INTEGRITY_GENERATE=1 ./yollayah.sh
```

## Troubleshooting

### GPU Not Being Used
If you have an NVIDIA GPU but Yollayah is slow:
```bash
# Check if nvidia-smi works
nvidia-smi

# Reinstall Ollama to pick up CUDA
curl -fsSL https://ollama.com/install.sh | sh

# Run with debug to see GPU detection
YOLLAYAH_DEBUG=1 YOLLAYAH_PERSIST_LOGS=1 ./yollayah.sh
cat .logs/ollama.log
```

### Integrity Check Failures
After pulling updates, if you get checksum errors:
```bash
# The checksums should be up to date in git
git pull

# If still failing, regenerate (dev mode)
YOLLAYAH_INTEGRITY_GENERATE=1 ./yollayah.sh
```

## License

AGPL-3.0
