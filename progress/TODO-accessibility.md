# TODO: Accessibility Roadmap

Comprehensive accessibility improvements for ai-way TUI and Conductor.

**Status**: Active
**Created**: 2026-01-01
**WCAG Target**: AA Compliance (currently ~25%)

---

## Executive Summary

The ai-way codebase has strong accessibility infrastructure in `conductor/core/src/accessibility.rs` (semantic announcements, ARIA roles, urgency levels), but **zero TUI-level implementation**. This document tracks the work to achieve WCAG AA compliance.

**Current State**: ~25% WCAG AA compliant
**Critical Gaps**: 10 major areas
**Estimated Effort**: 12 weeks for full compliance

---

## User Impact Assessment

| User Group | Population | Critical Features Needed |
|------------|------------|--------------------------|
| Blind/Low Vision | 285M | Screen reader, TTS, high contrast |
| Motor Disability | 1B | Keyboard navigation, focus management |
| Colorblind | 400M | Safe palettes, pattern-based indicators |
| Motion Sensitive | 5% | Reduced motion, static avatar option |
| Cognitive Disability | Varies | Clear messages, help system, confirmations |
| Mobile/Small Screen | Growing | Responsive layout, minimal width support |
| RTL Languages | 5% | Arabic, Hebrew, Persian support |

---

## Phase 1: Foundation [PRIORITY: CRITICAL]

### P1.1 Accessibility Config System
- [ ] Create `tui/src/accessibility/mod.rs` module
- [ ] Implement `AccessibilityConfig` struct:
  ```rust
  pub struct AccessibilityConfig {
      pub reduce_motion: bool,
      pub high_contrast: bool,
      pub colorblind_mode: ColorblindMode,
      pub text_direction: TextDirection,
      pub tts_enabled: bool,
      pub screen_reader_mode: bool,
  }
  ```
- [ ] Detect environment variables:
  - `REDUCE_MOTION=1` - disable animations
  - `FORCE_COLOR=0` - high contrast
  - `A11Y_HIGH_CONTRAST=1` - high contrast mode
  - `A11Y_COLORBLIND=deuteranopia|protanopia|tritanopia`
- [ ] Load config on startup
- [ ] Add tests for config detection

### P1.2 Color Interpolation Infrastructure
- [ ] Create `tui/src/color_animation/interpolator.rs`
- [ ] Implement RGB interpolation
- [ ] Implement alpha blending
- [ ] Add breathing effect sine wave functions
- [ ] Tests for interpolation accuracy

### P1.3 Reduced Motion Support [QUICK WIN]
- [ ] Check `REDUCE_MOTION` environment variable
- [ ] Add `AnimationEngine::set_disabled(bool)` method
- [ ] Skip animation frame advancement when disabled
- [ ] Keep avatar visible but static
- [ ] Test with motion-sensitive users

---

## Phase 2: Keyboard Navigation [PRIORITY: CRITICAL]

### P2.1 Focus Management
- [ ] Create `FocusManager` struct:
  ```rust
  pub enum UiFocus {
      Input,
      ConversationView,
      TaskPanel,
  }
  ```
- [ ] Implement Tab/Shift+Tab cycling
- [ ] Track current focus state
- [ ] Announce focus changes to status bar

### P2.2 Focus Indicators
- [ ] Highlight focused element with bright color/border
- [ ] Add "[ ]" brackets or ">>" prefix to indicate focus
- [ ] Test focus visibility on all themes

### P2.3 Extended Keyboard Commands
| Key | Action | Status |
|-----|--------|--------|
| Tab | Next focus area | [ ] |
| Shift+Tab | Previous focus | [ ] |
| Alt+H | Help / Key bindings | [ ] |
| Alt+T | Toggle task panel | [ ] |
| Alt+A | Toggle avatar | [ ] |
| Ctrl+L | Clear conversation | [ ] |
| Up/Down | Navigate history | [ ] |
| Escape | Quit (with confirm) | [ ] |

### P2.4 Confirmation Dialogs
- [ ] Quit confirmation: "Press Y to confirm, N to cancel"
- [ ] Clear confirmation for destructive actions
- [ ] Prevent accidental data loss

### P2.5 Help System
- [ ] Create help overlay (Alt+H or F1)
- [ ] List all keyboard commands
- [ ] Provide examples and tips
- [ ] Make discoverable (show hint in status bar)

---

## Phase 3: Screen Reader & TTS [PRIORITY: CRITICAL]

