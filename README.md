# ai-way

Privacy-first local AI runtime. Your AI, your data, your rules.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    AI-WAY CORE (headless)                        │
│                    Runs on powerful machine                      │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐ │
│  │   server.py │───▶│  conductor  │───▶│  specialist agents  │ │
│  │   (FastAPI) │    │  (Yollayah) │    │  (via Ollama)       │ │
│  └─────────────┘    └─────────────┘    └─────────────────────┘ │
│         │                                                       │
│         │ WebSocket / REST API                                  │
│         │ mDNS discovery (ai-way._tcp.local)                    │
└─────────┴───────────────────────────────────────────────────────┘
          │
┌─────────┴─────────┬─────────────────┬───────────────────────────┐
│                   │                 │                           │
▼                   ▼                 ▼                           ▼
┌──────────┐  ┌──────────┐  ┌──────────────┐  ┌─────────────────────┐
│ Terminal │  │  iPad    │  │ Browser Tab  │  │ E-ink Display      │
│ Surface  │  │  Surface │  │   Surface    │  │  (future)          │
└──────────┘  └──────────┘  └──────────────┘  └─────────────────────┘

Yollayah floats between surfaces. One conversation, many screens.
```

## Quick Start

```bash
# Install dependencies
pip install -r requirements.txt

# Generate agent modelfiles
python -m core.modelfile_gen ../agents ./modelfiles

# Start the core server
python -m core.server

# In another terminal, start a surface
python -m surfaces.terminal
```

## Components

- **core/** - The brain. Headless server running Ollama, Conductor, agents.
- **surfaces/** - The faces. Thin clients connecting to Core via API.
- **modelfiles/** - Generated Ollama modelfiles (gitignored).

## Configuration

Edit `config.yaml` to configure:
- Ollama connection settings
- Model selection
- Developer mode visibility
- Agent profiles path

## Agent Profiles

Agent personalities, expertise, and working styles are defined in the
[agents](https://github.com/8007342/agents) repository. The Constitution
(Five Laws of Evolution) is embedded in every agent.

## License

AGPL-3.0
