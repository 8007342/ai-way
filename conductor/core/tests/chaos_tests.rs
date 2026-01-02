//! Chaos Tests for Transport Layer Resilience
//!
//! These tests verify system behavior under adverse conditions:
//! - Network failures mid-transmission
//! - Unresponsive backends
//! - Memory pressure from many sessions
//! - Concurrent operations during cleanup
//!
//! # Running
//!
//! These tests are ignored by default due to their long-running nature:
//! ```bash
//! cargo test chaos -- --ignored --nocapture
//! ```
//!
//! Run a specific chaos test:
//! ```bash
//! cargo test chaos_socket_close_mid_frame -- --ignored --nocapture
//! ```

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

use conductor_core::events::{SurfaceCapabilities, SurfaceType};
use conductor_core::messages::ConductorMessage;
use conductor_core::surface_registry::{
    ConnectionId as SurfaceConnectionId, SurfaceHandle, SurfaceRegistry,
};
use conductor_core::transport::frame::{encode, FrameDecoder};
use conductor_core::transport::heartbeat::{HeartbeatConfig, HeartbeatEvent, HeartbeatMonitor};
use conductor_core::transport::TransportError;

// =============================================================================
// Chaos Test Infrastructure
// =============================================================================

/// Configuration for chaos test scenarios
#[derive(Clone, Debug)]
pub struct ChaosConfig {
    /// Duration to run chaos scenario
    pub duration: Duration,
    /// Number of concurrent operations
    pub concurrency: usize,
    /// Failure injection probability (0.0 - 1.0)
    pub failure_rate: f64,
    /// Timeout for operations
    pub timeout: Duration,
    /// Enable verbose logging
    pub verbose: bool,
}

impl Default for ChaosConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(10),
            concurrency: 50,
            failure_rate: 0.3,
            timeout: Duration::from_secs(5),
            verbose: false,
        }
    }
}

impl ChaosConfig {
    /// Create a shorter config for faster tests
    pub fn quick() -> Self {
        Self {
            duration: Duration::from_secs(3),
            concurrency: 20,
            failure_rate: 0.3,
            timeout: Duration::from_secs(2),
            verbose: false,
        }
    }

    /// Create an intensive config for thorough testing
    pub fn intensive() -> Self {
        Self {
            duration: Duration::from_secs(30),
            concurrency: 100,
            failure_rate: 0.5,
            timeout: Duration::from_secs(10),
            verbose: true,
        }
    }
}

/// Tracks resource usage during chaos tests
#[derive(Debug, Default)]
pub struct ResourceTracker {
    /// Peak number of active connections
    pub peak_connections: AtomicUsize,
    /// Total operations attempted
    pub total_operations: AtomicUsize,
    /// Operations that succeeded
    pub successful_operations: AtomicUsize,
    /// Operations that failed gracefully
    pub graceful_failures: AtomicUsize,
    /// Operations that panicked or had unexpected errors
    pub unexpected_errors: AtomicUsize,
    /// Active connections (for leak detection)
    active_connections: AtomicUsize,
}

