//! Connection Pool Management
//!
//! Manages connection pools for different backend types with:
//! - Per-backend connection limits
//! - Health checking and connection eviction
//! - Metrics tracking per pool
//! - Proper connection reuse via RAII pattern
//!
//! # Design
//!
//! Each backend gets its own pool with configurable limits. Pools are lazily
//! initialized and can be dynamically reconfigured. Connection health is
//! monitored continuously.
//!
//! ## Connection Lifecycle
//!
//! 1. `acquire()` - Get connection from pool or create new one
//! 2. Use connection via `PooledConnection` wrapper
//! 3. `PooledConnection::Drop` - Automatically returns connection to pool
//!
//! The pool uses `Arc<Self>` for shared ownership and an async mpsc channel
//! for returning connections from Drop handlers (which cannot be async).

use std::collections::HashMap;
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, RwLock, Semaphore};

use super::config::{BackendConfig, ConnectionConfig};

// ============================================================================
// Connection Pool Types
// ============================================================================

/// Statistics for a connection pool
#[derive(Clone, Debug, Default)]
pub struct PoolStats {
    /// Total connections created
    pub connections_created: u64,
    /// Total connections closed
    pub connections_closed: u64,
    /// Current active connections
    pub active_connections: usize,
    /// Current idle connections
    pub idle_connections: usize,
    /// Requests waiting for a connection
    pub waiting_requests: usize,
    /// Total wait time (ms)
    pub total_wait_time_ms: u64,
    /// Connection errors
    pub connection_errors: u64,
    /// Health check failures
    pub health_check_failures: u64,
    /// Connections reused (returned to pool)
    pub connections_reused: u64,
}

/// A managed connection slot
#[derive(Debug)]
pub struct ConnectionSlot {
    /// Backend ID this slot belongs to
    pub backend_id: String,
    /// When this connection was created
    pub created_at: Instant,
    /// When this connection was last used
    pub last_used: Instant,
    /// Number of requests handled
    pub requests_handled: u64,
    /// Whether the connection is healthy
    pub healthy: bool,
    /// Connection-specific data (backend-dependent)
    pub data: ConnectionData,
}

/// Backend-specific connection data
#[derive(Debug)]
pub enum ConnectionData {
    /// HTTP client (for Ollama, `OpenAI`, etc.)
    Http { client: reqwest::Client },
    /// gRPC channel
    Grpc {
        // Would hold tonic::Channel if implemented
        endpoint: String,
    },
    /// Local model handle
    LocalModel { model_id: String, loaded: bool },
}

impl ConnectionSlot {
    /// Create a new HTTP connection slot
    #[must_use]
    pub fn new_http(backend_id: String, client: reqwest::Client) -> Self {
        let now = Instant::now();
        Self {
            backend_id,
            created_at: now,
            last_used: now,
            requests_handled: 0,
            healthy: true,
            data: ConnectionData::Http { client },
        }
    }

    /// Mark the connection as used
    pub fn touch(&mut self) {
        self.last_used = Instant::now();
        self.requests_handled += 1;
    }

    /// Check if connection is stale
    #[must_use]
    pub fn is_stale(&self, max_idle: Duration, max_lifetime: Duration) -> bool {
        let now = Instant::now();
        let idle_time = now.duration_since(self.last_used);
        let lifetime = now.duration_since(self.created_at);

        idle_time > max_idle || lifetime > max_lifetime
    }

    /// Get the HTTP client (if this is an HTTP connection)
    #[must_use]
    pub fn http_client(&self) -> Option<&reqwest::Client> {
        match &self.data {
            ConnectionData::Http { client } => Some(client),
            _ => None,
        }
    }
}

// ============================================================================
// Connection Pool
// ============================================================================

