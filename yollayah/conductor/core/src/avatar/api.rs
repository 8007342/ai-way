//! Sprite Generation API (P4.4)
//!
//! This module provides a service-based API for sprite generation requests.
//! It integrates with the `SpriteRequest`/`SpriteResponse` message types and
//! the `RuleBasedGenerator` for sprite generation.
//!
//! # Design Philosophy
//!
//! The `SpriteService` acts as the main entry point for sprite generation:
//! - Accepts `SpriteRequest` messages from surfaces
//! - Routes to the appropriate generator (`RuleBasedGenerator`)
//! - Caches results in `SpriteCache` for efficient retrieval
//! - Supports both sync and async generation patterns
//!
//! # Progress Reporting
//!
//! For long-running generation operations, the API provides:
//! - `GenerationStatus` enum for tracking state
//! - Progress percentage and ETA estimation
//! - Cancellation support for pending requests
//!
//! # Usage
//!
//! ```
//! use conductor_core::avatar::api::{SpriteService, GenerationOptions};
//! use conductor_core::avatar::block::Mood;
//! use conductor_core::avatar::evolution::EvolutionLevel;
//!
//! // Create a sprite service
//! let mut service = SpriteService::new();
//!
//! // Request a sprite synchronously
//! let response = service.request_sprite(Mood::Happy, EvolutionLevel::Mature);
//! assert!(!response.blocks.is_empty());
//!
//! // Check if sprite was cached
//! assert!(response.cache_key.is_some());
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::block::{Mood, SpriteRequest, SpriteResponse};
use super::cache::{SpriteCache, SpriteData};
use super::evolution::EvolutionLevel;
use super::generation::{Accessory, RuleBasedGenerator, SpriteGenerator};

// =============================================================================
// Request ID Generation
// =============================================================================

/// Unique identifier for generation requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(pub u64);

impl RequestId {
    /// Generate a new unique request ID
    #[must_use]
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "req_{}", self.0)
    }
}

// =============================================================================
// Generation Options
// =============================================================================

/// Configuration options for sprite generation
///
/// These options control how sprites are generated and cached.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerationOptions {
    /// Whether to use cached sprites if available
    pub use_cache: bool,
    /// Whether to cache the generated sprite
    pub cache_result: bool,
    /// Specific variant index to generate (None = auto-select)
    pub variant: Option<u8>,
    /// Optional accessory to add to the sprite
    pub accessory: Option<Accessory>,
    /// Session ID for cache scoping
    pub session_id: Option<String>,
    /// TTL for cached sprites (None = use default)
    pub cache_ttl: Option<Duration>,
}

impl Default for GenerationOptions {
    fn default() -> Self {
        Self {
            use_cache: true,
            cache_result: true,
            variant: None,
            accessory: None,
            session_id: None,
            cache_ttl: None,
        }
    }
}

impl GenerationOptions {
    /// Create options with caching disabled
    #[must_use]
    pub fn no_cache() -> Self {
        Self {
            use_cache: false,
            cache_result: false,
            ..Default::default()
        }
    }

    /// Set a specific variant
    #[must_use]
    pub fn with_variant(mut self, variant: u8) -> Self {
        self.variant = Some(variant);
        self
    }

    /// Set an accessory to add
    #[must_use]
    pub fn with_accessory(mut self, accessory: Accessory) -> Self {
        self.accessory = Some(accessory);
        self
    }

    /// Set the session ID for cache scoping
    #[must_use]
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set a custom cache TTL
    #[must_use]
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = Some(ttl);
        self
    }
}

// =============================================================================
// Generation Status
// =============================================================================

/// Status of an async generation request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GenerationStatus {
    /// Request is queued and waiting to be processed
    Pending {
        /// Position in queue (0 = next)
        queue_position: usize,
    },
    /// Generation is in progress
    Generating {
        /// Progress percentage (0-100)
        progress_percent: u8,
        /// Estimated time remaining
        eta: Option<Duration>,
        /// Current generation stage
        stage: GenerationStage,
    },
    /// Generation completed successfully
    Complete {
        /// The generated sprite response
        response: SpriteResponse,
        /// Time taken to generate
        elapsed: Duration,
    },
    /// Generation failed
    Failed {
        /// Error description
        error: String,
        /// Whether the request can be retried
        retryable: bool,
    },
    /// Generation was cancelled
    Cancelled,
    /// Request ID not found
    NotFound,
}

