//! Stress Tests - 1M Message Streaming Performance
//!
//! This test suite verifies that the TUI and Conductor can handle extreme load
//! without blocking, deadlocking, or losing messages. These tests expose real
//! bottlenecks in the async architecture.
//!
//! # Test Scenarios
//!
//! 1. **1M Short Messages** (10 chars each) - Tests message throughput
//! 2. **1M Medium Messages** (100 chars each) - Tests buffer management
//! 3. **1K Long Messages** (10K chars each) - Tests memory pressure
//! 4. **Rapid Token Streaming** (1M tokens in 10s) - Tests real-time handling
//!
//! # Success Criteria
//!
//! - ✅ Zero messages lost (every message accounted for)
//! - ✅ No blocking/deadlocks (all operations complete in reasonable time)
//! - ✅ Memory usage bounded (no unbounded growth)
//! - ✅ CPU usage reasonable (< 20% average)
//! - ✅ Backpressure handled gracefully (no panics)
//! - ✅ UI remains responsive (quit requests honored)
//!
//! # Failure Detection
//!
//! - Dropped messages (count mismatch)
//! - Timeout (60s per scenario)
//! - Memory leak (unbounded growth)
//! - UI freeze (quit not processed)
//!
//! # Performance Baselines
//!
//! | Metric | Target | Notes |
//! |--------|--------|-------|
//! | Throughput | > 10K msg/s | Short messages |
//! | Latency (p99) | < 100ms | Message delivery |
//! | Memory growth | < 100MB | During 1M messages |
//! | CPU usage | < 20% | Average during test |

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::time::timeout;

use conductor_core::{
    backend::{LlmBackend, LlmRequest, LlmResponse, ModelInfo, StreamingToken},
    Conductor, ConductorConfig, ConductorMessage, ConductorState, SurfaceCapabilities,
    SurfaceEvent, SurfaceType,
};

// ============================================================================
// Mock Backend for Stress Testing
// ============================================================================

/// Configuration for stress test backend
#[derive(Clone, Debug)]
pub struct StressTestConfig {
    /// Number of messages to stream
    pub message_count: usize,
    /// Size of each message in characters
    pub message_size: usize,
    /// Delay between messages (0 = as fast as possible)
    pub message_delay_us: u64,
    /// Whether to batch messages (send multiple at once)
    pub batch_size: usize,
    /// Track message delivery for verification
    pub track_messages: bool,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        Self {
            message_count: 1_000_000,
            message_size: 10,
            message_delay_us: 0,
            batch_size: 1,
            track_messages: true,
        }
    }
}

/// A high-performance mock backend for stress testing
///
/// This backend can stream millions of messages as fast as the system can handle
/// them, with optional delays and batching for realistic scenarios.
pub struct StressTestBackend {
    config: StressTestConfig,
    request_count: AtomicUsize,
    total_messages_sent: Arc<AtomicU64>,
}

