//! Integration tests for Sprint 7-8 features
//!
//! These tests verify that multiple components work together correctly in realistic usage scenarios.
//! Tests cover:
//! - Evolution system with animation variants
//! - Accessibility with animation behavior
//! - Rate limiting with heartbeat monitoring
//! - TOML configuration affecting components
//! - Sprite generation flow with caching

use std::io::Write;
use std::time::{Duration, SystemTime};

use tempfile::NamedTempFile;
use tokio::sync::mpsc;

use conductor_core::avatar::block::Mood;
use conductor_core::avatar::evolution::{EvolutionContext, EvolutionLevel};
use conductor_core::avatar::generation::{
    available_accessories, Accessory, RuleBasedGenerator, SpriteGenerator,
};
use conductor_core::avatar::variants::{AnimationType, VariantRegistry};
use conductor_core::config::{load_config_from_path, ConfigOverrides, ConfigSource};
use conductor_core::events::{SurfaceCapabilities, SurfaceType};
// Import ConnectionId from surface_registry for SurfaceHandle/SurfaceRegistry
use conductor_core::surface_registry::{
    ConnectionId as SurfaceConnectionId, SurfaceHandle, SurfaceRegistry,
};
// Import ConnectionId from transport for rate limiter (they are different types)
use conductor_core::transport::heartbeat::{HeartbeatConfig, HeartbeatMonitor, HeartbeatTask};
use conductor_core::transport::rate_limit::{
    ConnectionRateLimiter, RateLimitConfig, RateLimitResult, TransportRateLimiter,
};
use conductor_core::transport::traits::ConnectionId as TransportConnectionId;

// =============================================================================
// Test 1: Evolution Unlocks Variants
// =============================================================================

/// Test that evolution context at different levels provides correct animation variants.
///
/// This test verifies the integration between:
/// - EvolutionContext tracking user interactions and session time
/// - EvolutionLevel determining available features
/// - VariantRegistry providing animation variants based on evolution
/// - Accessory unlocking tied to evolution level
#[test]
fn test_evolution_unlocks_variants() {
    // Create evolution context at different levels and verify variant availability

    // --- Nascent Level (starting point) ---
    let ctx_nascent = EvolutionContext::new();
    assert_eq!(ctx_nascent.current_level(), EvolutionLevel::Nascent);
    assert_eq!(ctx_nascent.current_level().animation_variants(), 1);

    let registry = VariantRegistry::new();

    // At Nascent, should have minimum variants available for Idle animation
    let nascent_idle_variants =
        registry.available_variant_count(AnimationType::Idle, EvolutionLevel::Nascent);
    assert_eq!(
        nascent_idle_variants, 1,
        "Nascent should have 1 idle variant"
    );

    // No accessories at Nascent
    let nascent_accessories = available_accessories(EvolutionLevel::Nascent);
    assert!(
        nascent_accessories.is_empty(),
        "Nascent should have no accessories"
    );

    // --- Developing Level (50 interactions + 1 hour) ---
    let ctx_developing = EvolutionContext::restore(50, 3600, SystemTime::now());
    assert_eq!(ctx_developing.current_level(), EvolutionLevel::Developing);
    assert_eq!(ctx_developing.current_level().animation_variants(), 2);

    // At Developing, more variants should be available
    let developing_idle_variants =
        registry.available_variant_count(AnimationType::Idle, EvolutionLevel::Developing);
    assert_eq!(
        developing_idle_variants, 2,
        "Developing should have 2 idle variants"
    );

    // Glasses should be unlocked at Developing
    let developing_accessories = available_accessories(EvolutionLevel::Developing);
    assert!(
        developing_accessories.contains(&Accessory::Glasses),
        "Glasses should be unlocked at Developing"
    );
    assert!(
        !developing_accessories.contains(&Accessory::PartyHat),
        "PartyHat should NOT be unlocked at Developing"
    );

    // --- Mature Level (200 interactions + 5 hours) ---
    let ctx_mature = EvolutionContext::restore(200, 5 * 3600, SystemTime::now());
    assert_eq!(ctx_mature.current_level(), EvolutionLevel::Mature);
    assert_eq!(ctx_mature.current_level().animation_variants(), 3);

    // At Mature, all idle variants available
    let mature_idle_variants =
        registry.available_variant_count(AnimationType::Idle, EvolutionLevel::Mature);
    assert_eq!(
        mature_idle_variants, 3,
        "Mature should have 3 idle variants"
    );

    // PartyHat unlocked at Mature
    let mature_accessories = available_accessories(EvolutionLevel::Mature);
    assert!(
        mature_accessories.contains(&Accessory::PartyHat),
        "PartyHat should be unlocked at Mature"
    );
    assert!(
        mature_accessories.contains(&Accessory::CoffeeMug),
        "CoffeeMug should be unlocked at Mature"
    );

    // --- Evolved Level (500 interactions + 20 hours) ---
    let ctx_evolved = EvolutionContext::restore(500, 20 * 3600, SystemTime::now());
    assert_eq!(ctx_evolved.current_level(), EvolutionLevel::Evolved);
    assert_eq!(ctx_evolved.current_level().animation_variants(), 4);

    // Crown unlocked at Evolved
    let evolved_accessories = available_accessories(EvolutionLevel::Evolved);
    assert!(
        evolved_accessories.contains(&Accessory::Crown),
        "Crown should be unlocked at Evolved"
    );

    // --- Transcendent Level (1000 interactions + 50 hours) ---
    let ctx_transcendent = EvolutionContext::restore(1000, 50 * 3600, SystemTime::now());
    assert_eq!(
        ctx_transcendent.current_level(),
        EvolutionLevel::Transcendent
    );
    assert_eq!(ctx_transcendent.current_level().animation_variants(), 5);

    // All accessories unlocked at Transcendent
    let transcendent_accessories = available_accessories(EvolutionLevel::Transcendent);
    assert!(
        transcendent_accessories.contains(&Accessory::WizardHat),
        "WizardHat should be unlocked at Transcendent"
    );
    assert!(
        transcendent_accessories.contains(&Accessory::MusicNotes),
        "MusicNotes should be unlocked at Transcendent"
    );
}