/// A pool of connections for a single backend.
///
/// Uses `Arc<Self>` pattern for shared ownership, allowing `PooledConnection`
/// to hold a reference and return connections on drop.
pub struct ConnectionPool {
    /// Backend identifier
    backend_id: String,
    /// Pool configuration
    config: ConnectionConfig,
    /// Available (idle) connections
    connections: RwLock<Vec<ConnectionSlot>>,
    /// Semaphore limiting concurrent connections
    semaphore: Arc<Semaphore>,
    /// Statistics
    stats: PoolStatsAtomic,
    /// Pool state
    state: RwLock<PoolState>,
    /// Channel for returning connections from Drop handlers
    return_tx: mpsc::UnboundedSender<ConnectionSlot>,
    /// Receiver is held by the pool for processing returns
    return_rx: RwLock<Option<mpsc::UnboundedReceiver<ConnectionSlot>>>,
}

/// Atomic statistics for lock-free updates
struct PoolStatsAtomic {
    connections_created: AtomicU64,
    connections_closed: AtomicU64,
    active_connections: AtomicUsize,
    waiting_requests: AtomicUsize,
    total_wait_time_ms: AtomicU64,
    connection_errors: AtomicU64,
    health_check_failures: AtomicU64,
    connections_reused: AtomicU64,
}

impl Default for PoolStatsAtomic {
    fn default() -> Self {
        Self {
            connections_created: AtomicU64::new(0),
            connections_closed: AtomicU64::new(0),
            active_connections: AtomicUsize::new(0),
            waiting_requests: AtomicUsize::new(0),
            total_wait_time_ms: AtomicU64::new(0),
            connection_errors: AtomicU64::new(0),
            health_check_failures: AtomicU64::new(0),
            connections_reused: AtomicU64::new(0),
        }
    }
}

impl PoolStatsAtomic {
    fn snapshot(&self) -> PoolStats {
        PoolStats {
            connections_created: self.connections_created.load(Ordering::Relaxed),
            connections_closed: self.connections_closed.load(Ordering::Relaxed),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            idle_connections: 0, // Calculated separately
            waiting_requests: self.waiting_requests.load(Ordering::Relaxed),
            total_wait_time_ms: self.total_wait_time_ms.load(Ordering::Relaxed),
            connection_errors: self.connection_errors.load(Ordering::Relaxed),
            health_check_failures: self.health_check_failures.load(Ordering::Relaxed),
            connections_reused: self.connections_reused.load(Ordering::Relaxed),
        }
    }
}

/// Pool state
#[derive(Clone, Debug, Default)]
struct PoolState {
    /// Whether the pool is accepting new requests
    accepting: bool,
    /// Whether the pool is draining (no new connections)
    draining: bool,
    /// Last health check time
    last_health_check: Option<Instant>,
    /// Health check result
    healthy: bool,
}

impl ConnectionPool {
    /// Create a new connection pool wrapped in Arc for shared ownership.
    ///
    /// This is the preferred way to create a pool, as it enables proper
    /// connection return on drop via the RAII pattern.
    #[must_use]
    pub fn new_shared(backend_id: String, config: ConnectionConfig) -> Arc<Self> {
        let (return_tx, return_rx) = mpsc::unbounded_channel();
        let max_connections = config.max_connections;

        Arc::new(Self {
            backend_id,
            config,
            connections: RwLock::new(Vec::with_capacity(max_connections)),
            semaphore: Arc::new(Semaphore::new(max_connections)),
            stats: PoolStatsAtomic::default(),
            state: RwLock::new(PoolState {
                accepting: true,
                healthy: true,
                ..Default::default()
            }),
            return_tx,
            return_rx: RwLock::new(Some(return_rx)),
        })
    }

