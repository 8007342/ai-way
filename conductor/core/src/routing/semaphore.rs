//! Weighted Semaphore for Resource Management
//!
//! A semaphore that supports weighted permits for managing heterogeneous
//! resources like GPU memory. Useful for local model loading where different
//! models have different resource requirements.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use tokio::sync::{Mutex, Notify};

// ============================================================================
// Weighted Semaphore
// ============================================================================

/// A weighted semaphore for managing resources with variable costs
///
/// Unlike a standard semaphore where each permit is equal, this allows
/// acquiring permits of different weights (e.g., model A needs 8GB,
/// model B needs 2GB of a 16GB GPU).
pub struct WeightedSemaphore {
    /// Total capacity
    capacity: u64,
    /// Currently available capacity
    available: AtomicU64,
    /// Waiters queue
    waiters: Mutex<VecDeque<Waiter>>,
    /// Notification for waiters
    notify: Notify,
    /// Statistics
    stats: SemaphoreStats,
}

/// A waiter in the queue
struct Waiter {
    weight: u64,
    notified: bool,
    queued_at: Instant,
}

/// Semaphore statistics
struct SemaphoreStats {
    total_acquires: AtomicU64,
    total_releases: AtomicU64,
    total_wait_time_ms: AtomicU64,
    current_waiters: AtomicU64,
    peak_usage: AtomicU64,
}

impl Default for SemaphoreStats {
    fn default() -> Self {
        Self {
            total_acquires: AtomicU64::new(0),
            total_releases: AtomicU64::new(0),
            total_wait_time_ms: AtomicU64::new(0),
            current_waiters: AtomicU64::new(0),
            peak_usage: AtomicU64::new(0),
        }
    }
}

impl WeightedSemaphore {
    /// Create a new weighted semaphore with the given capacity
    #[must_use]
    pub fn new(capacity: u64) -> Self {
        Self {
            capacity,
            available: AtomicU64::new(capacity),
            waiters: Mutex::new(VecDeque::new()),
            notify: Notify::new(),
            stats: SemaphoreStats::default(),
        }
    }

    /// Get total capacity
    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    /// Get currently available capacity
    pub fn available(&self) -> u64 {
        self.available.load(Ordering::Relaxed)
    }

    /// Get current usage
    pub fn usage(&self) -> u64 {
        self.capacity - self.available()
    }

    /// Get usage as a fraction (0.0 - 1.0)
    pub fn usage_fraction(&self) -> f64 {
        self.usage() as f64 / self.capacity as f64
    }

    /// Try to acquire a permit of the given weight without blocking
    pub fn try_acquire(&self, weight: u64) -> Option<WeightedPermit> {
        if weight > self.capacity {
            return None; // Weight exceeds capacity
        }

        // Try to atomically decrement available
        let mut current = self.available.load(Ordering::Relaxed);
        loop {
            if current < weight {
                return None; // Not enough capacity
            }

            match self.available.compare_exchange_weak(
                current,
                current - weight,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    self.stats.total_acquires.fetch_add(1, Ordering::Relaxed);

                    // Update peak usage
                    let usage = self.usage();
                    let peak = self.stats.peak_usage.load(Ordering::Relaxed);
                    if usage > peak {
                        self.stats.peak_usage.store(usage, Ordering::Relaxed);
                    }

                    return Some(WeightedPermit { weight });
                }
                Err(actual) => {
                    current = actual;
                }
            }
        }
    }

    /// Acquire a permit of the given weight, blocking if necessary
    pub async fn acquire(&self, weight: u64) -> Result<WeightedPermit, SemaphoreError> {
        if weight > self.capacity {
            return Err(SemaphoreError::WeightExceedsCapacity {
                weight,
                capacity: self.capacity,
            });
        }

        // Fast path: try without blocking
        if let Some(permit) = self.try_acquire(weight) {
            return Ok(permit);
        }

        // Slow path: add to waiters queue
        let wait_start = Instant::now();
        self.stats.current_waiters.fetch_add(1, Ordering::Relaxed);

        // Add ourselves to the wait queue
        {
            let mut waiters = self.waiters.lock().await;
            waiters.push_back(Waiter {
                weight,
                notified: false,
                queued_at: wait_start,
            });
        }

        loop {
            // Check if we can acquire now
            if let Some(permit) = self.try_acquire(weight) {
                self.stats.current_waiters.fetch_sub(1, Ordering::Relaxed);
                self.stats
                    .total_wait_time_ms
                    .fetch_add(wait_start.elapsed().as_millis() as u64, Ordering::Relaxed);

                // Remove ourselves from the wait queue
                {
                    let mut waiters = self.waiters.lock().await;
                    if let Some(pos) = waiters
                        .iter()
                        .position(|w| w.weight == weight && w.queued_at == wait_start)
                    {
                        waiters.remove(pos);
                    }
                }

                return Ok(permit);
            }

            // Wait for notification
            self.notify.notified().await;
        }
    }

    /// Acquire a permit with a timeout
    pub async fn acquire_timeout(
        &self,
        weight: u64,
        timeout: Duration,
    ) -> Result<WeightedPermit, SemaphoreError> {
        if let Ok(result) = tokio::time::timeout(timeout, self.acquire(weight)).await {
            result
        } else {
            self.stats.current_waiters.fetch_sub(1, Ordering::Relaxed);
            Err(SemaphoreError::Timeout)
        }
    }

    /// Release a permit back to the semaphore
    #[allow(clippy::needless_pass_by_value)]
    pub fn release(&self, permit: WeightedPermit) {
        self.available.fetch_add(permit.weight, Ordering::Release);
        self.stats.total_releases.fetch_add(1, Ordering::Relaxed);
        let _ = permit; // Consume the permit (no Drop impl)
        self.notify.notify_waiters();
    }

    /// Get statistics
    pub fn stats(&self) -> WeightedSemaphoreStats {
        WeightedSemaphoreStats {
            capacity: self.capacity,
            available: self.available(),
            usage: self.usage(),
            usage_fraction: self.usage_fraction(),
            total_acquires: self.stats.total_acquires.load(Ordering::Relaxed),
            total_releases: self.stats.total_releases.load(Ordering::Relaxed),
            current_waiters: self.stats.current_waiters.load(Ordering::Relaxed),
            peak_usage: self.stats.peak_usage.load(Ordering::Relaxed),
            avg_wait_time_ms: {
                let acquires = self.stats.total_acquires.load(Ordering::Relaxed);
                let wait_time = self.stats.total_wait_time_ms.load(Ordering::Relaxed);
                if acquires > 0 {
                    wait_time / acquires
                } else {
                    0
                }
            },
        }
    }
}