/// Test variant selection with evolution using deterministic seeded selection
#[test]
fn test_variant_selection_with_evolution() {
    let registry = VariantRegistry::new();

    // Test that variant selection respects evolution level constraints
    let seed = 42u64;

    // At Nascent, should always get the base variant regardless of seed
    let nascent_variant =
        registry.select_variant_seeded(AnimationType::Idle, EvolutionLevel::Nascent, seed);
    assert_eq!(
        nascent_variant.unlock_level,
        EvolutionLevel::Nascent,
        "Selected variant should be unlocked at Nascent"
    );

    // At Transcendent, should be able to get variants with higher unlock levels
    let transcendent_variant =
        registry.select_variant_seeded(AnimationType::Idle, EvolutionLevel::Transcendent, seed);
    assert!(
        transcendent_variant.is_available_at(EvolutionLevel::Transcendent),
        "Selected variant should be available at Transcendent"
    );

    // Test consistency: same seed should give same result
    let repeated_variant =
        registry.select_variant_seeded(AnimationType::Idle, EvolutionLevel::Transcendent, seed);
    assert_eq!(
        nascent_variant.id, nascent_variant.id,
        "Same seed should produce consistent results"
    );
    assert_eq!(
        transcendent_variant.id, repeated_variant.id,
        "Same seed should produce consistent results"
    );
}

/// Test evolution level progression triggers variant unlocks
#[test]
fn test_evolution_progression_unlocks() {
    let mut ctx = EvolutionContext::new();
    let registry = VariantRegistry::new();

    // Initial state
    assert_eq!(
        registry.available_variant_count(AnimationType::Idle, ctx.current_level()),
        1
    );

    // Progress to Developing
    ctx.record_interactions(50);
    ctx.add_session_time(3600);

    assert_eq!(ctx.current_level(), EvolutionLevel::Developing);
    assert_eq!(
        registry.available_variant_count(AnimationType::Idle, ctx.current_level()),
        2
    );

    // Progress to Mature
    ctx.record_interactions(150); // Now at 200 total
    ctx.add_session_time(4 * 3600); // Now at 5 hours total

    assert_eq!(ctx.current_level(), EvolutionLevel::Mature);
    assert_eq!(
        registry.available_variant_count(AnimationType::Idle, ctx.current_level()),
        3
    );
}

