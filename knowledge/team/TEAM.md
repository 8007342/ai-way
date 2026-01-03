# AI-Way Team Structure

**Purpose**: Agent specializations and associations for "have the team..." references

---

## Team Organization

When working on ai-way, different tasks require different expertise. This document maps common requests to the appropriate team of agents.

---

## Core Teams

### Rust & Ratatui Team
**When to use**: "Have the Rust experts review...", "Have the Ratatui team optimize..."

**Members**:
- Rust Expert - Language specialist, performance optimization
- Ratatui Specialist - Terminal UI, framebuffer rendering
- Async Specialist - Tokio, futures, non-blocking I/O

**Responsibilities**:
- TUI implementation and optimization
- Conductor core development
- Performance profiling and optimization
- Async architecture enforcement

**See**: [rust-ratatui-team.md](rust-ratatui-team.md)

---

### LLM & Ollama Team
**When to use**: "Have the LLM specialists optimize...", "Have the Ollama team investigate..."

**Members**:
- LLM Specialist - Model behavior, prompting, inference
- Ollama Specialist - Backend integration, model management
- Performance Specialist - Inference optimization, GPU utilization

**Responsibilities**:
- LLM backend integration
- Model selection and optimization
- Streaming response handling
- GPU detection and utilization

**See**: [llm-ollama-team.md](llm-ollama-team.md)

---

### UX & Security Team
**When to use**: "Have the UX team review...", "Have the hacker validate..."

**Members**:
- UX/UI Designer - User experience, interface design
- Ethical Hacker - Security, threat modeling, penetration testing
- Privacy Researcher - Privacy patterns, anonymization

**Responsibilities**:
- Surface design and UX
- Security threat analysis
- Privacy protection implementation
- User testing and feedback

**See**: [ux-security-team.md](ux-security-team.md)

---

## Specialized Roles

### Architect
**When to use**: "Have the architect review...", "Architectural decision needed..."

**Responsibilities**:
- System architecture and design
- Principle definition and enforcement
- Methodology updates
- Long-term technical direction

**Authority**: Can update knowledge/ directory (high trust)

---

### QA Engineer
**When to use**: "Have QA test...", "Need integration tests..."

**Responsibilities**:
- Test design and implementation
- Integration test infrastructure
- Smoke testing and regression testing
- CI/CD pipeline maintenance

---

## Team Collaboration Patterns

### Example 1: Performance Issue
**Request**: "TUI is slow, optimize rendering"

**Team**: Rust & Ratatui Team
**Process**:
1. Ratatui Specialist profiles rendering
2. Rust Expert identifies hotspots
3. Async Specialist ensures non-blocking patterns
4. Team implements optimizations together

---

### Example 2: Security Feature
**Request**: "Prevent data leaks in model responses"

**Team**: UX & Security Team + LLM Team
**Process**:
1. Ethical Hacker identifies threat model
2. Privacy Researcher suggests mitigation patterns
3. LLM Specialist implements prompt filtering
4. UX Designer ensures no friction for AJ

---

### Example 3: New Surface
**Request**: "Add web UI surface"

**Team**: Rust & Ratatui Team + UX & Security Team
**Process**:
1. Architect defines surface interface
2. Rust Expert implements Conductor protocol
3. UX Designer creates web interface
4. Ethical Hacker reviews security implications

---

## Communication Patterns

### In Documentation
- "Have the **Rust team** review this async code"
- "Have the **LLM specialists** optimize this prompt"
- "Have the **UX and security team** validate this flow"
- "Have the **architect** approve this principle"

### In Code Comments
```rust
// TODO: Have the Rust team optimize this allocation pattern
// TODO: Have the hacker validate this sanitization
// TODO: Have the LLM team review this prompt template
```

### In Commit Messages
```
fix: streaming latency regression

- Rust team identified blocking recv() call
- Async specialist suggested channel architecture
- Performance improved 10ms → 0ms latency
```

---

## Team Rotation and Growth

**Current**: Small project, overlapping roles
**Future**: As project grows, teams become more specialized

**Adding New Teams**:
1. Identify recurring expertise need
2. Create team documentation
3. Update this index
4. Reference in methodology

**Example Future Teams**:
- Database & Storage Team (RAG, vector search)
- DevOps & Infrastructure Team (CI/CD, deployment)
- Documentation Team (tutorials, guides, examples)

---

**Remember**: Teams are about expertise, not hierarchy. Everyone contributes where their skills shine.