/// A weighted permit (RAII guard)
#[derive(Debug)]
pub struct WeightedPermit {
    weight: u64,
}

impl WeightedPermit {
    /// Get the weight of this permit
    #[must_use]
    pub fn weight(&self) -> u64 {
        self.weight
    }
}

/// Statistics for a weighted semaphore
#[derive(Clone, Debug)]
pub struct WeightedSemaphoreStats {
    pub capacity: u64,
    pub available: u64,
    pub usage: u64,
    pub usage_fraction: f64,
    pub total_acquires: u64,
    pub total_releases: u64,
    pub current_waiters: u64,
    pub peak_usage: u64,
    pub avg_wait_time_ms: u64,
}

/// Semaphore errors
#[derive(Clone, Debug)]
pub enum SemaphoreError {
    /// Weight exceeds total capacity
    WeightExceedsCapacity { weight: u64, capacity: u64 },
    /// Timeout waiting for permit
    Timeout,
    /// Semaphore closed
    Closed,
}

impl std::fmt::Display for SemaphoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WeightExceedsCapacity { weight, capacity } => {
                write!(f, "Weight {weight} exceeds capacity {capacity}")
            }
            Self::Timeout => write!(f, "Timeout waiting for permit"),
            Self::Closed => write!(f, "Semaphore closed"),
        }
    }
}

impl std::error::Error for SemaphoreError {}

// ============================================================================
// GPU Memory Manager
// ============================================================================

/// Manages GPU memory allocation for local models
///
/// Uses a weighted semaphore where weights are memory sizes in bytes.
/// Supports memory pressure monitoring and eviction.
pub struct GpuMemoryManager {
    /// Memory semaphore (capacity = total GPU memory)
    semaphore: WeightedSemaphore,
    /// Currently loaded models: `model_id` -> `memory_bytes`
    loaded_models: Mutex<std::collections::HashMap<String, ModelAllocation>>,
    /// Memory pressure threshold (0.0 - 1.0)
    pressure_threshold: f64,
}

/// Information about a loaded model
#[derive(Clone, Debug)]
pub struct ModelAllocation {
    pub model_id: String,
    pub memory_bytes: u64,
    pub loaded_at: Instant,
    pub last_used: Instant,
    pub request_count: u64,
}

impl GpuMemoryManager {
    /// Create a new GPU memory manager
    #[must_use]
    pub fn new(total_memory_bytes: u64, pressure_threshold: f64) -> Self {
        Self {
            semaphore: WeightedSemaphore::new(total_memory_bytes),
            loaded_models: Mutex::new(std::collections::HashMap::new()),
            pressure_threshold: pressure_threshold.clamp(0.5, 0.99),
        }
    }

    /// Check if memory pressure is high
    pub fn is_under_pressure(&self) -> bool {
        self.semaphore.usage_fraction() >= self.pressure_threshold
    }

    /// Get available memory
    pub fn available_memory(&self) -> u64 {
        self.semaphore.available()
    }

    /// Get total memory
    pub fn total_memory(&self) -> u64 {
        self.semaphore.capacity()
    }

    /// Check if a model is loaded
    pub async fn is_model_loaded(&self, model_id: &str) -> bool {
        let models = self.loaded_models.lock().await;
        models.contains_key(model_id)
    }