### P3.1 TTS Abstraction Layer
- [ ] Create `tui/src/accessibility/tts.rs`
- [ ] Implement `TextToSpeech` trait:
  ```rust
  pub trait TextToSpeech: Send + Sync {
      async fn speak(&self, text: &str, urgency: Urgency);
      fn set_rate(&mut self, rate: f32);
  }
  ```
- [ ] Platform-specific implementations:
  - Linux: `espeak-ng` or `festival`
  - macOS: `say` command
  - Windows: SAPI via subprocess

### P3.2 Message Announcement Queue
- [ ] Queue announcements by urgency
- [ ] Immediate: Interrupt current speech (errors)
- [ ] Normal: Queue after current (messages)
- [ ] Low: Background (status updates)
- [ ] Don't announce streaming tokens (too verbose)

### P3.3 Sensitive Data Filtering [SECURITY]
- [ ] Create regex patterns for common secrets:
  - API keys (`sk_live_`, `AKIA`, etc.)
  - JWTs (`eyJ...`)
  - Passwords (explicit labels)
- [ ] Replace with "[REDACTED]" before TTS
- [ ] Add configuration for content types to speak
- [ ] Test filtering accuracy

### P3.4 Screen Reader Integration
- [ ] Implement `ScreenReaderOutput` trait
- [ ] Announce focus changes
- [ ] Announce new messages
- [ ] Announce status changes
- [ ] Test with NVDA, VoiceOver, Orca

---

## Phase 4: Visual Accessibility [PRIORITY: HIGH]

### P4.1 High Contrast Mode [QUICK WIN]
- [ ] Create alternative color palette:
  ```rust
  pub fn high_contrast_palette() -> HashMap<ColorRole, Color> {
      // Pure black/white with high saturation accents
  }
  ```
- [ ] Detect via environment variable
- [ ] Add toggle (Alt+C or similar)
- [ ] Test with WCAG contrast checker (4.5:1 minimum)

### P4.2 Colorblind Modes
- [ ] Implement `ColorblindMode` enum:
  ```rust
  pub enum ColorblindMode {
      None,
      Deuteranopia,  // Red-green blind
      Protanopia,    // Red-green blind (different)
      Tritanopia,    // Blue-yellow blind
  }
  ```
- [ ] Create safe palettes for each mode
- [ ] Add pattern-based indicators (not color alone):
  - User: "[U]" prefix + green
  - Assistant: "[Y]" prefix + yellow
  - Error: "[!]" prefix + red
  - Success: "[+]" prefix + blue
- [ ] Test with Coblis/Vischeck tools

### P4.3 Pattern-Based Indicators
- [ ] Progress bars: Use symbols (`#`, `=`, `-`)
- [ ] Status: Text + color (not color alone)
- [ ] Task states: Icons + text
  - Pending: "..." / Cyan
  - Running: ">>>" / Blue
  - Done: "[+]" / Green
  - Failed: "[!]" / Red

### P4.4 Scroll Gradient Accessibility
- [ ] Ensure gradients meet contrast requirements
- [ ] Use Unicode indicators (▲▼) with text fallback
- [ ] Test on low-contrast terminals

---

## Phase 5: Internationalization [PRIORITY: MEDIUM]

### P5.1 RTL Language Detection
- [ ] Detect RTL characters (Arabic, Hebrew, Persian)
- [ ] Use `unicode-bidi` crate for detection
- [ ] Store direction in `AccessibilityConfig`

### P5.2 RTL Layout Mirroring
- [ ] Mirror conversation/avatar positions
- [ ] Flip progress bar direction
- [ ] Mirror scroll indicators
- [ ] Test with Arabic/Hebrew sample text

### P5.3 Configurable Role Labels
- [ ] Make "You:", "Yollayah:" labels configurable
- [ ] Support abbreviated labels for small screens
- [ ] Support translated labels
- [ ] Support RTL label placement

---

## Phase 6: Mobile & Small Screen [PRIORITY: MEDIUM]

### P6.1 Responsive Layout System
- [ ] Detect screen size categories:
  ```rust
  enum ScreenSize {
      Mobile,   // < 40 columns, < 15 lines
      Tablet,   // 40-100 columns, 15-30 lines
      Desktop,  // >= 100 columns, >= 30 lines
  }
  ```
- [ ] Implement layout constraints per size

### P6.2 Mobile Layout
- [ ] Hide avatar on very small screens
- [ ] Single-line input
- [ ] Messages only (collapse task panel)
- [ ] Abbreviated role prefixes ("U:", "Y:")

### P6.3 Tablet Layout
- [ ] Smaller avatar (Tiny/Small)
- [ ] Side-by-side conversation + tasks
- [ ] 3-line input area