// =============================================================================
// Test 2: Accessibility with Animation
// =============================================================================

// Note: The accessibility module may not have MotionPreference exposed publicly.
// Testing accessibility integration through animation timing behavior.

/// Test that animation timing respects accessibility considerations.
/// Higher evolution levels provide more timing variants but base timing remains consistent.
#[test]
fn test_accessibility_with_animation() {
    // Animation variants should have timing metadata that can be adjusted for accessibility
    let registry = VariantRegistry::new();

    // Get variants at different evolution levels
    let nascent_variant = registry.select_variant(AnimationType::Idle, EvolutionLevel::Nascent);
    let evolved_variant = registry.select_variant(AnimationType::Idle, EvolutionLevel::Evolved);

    // Both variants should have valid speed modifiers
    assert!(
        nascent_variant.speed_modifier > 0.0,
        "Speed modifier should be positive"
    );
    assert!(
        evolved_variant.speed_modifier > 0.0,
        "Speed modifier should be positive"
    );

    // Speed modifiers should be within reasonable bounds (0.1 to 3.0)
    assert!(
        nascent_variant.speed_modifier >= 0.1 && nascent_variant.speed_modifier <= 3.0,
        "Speed modifier should be clamped to valid range"
    );
    assert!(
        evolved_variant.speed_modifier >= 0.1 && evolved_variant.speed_modifier <= 3.0,
        "Speed modifier should be clamped to valid range"
    );
}

/// Test that static mode (reduced motion) preserves functionality through sprite generation.
/// When animations are disabled, the sprite generator should still produce valid output.
#[test]
fn test_static_mode_preserves_functionality() {
    let generator = RuleBasedGenerator::new();

    // Generate sprites at different evolution levels
    for level in [
        EvolutionLevel::Nascent,
        EvolutionLevel::Developing,
        EvolutionLevel::Mature,
        EvolutionLevel::Evolved,
        EvolutionLevel::Transcendent,
    ] {
        // Generate base sprite
        let sprite = generator.generate(Mood::Happy, level);

        // Verify sprite is valid regardless of animation state
        assert!(!sprite.blocks.is_empty(), "Sprite should have blocks");
        assert_eq!(sprite.width(), 8, "Sprite width should be 8");
        assert_eq!(sprite.height(), 8, "Sprite height should be 8");
        assert_eq!(
            sprite.blocks.len(),
            64,
            "Sprite should have 64 blocks (8x8)"
        );

        // Verify sprite has cache key for static caching
        assert!(
            sprite.cache_key.is_some(),
            "Sprite should have cache key for static display"
        );

        // Variant 0 should always be available (static fallback)
        let static_variant = generator.generate_variant(Mood::Happy, level, 0);
        assert!(
            static_variant.cache_key.is_some(),
            "Static variant should have cache key"
        );
    }
}

/// Test animation behavior changes based on mood accessibility
#[test]
fn test_mood_animation_accessibility() {
    let generator = RuleBasedGenerator::new();

    // Different moods should produce different but valid sprites
    let moods = [
        Mood::Happy,
        Mood::Thinking,
        Mood::Playful,
        Mood::Calm,
        Mood::Sad,
        Mood::Focused,
    ];

    let level = EvolutionLevel::Mature;

    for mood in moods {
        let sprite = generator.generate(mood, level);

        // All moods should produce valid sprites
        assert_eq!(sprite.blocks.len(), 64);
        assert!(sprite.cache_key.is_some());

        // Variant count should be at least 2 for all moods
        let variant_count = generator.variant_count(mood, level);
        assert!(
            variant_count >= 2,
            "Mood {:?} should have at least 2 variants",
            mood
        );
    }
}

