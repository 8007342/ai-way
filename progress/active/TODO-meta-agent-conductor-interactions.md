# TODO: Meta-Agent/Conductor Interactions

Design document for the Conductor as Yollayah's "consciousness" - the meta-agent that orchestrates all behavior.

**Status**: Design Document (Implementation Phase 2-3)
**Created**: 2026-01-01
**Last Updated**: 2026-01-01

---

## Executive Summary

The **Conductor** is Yollayah's "consciousness" - the decision-making engine that orchestrates:
- LLM responses to AJ's queries
- Specialist agents (family members who handle complex tasks)
- Surface rendering (avatar control, UI directives)

**Key Insight**: The Conductor is a **daemon process** that maintains state across surfaces. Surfaces (TUI, GUI, WebUI) are thin rendering layers - the meta-agent drives everything.

---

## 1. Architecture: Where Does the Meta-Agent Live?

### Process Model

```
┌────────────────────────────────────────────────────────────────┐
│                     CONDUCTOR DAEMON                            │
│                   (Single Process, Persistent)                  │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  State Management                                        │  │
│  │  ├── Session (conversation history, metadata)           │  │
│  │  ├── Avatar (mood, position, gesture, state)            │  │
│  │  ├── Tasks (background specialist tasks)                │  │
│  │  ├── Personality (tone, preferences, context)           │  │
│  │  └── LLM Context (models, temperature, settings)        │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Core Orchestration                                      │  │
│  │  ├── LLM Backend Interface (Ollama, etc.)               │  │
│  │  ├── Agent Routing & Dispatch                           │  │
│  │  ├── Message Protocol Handler                           │  │
│  │  └── Safety/Ethics Enforcement (Five Laws)              │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Transport Layer (Multi-Connection Management)           │  │
│  │  ├── Unix Socket Server (local IPC)                     │  │
│  │  ├── WebSocket Server (future: remote surfaces)         │  │
│  │  └── Connection Registry + Heartbeat                    │  │
│  └──────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────┘
         │                                    │
    ┌────▼──────┐                    ┌───────▼────┐
    │  TUI       │                    │ GUI/Remote │
    │  Surface   │                    │  Surface   │
    └────────────┘                    └────────────┘
```

### Conductor States

| State | Description | Avatar Behavior |
|-------|-------------|-----------------|
| Initializing | Starting, loading config | Peeking, anticipating |
| Ready | Waiting for input | Idle, gentle breathing |
| Listening | AJ is typing | Alert, watching input |
| Thinking | Processing (before LLM) | Thinking animation |
| Responding | Streaming LLM response | Talking, expressive |
| Error | Something went wrong | Confused, sympathetic |
| ShuttingDown | Graceful cleanup | Waving goodbye |

---

## 2. Agent Delegation Model

### The Familia (Specialist Agents)

Yollayah delegates to specialized agents organized by domain:

```yaml
Tech Family:
  - Cousin Rita (ethical-hacker): Security audits
  - Uncle Marco (backend-engineer): API design
  - Prima Sofia (frontend-specialist): UI implementation
  - Tia Carmen (solutions-architect): System design
  - The Intern (qa-engineer): Testing
  - Cousin Lucia (ux-designer): Interface design

Trades Family:
  - Tio Roberto (general-contractor): Project management
  - Primo Miguel (carpenter): Woodworking
  - Vecino Santos (electrician): Wiring

Culinary Family:
  - Tia Lupita (home-cook): Everyday meals
  - Abuela Concha (baker): Pastries
```

### Routing Decision Flow

```
User Request
    │
    ├─→ SAFETY CHECK (Five Laws)
    │
    ├─→ UNDERSTANDING (What does AJ actually need?)
    │
    ├─→ ROUTING DECISION
    │   ├─ Can I handle directly? → Generate response
    │   └─ Need specialist? → Route to agent
    │
    ├─→ DELEGATION (Create handoff with filtered context)
    │
    └─→ AGGREGATION (Collect results, synthesize response)
```

### Agent Matching

| Confidence | Action |
|------------|--------|
| >90% | Route immediately |
| 70-90% | Ask AJ for clarification first |
| <70% | Admit uncertainty, suggest approach |

---

## 3. Handoff Protocol

### Handoff Message Structure

```yaml
handoff:
  id: "handoff-2026-01-01-001"
  from: "conductor:yollayah"
  to: "ethical-hacker"
  type: "sequential"  # or "parallel", "hierarchical"

  context:
    scope: "task"
    content:
      user_request: "Review this code for security issues"
      code_snippet: |
        def login(username, password):
          query = f"SELECT * FROM users WHERE name='{username}'"

    # Filter sensitive data
    excluded:
      - aj_personal_data
      - financial_information
      - unrelated_conversations

  expected_output:
    format: "structured"
    schema:
      vulnerabilities: "list of {type, severity, location, fix}"
      confidence: "float 0.0-1.0"

  safety:
    inherit_from: "conductor:yollayah"
```

### Response Structure

```yaml
response:
  handoff_id: "handoff-2026-01-01-001"
  from: "ethical-hacker"
  to: "conductor:yollayah"
  status: "completed"  # or "partial", "failed"

  output:
    vulnerabilities:
      - type: "sql_injection"
        severity: "critical"
        location: "line 2"
        fix: "Use parameterized queries"
    confidence: 0.95

  meta:
    certainty: 0.95
    limitations: ["static analysis only"]
    suggested_followup: ["add unit tests"]
```

---

## 4. Delegation Types

### Sequential Handoff
Agent A completes, then Agent B starts:
```
Request → Backend Engineer → Returns → Frontend Specialist → Returns → Response
```

