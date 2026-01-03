//! Heartbeat Enforcement for Connection Health Monitoring
//!
//! This module provides heartbeat-based health monitoring for Conductor-Surface connections.
//! The Conductor periodically sends Ping messages and expects Pong responses from surfaces
//! within a configurable timeout.
//!
//! # Protocol
//!
//! 1. Conductor sends `ConductorMessage::Ping { seq }` every `heartbeat_interval`
//! 2. Surface must respond with `SurfaceEvent::Pong { seq }` within `response_timeout`
//! 3. After `max_missed_pongs` consecutive missed responses, connection is terminated
//!
//! # Features
//!
//! - Configurable heartbeat interval, timeout, and missed pong threshold
//! - Round-trip time (RTT) tracking for metrics
//! - Timer reset on any message received (avoids pinging during active exchange)
//! - Can be disabled for testing
//! - Health event emission for monitoring
//!
//! # Usage
//!
//! ```ignore
//! let config = HeartbeatConfig::default();
//! let monitor = HeartbeatMonitor::new(config);
//! let registry = SurfaceRegistry::new();
//!
//! // Start the heartbeat task
//! let task = HeartbeatTask::new(monitor.clone(), registry.clone());
//! tokio::spawn(task.run());
//!
//! // When a pong is received
//! monitor.record_pong(&connection_id, seq);
//!
//! // When any message is received (resets timer)
//! monitor.record_activity(&connection_id);
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::messages::ConductorMessage;
use crate::surface_registry::{ConnectionId, SurfaceRegistry};

/// Configuration for heartbeat behavior
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    /// Interval between ping messages (default: 30 seconds)
    pub heartbeat_interval: Duration,
    /// Maximum time to wait for a pong response (default: 10 seconds)
    pub response_timeout: Duration,
    /// Number of consecutive missed pongs before disconnecting (default: 3)
    pub max_missed_pongs: u32,
    /// Whether heartbeat is enabled (can be disabled for testing)
    pub enabled: bool,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(30),
            response_timeout: Duration::from_secs(10),
            max_missed_pongs: 3,
            enabled: true,
        }
    }
}

impl HeartbeatConfig {
    /// Create a new config with default values
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a config with heartbeat disabled (for testing)
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Set the heartbeat interval
    #[must_use]
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.heartbeat_interval = interval;
        self
    }

    /// Set the response timeout
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.response_timeout = timeout;
        self
    }

    /// Set the maximum missed pongs
    #[must_use]
    pub fn with_max_missed(mut self, max_missed: u32) -> Self {
        self.max_missed_pongs = max_missed;
        self
    }

    /// Create a config suitable for testing (shorter intervals)
    #[must_use]
    pub fn for_testing() -> Self {
        Self {
            heartbeat_interval: Duration::from_millis(100),
            response_timeout: Duration::from_millis(50),
            max_missed_pongs: 2,
            enabled: true,
        }
    }
}

/// Health metrics for a connection
#[derive(Clone, Debug)]
pub struct ConnectionHealth {
    /// Number of consecutive missed pongs
    pub missed_pongs: u32,
    /// Last recorded round-trip time
    pub last_rtt: Option<Duration>,
    /// Average round-trip time (exponential moving average)
    pub avg_rtt: Option<Duration>,
    /// Minimum observed RTT
    pub min_rtt: Option<Duration>,
    /// Maximum observed RTT
    pub max_rtt: Option<Duration>,
    /// Total pings sent
    pub pings_sent: u64,
    /// Total pongs received
    pub pongs_received: u64,
    /// Time of last activity (ping, pong, or any message)
    pub last_activity: Instant,
    /// Whether connection is considered healthy
    pub healthy: bool,
}

impl Default for ConnectionHealth {
    fn default() -> Self {
        Self {
            missed_pongs: 0,
            last_rtt: None,
            avg_rtt: None,
            min_rtt: None,
            max_rtt: None,
            pings_sent: 0,
            pongs_received: 0,
            last_activity: Instant::now(),
            healthy: true,
        }
    }
}