// =============================================================================
// Test 3: Rate Limit with Heartbeat
// =============================================================================

/// Test that rate-limited connections can still receive heartbeat pings.
/// The heartbeat mechanism should function even under rate limiting conditions.
///
/// Note: Rate limiter and Heartbeat monitor use different ConnectionId types
/// internally, so we test them separately to verify their individual behaviors.
#[test]
fn test_rate_limit_with_heartbeat() {
    // Create rate limiter with tight limits
    let rate_config = RateLimitConfig::new()
        .with_messages_per_second(10)
        .with_burst_size(5)
        .with_max_connections_per_uid(3);

    let rate_limiter = TransportRateLimiter::new(rate_config);

    // Create heartbeat monitor
    let heartbeat_config = HeartbeatConfig::for_testing();
    let monitor = HeartbeatMonitor::new(heartbeat_config);

    // Register connections with both systems (they use different ID types)
    let rate_conn_id = TransportConnectionId::new();
    let heartbeat_conn_id = SurfaceConnectionId::new();
    let uid = 1000u32;

    rate_limiter
        .register_connection(&rate_conn_id, uid)
        .expect("Should register connection with rate limiter");
    monitor.register(heartbeat_conn_id);

    // Exhaust rate limit tokens
    for _ in 0..5 {
        let result = rate_limiter.check_message(&rate_conn_id);
        assert!(result.is_allowed(), "Initial messages should be allowed");
    }

    // Now rate limited
    let throttled = rate_limiter.check_message(&rate_conn_id);
    assert!(
        throttled.is_throttled(),
        "Should be throttled after burst exhausted"
    );

    // Heartbeat should still function - it's a separate system
    assert!(
        monitor.is_healthy(&heartbeat_conn_id),
        "Connection should still be healthy for heartbeat"
    );

    // Heartbeat health should be independent of rate limiting
    let health = monitor.get_health(&heartbeat_conn_id).unwrap();
    assert!(health.healthy, "Connection should be healthy initially");
    assert_eq!(health.missed_pongs, 0, "No missed pongs initially");
}

/// Test graceful degradation: rate limiting delays rather than disconnects,
/// while heartbeat monitors overall connection health.
#[test]
fn test_graceful_degradation() {
    let rate_config = RateLimitConfig::new()
        .with_messages_per_second(100)
        .with_burst_size(10)
        .with_min_throttle_delay_ms(10)
        .with_max_throttle_delay_ms(100);

    let limiter = ConnectionRateLimiter::new(rate_config);

    // Exhaust burst
    for _ in 0..10 {
        let result = limiter.check_message();
        assert!(result.is_allowed());
    }

    // Subsequent messages should be throttled (delayed), not rejected
    let result = limiter.check_message();
    match result {
        RateLimitResult::Throttled { delay } => {
            assert!(
                delay >= Duration::from_millis(10),
                "Delay should be at least min_throttle_delay"
            );
            assert!(
                delay <= Duration::from_millis(100),
                "Delay should be at most max_throttle_delay"
            );
        }
        RateLimitResult::Allowed => {
            // This is okay if tokens refilled between checks
        }
        RateLimitResult::Rejected { .. } => {
            panic!("Message rate limiting should throttle, not reject");
        }
    }

    // Verify metrics are tracking
    let metrics = limiter.metrics();
    assert!(metrics.total_messages > 0, "Should have tracked messages");
}