impl GenerationStatus {
    /// Check if the generation is complete (success or failure)
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Complete { .. } | Self::Failed { .. } | Self::Cancelled | Self::NotFound
        )
    }

    /// Check if the generation completed successfully
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Complete { .. })
    }

    /// Get the progress percentage if generating
    #[must_use]
    pub fn progress(&self) -> Option<u8> {
        match self {
            Self::Pending { .. } => Some(0),
            Self::Generating {
                progress_percent, ..
            } => Some(*progress_percent),
            Self::Complete { .. } => Some(100),
            _ => None,
        }
    }
}

/// Current stage of sprite generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenerationStage {
    /// Initializing generator
    Initializing,
    /// Generating base sprite
    GeneratingBase,
    /// Applying mood overlay
    ApplyingMood,
    /// Adding accessory
    AddingAccessory,
    /// Finalizing and caching
    Finalizing,
}

impl GenerationStage {
    /// Get the typical progress percentage for this stage
    #[must_use]
    pub fn typical_progress(&self) -> u8 {
        match self {
            Self::Initializing => 5,
            Self::GeneratingBase => 30,
            Self::ApplyingMood => 60,
            Self::AddingAccessory => 80,
            Self::Finalizing => 95,
        }
    }
}

// =============================================================================
// Generation Progress
// =============================================================================

/// Progress information for a generation request
///
/// Note: The timing fields (`created_at`, `updated_at`) use `Instant` which
/// cannot be serialized. Use `elapsed()` and `since_update()` methods instead.
#[derive(Debug, Clone)]
pub struct GenerationProgress {
    /// Unique request identifier
    pub request_id: RequestId,
    /// Current status
    pub status: GenerationStatus,
    /// When the request was created
    created_at: Instant,
    /// When the request was last updated
    updated_at: Instant,
}