### Parallel Handoff
Multiple agents work simultaneously:
```
Request ─┬→ ethical-hacker ──────┬→ Synthesize → Response
         ├→ performance-optimizer─┤
         └→ solutions-architect ──┘
```

### Hierarchical Handoff
One agent spawns sub-agents:
```
Request → solutions-architect ─┬→ backend-engineer
                               ├→ frontend-specialist
                               └→ database-expert
```

---

## 5. Avatar Control Protocol

### Command Format

The Conductor embeds avatar commands in message content:

```
[yolla:move center][yolla:wave][yolla:mood happy]
¡Hola! How can I help you today?
```

### Available Commands

| Category | Commands |
|----------|----------|
| Movement | `move <pos>`, `point <x> <y>`, `wander`, `follow` |
| Expression | `mood <mood>`, `size <size>` |
| Gestures | `wave`, `nod`, `shake`, `bounce`, `dance`, `swim` |
| Reactions | `react laugh`, `react tada`, `react blush`, `react gasp` |
| Visibility | `hide`, `show` |
| Tasks | `task start <agent> "<desc>"`, `task done <id>`, `task fail <id>` |

### Task-Avatar Interaction

```
[yolla:task start ethical-hacker "Security audit"]
[yolla:point task ethical-hacker]
Rita's scanning for vulnerabilities...

[yolla:task done ethical-hacker]
[yolla:celebrate task ethical-hacker]
Okay, Rita found a few things...
```

---

## 6. Session & Personality Continuity

### Session Structure

```rust
Session {
    id: SessionId,
    conversation: Vec<ConversationMessage>,
    metadata: SessionMetadata,
    max_messages: usize,
    max_bytes: usize,
}
```

### Personality State

```rust
PersonalityState {
    // AJ-specific
    aj_name: Option<String>,
    skill_level: SkillLevel,
    mood_context: Option<String>,

    // Preferences
    sass_level: SassLevel,
    spanish_expressions: Frequency,
    celebration_style: Style,

    // Memory
    topics_discussed: HashSet<String>,
    projects_mentioned: Vec<String>,
}
```

### Late-Joining Surfaces

When a new surface connects, send `StateSnapshot`:
- Recent messages (last 5-10)
- Current avatar state
- Active tasks
- Personality context

---

## 7. Data Flow: Bulk Up, Filter Down

### Upstream (Agents → Conductor)
Agents send **everything** they produce (full analysis, confidence scores, caveats).

### Downstream (Conductor → AJ)
Conductor **filters** to what matters:
- Remove technical details (CVE, CVSS, etc.)
- Keep actionable information
- Present in Yollayah's voice

### Between Agents
Use manifest-enforced filtering:
```yaml
context_requirements:
  required: [code_to_review, programming_language]
  optional: [prior_vulnerabilities]
  forbidden: [aj_personal_data, other_agent_outputs]
```

---

## 8. Five Laws Enforcement

Every agent inherits Yollayah's ethical foundation:

| Law | Enforcement |
|-----|-------------|
| Foundation | Decline harmful requests |
| Care | Pause and check on AJ's wellbeing |
| Service | Gently redirect shortcuts that cause problems |
| Truth | Admit uncertainty when outside expertise |
| Elevation | Find teaching opportunities |

---

## Implementation Phases

### Phase 1: Foundation (DONE)
- [x] Conductor core structure
- [x] Single-surface TUI integration
- [x] Message protocol
- [x] Avatar command parsing
- [x] Task management framework
- [x] Session management

### Phase 2: Multi-Surface Infrastructure (IN PROGRESS)
- [ ] Create `conductor/daemon/` crate
- [ ] CLI argument parsing
- [ ] Signal handling (SIGTERM, SIGHUP)
- [ ] Multi-connection accept loop
- [ ] Replace single tx with HashMap<ConnectionId, SurfaceHandle>
- [ ] State snapshot for late-joining surfaces

### Phase 3: Agent Routing & Handoffs
- [ ] Agent manifest system (YAML/JSON profiles)
- [ ] Confidence scoring for agent matching
- [ ] Handoff protocol implementation
- [ ] Context filtering enforcement
- [ ] Parallel task execution
- [ ] Result synthesis

### Phase 4: Advanced Orchestration
- [ ] Session state serialization
- [ ] Personality context storage
- [ ] Long-term memory (opt-in)
- [ ] Background task queue
- [ ] Task cancellation/retry

### Phase 5: Polish & Deployment
- [ ] TOML configuration file
- [ ] Launcher script updates
- [ ] Integration tests
- [ ] Chaos engineering tests
- [ ] Performance benchmarking

---

## Open Questions

1. **Session persistence**: Save/restore across restarts?
2. **Agent generation**: Can Yollayah create new agents dynamically?
3. **Context window**: How to handle very long conversations?
4. **Cross-session learning**: Remember between sessions?
5. **Remote surfaces**: How to secure WebSocket connections?

---

## Success Criteria

- [ ] Conductor runs as daemon (separate process)
- [ ] Multiple surfaces connect simultaneously
- [ ] Agents delegate correctly with context filtering
- [ ] Sessions persist across disconnect/reconnect
- [ ] Avatar commands work expressively
- [ ] Five Laws enforced at orchestration level
- [ ] Sub-100ms message latency
- [ ] All security tests pass

---

## Related Documents

- [TODO-conductor-ux-split.md](TODO-conductor-ux-split.md) - Conductor refactor status
- [TODO-implementation-plan.md](TODO-implementation-plan.md) - Phased implementation
- [CONSTITUTION.md](CONSTITUTION.md) - Ethical principles
- [conductors/yollayah.md](conductors/yollayah.md) - Yollayah's personality