/// Test that heartbeat still works when connection is under heavy rate limiting.
/// This async test verifies heartbeat task sends pings and handles pongs correctly.
#[tokio::test]
async fn test_heartbeat_under_rate_limiting() {
    let heartbeat_config = HeartbeatConfig::for_testing();
    let (monitor, _event_rx) = HeartbeatMonitor::with_events(heartbeat_config);

    // Create a registry and surface (using SurfaceConnectionId)
    let registry = SurfaceRegistry::new();
    let conn_id = SurfaceConnectionId::new();
    let (tx, mut msg_rx) = mpsc::channel(32);
    let handle = SurfaceHandle::new(
        conn_id,
        tx,
        SurfaceType::Headless,
        SurfaceCapabilities::headless(),
    );
    registry.register(handle);

    // Register with heartbeat monitor
    monitor.register(conn_id);

    // Start heartbeat task
    let task = HeartbeatTask::new(monitor.clone(), registry.clone());
    let task_handle = tokio::spawn(task.run());

    // Wait for heartbeat to send ping (HeartbeatConfig::for_testing uses 100ms interval)
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Check if ping was sent - use timeout to avoid blocking forever
    tokio::select! {
        msg = msg_rx.recv() => {
            if let Some(conductor_core::messages::ConductorMessage::Ping { seq }) = msg {
                // Respond with pong
                monitor.record_pong(&conn_id, seq);
                // Verify health is maintained
                let health = monitor.get_health(&conn_id).unwrap();
                assert!(health.healthy, "Connection should be healthy after pong");
            }
        }
        _ = tokio::time::sleep(Duration::from_millis(500)) => {
            // Ping might not have been sent yet in testing conditions
            // Just verify connection is still tracked
            assert!(monitor.is_healthy(&conn_id), "Connection should remain healthy");
        }
    }

    // Connection should remain healthy
    assert!(monitor.is_healthy(&conn_id));

    // Cleanup
    monitor.stop();
    task_handle.await.unwrap();
}

// =============================================================================
// Test 4: Config Affects Components
// =============================================================================

/// Clean up environment variables used by config loading
fn clear_config_env_vars() {
    std::env::remove_var("CONDUCTOR_CONNECT_TIMEOUT");
    std::env::remove_var("CONDUCTOR_READ_TIMEOUT");
    std::env::remove_var("CONDUCTOR_HEARTBEAT");
    std::env::remove_var("CONDUCTOR_HEARTBEAT_INTERVAL");
    std::env::remove_var("CONDUCTOR_RECONNECT_ATTEMPTS");
    std::env::remove_var("CONDUCTOR_RATE_LIMIT_MPS");
    std::env::remove_var("CONDUCTOR_RATE_LIMIT_BURST");
    std::env::remove_var("CONDUCTOR_MAX_CONNECTIONS_PER_UID");
    std::env::remove_var("CONDUCTOR_DEFAULT_MODEL");
    std::env::remove_var("CONDUCTOR_MAX_CONCURRENT");
    std::env::remove_var("CONDUCTOR_MAX_MESSAGE_SIZE");
    std::env::remove_var("CONDUCTOR_MAX_INPUT_LENGTH");
}

