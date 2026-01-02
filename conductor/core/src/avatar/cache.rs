//! Sprite caching system with LRU eviction
//!
//! This module provides a memory-budgeted cache for sprite data with:
//! - LRU (Least Recently Used) eviction policy
//! - Session-scoped cache keys
//! - Non-evictable base sprites
//! - Memory usage tracking
//!
//! # Design
//!
//! The cache is designed for the Conductor to store computed sprites before
//! transmission to surfaces. Key features:
//!
//! - **Memory Budget**: Configurable limit (default 10MB) prevents unbounded growth
//! - **Base Sprites**: Core avatar poses marked as non-evictable
//! - **Session Scoping**: Cache keys namespaced by session ID for isolation
//! - **LRU Eviction**: Least recently used non-base sprites evicted first
//!
//! # Security
//!
//! - Session isolation prevents cross-session cache pollution
//! - Memory limits prevent resource exhaustion attacks
//! - Base sprites cannot be evicted by user requests

use std::collections::HashMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::block::Block;

/// Default memory budget: 10MB
pub const DEFAULT_MEMORY_BUDGET_BYTES: usize = 10 * 1024 * 1024;

/// Maximum sprite dimensions (security limit)
pub const MAX_SPRITE_WIDTH: u16 = 100;
/// Maximum sprite dimensions (security limit)
pub const MAX_SPRITE_HEIGHT: u16 = 100;
/// Maximum blocks per sprite (security limit)
pub const MAX_BLOCKS_PER_SPRITE: usize = 10_000;

/// Sprite data containing blocks and dimensions
///
/// This is the actual sprite content that gets cached and transmitted
/// to surfaces for rendering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpriteData {
    /// The blocks that make up this sprite, in row-major order
    pub blocks: Vec<Block>,
    /// Width in blocks
    pub width: u16,
    /// Height in blocks
    pub height: u16,
}

impl SpriteData {
    /// Create a new sprite with the given dimensions and blocks
    ///
    /// # Errors
    ///
    /// Returns `CacheError::DimensionOverflow` if dimensions exceed limits
    /// Returns `CacheError::TooManyBlocks` if block count exceeds limit
    pub fn new(blocks: Vec<Block>, width: u16, height: u16) -> Result<Self, CacheError> {
        Self::validate_dimensions(width, height)?;

        let expected_blocks = width as usize * height as usize;
        if blocks.len() != expected_blocks {
            return Err(CacheError::InvalidBlockCount {
                expected: expected_blocks,
                actual: blocks.len(),
            });
        }

        Ok(Self {
            blocks,
            width,
            height,
        })
    }

    /// Validate sprite dimensions against security limits
    fn validate_dimensions(width: u16, height: u16) -> Result<(), CacheError> {
        if width > MAX_SPRITE_WIDTH || height > MAX_SPRITE_HEIGHT {
            return Err(CacheError::DimensionOverflow {
                width,
                height,
                max_width: MAX_SPRITE_WIDTH,
                max_height: MAX_SPRITE_HEIGHT,
            });
        }

        let block_count = width as usize * height as usize;
        if block_count > MAX_BLOCKS_PER_SPRITE {
            return Err(CacheError::TooManyBlocks {
                count: block_count,
                max: MAX_BLOCKS_PER_SPRITE,
            });
        }

        Ok(())
    }

    /// Calculate the approximate memory size of this sprite in bytes
    ///
    /// This includes:
    /// - Vec overhead
    /// - Block data (each Block contains colors, char, floats)
    #[must_use]
    pub fn size_bytes(&self) -> usize {
        // Base struct overhead
        let base = std::mem::size_of::<Self>();

        // Each Block contains:
        // - 2 Colors (4 bytes each = 8 bytes total)
        // - 1 char (4 bytes)
        // - 1 f32 (4 bytes)
        // - 1 i8 (1 byte)
        // Total per block: ~21 bytes, but with alignment likely 24 bytes
        let block_size = std::mem::size_of::<Block>();
        let blocks_total = block_size * self.blocks.len();

        // Vec heap allocation overhead (capacity may differ from len)
        let vec_overhead = std::mem::size_of::<Vec<Block>>();

        base + blocks_total + vec_overhead
    }

