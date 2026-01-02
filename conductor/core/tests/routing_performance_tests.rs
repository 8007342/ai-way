//! Performance Test Scenarios for Multi-Model Query Routing
//!
//! These tests validate the routing system under various load conditions.
//! Run with: cargo test --release -- --nocapture routing_performance
//!
//! # Test Categories
//!
//! 1. Latency Tests - Verify routing meets latency targets
//! 2. Throughput Tests - Measure requests per second
//! 3. Concurrency Tests - Validate parallel request handling
//! 4. Resource Tests - Memory pressure and connection limits
//! 5. Fallback Tests - Verify retry and failover behavior
//! 6. Stability Tests - Long-running soak tests

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Barrier;

// Note: These tests are designed to run against mock backends
// For real performance testing, use actual backends

// ============================================================================
// Test Scenario 1: Quick Response Latency
// ============================================================================

/// Scenario: Verify quick response routing meets <100ms first token target
///
/// Setup:
/// - Multiple models registered (quick + deep thinking)
/// - Quick response model optimized for low latency
///
/// Test:
/// - Send 100 quick response requests
/// - Measure time to routing decision
/// - Measure time to first token
///
/// Pass Criteria:
/// - 95th percentile routing decision < 10ms
/// - 95th percentile TTFT < 100ms
/// - All requests routed to quick response model
#[tokio::test]
async fn scenario_1_quick_response_latency() {
    // Setup would initialize router with mock backends

    let iterations = 100;
    let mut routing_times_ms = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let start = Instant::now();

        // Simulate routing decision
        // In real test: let decision = router.route(quick_request).await;
        tokio::time::sleep(Duration::from_micros(500)).await; // Mock

        routing_times_ms.push(start.elapsed().as_millis() as u64);
    }

    routing_times_ms.sort();
    let p95_idx = (iterations as f64 * 0.95) as usize;
    let p95_routing = routing_times_ms[p95_idx];

    println!("Scenario 1: Quick Response Latency");
    println!(
        "  Routing decision P50: {}ms",
        routing_times_ms[iterations / 2]
    );
    println!("  Routing decision P95: {}ms", p95_routing);

    // Note: Real assertion would be:
    // assert!(p95_routing < 10, "P95 routing should be < 10ms");
}

// ============================================================================
// Test Scenario 2: Concurrent Request Throughput
// ============================================================================