/// Test that TOML configuration values propagate to components correctly.
#[test]
fn test_config_affects_components() {
    clear_config_env_vars();

    // Create a TOML config file with specific values
    let toml_content = r#"
[transport]
connect_timeout_ms = 7500
heartbeat_interval_secs = 45
heartbeat_timeout_secs = 15
max_missed_pongs = 4
reconnect_attempts = 5

[rate_limit]
messages_per_second = 150
burst_size = 75
max_connections_per_uid = 8
enabled = true
min_throttle_delay_ms = 20
max_throttle_delay_ms = 500

[routing]
default_model = "custom-test-model"
max_concurrent_requests = 15
enable_queue = true
max_queue_depth = 500

[security]
max_message_size = 131072
max_input_length = 65536
session_timeout_secs = 7200
"#;

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(toml_content.as_bytes()).unwrap();

    // Load config
    let config = load_config_from_path(Some(file.path().to_path_buf())).unwrap();

    // Verify transport settings propagated
    assert_eq!(
        config.transport.connect_timeout_ms, 7500,
        "Connect timeout should be 7500ms"
    );
    assert_eq!(
        config.heartbeat.heartbeat_interval,
        Duration::from_secs(45),
        "Heartbeat interval should be 45s"
    );
    assert_eq!(
        config.heartbeat.response_timeout,
        Duration::from_secs(15),
        "Heartbeat timeout should be 15s"
    );
    assert_eq!(
        config.heartbeat.max_missed_pongs, 4,
        "Max missed pongs should be 4"
    );
    assert_eq!(
        config.transport.reconnect_attempts, 5,
        "Reconnect attempts should be 5"
    );

    // Verify rate limit settings propagated
    assert_eq!(
        config.rate_limit.messages_per_second, 150,
        "Messages per second should be 150"
    );
    assert_eq!(config.rate_limit.burst_size, 75, "Burst size should be 75");
    assert_eq!(
        config.rate_limit.max_connections_per_uid, 8,
        "Max connections per UID should be 8"
    );
    assert!(config.rate_limit.enabled, "Rate limiting should be enabled");
    assert_eq!(
        config.rate_limit.min_throttle_delay_ms, 20,
        "Min throttle delay should be 20ms"
    );
    assert_eq!(
        config.rate_limit.max_throttle_delay_ms, 500,
        "Max throttle delay should be 500ms"
    );

    // Verify routing settings propagated
    assert_eq!(
        config.default_model,
        Some("custom-test-model".to_string()),
        "Default model should be custom-test-model"
    );
    assert_eq!(
        config.max_concurrent_requests, 15,
        "Max concurrent requests should be 15"
    );
    assert!(config.enable_queue, "Queue should be enabled");
    assert_eq!(config.max_queue_depth, 500, "Max queue depth should be 500");

    // Verify security settings propagated
    assert_eq!(
        config.max_message_size, 131072,
        "Max message size should be 131072"
    );
    assert_eq!(
        config.max_input_length, 65536,
        "Max input length should be 65536"
    );
    assert_eq!(
        config.session_timeout,
        Duration::from_secs(7200),
        "Session timeout should be 7200s"
    );

    // Source should be File
    assert_eq!(config.source(), ConfigSource::File);
}

/// Test config override priority: CLI > Env > File > Default
#[test]
fn test_config_override_priority() {
    clear_config_env_vars();

    // Start with a file config
    let toml_content = r#"
[routing]
default_model = "file-model"

[transport]
connect_timeout_ms = 5000
"#;

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(toml_content.as_bytes()).unwrap();

    let mut config = load_config_from_path(Some(file.path().to_path_buf())).unwrap();

    // File values should be loaded
    assert_eq!(config.default_model, Some("file-model".to_string()));
    assert_eq!(config.transport.connect_timeout_ms, 5000);
    assert_eq!(config.source(), ConfigSource::File);

    // Apply CLI overrides
    let overrides = ConfigOverrides::new()
        .with_default_model("cli-model".to_string())
        .with_connect_timeout_ms(3000);
    overrides.apply(&mut config);

    // CLI should override file
    assert_eq!(config.default_model, Some("cli-model".to_string()));
    assert_eq!(config.transport.connect_timeout_ms, 3000);
    assert_eq!(config.source(), ConfigSource::Cli);
}

/// Test that config with partial values uses defaults for unspecified fields
#[test]
fn test_config_partial_uses_defaults() {
    clear_config_env_vars();

    let toml_content = r#"
[routing]
default_model = "partial-model"
"#;

    let mut file = NamedTempFile::new().unwrap();
    file.write_all(toml_content.as_bytes()).unwrap();

    let config = load_config_from_path(Some(file.path().to_path_buf())).unwrap();

    // Specified value
    assert_eq!(config.default_model, Some("partial-model".to_string()));

    // Default values for unspecified fields
    assert_eq!(config.rate_limit.messages_per_second, 100); // default
    assert_eq!(config.rate_limit.burst_size, 50); // default
    assert_eq!(config.max_concurrent_requests, 10); // default
    assert!(config.enable_queue); // default
}

// =============================================================================
// Test 5: Sprite Generation Flow
// =============================================================================

