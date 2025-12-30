# ai-way

Privacy-first local AI runtime. Your AI, your data, your rules.

## Quick Start

```bash
git clone https://github.com/8007342/ai-way.git
cd ai-way
./yollayah.sh
```

That's it. Yollayah will:
1. Check for Ollama (offer to install if missing)
2. Pull a small base model (~2GB)
3. Create their personality from the conductor profile
4. Start the TUI (if Rust is available) or fall back to terminal chat

**Requirements**: Just Ollama. The script handles everything else.

### Environment Variables

```bash
YOLLAYAH_MODEL=llama3.2:3b  # Base model (default: auto-detected)
YOLLAYAH_DEBUG=1            # Enable debug logging
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
│  ├── logging/     Internal logs (for PJ debugging)              │
│  ├── ollama/      Model management, lifecycle                   │
│  ├── user/        Preferences, history (privacy-first)          │
│  ├── ux/          Terminal output, TUI launcher                 │
│  └── yollayah/    Personality, modelfile generation             │
├─────────────────────────────────────────────────────────────────┤
│  tui/             Rust TUI with animated axolotl avatar         │
│  agents/          Conductor + specialist profiles (synced)      │
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
- **Ephemeral**: Session data destroyed on close
- **Offline**: Works without internet
- **No telemetry**: Zero data collection

## Agent Profiles

Agent personalities, expertise, and working styles are defined in the
[agents](https://github.com/8007342/agents) repository. The Constitution
(Five Laws of Evolution) is embedded in every agent.

See `agents/conductors/yollayah.md` for the conductor profile.

## Development

```bash
# Install git hooks (auto-updates checksums)
./scripts/install-hooks.sh

# Run with debug logging
YOLLAYAH_DEBUG=1 ./yollayah.sh
```

## License

AGPL-3.0