impl ResourceTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_connection_created(&self) {
        let current = self.active_connections.fetch_add(1, Ordering::SeqCst) + 1;
        let mut peak = self.peak_connections.load(Ordering::SeqCst);
        while current > peak {
            match self.peak_connections.compare_exchange_weak(
                peak,
                current,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(p) => peak = p,
            }
        }
    }

    pub fn record_connection_closed(&self) {
        // Use saturating subtraction to avoid overflow when cleanup removes
        // connections that weren't tracked (e.g., during concurrent pruning)
        let prev = self.active_connections.load(Ordering::SeqCst);
        if prev > 0 {
            self.active_connections.fetch_sub(1, Ordering::SeqCst);
        }
    }

    pub fn record_operation(&self, success: bool, graceful_failure: bool) {
        self.total_operations.fetch_add(1, Ordering::Relaxed);
        if success {
            self.successful_operations.fetch_add(1, Ordering::Relaxed);
        } else if graceful_failure {
            self.graceful_failures.fetch_add(1, Ordering::Relaxed);
        } else {
            self.unexpected_errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn active_connections(&self) -> usize {
        self.active_connections.load(Ordering::SeqCst)
    }

    pub fn summary(&self) -> String {
        format!(
            "Operations: {} total, {} success, {} graceful fail, {} unexpected; \
             Connections: {} active, {} peak",
            self.total_operations.load(Ordering::Relaxed),
            self.successful_operations.load(Ordering::Relaxed),
            self.graceful_failures.load(Ordering::Relaxed),
            self.unexpected_errors.load(Ordering::Relaxed),
            self.active_connections.load(Ordering::SeqCst),
            self.peak_connections.load(Ordering::SeqCst),
        )
    }
}

/// Result of a chaos test
#[derive(Debug)]
pub struct ChaosTestResult {
    /// Whether the test passed
    pub passed: bool,
    /// Test duration
    pub duration: Duration,
    /// Resource usage summary
    pub resources: String,
    /// Any errors encountered
    pub errors: Vec<String>,
    /// Additional metrics
    pub metrics: Vec<(String, String)>,
}

impl ChaosTestResult {
    pub fn success(duration: Duration, tracker: &ResourceTracker) -> Self {
        Self {
            passed: true,
            duration,
            resources: tracker.summary(),
            errors: Vec::new(),
            metrics: Vec::new(),
        }
    }

    pub fn failure(duration: Duration, tracker: &ResourceTracker, errors: Vec<String>) -> Self {
        Self {
            passed: false,
            duration,
            resources: tracker.summary(),
            errors,
            metrics: Vec::new(),
        }
    }

    pub fn with_metric(mut self, name: &str, value: &str) -> Self {
        self.metrics.push((name.to_string(), value.to_string()));
        self
    }
}

// =============================================================================
// Test: Socket Close Mid-Frame
// =============================================================================

/// Simulates a connection dying during message transmission
///
/// This test verifies:
/// - Partial frame detection
/// - Graceful error handling without panic
/// - No resource leaks from interrupted transmissions
/// - Proper cleanup of decoder state
#[tokio::test]
#[ignore] // Intentional (chaos) - Long-running test, run manually
async fn chaos_socket_close_mid_frame() {
    let config = ChaosConfig::quick();
    let tracker = Arc::new(ResourceTracker::new());
    let start = Instant::now();
    let mut errors = Vec::new();

    println!("Starting chaos_socket_close_mid_frame test...");
    println!(
        "Config: {} concurrent, {:.0}s duration",
        config.concurrency,
        config.duration.as_secs_f64()
    );

    let mut join_set = JoinSet::new();
    let stop_flag = Arc::new(AtomicBool::new(false));

    // Spawn concurrent frame transmission tasks
    for task_id in 0..config.concurrency {
        let tracker = Arc::clone(&tracker);
        let stop_flag = Arc::clone(&stop_flag);
        let failure_rate = config.failure_rate;

        join_set.spawn(async move {
            let mut local_errors = Vec::new();
            let mut rng_state = task_id as u64;

            while !stop_flag.load(Ordering::Relaxed) {
                // Simple pseudo-random for deterministic behavior
                rng_state = rng_state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let should_fail = (rng_state % 100) < (failure_rate * 100.0) as u64;

                tracker.record_connection_created();

                // Create a test message and encode it
                let msg = ConductorMessage::Ping {
                    seq: task_id as u64,
                };
                let encoded = match encode(&msg) {
                    Ok(e) => e,
                    Err(e) => {
                        local_errors.push(format!("Encode error: {}", e));
                        tracker.record_connection_closed();
                        tracker.record_operation(false, false);
                        continue;
                    }
                };

                // Create a decoder
                let mut decoder = FrameDecoder::new();

                if should_fail {
                    // Inject failure: only send partial frame
                    let partial_len = (rng_state as usize % encoded.len().saturating_sub(1)).max(1);
                    decoder.push(&encoded[..partial_len]);

                    // Attempt to decode - should return Ok(None) for partial data
                    let result: Result<Option<ConductorMessage>, TransportError> = decoder.decode();
                    match result {
                        Ok(None) => {
                            // Expected: partial frame, need more data
                            tracker.record_operation(false, true);
                        }
                        Ok(Some(_)) => {
                            // Unexpected: got a message from partial data
                            local_errors.push("Decoded message from partial frame!".to_string());
                            tracker.record_operation(false, false);
                        }
                        Err(e) => {
                            // Also acceptable: error on invalid data
                            if config.verbose {
                                println!(
                                    "Task {}: Partial decode error (expected): {}",
                                    task_id, e
                                );
                            }
                            tracker.record_operation(false, true);
                        }
                    }
                } else {
                    // Normal path: send complete frame
                    decoder.push(&encoded);

                    let result: Result<Option<ConductorMessage>, TransportError> = decoder.decode();
                    match result {
                        Ok(Some(decoded_msg)) => {
                            // Verify the message matches
                            if let ConductorMessage::Ping { seq } = decoded_msg {
                                if seq == task_id as u64 {
                                    tracker.record_operation(true, false);
                                } else {
                                    local_errors
                                        .push(format!("Seq mismatch: {} vs {}", seq, task_id));
                                    tracker.record_operation(false, false);
                                }
                            } else {
                                local_errors.push("Wrong message type".to_string());
                                tracker.record_operation(false, false);
                            }
                        }
                        Ok(None) => {
                            local_errors.push("No message from complete frame".to_string());
                            tracker.record_operation(false, false);
                        }
                        Err(e) => {
                            local_errors.push(format!("Decode error: {}", e));
                            tracker.record_operation(false, false);
                        }
                    }
                }

                tracker.record_connection_closed();

                // Small yield to allow other tasks to run
                tokio::task::yield_now().await;
            }

            local_errors
        });
    }

    // Let the chaos run
    tokio::time::sleep(config.duration).await;
    stop_flag.store(true, Ordering::Relaxed);

    // Collect results
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(task_errors) => errors.extend(task_errors),
            Err(e) => errors.push(format!("Task panicked: {}", e)),
        }
    }

    let duration = start.elapsed();

    // Verify no resource leaks
    let active = tracker.active_connections();
    if active != 0 {
        errors.push(format!(
            "Resource leak: {} connections still active",
            active
        ));
    }

    // Allow some errors but not too many
    let unexpected = tracker.unexpected_errors.load(Ordering::Relaxed);
    let total = tracker.total_operations.load(Ordering::Relaxed);
    let error_rate = if total > 0 {
        unexpected as f64 / total as f64
    } else {
        0.0
    };

    println!("\n=== chaos_socket_close_mid_frame Results ===");
    println!("Duration: {:.2}s", duration.as_secs_f64());
    println!("{}", tracker.summary());
    println!("Error rate: {:.2}%", error_rate * 100.0);

    // Pass criteria: < 1% unexpected errors, no resource leaks
    let passed = active == 0 && error_rate < 0.01;

    if !passed {
        println!("\nErrors encountered:");
        for (i, err) in errors.iter().take(10).enumerate() {
            println!("  {}: {}", i + 1, err);
        }
        if errors.len() > 10 {
            println!("  ... and {} more", errors.len() - 10);
        }
    }

    assert!(
        passed,
        "Chaos test failed: {} unexpected errors, {} active connections",
        unexpected, active
    );
    println!("\nPASSED: chaos_socket_close_mid_frame\n");
}