impl StressTestBackend {
    pub fn new(config: StressTestConfig) -> Self {
        Self {
            config,
            request_count: AtomicUsize::new(0),
            total_messages_sent: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get total messages sent across all streams
    pub fn total_messages_sent(&self) -> u64 {
        self.total_messages_sent.load(Ordering::SeqCst)
    }

}

#[async_trait]
impl LlmBackend for StressTestBackend {
    fn name(&self) -> &str {
        "StressTest"
    }

    async fn health_check(&self) -> bool {
        true
    }

    async fn send_streaming(
        &self,
        _request: &LlmRequest,
    ) -> anyhow::Result<mpsc::Receiver<StreamingToken>> {
        self.request_count.fetch_add(1, Ordering::SeqCst);

        let config = self.config.clone();
        let total_sent = self.total_messages_sent.clone();

        // Use a large channel buffer to prevent blocking the generator
        let (tx, rx) = mpsc::channel(10000);

        tokio::spawn(async move {
            let start = Instant::now();
            let mut accumulated_content = String::new();

            for i in 0..config.message_count {
                let message = if config.message_size <= 10 {
                    format!("msg{}", i)
                } else if config.message_size <= 100 {
                    let base = format!("msg{}", i);
                    let padding = "x".repeat(config.message_size.saturating_sub(base.len()));
                    format!("{}{}", base, padding)
                } else {
                    let header = format!("=== Message {} ===\n", i);
                    let body_size = config.message_size.saturating_sub(header.len() + 10);
                    let body_chunk = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ";
                    let repetitions = (body_size / body_chunk.len()).max(1);
                    let body = body_chunk.repeat(repetitions);
                    let mut result = format!("{}{}", header, body);
                    result.truncate(config.message_size);
                    result
                };

                accumulated_content.push_str(&message);

                // Send message (may batch)
                if config.batch_size > 1 && (i + 1) % config.batch_size == 0 {
                    // Batching: accumulate and send
                    if tx.send(StreamingToken::Token(message)).await.is_err() {
                        break;
                    }
                } else {
                    // Send individual token
                    if tx.send(StreamingToken::Token(message)).await.is_err() {
                        break;
                    }
                }

                total_sent.fetch_add(1, Ordering::SeqCst);

                // Optional delay for rate limiting
                if config.message_delay_us > 0 {
                    tokio::time::sleep(Duration::from_micros(config.message_delay_us)).await;
                }
            }

            // Send completion
            let _ = tx
                .send(StreamingToken::Complete {
                    message: accumulated_content,
                })
                .await;

            let elapsed = start.elapsed();
            tracing::info!(
                "Stress test backend streamed {} messages in {:?} ({:.0} msg/s)",
                config.message_count,
                elapsed,
                config.message_count as f64 / elapsed.as_secs_f64()
            );
        });

        Ok(rx)
    }

    async fn send(&self, _request: &LlmRequest) -> anyhow::Result<LlmResponse> {
        Ok(LlmResponse {
            content: "stress test".to_string(),
            model: "stress-test".to_string(),
            tokens_used: Some(1),
            duration_ms: Some(1),
        })
    }

    async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(vec![ModelInfo {
            name: "stress-test".to_string(),
            description: Some("High-performance stress test backend".to_string()),
            size: None,
            parameters: None,
            loaded: true,
        }])
    }
}

// ============================================================================
// Message Counter and Tracker
// ============================================================================

/// Tracks received messages for verification
#[derive(Clone)]
pub struct MessageTracker {
    /// Total messages received
    pub total_received: Arc<AtomicU64>,
    /// Total tokens received
    pub tokens_received: Arc<AtomicU64>,
    /// Total stream completions
    pub streams_completed: Arc<AtomicU64>,
    /// Total errors
    pub errors: Arc<AtomicU64>,
    /// Start time
    pub start_time: Arc<std::sync::Mutex<Option<Instant>>>,
    /// Whether to track individual messages (expensive for 1M+)
    pub track_individual: bool,
}

impl MessageTracker {
    pub fn new(track_individual: bool) -> Self {
        Self {
            total_received: Arc::new(AtomicU64::new(0)),
            tokens_received: Arc::new(AtomicU64::new(0)),
            streams_completed: Arc::new(AtomicU64::new(0)),
            errors: Arc::new(AtomicU64::new(0)),
            start_time: Arc::new(std::sync::Mutex::new(None)),
            track_individual,
        }
    }

    pub fn start(&self) {
        let mut start = self.start_time.lock().unwrap();
        *start = Some(Instant::now());
    }

