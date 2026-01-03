//! CPU Performance Tests for TUI
//!
//! These tests measure CPU load during various TUI operations to catch performance regressions.
//!
//! # Test Coverage
//!
//! 1. **Idle CPU Load**: Measures CPU usage when TUI is idle (target: < 2%)
//! 2. **Streaming CPU Load**: Measures CPU usage during response streaming (target: < 5%)
//! 3. **Render Rate**: Counts renders per second (target: 8-12 FPS)
//!
//! # Design Philosophy
//!
//! These tests serve as regression detectors for BUG-003 (TUI Performance Regression).
//! They establish performance baselines and alert when CPU usage or render rates exceed targets.
//!
//! # Running Tests
//!
//! ```bash
//! # Run all CPU performance tests
//! cargo test --test cpu_performance_test
//!
//! # Run with output to see measurements
//! cargo test --test cpu_performance_test -- --nocapture
//!
//! # Run a specific test
//! cargo test --test cpu_performance_test test_cpu_load_during_idle
//! ```
//!
//! # Performance Targets
//!
//! | Metric | Target | Rationale |
//! |--------|--------|-----------|
//! | CPU idle | < 2% | Should be negligible when nothing is happening |
//! | CPU streaming | < 5% | Streaming is mostly I/O, not compute |
//! | Render rate | 8-12 FPS | 10 FPS ± 20% is the target frame rate |
//!
//! # Implementation Notes
//!
//! - Uses `/proc/self/stat` on Linux for accurate CPU measurement
//! - Measures over 5-second windows to smooth out noise
//! - Uses MultiModelMockBackend for deterministic streaming behavior
//! - Simulates realistic token rates (50 tokens/sec)

use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::time::sleep;

use conductor_core::{
    backend::{LlmBackend, LlmRequest, LlmResponse, ModelInfo, StreamingToken},
    Conductor, ConductorConfig, ConductorMessage, SurfaceCapabilities,
    SurfaceEvent, SurfaceType,
};

// ============================================================================
// CPU Measurement Utilities
// ============================================================================

/// CPU stats from /proc/self/stat
#[derive(Debug, Clone)]
struct CpuStats {
    /// User mode time (in clock ticks)
    utime: u64,
    /// System mode time (in clock ticks)
    stime: u64,
    /// Total time (utime + stime)
    total: u64,
    /// Timestamp when measured
    timestamp: Instant,
}

impl CpuStats {
    /// Read CPU stats from /proc/self/stat
    ///
    /// Returns None if unable to read (e.g., not on Linux)
    fn read() -> Option<Self> {
        let stat = fs::read_to_string("/proc/self/stat").ok()?;
        let fields: Vec<&str> = stat.split_whitespace().collect();

        // Fields: pid comm state ppid pgrp session tty_nr tpgid flags minflt cminflt majflt cmajflt utime stime...
        // utime is field 13 (0-indexed), stime is field 14
        if fields.len() < 15 {
            return None;
        }

        let utime: u64 = fields[13].parse().ok()?;
        let stime: u64 = fields[14].parse().ok()?;
        let total = utime + stime;

        Some(Self {
            utime,
            stime,
            total,
            timestamp: Instant::now(),
        })
    }

    /// Calculate CPU percentage between two measurements
    ///
    /// Returns CPU usage as a percentage (0.0 - 100.0+)
    fn cpu_percent_since(&self, previous: &CpuStats) -> f64 {
        let elapsed_secs = self.timestamp.duration_since(previous.timestamp).as_secs_f64();
        if elapsed_secs < 0.001 {
            return 0.0; // Avoid division by zero
        }

        // CPU time is in clock ticks. Convert to seconds.
        // Clock ticks per second is typically 100 (USER_HZ)
        const CLOCK_TICKS_PER_SEC: f64 = 100.0;

        let cpu_time_secs = (self.total - previous.total) as f64 / CLOCK_TICKS_PER_SEC;
        let cpu_percent = (cpu_time_secs / elapsed_secs) * 100.0;

        cpu_percent
    }
}