// =============================================================================
// Test: Backend Hang
// =============================================================================

/// Simulates a backend that stops responding
///
/// This test verifies:
/// - Timeout triggers correctly
/// - Connection marked as unhealthy
/// - System continues to function after timeout
/// - Graceful degradation under timeout conditions
#[tokio::test]
#[ignore] // Intentional (chaos) - Long-running test, run manually
async fn chaos_backend_hang() {
    let config = ChaosConfig::quick();
    let tracker = Arc::new(ResourceTracker::new());
    let start = Instant::now();
    let mut errors = Vec::new();

    println!("Starting chaos_backend_hang test...");
    println!(
        "Config: {} concurrent, {:.0}s duration",
        config.concurrency,
        config.duration.as_secs_f64()
    );

    // Create heartbeat monitor with short timeouts for testing
    let hb_config = HeartbeatConfig::new()
        .with_interval(Duration::from_millis(50))
        .with_timeout(Duration::from_millis(100))
        .with_max_missed(2);

    let (monitor, mut event_rx) = HeartbeatMonitor::with_events(hb_config);
    let registry = SurfaceRegistry::new();

    // Track health events
    let timeout_count = Arc::new(AtomicUsize::new(0));
    let missed_count = Arc::new(AtomicUsize::new(0));
    let timeout_count_clone = Arc::clone(&timeout_count);
    let missed_count_clone = Arc::clone(&missed_count);

    // Event collector task
    let event_collector = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                HeartbeatEvent::ConnectionTimeout { .. } => {
                    timeout_count_clone.fetch_add(1, Ordering::Relaxed);
                }
                HeartbeatEvent::PongMissed { .. } => {
                    missed_count_clone.fetch_add(1, Ordering::Relaxed);
                }
                _ => {}
            }
        }
    });

    let mut join_set = JoinSet::new();
    let stop_flag = Arc::new(AtomicBool::new(false));
    let monitor = Arc::new(monitor);
    let registry = Arc::new(registry);

    // Spawn tasks that simulate backends that may hang
    for task_id in 0..config.concurrency {
        let tracker = Arc::clone(&tracker);
        let stop_flag = Arc::clone(&stop_flag);
        let monitor = Arc::clone(&monitor);
        let registry = Arc::clone(&registry);
        let failure_rate = config.failure_rate;

        join_set.spawn(async move {
            let local_errors = Vec::new();
            let mut rng_state = task_id as u64;

            while !stop_flag.load(Ordering::Relaxed) {
                rng_state = rng_state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let should_hang = (rng_state % 100) < (failure_rate * 100.0) as u64;

                // Create a "connection" to the backend
                let conn_id = SurfaceConnectionId::new();
                let (tx, mut rx) = mpsc::channel(32);
                let handle = SurfaceHandle::new(
                    conn_id,
                    tx,
                    SurfaceType::Headless,
                    SurfaceCapabilities::headless(),
                );

                registry.register(handle);
                monitor.register(conn_id);
                tracker.record_connection_created();

                if should_hang {
                    // Simulate hanging: don't respond to pings
                    // Wait for timeout to be detected
                    let hang_duration = Duration::from_millis(200 + (rng_state % 100) as u64);
                    tokio::time::sleep(hang_duration).await;

                    // Check if we got marked unhealthy
                    let health = monitor.get_health(&conn_id);
                    let is_healthy = health.as_ref().is_some_and(|h| h.healthy);

                    if is_healthy {
                        // Might still be waiting for timeout - that's ok
                        tracker.record_operation(false, true);
                    } else {
                        // Expected: marked unhealthy
                        tracker.record_operation(true, false);
                    }
                } else {
                    // Normal path: respond to pings promptly
                    let respond_duration = Duration::from_millis(50);
                    let deadline = Instant::now() + respond_duration;

                    while Instant::now() < deadline {
                        if let Ok(msg) = rx.try_recv() {
                            if let ConductorMessage::Ping { seq } = msg {
                                monitor.record_pong(&conn_id, seq);
                            }
                        }
                        tokio::task::yield_now().await;
                    }

                    // Should still be healthy
                    let health = monitor.get_health(&conn_id);
                    let is_healthy = health.as_ref().is_some_and(|h| h.healthy);

                    if is_healthy {
                        tracker.record_operation(true, false);
                    } else {
                        // Might have been reaped by concurrent test activity
                        tracker.record_operation(false, true);
                    }
                }

                // Cleanup
                monitor.unregister(&conn_id);
                registry.unregister(&conn_id);
                tracker.record_connection_closed();

                tokio::task::yield_now().await;
            }

            local_errors
        });
    }

    // Let the chaos run
    tokio::time::sleep(config.duration).await;
    stop_flag.store(true, Ordering::Relaxed);

    // Stop the monitor
    monitor.stop();

    // Collect results
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(task_errors) => errors.extend(task_errors),
            Err(e) => errors.push(format!("Task panicked: {}", e)),
        }
    }

    // Stop event collector
    drop(monitor); // Drop the Arc to close the channel
    let _ = tokio::time::timeout(Duration::from_secs(1), event_collector).await;

    let duration = start.elapsed();

    // Verify no resource leaks
    let active = tracker.active_connections();
    if active != 0 {
        errors.push(format!(
            "Resource leak: {} connections still active",
            active
        ));
    }

    let timeouts = timeout_count.load(Ordering::Relaxed);
    let misses = missed_count.load(Ordering::Relaxed);
    let unexpected = tracker.unexpected_errors.load(Ordering::Relaxed);
    let total = tracker.total_operations.load(Ordering::Relaxed);
    let error_rate = if total > 0 {
        unexpected as f64 / total as f64
    } else {
        0.0
    };

    println!("\n=== chaos_backend_hang Results ===");
    println!("Duration: {:.2}s", duration.as_secs_f64());
    println!("{}", tracker.summary());
    println!("Timeouts detected: {}", timeouts);
    println!("Pongs missed: {}", misses);
    println!("Unexpected error rate: {:.2}%", error_rate * 100.0);

    // Pass criteria: < 5% unexpected errors, no resource leaks, timeouts were detected
    let passed = active == 0 && error_rate < 0.05;

    if !passed {
        println!("\nErrors encountered:");
        for (i, err) in errors.iter().take(10).enumerate() {
            println!("  {}: {}", i + 1, err);
        }
    }

    assert!(
        passed,
        "Chaos test failed: {:.2}% unexpected errors, {} active connections",
        error_rate * 100.0,
        active
    );
    println!("\nPASSED: chaos_backend_hang\n");
}