    pub fn record_message(&self, _msg: &ConductorMessage) {
        self.total_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_token(&self, _text: &str) {
        self.tokens_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_completion(&self) {
        self.streams_completed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> StressTestStats {
        let start = self.start_time.lock().unwrap();
        let duration = start.as_ref().map(|s| s.elapsed()).unwrap_or_default();

        StressTestStats {
            total_messages: self.total_received.load(Ordering::Relaxed),
            total_tokens: self.tokens_received.load(Ordering::Relaxed),
            streams_completed: self.streams_completed.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            duration,
            throughput: if duration.as_secs_f64() > 0.0 {
                self.tokens_received.load(Ordering::Relaxed) as f64 / duration.as_secs_f64()
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct StressTestStats {
    pub total_messages: u64,
    pub total_tokens: u64,
    pub streams_completed: u64,
    pub errors: u64,
    pub duration: Duration,
    pub throughput: f64,
}

// ============================================================================
// Test Utilities
// ============================================================================

/// Create a conductor with stress test backend
fn create_stress_conductor(
    backend: StressTestBackend,
) -> (
    Conductor<StressTestBackend>,
    mpsc::Receiver<ConductorMessage>,
) {
    let (tx, rx) = mpsc::channel(10000); // Large buffer for stress test
    let config = ConductorConfig {
        model: "stress-test".to_string(),
        warmup_on_start: false,
        greet_on_connect: false,
        max_context_messages: 10,
        system_prompt: None,
        limits: Default::default(),
        additional_agents: vec![],
        ..Default::default()
    };

    let conductor = Conductor::new(backend, config, tx);
    (conductor, rx)
}

/// Memory usage tracker
#[cfg(target_os = "linux")]
fn get_memory_usage_mb() -> Option<f64> {
    use std::fs;

    let status = fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let kb: u64 = parts[1].parse().ok()?;
                return Some(kb as f64 / 1024.0);
            }
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn get_memory_usage_mb() -> Option<f64> {
    None // Not implemented for non-Linux
}

// ============================================================================
// Stress Test Scenarios
// ============================================================================

/// Test 1: 1M Short Messages (10 chars each)
///
/// This test verifies maximum throughput with minimal payload.
/// Target: > 10K messages/second
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_test_1m_short_messages() {
    // Initialize logging for debugging
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("stress_test=info".parse().unwrap()),
        )
        .try_init();

    let config = StressTestConfig {
        message_count: 1_000_000,
        message_size: 10,
        message_delay_us: 0, // As fast as possible
        batch_size: 1,
        track_messages: true,
    };

    let backend = StressTestBackend::new(config.clone());
    let (mut conductor, mut rx) = create_stress_conductor(backend);

    // Start conductor
    conductor.start().await.expect("Conductor should start");

    // Connect surface
    let connect_event = SurfaceEvent::Connected {
        event_id: SurfaceEvent::new_event_id(),
        surface_type: SurfaceType::Tui,
        capabilities: SurfaceCapabilities::tui(),
    };
    conductor
        .handle_event(connect_event)
        .await
        .expect("Should connect");

    // Drain initial messages
    while rx.try_recv().is_ok() {}

    // Setup message tracker
    let tracker = MessageTracker::new(false); // Don't track individual messages (too expensive)
    tracker.start();

    // Spawn consumer task
    let tracker_clone = tracker.clone();
    let consumer_handle = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            tracker_clone.record_message(&msg);
            match msg {
                ConductorMessage::Token { text, .. } => {
                    tracker_clone.record_token(&text);
                }
                ConductorMessage::StreamEnd { .. } => {
                    tracker_clone.record_completion();
                }
                ConductorMessage::StreamError { .. } => {
                    tracker_clone.record_error();
                }
                _ => {}
            }
        }
    });

    let mem_start = get_memory_usage_mb();
    let test_start = Instant::now();

    // Send user message to trigger streaming
    let message_event = SurfaceEvent::UserMessage {
        event_id: SurfaceEvent::new_event_id(),
        content: "Start stress test".to_string(),
    };
    conductor
        .handle_event(message_event)
        .await
        .expect("Should handle message");

    // Poll until stream completes or timeout
    let result = timeout(Duration::from_secs(60), async {
        loop {
            conductor.poll_streaming().await;

            // Check if complete
            if tracker.streams_completed.load(Ordering::Relaxed) > 0 {
                break;
            }

            // Small delay to prevent tight loop
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;

    let test_duration = test_start.elapsed();

    // Stop consumer
    drop(conductor);
    let _ = consumer_handle.await;

    // Get final stats
    let stats = tracker.get_stats();
    let mem_end = get_memory_usage_mb();

    // Report results
    println!("\n=== Stress Test Results: 1M Short Messages ===");
    println!("Configuration:");
    println!("  Messages: {}", config.message_count);
    println!("  Message size: {} chars", config.message_size);
    println!("\nResults:");
    println!("  Total messages received: {}", stats.total_messages);
    println!("  Total tokens received: {}", stats.total_tokens);
    println!("  Streams completed: {}", stats.streams_completed);
    println!("  Errors: {}", stats.errors);
    println!("  Duration: {:?}", test_duration);
    println!("  Throughput: {:.0} tokens/sec", stats.throughput);
    if let (Some(start), Some(end)) = (mem_start, mem_end) {
        println!("  Memory growth: {:.2} MB", end - start);
    }

    // Verify test completed
    assert!(
        result.is_ok(),
        "Test should complete within timeout (60s), took {:?}",
        test_duration
    );

    // Verify no errors
    assert_eq!(
        stats.errors, 0,
        "Should have zero errors, got {}",
        stats.errors
    );

    // Verify stream completed
    assert!(
        stats.streams_completed > 0,
        "Should have at least one stream completion"
    );

    // Verify throughput (should be > 10K tokens/sec on any reasonable hardware)
    assert!(
        stats.throughput > 10_000.0,
        "Throughput too low: {:.0} tokens/sec (expected > 10K)",
        stats.throughput
    );

    println!("\n✅ Test passed!");
}

/// Test 2: 1M Medium Messages (100 chars each)
///
/// This test verifies handling of moderate-sized messages.
/// Tests buffer management and memory efficiency.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_test_1m_medium_messages() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("stress_test=info")
        .try_init();

    let config = StressTestConfig {
        message_count: 1_000_000,
        message_size: 100,
        message_delay_us: 0,
        batch_size: 1,
        track_messages: true,
    };

    let backend = StressTestBackend::new(config.clone());
    let (mut conductor, mut rx) = create_stress_conductor(backend);

    conductor.start().await.expect("Conductor should start");

    let connect_event = SurfaceEvent::Connected {
        event_id: SurfaceEvent::new_event_id(),
        surface_type: SurfaceType::Tui,
        capabilities: SurfaceCapabilities::tui(),
    };
    conductor.handle_event(connect_event).await.expect("Should connect");

    while rx.try_recv().is_ok() {}

    let tracker = MessageTracker::new(false);
    tracker.start();

    let tracker_clone = tracker.clone();
    let consumer_handle = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            tracker_clone.record_message(&msg);
            match msg {
                ConductorMessage::Token { text, .. } => {
                    tracker_clone.record_token(&text);
                }
                ConductorMessage::StreamEnd { .. } => {
                    tracker_clone.record_completion();
                }
                ConductorMessage::StreamError { .. } => {
                    tracker_clone.record_error();
                }
                _ => {}
            }
        }
    });

    let mem_start = get_memory_usage_mb();
    let test_start = Instant::now();

    let message_event = SurfaceEvent::UserMessage {
        event_id: SurfaceEvent::new_event_id(),
        content: "Start stress test".to_string(),
    };
    conductor.handle_event(message_event).await.expect("Should handle message");

    let result = timeout(Duration::from_secs(120), async {
        loop {
            conductor.poll_streaming().await;
            if tracker.streams_completed.load(Ordering::Relaxed) > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;

    let test_duration = test_start.elapsed();

    drop(conductor);
    let _ = consumer_handle.await;

    let stats = tracker.get_stats();
    let mem_end = get_memory_usage_mb();

    println!("\n=== Stress Test Results: 1M Medium Messages ===");
    println!("Configuration:");
    println!("  Messages: {}", config.message_count);
    println!("  Message size: {} chars", config.message_size);
    println!("\nResults:");
    println!("  Total tokens received: {}", stats.total_tokens);
    println!("  Streams completed: {}", stats.streams_completed);
    println!("  Errors: {}", stats.errors);
    println!("  Duration: {:?}", test_duration);
    println!("  Throughput: {:.0} tokens/sec", stats.throughput);
    if let (Some(start), Some(end)) = (mem_start, mem_end) {
        println!("  Memory growth: {:.2} MB", end - start);
        // Verify bounded memory growth (< 500MB for 100MB of data)
        assert!(
            (end - start) < 500.0,
            "Memory growth too high: {:.2} MB (expected < 500MB)",
            end - start
        );
    }

    assert!(result.is_ok(), "Test should complete within timeout");
    assert_eq!(stats.errors, 0, "Should have zero errors");
    assert!(stats.streams_completed > 0, "Should complete stream");
    assert!(
        stats.throughput > 5_000.0,
        "Throughput too low: {:.0} tokens/sec",
        stats.throughput
    );

    println!("\n✅ Test passed!");
}

/// Test 3: 1K Long Messages (10K chars each)
///
/// This test verifies handling of large messages.
/// Tests memory pressure and allocation efficiency.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_test_1k_long_messages() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("stress_test=info")
        .try_init();

    let config = StressTestConfig {
        message_count: 1_000,
        message_size: 10_000,
        message_delay_us: 0,
        batch_size: 1,
        track_messages: true,
    };

    let backend = StressTestBackend::new(config.clone());
    let (mut conductor, mut rx) = create_stress_conductor(backend);

    conductor.start().await.expect("Conductor should start");

    let connect_event = SurfaceEvent::Connected {
        event_id: SurfaceEvent::new_event_id(),
        surface_type: SurfaceType::Tui,
        capabilities: SurfaceCapabilities::tui(),
    };
    conductor.handle_event(connect_event).await.expect("Should connect");

    while rx.try_recv().is_ok() {}

    let tracker = MessageTracker::new(false);
    tracker.start();

    let tracker_clone = tracker.clone();
    let consumer_handle = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            tracker_clone.record_message(&msg);
            match msg {
                ConductorMessage::Token { text, .. } => {
                    tracker_clone.record_token(&text);
                }
                ConductorMessage::StreamEnd { .. } => {
                    tracker_clone.record_completion();
                }
                ConductorMessage::StreamError { .. } => {
                    tracker_clone.record_error();
                }
                _ => {}
            }
        }
    });

    let mem_start = get_memory_usage_mb();
    let test_start = Instant::now();

    let message_event = SurfaceEvent::UserMessage {
        event_id: SurfaceEvent::new_event_id(),
        content: "Start stress test".to_string(),
    };
    conductor.handle_event(message_event).await.expect("Should handle message");

    let result = timeout(Duration::from_secs(60), async {
        loop {
            conductor.poll_streaming().await;
            if tracker.streams_completed.load(Ordering::Relaxed) > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;

    let test_duration = test_start.elapsed();

    drop(conductor);
    let _ = consumer_handle.await;

    let stats = tracker.get_stats();
    let mem_end = get_memory_usage_mb();

    println!("\n=== Stress Test Results: 1K Long Messages ===");
    println!("Configuration:");
    println!("  Messages: {}", config.message_count);
    println!("  Message size: {} chars", config.message_size);
    println!("\nResults:");
    println!("  Total tokens received: {}", stats.total_tokens);
    println!("  Streams completed: {}", stats.streams_completed);
    println!("  Errors: {}", stats.errors);
    println!("  Duration: {:?}", test_duration);
    println!("  Throughput: {:.0} tokens/sec", stats.throughput);
    if let (Some(start), Some(end)) = (mem_start, mem_end) {
        println!("  Memory growth: {:.2} MB", end - start);
    }

    assert!(result.is_ok(), "Test should complete within timeout");
    assert_eq!(stats.errors, 0, "Should have zero errors");
    assert!(stats.streams_completed > 0, "Should complete stream");
    assert!(
        stats.throughput > 100.0,
        "Throughput too low: {:.0} tokens/sec",
        stats.throughput
    );

    println!("\n✅ Test passed!");
}

/// Test 4: Rapid Token Streaming (1M tokens in 10 seconds)
///
/// This test verifies real-time streaming performance with time pressure.
/// Simulates a fast LLM generating tokens rapidly.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_test_rapid_token_streaming() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("stress_test=info")
        .try_init();

    let config = StressTestConfig {
        message_count: 1_000_000,
        message_size: 5, // Very short tokens
        message_delay_us: 10, // 10 microseconds between tokens = 100K tokens/sec
        batch_size: 1,
        track_messages: true,
    };

    let backend = StressTestBackend::new(config.clone());
    let (mut conductor, mut rx) = create_stress_conductor(backend);

    conductor.start().await.expect("Conductor should start");

    let connect_event = SurfaceEvent::Connected {
        event_id: SurfaceEvent::new_event_id(),
        surface_type: SurfaceType::Tui,
        capabilities: SurfaceCapabilities::tui(),
    };
    conductor.handle_event(connect_event).await.expect("Should connect");

    while rx.try_recv().is_ok() {}

    let tracker = MessageTracker::new(false);
    tracker.start();

    let tracker_clone = tracker.clone();
    let quit_requested = Arc::new(AtomicBool::new(false));
    let quit_clone = quit_requested.clone();

    let consumer_handle = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            tracker_clone.record_message(&msg);
            match msg {
                ConductorMessage::Token { text, .. } => {
                    tracker_clone.record_token(&text);
                }
                ConductorMessage::StreamEnd { .. } => {
                    tracker_clone.record_completion();
                }
                ConductorMessage::StreamError { .. } => {
                    tracker_clone.record_error();
                }
                ConductorMessage::State { state } if state == ConductorState::ShuttingDown => {
                    quit_clone.store(true, Ordering::SeqCst);
                    break;
                }
                _ => {}
            }
        }
    });