    /// Create a new connection pool (legacy, for backwards compatibility).
    ///
    /// Note: For proper connection reuse, prefer `new_shared()` which returns
    /// an `Arc<ConnectionPool>`.
    #[must_use]
    pub fn new(backend_id: String, config: ConnectionConfig) -> Self {
        let (return_tx, return_rx) = mpsc::unbounded_channel();
        let max_connections = config.max_connections;

        Self {
            backend_id,
            config,
            connections: RwLock::new(Vec::with_capacity(max_connections)),
            semaphore: Arc::new(Semaphore::new(max_connections)),
            stats: PoolStatsAtomic::default(),
            state: RwLock::new(PoolState {
                accepting: true,
                healthy: true,
                ..Default::default()
            }),
            return_tx,
            return_rx: RwLock::new(Some(return_rx)),
        }
    }

    /// Process any pending connection returns from Drop handlers.
    ///
    /// This should be called periodically or before acquiring connections
    /// to ensure returned connections are available for reuse.
    pub async fn process_returns(&self) {
        let mut rx_guard = self.return_rx.write().await;
        if let Some(ref mut rx) = *rx_guard {
            // Process all available returns without blocking
            while let Ok(mut connection) = rx.try_recv() {
                self.handle_return(&mut connection).await;
            }
        }
    }

    /// Internal handler for connection returns
    async fn handle_return(&self, connection: &mut ConnectionSlot) {
        self.stats
            .active_connections
            .fetch_sub(1, Ordering::Relaxed);

        // Check if connection is still healthy and not stale
        let max_idle = Duration::from_millis(self.config.keepalive_interval_ms * 2);
        let max_lifetime = Duration::from_secs(3600); // 1 hour max lifetime

        if connection.healthy && !connection.is_stale(max_idle, max_lifetime) {
            connection.touch();
            let mut connections = self.connections.write().await;
            if connections.len() < self.config.max_idle_connections {
                connections.push(std::mem::replace(
                    connection,
                    ConnectionSlot {
                        backend_id: String::new(),
                        created_at: Instant::now(),
                        last_used: Instant::now(),
                        requests_handled: 0,
                        healthy: false,
                        data: ConnectionData::Grpc {
                            endpoint: String::new(),
                        },
                    },
                ));
                self.stats
                    .connections_reused
                    .fetch_add(1, Ordering::Relaxed);
            } else {
                // Pool is full, close the connection
                self.stats
                    .connections_closed
                    .fetch_add(1, Ordering::Relaxed);
            }
        } else {
            self.stats
                .connections_closed
                .fetch_add(1, Ordering::Relaxed);
        }

        // Release the semaphore permit
        self.semaphore.add_permits(1);
    }

    /// Get current pool statistics
    pub async fn stats(&self) -> PoolStats {
        // Process any pending returns first
        self.process_returns().await;

        let mut stats = self.stats.snapshot();
        let connections = self.connections.read().await;
        stats.idle_connections = connections.len();
        stats
    }

    /// Check if pool is healthy
    pub async fn is_healthy(&self) -> bool {
        let state = self.state.read().await;
        state.healthy && state.accepting
    }

    /// Acquire a connection from the pool (for non-Arc pools).
    ///
    /// For pools created with `new_shared()`, use `acquire_shared()` instead
    /// to enable automatic connection return on drop.
    pub async fn acquire(&self, timeout: Duration) -> Result<PooledConnection, PoolError> {
        // Process any pending returns first
        self.process_returns().await;

        self.stats.waiting_requests.fetch_add(1, Ordering::Relaxed);
        let wait_start = Instant::now();

        // Try to acquire a permit (respects max_connections)
        let permit =
            match tokio::time::timeout(timeout, self.semaphore.clone().acquire_owned()).await {
                Ok(Ok(permit)) => permit,
                Ok(Err(_)) => {
                    self.stats.waiting_requests.fetch_sub(1, Ordering::Relaxed);
                    return Err(PoolError::PoolClosed);
                }
                Err(_) => {
                    self.stats.waiting_requests.fetch_sub(1, Ordering::Relaxed);
                    return Err(PoolError::Timeout);
                }
            };

        let wait_time = wait_start.elapsed();
        self.stats
            .total_wait_time_ms
            .fetch_add(wait_time.as_millis() as u64, Ordering::Relaxed);
        self.stats.waiting_requests.fetch_sub(1, Ordering::Relaxed);

        // Try to get an existing connection
        let mut connections = self.connections.write().await;
        let connection = if let Some(conn) = connections.pop() {
            conn
        } else {
            // Create a new connection
            drop(connections); // Release lock before creating connection
            match self.create_connection().await {
                Ok(conn) => conn,
                Err(e) => {
                    // Return the permit since we couldn't create a connection
                    drop(permit);
                    self.stats.connection_errors.fetch_add(1, Ordering::Relaxed);
                    return Err(e);
                }
            }
        };

        self.stats
            .active_connections
            .fetch_add(1, Ordering::Relaxed);

        Ok(PooledConnection {
            connection: Some(connection),
            return_tx: self.return_tx.clone(),
            semaphore: self.semaphore.clone(),
            _permit: Some(permit),
        })
    }

