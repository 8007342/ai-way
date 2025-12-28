# ai-way

Privacy-first local AI runtime. Your AI, your data, your rules.

## Quick Start (ai-way-lite)

```bash
git clone https://github.com/8007342/ai-way.git
cd ai-way
./yollayah.sh
```

That's it. Yollayah will:
1. Check for Ollama (offer to install if missing)
2. Pull a small base model (~2GB)
3. Create her personality
4. Start chatting with you

**Requirements**: Just Ollama. The script handles everything else.

### Environment Variables

```bash
YOLLAYAH_MODEL=llama3.2:3b  # Base model (default: llama3.2:3b)
```

## Two Modes

### ai-way-lite (Quick Start)
- Single bash script
- Direct Ollama conversation
- No Python dependencies
- Perfect for trying it out

### ai-way-full (Coming Soon)
- Multi-agent routing (19 specialists)
- Session persistence
- API server for surfaces
- Web UI, terminal UI, mobile surfaces
- Yollayah floats between screens

## Architecture (Full Mode)

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
