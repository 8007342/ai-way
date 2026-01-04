# TODO-STORY-Conductor-Warmup-Elimination

**Status**: 🔴 CRITICAL - Ready to Implement
**Created**: 2026-01-03
**Priority**: P0
**Team**: Rust Specialist + Async Expert
**Estimated Effort**: 30 minutes

---

## Navigation

**Parent**: [TODO-EPIC-conductor-reactive-overhaul.md](TODO-EPIC-conductor-reactive-overhaul.md)
**Siblings**:
- [TODO-STORY-conductor-timeout-optimization.md](TODO-STORY-conductor-timeout-optimization.md)
- [TODO-STORY-conductor-block-on-fix.md](TODO-STORY-conductor-block-on-fix.md)
**Children**: None
**QA**: [TODO-QA-verify-STORY-conductor-warmup.md](TODO-QA-verify-STORY-conductor-warmup.md) _(when complete)_

---

## Goal

**Eliminate warmup function to remove initialization overhead.**

### Why This Matters

The warmup function:
1. Sends actual LLM request during startup (blocking)
2. Uses `rx.recv().await` which blocks until complete
3. Adds 2-5 seconds to startup time
4. **Is completely unnecessary** - Ollama has keep_alive

---

## Changes Required

### File: `yollayah/conductor/core/src/conductor.rs`

#### Change 1: Remove warmup() function (lines 400-431)

```diff
-    /// Warm up the model
-    async fn warmup(&mut self) -> anyhow::Result<()> {
-        self.set_state(ConductorState::WarmingUp).await;
-
-        let request =
-            LlmRequest::new("Say hi in 5 words or less.", &self.config.model).with_stream(true);
-
-        match self.backend.send_streaming(&request).await {
-            Ok(mut rx) => {
-                // Drain the warmup response
-                while let Some(token) = rx.recv().await {
-                    match token {
-                        StreamingToken::Complete { .. } => break,
-                        StreamingToken::Error(e) => {
-                            tracing::warn!("Warmup error: {}", e);
-                            break;
-                        }
-                        _ => {}
-                    }
-                }
-                self.warmup_complete = true;
-                self.set_state(ConductorState::Ready).await;
-            }
-            Err(e) => {
-                tracing::warn!("Warmup failed: {}", e);
-                self.warmup_complete = true; // Allow proceeding anyway
-                self.set_state(ConductorState::Ready).await;
-            }
-        }
-
-        Ok(())
-    }
```

#### Change 2: Remove warmup_complete field

Search for field definition and remove:
```diff
-    warmup_complete: bool,
```

#### Change 3: Remove warmup initialization

Find where `warmup_complete: false` is set in constructor and remove.

#### Change 4: Remove warmup call

Find where `warmup()` is called (likely in initialization) and remove call.

#### Change 5: Remove WarmingUp state (OPTIONAL)

If `ConductorState::WarmingUp` is not used elsewhere:
```diff
pub enum ConductorState {
    Initializing,
-    WarmingUp,
    Ready,
    Thinking,
    Responding,
}
```

---

## Testing Checklist

### Build Verification
- [ ] Conductor builds without errors
- [ ] No unused code warnings for removed function
- [ ] All existing tests pass

### Performance Testing
- [ ] Measure startup time BEFORE: `time ./yollayah.sh --test-interactive`
- [ ] Apply changes
- [ ] Measure startup time AFTER: `time ./yollayah.sh --test-interactive`
- [ ] Verify startup is < 1 second (should be near-instant)

### Functional Testing
- [ ] Launch `./yollayah.sh --interactive`
- [ ] Send first message
- [ ] Verify response is immediate (no warmup needed)
- [ ] Verify GPU utilization matches direct Ollama

### Regression Testing
- [ ] All integration tests pass
- [ ] TUI mode works
- [ ] Bash mode works
- [ ] No error messages

---

## Expected Results

**Before**:
- Startup: 3-7 seconds (with warmup LLM call)
- First message: Additional delay

**After**:
- Startup: < 1 second (no warmup)
- First message: Immediate (Ollama keep_alive keeps model loaded)

**Performance Gain**: 3-7 second reduction in startup time

---

## Rationale

### Why Warmup Was Wrong

1. **Ollama has keep_alive**: Models stay loaded for 24h by default
2. **Blocking during init**: Delays user interaction
3. **Unnecessary overhead**: First real message is just as fast
4. **Security concern**: Predictable warmup prompt

### Why Elimination Is Safe

1. **Ollama keeps models loaded**: No "cold start" penalty
2. **First message is fast**: Direct testing confirms this
3. **Environment variable exists**: `YOLLAYAH_OLLAMA_KEEP_ALIVE` already set to 24h
4. **No functionality lost**: Warmup didn't provide value

---

## Implementation Steps

1. **Read full conductor.rs** to find all warmup references
2. **Remove warmup() function**
3. **Remove warmup_complete field**
4. **Remove warmup() call from initialization**
5. **Remove WarmingUp state if unused**
6. **Build and test**
7. **Measure performance improvement**
8. **Create QA verification task**

---

## Success Criteria

- [x] warmup() function removed
- [x] warmup_complete field removed
- [x] WarmingUp state removed (if unused)
- [ ] Builds pass (0 errors)
- [ ] Tests pass
- [ ] Startup time < 1 second
- [ ] First message response immediate
- [ ] Performance matches direct Ollama CLI

---

## Notes

**CRITICAL**: This is the PRIMARY fix for conductor slowness.

Warmup was well-intentioned but unnecessary with modern Ollama.
Trust the backend's keep_alive mechanism.