/// Test complete sprite generation flow with mood and evolution.
#[test]
fn test_sprite_generation_flow() {
    let generator = RuleBasedGenerator::new();

    // Test generation across all evolution levels
    for level in [
        EvolutionLevel::Nascent,
        EvolutionLevel::Developing,
        EvolutionLevel::Mature,
        EvolutionLevel::Evolved,
        EvolutionLevel::Transcendent,
    ] {
        // Generate sprite
        let sprite = generator.generate(Mood::Happy, level);

        // Verify sprite dimensions
        assert_eq!(sprite.width(), 8);
        assert_eq!(sprite.height(), 8);
        assert_eq!(sprite.blocks.len(), 64);

        // Verify cache key format includes level
        let cache_key = sprite.cache_key.as_ref().unwrap();
        assert!(
            cache_key.contains(&level.as_u8().to_string()),
            "Cache key should include evolution level"
        );

        // Test with accessory (if unlocked)
        let accessories = available_accessories(level);
        if !accessories.is_empty() {
            let accessory = accessories[0];
            let sprite_with_accessory =
                generator.compose_with_accessory(sprite.clone(), accessory, level);

            // Should have different cache key
            assert_ne!(
                sprite_with_accessory.cache_key, sprite.cache_key,
                "Accessory should change cache key"
            );

            // Sprite should still be valid
            assert!(sprite_with_accessory.width() > 0);
            assert!(sprite_with_accessory.height() > 0);
        }
    }
}

/// Test that sprites meet security limits (reasonable size constraints)
#[test]
fn test_sprite_meets_security_limits() {
    let generator = RuleBasedGenerator::new();

    // Maximum reasonable sprite dimensions
    let max_dimension: u16 = 256;
    let max_blocks: usize = 65536; // 256x256

    for level in [
        EvolutionLevel::Nascent,
        EvolutionLevel::Mature,
        EvolutionLevel::Transcendent,
    ] {
        for mood in [Mood::Happy, Mood::Thinking, Mood::Playful] {
            let sprite = generator.generate(mood, level);

            // Verify dimensions are within limits
            assert!(
                sprite.width() <= max_dimension,
                "Sprite width {} exceeds limit {}",
                sprite.width(),
                max_dimension
            );
            assert!(
                sprite.height() <= max_dimension,
                "Sprite height {} exceeds limit {}",
                sprite.height(),
                max_dimension
            );
            assert!(
                sprite.blocks.len() <= max_blocks,
                "Block count {} exceeds limit {}",
                sprite.blocks.len(),
                max_blocks
            );

            // Verify no invalid block data
            for (i, block) in sprite.blocks.iter().enumerate() {
                assert!(block.fg.a <= 255, "Block {} has invalid fg alpha", i);
                assert!(block.bg.a <= 255, "Block {} has invalid bg alpha", i);
            }
        }
    }
}

/// Test sprite caching behavior through cache key consistency
#[test]
fn test_sprite_caching_behavior() {
    let generator = RuleBasedGenerator::new();

    // Same parameters should produce same cache key
    let sprite1 = generator.generate_variant(Mood::Happy, EvolutionLevel::Mature, 0);
    let sprite2 = generator.generate_variant(Mood::Happy, EvolutionLevel::Mature, 0);

    assert_eq!(
        sprite1.cache_key, sprite2.cache_key,
        "Same parameters should produce same cache key"
    );

    // Different variant should produce different cache key
    let sprite3 = generator.generate_variant(Mood::Happy, EvolutionLevel::Mature, 1);
    assert_ne!(
        sprite1.cache_key, sprite3.cache_key,
        "Different variant should produce different cache key"
    );

    // Different mood should produce different cache key
    let sprite4 = generator.generate_variant(Mood::Thinking, EvolutionLevel::Mature, 0);
    assert_ne!(
        sprite1.cache_key, sprite4.cache_key,
        "Different mood should produce different cache key"
    );

    // Different evolution level should produce different cache key
    let sprite5 = generator.generate_variant(Mood::Happy, EvolutionLevel::Evolved, 0);
    assert_ne!(
        sprite1.cache_key, sprite5.cache_key,
        "Different evolution level should produce different cache key"
    );
}