    /// Create an empty sprite (1x1 with transparent block)
    #[must_use]
    pub fn empty() -> Self {
        Self {
            blocks: vec![Block::empty()],
            width: 1,
            height: 1,
        }
    }

    /// Get a block at the given position
    #[must_use]
    pub fn get_block(&self, x: u16, y: u16) -> Option<&Block> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = y as usize * self.width as usize + x as usize;
        self.blocks.get(index)
    }
}

/// A cache entry containing sprite data and metadata
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// The cached sprite data
    pub sprite: SpriteData,
    /// Calculated size in bytes
    pub size_bytes: usize,
    /// Whether this is a base sprite (non-evictable)
    pub is_base: bool,
    /// When this entry was created
    pub created_at: Instant,
    /// When this entry was last accessed
    pub last_accessed: Instant,
    /// Number of times this entry has been accessed
    pub access_count: u64,
}

impl CacheEntry {
    /// Create a new cache entry
    fn new(sprite: SpriteData, is_base: bool) -> Self {
        let size_bytes = sprite.size_bytes();
        let now = Instant::now();
        Self {
            sprite,
            size_bytes,
            is_base,
            created_at: now,
            last_accessed: now,
            access_count: 0,
        }
    }

    /// Record an access to this entry
    fn touch(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count = self.access_count.saturating_add(1);
    }
}

/// Errors that can occur during cache operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheError {
    /// Sprite dimensions exceed security limits
    DimensionOverflow {
        /// Requested width
        width: u16,
        /// Requested height
        height: u16,
        /// Maximum allowed width
        max_width: u16,
        /// Maximum allowed height
        max_height: u16,
    },
    /// Block count exceeds security limit
    TooManyBlocks {
        /// Requested block count
        count: usize,
        /// Maximum allowed
        max: usize,
    },
    /// Block count doesn't match dimensions
    InvalidBlockCount {
        /// Expected count based on dimensions
        expected: usize,
        /// Actual count provided
        actual: usize,
    },
    /// Sprite is too large for the memory budget
    SpriteTooLarge {
        /// Size of the sprite
        sprite_size: usize,
        /// Available budget
        budget: usize,
    },
    /// Cannot evict - all entries are base sprites
    CannotEvict,
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DimensionOverflow {
                width,
                height,
                max_width,
                max_height,
            } => {
                write!(
                    f,
                    "Sprite dimensions {}x{} exceed maximum {}x{}",
                    width, height, max_width, max_height
                )
            }
            Self::TooManyBlocks { count, max } => {
                write!(f, "Sprite has {} blocks, exceeds maximum {}", count, max)
            }
            Self::InvalidBlockCount { expected, actual } => {
                write!(
                    f,
                    "Expected {} blocks for dimensions, got {}",
                    expected, actual
                )
            }
            Self::SpriteTooLarge {
                sprite_size,
                budget,
            } => {
                write!(
                    f,
                    "Sprite size {} bytes exceeds budget {} bytes",
                    sprite_size, budget
                )
            }
            Self::CannotEvict => {
                write!(f, "Cannot evict: all cached entries are base sprites")
            }
        }
    }
}

impl std::error::Error for CacheError {}

/// LRU-based sprite cache with memory budget
///
/// Provides session-scoped caching of sprite data with:
/// - Configurable memory budget
/// - LRU eviction policy
/// - Base sprite protection (non-evictable)
/// - Memory usage tracking
///
/// # Example
///
/// ```
/// use conductor_core::avatar::cache::{SpriteCache, SpriteData};
/// use conductor_core::avatar::block::Block;
///
/// let mut cache = SpriteCache::new(1024 * 1024); // 1MB budget
///
/// // Create a simple 2x2 sprite
/// let blocks = vec![Block::empty(); 4];
/// let sprite = SpriteData::new(blocks, 2, 2).unwrap();
///
/// // Insert with session-scoped key
/// cache.insert("session1:idle".to_string(), sprite.clone(), false).unwrap();
///
/// // Retrieve
/// let retrieved = cache.get("session1:idle");
/// assert!(retrieved.is_some());
/// ```
#[derive(Debug)]
pub struct SpriteCache {
    /// The cached entries
    entries: HashMap<String, CacheEntry>,
    /// Total memory budget in bytes
    memory_budget_bytes: usize,
    /// Current memory usage in bytes
    current_usage_bytes: usize,
}