impl ConnectionHealth {
    /// Update RTT statistics with a new measurement
    fn update_rtt(&mut self, rtt: Duration) {
        self.last_rtt = Some(rtt);

        // Update min/max
        match self.min_rtt {
            Some(min) if rtt < min => self.min_rtt = Some(rtt),
            None => self.min_rtt = Some(rtt),
            _ => {}
        }
        match self.max_rtt {
            Some(max) if rtt > max => self.max_rtt = Some(rtt),
            None => self.max_rtt = Some(rtt),
            _ => {}
        }

        // Exponential moving average (alpha = 0.2)
        const ALPHA: f64 = 0.2;
        let rtt_nanos = rtt.as_nanos() as f64;
        let new_avg = match self.avg_rtt {
            Some(avg) => {
                let avg_nanos = avg.as_nanos() as f64;
                Duration::from_nanos((ALPHA * rtt_nanos + (1.0 - ALPHA) * avg_nanos) as u64)
            }
            None => rtt,
        };
        self.avg_rtt = Some(new_avg);
    }
}

/// State tracking for a single connection's heartbeat
#[derive(Debug)]
struct ConnectionState {
    /// Health metrics
    health: ConnectionHealth,
    /// Current pending ping sequence number (if any)
    pending_ping_seq: Option<u64>,
    /// Time the pending ping was sent
    pending_ping_sent: Option<Instant>,
}

impl ConnectionState {
    fn new() -> Self {
        Self {
            health: ConnectionHealth::default(),
            pending_ping_seq: None,
            pending_ping_sent: None,
        }
    }
}

/// Events emitted by the heartbeat monitor for observability
#[derive(Clone, Debug)]
pub enum HeartbeatEvent {
    /// A ping was sent to a connection
    PingSent {
        /// The connection that received the ping
        connection_id: ConnectionId,
        /// Sequence number of the ping
        seq: u64,
    },
    /// A pong was received from a connection
    PongReceived {
        /// The connection that sent the pong
        connection_id: ConnectionId,
        /// Sequence number of the pong
        seq: u64,
        /// Round-trip time for this ping/pong cycle
        rtt: Duration,
    },
    /// A pong was missed (timeout expired)
    PongMissed {
        /// The connection that missed the pong
        connection_id: ConnectionId,
        /// Sequence number of the missed ping
        seq: u64,
        /// Total number of consecutive missed pongs
        missed_count: u32,
    },
    /// A connection timed out and will be disconnected
    ConnectionTimeout {
        /// The connection that timed out
        connection_id: ConnectionId,
        /// Total number of consecutive missed pongs
        missed_count: u32,
    },
    /// Connection health changed
    HealthChanged {
        /// The connection whose health changed
        connection_id: ConnectionId,
        /// Whether the connection is now healthy
        healthy: bool,
        /// Current average round-trip time
        avg_rtt: Option<Duration>,
    },
}

/// Heartbeat monitor that tracks connection health
///
/// Thread-safe monitor that can be shared across tasks.
#[derive(Clone)]
pub struct HeartbeatMonitor {
    /// Configuration
    config: HeartbeatConfig,
    /// Per-connection state
    connections: Arc<RwLock<HashMap<ConnectionId, ConnectionState>>>,
    /// Global sequence counter for pings
    seq_counter: Arc<AtomicU64>,
    /// Channel for emitting health events
    event_tx: Option<mpsc::UnboundedSender<HeartbeatEvent>>,
    /// Flag to stop the monitor
    stopped: Arc<AtomicBool>,
}

impl HeartbeatMonitor {
    /// Create a new heartbeat monitor with the given config
    #[must_use]
    pub fn new(config: HeartbeatConfig) -> Self {
        Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            seq_counter: Arc::new(AtomicU64::new(1)),
            event_tx: None,
            stopped: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a monitor with an event channel for observability
    #[must_use]
    pub fn with_events(config: HeartbeatConfig) -> (Self, mpsc::UnboundedReceiver<HeartbeatEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let monitor = Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            seq_counter: Arc::new(AtomicU64::new(1)),
            event_tx: Some(tx),
            stopped: Arc::new(AtomicBool::new(false)),
        };
        (monitor, rx)
    }

    /// Get the heartbeat configuration
    #[must_use]
    pub fn config(&self) -> &HeartbeatConfig {
        &self.config
    }

    /// Check if heartbeat is enabled
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Register a new connection for monitoring
    pub fn register(&self, connection_id: ConnectionId) {
        if !self.config.enabled {
            return;
        }
        let mut connections = self.connections.write();
        connections.insert(connection_id, ConnectionState::new());
        tracing::debug!(
            connection_id = %connection_id,
            "Registered connection for heartbeat monitoring"
        );
    }