---

## Phase 7: Advanced Features [PRIORITY: LOW]

### P7.1 Braille Display Support
- [ ] Create `BrailleFormatter` struct
- [ ] Linearize conversation for braille
- [ ] Add paragraph structure markers
- [ ] Spell out status indicators
- [ ] Test with liblouis

### P7.2 Cognitive Accessibility
- [ ] Clear status messages (full sentences)
- [ ] Predictable behavior (announce avatar wandering)
- [ ] Better error messages (suggest solutions)
- [ ] Consistent terminology throughout

---

## WCAG 2.1 Compliance Tracker

### Level A (Minimum)

| Criterion | Status | Notes |
|-----------|--------|-------|
| 1.1.1 Non-text Content | ❌ | Avatar needs alt text |
| 1.3.1 Info and Relationships | ⚠️ | Structure implied, not semantic |
| 1.4.1 Use of Color | ❌ | Color used alone for errors |
| 2.1.1 Keyboard | ⚠️ | Basic support, incomplete |
| 2.1.2 No Keyboard Trap | ✅ | Esc always works |
| 2.4.1 Bypass Blocks | ❌ | No skip navigation |
| 2.4.3 Focus Order | ❌ | No visible focus |
| 3.1.1 Language of Page | ❌ | No language metadata |
| 3.2.1 On Focus | ✅ | No unexpected behavior |
| 3.3.1 Error Identification | ⚠️ | Errors shown, not always clear |
| 4.1.1 Parsing | ✅ | Valid Rust |

### Level AA (Target)

| Criterion | Status | Notes |
|-----------|--------|-------|
| 1.4.3 Contrast (Minimum) | ❌ | Some colors fail 4.5:1 |
| 1.4.4 Resize Text | ⚠️ | Terminal limits |
| 2.4.7 Focus Visible | ❌ | No focus ring |
| 3.3.4 Error Prevention | ❌ | No confirmation dialogs |
| 4.1.2 Name, Role, Value | ⚠️ | Core exists, not exposed |

---

## Testing Strategy

### Automated Testing
```rust
#[test]
fn test_high_contrast_meets_wcag_aa() { }

#[test]
fn test_colorblind_mode_uses_patterns() { }

#[test]
fn test_keyboard_can_navigate_all_functions() { }

#[test]
fn test_tts_filters_sensitive_data() { }

#[test]
fn test_focus_indicators_visible() { }

#[test]
fn test_mobile_layout_fits_40_columns() { }

#[test]
fn test_rtl_layout_mirrors_correctly() { }
```

### Manual Testing Checklist
- [ ] Test with keyboard only (no mouse)
- [ ] Test with NVDA (Windows)
- [ ] Test with VoiceOver (macOS)
- [ ] Test with Orca (Linux)
- [ ] Test with colorblind simulation tools
- [ ] Test on 40x15 terminal
- [ ] Test with Arabic/Hebrew text

### User Testing
- [ ] Blind/VI user testing
- [ ] Motor disability user testing
- [ ] Colorblind user testing
- [ ] Motion-sensitive user testing
- [ ] Cognitive accessibility testing

---

## Security Considerations

### TTS Data Leakage (CRITICAL)
- **Risk**: Speaking sensitive data (API keys, passwords)
- **Mitigation**: Regex filtering before TTS
- **Test**: Verify secrets are redacted

### Accessibility Settings Fingerprinting
- **Risk**: Preferences could identify users
- **Mitigation**: Store locally only, don't send to server

### Screen Reader Process Security
- **Risk**: SR subprocess could be monitored
- **Mitigation**: Use platform-native TTS, document risks

---

## Quick Wins (< 1 day each)

1. **Reduced Motion** - Check env var, skip animations
2. **Quit Confirmation** - Simple Y/N prompt
3. **Help System** - Alt+H shows key bindings
4. **High Contrast Env Var** - Switch palette on detection
5. **Pattern Indicators** - Add [U], [Y], [!] prefixes

---

## Dependencies

- `unicode-bidi` - RTL detection
- `espeak-ng` (Linux) - TTS
- `lru` - Caching for performance
- Existing: `conductor/core/src/accessibility.rs` - Message semantics

---

## Related Documents

- [TODO-conductor-ux-split.md](TODO-conductor-ux-split.md) - Conductor architecture
- [TODO-meta-agent-conductor-interactions.md](TODO-meta-agent-conductor-interactions.md) - Agent control
- [TODO-implementation-plan.md](TODO-implementation-plan.md) - Implementation phases

---

**Last Updated**: 2026-01-01