    /// Acquire a connection from an Arc-wrapped pool.
    ///
    /// This is the preferred method when using `new_shared()` as it enables
    /// automatic connection return via the RAII pattern.
    pub async fn acquire_shared(
        self: &Arc<Self>,
        timeout: Duration,
    ) -> Result<PooledConnection, PoolError> {
        self.acquire(timeout).await
    }

    /// Create a new connection (HTTP client for now)
    async fn create_connection(&self) -> Result<ConnectionSlot, PoolError> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(self.config.connect_timeout_ms))
            .timeout(Duration::from_millis(self.config.read_timeout_ms))
            .pool_max_idle_per_host(self.config.max_idle_connections)
            .pool_idle_timeout(Duration::from_millis(self.config.keepalive_interval_ms))
            .http2_prior_knowledge()
            .build()
            .map_err(|e| PoolError::ConnectionFailed(e.to_string()))?;

        self.stats
            .connections_created
            .fetch_add(1, Ordering::Relaxed);

        Ok(ConnectionSlot::new_http(self.backend_id.clone(), client))
    }

    /// Return a connection to the pool (for direct/synchronous return).
    ///
    /// Normally connections are returned automatically via `PooledConnection::Drop`,
    /// but this method allows explicit return when needed.
    pub async fn release(&self, mut connection: ConnectionSlot) {
        self.handle_return(&mut connection).await;
    }

    /// Clean up idle connections that have exceeded their timeout.
    ///
    /// This should be called periodically (e.g., every 30 seconds) to
    /// prevent resource leaks from stale connections.
    pub async fn cleanup_idle(&self) {
        let max_idle = Duration::from_millis(self.config.keepalive_interval_ms * 2);
        let max_lifetime = Duration::from_secs(3600);

        let mut connections = self.connections.write().await;
        let original_len = connections.len();

        connections.retain(|conn| !conn.is_stale(max_idle, max_lifetime));

        let removed = original_len - connections.len();
        if removed > 0 {
            self.stats
                .connections_closed
                .fetch_add(removed as u64, Ordering::Relaxed);
        }
    }

    /// Drain the pool (close all idle connections)
    pub async fn drain(&self) {
        {
            let mut state = self.state.write().await;
            state.draining = true;
        }

        let mut connections = self.connections.write().await;
        let count = connections.len();
        connections.clear();
        self.stats
            .connections_closed
            .fetch_add(count as u64, Ordering::Relaxed);
    }

    /// Run health check on the pool
    pub async fn health_check(&self) -> bool {
        // For now, just check if we can create a connection
        if let Ok(conn) = self.create_connection().await {
            // Return it to the pool
            let mut connections = self.connections.write().await;
            connections.push(conn);

            let mut state = self.state.write().await;
            state.healthy = true;
            state.last_health_check = Some(Instant::now());
            true
        } else {
            self.stats
                .health_check_failures
                .fetch_add(1, Ordering::Relaxed);
            let mut state = self.state.write().await;
            state.healthy = false;
            state.last_health_check = Some(Instant::now());
            false
        }
    }

    /// Start a background task that periodically cleans up idle connections.
    ///
    /// Returns a handle that can be used to stop the cleanup task.
    pub fn start_cleanup_task(self: Arc<Self>, interval: Duration) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;

                // Process any pending returns
                self.process_returns().await;

                // Clean up stale connections
                self.cleanup_idle().await;

                // Check if pool is draining
                let state = self.state.read().await;
                if state.draining {
                    break;
                }
            }
        })
    }
}