    /// Unregister a connection (e.g., when it disconnects)
    pub fn unregister(&self, connection_id: &ConnectionId) {
        let mut connections = self.connections.write();
        if connections.remove(connection_id).is_some() {
            tracing::debug!(
                connection_id = %connection_id,
                "Unregistered connection from heartbeat monitoring"
            );
        }
    }

    /// Record that a pong was received for a connection
    ///
    /// Returns true if the pong matched a pending ping.
    pub fn record_pong(&self, connection_id: &ConnectionId, seq: u64) -> bool {
        if !self.config.enabled {
            return false;
        }

        let mut connections = self.connections.write();
        let Some(state) = connections.get_mut(connection_id) else {
            tracing::warn!(
                connection_id = %connection_id,
                seq = seq,
                "Received pong for unknown connection"
            );
            return false;
        };

        // Check if this matches our pending ping
        if state.pending_ping_seq != Some(seq) {
            tracing::warn!(
                connection_id = %connection_id,
                expected_seq = ?state.pending_ping_seq,
                received_seq = seq,
                "Received pong with unexpected sequence number"
            );
            return false;
        }

        // Calculate RTT
        let rtt = state
            .pending_ping_sent
            .map(|sent| sent.elapsed())
            .unwrap_or_default();

        // Update health metrics
        state.health.pongs_received += 1;
        state.health.missed_pongs = 0;
        state.health.healthy = true;
        state.health.last_activity = Instant::now();
        state.health.update_rtt(rtt);

        // Clear pending state
        state.pending_ping_seq = None;
        state.pending_ping_sent = None;

        tracing::trace!(
            connection_id = %connection_id,
            seq = seq,
            rtt_ms = rtt.as_millis(),
            "Pong received"
        );

        // Emit event
        self.emit_event(HeartbeatEvent::PongReceived {
            connection_id: *connection_id,
            seq,
            rtt,
        });

        true
    }

    /// Record activity on a connection (resets the timer)
    ///
    /// Call this when any message is received from a surface, not just pongs.
    /// This prevents sending pings during active message exchange.
    pub fn record_activity(&self, connection_id: &ConnectionId) {
        if !self.config.enabled {
            return;
        }

        let mut connections = self.connections.write();
        if let Some(state) = connections.get_mut(connection_id) {
            state.health.last_activity = Instant::now();
        }
    }

    /// Get health metrics for a connection
    #[must_use]
    pub fn get_health(&self, connection_id: &ConnectionId) -> Option<ConnectionHealth> {
        let connections = self.connections.read();
        connections.get(connection_id).map(|s| s.health.clone())
    }

    /// Get health metrics for all connections
    #[must_use]
    pub fn get_all_health(&self) -> HashMap<ConnectionId, ConnectionHealth> {
        let connections = self.connections.read();
        connections
            .iter()
            .map(|(id, state)| (*id, state.health.clone()))
            .collect()
    }

    /// Check if a connection is healthy
    #[must_use]
    pub fn is_healthy(&self, connection_id: &ConnectionId) -> bool {
        let connections = self.connections.read();
        connections
            .get(connection_id)
            .is_some_and(|s| s.health.healthy)
    }

    /// Get the number of monitored connections
    #[must_use]
    pub fn connection_count(&self) -> usize {
        self.connections.read().len()
    }

