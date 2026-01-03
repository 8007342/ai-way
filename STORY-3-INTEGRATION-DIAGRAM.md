# STORY 3 - Integration Flow Diagram

## Complete Dirty Tracking Flow

```
┌─────────────────────────────────────────────────────────────┐
│                     EPIC-002 STORY 3                        │
│              Dirty Tracking Integration                     │
└─────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│  STEP 1: User Event / State Change                          │
└──────────────────────────────────────────────────────────────┘
                          │
                          ├─► User types → input_buffer changes
                          ├─► Message arrives → display.messages changes
                          ├─► Status changes → conductor_state changes
                          ├─► Avatar animates → frame time elapsed
                          └─► Tasks update → display.tasks changes
                          │
                          ▼
┌──────────────────────────────────────────────────────────────┐
│  STEP 2: App::render() Called (Every Frame)                 │
│                                                              │
│  fn render(&mut self, terminal: &mut Terminal) {            │
│      self.render_conversation();  ──┐                       │
│      self.render_tasks();          ─┤                       │
│      self.render_input();           │                       │
│      self.render_status();         ─┤                       │
│      self.render_avatar();         ─┘                       │
│                                                              │
│      terminal.draw(|frame| {                                │
│          let output = self.compositor.composite();          │
│          frame.buffer_mut().merge(output);                  │
│      })?;                                                    │
│  }                                                           │
└──────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌──────────────────────────────────────────────────────────────┐
│  STEP 3: Each Render Function Updates Layer Buffer          │
│          (STORY 3 Implementation)                            │
└──────────────────────────────────────────────────────────────┘
                          │
         ┌────────────────┼────────────────┬──────────────────┐
         │                │                │                  │
         ▼                ▼                ▼                  ▼

┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│ Conversation │  │    Input     │  │   Status     │  │   Avatar     │
│   Layer      │  │    Layer     │  │    Layer     │  │    Layer     │
├──────────────┤  ├──────────────┤  ├──────────────┤  ├──────────────┤
│ Render msgs  │  │ Render input │  │ Render bar   │  │ Render axol  │
│ to buffer    │  │ to buffer    │  │ to buffer    │  │ to buffer    │
│              │  │              │  │              │  │              │
│ ✅ Line 998: │  │ ✅ Line 1069:│  │ ✅ Line 1184:│  │ ✅ Line 1265:│
│ mark_layer_  │  │ mark_layer_  │  │ mark_layer_  │  │ mark_layer_  │
│ dirty(conv)  │  │ dirty(input) │  │ dirty(status)│  │ dirty(avatar)│
└──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘
         │                │                │                  │
         └────────────────┴────────────────┴──────────────────┘
                          │
                          ▼
┌──────────────────────────────────────────────────────────────┐
│  STEP 4: Compositor Tracks Dirty Layers                     │
│          (Developer 1's Infrastructure)                      │
│                                                              │
│  struct Compositor {                                         │
│      dirty_layers: HashSet<LayerId>,  // ← Tracks dirty     │
│      ...                                                     │
│  }                                                           │
│                                                              │
│  pub fn mark_layer_dirty(&mut self, id: LayerId) {          │
│      self.dirty_layers.insert(id);    // ← Marks dirty      │
│  }                                                           │
└──────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌──────────────────────────────────────────────────────────────┐
│  STEP 5: Compositor::composite() Called                     │
│          (Optimized by Developer 1)                          │
│                                                              │
│  pub fn composite(&mut self) -> &Buffer {                   │
│      // ✅ OPTIMIZATION: Skip if nothing dirty              │
│      if self.dirty_layers.is_empty() {                      │
│          return &self.output;  // ← Cached!                 │
│      }                                                       │
│                                                              │
│      // Re-composite all visible layers                     │
│      self.output.reset();                                   │
│      for &id in &self.render_order {                        │
│          if let Some(layer) = self.layers.get(&id) {        │
│              if layer.visible {                             │
│                  Self::blit_layer(&mut self.output,         │
│                                   &self.area, layer);       │
│              }                                               │
│          }                                                   │
│      }                                                       │
│                                                              │
│      self.dirty_layers.clear();  // ← Reset for next frame  │
│      &self.output                                            │
│  }                                                           │
└──────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌──────────────────────────────────────────────────────────────┐
│  STEP 6: Ratatui Terminal::draw() Called                    │
│          (Framework Level)                                   │
│                                                              │
│  frame.buffer_mut().merge(output);                          │
│  // ✅ Ratatui's cell-level diffing (EPIC-001)              │
│  // Only writes changed cells to terminal                   │
└──────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌──────────────────────────────────────────────────────────────┐
│  RESULT: Terminal Updated Efficiently                       │
│                                                              │
│  Layer Dirty Tracking (EPIC-002) Handles:                   │
│  ✅ Skip compositor work when nothing changed               │
│  ✅ Track which layers need re-compositing                  │
│  ✅ Maintain Z-order correctness                            │
│                                                              │
│  Cell-Level Diffing (EPIC-001) Handles:                     │
│  ✅ Skip terminal writes for unchanged cells                │
│  ✅ Reduce actual terminal I/O                              │
└──────────────────────────────────────────────────────────────┘
```