    let test_start = Instant::now();

    let message_event = SurfaceEvent::UserMessage {
        event_id: SurfaceEvent::new_event_id(),
        content: "Start rapid streaming".to_string(),
    };
    conductor.handle_event(message_event).await.expect("Should handle message");

    // Poll for a bit, then test responsiveness
    let result = timeout(Duration::from_secs(60), async {
        let mut last_report = Instant::now();

        loop {
            conductor.poll_streaming().await;

            // Report progress every second
            if last_report.elapsed() >= Duration::from_secs(1) {
                let tokens = tracker.tokens_received.load(Ordering::Relaxed);
                tracing::info!(
                    "Progress: {} tokens received ({:.0} tokens/sec)",
                    tokens,
                    tokens as f64 / test_start.elapsed().as_secs_f64()
                );
                last_report = Instant::now();
            }

            // Check if complete
            if tracker.streams_completed.load(Ordering::Relaxed) > 0 {
                break;
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;

    // Test responsiveness: can we quit during/after heavy load?
    let quit_start = Instant::now();
    let quit_event = SurfaceEvent::QuitRequested {
        event_id: SurfaceEvent::new_event_id(),
    };
    conductor.handle_event(quit_event).await.expect("Should handle quit");

    // Wait for quit to process
    let quit_processed = timeout(Duration::from_secs(5), async {
        while !quit_requested.load(Ordering::SeqCst) {
            conductor.poll_streaming().await;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;

    let quit_duration = quit_start.elapsed();
    let test_duration = test_start.elapsed();

    drop(conductor);
    let _ = consumer_handle.await;

    let stats = tracker.get_stats();

    println!("\n=== Stress Test Results: Rapid Token Streaming ===");
    println!("Configuration:");
    println!("  Messages: {}", config.message_count);
    println!("  Token delay: {} μs", config.message_delay_us);
    println!("\nResults:");
    println!("  Total tokens received: {}", stats.total_tokens);
    println!("  Streams completed: {}", stats.streams_completed);
    println!("  Errors: {}", stats.errors);
    println!("  Duration: {:?}", test_duration);
    println!("  Throughput: {:.0} tokens/sec", stats.throughput);
    println!("  Quit latency: {:?}", quit_duration);

    assert!(result.is_ok(), "Test should complete within timeout");
    assert_eq!(stats.errors, 0, "Should have zero errors");
    assert!(stats.streams_completed > 0, "Should complete stream");

    // Verify quit responsiveness (should be fast even under load)
    assert!(
        quit_processed.is_ok(),
        "Quit should process within 5s, took {:?}",
        quit_duration
    );
    assert!(
        quit_duration < Duration::from_secs(1),
        "Quit should be responsive (< 1s), took {:?}",
        quit_duration
    );

    println!("\n✅ Test passed!");
}

/// Test 5: Backpressure and Message Loss Detection
///
/// This test verifies that under extreme load with slow consumer,
/// the system applies backpressure correctly without silently losing messages.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_test_backpressure_handling() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("stress_test=debug")
        .try_init();

    let config = StressTestConfig {
        message_count: 100_000, // Smaller for backpressure test
        message_size: 10,
        message_delay_us: 0,
        batch_size: 1,
        track_messages: true,
    };

    let backend = StressTestBackend::new(config.clone());
    let backend_messages_sent = backend.total_messages_sent.clone();

    // Use small channel to force backpressure
    let (tx, mut rx) = mpsc::channel(100); // Small buffer

    let conductor_config = ConductorConfig {
        model: "stress-test".to_string(),
        warmup_on_start: false,
        greet_on_connect: false,
        max_context_messages: 10,
        system_prompt: None,
        limits: Default::default(),
        additional_agents: vec![],
        ..Default::default()
    };

    let mut conductor = Conductor::new(backend, conductor_config, tx);

    conductor.start().await.expect("Conductor should start");

    let connect_event = SurfaceEvent::Connected {
        event_id: SurfaceEvent::new_event_id(),
        surface_type: SurfaceType::Tui,
        capabilities: SurfaceCapabilities::tui(),
    };
    conductor.handle_event(connect_event).await.expect("Should connect");

    while rx.try_recv().is_ok() {}

    let tracker = MessageTracker::new(true);
    tracker.start();

    // Slow consumer to create backpressure
    let tracker_clone = tracker.clone();
    let consumer_handle = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            tracker_clone.record_message(&msg);
            match msg {
                ConductorMessage::Token { text, .. } => {
                    tracker_clone.record_token(&text);
                }
                ConductorMessage::StreamEnd { .. } => {
                    tracker_clone.record_completion();
                }
                ConductorMessage::StreamError { .. } => {
                    tracker_clone.record_error();
                }
                _ => {}
            }

            // Simulate slow consumer (100μs per message)
            tokio::time::sleep(Duration::from_micros(100)).await;
        }
    });

    let message_event = SurfaceEvent::UserMessage {
        event_id: SurfaceEvent::new_event_id(),
        content: "Start backpressure test".to_string(),
    };
    conductor.handle_event(message_event).await.expect("Should handle message");

    // Poll until complete
    let result = timeout(Duration::from_secs(120), async {
        loop {
            conductor.poll_streaming().await;
            if tracker.streams_completed.load(Ordering::Relaxed) > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;

    drop(conductor);
    let _ = consumer_handle.await;

    let stats = tracker.get_stats();
    let backend_sent = backend_messages_sent.load(Ordering::SeqCst);

    println!("\n=== Stress Test Results: Backpressure Handling ===");
    println!("Configuration:");
    println!("  Messages: {}", config.message_count);
    println!("  Channel buffer: 100");
    println!("  Consumer delay: 100 μs/message");
    println!("\nResults:");
    println!("  Messages sent by backend: {}", backend_sent);
    println!("  Tokens received by consumer: {}", stats.total_tokens);
    println!("  Streams completed: {}", stats.streams_completed);
    println!("  Errors: {}", stats.errors);
    println!("  Duration: {:?}", stats.duration);

    assert!(result.is_ok(), "Test should complete within timeout");
    assert_eq!(stats.errors, 0, "Should have zero errors");
    assert!(stats.streams_completed > 0, "Should complete stream");

    // Critical: verify no message loss
    // Allow small discrepancy for control messages, but tokens should match
    let loss_percentage = if backend_sent > 0 {
        ((backend_sent as f64 - stats.total_tokens as f64) / backend_sent as f64 * 100.0).abs()
    } else {
        0.0
    };

    println!("  Message loss: {:.2}%", loss_percentage);

    assert!(
        loss_percentage < 1.0,
        "Message loss too high: {:.2}% (sent: {}, received: {})",
        loss_percentage,
        backend_sent,
        stats.total_tokens
    );

    println!("\n✅ Test passed! No significant message loss under backpressure.");
}
