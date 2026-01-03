# TODO: Coherent Evolving Mood System - Initial Design

**Created**: 2026-01-03
**Priority**: P2 - Feature Development
**Status**: 🔵 PROPOSED - Awaiting Planning

---

## Overview

Design and implement a mood-based animation system that makes Yollayah's avatar evolve over time based on context, user interaction, and elapsed time. Animations should respond intelligently to the conversation state and provide emotional feedback to Average Joe (AJ).

---

## Vision

> "This animation is stale, make it look bored now."
> "User just got a great response - make it happy!"
> "Long processing time - show patience/thinking."

The avatar should feel **alive** and **responsive**, not just looping the same animation mechanically.

---

## Goals

1. **Define mood states** - Happy, thinking, waiting, bored, excited, patient, etc.
2. **Mood transition system** - How moods change based on triggers
3. **Animation selection** - Map moods to sprite animations
4. **Metadata system** - Track animation staleness and context

---

## Mood States (Initial Set)

### Primary Moods
1. **Idle** - Default state, gentle breathing animation
2. **Thinking** - Processing user request, active state
3. **Waiting** - Waiting for external resource (Ollama, network)
4. **Happy** - Successful completion, positive feedback
5. **Bored** - Animation has been playing too long without change
6. **Patient** - Long-running operation, still working

### Transition Triggers
- **User message received** → Idle → Thinking
- **Processing started** → Thinking or Waiting (based on operation type)
- **Response generated** → Happy (if successful) or Idle (if neutral)
- **Time elapsed** → Any state → Bored (after threshold)
- **Long operation** → Waiting → Patient

---

## Technical Design

### Mood State Machine

```rust
enum Mood {
    Idle,
    Thinking,
    Waiting,
    Happy,
    Bored,
    Patient,
}

struct MoodState {
    current: Mood,
    entered_at: Instant,
    transition_count: u32,
    last_trigger: MoodTrigger,
}
```

### Mood Triggers

```rust
enum MoodTrigger {
    UserMessage,
    ProcessingStarted,
    ProcessingCompleted,
    TimeElapsed(Duration),
    LongOperation,
    SuccessfulResponse,
}
```

### Animation Metadata

Each animation should have metadata:
```rust
struct AnimationMetadata {
    mood: Mood,
    duration: Duration,
    staleness_threshold: Duration, // When to transition to Bored
    can_loop: bool,
    priority: u8, // For mood conflicts
}
```

---

## Coherence Requirements

### Temporal Coherence
- **Smooth transitions** - Don't snap between moods, fade or blend
- **Staleness detection** - If same animation plays > threshold → Bored
- **Context awareness** - Remember recent mood history

### Behavioral Coherence
- **Match user experience** - If LLM is stuck, show patience not happiness
- **Respond to failures** - Different mood for errors vs successes
- **Progressive boredom** - Longer wait → more bored variations

---

## Implementation Phases

### Phase 1: Basic Mood States (This TODO)
1. Define `Mood` enum and basic states
2. Implement mood state machine
3. Add basic triggers (user message, processing, completion)
4. Test mood transitions in TUI

### Phase 2: Animation Mapping
1. Create animation metadata system
2. Map moods to sprite animations
3. Implement animation selection logic
4. Test with TODO-sprites-init animations

### Phase 3: Staleness & Evolution
1. Add staleness tracking
2. Implement Bored mood transitions
3. Create variation system (slight differences in repeated animations)
4. Test long-running scenarios

### Phase 4: Advanced Moods
1. Add Patient, Frustrated, Confused moods
2. Implement context-aware transitions
3. Add mood history tracking
4. Fine-tune thresholds based on user feedback

---

## Design Decisions

### Where does mood state live?
**Answer**: Conductor, not TUI.

- TUI is a surface, should be stateless
- Conductor owns conversation context
- Mood is part of conversation state
- TUI receives mood updates via messages

### How are animations synchronized?
**Answer**: Conductor sends mood + timestamp, TUI selects animation.

- Conductor: "Switch to Thinking mood at T=1234"
- TUI: Looks up Thinking animations, selects based on staleness
- TUI: Sends back "Playing animation X, duration Y" (for staleness tracking)

### How to handle mood conflicts?
**Answer**: Priority system + recency.

- Each mood has priority (Happy > Bored, Thinking > Idle)
- Recent explicit triggers override staleness
- User-facing feedback (Happy, Patient) takes priority over ambient (Idle, Bored)

---

## Staleness Thresholds (Initial Values)

| Mood | Staleness Threshold | Bored Transition |
|------|---------------------|------------------|
| Idle | 30 seconds | Bored (Idle variant) |
| Thinking | 10 seconds | Patient |
| Waiting | 15 seconds | Patient |
| Happy | 5 seconds | Idle |
| Patient | 60 seconds | Bored (Patient variant) |
| Bored | N/A | Stays bored |

**Note**: These are starting values, tune based on user testing.

---

## Dependencies

- **Blocks**: None
- **Blocked by**: TODO-sprites-init.md (need animations to map to moods)
- **Enables**: Rich avatar personality system

---

## Acceptance Criteria

- ✅ Mood enum and state machine implemented
- ✅ Basic triggers (user message, processing, completion) working
- ✅ Mood state lives in Conductor
- ✅ TUI receives mood updates and switches animations
- ✅ Staleness tracking functional
- ✅ Bored transitions trigger after threshold
- ✅ Documentation for adding new moods

---

## Related Documents

- **Design**: `facts/design/yollayah-avatar-constraints.md` - Avatar design constraints
- **Sprites**: `progress/active/TODO-sprites-init.md` - Sprite system foundation
- **Epic**: `progress/active/TODO-epic-2026Q1-avatar-animation.md` - Avatar animation roadmap

---

## Open Questions

1. **Should mood be persistent across sessions?**
   - Could load last mood on startup
   - Or always start with Idle?
   - **Decision needed**: User preference?

2. **How to handle multiple concurrent operations?**
   - Example: Thinking (LLM) + Waiting (network) simultaneously
   - **Proposed**: Stack moods, highest priority wins

3. **Should animations have audio cues?**
   - Terminal beeps for mood changes?
   - **Decision**: Out of scope for init, revisit later

---

## Notes

- This is exploratory work - expect iteration!
- Focus on AJ experience: does the avatar feel responsive and alive?
- Keep it simple initially, add complexity as needed
- Consider accessibility: mood changes should be observable without color

---

**Next Steps**: Review design, get feedback from UX team, implement Phase 1 when TODO-sprites-init is complete.