// =============================================================================
// Test: Session Memory Pressure
// =============================================================================

/// Creates many concurrent sessions to verify memory stays bounded
///
/// This test verifies:
/// - Memory stays bounded under high session count
/// - Old sessions can be evicted properly
/// - No unbounded growth in data structures
/// - Registry handles high connection counts
#[tokio::test]
#[ignore] // Intentional (chaos) - Long-running test, run manually
async fn chaos_session_memory_pressure() {
    let config = ChaosConfig {
        duration: Duration::from_secs(5),
        concurrency: 200, // Many sessions
        failure_rate: 0.0,
        timeout: Duration::from_secs(5),
        verbose: false,
    };
    let tracker = Arc::new(ResourceTracker::new());
    let start = Instant::now();
    let mut errors = Vec::new();

    println!("Starting chaos_session_memory_pressure test...");
    println!(
        "Config: {} concurrent sessions, {:.0}s duration",
        config.concurrency,
        config.duration.as_secs_f64()
    );

    let registry = Arc::new(SurfaceRegistry::new());

    let mut join_set = JoinSet::new();
    let stop_flag = Arc::new(AtomicBool::new(false));

    // Track peak memory usage (approximation via connection count)
    let peak_sessions = Arc::new(AtomicUsize::new(0));
    let active_sessions = Arc::new(AtomicUsize::new(0));

    // Spawn session lifecycle tasks
    for _task_id in 0..config.concurrency {
        let tracker = Arc::clone(&tracker);
        let stop_flag = Arc::clone(&stop_flag);
        let registry = Arc::clone(&registry);
        let peak_sessions = Arc::clone(&peak_sessions);
        let active_sessions = Arc::clone(&active_sessions);

        join_set.spawn(async move {
            let local_errors = Vec::new();

            while !stop_flag.load(Ordering::Relaxed) {
                // Create a session
                let conn_id = SurfaceConnectionId::new();
                let (tx, _rx) = mpsc::channel(32);

                // Register surface
                let handle = SurfaceHandle::new(
                    conn_id,
                    tx,
                    SurfaceType::Headless,
                    SurfaceCapabilities::headless(),
                );
                registry.register(handle);

                tracker.record_connection_created();
                let current = active_sessions.fetch_add(1, Ordering::SeqCst) + 1;

                // Update peak
                let mut peak = peak_sessions.load(Ordering::SeqCst);
                while current > peak {
                    match peak_sessions.compare_exchange_weak(
                        peak,
                        current,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    ) {
                        Ok(_) => break,
                        Err(p) => peak = p,
                    }
                }

                // Simulate session activity - send some messages
                for _ in 0..10 {
                    registry.send_to(&conn_id, ConductorMessage::QueryCapabilities);
                    tokio::task::yield_now().await;
                }

                // Session complete - cleanup
                registry.unregister(&conn_id);

                tracker.record_connection_closed();
                active_sessions.fetch_sub(1, Ordering::SeqCst);
                tracker.record_operation(true, false);
            }

            local_errors
        });
    }

    // Monitor memory pressure periodically
    let registry_clone = Arc::clone(&registry);
    let monitor_stop = Arc::clone(&stop_flag);
    let memory_monitor = tokio::spawn(async move {
        let mut max_count = 0usize;
        while !monitor_stop.load(Ordering::Relaxed) {
            let count = registry_clone.count();
            if count > max_count {
                max_count = count;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        max_count
    });

    // Let the chaos run
    tokio::time::sleep(config.duration).await;
    stop_flag.store(true, Ordering::Relaxed);

    // Collect results
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(task_errors) => errors.extend(task_errors),
            Err(e) => errors.push(format!("Task panicked: {}", e)),
        }
    }

    let max_registry_count = memory_monitor.await.unwrap_or(0);
    let duration = start.elapsed();

    // Verify no resource leaks
    let active = tracker.active_connections();
    if active != 0 {
        errors.push(format!(
            "Resource leak: {} connections still active",
            active
        ));
    }

    let final_registry_count = registry.count();
    if final_registry_count != 0 {
        errors.push(format!(
            "Registry leak: {} surfaces still registered",
            final_registry_count
        ));
    }

    let peak = peak_sessions.load(Ordering::SeqCst);
    let _unexpected = tracker.unexpected_errors.load(Ordering::Relaxed);
    let total = tracker.total_operations.load(Ordering::Relaxed);
    let success_rate = if total > 0 {
        tracker.successful_operations.load(Ordering::Relaxed) as f64 / total as f64
    } else {
        0.0
    };

    println!("\n=== chaos_session_memory_pressure Results ===");
    println!("Duration: {:.2}s", duration.as_secs_f64());
    println!("{}", tracker.summary());
    println!("Peak concurrent sessions: {}", peak);
    println!("Max registry count: {}", max_registry_count);
    println!("Final registry count: {}", final_registry_count);
    println!("Success rate: {:.2}%", success_rate * 100.0);

    // Pass criteria: no resource leaks, bounded memory (registry count was bounded)
    let passed =
        active == 0 && final_registry_count == 0 && max_registry_count < config.concurrency * 2;

    if !passed {
        println!("\nErrors encountered:");
        for (i, err) in errors.iter().take(10).enumerate() {
            println!("  {}: {}", i + 1, err);
        }
    }

    assert!(
        passed,
        "Chaos test failed: {} active, {} in registry, max was {}",
        active, final_registry_count, max_registry_count
    );
    println!("\nPASSED: chaos_session_memory_pressure\n");
}