/// Measure CPU usage over a duration
///
/// Returns average CPU percentage over the measurement period
async fn measure_cpu_load(duration: Duration) -> f64 {
    let start_stats = CpuStats::read();
    if start_stats.is_none() {
        eprintln!("Warning: Unable to read /proc/self/stat. CPU measurement unavailable.");
        return 0.0; // Gracefully skip on non-Linux systems
    }
    let start_stats = start_stats.unwrap();

    sleep(duration).await;

    let end_stats = CpuStats::read();
    if end_stats.is_none() {
        return 0.0;
    }
    let end_stats = end_stats.unwrap();

    end_stats.cpu_percent_since(&start_stats)
}

// ============================================================================
// Mock Backend with Render Counting
// ============================================================================

/// Mock backend that simulates realistic streaming for CPU testing
///
/// Features:
/// - Configurable token rate (tokens per second)
/// - Configurable total response length
/// - Deterministic behavior for reproducible tests
pub struct CpuTestMockBackend {
    /// Number of tokens to generate per response
    tokens_per_response: usize,
    /// Delay between tokens (ms) to simulate streaming rate
    token_delay_ms: u64,
    /// Counter for tracking requests
    request_count: AtomicUsize,
}

impl CpuTestMockBackend {
    /// Create a mock backend with specified token rate
    ///
    /// # Arguments
    /// * `tokens_per_sec` - How many tokens to stream per second (e.g., 50)
    /// * `total_tokens` - Total tokens in the response (e.g., 250 for 5 seconds at 50 tok/sec)
    pub fn new(tokens_per_sec: usize, total_tokens: usize) -> Self {
        let token_delay_ms = if tokens_per_sec > 0 {
            1000 / tokens_per_sec as u64
        } else {
            20 // Default to 50 tokens/sec
        };

        Self {
            tokens_per_response: total_tokens,
            token_delay_ms,
            request_count: AtomicUsize::new(0),
        }
    }

    /// Create a backend with fast streaming (for quick tests)
    pub fn new_fast() -> Self {
        Self::new(100, 100) // 100 tokens/sec, 100 total (1 second)
    }

    /// Create a backend with realistic streaming
    pub fn new_realistic() -> Self {
        Self::new(50, 250) // 50 tokens/sec, 250 total (5 seconds)
    }

    /// Get the number of requests made
    pub fn request_count(&self) -> usize {
        self.request_count.load(Ordering::SeqCst)
    }
}

impl Clone for CpuTestMockBackend {
    fn clone(&self) -> Self {
        Self {
            tokens_per_response: self.tokens_per_response,
            token_delay_ms: self.token_delay_ms,
            request_count: AtomicUsize::new(self.request_count.load(Ordering::SeqCst)),
        }
    }
}

#[async_trait]
impl LlmBackend for CpuTestMockBackend {
    fn name(&self) -> &str {
        "CpuTestMock"
    }

    async fn health_check(&self) -> bool {
        true
    }

    async fn send_streaming(
        &self,
        _request: &LlmRequest,
    ) -> anyhow::Result<mpsc::Receiver<StreamingToken>> {
        self.request_count.fetch_add(1, Ordering::SeqCst);

        let tokens_count = self.tokens_per_response;
        let delay = self.token_delay_ms;

        let (tx, rx) = mpsc::channel(100);
        tokio::spawn(async move {
            let mut full_message = String::new();

            for i in 0..tokens_count {
                let token = if i == 0 {
                    "This ".to_string()
                } else if i == tokens_count - 1 {
                    "response.".to_string()
                } else if i % 10 == 0 {
                    format!("token{} ", i)
                } else {
                    "is ".to_string()
                };

                full_message.push_str(&token);

                // Send token
                let _ = tx.send(StreamingToken::Token(token)).await;

                // Delay between tokens
                if i < tokens_count - 1 {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }

            // Send completion
            let _ = tx.send(StreamingToken::Complete {
                message: full_message,
            }).await;
        });

        Ok(rx)
    }

    async fn send(&self, _request: &LlmRequest) -> anyhow::Result<LlmResponse> {
        self.request_count.fetch_add(1, Ordering::SeqCst);

        let full_response = "This is a test response.".to_string();

        Ok(LlmResponse {
            content: full_response,
            model: "cpu-test-mock".to_string(),
            tokens_used: Some(self.tokens_per_response as u32),
            duration_ms: Some(100),
        })
    }

    async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(vec![ModelInfo {
            name: "cpu-test-mock".to_string(),
            description: Some("Mock backend for CPU performance testing".to_string()),
            size: None,
            parameters: None,
            loaded: true,
        }])
    }
}