    /// Stop the monitor
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }

    /// Check if the monitor is stopped
    #[must_use]
    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    /// Get the next sequence number for a ping
    fn next_seq(&self) -> u64 {
        self.seq_counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Emit a heartbeat event (if event channel is configured)
    fn emit_event(&self, event: HeartbeatEvent) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(event);
        }
    }

    /// Internal: prepare a ping for a connection
    ///
    /// Returns the sequence number if a ping should be sent.
    fn prepare_ping(&self, connection_id: &ConnectionId) -> Option<u64> {
        self.prepare_ping_internal(connection_id, false)
    }

    /// Force a ping to be prepared, bypassing the heartbeat interval check.
    ///
    /// This is useful for testing. In production, use the normal heartbeat task.
    #[cfg(test)]
    pub fn force_prepare_ping(&self, connection_id: &ConnectionId) -> Option<u64> {
        self.prepare_ping_internal(connection_id, true)
    }

    /// Internal implementation of `prepare_ping`
    fn prepare_ping_internal(&self, connection_id: &ConnectionId, force: bool) -> Option<u64> {
        let mut connections = self.connections.write();
        let state = connections.get_mut(connection_id)?;

        // Don't send a new ping if one is already pending
        if state.pending_ping_seq.is_some() {
            return None;
        }

        // Check if enough time has passed since last activity (unless forced)
        if !force {
            let since_activity = state.health.last_activity.elapsed();
            if since_activity < self.config.heartbeat_interval {
                return None;
            }
        }

        let seq = self.next_seq();
        state.pending_ping_seq = Some(seq);
        state.pending_ping_sent = Some(Instant::now());
        state.health.pings_sent += 1;

        Some(seq)
    }

    /// Internal: check for timed out pings
    ///
    /// Returns a list of (`connection_id`, `should_disconnect`) pairs for connections
    /// that have timed out pings.
    fn check_timeouts(&self) -> Vec<(ConnectionId, bool)> {
        let mut results = Vec::new();
        let mut connections = self.connections.write();

        for (id, state) in connections.iter_mut() {
            // Skip if no pending ping
            let Some(sent_time) = state.pending_ping_sent else {
                continue;
            };

            // Check if timed out
            if sent_time.elapsed() < self.config.response_timeout {
                continue;
            }

            // Missed a pong
            let missed_seq = state.pending_ping_seq.unwrap_or(0);
            state.pending_ping_seq = None;
            state.pending_ping_sent = None;
            state.health.missed_pongs += 1;

            let should_disconnect = state.health.missed_pongs >= self.config.max_missed_pongs;

            if should_disconnect {
                state.health.healthy = false;
                tracing::warn!(
                    connection_id = %id,
                    missed_count = state.health.missed_pongs,
                    "Connection timed out - will disconnect"
                );
                self.emit_event(HeartbeatEvent::ConnectionTimeout {
                    connection_id: *id,
                    missed_count: state.health.missed_pongs,
                });
            } else {
                tracing::debug!(
                    connection_id = %id,
                    missed_count = state.health.missed_pongs,
                    max_missed = self.config.max_missed_pongs,
                    "Pong missed"
                );
                self.emit_event(HeartbeatEvent::PongMissed {
                    connection_id: *id,
                    seq: missed_seq,
                    missed_count: state.health.missed_pongs,
                });
            }

            results.push((*id, should_disconnect));
        }

        results
    }
}

impl std::fmt::Debug for HeartbeatMonitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeartbeatMonitor")
            .field("config", &self.config)
            .field("connection_count", &self.connection_count())
            .field("stopped", &self.is_stopped())
            .finish()
    }
}

/// Async task that runs the heartbeat protocol
///
/// This task periodically:
/// 1. Sends pings to connections that need them
/// 2. Checks for timed out pings
/// 3. Disconnects unhealthy connections via the registry
pub struct HeartbeatTask {
    monitor: HeartbeatMonitor,
    registry: SurfaceRegistry,
    /// Tick interval for checking (smaller than heartbeat interval for responsiveness)
    tick_interval: Duration,
}

impl HeartbeatTask {
    /// Create a new heartbeat task
    #[must_use]
    pub fn new(monitor: HeartbeatMonitor, registry: SurfaceRegistry) -> Self {
        // Check every 1/4 of the response timeout for responsiveness
        let tick_interval = monitor.config.response_timeout / 4;
        // Ensure tick interval is at least 10ms
        let tick_interval = tick_interval.max(Duration::from_millis(10));

        Self {
            monitor,
            registry,
            tick_interval,
        }
    }

    /// Create a task with a custom tick interval
    #[must_use]
    pub fn with_tick_interval(
        monitor: HeartbeatMonitor,
        registry: SurfaceRegistry,
        tick_interval: Duration,
    ) -> Self {
        Self {
            monitor,
            registry,
            tick_interval,
        }
    }