// ============================================================================
// Pooled Connection (RAII Guard)
// ============================================================================

/// A connection borrowed from the pool (RAII guard).
///
/// Implements `Deref` to allow transparent access to the underlying `ConnectionSlot`.
/// Automatically returns the connection to the pool when dropped.
pub struct PooledConnection {
    connection: Option<ConnectionSlot>,
    return_tx: mpsc::UnboundedSender<ConnectionSlot>,
    semaphore: Arc<Semaphore>,
    _permit: Option<tokio::sync::OwnedSemaphorePermit>,
}

impl PooledConnection {
    /// Get the underlying connection (legacy method for compatibility)
    #[must_use]
    pub fn connection(&self) -> &ConnectionSlot {
        self.connection.as_ref().expect("connection taken")
    }

    /// Get mutable access to the connection
    pub fn connection_mut(&mut self) -> &mut ConnectionSlot {
        self.connection.as_mut().expect("connection taken")
    }

    /// Mark the connection as unhealthy (will be closed on release)
    pub fn mark_unhealthy(&mut self) {
        if let Some(ref mut conn) = self.connection {
            conn.healthy = false;
        }
    }

    /// Get the HTTP client (if this is an HTTP connection)
    #[must_use]
    pub fn http_client(&self) -> Option<&reqwest::Client> {
        self.connection.as_ref()?.http_client()
    }
}

impl Deref for PooledConnection {
    type Target = ConnectionSlot;

    fn deref(&self) -> &Self::Target {
        self.connection.as_ref().expect("connection taken")
    }
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.take() {
            // Send the connection back to the pool via channel.
            // This is non-blocking and works from sync Drop context.
            // The pool will process returns and either reuse or close the connection.
            if self.return_tx.send(connection).is_err() {
                // Pool is closed, just release the semaphore permit
                self.semaphore.add_permits(1);
            }
            // Note: permit is dropped automatically, but we use the channel
            // to actually return the connection for reuse
        }
    }
}

/// Pool errors
#[derive(Clone, Debug)]
pub enum PoolError {
    /// Pool is closed
    PoolClosed,
    /// Timeout waiting for connection
    Timeout,
    /// Failed to create connection
    ConnectionFailed(String),
    /// Connection not healthy
    Unhealthy,
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PoolClosed => write!(f, "Connection pool is closed"),
            Self::Timeout => write!(f, "Timeout waiting for connection"),
            Self::ConnectionFailed(e) => write!(f, "Failed to create connection: {e}"),
            Self::Unhealthy => write!(f, "Connection is unhealthy"),
        }
    }
}

impl std::error::Error for PoolError {}

// ============================================================================
// Pool Manager
// ============================================================================

/// Manages connection pools for all backends
pub struct PoolManager {
    /// Pools by backend ID
    pools: RwLock<HashMap<String, Arc<ConnectionPool>>>,
    /// Default connection config
    #[allow(dead_code)]
    default_config: ConnectionConfig,
}

impl PoolManager {
    /// Create a new pool manager
    #[must_use]
    pub fn new(default_config: ConnectionConfig) -> Self {
        Self {
            pools: RwLock::new(HashMap::new()),
            default_config,
        }
    }

    /// Get or create a pool for a backend
    pub async fn get_pool(&self, backend_id: &str) -> Option<Arc<ConnectionPool>> {
        let pools = self.pools.read().await;
        pools.get(backend_id).cloned()
    }