    /// Try to allocate memory for a model
    pub async fn allocate(
        &self,
        model_id: &str,
        memory_bytes: u64,
        timeout: Duration,
    ) -> Result<WeightedPermit, MemoryError> {
        // Check if already loaded
        {
            let models = self.loaded_models.lock().await;
            if models.contains_key(model_id) {
                return Err(MemoryError::AlreadyLoaded(model_id.to_string()));
            }
        }

        // Try to acquire memory
        match self.semaphore.acquire_timeout(memory_bytes, timeout).await {
            Ok(permit) => {
                // Record the allocation
                let mut models = self.loaded_models.lock().await;
                models.insert(
                    model_id.to_string(),
                    ModelAllocation {
                        model_id: model_id.to_string(),
                        memory_bytes,
                        loaded_at: Instant::now(),
                        last_used: Instant::now(),
                        request_count: 0,
                    },
                );
                Ok(permit)
            }
            Err(SemaphoreError::Timeout) => Err(MemoryError::InsufficientMemory {
                requested: memory_bytes,
                available: self.semaphore.available(),
            }),
            Err(SemaphoreError::WeightExceedsCapacity { weight, capacity }) => {
                Err(MemoryError::ModelTooLarge {
                    model_id: model_id.to_string(),
                    size: weight,
                    capacity,
                })
            }
            Err(e) => Err(MemoryError::Other(e.to_string())),
        }
    }

    /// Release memory for a model
    pub async fn release(&self, model_id: &str, permit: WeightedPermit) {
        let mut models = self.loaded_models.lock().await;
        models.remove(model_id);
        self.semaphore.release(permit);
    }

    /// Mark a model as used (updates LRU tracking)
    pub async fn touch(&self, model_id: &str) {
        let mut models = self.loaded_models.lock().await;
        if let Some(alloc) = models.get_mut(model_id) {
            alloc.last_used = Instant::now();
            alloc.request_count += 1;
        }
    }

    /// Get the least recently used model (for eviction)
    pub async fn get_lru_model(&self) -> Option<String> {
        let models = self.loaded_models.lock().await;
        models
            .values()
            .min_by_key(|a| a.last_used)
            .map(|a| a.model_id.clone())
    }

    /// Get all loaded models
    pub async fn loaded_models(&self) -> Vec<ModelAllocation> {
        let models = self.loaded_models.lock().await;
        models.values().cloned().collect()
    }

    /// Get memory statistics
    pub fn stats(&self) -> WeightedSemaphoreStats {
        self.semaphore.stats()
    }
}

/// Memory management errors
#[derive(Clone, Debug)]
pub enum MemoryError {
    /// Model is already loaded
    AlreadyLoaded(String),
    /// Not enough memory available
    InsufficientMemory { requested: u64, available: u64 },
    /// Model is too large for total capacity
    ModelTooLarge {
        model_id: String,
        size: u64,
        capacity: u64,
    },
    /// Other error
    Other(String),
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyLoaded(id) => write!(f, "Model {id} is already loaded"),
            Self::InsufficientMemory {
                requested,
                available,
            } => {
                write!(
                    f,
                    "Insufficient memory: requested {requested} bytes, {available} available"
                )
            }
            Self::ModelTooLarge {
                model_id,
                size,
                capacity,
            } => {
                write!(
                    f,
                    "Model {model_id} ({size} bytes) exceeds total capacity ({capacity} bytes)"
                )
            }
            Self::Other(e) => write!(f, "Memory error: {e}"),
        }
    }
}

impl std::error::Error for MemoryError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weighted_semaphore_try_acquire() {
        let sem = WeightedSemaphore::new(100);

        // Should succeed
        let p1 = sem.try_acquire(30).expect("should acquire 30");
        assert_eq!(sem.available(), 70);

        let p2 = sem.try_acquire(50).expect("should acquire 50");
        assert_eq!(sem.available(), 20);

        // Should fail - not enough capacity
        assert!(sem.try_acquire(30).is_none());

        // Release one
        sem.release(p1);
        assert_eq!(sem.available(), 50);

        // Now should succeed
        let _p3 = sem
            .try_acquire(30)
            .expect("should acquire 30 after release");
        assert_eq!(sem.available(), 20);

        // Clean up
        sem.release(p2);
    }

    #[test]
    fn test_weighted_semaphore_exceeds_capacity() {
        let sem = WeightedSemaphore::new(100);

        // Trying to acquire more than capacity should fail
        assert!(sem.try_acquire(101).is_none());
    }

    #[tokio::test]
    async fn test_gpu_memory_manager() {
        let manager = GpuMemoryManager::new(16 * 1024 * 1024 * 1024, 0.85); // 16GB

        // Allocate a model
        let permit = manager
            .allocate("model-a", 4 * 1024 * 1024 * 1024, Duration::from_secs(1))
            .await
            .expect("should allocate");

        assert!(manager.is_model_loaded("model-a").await);
        assert!(!manager.is_model_loaded("model-b").await);

        // Touch the model
        manager.touch("model-a").await;

        // Release
        manager.release("model-a", permit).await;
        assert!(!manager.is_model_loaded("model-a").await);
    }
}