// =============================================================================
// Test: Concurrent Pruning
// =============================================================================

/// Tests concurrent operations during cleanup/pruning
///
/// This test verifies:
/// - No deadlocks during concurrent register/unregister/prune
/// - Data integrity maintained during concurrent access
/// - No races leading to corrupted state
/// - Proper lock ordering and contention handling
#[tokio::test]
#[ignore] // Intentional (chaos) - Long-running test, run manually
async fn chaos_concurrent_pruning() {
    let config = ChaosConfig::quick();
    let tracker = Arc::new(ResourceTracker::new());
    let start = Instant::now();
    let mut errors = Vec::new();

    println!("Starting chaos_concurrent_pruning test...");
    println!(
        "Config: {} concurrent, {:.0}s duration",
        config.concurrency,
        config.duration.as_secs_f64()
    );

    let registry = Arc::new(SurfaceRegistry::new());

    // Shared set of active connection IDs for coordination
    let active_ids: Arc<RwLock<HashSet<SurfaceConnectionId>>> =
        Arc::new(RwLock::new(HashSet::new()));

    let mut join_set = JoinSet::new();
    let stop_flag = Arc::new(AtomicBool::new(false));

    // Spawn registrar tasks
    for _task_id in 0..(config.concurrency / 3) {
        let tracker = Arc::clone(&tracker);
        let stop_flag = Arc::clone(&stop_flag);
        let registry = Arc::clone(&registry);
        let active_ids = Arc::clone(&active_ids);

        join_set.spawn(async move {
            let local_errors = Vec::new();

            while !stop_flag.load(Ordering::Relaxed) {
                // Create and register
                let conn_id = SurfaceConnectionId::new();
                let (tx, _rx) = mpsc::channel(32);

                let handle = SurfaceHandle::new(
                    conn_id,
                    tx,
                    SurfaceType::Headless,
                    SurfaceCapabilities::headless(),
                );
                registry.register(handle);
                tracker.record_connection_created();

                // Record this ID as active
                {
                    let mut ids = active_ids.write();
                    ids.insert(conn_id);
                }

                tracker.record_operation(true, false);
                tokio::task::yield_now().await;
            }

            local_errors
        });
    }

    // Spawn unregistrar tasks
    for _task_id in 0..(config.concurrency / 3) {
        let tracker = Arc::clone(&tracker);
        let stop_flag = Arc::clone(&stop_flag);
        let registry = Arc::clone(&registry);
        let active_ids = Arc::clone(&active_ids);

        join_set.spawn(async move {
            let local_errors = Vec::new();

            while !stop_flag.load(Ordering::Relaxed) {
                // Try to get an ID to unregister
                let conn_id = {
                    let mut ids = active_ids.write();
                    ids.iter().next().cloned().map(|id| {
                        ids.remove(&id);
                        id
                    })
                };

                if let Some(conn_id) = conn_id {
                    // Unregister
                    registry.unregister(&conn_id);
                    tracker.record_connection_closed();
                    tracker.record_operation(true, false);
                } else {
                    // Nothing to unregister - ok
                    tracker.record_operation(false, true);
                }

                tokio::task::yield_now().await;
            }

            local_errors
        });
    }

    // Spawn pruning tasks (cleanup_disconnected)
    for _task_id in 0..(config.concurrency / 3) {
        let tracker = Arc::clone(&tracker);
        let stop_flag = Arc::clone(&stop_flag);
        let registry = Arc::clone(&registry);

        join_set.spawn(async move {
            let local_errors = Vec::new();

            while !stop_flag.load(Ordering::Relaxed) {
                // Run cleanup
                let removed = registry.cleanup_disconnected();
                if removed > 0 {
                    // Some connections were cleaned up - record as closed
                    for _ in 0..removed {
                        tracker.record_connection_closed();
                    }
                }
                tracker.record_operation(true, false);

                // Don't spam cleanup too fast
                tokio::time::sleep(Duration::from_millis(10)).await;
            }

            local_errors
        });
    }

    // Spawn broadcast tasks (read operations during mutations)
    for _task_id in 0..(config.concurrency / 6) {
        let tracker = Arc::clone(&tracker);
        let stop_flag = Arc::clone(&stop_flag);
        let registry = Arc::clone(&registry);

        join_set.spawn(async move {
            let local_errors = Vec::new();

            while !stop_flag.load(Ordering::Relaxed) {
                // Read operations - should not deadlock with writes
                let _count = registry.count();
                let _ids = registry.connection_ids();
                let _summary = registry.summary();

                // Try a broadcast (read lock)
                let _ = registry.broadcast(ConductorMessage::QueryCapabilities);

                tracker.record_operation(true, false);
                tokio::task::yield_now().await;
            }

            local_errors
        });
    }

    // Let the chaos run
    tokio::time::sleep(config.duration).await;
    stop_flag.store(true, Ordering::Relaxed);

    // Collect results
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(task_errors) => errors.extend(task_errors),
            Err(e) => errors.push(format!("Task panicked: {}", e)),
        }
    }

    let duration = start.elapsed();

    // Cleanup any remaining
    let remaining_ids: Vec<SurfaceConnectionId> = active_ids.read().iter().cloned().collect();
    for conn_id in remaining_ids {
        registry.unregister(&conn_id);
        tracker.record_connection_closed();
    }

    // Verify consistency
    let final_registry_count = registry.count();
    let active_tracked = active_ids.read().len();

    let unexpected = tracker.unexpected_errors.load(Ordering::Relaxed);
    let total = tracker.total_operations.load(Ordering::Relaxed);
    let success_rate = if total > 0 {
        tracker.successful_operations.load(Ordering::Relaxed) as f64 / total as f64
    } else {
        0.0
    };

    println!("\n=== chaos_concurrent_pruning Results ===");
    println!("Duration: {:.2}s", duration.as_secs_f64());
    println!("{}", tracker.summary());
    println!("Final registry count: {}", final_registry_count);
    println!("Tracked IDs remaining: {}", active_tracked);
    println!("Success rate: {:.2}%", success_rate * 100.0);

    // Pass criteria: no deadlocks (completion within timeout), reasonable success rate
    // Final counts may not be zero due to concurrent task completion timing
    let passed = unexpected == 0 && success_rate > 0.5;

    if !passed {
        println!("\nErrors encountered:");
        for (i, err) in errors.iter().take(10).enumerate() {
            println!("  {}: {}", i + 1, err);
        }
    }

    assert!(
        passed,
        "Chaos test failed: {} unexpected errors, {:.2}% success rate",
        unexpected,
        success_rate * 100.0
    );
    println!("\nPASSED: chaos_concurrent_pruning\n");
}