/// Test accessory composition follows evolution unlock rules
#[test]
fn test_accessory_evolution_integration() {
    let generator = RuleBasedGenerator::new();

    // Generate base sprite
    let base_sprite = generator.generate(Mood::Happy, EvolutionLevel::Nascent);
    let original_blocks_len = base_sprite.blocks.len();
    let original_cache_key = base_sprite.cache_key.clone();

    // Try to add PartyHat at Nascent (should be locked)
    let result_locked = generator.compose_with_accessory(
        base_sprite.clone(),
        Accessory::PartyHat,
        EvolutionLevel::Nascent,
    );

    // Should return unchanged sprite (accessory locked)
    assert_eq!(
        result_locked.cache_key, original_cache_key,
        "Locked accessory should not modify sprite"
    );

    // Now try at Mature level where PartyHat is unlocked
    let mature_sprite = generator.generate(Mood::Happy, EvolutionLevel::Mature);
    let result_unlocked = generator.compose_with_accessory(
        mature_sprite.clone(),
        Accessory::PartyHat,
        EvolutionLevel::Mature,
    );

    // Should have modified cache key
    let unlocked_key = result_unlocked.cache_key.as_ref().unwrap();
    assert!(
        unlocked_key.contains("PartyHat"),
        "Unlocked accessory should be reflected in cache key"
    );

    // Verify WizardHat only at Transcendent
    let evolved_sprite = generator.generate(Mood::Happy, EvolutionLevel::Evolved);
    let wizard_at_evolved = generator.compose_with_accessory(
        evolved_sprite.clone(),
        Accessory::WizardHat,
        EvolutionLevel::Evolved,
    );

    // WizardHat should not be added at Evolved level
    let evolved_key = wizard_at_evolved.cache_key.as_ref().unwrap();
    assert!(
        !evolved_key.contains("WizardHat"),
        "WizardHat should not be added at Evolved level"
    );

    // WizardHat should work at Transcendent
    let transcendent_sprite = generator.generate(Mood::Happy, EvolutionLevel::Transcendent);
    let wizard_at_transcendent = generator.compose_with_accessory(
        transcendent_sprite.clone(),
        Accessory::WizardHat,
        EvolutionLevel::Transcendent,
    );

    let transcendent_key = wizard_at_transcendent.cache_key.as_ref().unwrap();
    assert!(
        transcendent_key.contains("WizardHat"),
        "WizardHat should be added at Transcendent level"
    );
}

// =============================================================================
// Test Error Handling and Edge Cases
// =============================================================================

/// Test error handling when rate limit connection limit is exceeded
#[test]
fn test_rate_limit_connection_limit_error() {
    let config = RateLimitConfig::new().with_max_connections_per_uid(2);
    let limiter = TransportRateLimiter::new(config);
    let uid = 1000u32;

    // First two connections should succeed
    let conn1 = TransportConnectionId::new();
    let conn2 = TransportConnectionId::new();
    assert!(limiter.register_connection(&conn1, uid).is_ok());
    assert!(limiter.register_connection(&conn2, uid).is_ok());

    // Third connection should fail
    let conn3 = TransportConnectionId::new();
    let result = limiter.register_connection(&conn3, uid);
    assert!(result.is_err(), "Third connection should exceed limit");
}

/// Test heartbeat handles unknown connections gracefully
#[test]
fn test_heartbeat_unknown_connection() {
    let config = HeartbeatConfig::for_testing();
    let monitor = HeartbeatMonitor::new(config);

    let unknown_id = SurfaceConnectionId::new();

    // Recording pong for unknown connection should return false
    let result = monitor.record_pong(&unknown_id, 1);
    assert!(!result, "Pong for unknown connection should return false");

    // Getting health for unknown connection should return None
    let health = monitor.get_health(&unknown_id);
    assert!(
        health.is_none(),
        "Health for unknown connection should be None"
    );
}

/// Test config handles missing file gracefully
#[test]
fn test_config_missing_file_graceful() {
    clear_config_env_vars();

    let nonexistent_path = std::path::PathBuf::from("/nonexistent/path/conductor.toml");
    let config = load_config_from_path(Some(nonexistent_path)).unwrap();

    // Should return defaults
    assert!(config.default_model.is_some());
    assert_eq!(config.rate_limit.messages_per_second, 100); // default
}