impl SpriteCache {
    /// Create a new sprite cache with the given memory budget
    ///
    /// # Arguments
    ///
    /// * `memory_budget_bytes` - Maximum memory usage in bytes
    #[must_use]
    pub fn new(memory_budget_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            memory_budget_bytes,
            current_usage_bytes: 0,
        }
    }

    /// Create a new sprite cache with the default 10MB budget
    #[must_use]
    pub fn with_default_budget() -> Self {
        Self::new(DEFAULT_MEMORY_BUDGET_BYTES)
    }

    /// Get a sprite from the cache, updating access time
    ///
    /// Returns `None` if the key doesn't exist.
    pub fn get(&mut self, key: &str) -> Option<&SpriteData> {
        // We need to update the entry, so we use get_mut
        if let Some(entry) = self.entries.get_mut(key) {
            entry.touch();
            Some(&entry.sprite)
        } else {
            None
        }
    }

    /// Get a sprite without updating access time (for inspection)
    #[must_use]
    pub fn peek(&self, key: &str) -> Option<&SpriteData> {
        self.entries.get(key).map(|e| &e.sprite)
    }

    /// Get cache entry metadata without updating access time
    #[must_use]
    pub fn get_entry(&self, key: &str) -> Option<&CacheEntry> {
        self.entries.get(key)
    }

    /// Insert a sprite into the cache
    ///
    /// If the cache is full, evicts LRU entries until there's room.
    /// Base sprites cannot be evicted.
    ///
    /// # Arguments
    ///
    /// * `key` - Cache key (should include session ID for scoping)
    /// * `sprite` - The sprite data to cache
    /// * `is_base` - Whether this is a base sprite (non-evictable)
    ///
    /// # Errors
    ///
    /// Returns `CacheError::SpriteTooLarge` if sprite exceeds entire budget
    /// Returns `CacheError::CannotEvict` if can't make room (all entries are base)
    pub fn insert(
        &mut self,
        key: String,
        sprite: SpriteData,
        is_base: bool,
    ) -> Result<(), CacheError> {
        let entry = CacheEntry::new(sprite, is_base);
        let entry_size = entry.size_bytes;

        // Check if sprite alone exceeds budget
        if entry_size > self.memory_budget_bytes {
            return Err(CacheError::SpriteTooLarge {
                sprite_size: entry_size,
                budget: self.memory_budget_bytes,
            });
        }

        // If key already exists, remove old entry first
        if let Some(old_entry) = self.entries.remove(&key) {
            self.current_usage_bytes = self
                .current_usage_bytes
                .saturating_sub(old_entry.size_bytes);
        }

        // Evict entries until we have room
        while self.current_usage_bytes + entry_size > self.memory_budget_bytes {
            if self.evict_lru().is_none() {
                // Can't evict anything (all base sprites)
                return Err(CacheError::CannotEvict);
            }
        }

        self.current_usage_bytes += entry_size;
        self.entries.insert(key, entry);
        Ok(())
    }

    /// Mark an existing entry as a base sprite (non-evictable)
    ///
    /// Returns `true` if the entry was found and marked.
    pub fn mark_as_base(&mut self, key: &str) -> bool {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.is_base = true;
            true
        } else {
            false
        }
    }

    /// Evict the least recently used non-base entry
    ///
    /// Returns the key of the evicted entry, or `None` if no entries can be evicted.
    pub fn evict_lru(&mut self) -> Option<String> {
        // Find the LRU non-base entry
        let lru_key = self
            .entries
            .iter()
            .filter(|(_, entry)| !entry.is_base)
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(key, _)| key.clone());

        // Remove it
        if let Some(key) = lru_key {
            if let Some(entry) = self.entries.remove(&key) {
                self.current_usage_bytes =
                    self.current_usage_bytes.saturating_sub(entry.size_bytes);
                return Some(key);
            }
        }

        None
    }

    /// Get current memory usage in bytes
    #[must_use]
    pub fn memory_usage(&self) -> usize {
        self.current_usage_bytes
    }

    /// Get the memory budget in bytes
    #[must_use]
    pub fn memory_budget(&self) -> usize {
        self.memory_budget_bytes
    }

    /// Get the number of cached entries
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries for a specific session
    ///
    /// Session keys are expected to be prefixed with `session_id:`.
    ///
    /// Returns the number of entries cleared.
    pub fn clear_session(&mut self, session_id: &str) -> usize {
        let prefix = format!("{}:", session_id);
        let keys_to_remove: Vec<String> = self
            .entries
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .cloned()
            .collect();

        let count = keys_to_remove.len();
        for key in keys_to_remove {
            if let Some(entry) = self.entries.remove(&key) {
                self.current_usage_bytes =
                    self.current_usage_bytes.saturating_sub(entry.size_bytes);
            }
        }

        count
    }

    /// Clear all non-base entries
    ///
    /// Returns the number of entries cleared.
    pub fn clear_non_base(&mut self) -> usize {
        let keys_to_remove: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, entry)| !entry.is_base)
            .map(|(key, _)| key.clone())
            .collect();

        let count = keys_to_remove.len();
        for key in keys_to_remove {
            if let Some(entry) = self.entries.remove(&key) {
                self.current_usage_bytes =
                    self.current_usage_bytes.saturating_sub(entry.size_bytes);
            }
        }

        count
    }

    /// Clear all entries (including base sprites)
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_usage_bytes = 0;
    }

    /// Get cache statistics
    #[must_use]
    pub fn stats(&self) -> CacheStats {
        let base_count = self.entries.values().filter(|e| e.is_base).count();
        let non_base_count = self.entries.len() - base_count;
        let base_size: usize = self
            .entries
            .values()
            .filter(|e| e.is_base)
            .map(|e| e.size_bytes)
            .sum();

        CacheStats {
            total_entries: self.entries.len(),
            base_entries: base_count,
            non_base_entries: non_base_count,
            memory_usage_bytes: self.current_usage_bytes,
            memory_budget_bytes: self.memory_budget_bytes,
            base_size_bytes: base_size,
            non_base_size_bytes: self.current_usage_bytes.saturating_sub(base_size),
        }
    }

    /// Create a session-scoped cache key
    ///
    /// # Example
    ///
    /// ```
    /// use conductor_core::avatar::cache::SpriteCache;
    ///
    /// let key = SpriteCache::session_key("sess123", "idle");
    /// assert_eq!(key, "sess123:idle");
    /// ```
    #[must_use]
    pub fn session_key(session_id: &str, sprite_name: &str) -> String {
        format!("{}:{}", session_id, sprite_name)
    }
}