    /// Run the heartbeat task
    ///
    /// This runs until the monitor is stopped or the task is cancelled.
    pub async fn run(self) {
        if !self.monitor.is_enabled() {
            tracing::info!("Heartbeat monitoring disabled");
            return;
        }

        tracing::info!(
            interval_secs = self.monitor.config.heartbeat_interval.as_secs(),
            timeout_secs = self.monitor.config.response_timeout.as_secs(),
            max_missed = self.monitor.config.max_missed_pongs,
            "Starting heartbeat task"
        );

        let mut interval = tokio::time::interval(self.tick_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            if self.monitor.is_stopped() {
                tracing::info!("Heartbeat task stopped");
                break;
            }

            // Check for timeouts and get connections that need action
            let timeouts = self.monitor.check_timeouts();

            // Handle disconnections
            for (connection_id, should_disconnect) in timeouts {
                if should_disconnect {
                    tracing::info!(
                        connection_id = %connection_id,
                        "Disconnecting unhealthy connection"
                    );
                    self.registry.unregister(&connection_id);
                    self.monitor.unregister(&connection_id);
                }
            }

            // Send pings to connections that need them
            let connection_ids = self.registry.connection_ids();
            for connection_id in connection_ids {
                if let Some(seq) = self.monitor.prepare_ping(&connection_id) {
                    let ping = ConductorMessage::Ping { seq };
                    if self.registry.send_to(&connection_id, ping) {
                        tracing::trace!(
                            connection_id = %connection_id,
                            seq = seq,
                            "Ping sent"
                        );
                        self.monitor
                            .emit_event(HeartbeatEvent::PingSent { connection_id, seq });
                    } else {
                        tracing::warn!(
                            connection_id = %connection_id,
                            "Failed to send ping - channel closed"
                        );
                    }
                }
            }
        }
    }