impl GenerationProgress {
    /// Create new progress for a pending request
    fn new_pending(request_id: RequestId, queue_position: usize) -> Self {
        let now = Instant::now();
        Self {
            request_id,
            status: GenerationStatus::Pending { queue_position },
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the status
    fn update(&mut self, status: GenerationStatus) {
        self.status = status;
        self.updated_at = Instant::now();
    }

    /// Get elapsed time since creation
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Get time since last update
    #[must_use]
    pub fn since_update(&self) -> Duration {
        self.updated_at.elapsed()
    }

    /// Get the created_at instant
    #[must_use]
    pub fn created_at(&self) -> Instant {
        self.created_at
    }

    /// Get the updated_at instant
    #[must_use]
    pub fn updated_at(&self) -> Instant {
        self.updated_at
    }
}

impl PartialEq for GenerationProgress {
    fn eq(&self, other: &Self) -> bool {
        self.request_id == other.request_id && self.status == other.status
    }
}

// =============================================================================
// Progress Callback
// =============================================================================

/// Callback type for async generation progress updates
pub type ProgressCallback = Box<dyn Fn(&GenerationProgress) + Send + Sync>;

// =============================================================================
// Async Request
// =============================================================================

/// An async generation request
struct AsyncRequest {
    /// Request ID
    id: RequestId,
    /// Mood to generate
    mood: Mood,
    /// Evolution level
    evolution_level: EvolutionLevel,
    /// Generation options
    options: GenerationOptions,
    /// Progress callback
    callback: Option<ProgressCallback>,
    /// Current progress
    progress: GenerationProgress,
}

// =============================================================================
// Sprite Service
// =============================================================================

/// Service for handling sprite generation requests
///
/// The `SpriteService` provides a high-level API for sprite generation:
/// - Synchronous and asynchronous generation
/// - Result caching with configurable TTL
/// - Progress tracking for async requests
/// - Cancellation support
///
/// # Thread Safety
///
/// The service uses interior mutability (`RwLock`) for the cache and
/// request tracking, making it safe to share across threads.
///
/// # Example
///
/// ```
/// use conductor_core::avatar::api::{SpriteService, GenerationOptions};
/// use conductor_core::avatar::block::Mood;
/// use conductor_core::avatar::evolution::EvolutionLevel;
///
/// let mut service = SpriteService::new();
///
/// // Synchronous request
/// let sprite = service.request_sprite(Mood::Happy, EvolutionLevel::Mature);
///
/// // With options
/// let options = GenerationOptions::default()
///     .with_session("session123".to_string());
/// let sprite = service.request_sprite_with_options(
///     Mood::Thinking,
///     EvolutionLevel::Developing,
///     options
/// );
/// ```
pub struct SpriteService {
    /// The sprite generator
    generator: RuleBasedGenerator,
    /// Sprite cache
    cache: Arc<RwLock<SpriteCache>>,
    /// Active async requests
    requests: Arc<RwLock<HashMap<RequestId, AsyncRequest>>>,
    /// Default cache TTL (5 minutes)
    default_ttl: Duration,
}

impl SpriteService {
    /// Create a new sprite service with default settings
    #[must_use]
    pub fn new() -> Self {
        Self {
            generator: RuleBasedGenerator::new(),
            cache: Arc::new(RwLock::new(SpriteCache::with_default_budget())),
            requests: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Create a sprite service with a custom generator
    #[must_use]
    pub fn with_generator(generator: RuleBasedGenerator) -> Self {
        Self {
            generator,
            cache: Arc::new(RwLock::new(SpriteCache::with_default_budget())),
            requests: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: Duration::from_secs(300),
        }
    }

    /// Create a sprite service with a custom cache budget
    #[must_use]
    pub fn with_cache_budget(budget_bytes: usize) -> Self {
        Self {
            generator: RuleBasedGenerator::new(),
            cache: Arc::new(RwLock::new(SpriteCache::new(budget_bytes))),
            requests: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: Duration::from_secs(300),
        }
    }

    /// Set the default cache TTL
    pub fn set_default_ttl(&mut self, ttl: Duration) {
        self.default_ttl = ttl;
    }

    // =========================================================================
    // Synchronous Generation
    // =========================================================================

    /// Request a sprite synchronously
    ///
    /// This is the simplest way to get a sprite. It uses default options
    /// and returns immediately with the generated sprite.
    ///
    /// # Arguments
    ///
    /// * `mood` - The mood to express
    /// * `evolution_level` - The avatar's evolution level
    ///
    /// # Returns
    ///
    /// A `SpriteResponse` containing the generated sprite
    pub fn request_sprite(
        &mut self,
        mood: Mood,
        evolution_level: EvolutionLevel,
    ) -> SpriteResponse {
        self.request_sprite_with_options(mood, evolution_level, GenerationOptions::default())
    }

    /// Request a sprite with custom options
    ///
    /// # Arguments
    ///
    /// * `mood` - The mood to express
    /// * `evolution_level` - The avatar's evolution level
    /// * `options` - Generation options
    ///
    /// # Returns
    ///
    /// A `SpriteResponse` containing the generated sprite
    pub fn request_sprite_with_options(
        &mut self,
        mood: Mood,
        evolution_level: EvolutionLevel,
        options: GenerationOptions,
    ) -> SpriteResponse {
        let cache_key = self.make_cache_key(mood, evolution_level, &options);

        // Check cache first
        if options.use_cache {
            if let Some(sprite) = self.get_cached(&cache_key) {
                return sprite;
            }
        }

        // Generate sprite
        let sprite = self.generate_sprite(mood, evolution_level, &options);

        // Cache result if requested
        if options.cache_result {
            self.cache_sprite(&cache_key, &sprite, &options);
        }

        sprite
    }

    /// Handle a SpriteRequest message from a surface
    ///
    /// This method accepts the protocol `SpriteRequest` type and returns
    /// a `SpriteResponse`, making it suitable for message-based communication.
    ///
    /// # Arguments
    ///
    /// * `request` - The sprite request from a surface
    ///
    /// # Returns
    ///
    /// A `SpriteResponse` containing the generated sprite
    pub fn handle_request(&mut self, request: SpriteRequest) -> SpriteResponse {
        let mood = request.mood.unwrap_or_default();
        let evolution_level = evolution_from_percent(request.evolution.unwrap_or(0));

        let options = GenerationOptions {
            session_id: request.context.clone(),
            ..Default::default()
        };

        self.request_sprite_with_options(mood, evolution_level, options)
    }

    // =========================================================================
    // Asynchronous Generation
    // =========================================================================

    /// Request a sprite asynchronously with a progress callback
    ///
    /// This method queues the request and returns immediately with a request ID.
    /// The callback will be invoked with progress updates as generation proceeds.
    ///
    /// Note: In this implementation, generation is actually synchronous but
    /// the API is designed to support true async generation in the future.
    ///
    /// # Arguments
    ///
    /// * `mood` - The mood to express
    /// * `evolution_level` - The avatar's evolution level
    /// * `callback` - Function called with progress updates
    ///
    /// # Returns
    ///
    /// A `RequestId` that can be used to query status or cancel the request
    pub fn request_sprite_async(
        &mut self,
        mood: Mood,
        evolution_level: EvolutionLevel,
        callback: ProgressCallback,
    ) -> RequestId {
        self.request_sprite_async_with_options(
            mood,
            evolution_level,
            GenerationOptions::default(),
            callback,
        )
    }

    /// Request a sprite asynchronously with options and callback
    pub fn request_sprite_async_with_options(
        &mut self,
        mood: Mood,
        evolution_level: EvolutionLevel,
        options: GenerationOptions,
        callback: ProgressCallback,
    ) -> RequestId {
        let request_id = RequestId::new();
        let queue_position = self.requests.read().unwrap().len();

        let progress = GenerationProgress::new_pending(request_id, queue_position);

        // Notify initial status
        callback(&progress);

        let request = AsyncRequest {
            id: request_id,
            mood,
            evolution_level,
            options,
            callback: Some(callback),
            progress,
        };

        self.requests.write().unwrap().insert(request_id, request);

        // Process immediately (in real impl, this would be queued)
        self.process_async_request(request_id);

        request_id
    }

    /// Process an async request
    fn process_async_request(&mut self, request_id: RequestId) {
        let start = Instant::now();

        // Get request details
        let (mood, evolution_level, options) = {
            let requests = self.requests.read().unwrap();
            if let Some(req) = requests.get(&request_id) {
                (req.mood, req.evolution_level, req.options.clone())
            } else {
                return;
            }
        };

        // Update to generating status
        self.update_request_status(
            request_id,
            GenerationStatus::Generating {
                progress_percent: 0,
                eta: Some(Duration::from_millis(50)),
                stage: GenerationStage::Initializing,
            },
        );

        // Generate the sprite
        let sprite = self.generate_sprite_with_progress(mood, evolution_level, &options, |stage| {
            let progress = stage.typical_progress();
            let eta = if progress < 100 {
                Some(Duration::from_millis(((100 - progress) as u64) / 2))
            } else {
                None
            };
            self.update_request_status(
                request_id,
                GenerationStatus::Generating {
                    progress_percent: progress,
                    eta,
                    stage,
                },
            );
        });

        let elapsed = start.elapsed();

        // Cache if requested
        let cache_key = self.make_cache_key(mood, evolution_level, &options);
        if options.cache_result {
            self.cache_sprite(&cache_key, &sprite, &options);
        }

        // Mark complete
        self.update_request_status(
            request_id,
            GenerationStatus::Complete {
                response: sprite,
                elapsed,
            },
        );
    }

    /// Update the status of an async request and notify callback
    fn update_request_status(&self, request_id: RequestId, status: GenerationStatus) {
        let callback_to_invoke = {
            let mut requests = self.requests.write().unwrap();
            if let Some(request) = requests.get_mut(&request_id) {
                request.progress.update(status);

                // Clone what we need for callback
                let progress = request.progress.clone();
                request.callback.as_ref().map(|_| (progress, request_id))
            } else {
                None
            }
        };

        // Invoke callback outside of lock
        if let Some((progress, req_id)) = callback_to_invoke {
            let requests = self.requests.read().unwrap();
            if let Some(request) = requests.get(&req_id) {
                if let Some(ref callback) = request.callback {
                    callback(&progress);
                }
            }
        }
    }

    /// Get the status of an async generation request
    ///
    /// # Arguments
    ///
    /// * `request_id` - The ID returned from `request_sprite_async`
    ///
    /// # Returns
    ///
    /// The current status of the generation request
    pub fn get_generation_status(&self, request_id: RequestId) -> GenerationStatus {
        self.requests
            .read()
            .unwrap()
            .get(&request_id)
            .map(|r| r.progress.status.clone())
            .unwrap_or(GenerationStatus::NotFound)
    }

    /// Cancel an async generation request
    ///
    /// # Arguments
    ///
    /// * `request_id` - The ID of the request to cancel
    ///
    /// # Returns
    ///
    /// `true` if the request was found and cancelled, `false` if not found
    /// or already complete
    pub fn cancel_generation(&mut self, request_id: RequestId) -> bool {
        let mut requests = self.requests.write().unwrap();

        if let Some(request) = requests.get_mut(&request_id) {
            // Can only cancel pending or generating requests
            if !request.progress.status.is_terminal() {
                request.progress.update(GenerationStatus::Cancelled);

                // Notify callback
                if let Some(ref callback) = request.callback {
                    callback(&request.progress);
                }

                return true;
            }
        }

        false
    }

    // =========================================================================
    // Internal Generation
    // =========================================================================

    /// Generate a sprite with the given parameters
    fn generate_sprite(
        &self,
        mood: Mood,
        evolution_level: EvolutionLevel,
        options: &GenerationOptions,
    ) -> SpriteResponse {
        // Generate base sprite
        let mut sprite = if let Some(variant) = options.variant {
            self.generator
                .generate_variant(mood, evolution_level, variant)
        } else {
            self.generator.generate(mood, evolution_level)
        };

        // Add accessory if requested
        if let Some(accessory) = options.accessory {
            sprite = self
                .generator
                .compose_with_accessory(sprite, accessory, evolution_level);
        }

        // Set TTL
        let ttl = options.cache_ttl.unwrap_or(self.default_ttl);
        sprite = sprite.with_ttl(ttl);

        sprite
    }

    /// Generate a sprite with progress callback for stages
    fn generate_sprite_with_progress<F>(
        &self,
        mood: Mood,
        evolution_level: EvolutionLevel,
        options: &GenerationOptions,
        mut on_stage: F,
    ) -> SpriteResponse
    where
        F: FnMut(GenerationStage),
    {
        on_stage(GenerationStage::Initializing);

        // Generate base sprite
        on_stage(GenerationStage::GeneratingBase);
        let mut sprite = if let Some(variant) = options.variant {
            self.generator
                .generate_variant(mood, evolution_level, variant)
        } else {
            self.generator.generate(mood, evolution_level)
        };

        on_stage(GenerationStage::ApplyingMood);
        // Mood is already applied in generate()

        // Add accessory if requested
        on_stage(GenerationStage::AddingAccessory);
        if let Some(accessory) = options.accessory {
            sprite = self
                .generator
                .compose_with_accessory(sprite, accessory, evolution_level);
        }

        on_stage(GenerationStage::Finalizing);

        // Set TTL
        let ttl = options.cache_ttl.unwrap_or(self.default_ttl);
        sprite = sprite.with_ttl(ttl);

        sprite
    }

    // =========================================================================
    // Cache Management
    // =========================================================================

    /// Generate a cache key for the given parameters
    fn make_cache_key(
        &self,
        mood: Mood,
        evolution_level: EvolutionLevel,
        options: &GenerationOptions,
    ) -> String {
        let mut key = format!(
            "sprite_{:?}_{}_v{}",
            mood,
            evolution_level.as_u8(),
            options.variant.unwrap_or(0)
        );

        if let Some(ref accessory) = options.accessory {
            key.push_str(&format!("_{accessory:?}"));
        }

        if let Some(ref session_id) = options.session_id {
            key = format!("{session_id}:{key}");
        }

        key
    }

    /// Get a sprite from the cache
    fn get_cached(&self, key: &str) -> Option<SpriteResponse> {
        let mut cache = self.cache.write().unwrap();
        let sprite_data = cache.get(key)?;

        // Convert SpriteData to SpriteResponse
        Some(SpriteResponse::new(
            sprite_data.blocks.clone(),
            sprite_data.width,
            sprite_data.height,
        ))
    }

    /// Cache a sprite response
    fn cache_sprite(&self, key: &str, sprite: &SpriteResponse, _options: &GenerationOptions) {
        // Convert SpriteResponse to SpriteData
        let sprite_data = SpriteData::new(sprite.blocks.clone(), sprite.width(), sprite.height());

        if let Ok(data) = sprite_data {
            let mut cache = self.cache.write().unwrap();
            // Ignore cache insert errors (budget exceeded, etc.)
            let _ = cache.insert(key.to_string(), data, false);
        }
    }

    /// Clear all cached sprites for a session
    pub fn clear_session_cache(&mut self, session_id: &str) -> usize {
        let mut cache = self.cache.write().unwrap();
        cache.clear_session(session_id)
    }

    /// Get cache statistics
    #[must_use]
    pub fn cache_stats(&self) -> super::cache::CacheStats {
        self.cache.read().unwrap().stats()
    }

    /// Clear the entire cache
    pub fn clear_cache(&mut self) {
        self.cache.write().unwrap().clear();
    }

    // =========================================================================
    // Request Management
    // =========================================================================

    /// Get the number of pending async requests
    #[must_use]
    pub fn pending_request_count(&self) -> usize {
        self.requests
            .read()
            .unwrap()
            .values()
            .filter(|r| !r.progress.status.is_terminal())
            .count()
    }

    /// Clean up completed requests older than the given age
    pub fn cleanup_old_requests(&mut self, max_age: Duration) {
        let mut requests = self.requests.write().unwrap();
        let now = Instant::now();

        requests.retain(|_, req| {
            if req.progress.status.is_terminal() {
                now.duration_since(req.progress.created_at) < max_age
            } else {
                true // Keep active requests
            }
        });
    }
}

impl Default for SpriteService {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SpriteService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpriteService")
            .field("cache_stats", &self.cache_stats())
            .field("pending_requests", &self.pending_request_count())
            .finish()
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Convert evolution percentage (0-100) to EvolutionLevel
fn evolution_from_percent(percent: u8) -> EvolutionLevel {
    match percent {
        0..=19 => EvolutionLevel::Nascent,
        20..=39 => EvolutionLevel::Developing,
        40..=59 => EvolutionLevel::Mature,
        60..=79 => EvolutionLevel::Evolved,
        _ => EvolutionLevel::Transcendent,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    // =========================================================================
    // RequestId Tests
    // =========================================================================

    #[test]
    fn test_request_id_unique() {
        let id1 = RequestId::new();
        let id2 = RequestId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_request_id_display() {
        let id = RequestId(42);
        assert_eq!(format!("{}", id), "req_42");
    }

    // =========================================================================
    // GenerationOptions Tests
    // =========================================================================

    #[test]
    fn test_generation_options_default() {
        let options = GenerationOptions::default();
        assert!(options.use_cache);
        assert!(options.cache_result);
        assert!(options.variant.is_none());
        assert!(options.accessory.is_none());
        assert!(options.session_id.is_none());
    }

    #[test]
    fn test_generation_options_no_cache() {
        let options = GenerationOptions::no_cache();
        assert!(!options.use_cache);
        assert!(!options.cache_result);
    }

    #[test]
    fn test_generation_options_builder() {
        let options = GenerationOptions::default()
            .with_variant(2)
            .with_accessory(Accessory::Glasses)
            .with_session("test_session")
            .with_ttl(Duration::from_secs(600));

        assert_eq!(options.variant, Some(2));
        assert_eq!(options.accessory, Some(Accessory::Glasses));
        assert_eq!(options.session_id, Some("test_session".to_string()));
        assert_eq!(options.cache_ttl, Some(Duration::from_secs(600)));
    }

    // =========================================================================
    // GenerationStatus Tests
    // =========================================================================

    #[test]
    fn test_generation_status_is_terminal() {
        assert!(!GenerationStatus::Pending { queue_position: 0 }.is_terminal());
        assert!(!GenerationStatus::Generating {
            progress_percent: 50,
            eta: None,
            stage: GenerationStage::GeneratingBase
        }
        .is_terminal());

        assert!(GenerationStatus::Complete {
            response: SpriteResponse::new(vec![], 0, 0),
            elapsed: Duration::ZERO
        }
        .is_terminal());
        assert!(GenerationStatus::Failed {
            error: "test".to_string(),
            retryable: false
        }
        .is_terminal());
        assert!(GenerationStatus::Cancelled.is_terminal());
        assert!(GenerationStatus::NotFound.is_terminal());
    }

    #[test]
    fn test_generation_status_is_success() {
        assert!(GenerationStatus::Complete {
            response: SpriteResponse::new(vec![], 0, 0),
            elapsed: Duration::ZERO
        }
        .is_success());

        assert!(!GenerationStatus::Failed {
            error: "test".to_string(),
            retryable: false
        }
        .is_success());
    }

    #[test]
    fn test_generation_status_progress() {
        assert_eq!(
            GenerationStatus::Pending { queue_position: 0 }.progress(),
            Some(0)
        );
        assert_eq!(
            GenerationStatus::Generating {
                progress_percent: 50,
                eta: None,
                stage: GenerationStage::ApplyingMood
            }
            .progress(),
            Some(50)
        );
        assert_eq!(
            GenerationStatus::Complete {
                response: SpriteResponse::new(vec![], 0, 0),
                elapsed: Duration::ZERO
            }
            .progress(),
            Some(100)
        );
        assert_eq!(GenerationStatus::Cancelled.progress(), None);
    }

    // =========================================================================
    // GenerationStage Tests
    // =========================================================================

    #[test]
    fn test_generation_stage_progress() {
        assert!(
            GenerationStage::Initializing.typical_progress()
                < GenerationStage::GeneratingBase.typical_progress()
        );
        assert!(
            GenerationStage::GeneratingBase.typical_progress()
                < GenerationStage::ApplyingMood.typical_progress()
        );
        assert!(
            GenerationStage::ApplyingMood.typical_progress()
                < GenerationStage::AddingAccessory.typical_progress()
        );
        assert!(
            GenerationStage::AddingAccessory.typical_progress()
                < GenerationStage::Finalizing.typical_progress()
        );
    }

    // =========================================================================
    // SpriteService Sync Tests
    // =========================================================================

    #[test]
    fn test_service_request_sprite() {
        let mut service = SpriteService::new();
        let sprite = service.request_sprite(Mood::Happy, EvolutionLevel::Mature);

        assert!(!sprite.blocks.is_empty());
        assert!(sprite.width() > 0);
        assert!(sprite.height() > 0);
    }

    #[test]
    fn test_service_request_sprite_with_options() {
        let mut service = SpriteService::new();
        let options = GenerationOptions::default()
            .with_variant(1)
            .with_session("test");

        let sprite = service.request_sprite_with_options(
            Mood::Thinking,
            EvolutionLevel::Developing,
            options,
        );

        assert!(!sprite.blocks.is_empty());
    }

    #[test]
    fn test_service_request_sprite_with_accessory() {
        let mut service = SpriteService::new();
        let options = GenerationOptions::default().with_accessory(Accessory::Glasses);

        let sprite = service.request_sprite_with_options(
            Mood::Happy,
            EvolutionLevel::Mature, // Glasses available at Developing+
            options,
        );

        assert!(!sprite.blocks.is_empty());
    }

    #[test]
    fn test_service_caching_prevents_duplicate_generation() {
        let mut service = SpriteService::new();

        // First request - generates and caches
        let sprite1 = service.request_sprite(Mood::Happy, EvolutionLevel::Nascent);

        // Second request - should use cache
        let sprite2 = service.request_sprite(Mood::Happy, EvolutionLevel::Nascent);

        // Both should have same dimensions
        assert_eq!(sprite1.width(), sprite2.width());
        assert_eq!(sprite1.height(), sprite2.height());

        // Cache should have an entry
        let stats = service.cache_stats();
        assert!(stats.total_entries > 0);
    }

    #[test]
    fn test_service_no_cache_option() {
        let mut service = SpriteService::new();

        let options = GenerationOptions::no_cache();
        let _ = service.request_sprite_with_options(
            Mood::Happy,
            EvolutionLevel::Nascent,
            options.clone(),
        );

        // With no_cache, nothing should be cached
        let stats = service.cache_stats();
        // The cache might have entries from other tests, but this specific
        // no_cache request shouldn't add one
        let initial_count = stats.total_entries;

        let _ = service.request_sprite_with_options(
            Mood::Thinking, // Different mood
            EvolutionLevel::Nascent,
            options,
        );

        let stats_after = service.cache_stats();
        assert_eq!(initial_count, stats_after.total_entries);
    }

    // =========================================================================
    // SpriteService Async Tests
    // =========================================================================

    #[test]
    fn test_service_async_generation_with_callback() {
        let mut service = SpriteService::new();
        let callback_count = Arc::new(AtomicUsize::new(0));
        let callback_count_clone = callback_count.clone();

        let last_status = Arc::new(RwLock::new(None::<GenerationStatus>));
        let last_status_clone = last_status.clone();

        let callback = Box::new(move |progress: &GenerationProgress| {
            callback_count_clone.fetch_add(1, AtomicOrdering::SeqCst);
            *last_status_clone.write().unwrap() = Some(progress.status.clone());
        });

        let request_id =
            service.request_sprite_async(Mood::Excited, EvolutionLevel::Mature, callback);

        // Callback should have been called multiple times
        assert!(callback_count.load(AtomicOrdering::SeqCst) > 1);

        // Final status should be Complete
        let status = last_status.read().unwrap().clone();
        assert!(status.is_some());
        assert!(status.unwrap().is_success());

        // get_generation_status should also return Complete
        let status = service.get_generation_status(request_id);
        assert!(status.is_success());
    }

    #[test]
    fn test_service_cancellation() {
        let mut service = SpriteService::new();

        // Since our impl is synchronous, we need to test cancellation
        // on a completed request (which should fail to cancel)
        let callback = Box::new(|_: &GenerationProgress| {});
        let request_id =
            service.request_sprite_async(Mood::Happy, EvolutionLevel::Nascent, callback);

        // Request is already complete, so cancel should return false
        let cancelled = service.cancel_generation(request_id);
        assert!(!cancelled);

        // Status should still be Complete
        let status = service.get_generation_status(request_id);
        assert!(status.is_success());
    }

    #[test]
    fn test_service_get_status_not_found() {
        let service = SpriteService::new();
        let status = service.get_generation_status(RequestId(99999));
        assert_eq!(status, GenerationStatus::NotFound);
    }

    // =========================================================================
    // SpriteService Handle Request Tests
    // =========================================================================

    #[test]
    fn test_service_handle_request() {
        let mut service = SpriteService::new();
        let request = SpriteRequest::new("idle")
            .with_mood(Mood::Happy)
            .with_evolution(50); // Should map to Mature

        let response = service.handle_request(request);
        assert!(!response.blocks.is_empty());
    }

    #[test]
    fn test_service_handle_request_defaults() {
        let mut service = SpriteService::new();
        let request = SpriteRequest::new("idle");

        let response = service.handle_request(request);
        assert!(!response.blocks.is_empty());
    }

    // =========================================================================
    // Cache Management Tests
    // =========================================================================

    #[test]
    fn test_service_clear_session_cache() {
        let mut service = SpriteService::new();

        // Create sprites for different sessions
        let options1 = GenerationOptions::default().with_session("session1");
        let options2 = GenerationOptions::default().with_session("session2");

        service.request_sprite_with_options(Mood::Happy, EvolutionLevel::Nascent, options1);
        service.request_sprite_with_options(Mood::Happy, EvolutionLevel::Nascent, options2);

        let stats_before = service.cache_stats();
        let count_before = stats_before.total_entries;

        // Clear only session1
        let cleared = service.clear_session_cache("session1");
        assert!(cleared > 0);

        let stats_after = service.cache_stats();
        assert!(stats_after.total_entries < count_before);
    }

    #[test]
    fn test_service_clear_cache() {
        let mut service = SpriteService::new();

        // Generate some sprites
        service.request_sprite(Mood::Happy, EvolutionLevel::Nascent);
        service.request_sprite(Mood::Thinking, EvolutionLevel::Developing);

        assert!(service.cache_stats().total_entries > 0);

        service.clear_cache();

        assert_eq!(service.cache_stats().total_entries, 0);
    }

    // =========================================================================
    // Request Management Tests
    // =========================================================================

    #[test]
    fn test_service_pending_request_count() {
        let mut service = SpriteService::new();

        // After async request completes, pending count should be 0
        let callback = Box::new(|_: &GenerationProgress| {});
        service.request_sprite_async(Mood::Happy, EvolutionLevel::Nascent, callback);

        // Request completes immediately in this impl, so count should be 0
        assert_eq!(service.pending_request_count(), 0);
    }

    #[test]
    fn test_service_cleanup_old_requests() {
        let mut service = SpriteService::new();

        // Create a request
        let callback = Box::new(|_: &GenerationProgress| {});
        service.request_sprite_async(Mood::Happy, EvolutionLevel::Nascent, callback);

        let requests_before = service.requests.read().unwrap().len();
        assert!(requests_before > 0);

        // Cleanup with 0 max_age should remove completed requests
        service.cleanup_old_requests(Duration::ZERO);

        let requests_after = service.requests.read().unwrap().len();
        assert!(requests_after < requests_before);
    }

    // =========================================================================
    // Helper Function Tests
    // =========================================================================

    #[test]
    fn test_evolution_from_percent() {
        assert_eq!(evolution_from_percent(0), EvolutionLevel::Nascent);
        assert_eq!(evolution_from_percent(10), EvolutionLevel::Nascent);
        assert_eq!(evolution_from_percent(19), EvolutionLevel::Nascent);
        assert_eq!(evolution_from_percent(20), EvolutionLevel::Developing);
        assert_eq!(evolution_from_percent(40), EvolutionLevel::Mature);
        assert_eq!(evolution_from_percent(60), EvolutionLevel::Evolved);
        assert_eq!(evolution_from_percent(80), EvolutionLevel::Transcendent);
        assert_eq!(evolution_from_percent(100), EvolutionLevel::Transcendent);
    }

    // =========================================================================
    // Service Configuration Tests
    // =========================================================================

    #[test]
    fn test_service_with_custom_generator() {
        let generator = RuleBasedGenerator::new().with_dimensions(16, 16);
        let mut service = SpriteService::with_generator(generator);

        let sprite = service.request_sprite(Mood::Happy, EvolutionLevel::Nascent);
        assert_eq!(sprite.width(), 16);
        assert_eq!(sprite.height(), 16);
    }

    #[test]
    fn test_service_with_cache_budget() {
        let service = SpriteService::with_cache_budget(1024 * 1024);
        let stats = service.cache_stats();
        assert_eq!(stats.memory_budget_bytes, 1024 * 1024);
    }

    #[test]
    fn test_service_set_default_ttl() {
        let mut service = SpriteService::new();
        service.set_default_ttl(Duration::from_secs(600));

        let sprite = service.request_sprite(Mood::Happy, EvolutionLevel::Nascent);
        assert_eq!(sprite.ttl, Some(Duration::from_secs(600)));
    }

    // =========================================================================
    // Debug Trait Tests
    // =========================================================================

    #[test]
    fn test_service_debug() {
        let service = SpriteService::new();
        let debug_str = format!("{:?}", service);
        assert!(debug_str.contains("SpriteService"));
        assert!(debug_str.contains("cache_stats"));
        assert!(debug_str.contains("pending_requests"));
    }
}
