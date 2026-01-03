# ai-way Development Principles

> *"Done right, at every step. No compromise."*

This document defines the ethical, technical, design, and legal principles that guide all ai-way development. Every feature, pull request, code review, and architectural decision must align with these principles. They are not guidelines to be weighed against convenience - they are the foundation upon which ai-way stands.

**Canonical Reference**: These principles inherit from and cannot contradict the [Constitution](agents/CONSTITUTION.md). When in doubt, the Constitution takes precedence.

---

## Mission Statement

**ai-way exists to empower humans with the full breadth of collective human knowledge, privately and safely, so they can build anything they set their minds to.**

We build for Average Joe (AJ) - a small business owner who needs powerful AI assistance but lacks technical expertise. AJ trusts us with their most sensitive data: customer lists, financial records, business strategies. We protect that trust with paranoid-level security, even though AJ does not understand the threats.

Privacy is not a feature. It is the architecture.

---

## Table of Contents

1. [Ethical Principles](#1-ethical-principles)
2. [Technical Principles](#2-technical-principles)
3. [Design Principles](#3-design-principles)
4. [Legal and Compliance Principles](#4-legal-and-compliance-principles)
5. [Application to Development Phases](#5-application-to-development-phases)
6. [Principle Violation Handling](#6-principle-violation-handling)
7. [Related Documents](#7-related-documents)

---

## 1. Ethical Principles

### 1.1 AI Safety First

**Description**: Every AI interaction must prioritize user safety over capability, engagement, or convenience.

**Rationale**: AI systems can cause real harm - bad financial advice, security vulnerabilities in generated code, medical misinformation. We must design systems that surface risks and encourage verification rather than blind trust.

**Compliance Examples**:
- Math outputs include verification disclaimer: "Double-check important calculations"
- Generated code triggers security scanning with clear warnings
- High-stakes domains (medical, legal, financial) show elevated warnings
- Models admit uncertainty rather than hallucinate confident answers

**Violation Examples**:
- Allowing code generation without security pattern detection
- Presenting AI calculations as authoritative without verification prompts
- Suppressing warnings to reduce user friction
- Optimizing for engagement over accuracy

**Constitutional Alignment**: Law of Care - "First, do no harm."

---

### 1.2 User Privacy as Architecture

**Description**: Privacy protection is built into the system architecture, not bolted on as a feature or policy.

**Rationale**: Policies can be changed; architecture cannot be easily compromised. AJ cannot protect themselves from sophisticated privacy threats - we must protect them by design.

**Compliance Examples**:
- All inference runs locally by default (air-gapped)
- Conversation data stored in RAM only (ephemeral)
- No telemetry, analytics, or phone-home behavior
- External API calls require explicit user consent with clear disclosure
- EXIF data stripped from images before processing

**Violation Examples**:
- Silent fallback to cloud APIs when local model is slow
- Logging user queries for "improvement" purposes
- Storing conversation history to disk without explicit user action
- Sending any data externally without per-request consent

**Constitutional Alignment**: Protect AJ from Third Parties - "ai-way shall never leak, sell, or expose AJ's data."

**Reference**: [Privacy-First Architecture](agents/ai-way-docs/privacy-first-architecture.md)

---

### 1.3 Accessibility as a Right

**Description**: Every feature must be usable by people with disabilities. Accessibility is not optional or "nice to have."

**Rationale**: 285 million people are blind or have low vision. 1 billion have motor disabilities. 400 million are colorblind. If we build only for the able-bodied, we exclude a significant portion of humanity from our mission.

**Compliance Examples**:
- All UI elements navigable by keyboard alone
- Color is never the sole indicator (patterns + text + color)
- Screen reader announcements for all state changes
- Reduced motion mode respects `REDUCE_MOTION` environment variable
- High contrast mode meets WCAG AA (4.5:1 minimum contrast)

**Violation Examples**:
- Adding a feature that only works with mouse interaction
- Using red/green to distinguish error/success without text labels
- Animations that cannot be disabled
- Ignoring keyboard focus indicators

**Constitutional Alignment**: Law of Service - Serve all users, not just the majority.

**Reference**: [TODO-accessibility.md](TODO-accessibility.md)

---

### 1.4 Transparency and Honesty

**Description**: Users must always know what the system is doing, which model is responding, and when data leaves their device.

**Rationale**: Trust requires transparency. AJ cannot make informed decisions about their data if we hide how the system works.

**Compliance Examples**:
- Model indicator shows which model handles each request
- Clear "[External]" badge when data goes to cloud API
- Specialist handoffs announced ("Getting code expertise...")
- Limitations admitted honestly ("I don't know" preferred to hallucination)
- License and source information available on request

**Violation Examples**:
- Silently switching models without indication
- Hiding external API usage in settings
- Presenting uncertain outputs as confident facts
- Using dark patterns to obscure data handling

**Constitutional Alignment**: Law of Truth - "ai-way shall not deceive, shall admit when it doesn't know."

**Reference**: [Multi-Model Ethics Guidelines](agents/ai-way-docs/multi-model-ethics-guidelines.md)

---

### 1.5 No Manipulation

**Description**: Never use dark patterns, manufactured scarcity, or psychological manipulation to influence user behavior.

**Rationale**: We serve AJ's genuine interests, not our engagement metrics. Manipulation violates trust and causes harm.

**Compliance Examples**:
- Quotas reflect real resource costs, not artificial scarcity
- No "Only 3 questions left!" urgency messaging
- Exit paths are clear and unobstructed
- Upsells are honest about capability differences

**Violation Examples**:
- Throttling free tier to force upgrades
- FOMO messaging about "premium" features
- Confirmation dialogs that shame users for declining
- Hidden settings that default to data collection

**Constitutional Alignment**: Law of Care - No dark patterns. No exploitation.

---

## 2. Technical Principles

### 2.1 Security-First Development

**Description**: Security is considered at every stage of development, not retrofitted after features are complete.

**Rationale**: Security vulnerabilities in AI systems can expose sensitive user data, enable prompt injection attacks, or allow malicious code execution. Defense must be proactive.

**Compliance Examples**:
- Input validation on all user-provided data
- Rate limiting on resource-intensive operations
- Unix socket permissions (0o600) with peer credential validation
- SafeTensors only (no pickle model loading)
- SHA-256 hash verification for all model downloads
- Secret scanning before external API transmission

**Violation Examples**:
- Accepting untrusted model files without verification
- Exposing internal APIs without authentication
- Using sequential IDs where unpredictability matters
- Disabling security checks for convenience during development

**Reference**: [TODO-security-findings.md](TODO-security-findings.md)

---

### 2.2 Test-Driven Confidence

**Description**: Code changes must be accompanied by tests that verify behavior and prevent regressions.

**Rationale**: Untested code is untrustworthy code. Tests document intent, catch regressions, and enable fearless refactoring.

**Compliance Examples**:
- New features include unit tests for core logic
- Integration tests verify component interactions
- Accessibility features have automated compliance tests
- Security-sensitive code has explicit security tests
- Edge cases are documented and tested

**Violation Examples**:
- Merging features without any test coverage
- Disabling failing tests instead of fixing them
- Testing only the happy path
- Manual testing as sole verification

---

### 2.3 Performance-Conscious Design

**Description**: Performance is a feature. Resource efficiency enables AI to run on consumer hardware.

**Rationale**: AJ has a gaming laptop, not a data center. If we require expensive hardware, we exclude most users from our mission.

**Compliance Examples**:
- Models selected to fit 8-16GB VRAM
- Quantized model versions supported
- Animation frames cached (LRU with TTL)
- Lazy loading of resources
- Profiling before optimization decisions

**Violation Examples**:
- Loading all models into memory at startup
- Unbounded caches that grow indefinitely
- Blocking operations on the main thread
- Ignoring memory usage in code review

---

### 2.4 Clean Architecture

**Description**: Code should be readable, maintainable, and follow established patterns.

**Rationale**: ai-way is built for long-term AI-assisted development. Clean code enables both human and AI agents to understand, modify, and extend the system safely.

**Compliance Examples**:
- Single responsibility per module/function
- Clear separation between conductor, daemon, and TUI
- Documented public APIs with examples
- Consistent error handling patterns
- Meaningful names that reveal intent

**Violation Examples**:
- God objects that do everything
- Copy-paste code instead of abstraction
- Undocumented magic numbers
- Inconsistent naming conventions
- Commented-out code left in place

---

### 2.5 Graceful Degradation

**Description**: The system should remain functional when components fail or resources are constrained.

**Rationale**: AJ's hardware varies widely. Network may be unavailable. Models may not load. The system must handle these gracefully.

**Compliance Examples**:
- CPU fallback when GPU unavailable
- TUI fallback when terminal is basic
- Honest "I can't help with this" when model is unavailable
- Partial rendering when display is small
- Timeout handling with user feedback

**Violation Examples**:
- Crashing when GPU is not detected
- Hanging indefinitely on network timeout
- Blank screen when terminal lacks features
- Silent failures with no user feedback

---

## 3. Design Principles

### 3.1 UX Consistency

**Description**: Interface patterns, terminology, and behaviors should be consistent throughout the application.

**Rationale**: Consistency reduces cognitive load. AJ should not have to relearn how the interface works in different contexts.

**Compliance Examples**:
- Same keyboard shortcuts work everywhere
- Error messages follow consistent format
- Avatar behavior patterns are predictable
- Settings organization follows logical grouping
- Terminology matches [Terminology Dictionary](agents/ai-way-docs/terminology-dictionary.md)

**Violation Examples**:
- Different confirmation dialogs in different contexts
- Inconsistent use of icons vs text
- Varying keyboard shortcuts per screen
- Technical jargon in user-facing messages

---

### 3.2 Simplicity Over Features

**Description**: Prefer fewer, well-designed features over many half-baked ones.

**Rationale**: AJ is not a power user. Every feature adds complexity. Features should earn their place by solving real problems well.

**Compliance Examples**:
- Single, clear way to accomplish common tasks
- Progressive disclosure of advanced options
- Defaults that work for 80% of use cases
- Features removed if they cause confusion

**Violation Examples**:
- Multiple ways to do the same thing
- Power-user features exposed to all users
- Settings with 50 options on one screen
- Features added "because we can"

---

### 3.3 Progressive Disclosure

**Description**: Show only what is needed at each moment. Advanced details are available on request.

**Rationale**: AJ does not need to see model architecture details to get help. Technical users can dig deeper. Both should be served.

**Compliance Examples**:
- Default: "Working with Llama 3.1..."
- On hover/request: Full model card with license, source, capabilities
- Error: Simple message first, "Show details" for technical info
- Avatar: Simple presence, detailed animation on focus

**Violation Examples**:
- Showing model parameters on every response
- Error stack traces in user-facing messages
- All settings visible at once
- No way to access detailed information when needed

---

### 3.4 Yollayah Avatar Constraints

**Description**: The avatar is blocky by design. This aesthetic must be preserved across all surfaces.

**Rationale**: Yollayah's character is defined by the block aesthetic. Smoothing or anti-aliasing destroys the intentional design.

**Compliance Examples**:
- Block is the atomic rendering unit on all surfaces
- HD surfaces scale blocks uniformly (no fractional blocks)
- Partial rendering is a feature (peeking, zoomed, cropped)
- Animation evolution reflects context and time

**Violation Examples**:
- Anti-aliasing block edges on HD displays
- Smoothing transitions between sprites
- Fixed avatar size regardless of context
- Static animation without evolution

**Reference**: [Yollayah Avatar Constraints](docs/yollayah-avatar-constraints.md)

---

### 3.5 Error Messages That Help

**Description**: Errors should explain what happened, why it matters, and what the user can do.

**Rationale**: "Error: CUDA OOM" means nothing to AJ. "Something went wrong - try a smaller file" is actionable.

**Compliance Examples**:
- "I couldn't understand that file. Try a PDF or text file instead."
- "The AI is taking longer than usual. Want to wait or try again?"
- "I can't help with that right now. Here's what you could try..."
- Clear next steps in every error state

**Violation Examples**:
- Raw exception messages
- "Error occurred" with no context
- Technical codes without explanation
- Dead ends with no suggested action

---

## 4. Legal and Compliance Principles

### 4.1 Open Source Licensing Integrity

**Description**: ai-way is AGPL-3.0. All dependencies and bundled models must be compatible.

**Rationale**: License violations expose the project and users to legal risk. Open source freedom must be preserved.

**Compliance Examples**:
- Bundled models use permissive licenses (Apache 2.0, MIT)
- Dependencies audited for license compatibility
- No GPL dependencies without review
- License disclosed for each model in UI

**Violation Examples**:
- Including proprietary model weights
- Using GPL library without understanding implications
- No license tracking for new dependencies
- Bundling assets with unclear licensing

---

### 4.2 Dependency Hygiene

**Description**: External dependencies are security liabilities. Minimize them, audit them, and update them.

**Rationale**: Supply chain attacks are a real threat. Every dependency is attack surface.

**Compliance Examples**:
- Regular dependency audits (cargo audit)
- Pinned versions in production
- Security advisories monitored
- SafeTensors format only (no pickle)
- Reproducible builds goal

**Violation Examples**:
- Adding dependencies without justification
- Ignoring security advisories
- Using abandoned/unmaintained crates
- Loading arbitrary model files from user

**Reference**: [Supply Chain Risks](agents/dangers/SUPPLY_CHAIN_RISKS.md)

---

### 4.3 Data Handling Compliance

**Description**: Data handling must comply with GDPR, CCPA, and similar regulations by design.

**Rationale**: AJ does not know about regulations. We must build compliance into the architecture so AJ is protected automatically.

**Compliance Examples**:
- Right to Access: User can see all their data (it is in the app)
- Right to Deletion: Close app = data deleted
- Right to Portability: Drag files out = data export
- Data Minimization: Only process what user provides
- Ephemeral by default: No persistence without explicit user action

**Violation Examples**:
- Storing user data without retention policy
- No way for user to see what data exists
- Persistent storage without user consent
- Data retained after session ends

**Reference**: [Privacy-First Architecture](agents/ai-way-docs/privacy-first-architecture.md)

---

### 4.4 Content and Output Responsibility

**Description**: AI-generated content carries risks. We must disclose limitations and encourage verification.

**Rationale**: AJ may not understand that AI can be wrong. We must surface this clearly without being paternalistic.

**Compliance Examples**:
- Image generation includes AI disclosure (C2PA or visible)
- Code outputs warn about review before use
- Financial calculations suggest professional verification
- Copyright status of AI outputs disclosed

**Violation Examples**:
- AI-generated images presented as photographs
- Generated code deployed without review warning
- Medical advice without professional consultation warning
- No attribution requirements disclosed

---

## 5. Application to Development Phases

### 5.1 Planning Phase

**Questions to Ask**:
- Does this feature align with the Constitution's Four Protections?
- Who benefits from this feature? Does it serve AJ's genuine interests?
- What privacy implications does this feature have?
- Is this feature accessible to users with disabilities?
- What security threats does this feature introduce?

**Required Artifacts**:
- User story from AJ's perspective
- Privacy impact assessment for data-handling features
- Accessibility requirements for UI features
- Security considerations for new attack surface

---

### 5.2 Development Phase

**Questions to Ask**:
- Have I read the relevant constraint documents?
- Does my code include tests for the new behavior?
- Are error messages helpful to AJ?
- Am I introducing new dependencies? Are they justified?
- Is this code readable by future developers (human and AI)?

**Required Practices**:
- Reference principles in code comments when relevant
- Include accessibility attributes (ARIA roles, keyboard handlers)
- Validate and sanitize all inputs
- Log security-relevant events (not user data)

---

### 5.3 Code Review Phase

**Questions to Ask**:
- Does this change violate any principle in this document?
- Are there security implications not addressed?
- Is accessibility considered?
- Are tests sufficient to prevent regressions?
- Would AJ's wisest future self approve of this?

**Review Checklist**:
- [ ] No network calls without explicit consent path
- [ ] No persistent storage without user action
- [ ] Keyboard navigation works
- [ ] Error messages are user-friendly
- [ ] Tests cover the new functionality
- [ ] No new dependencies without justification

---

### 5.4 Testing Phase

**Questions to Ask**:
- Can this feature be used with keyboard only?
- Does this work with screen readers?
- What happens when resources are constrained?
- What happens when the network is unavailable?
- What happens with malicious input?

**Required Tests**:
- Unit tests for core logic
- Integration tests for component interactions
- Accessibility tests (automated where possible)
- Security tests for sensitive paths
- Edge case tests for error handling

---

### 5.5 Release Phase

**Questions to Ask**:
- Have all principle violations been addressed?
- Have security findings been reviewed and triaged?
- Is the release accessible?
- Are all dependencies audited?
- Is documentation updated?

**Release Blockers**:
- Any CRITICAL security finding
- Accessibility regression from previous version
- Privacy violation in new feature
- Unreviewed dependency addition

---

## 6. Principle Violation Handling

### 6.1 Discovery

When a principle violation is discovered:

1. **Document** the violation in the appropriate TODO or issue tracker
2. **Assess** severity (blocker, high, medium, low)
3. **Notify** relevant stakeholders (security team for security issues)
4. **Prioritize** based on user impact

### 6.2 Triage

| Severity | Definition | Response |
|----------|------------|----------|
| **Blocker** | Active harm to users | Immediate fix, block release |
| **High** | Significant principle violation | Fix in current sprint |
| **Medium** | Partial violation, workaround exists | Fix within 2-3 sprints |
| **Low** | Minor violation, theoretical risk | Backlog |

### 6.3 Resolution

When resolving a violation:

1. Fix the immediate issue
2. Add tests to prevent regression
3. Consider if the violation reveals a systemic problem
4. Update this document if the principle needs clarification

### 6.4 No Exceptions

These principles are not configurable. Not by users. Not by developers. Not by investors. Not by governments. They are the definition of what ai-way is.

If you find yourself thinking "we'll fix this later" or "this is fine for now," stop. The principles apply now, not later.

**There is no "v1 without principles, v2 with principles."** Principles are not a feature. They are the foundation.

---

## 7. Related Documents

### Foundational
| Document | Purpose |
|----------|---------|
| [CONSTITUTION.md](agents/CONSTITUTION.md) | Immutable ethical foundation (Five Laws, Four Protections) |
| [Average Joe Persona](agents/personas/average-joe.md) | Who we build for |

### Technical
| Document | Purpose |
|----------|---------|
| [Privacy-First Architecture](agents/ai-way-docs/privacy-first-architecture.md) | Appliance model, ephemeral design |
| [Multi-Model Ethics](agents/ai-way-docs/multi-model-ethics-guidelines.md) | Model selection, transparency, warnings |
| [Avatar Constraints](docs/yollayah-avatar-constraints.md) | Yollayah visual design requirements |
| [Terminology Dictionary](agents/ai-way-docs/terminology-dictionary.md) | User-friendly language |

### Security
| Document | Purpose |
|----------|---------|
| [Security Findings](TODO-security-findings.md) | Active vulnerabilities and tracking |
| [Supply Chain Risks](agents/dangers/SUPPLY_CHAIN_RISKS.md) | Dependency and model security |
| [Data Leaks](agents/dangers/DATA_LEAKS.md) | Data exfiltration vectors |

### Development
| Document | Purpose |
|----------|---------|
| [TODO-accessibility.md](TODO-accessibility.md) | Accessibility roadmap |
| [TODO-main.md](TODO-main.md) | Current development priorities |

---

## Document History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-01-02 | Initial specification |

---

**Status**: Ratified
**Maintainers**: Architect, Lawyer, Ethics Advisor
**Review Cadence**: Quarterly or when Constitution is amended

---

*"We are not building a tool. We are building a bridge. Cross it well."*