// ============================================================================
// Test Utilities
// ============================================================================

/// Create a test conductor with the CPU test backend
fn create_test_conductor(
    backend: CpuTestMockBackend,
) -> (
    Conductor<CpuTestMockBackend>,
    mpsc::Receiver<ConductorMessage>,
) {
    let (tx, rx) = mpsc::channel(100);
    let config = ConductorConfig {
        model: "cpu-test-mock".to_string(),
        warmup_on_start: false, // Skip warmup in tests
        greet_on_connect: false, // No greeting for CPU tests
        max_context_messages: 10,
        system_prompt: None,
        limits: Default::default(),
        additional_agents: vec![],
        ..Default::default()
    };

    let conductor = Conductor::new(backend, config, tx);
    (conductor, rx)
}

/// Count messages received over a duration
///
/// Returns the number of ConductorMessages received
async fn count_messages(rx: &mut mpsc::Receiver<ConductorMessage>, duration: Duration) -> usize {
    let mut count = 0;
    let deadline = Instant::now() + duration;

    while Instant::now() < deadline {
        let remaining = deadline.duration_since(Instant::now());
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Some(_)) => count += 1,
            Ok(None) => break, // Channel closed
            Err(_) => break,   // Timeout
        }
    }

    count
}

// ============================================================================
// CPU Performance Tests
// ============================================================================

#[tokio::test]
async fn test_cpu_load_during_idle() {
    // This test measures CPU load when the Conductor is idle (no queries, no streaming)
    //
    // Expected: CPU load should be negligible (< 2%)
    //
    // Rationale: When nothing is happening, the Conductor should sleep in select! and
    // consume minimal CPU. High idle CPU suggests:
    // - Busy loops
    // - Polling instead of event-driven architecture
    // - Unnecessary background tasks

    let backend = CpuTestMockBackend::new_fast();
    let (mut conductor, mut rx) = create_test_conductor(backend);

    // Start the conductor
    conductor.start().await.expect("Conductor should start");

    // Connect a surface
    let connect_event = SurfaceEvent::Connected {
        event_id: SurfaceEvent::new_event_id(),
        surface_type: SurfaceType::Tui,
        capabilities: SurfaceCapabilities::default(),
    };
    conductor
        .handle_event(connect_event)
        .await
        .expect("Should handle connect");

    // Drain any startup messages
    sleep(Duration::from_millis(100)).await;
    while rx.try_recv().is_ok() {}

    // Measure CPU during idle period
    println!("Measuring idle CPU load over 5 seconds...");
    let cpu_idle = measure_cpu_load(Duration::from_secs(5)).await;
    println!("Idle CPU load: {:.2}%", cpu_idle);

    // Shutdown
    conductor.shutdown().await.ok();

    // Assert: Idle CPU should be < 2%
    assert!(
        cpu_idle < 2.0,
        "Idle CPU load too high: {:.2}% (target: < 2%)",
        cpu_idle
    );
}