---

## Performance Impact (Current vs Future)

### Current State (STORY 3 Complete)
```
Frame 1:
  render_conversation() → marks conversation dirty
  render_input()        → marks input dirty
  render_status()       → marks status dirty
  render_tasks()        → marks tasks dirty
  render_avatar()       → marks avatar dirty

  compositor.composite():
    dirty_layers = {conversation, input, status, tasks, avatar}
    ✅ Re-composes all 5 layers

  terminal.draw():
    ✅ Ratatui diffs cells, writes changes

RESULT: Full re-composite every frame (no optimization yet)
        BUT: Infrastructure in place for STORY 4!
```

### After STORY 4 (Conditional Rendering)
```
Idle Frame (no user input, no messages):
  ❌ render_conversation() SKIPPED (messages unchanged)
  ❌ render_input()        SKIPPED (input unchanged)
  ❌ render_status()       SKIPPED (status unchanged)
  ❌ render_tasks()        SKIPPED (tasks unchanged)
  ✅ render_avatar()       → marks avatar dirty (animation)

  compositor.composite():
    dirty_layers = {avatar}
    ✅ Re-composes all layers (1 dirty)

  terminal.draw():
    ✅ Ratatui diffs cells, writes only avatar changes

RESULT: 80% less work (1 layer vs 5 layers)
        CPU: ~10% → <2%
```

---

## Code Locations

### Compositor Infrastructure (Developer 1)
- **File**: `/var/home/machiyotl/src/ai-way/tui/src/compositor/mod.rs`
- **Lines**:
  - 35: `dirty_layers: HashSet<LayerId>` field
  - 126-132: `mark_layer_dirty()` method
  - 153-179: Optimized `composite()` method

### App Dirty Marking (Developer 2 - STORY 3)
- **File**: `/var/home/machiyotl/src/ai-way/tui/src/app.rs`
- **Lines**:
  - 998: `render_conversation()` dirty marking
  - 1069: `render_input()` dirty marking
  - 1184: `render_status()` dirty marking
  - 1207: `render_tasks()` dirty marking
  - 1265: `render_avatar()` dirty marking

---

## Next Steps (STORY 4)

Add conditional rendering to only call render functions when needed:

```rust
struct App {
    // NEW: Track what changed since last frame
    conversation_dirty: bool,
    input_dirty: bool,
    status_dirty: bool,
    tasks_dirty: bool,
    // Note: Avatar always dirty (animation)
}

fn render(&mut self, terminal: &mut Terminal) -> Result<()> {
    // Only render layers that changed
    if self.conversation_dirty {
        self.render_conversation();
        self.conversation_dirty = false;
    }

    if self.input_dirty {
        self.render_input();
        self.input_dirty = false;
    }

    // Avatar ALWAYS renders (animation)
    self.render_avatar();

    // Compositor handles the rest
    terminal.draw(|frame| {
        let output = self.compositor.composite();
        frame.buffer_mut().merge(output);
    })?;

    Ok(())
}
```

Set flags when state changes:
```rust
fn process_conductor_messages(&mut self) {
    for msg in self.conductor.recv_all() {
        self.display.apply_message(msg);

        // Mark what changed
        self.conversation_dirty = true;  // Messages updated
        self.status_dirty = true;        // State might have changed
    }
}

fn handle_key(&mut self, key: KeyEvent) {
    // ... handle input ...
    self.input_dirty = true;  // Input changed
}
```

This will make the dirty tracking optimization actually effective!