    /// Create a pool for a backend
    pub async fn create_pool(&self, backend: &BackendConfig) -> Arc<ConnectionPool> {
        let pool = ConnectionPool::new_shared(backend.id.clone(), backend.connection.clone());

        let mut pools = self.pools.write().await;
        pools.insert(backend.id.clone(), pool.clone());
        pool
    }

    /// Remove a pool
    pub async fn remove_pool(&self, backend_id: &str) {
        let mut pools = self.pools.write().await;
        if let Some(pool) = pools.remove(backend_id) {
            pool.drain().await;
        }
    }

    /// Get stats for all pools
    pub async fn all_stats(&self) -> HashMap<String, PoolStats> {
        let pools = self.pools.read().await;
        let mut stats = HashMap::new();

        for (id, pool) in pools.iter() {
            stats.insert(id.clone(), pool.stats().await);
        }

        stats
    }

    /// Run health checks on all pools
    pub async fn health_check_all(&self) -> HashMap<String, bool> {
        let pools = self.pools.read().await;
        let mut results = HashMap::new();

        for (id, pool) in pools.iter() {
            results.insert(id.clone(), pool.health_check().await);
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_slot_staleness() {
        let client = reqwest::Client::new();
        let slot = ConnectionSlot::new_http("test".to_string(), client);

        // New connection should not be stale
        assert!(!slot.is_stale(Duration::from_secs(60), Duration::from_secs(3600)));
    }

    #[tokio::test]
    async fn test_pool_creation() {
        let config = ConnectionConfig::default();
        let pool = ConnectionPool::new("test-backend".to_string(), config);

        let stats = pool.stats().await;
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.connections_created, 0);
    }

    #[tokio::test]
    async fn test_pool_shared_creation() {
        let config = ConnectionConfig::default();
        let pool = ConnectionPool::new_shared("test-backend".to_string(), config);

        let stats = pool.stats().await;
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.connections_created, 0);
    }

    #[tokio::test]
    async fn test_connection_reuse() {
        let config = ConnectionConfig {
            max_connections: 4,
            max_idle_connections: 2,
            ..Default::default()
        };
        let pool = ConnectionPool::new_shared("test-backend".to_string(), config);

        // Acquire and release connections multiple times
        for _ in 0..5 {
            let conn = pool.acquire(Duration::from_secs(5)).await;
            assert!(conn.is_ok(), "Should acquire connection");
            // Connection automatically returned when dropped
            drop(conn);

            // Process returns
            pool.process_returns().await;
        }

        let stats = pool.stats().await;

        // Should have created fewer connections than requests due to reuse
        assert!(
            stats.connections_created < 5,
            "Should reuse connections, created: {}",
            stats.connections_created
        );
        assert!(
            stats.connections_reused > 0,
            "Should have reused at least one connection"
        );
    }

    #[tokio::test]
    async fn test_pooled_connection_deref() {
        let config = ConnectionConfig::default();
        let pool = ConnectionPool::new_shared("test-backend".to_string(), config);

        let conn = pool.acquire(Duration::from_secs(5)).await.unwrap();

        // Test Deref - should be able to access ConnectionSlot fields directly
        assert_eq!(conn.backend_id, "test-backend");
        assert!(conn.healthy);
    }

    #[tokio::test]
    async fn test_idle_cleanup() {
        let config = ConnectionConfig {
            max_connections: 4,
            max_idle_connections: 4,
            keepalive_interval_ms: 1, // Very short for testing
            ..Default::default()
        };
        let pool = ConnectionPool::new_shared("test-backend".to_string(), config);

        // Acquire and release a connection
        {
            let _conn = pool.acquire(Duration::from_secs(5)).await.unwrap();
        }
        pool.process_returns().await;

        // Wait for idle timeout
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Cleanup should remove stale connections
        pool.cleanup_idle().await;

        let stats = pool.stats().await;
        assert_eq!(
            stats.idle_connections, 0,
            "Stale connections should be cleaned up"
        );
    }
}