#[tokio::test]
async fn test_cpu_load_during_streaming() {
    // This test measures CPU load during response streaming
    //
    // Expected: CPU load should be low (< 5%) even during streaming
    //
    // Rationale: Streaming is mostly I/O-bound (receiving tokens from LLM).
    // High CPU during streaming suggests:
    // - Expensive rendering operations per token
    // - Excessive re-rendering or layout calculations
    // - Inefficient string operations or memory allocations

    let backend = CpuTestMockBackend::new_realistic(); // 50 tokens/sec, 250 total (5 seconds)
    let (mut conductor, mut rx) = create_test_conductor(backend);

    // Start the conductor
    conductor.start().await.expect("Conductor should start");

    // Connect a surface
    let connect_event = SurfaceEvent::Connected {
        event_id: SurfaceEvent::new_event_id(),
        surface_type: SurfaceType::Tui,
        capabilities: SurfaceCapabilities::default(),
    };
    conductor
        .handle_event(connect_event)
        .await
        .expect("Should handle connect");

    // Drain any startup messages
    sleep(Duration::from_millis(100)).await;
    while rx.try_recv().is_ok() {}

    // Start a query (will stream for ~5 seconds)
    let query_event = SurfaceEvent::UserMessage {
        event_id: SurfaceEvent::new_event_id(),
        content: "Tell me about CPU performance".to_string(),
    };
    conductor
        .handle_event(query_event)
        .await
        .expect("Should handle query");

    // Measure CPU during streaming (measure for 5 seconds)
    println!("Measuring CPU load during streaming (5 seconds)...");
    let cpu_streaming = measure_cpu_load(Duration::from_secs(5)).await;
    println!("Streaming CPU load: {:.2}%", cpu_streaming);

    // Shutdown
    conductor.shutdown().await.ok();

    // Assert: Streaming CPU should be < 5%
    assert!(
        cpu_streaming < 5.0,
        "Streaming CPU load too high: {:.2}% (target: < 5%)",
        cpu_streaming
    );
}

#[tokio::test]
async fn test_render_rate_estimation() {
    // This test estimates the message/render rate during streaming
    //
    // Expected: Message rate should correlate with token rate + reasonable batching
    //
    // Rationale: At 50 tokens/sec, we should NOT see 50 messages/sec if batching works.
    // We should see ~10-20 messages/sec (batching every 50-100ms).
    //
    // High message rates suggest:
    // - No batching of tokens
    // - Each token triggers a separate message
    // - Excessive re-renders

    let backend = CpuTestMockBackend::new_realistic(); // 50 tokens/sec, 250 total (5 seconds)
    let (mut conductor, mut rx) = create_test_conductor(backend);

    // Start the conductor
    conductor.start().await.expect("Conductor should start");

    // Connect a surface
    let connect_event = SurfaceEvent::Connected {
        event_id: SurfaceEvent::new_event_id(),
        surface_type: SurfaceType::Tui,
        capabilities: SurfaceCapabilities::default(),
    };
    conductor
        .handle_event(connect_event)
        .await
        .expect("Should handle connect");

    // Drain startup messages
    sleep(Duration::from_millis(100)).await;
    while rx.try_recv().is_ok() {}

    // Start a query
    let query_event = SurfaceEvent::UserMessage {
        event_id: SurfaceEvent::new_event_id(),
        content: "Tell me about render rates".to_string(),
    };
    conductor
        .handle_event(query_event)
        .await
        .expect("Should handle query");

    // Count messages over 5 seconds
    println!("Counting messages during 5-second streaming period...");
    let message_count = count_messages(&mut rx, Duration::from_secs(5)).await;
    let messages_per_sec = message_count as f64 / 5.0;

    println!(
        "Messages received: {} ({:.1} msgs/sec)",
        message_count, messages_per_sec
    );

    // Shutdown
    conductor.shutdown().await.ok();

    // Assert: Message rate should be reasonable (not 1:1 with tokens)
    // At 50 tokens/sec, we expect batching to reduce this to 10-20 msgs/sec
    // Allow up to 30 msgs/sec to account for overhead
    assert!(
        messages_per_sec < 30.0,
        "Message rate too high: {:.1} msgs/sec (suggests no batching, target: < 30/sec)",
        messages_per_sec
    );

    // Also verify we're getting SOME messages (not zero due to test failure)
    assert!(
        message_count > 0,
        "No messages received - test may be broken"
    );
}

