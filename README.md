# ai-way

> **Work in Progress**: This project is under active development. Features may change, and documentation is being updated regularly.

Privacy-first local AI runtime. Your AI, your data, your rules.

## Quick Start

```bash
git clone https://github.com/8007342/ai-way.git
cd ai-way
./yollayah.sh
```

That's it. Yollayah will:
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
YOLLAYAH_DEBUG=1              # Enable debug logging
YOLLAYAH_PERSIST_LOGS=1       # Keep logs after shutdown (default: deleted)
YOLLAYAH_SKIP_INTEGRITY=1     # Skip checksum verification (dev mode)
```

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
│  tui/             Rust TUI with animated axolotl avatar         │
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