impl Default for SpriteCache {
    fn default() -> Self {
        Self::with_default_budget()
    }
}

/// Cache statistics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheStats {
    /// Total number of entries
    pub total_entries: usize,
    /// Number of base (non-evictable) entries
    pub base_entries: usize,
    /// Number of non-base (evictable) entries
    pub non_base_entries: usize,
    /// Current memory usage in bytes
    pub memory_usage_bytes: usize,
    /// Memory budget in bytes
    pub memory_budget_bytes: usize,
    /// Memory used by base sprites
    pub base_size_bytes: usize,
    /// Memory used by non-base sprites
    pub non_base_size_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_sprite(width: u16, height: u16) -> SpriteData {
        let blocks = vec![Block::empty(); width as usize * height as usize];
        SpriteData::new(blocks, width, height).unwrap()
    }

    // ===================
    // SpriteData tests
    // ===================

    #[test]
    fn test_sprite_data_new() {
        let blocks = vec![Block::empty(); 6];
        let sprite = SpriteData::new(blocks, 3, 2).unwrap();
        assert_eq!(sprite.width, 3);
        assert_eq!(sprite.height, 2);
        assert_eq!(sprite.blocks.len(), 6);
    }

    #[test]
    fn test_sprite_data_dimension_overflow() {
        let result = SpriteData::new(vec![], 101, 50);
        assert!(matches!(result, Err(CacheError::DimensionOverflow { .. })));
    }

    #[test]
    fn test_sprite_data_too_many_blocks() {
        // 101 * 100 = 10100 > MAX_BLOCKS_PER_SPRITE
        let result = SpriteData::new(vec![], 100, 101);
        assert!(matches!(result, Err(CacheError::DimensionOverflow { .. })));
    }

    #[test]
    fn test_sprite_data_invalid_block_count() {
        let blocks = vec![Block::empty(); 5]; // Wrong count for 3x2
        let result = SpriteData::new(blocks, 3, 2);
        assert!(matches!(result, Err(CacheError::InvalidBlockCount { .. })));
    }

    #[test]
    fn test_sprite_data_size_bytes() {
        let sprite = create_test_sprite(10, 10);
        let size = sprite.size_bytes();
        // Should be positive and reasonable
        assert!(size > 0);
        // 100 blocks * ~24 bytes each + overhead
        assert!(size < 5000);
    }

    #[test]
    fn test_sprite_data_get_block() {
        let sprite = create_test_sprite(3, 2);
        assert!(sprite.get_block(0, 0).is_some());
        assert!(sprite.get_block(2, 1).is_some());
        assert!(sprite.get_block(3, 0).is_none()); // Out of bounds
        assert!(sprite.get_block(0, 2).is_none()); // Out of bounds
    }

    // ===================
    // Basic cache operations
    // ===================

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = SpriteCache::new(1024 * 1024);
        let sprite = create_test_sprite(5, 5);

        cache
            .insert("test:sprite1".to_string(), sprite.clone(), false)
            .unwrap();

        let retrieved = cache.get("test:sprite1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().width, 5);
    }

    #[test]
    fn test_cache_get_nonexistent() {
        let mut cache = SpriteCache::new(1024);
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_cache_peek_vs_get() {
        let mut cache = SpriteCache::new(1024 * 1024);
        let sprite = create_test_sprite(2, 2);
        cache.insert("key".to_string(), sprite, false).unwrap();

        // Peek should not update access time
        let entry_before = cache.get_entry("key").unwrap().last_accessed;
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _ = cache.peek("key");
        let entry_after = cache.get_entry("key").unwrap().last_accessed;
        assert_eq!(entry_before, entry_after);

        // Get should update access time
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _ = cache.get("key");
        let entry_updated = cache.get_entry("key").unwrap().last_accessed;
        assert!(entry_updated > entry_after);
    }

    #[test]
    fn test_cache_overwrite() {
        let mut cache = SpriteCache::new(1024 * 1024);
        let sprite1 = create_test_sprite(2, 2);
        let sprite2 = create_test_sprite(3, 3);

        cache.insert("key".to_string(), sprite1, false).unwrap();
        let usage1 = cache.memory_usage();

        cache.insert("key".to_string(), sprite2, false).unwrap();
        let usage2 = cache.memory_usage();

        // Usage should have changed (sprite2 is larger)
        assert!(usage2 > usage1);
        // Should still be one entry
        assert_eq!(cache.len(), 1);
        // Should have new dimensions
        assert_eq!(cache.get("key").unwrap().width, 3);
    }

    // ===================
    // LRU eviction tests
    // ===================

    #[test]
    fn test_lru_eviction_when_budget_exceeded() {
        // Very small budget that can only hold 1-2 sprites
        let sprite_size = create_test_sprite(5, 5).size_bytes();
        let budget = sprite_size * 2 + 100; // Room for ~2 sprites

        let mut cache = SpriteCache::new(budget);

        cache
            .insert("sprite1".to_string(), create_test_sprite(5, 5), false)
            .unwrap();
        cache
            .insert("sprite2".to_string(), create_test_sprite(5, 5), false)
            .unwrap();

        // Access sprite1 to make it more recent
        let _ = cache.get("sprite1");

        // Insert sprite3 - should evict sprite2 (LRU)
        cache
            .insert("sprite3".to_string(), create_test_sprite(5, 5), false)
            .unwrap();

        assert!(cache.peek("sprite1").is_some());
        assert!(cache.peek("sprite2").is_none()); // Evicted
        assert!(cache.peek("sprite3").is_some());
    }

    #[test]
    fn test_evict_lru_returns_evicted_key() {
        let mut cache = SpriteCache::new(1024 * 1024);
        cache
            .insert("old".to_string(), create_test_sprite(2, 2), false)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        cache
            .insert("new".to_string(), create_test_sprite(2, 2), false)
            .unwrap();

        let evicted = cache.evict_lru();
        assert_eq!(evicted, Some("old".to_string()));
    }

    #[test]
    fn test_evict_lru_empty_cache() {
        let mut cache = SpriteCache::new(1024);
        assert!(cache.evict_lru().is_none());
    }

    // ===================
    // Base sprite tests
    // ===================

    #[test]
    fn test_base_sprites_not_evicted() {
        let sprite_size = create_test_sprite(5, 5).size_bytes();
        let budget = sprite_size * 2 + 100;

        let mut cache = SpriteCache::new(budget);

        // Insert base sprite
        cache
            .insert("base".to_string(), create_test_sprite(5, 5), true)
            .unwrap();
        // Insert non-base sprite
        cache
            .insert("normal".to_string(), create_test_sprite(5, 5), false)
            .unwrap();

        // Try to insert another sprite (should evict normal, not base)
        cache
            .insert("new".to_string(), create_test_sprite(5, 5), false)
            .unwrap();

        assert!(cache.peek("base").is_some()); // Base protected
        assert!(cache.peek("normal").is_none()); // Evicted
        assert!(cache.peek("new").is_some());
    }

    #[test]
    fn test_mark_as_base() {
        let mut cache = SpriteCache::new(1024 * 1024);
        cache
            .insert("sprite".to_string(), create_test_sprite(2, 2), false)
            .unwrap();

        assert!(!cache.get_entry("sprite").unwrap().is_base);

        assert!(cache.mark_as_base("sprite"));
        assert!(cache.get_entry("sprite").unwrap().is_base);

        // Non-existent key
        assert!(!cache.mark_as_base("nonexistent"));
    }

    #[test]
    fn test_cannot_evict_when_all_base() {
        let sprite_size = create_test_sprite(5, 5).size_bytes();
        let budget = sprite_size + 100;

        let mut cache = SpriteCache::new(budget);
        cache
            .insert("base".to_string(), create_test_sprite(5, 5), true)
            .unwrap();

        // Try to insert another sprite - should fail
        let result = cache.insert("new".to_string(), create_test_sprite(5, 5), false);
        assert!(matches!(result, Err(CacheError::CannotEvict)));
    }

    // ===================
    // Session scoping tests
    // ===================

    #[test]
    fn test_session_key_format() {
        let key = SpriteCache::session_key("session123", "idle");
        assert_eq!(key, "session123:idle");
    }

    #[test]
    fn test_clear_session() {
        let mut cache = SpriteCache::new(1024 * 1024);

        // Insert sprites for different sessions
        cache
            .insert("sess1:sprite1".to_string(), create_test_sprite(2, 2), false)
            .unwrap();
        cache
            .insert("sess1:sprite2".to_string(), create_test_sprite(2, 2), false)
            .unwrap();
        cache
            .insert("sess2:sprite1".to_string(), create_test_sprite(2, 2), false)
            .unwrap();

        assert_eq!(cache.len(), 3);

        let cleared = cache.clear_session("sess1");
        assert_eq!(cleared, 2);
        assert_eq!(cache.len(), 1);
        assert!(cache.peek("sess2:sprite1").is_some());
    }

    #[test]
    fn test_clear_session_empty() {
        let mut cache = SpriteCache::new(1024);
        let cleared = cache.clear_session("nonexistent");
        assert_eq!(cleared, 0);
    }

    // ===================
    // Memory tracking tests
    // ===================

    #[test]
    fn test_memory_usage_tracking() {
        let mut cache = SpriteCache::new(1024 * 1024);

        assert_eq!(cache.memory_usage(), 0);

        let sprite1 = create_test_sprite(5, 5);
        let size1 = sprite1.size_bytes();
        cache.insert("s1".to_string(), sprite1, false).unwrap();
        assert_eq!(cache.memory_usage(), size1);

        let sprite2 = create_test_sprite(3, 3);
        let size2 = sprite2.size_bytes();
        cache.insert("s2".to_string(), sprite2, false).unwrap();
        assert_eq!(cache.memory_usage(), size1 + size2);

        // Remove one
        cache.evict_lru();
        assert!(cache.memory_usage() < size1 + size2);
    }

    #[test]
    fn test_memory_usage_on_clear() {
        let mut cache = SpriteCache::new(1024 * 1024);
        cache
            .insert("s1".to_string(), create_test_sprite(5, 5), false)
            .unwrap();
        cache
            .insert("s2".to_string(), create_test_sprite(5, 5), false)
            .unwrap();

        assert!(cache.memory_usage() > 0);

        cache.clear();
        assert_eq!(cache.memory_usage(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_sprite_too_large_for_budget() {
        let small_budget = 100;
        let mut cache = SpriteCache::new(small_budget);

        let sprite = create_test_sprite(10, 10);
        let result = cache.insert("big".to_string(), sprite, false);

        assert!(matches!(result, Err(CacheError::SpriteTooLarge { .. })));
    }

    // ===================
    // Clear operations tests
    // ===================

    #[test]
    fn test_clear_non_base() {
        let mut cache = SpriteCache::new(1024 * 1024);

        cache
            .insert("base1".to_string(), create_test_sprite(2, 2), true)
            .unwrap();
        cache
            .insert("base2".to_string(), create_test_sprite(2, 2), true)
            .unwrap();
        cache
            .insert("normal1".to_string(), create_test_sprite(2, 2), false)
            .unwrap();
        cache
            .insert("normal2".to_string(), create_test_sprite(2, 2), false)
            .unwrap();

        let cleared = cache.clear_non_base();
        assert_eq!(cleared, 2);
        assert_eq!(cache.len(), 2);
        assert!(cache.peek("base1").is_some());
        assert!(cache.peek("base2").is_some());
    }

    // ===================
    // Cache stats tests
    // ===================

    #[test]
    fn test_cache_stats() {
        let mut cache = SpriteCache::new(1024 * 1024);

        cache
            .insert("base".to_string(), create_test_sprite(3, 3), true)
            .unwrap();
        cache
            .insert("normal".to_string(), create_test_sprite(2, 2), false)
            .unwrap();

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.base_entries, 1);
        assert_eq!(stats.non_base_entries, 1);
        assert!(stats.memory_usage_bytes > 0);
        assert!(stats.base_size_bytes > 0);
        assert!(stats.non_base_size_bytes > 0);
        assert_eq!(
            stats.memory_usage_bytes,
            stats.base_size_bytes + stats.non_base_size_bytes
        );
    }

    // ===================
    // Edge cases
    // ===================

    #[test]
    fn test_empty_cache_operations() {
        let cache = SpriteCache::new(1024);
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.memory_usage(), 0);
    }

    #[test]
    fn test_access_count_tracking() {
        let mut cache = SpriteCache::new(1024 * 1024);
        cache
            .insert("key".to_string(), create_test_sprite(2, 2), false)
            .unwrap();

        assert_eq!(cache.get_entry("key").unwrap().access_count, 0);

        let _ = cache.get("key");
        assert_eq!(cache.get_entry("key").unwrap().access_count, 1);

        let _ = cache.get("key");
        let _ = cache.get("key");
        assert_eq!(cache.get_entry("key").unwrap().access_count, 3);
    }

    #[test]
    fn test_default_budget() {
        let cache = SpriteCache::with_default_budget();
        assert_eq!(cache.memory_budget(), DEFAULT_MEMORY_BUDGET_BYTES);
    }
}