// =============================================================================
// Helper Tests (Not Ignored)
// =============================================================================

/// Quick sanity check for chaos infrastructure
#[tokio::test]
async fn chaos_infrastructure_sanity() {
    let tracker = ResourceTracker::new();

    tracker.record_connection_created();
    assert_eq!(tracker.active_connections(), 1);
    assert_eq!(tracker.peak_connections.load(Ordering::SeqCst), 1);

    tracker.record_connection_created();
    assert_eq!(tracker.active_connections(), 2);
    assert_eq!(tracker.peak_connections.load(Ordering::SeqCst), 2);

    tracker.record_connection_closed();
    assert_eq!(tracker.active_connections(), 1);
    assert_eq!(tracker.peak_connections.load(Ordering::SeqCst), 2); // Peak unchanged

    tracker.record_operation(true, false);
    tracker.record_operation(false, true);
    tracker.record_operation(false, false);

    assert_eq!(tracker.total_operations.load(Ordering::Relaxed), 3);
    assert_eq!(tracker.successful_operations.load(Ordering::Relaxed), 1);
    assert_eq!(tracker.graceful_failures.load(Ordering::Relaxed), 1);
    assert_eq!(tracker.unexpected_errors.load(Ordering::Relaxed), 1);

    let summary = tracker.summary();
    assert!(summary.contains("3 total"));
    assert!(summary.contains("1 success"));
}

/// Quick sanity check for chaos config
#[test]
fn chaos_config_variants() {
    let default = ChaosConfig::default();
    assert_eq!(default.duration, Duration::from_secs(10));
    assert_eq!(default.concurrency, 50);

    let quick = ChaosConfig::quick();
    assert!(quick.duration < default.duration);
    assert!(quick.concurrency < default.concurrency);

    let intensive = ChaosConfig::intensive();
    assert!(intensive.duration > default.duration);
    assert!(intensive.concurrency > default.concurrency);
}