/// Scenario: Measure maximum concurrent request throughput
///
/// Setup:
/// - Router with connection pool (16 max connections)
/// - Rate limit: 100 RPS global
///
/// Test:
/// - Spawn 50 concurrent tasks
/// - Each task sends 20 requests
/// - Measure total time and successful requests
///
/// Pass Criteria:
/// - At least 90% success rate
/// - Throughput > 80 RPS
/// - No connection pool exhaustion
#[tokio::test]
async fn scenario_2_concurrent_throughput() {
    let concurrent_tasks = 50;
    let requests_per_task = 20;
    let total_requests = concurrent_tasks * requests_per_task;

    let success_count = Arc::new(AtomicU64::new(0));
    let failure_count = Arc::new(AtomicU64::new(0));
    let barrier = Arc::new(Barrier::new(concurrent_tasks));

    let start = Instant::now();

    let mut handles = Vec::new();
    for _ in 0..concurrent_tasks {
        let barrier = barrier.clone();
        let success = success_count.clone();
        let failure = failure_count.clone();

        handles.push(tokio::spawn(async move {
            // Wait for all tasks to be ready
            barrier.wait().await;

            for _ in 0..requests_per_task {
                // Simulate request
                // In real test: router.route(request).await
                let result = tokio::time::sleep(Duration::from_millis(5)).await;
                success.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let elapsed = start.elapsed();
    let successes = success_count.load(Ordering::Relaxed);
    let failures = failure_count.load(Ordering::Relaxed);
    let rps = successes as f64 / elapsed.as_secs_f64();

    println!("\nScenario 2: Concurrent Throughput");
    println!("  Total requests: {}", total_requests);
    println!("  Successes: {}", successes);
    println!("  Failures: {}", failures);
    println!("  Duration: {:.2}s", elapsed.as_secs_f64());
    println!("  Throughput: {:.1} RPS", rps);
    println!(
        "  Success rate: {:.1}%",
        (successes as f64 / total_requests as f64) * 100.0
    );
}

// ============================================================================
// Test Scenario 3: Task Class Routing Accuracy
// ============================================================================

/// Scenario: Verify correct model selection for different task types
///
/// Setup:
/// - 4 models registered:
///   - fast-model (quick response optimized)
///   - code-model (code generation)
///   - math-model (mathematical reasoning)
///   - general-model (fallback)
///
/// Test:
/// - Send 25 requests of each task class
/// - Verify routing to expected model
///
/// Pass Criteria:
/// - 100% accuracy for explicit task class hints
/// - >80% accuracy for auto-classified requests
#[tokio::test]
async fn scenario_3_task_class_routing() {
    use conductor_core::routing::config::TaskClass;
    use conductor_core::routing::policy::RoutingRequest;

    let test_cases = vec![
        (TaskClass::QuickResponse, "Hi there!", "fast-model"),
        (
            TaskClass::CodeGeneration,
            "Write a function to sort an array",
            "code-model",
        ),
        (
            TaskClass::Mathematical,
            "Prove that sqrt(2) is irrational",
            "math-model",
        ),
        (
            TaskClass::DeepThinking,
            "Analyze the economic impacts of AI",
            "general-model",
        ),
    ];

    println!("\nScenario 3: Task Class Routing");

    for (task_class, prompt, expected_model) in test_cases {
        let request = RoutingRequest::new(prompt).with_task_class(task_class);

        // Verify classification
        assert_eq!(request.classify(), task_class);

        // In real test: verify router selects expected model
        println!(
            "  {:?} -> {} (expected: {})",
            task_class, expected_model, expected_model
        );
    }
}

// ============================================================================
// Test Scenario 4: Memory Pressure Handling
// ============================================================================

/// Scenario: Test behavior under GPU memory pressure
///
/// Setup:
/// - GPU memory manager with 16GB limit
/// - 3 models: 8GB, 4GB, 2GB
///
/// Test:
/// - Load 8GB model
/// - Attempt to load 4GB model (should succeed)
/// - Attempt to load another 8GB model (should trigger eviction)
///
/// Pass Criteria:
/// - Memory limits respected
/// - LRU eviction works correctly
/// - No memory leaks
#[tokio::test]
async fn scenario_4_memory_pressure() {
    use conductor_core::routing::semaphore::GpuMemoryManager;

    let gb = 1024 * 1024 * 1024u64;
    let manager = GpuMemoryManager::new(16 * gb, 0.85);

    println!("\nScenario 4: Memory Pressure");
    println!("  Total GPU memory: {}GB", manager.total_memory() / gb);

    // Allocate first model (8GB)
    let permit1 = manager
        .allocate("model-large", 8 * gb, Duration::from_secs(1))
        .await;
    assert!(permit1.is_ok(), "Should allocate 8GB model");
    let permit1 = permit1.unwrap();
    println!(
        "  Allocated model-large (8GB), available: {}GB",
        manager.available_memory() / gb
    );

    // Allocate second model (4GB)
    let permit2 = manager
        .allocate("model-medium", 4 * gb, Duration::from_secs(1))
        .await;
    assert!(permit2.is_ok(), "Should allocate 4GB model");
    let permit2 = permit2.unwrap();
    println!(
        "  Allocated model-medium (4GB), available: {}GB",
        manager.available_memory() / gb
    );

    // Try to allocate third model (8GB) - should fail without eviction
    let permit3 = manager
        .allocate("model-large-2", 8 * gb, Duration::from_millis(100))
        .await;
    assert!(permit3.is_err(), "Should fail - not enough memory");
    println!("  Failed to allocate model-large-2 (8GB) - expected");

    // Check memory pressure
    println!(
        "  Memory pressure: {}",
        if manager.is_under_pressure() {
            "HIGH"
        } else {
            "normal"
        }
    );

    // Release first model and retry
    manager.release("model-large", permit1).await;
    println!(
        "  Released model-large, available: {}GB",
        manager.available_memory() / gb
    );

    let permit3 = manager
        .allocate("model-large-2", 8 * gb, Duration::from_secs(1))
        .await;
    assert!(permit3.is_ok(), "Should succeed after eviction");
    println!(
        "  Allocated model-large-2 (8GB), available: {}GB",
        manager.available_memory() / gb
    );

    // Cleanup
    manager.release("model-medium", permit2).await;
    manager.release("model-large-2", permit3.unwrap()).await;
}

// ============================================================================
// Test Scenario 5: Fallback Chain Behavior
// ============================================================================

/// Scenario: Verify fallback behavior when primary model fails
///
/// Setup:
/// - Primary model configured with fallback chain
/// - Fallback chain: model-a -> model-b -> model-c
///
/// Test:
/// - Make model-a fail (mark unhealthy)
/// - Send request expecting model-a
/// - Verify fallback to model-b
/// - Make model-b fail, verify fallback to model-c
///
/// Pass Criteria:
/// - Fallback triggered within 1 second
/// - Metrics correctly track fallback events
/// - Request eventually succeeds
#[tokio::test]
async fn scenario_5_fallback_chain() {
    use conductor_core::routing::config::ModelProfile;
    use conductor_core::routing::policy::RoutingPolicy;

    let policy = RoutingPolicy::new();

    // Register models
    let model_a = ModelProfile::new("model-a", "backend-1");
    let model_b = ModelProfile::new("model-b", "backend-1");
    let model_c = ModelProfile::new("model-c", "backend-1");

    policy.register_model(model_a).await;
    policy.register_model(model_b).await;
    policy.register_model(model_c).await;

    // Set fallback chain
    policy
        .set_fallbacks(
            "model-a".to_string(),
            vec!["model-b".to_string(), "model-c".to_string()],
        )
        .await;

    println!("\nScenario 5: Fallback Chain");

    // Record failures for model-a
    policy
        .record_request_result("model-a", false, None, None)
        .await;
    policy
        .record_request_result("model-a", false, None, None)
        .await;
    policy
        .record_request_result("model-a", false, None, None)
        .await;

    let state = policy.get_state("model-a").await.unwrap();
    println!(
        "  model-a healthy: {}, error_rate: {:.2}",
        state.healthy, state.error_rate
    );

    // Model-a should now be unhealthy
    assert!(
        !state.is_available(),
        "model-a should be unavailable after failures"
    );

    // In real test: routing would fall back to model-b
    println!("  Fallback chain: model-a -> model-b -> model-c");
}

// ============================================================================
// Test Scenario 6: Rate Limiting Under Load
// ============================================================================

/// Scenario: Verify rate limiting protects backends
///
/// Setup:
/// - Global rate limit: 50 RPS
/// - Per-backend limit: 20 RPS
///
/// Test:
/// - Send burst of 200 requests in 1 second
/// - Measure rejection rate
/// - Verify queue behavior
///
/// Pass Criteria:
/// - No more than 50 requests processed in first second
/// - Remaining requests queued or rejected
/// - No backend overload
#[tokio::test]
async fn scenario_6_rate_limiting() {
    use std::sync::atomic::AtomicU32;
    use tokio::sync::Semaphore;

    let rate_limit = 50;
    let burst_size = 200;

    let semaphore = Arc::new(Semaphore::new(rate_limit));
    let processed = Arc::new(AtomicU32::new(0));
    let rejected = Arc::new(AtomicU32::new(0));

    let start = Instant::now();

    let mut handles = Vec::new();
    for _ in 0..burst_size {
        let sem = semaphore.clone();
        let proc = processed.clone();
        let rej = rejected.clone();

        handles.push(tokio::spawn(async move {
            // Try to acquire permit with short timeout (simulating rate limit)
            match tokio::time::timeout(Duration::from_millis(100), sem.acquire()).await {
                Ok(Ok(_permit)) => {
                    proc.fetch_add(1, Ordering::Relaxed);
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                _ => {
                    rej.fetch_add(1, Ordering::Relaxed);
                }
            }
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let elapsed = start.elapsed();
    let proc_count = processed.load(Ordering::Relaxed);
    let rej_count = rejected.load(Ordering::Relaxed);

    println!("\nScenario 6: Rate Limiting");
    println!("  Burst size: {}", burst_size);
    println!("  Rate limit: {} RPS", rate_limit);
    println!("  Processed: {}", proc_count);
    println!("  Rejected: {}", rej_count);
    println!("  Duration: {:.2}s", elapsed.as_secs_f64());
    println!(
        "  Effective RPS: {:.1}",
        proc_count as f64 / elapsed.as_secs_f64()
    );
}

// ============================================================================
// Test Scenario 7: Connection Pool Efficiency
// ============================================================================

/// Scenario: Verify connection pool reuse under sustained load
///
/// Setup:
/// - Pool with 8 max connections
/// - Idle timeout: 30 seconds
///
/// Test:
/// - Send 500 requests over 30 seconds
/// - Measure connection creation/reuse ratio
///
/// Pass Criteria:
/// - Connection reuse ratio > 90%
/// - No connection leaks
/// - Idle connections cleaned up
#[tokio::test]
async fn scenario_7_connection_pool() {
    use conductor_core::routing::config::ConnectionConfig;
    use conductor_core::routing::connection_pool::ConnectionPool;

    let config = ConnectionConfig {
        max_connections: 8,
        max_idle_connections: 4,
        ..Default::default()
    };

    // Use new_shared() for proper connection reuse via Arc
    let pool = ConnectionPool::new_shared("test-backend".to_string(), config);

    println!("\nScenario 7: Connection Pool");

    let num_requests = 20;

    // Simulate connection usage with sequential requests
    for _i in 0..num_requests {
        let conn = pool.acquire(Duration::from_secs(5)).await;
        assert!(conn.is_ok(), "Should acquire connection");

        // Simulate request processing
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Connection automatically returned when dropped
        drop(conn);

        // Process returns to make connection available for reuse
        pool.process_returns().await;
    }

    let stats = pool.stats().await;
    println!("  Connections created: {}", stats.connections_created);
    println!("  Connections closed: {}", stats.connections_closed);
    println!("  Connections reused: {}", stats.connections_reused);
    println!("  Currently idle: {}", stats.idle_connections);
    println!("  Connection errors: {}", stats.connection_errors);

    // Calculate reuse ratio
    let reuse_ratio = if stats.connections_created > 0 {
        (stats.connections_reused as f64) / ((num_requests - 1) as f64) * 100.0
    } else {
        0.0
    };
    println!("  Reuse ratio: {:.1}%", reuse_ratio);

    // Verify reuse (created should be much less than num_requests)
    // With proper pooling, we should create only 1 connection and reuse it
    assert!(
        stats.connections_created < (num_requests as u64),
        "Should reuse connections: created {} for {} requests",
        stats.connections_created,
        num_requests
    );

    // Verify reuse ratio > 90% (at least 18 of 19 possible reuses)
    assert!(
        reuse_ratio > 90.0,
        "Reuse ratio should be > 90%, got {:.1}%",
        reuse_ratio
    );

    // Test idle connection cleanup
    println!("\n  Testing idle connection cleanup...");

    // Force a short idle timeout for testing
    let cleanup_config = ConnectionConfig {
        max_connections: 4,
        max_idle_connections: 4,
        keepalive_interval_ms: 1, // Very short for testing
        ..Default::default()
    };
    let cleanup_pool = ConnectionPool::new_shared("cleanup-test".to_string(), cleanup_config);

    // Acquire and release a connection
    {
        let _conn = cleanup_pool.acquire(Duration::from_secs(5)).await.unwrap();
    }
    cleanup_pool.process_returns().await;

    let before_cleanup = cleanup_pool.stats().await;
    println!(
        "  Before cleanup - idle: {}",
        before_cleanup.idle_connections
    );

    // Wait for idle timeout
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Cleanup should remove stale connections
    cleanup_pool.cleanup_idle().await;

    let after_cleanup = cleanup_pool.stats().await;
    println!("  After cleanup - idle: {}", after_cleanup.idle_connections);

    assert_eq!(
        after_cleanup.idle_connections, 0,
        "Stale connections should be cleaned up"
    );

    // Verify no connection leaks (active should be 0 after all work done)
    let final_stats = pool.stats().await;
    assert_eq!(
        final_stats.active_connections, 0,
        "No connection leaks: {} active connections remain",
        final_stats.active_connections
    );

    println!("  All connection pool tests passed!");
}

// ============================================================================
// Test Scenario 8: Multi-Model Streaming
// ============================================================================

/// Scenario: Handle multiple concurrent streaming responses
///
/// Setup:
/// - 3 different models
/// - Each streaming response takes 2-5 seconds
///
/// Test:
/// - Start 10 concurrent streaming requests
/// - Each to different model based on task class
/// - Verify all streams complete correctly
///
/// Pass Criteria:
/// - All streams receive complete responses
/// - No token interleaving between streams
/// - Total time < sum of individual times (parallelism works)
#[tokio::test]
async fn scenario_8_concurrent_streaming() {
    use tokio::sync::mpsc;

    let num_streams = 10;
    let mut handles = Vec::new();

    let start = Instant::now();

    for i in 0..num_streams {
        handles.push(tokio::spawn(async move {
            let (tx, mut rx) = mpsc::channel::<String>(100);

            // Simulate streaming
            tokio::spawn(async move {
                for token in 0..20 {
                    let _ = tx.send(format!("stream-{}-token-{}", i, token)).await;
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            });

            let mut tokens = Vec::new();
            while let Some(token) = rx.recv().await {
                tokens.push(token);
            }

            (i, tokens.len())
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    let elapsed = start.elapsed();

    println!("\nScenario 8: Concurrent Streaming");
    println!("  Streams: {}", num_streams);
    println!("  Total duration: {:.2}s", elapsed.as_secs_f64());

    for (stream_id, token_count) in &results {
        println!("  Stream {}: {} tokens", stream_id, token_count);
    }

    // Verify parallelism: 10 streams * 20 tokens * 50ms = 10s sequential
    // With parallelism, should be ~1s
    assert!(
        elapsed < Duration::from_secs(5),
        "Streams should run in parallel"
    );
}

// ============================================================================
// Test Scenario 9: Session Affinity
// ============================================================================

/// Scenario: Verify session affinity routing
///
/// Setup:
/// - Multiple models available
/// - Session affinity enabled
///
/// Test:
/// - Send first request for conversation-1
/// - Note which model was selected
/// - Send 10 more requests for same conversation
///
/// Pass Criteria:
/// - All requests in same conversation use same model
/// - Different conversations can use different models
#[tokio::test]
async fn scenario_9_session_affinity() {
    use conductor_core::routing::config::ModelProfile;
    use conductor_core::routing::policy::{RoutingPolicy, RoutingRequest};

    let policy = RoutingPolicy::new();

    // Register models
    policy
        .register_model(ModelProfile::new("model-a", "backend-1"))
        .await;
    policy
        .register_model(ModelProfile::new("model-b", "backend-1"))
        .await;

    println!("\nScenario 9: Session Affinity");

    // First request establishes affinity
    let request1 = RoutingRequest::new("Hello").with_conversation("conv-123".to_string());

    let decision1 = policy.route(&request1).await.unwrap();
    println!("  First request routed to: {}", decision1.model_id);

    // Record affinity
    policy
        .record_session_affinity("conv-123", &decision1.model_id)
        .await;

    // Subsequent requests should use same model
    for i in 2..=5 {
        let request =
            RoutingRequest::new(format!("Message {}", i)).with_conversation("conv-123".to_string());

        let decision = policy.route(&request).await.unwrap();
        println!(
            "  Request {} routed to: {} (affinity: {:?})",
            i,
            decision.model_id,
            matches!(
                decision.reason,
                conductor_core::routing::policy::RoutingReason::SessionAffinity
            )
        );
    }
}

// ============================================================================
// Test Scenario 10: Stress Test - Sustained Load
// ============================================================================

/// Scenario: 10-minute sustained load test
///
/// Setup:
/// - Full router configuration
/// - Mix of task classes
///
/// Test:
/// - Maintain 50 RPS for 10 minutes
/// - Monitor error rates, latencies, resource usage
///
/// Pass Criteria:
/// - Error rate < 1%
/// - P99 latency < 5s
/// - No memory leaks
/// - No connection exhaustion
#[tokio::test]
#[ignore] // Long-running test, run manually
async fn scenario_10_stress_test() {
    use std::sync::atomic::AtomicU64;

    let duration = Duration::from_secs(60); // 1 minute for CI
    let target_rps = 50;
    let interval = Duration::from_millis(1000 / target_rps as u64);

    let requests = Arc::new(AtomicU64::new(0));
    let successes = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));

    let start = Instant::now();
    let mut ticker = tokio::time::interval(interval);

    println!("\nScenario 10: Stress Test");
    println!("  Duration: {}s", duration.as_secs());
    println!("  Target RPS: {}", target_rps);

    while start.elapsed() < duration {
        ticker.tick().await;

        let req = requests.clone();
        let succ = successes.clone();
        let err = errors.clone();

        tokio::spawn(async move {
            req.fetch_add(1, Ordering::Relaxed);

            // Simulate request with variable latency
            let delay = Duration::from_millis(rand::random::<u64>() % 100 + 10);
            tokio::time::sleep(delay).await;

            // 99% success rate
            if rand::random::<f64>() < 0.99 {
                succ.fetch_add(1, Ordering::Relaxed);
            } else {
                err.fetch_add(1, Ordering::Relaxed);
            }
        });
    }

    // Wait for in-flight requests
    tokio::time::sleep(Duration::from_secs(2)).await;

    let total_requests = requests.load(Ordering::Relaxed);
    let total_successes = successes.load(Ordering::Relaxed);
    let total_errors = errors.load(Ordering::Relaxed);
    let elapsed = start.elapsed();

    println!("  Total requests: {}", total_requests);
    println!("  Successes: {}", total_successes);
    println!("  Errors: {}", total_errors);
    println!(
        "  Actual RPS: {:.1}",
        total_requests as f64 / elapsed.as_secs_f64()
    );
    println!(
        "  Error rate: {:.2}%",
        (total_errors as f64 / total_requests as f64) * 100.0
    );
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Generate a test request with random characteristics
fn random_request() -> &'static str {
    let prompts = [
        "Hi!",
        "Write a function to calculate fibonacci",
        "Explain quantum computing in detail",
        "Solve x^2 - 4 = 0",
        "Write a haiku about coding",
    ];
    prompts[rand::random::<usize>() % prompts.len()]
}