    /// Get a reference to the monitor
    #[must_use]
    pub fn monitor(&self) -> &HeartbeatMonitor {
        &self.monitor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    use crate::events::{SurfaceCapabilities, SurfaceType};
    use crate::surface_registry::SurfaceHandle;

    fn create_test_registry_with_connection() -> (
        SurfaceRegistry,
        ConnectionId,
        mpsc::Receiver<ConductorMessage>,
    ) {
        let registry = SurfaceRegistry::new();
        let id = ConnectionId::new();
        let (tx, rx) = mpsc::channel(32);
        let handle = SurfaceHandle::new(
            id.clone(),
            tx,
            SurfaceType::Headless,
            SurfaceCapabilities::headless(),
        );
        registry.register(handle);
        (registry, id, rx)
    }

    #[test]
    fn test_heartbeat_config_default() {
        let config = HeartbeatConfig::default();
        assert_eq!(config.heartbeat_interval, Duration::from_secs(30));
        assert_eq!(config.response_timeout, Duration::from_secs(10));
        assert_eq!(config.max_missed_pongs, 3);
        assert!(config.enabled);
    }

    #[test]
    fn test_heartbeat_config_disabled() {
        let config = HeartbeatConfig::disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn test_heartbeat_config_for_testing() {
        let config = HeartbeatConfig::for_testing();
        assert_eq!(config.heartbeat_interval, Duration::from_millis(100));
        assert_eq!(config.response_timeout, Duration::from_millis(50));
        assert_eq!(config.max_missed_pongs, 2);
        assert!(config.enabled);
    }

    #[test]
    fn test_heartbeat_config_builder() {
        let config = HeartbeatConfig::new()
            .with_interval(Duration::from_secs(60))
            .with_timeout(Duration::from_secs(20))
            .with_max_missed(5);

        assert_eq!(config.heartbeat_interval, Duration::from_secs(60));
        assert_eq!(config.response_timeout, Duration::from_secs(20));
        assert_eq!(config.max_missed_pongs, 5);
    }

    #[test]
    fn test_monitor_register_unregister() {
        let config = HeartbeatConfig::for_testing();
        let monitor = HeartbeatMonitor::new(config);
        let id = ConnectionId::new();

        assert_eq!(monitor.connection_count(), 0);

        monitor.register(id.clone());
        assert_eq!(monitor.connection_count(), 1);
        assert!(monitor.is_healthy(&id));

        monitor.unregister(&id);
        assert_eq!(monitor.connection_count(), 0);
    }

    #[test]
    fn test_monitor_disabled_no_registration() {
        let config = HeartbeatConfig::disabled();
        let monitor = HeartbeatMonitor::new(config);
        let id = ConnectionId::new();

        monitor.register(id.clone());
        // When disabled, connections are not tracked
        assert_eq!(monitor.connection_count(), 0);
    }

    #[test]
    fn test_monitor_record_pong() {
        let config = HeartbeatConfig::for_testing();
        let monitor = HeartbeatMonitor::new(config);
        let id = ConnectionId::new();

        monitor.register(id.clone());

        // Force a ping (bypasses heartbeat interval check for testing)
        let seq = monitor.force_prepare_ping(&id);
        assert!(seq.is_some());
        let seq = seq.unwrap();

        // Record the pong
        let result = monitor.record_pong(&id, seq);
        assert!(result);

        // Check health was updated
        let health = monitor.get_health(&id).unwrap();
        assert_eq!(health.pongs_received, 1);
        assert_eq!(health.missed_pongs, 0);
        assert!(health.last_rtt.is_some());
    }

    #[test]
    fn test_monitor_record_pong_wrong_seq() {
        let config = HeartbeatConfig::for_testing();
        let monitor = HeartbeatMonitor::new(config);
        let id = ConnectionId::new();

        monitor.register(id.clone());

        // Force a ping (bypasses heartbeat interval check for testing)
        let seq = monitor.force_prepare_ping(&id).unwrap();

        // Record a pong with wrong seq
        let result = monitor.record_pong(&id, seq + 100);
        assert!(!result);
    }

    #[test]
    fn test_monitor_record_activity() {
        let config = HeartbeatConfig::for_testing();
        let monitor = HeartbeatMonitor::new(config);
        let id = ConnectionId::new();

        monitor.register(id.clone());

        // Get initial activity time
        let health_before = monitor.get_health(&id).unwrap();
        let activity_before = health_before.last_activity;

        // Small delay
        std::thread::sleep(Duration::from_millis(10));

        // Record activity
        monitor.record_activity(&id);

        // Check activity time was updated
        let health_after = monitor.get_health(&id).unwrap();
        assert!(health_after.last_activity > activity_before);
    }

    #[test]
    fn test_connection_health_rtt_stats() {
        let mut health = ConnectionHealth::default();

        // First RTT
        health.update_rtt(Duration::from_millis(100));
        assert_eq!(health.last_rtt, Some(Duration::from_millis(100)));
        assert_eq!(health.min_rtt, Some(Duration::from_millis(100)));
        assert_eq!(health.max_rtt, Some(Duration::from_millis(100)));
        assert!(health.avg_rtt.is_some());

        // Faster RTT
        health.update_rtt(Duration::from_millis(50));
        assert_eq!(health.last_rtt, Some(Duration::from_millis(50)));
        assert_eq!(health.min_rtt, Some(Duration::from_millis(50)));
        assert_eq!(health.max_rtt, Some(Duration::from_millis(100)));

        // Slower RTT
        health.update_rtt(Duration::from_millis(200));
        assert_eq!(health.last_rtt, Some(Duration::from_millis(200)));
        assert_eq!(health.min_rtt, Some(Duration::from_millis(50)));
        assert_eq!(health.max_rtt, Some(Duration::from_millis(200)));
    }

    #[test]
    fn test_monitor_with_events() {
        let config = HeartbeatConfig::for_testing();
        let (monitor, mut rx) = HeartbeatMonitor::with_events(config);
        let id = ConnectionId::new();

        monitor.register(id.clone());

        // Force a ping/pong cycle (bypasses heartbeat interval check for testing)
        let seq = monitor.force_prepare_ping(&id).unwrap();
        monitor.record_pong(&id, seq);

        // Should have received a PongReceived event
        let event = rx.try_recv().unwrap();
        matches!(event, HeartbeatEvent::PongReceived { .. });
    }

    #[test]
    fn test_monitor_stop() {
        let config = HeartbeatConfig::for_testing();
        let monitor = HeartbeatMonitor::new(config);

        assert!(!monitor.is_stopped());
        monitor.stop();
        assert!(monitor.is_stopped());
    }

    #[tokio::test]
    async fn test_healthy_connection_stays_connected() {
        let config = HeartbeatConfig::for_testing();
        let (monitor, mut event_rx) = HeartbeatMonitor::with_events(config);
        let (registry, connection_id, mut msg_rx) = create_test_registry_with_connection();

        // Register for heartbeat monitoring
        monitor.register(connection_id.clone());

        // Create and spawn the heartbeat task
        let task = HeartbeatTask::new(monitor.clone(), registry.clone());
        let task_handle = tokio::spawn(task.run());

        // Give time for the first ping to be sent
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should have received a ping
        if let Some(msg) = msg_rx.recv().await {
            if let ConductorMessage::Ping { seq } = msg {
                // Respond with pong
                monitor.record_pong(&connection_id, seq);
            }
        }

        // Connection should still be registered
        assert!(registry.contains(&connection_id));
        assert!(monitor.is_healthy(&connection_id));

        // Stop the task
        monitor.stop();
        task_handle.await.unwrap();

        // Check we received events
        let mut ping_sent = false;
        let mut pong_received = false;
        while let Ok(event) = event_rx.try_recv() {
            match event {
                HeartbeatEvent::PingSent { .. } => ping_sent = true,
                HeartbeatEvent::PongReceived { .. } => pong_received = true,
                _ => {}
            }
        }
        assert!(ping_sent);
        assert!(pong_received);
    }

    #[tokio::test]
    async fn test_unresponsive_connection_gets_disconnected() {
        let config = HeartbeatConfig::for_testing();
        let (monitor, mut event_rx) = HeartbeatMonitor::with_events(config);
        let (registry, connection_id, _msg_rx) = create_test_registry_with_connection();

        // Register for heartbeat monitoring
        monitor.register(connection_id.clone());

        // Create and spawn the heartbeat task
        let task = HeartbeatTask::new(monitor.clone(), registry.clone());
        let task_handle = tokio::spawn(task.run());

        // Wait long enough for heartbeat interval + multiple timeouts
        // Config: 100ms interval, 50ms timeout, 2 max missed
        // So we need to wait for: 100ms + 50ms + 100ms + 50ms + some buffer
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Stop the task
        monitor.stop();
        task_handle.await.unwrap();

        // Connection should have been unregistered due to timeout
        assert!(
            !registry.contains(&connection_id),
            "Unresponsive connection should be disconnected"
        );

        // Check we received timeout event
        let mut timeout_received = false;
        while let Ok(event) = event_rx.try_recv() {
            if matches!(event, HeartbeatEvent::ConnectionTimeout { .. }) {
                timeout_received = true;
            }
        }
        assert!(timeout_received, "Should have received timeout event");
    }

    #[tokio::test]
    async fn test_timer_reset_on_activity() {
        let config = HeartbeatConfig::for_testing();
        let monitor = HeartbeatMonitor::new(config);
        let (registry, connection_id, _msg_rx) = create_test_registry_with_connection();

        monitor.register(connection_id.clone());

        // Get initial state
        let health_before = monitor.get_health(&connection_id).unwrap();

        // Record activity (simulating any message received)
        std::thread::sleep(Duration::from_millis(10));
        monitor.record_activity(&connection_id);

        // Check that activity time was updated
        let health_after = monitor.get_health(&connection_id).unwrap();
        assert!(health_after.last_activity > health_before.last_activity);

        // prepare_ping should return None because not enough time has passed
        assert!(monitor.prepare_ping(&connection_id).is_none());

        // Connection should remain registered
        assert!(registry.contains(&connection_id));
    }

    #[test]
    fn test_get_all_health() {
        let config = HeartbeatConfig::for_testing();
        let monitor = HeartbeatMonitor::new(config);

        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();
        let id3 = ConnectionId::new();

        monitor.register(id1.clone());
        monitor.register(id2.clone());
        monitor.register(id3.clone());

        let all_health = monitor.get_all_health();
        assert_eq!(all_health.len(), 3);
        assert!(all_health.contains_key(&id1));
        assert!(all_health.contains_key(&id2));
        assert!(all_health.contains_key(&id3));
    }

    #[test]
    fn test_check_timeouts_increments_missed() {
        let config = HeartbeatConfig::for_testing();
        let monitor = HeartbeatMonitor::new(config.clone());
        let id = ConnectionId::new();

        monitor.register(id.clone());

        // Force a ping (bypasses heartbeat interval check for testing)
        let _seq = monitor.force_prepare_ping(&id).unwrap();

        // Wait for timeout
        std::thread::sleep(config.response_timeout + Duration::from_millis(10));

        // Check timeouts
        let timeouts = monitor.check_timeouts();

        // Should have one timeout, not disconnected yet (missed = 1, max = 2)
        assert_eq!(timeouts.len(), 1);
        assert_eq!(timeouts[0].0, id);
        assert!(!timeouts[0].1); // should_disconnect = false

        // Check missed count
        let health = monitor.get_health(&id).unwrap();
        assert_eq!(health.missed_pongs, 1);
    }
}
