# Routing Module

> **Work in Progress**: This module is under active development.

The routing module handles task delegation to specialist agents and manages
context flow between Yollayah and their expert family.

## Overview

When Yollayah encounters a complex request, they can delegate to specialists:

```
User: "Can you review this API for security issues?"
       ↓
Yollayah: "Let me check with my cousin Rita..."
       ↓
[Routes to ethical-hacker specialist]
       ↓
Rita analyzes, returns findings
       ↓
Yollayah: "Rita found some issues..."
```

## Module Structure

```
lib/routing/
├── init.sh          # Module initialization
├── classifier.sh    # Determines which specialist to invoke
├── invoker.sh       # Executes specialist delegation
├── context.sh       # Manages context between specialists
├── tasks.sh         # Background task management
├── aggregator.sh    # Combines multi-specialist responses
└── avatar_agent.sh  # Coordinates avatar state with tasks
```

## Key Concepts

### Task Delegation

Tasks are delegated using structured directives:

```
[yolla:task specialist="ethical-hacker" priority="high"]
Review this code for security vulnerabilities
[/yolla:task]
```

### Context Flow

Context flows through specialists:
1. User request → Yollayah (conductor)
2. Yollayah classifies task → routes to specialist
3. Specialist processes with full context
4. Response aggregated → returned to Yollayah
5. Yollayah presents to user with personality

### Avatar Integration

The routing module coordinates with the TUI avatar:
- Shows thinking state during delegation
- Displays background task progress
- Animates responses as they stream back

## Environment Variables

```bash
YOLLAYAH_ROUTING_DEBUG=1    # Verbose routing logs
```

## Constitution Reference

- **Law of Elevation**: "Lift others higher" - Specialists help users grow
- **Law of Service**: "Serve genuine interests" - Route to best expert for task

---

*Last Updated: 2025-12-30*