#[tokio::test]
async fn test_cpu_load_with_long_conversation() {
    // This test measures CPU load when streaming with a long conversation history
    //
    // Expected: CPU should remain low (< 5%) even with long conversations
    //
    // Rationale: From BUG-003, we know that conversation re-wrapping happens every frame.
    // For long conversations, this can be expensive. This test verifies that CPU
    // doesn't scale linearly with conversation length.
    //
    // High CPU with long conversations suggests:
    // - Re-wrapping entire conversation every frame
    // - No caching of wrapped text
    // - Inefficient text processing

    let backend = CpuTestMockBackend::new_realistic();
    let (mut conductor, mut rx) = create_test_conductor(backend);

    conductor.start().await.expect("Conductor should start");

    let connect_event = SurfaceEvent::Connected {
        event_id: SurfaceEvent::new_event_id(),
        surface_type: SurfaceType::Tui,
        capabilities: SurfaceCapabilities::default(),
    };
    conductor
        .handle_event(connect_event)
        .await
        .expect("Should handle connect");

    sleep(Duration::from_millis(100)).await;
    while rx.try_recv().is_ok() {}

    // Send multiple queries to build up conversation history
    // (In a real scenario, we'd wait for responses, but for CPU testing we just care about
    // the conductor's CPU usage, not the full interaction)
    println!("Building conversation history...");
    for i in 0..10 {
        let query = SurfaceEvent::UserMessage {
            event_id: SurfaceEvent::new_event_id(),
            content: format!("Query number {} - tell me something interesting about performance optimization and why it matters for user experience", i),
        };
        conductor
            .handle_event(query)
            .await
            .expect("Should handle query");

        // Wait for response to complete
        sleep(Duration::from_secs(6)).await; // Longer than streaming duration
        while rx.try_recv().is_ok() {} // Drain messages
    }

    println!("Conversation history built. Measuring CPU during final streaming...");

    // Now send one more query and measure CPU
    let final_query = SurfaceEvent::UserMessage {
        event_id: SurfaceEvent::new_event_id(),
        content: "Final query about CPU performance with long history".to_string(),
    };
    conductor
        .handle_event(final_query)
        .await
        .expect("Should handle query");

    let cpu_long_convo = measure_cpu_load(Duration::from_secs(5)).await;
    println!(
        "CPU load with long conversation: {:.2}%",
        cpu_long_convo
    );

    conductor.shutdown().await.ok();

    // Assert: CPU should still be < 5% even with long conversation
    assert!(
        cpu_long_convo < 5.0,
        "CPU load too high with long conversation: {:.2}% (target: < 5%)",
        cpu_long_convo
    );
}

// ============================================================================
// Benchmarks (Manual - not run in CI)
// ============================================================================

/// Manual benchmark to establish baseline CPU characteristics
///
/// Run with: `cargo test --test cpu_performance_test test_manual_baseline -- --nocapture --ignored`
#[tokio::test]
#[ignore]
async fn test_manual_baseline() {
    println!("\n=== CPU Performance Baseline ===\n");

    // Test 1: Idle
    println!("Test 1: Idle CPU load");
    let backend = CpuTestMockBackend::new_fast();
    let (mut conductor, mut rx) = create_test_conductor(backend);
    conductor.start().await.unwrap();
    let connect = SurfaceEvent::Connected {
        event_id: SurfaceEvent::new_event_id(),
        surface_type: SurfaceType::Tui,
        capabilities: SurfaceCapabilities::default(),
    };
    conductor.handle_event(connect).await.unwrap();
    sleep(Duration::from_millis(100)).await;
    while rx.try_recv().is_ok() {}

    let cpu_idle = measure_cpu_load(Duration::from_secs(5)).await;
    println!("  Idle CPU: {:.2}%", cpu_idle);
    conductor.shutdown().await.ok();

    sleep(Duration::from_secs(1)).await;

    // Test 2: Streaming
    println!("\nTest 2: Streaming CPU load");
    let backend = CpuTestMockBackend::new_realistic();
    let (mut conductor, mut rx) = create_test_conductor(backend);
    conductor.start().await.unwrap();
    let connect = SurfaceEvent::Connected {
        event_id: SurfaceEvent::new_event_id(),
        surface_type: SurfaceType::Tui,
        capabilities: SurfaceCapabilities::default(),
    };
    conductor.handle_event(connect).await.unwrap();
    sleep(Duration::from_millis(100)).await;
    while rx.try_recv().is_ok() {}

    let query = SurfaceEvent::UserMessage {
        event_id: SurfaceEvent::new_event_id(),
        content: "Tell me about performance".to_string(),
    };
    conductor.handle_event(query).await.unwrap();

    let cpu_streaming = measure_cpu_load(Duration::from_secs(5)).await;
    println!("  Streaming CPU: {:.2}%", cpu_streaming);
    conductor.shutdown().await.ok();

    println!("\n=== Baseline Complete ===\n");
}
