# Integration Testing - Gatekeeper Documentation

This document tracks the integration test suite that serves as the **gatekeeper** for the ai-way codebase. These tests verify that the TUI, Conductor, and LLM backend work together correctly.

## Purpose

The integration tests ensure:
1. TUI can start and connect to Conductor
2. Conductor can initialize and manage LLM backends
3. Yollayah greeting flow works correctly
4. User messages flow through the full stack
5. Response streaming works end-to-end
6. Avatar commands are parsed and dispatched
7. Graceful shutdown works correctly

## Current Tests

Located in `tui/tests/integration_test.rs`:

| Test | Description | Status |
|------|-------------|--------|
| `test_conductor_startup_and_greeting` | Verifies conductor starts and sends greeting | Passing |
| `test_user_message_and_response` | Tests message send/receive flow | Passing |
| `test_multi_turn_conversation` | Tests conversation history works | Passing |
| `test_avatar_messages_processed` | Tests avatar command parsing | Passing |
| `test_graceful_shutdown` | Tests clean shutdown | Passing |
| `test_exit_before_start` | Exit before conductor starts (Ctrl+C/Esc) | Passing |
| `test_exit_during_startup` | Exit during initialization phase | Passing |
| `test_exit_during_streaming` | Exit while response is streaming | Passing |
| `test_multiple_exit_requests` | Multiple rapid quit requests | Passing |
| `test_exit_with_pending_input` | Exit while user is typing | Passing |
| `test_input_after_slow_response` | Verify input works after slow LLM | Passing |

## Architecture

```
+----------------+     +-------------+     +----------------------+
|  Test Harness  | --> |  Conductor  | --> | IntegrationMockBackend |
+----------------+     +-------------+     +----------------------+
        |                    |                       |
        v                    v                       v
   mpsc channel         LlmBackend trait      Simulated responses
   (messages)           (async_trait)          (token streaming)
```

The tests use `IntegrationMockBackend` which:
- Implements the `LlmBackend` trait
- Returns predictable token streams based on prompt content
- Simulates realistic streaming delays (configurable)
- Tracks request counts for verification

## Running Tests

```bash
# Run integration tests only
cargo test --test integration_test

# Run with output
cargo test --test integration_test -- --nocapture

# Run specific test
cargo test --test integration_test test_conductor_startup_and_greeting
```

## Pre-commit Integration

Integration tests are run automatically on every commit via `.git/hooks/pre-commit`:
- Triggered when any `.rs` file is staged
- Blocks commit if tests fail
- Provides clear error messages pointing to test file

## Pending Enhancements

### Priority 1: Critical Path Coverage

- [ ] **Warmup flow test**: Test that `warmup_on_start = true` correctly pre-loads model
- [ ] **Error recovery test**: Test conductor behavior when backend returns errors
- [ ] **Timeout handling test**: Verify streaming timeout behavior
- [ ] **Channel buffer full test**: Test behavior when message channel fills up

### Priority 2: Edge Cases

- [ ] **Empty response test**: Test handling of empty LLM responses
- [ ] **Very long response test**: Test streaming of large responses (100+ tokens)
- [ ] **Special characters test**: Test UTF-8, emoji, control characters in responses
- [ ] **Rapid message sending**: Test multiple messages sent before responses complete
- [ ] **Concurrent operations**: Test greeting + user message overlap

### Priority 3: Real Backend Tests (Optional)

These require a running Ollama instance and should be skipped in CI:

- [ ] **Real Ollama connection**: Test actual HTTP connection to Ollama
- [ ] **Real streaming**: Verify actual streaming response parsing
- [ ] **Real model loading**: Test warmup with actual model loading

Mark these with `#[ignore]` and run explicitly:
```bash
cargo test --test integration_test -- --ignored
```

### Priority 4: Performance Tests

- [ ] **Latency measurement**: Track message round-trip time
- [ ] **Memory usage**: Verify no memory leaks during long conversations
- [ ] **Throughput**: Test sustained message volume

## Known Weaknesses

1. **Mock-only testing**: Current tests don't verify actual HTTP/JSON parsing
2. **No TUI rendering tests**: Tests verify conductor, not terminal output
3. **No async race conditions**: Tests run sequentially, missing potential races
4. **Fixed delays**: Tests use fixed sleep durations, may be flaky on slow systems

## Async Issues Found (from audit)

The following async issues were identified during development and should be considered when adding new tests:

1. **Blocking in async context**: Use `tokio::sync::Mutex` not `std::sync::Mutex`
2. **EventStream handling**: Terminal events must use crossterm's `EventStream`
3. **Startup blocking**: Long operations must be broken into timeout chunks
4. **Channel sizing**: Consider backpressure when channels fill up
5. **Channel drain order**: Drain channel BEFORE polling streaming to prevent blocking

## Fixes Applied

### Input Blocking After Response (Fixed)

**Symptom**: After a slow LLM response, keyboard input stops working but avatar continues frolicking.

**Root Cause**: In the TUI event loop, `poll_streaming()` was called before `process_conductor_messages()`. If the conductor->TUI channel filled up during slow streaming, `poll_streaming()` would block on `send()` before the channel could be drained.

**Fix Applied** (in `tui/src/app.rs`):
```rust
// IMPORTANT: Process messages FIRST to drain the channel
// This prevents poll_streaming from blocking if channel is full
self.process_conductor_messages();

// Poll conductor for streaming tokens
self.conductor.poll_streaming().await;

// Process any newly arrived messages from streaming
self.process_conductor_messages();
```

**Test Coverage**: `test_input_after_slow_response` verifies input works after slow LLM responses.

## Contributing

When adding new tests:
1. Follow existing naming convention: `test_<feature>_<scenario>`
2. Use `IntegrationMockBackend` for deterministic behavior
3. Add timeouts to prevent hanging tests
4. Document test purpose in function docstring
5. Add entry to table in this document

## Maintenance

This test suite should be:
- **Hardened over time**: Add tests for any bugs found in production
- **Performance-conscious**: Keep total test time under 5 seconds
- **Self-documenting**: Clear test names and failure messages
- **Resilient**: No flaky tests that fail intermittently
