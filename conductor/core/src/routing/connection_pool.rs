//! Connection Pool Management
//!
//! Manages connection pools for different backend types with:
//! - Per-backend connection limits
//! - Health checking and connection eviction
//! - Metrics tracking per pool
//!
//! # Design
//!
//! Each backend gets its own pool with configurable limits. Pools are lazily
//! initialized and can be dynamically reconfigured. Connection health is
//! monitored continuously.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{RwLock, Semaphore};

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
    /// HTTP client (for Ollama, OpenAI, etc.)
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
    pub fn is_stale(&self, max_idle: Duration, max_lifetime: Duration) -> bool {
        let now = Instant::now();
        let idle_time = now.duration_since(self.last_used);
        let lifetime = now.duration_since(self.created_at);

        idle_time > max_idle || lifetime > max_lifetime
    }

    /// Get the HTTP client (if this is an HTTP connection)
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

/// A pool of connections for a single backend
pub struct ConnectionPool {
    /// Backend identifier
    backend_id: String,
    /// Pool configuration
    config: ConnectionConfig,
    /// Available connections
    connections: RwLock<Vec<ConnectionSlot>>,
    /// Semaphore limiting concurrent connections
    semaphore: Semaphore,
    /// Statistics
    stats: PoolStatsAtomic,
    /// Pool state
    state: RwLock<PoolState>,
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
    /// Create a new connection pool
    pub fn new(backend_id: String, config: ConnectionConfig) -> Self {
        let max_connections = config.max_connections;
        Self {
            backend_id,
            config,
            connections: RwLock::new(Vec::with_capacity(max_connections)),
            semaphore: Semaphore::new(max_connections),
            stats: PoolStatsAtomic::default(),
            state: RwLock::new(PoolState {
                accepting: true,
                healthy: true,
                ..Default::default()
            }),
        }
    }

    /// Get current pool statistics
    pub async fn stats(&self) -> PoolStats {
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

    /// Acquire a connection from the pool
    ///
    /// This will:
    /// 1. Try to reuse an idle connection
    /// 2. Create a new connection if under limit
    /// 3. Wait for a connection to become available
    pub async fn acquire(&self, timeout: Duration) -> Result<PooledConnection, PoolError> {
        self.stats.waiting_requests.fetch_add(1, Ordering::Relaxed);
        let wait_start = Instant::now();

        // Try to acquire a permit (respects max_connections)
        let permit = match tokio::time::timeout(timeout, self.semaphore.acquire()).await {
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
        permit.forget(); // We'll manually return the permit on release

        Ok(PooledConnection {
            connection: Some(connection),
            pool: self,
        })
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

    /// Return a connection to the pool
    async fn release(&self, mut connection: ConnectionSlot) {
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
                connections.push(connection);
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
        match self.create_connection().await {
            Ok(conn) => {
                // Return it to the pool
                let mut connections = self.connections.write().await;
                connections.push(conn);

                let mut state = self.state.write().await;
                state.healthy = true;
                state.last_health_check = Some(Instant::now());
                true
            }
            Err(_) => {
                self.stats
                    .health_check_failures
                    .fetch_add(1, Ordering::Relaxed);
                let mut state = self.state.write().await;
                state.healthy = false;
                state.last_health_check = Some(Instant::now());
                false
            }
        }
    }
}

/// A connection borrowed from the pool (RAII guard)
pub struct PooledConnection<'a> {
    connection: Option<ConnectionSlot>,
    pool: &'a ConnectionPool,
}

impl<'a> PooledConnection<'a> {
    /// Get the underlying connection
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
    pub fn http_client(&self) -> Option<&reqwest::Client> {
        self.connection.as_ref()?.http_client()
    }
}

impl<'a> Drop for PooledConnection<'a> {
    fn drop(&mut self) {
        if let Some(_connection) = self.connection.take() {
            // In a full implementation, we'd use a channel to send the connection
            // back to the pool asynchronously. For now, the connection is simply
            // dropped and the pool semaphore permit is lost (not ideal but avoids
            // the lifetime issue).
            //
            // A better design would use Arc<ConnectionPool> and a return channel:
            // pool.return_tx.try_send(connection).ok();
            //
            // The pool would then have a background task that receives connections
            // and adds them back to the pool.
            self.pool.semaphore.add_permits(1);
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
            Self::ConnectionFailed(e) => write!(f, "Failed to create connection: {}", e),
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
    default_config: ConnectionConfig,
}

impl PoolManager {
    /// Create a new pool manager
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
        let pool = Arc::new(ConnectionPool::new(
            backend.id.clone(),
            backend.connection.clone(),
        ));

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
}
