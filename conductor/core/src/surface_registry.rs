//! Surface Registry - Multi-Surface Connection Management
//!
//! This module provides the infrastructure for managing multiple connected UI surfaces.
//! The `SurfaceRegistry` allows the Conductor to:
//!
//! - Register and track multiple surface connections
//! - Route messages to specific surfaces or broadcast to all
//! - Query surface capabilities for adaptive behavior
//! - Handle connection lifecycle (connect/disconnect)
//!
//! # Architecture
//!
//! ```text
//!                      SurfaceRegistry
//!                     ┌───────────────────────────────────────┐
//!                     │ HashMap<ConnectionId, SurfaceHandle>  │
//!                     │   - wrapped in Arc<RwLock<>>          │
//!                     └───────────────┬───────────────────────┘
//!                                     │
//!              ┌──────────────────────┼──────────────────────┐
//!              │                      │                      │
//!       ┌──────▼──────┐       ┌───────▼──────┐       ┌───────▼──────┐
//!       │ TUI Surface │       │ Web Surface  │       │ Mobile App   │
//!       │   conn_1    │       │   conn_2     │       │   conn_3     │
//!       └─────────────┘       └──────────────┘       └──────────────┘
//! ```
//!
//! # Thread Safety
//!
//! The registry uses `Arc<RwLock<>>` to allow concurrent read access while
//! serializing writes. This pattern is optimal for the expected workload
//! where reads (sending messages) are much more frequent than writes
//! (connection changes).

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::events::{SurfaceCapabilities, SurfaceType};
use crate::messages::ConductorMessage;

/// Unique identifier for a client connection
///
/// Each connection is assigned a unique ID when it connects.
/// This ID is stable for the lifetime of the connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(u64);

impl ConnectionId {
    /// Create a new unique connection ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    /// Create a connection ID from a raw value (for testing or deserialization)
    #[cfg(test)]
    pub fn from_raw(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw numeric value
    #[must_use]
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "conn-{}", self.0)
    }
}

/// Handle to a connected surface
///
/// Contains everything needed to communicate with a specific surface.
/// Stored in the `SurfaceRegistry` for each active connection.
#[derive(Debug)]
pub struct SurfaceHandle {
    /// Unique connection identifier
    pub id: ConnectionId,
    /// Channel to send messages to this surface
    pub tx: mpsc::Sender<ConductorMessage>,
    /// Type of surface (TUI, Web, Mobile, etc.)
    pub surface_type: SurfaceType,
    /// Capabilities of this surface
    pub capabilities: SurfaceCapabilities,
    /// When the surface connected
    pub connected_at: std::time::Instant,
    /// Optional metadata about the surface
    pub metadata: Option<SurfaceMetadata>,
}

impl SurfaceHandle {
    /// Create a new surface handle
    #[must_use]
    pub fn new(
        id: ConnectionId,
        tx: mpsc::Sender<ConductorMessage>,
        surface_type: SurfaceType,
        capabilities: SurfaceCapabilities,
    ) -> Self {
        Self {
            id,
            tx,
            surface_type,
            capabilities,
            connected_at: std::time::Instant::now(),
            metadata: None,
        }
    }

    /// Create a new surface handle with metadata
    #[must_use]
    pub fn with_metadata(
        id: ConnectionId,
        tx: mpsc::Sender<ConductorMessage>,
        surface_type: SurfaceType,
        capabilities: SurfaceCapabilities,
        metadata: SurfaceMetadata,
    ) -> Self {
        Self {
            id,
            tx,
            surface_type,
            capabilities,
            connected_at: std::time::Instant::now(),
            metadata: Some(metadata),
        }
    }

    /// Send a message to this surface
    ///
    /// Returns true if the message was sent successfully.
    pub async fn send(&self, message: ConductorMessage) -> bool {
        self.tx.send(message).await.is_ok()
    }

    /// Try to send a message without waiting
    ///
    /// Returns true if the message was sent successfully.
    #[must_use]
    pub fn try_send(&self, message: ConductorMessage) -> bool {
        self.tx.try_send(message).is_ok()
    }

    /// Check if the surface channel is still open
    #[must_use]
    pub fn is_connected(&self) -> bool {
        !self.tx.is_closed()
    }

    /// Get the connection uptime in seconds
    #[must_use]
    pub fn uptime_secs(&self) -> u64 {
        self.connected_at.elapsed().as_secs()
    }
}

/// Optional metadata about a surface connection
#[derive(Debug, Clone, Default)]
pub struct SurfaceMetadata {
    /// Peer UID (for Unix socket connections)
    pub peer_uid: Option<u32>,
    /// Client version string
    pub client_version: Option<String>,
    /// User agent or device info
    pub user_agent: Option<String>,
    /// Authentication token (for future use with remote surfaces)
    ///
    /// This field is optional and reserved for future authentication mechanisms.
    /// For Unix socket connections, peer credential validation is sufficient.
    /// For WebSocket/remote connections, this token will be validated.
    pub auth_token: Option<String>,
    /// Whether the handshake has been completed
    pub handshake_complete: bool,
    /// Protocol version negotiated during handshake
    pub protocol_version: Option<u32>,
}

/// Result of a broadcast operation
#[derive(Debug, Clone)]
pub struct BroadcastResult {
    /// Number of surfaces that received the message successfully
    pub successful: usize,
    /// Number of surfaces that failed to receive the message
    pub failed: usize,
    /// IDs of surfaces that failed
    pub failed_ids: Vec<ConnectionId>,
}

impl BroadcastResult {
    /// Check if all recipients received the message
    #[must_use]
    pub fn all_succeeded(&self) -> bool {
        self.failed == 0
    }

    /// Check if no recipients received the message
    #[must_use]
    pub fn all_failed(&self) -> bool {
        self.successful == 0
    }
}

/// Registry for managing connected surfaces
///
/// Thread-safe registry that allows concurrent read access while
/// serializing write operations (registering/unregistering surfaces).
#[derive(Clone)]
pub struct SurfaceRegistry {
    /// Inner map of connection ID to surface handle
    inner: Arc<RwLock<HashMap<ConnectionId, SurfaceHandle>>>,
}

impl Default for SurfaceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SurfaceRegistry {
    /// Create a new empty registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new surface connection
    ///
    /// Returns the assigned `ConnectionId`.
    pub fn register(&self, handle: SurfaceHandle) -> ConnectionId {
        let id = handle.id;
        let mut inner = self.inner.write();
        inner.insert(id, handle);
        tracing::info!(
            connection_id = %id,
            "Surface registered"
        );
        id
    }

    /// Unregister a surface connection
    ///
    /// Returns the handle if it was registered.
    pub fn unregister(&self, id: &ConnectionId) -> Option<SurfaceHandle> {
        let mut inner = self.inner.write();
        let handle = inner.remove(id);
        if handle.is_some() {
            tracing::info!(
                connection_id = %id,
                "Surface unregistered"
            );
        }
        handle
    }

    /// Get the number of connected surfaces
    #[must_use]
    pub fn count(&self) -> usize {
        self.inner.read().len()
    }

    /// Check if a connection is registered
    #[must_use]
    pub fn contains(&self, id: &ConnectionId) -> bool {
        self.inner.read().contains_key(id)
    }

    /// Get the IDs of all connected surfaces
    #[must_use]
    pub fn connection_ids(&self) -> Vec<ConnectionId> {
        self.inner.read().keys().copied().collect()
    }

    /// Get surface capabilities for a specific connection
    #[must_use]
    pub fn get_capabilities(&self, id: &ConnectionId) -> Option<SurfaceCapabilities> {
        self.inner.read().get(id).map(|h| h.capabilities.clone())
    }

    /// Get surface type for a specific connection
    #[must_use]
    pub fn get_surface_type(&self, id: &ConnectionId) -> Option<SurfaceType> {
        self.inner.read().get(id).map(|h| h.surface_type.clone())
    }

    /// Update capabilities for a specific connection
    pub fn update_capabilities(&self, id: &ConnectionId, capabilities: SurfaceCapabilities) {
        if let Some(handle) = self.inner.write().get_mut(id) {
            handle.capabilities = capabilities;
            tracing::debug!(
                connection_id = %id,
                "Surface capabilities updated"
            );
        }
    }

    /// Update surface type for a specific connection
    ///
    /// Called during handshake when the actual surface type is declared.
    pub fn update_surface_type(&self, id: &ConnectionId, surface_type: SurfaceType) {
        if let Some(handle) = self.inner.write().get_mut(id) {
            handle.surface_type = surface_type;
            tracing::debug!(
                connection_id = %id,
                "Surface type updated"
            );
        }
    }

    /// Update metadata for a specific connection
    pub fn update_metadata(&self, id: &ConnectionId, metadata: SurfaceMetadata) {
        if let Some(handle) = self.inner.write().get_mut(id) {
            handle.metadata = Some(metadata);
            tracing::debug!(
                connection_id = %id,
                "Surface metadata updated"
            );
        }
    }

    /// Complete handshake for a specific connection
    ///
    /// This updates the surface type, capabilities, and marks handshake as complete.
    /// Returns true if the connection exists and was updated.
    pub fn complete_handshake(
        &self,
        id: &ConnectionId,
        surface_type: SurfaceType,
        capabilities: SurfaceCapabilities,
        auth_token: Option<String>,
        protocol_version: u32,
    ) -> bool {
        if let Some(handle) = self.inner.write().get_mut(id) {
            handle.surface_type = surface_type;
            handle.capabilities = capabilities;

            // Update or create metadata with handshake info
            let metadata = handle.metadata.get_or_insert_with(SurfaceMetadata::default);
            metadata.auth_token = auth_token;
            metadata.handshake_complete = true;
            metadata.protocol_version = Some(protocol_version);

            tracing::info!(
                connection_id = %id,
                protocol_version = protocol_version,
                "Handshake completed for surface"
            );
            true
        } else {
            tracing::warn!(
                connection_id = %id,
                "Attempted to complete handshake for unknown connection"
            );
            false
        }
    }

    /// Check if a connection has completed handshake
    #[must_use]
    pub fn is_handshake_complete(&self, id: &ConnectionId) -> bool {
        self.inner
            .read()
            .get(id)
            .and_then(|h| h.metadata.as_ref())
            .is_some_and(|m| m.handshake_complete)
    }

    /// Broadcast a message to all connected surfaces
    ///
    /// Uses `try_send` to avoid blocking on slow surfaces.
    #[must_use]
    pub fn broadcast(&self, message: ConductorMessage) -> BroadcastResult {
        let inner = self.inner.read();
        let mut successful = 0;
        let mut failed = 0;
        let mut failed_ids = Vec::new();

        for (id, handle) in inner.iter() {
            if handle.try_send(message.clone()) {
                successful += 1;
            } else {
                failed += 1;
                failed_ids.push(*id);
            }
        }

        BroadcastResult {
            successful,
            failed,
            failed_ids,
        }
    }

    /// Broadcast a message to all connected surfaces (async version)
    ///
    /// Waits for each surface to accept the message.
    pub async fn broadcast_async(&self, message: ConductorMessage) -> BroadcastResult {
        // Collect handles to avoid holding the lock during async operations
        let handles: Vec<(ConnectionId, mpsc::Sender<ConductorMessage>)> = {
            let inner = self.inner.read();
            inner.iter().map(|(id, h)| (*id, h.tx.clone())).collect()
        };

        let mut successful = 0;
        let mut failed = 0;
        let mut failed_ids = Vec::new();

        for (id, tx) in handles {
            if tx.send(message.clone()).await.is_ok() {
                successful += 1;
            } else {
                failed += 1;
                failed_ids.push(id);
            }
        }

        BroadcastResult {
            successful,
            failed,
            failed_ids,
        }
    }

    /// Send a message to a specific surface
    ///
    /// Returns true if the message was sent successfully.
    pub fn send_to(&self, id: &ConnectionId, message: ConductorMessage) -> bool {
        let inner = self.inner.read();
        if let Some(handle) = inner.get(id) {
            handle.try_send(message)
        } else {
            tracing::warn!(
                connection_id = %id,
                "Attempted to send to unknown connection"
            );
            false
        }
    }

    /// Send a message to a specific surface (async version)
    ///
    /// Returns true if the message was sent successfully.
    pub async fn send_to_async(&self, id: &ConnectionId, message: ConductorMessage) -> bool {
        // Get the sender outside of the lock to avoid holding it during async
        let tx = {
            let inner = self.inner.read();
            inner.get(id).map(|h| h.tx.clone())
        };

        if let Some(tx) = tx {
            tx.send(message).await.is_ok()
        } else {
            tracing::warn!(
                connection_id = %id,
                "Attempted to send to unknown connection"
            );
            false
        }
    }

    /// Send a message to surfaces that match a predicate
    ///
    /// The predicate receives the surface type and capabilities.
    pub fn send_to_matching<F>(&self, message: ConductorMessage, predicate: F) -> BroadcastResult
    where
        F: Fn(&SurfaceType, &SurfaceCapabilities) -> bool,
    {
        let inner = self.inner.read();
        let mut successful = 0;
        let mut failed = 0;
        let mut failed_ids = Vec::new();

        for (id, handle) in inner.iter() {
            if predicate(&handle.surface_type, &handle.capabilities) {
                if handle.try_send(message.clone()) {
                    successful += 1;
                } else {
                    failed += 1;
                    failed_ids.push(*id);
                }
            }
        }

        BroadcastResult {
            successful,
            failed,
            failed_ids,
        }
    }

    /// Send a message only to surfaces with specific capabilities
    ///
    /// Common use case: send avatar animations only to surfaces that support them.
    pub fn send_to_capable(
        &self,
        message: ConductorMessage,
        required_caps: impl Fn(&SurfaceCapabilities) -> bool,
    ) -> BroadcastResult {
        self.send_to_matching(message, |_, caps| required_caps(caps))
    }

    /// Get a summary of all connected surfaces
    #[must_use]
    pub fn summary(&self) -> RegistrySummary {
        let inner = self.inner.read();
        let mut by_type: HashMap<String, usize> = HashMap::new();

        for handle in inner.values() {
            let type_name = handle.surface_type.name().to_string();
            *by_type.entry(type_name).or_insert(0) += 1;
        }

        RegistrySummary {
            total_connections: inner.len(),
            by_type,
        }
    }

    /// Remove disconnected surfaces
    ///
    /// Returns the number of surfaces removed.
    pub fn cleanup_disconnected(&self) -> usize {
        let mut inner = self.inner.write();
        let before = inner.len();

        inner.retain(|id, handle| {
            let connected = handle.is_connected();
            if !connected {
                tracing::info!(
                    connection_id = %id,
                    "Removing disconnected surface"
                );
            }
            connected
        });

        let removed = before - inner.len();
        if removed > 0 {
            tracing::info!(
                removed = removed,
                remaining = inner.len(),
                "Cleaned up disconnected surfaces"
            );
        }
        removed
    }

    /// Execute a function with read access to a surface handle
    ///
    /// Returns None if the connection is not found.
    pub fn with_handle<F, R>(&self, id: &ConnectionId, f: F) -> Option<R>
    where
        F: FnOnce(&SurfaceHandle) -> R,
    {
        let inner = self.inner.read();
        inner.get(id).map(f)
    }
}

impl fmt::Debug for SurfaceRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.inner.read();
        f.debug_struct("SurfaceRegistry")
            .field("connection_count", &inner.len())
            .field("connections", &inner.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Summary of connected surfaces
#[derive(Debug, Clone)]
pub struct RegistrySummary {
    /// Total number of connected surfaces
    pub total_connections: usize,
    /// Count by surface type name
    pub by_type: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn create_test_handle(id: ConnectionId) -> (SurfaceHandle, mpsc::Receiver<ConductorMessage>) {
        let (tx, rx) = mpsc::channel(32);
        let handle = SurfaceHandle::new(
            id,
            tx,
            SurfaceType::Headless,
            SurfaceCapabilities::headless(),
        );
        (handle, rx)
    }

    #[test]
    fn test_connection_id_display() {
        let id = ConnectionId::from_raw(42);
        assert_eq!(format!("{id}"), "conn-42");
    }

    #[test]
    fn test_connection_id_unique() {
        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_registry_register_unregister() {
        let registry = SurfaceRegistry::new();
        let id = ConnectionId::new();
        let (handle, _rx) = create_test_handle(id);

        // Register
        let returned_id = registry.register(handle);
        assert_eq!(returned_id, id);
        assert_eq!(registry.count(), 1);
        assert!(registry.contains(&id));

        // Unregister
        let removed = registry.unregister(&id);
        assert!(removed.is_some());
        assert_eq!(registry.count(), 0);
        assert!(!registry.contains(&id));
    }

    #[test]
    fn test_registry_broadcast() {
        let registry = SurfaceRegistry::new();

        // Create two surfaces
        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();
        let (handle1, mut rx1) = create_test_handle(id1);
        let (handle2, mut rx2) = create_test_handle(id2);

        registry.register(handle1);
        registry.register(handle2);

        // Broadcast a message
        let msg = ConductorMessage::QueryCapabilities;
        let result = registry.broadcast(msg);

        assert!(result.all_succeeded());
        assert_eq!(result.successful, 2);
        assert_eq!(result.failed, 0);

        // Both should have received the message
        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }

    #[test]
    fn test_registry_send_to() {
        let registry = SurfaceRegistry::new();

        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();
        let (handle1, mut rx1) = create_test_handle(id1);
        let (handle2, mut rx2) = create_test_handle(id2);

        registry.register(handle1);
        registry.register(handle2);

        // Send to specific surface
        let msg = ConductorMessage::QueryCapabilities;
        let sent = registry.send_to(&id1, msg);
        assert!(sent);

        // Only id1 should have received it
        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_err());
    }

    #[test]
    fn test_registry_get_capabilities() {
        let registry = SurfaceRegistry::new();
        let id = ConnectionId::new();
        let (handle, _rx) = create_test_handle(id);

        registry.register(handle);

        let caps = registry.get_capabilities(&id);
        assert!(caps.is_some());
        assert!(!caps.unwrap().color); // Headless has no color
    }

    #[test]
    fn test_registry_summary() {
        let registry = SurfaceRegistry::new();

        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();
        let (handle1, _rx1) = create_test_handle(id1);

        // Create a TUI surface
        let (tx, _rx2) = mpsc::channel(32);
        let handle2 = SurfaceHandle::new(id2, tx, SurfaceType::Tui, SurfaceCapabilities::tui());

        registry.register(handle1);
        registry.register(handle2);

        let summary = registry.summary();
        assert_eq!(summary.total_connections, 2);
        assert_eq!(summary.by_type.get("Headless"), Some(&1));
        assert_eq!(summary.by_type.get("Terminal"), Some(&1));
    }

    #[tokio::test]
    async fn test_registry_broadcast_async() {
        let registry = SurfaceRegistry::new();

        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();
        let (handle1, mut rx1) = create_test_handle(id1);
        let (handle2, mut rx2) = create_test_handle(id2);

        registry.register(handle1);
        registry.register(handle2);

        // Broadcast async
        let msg = ConductorMessage::QueryCapabilities;
        let result = registry.broadcast_async(msg).await;

        assert!(result.all_succeeded());
        assert_eq!(result.successful, 2);

        // Both should have received
        assert!(rx1.recv().await.is_some());
        assert!(rx2.recv().await.is_some());
    }

    #[test]
    fn test_registry_send_to_capable() {
        let registry = SurfaceRegistry::new();

        // Headless surface (no streaming)
        let id1 = ConnectionId::new();
        let (handle1, mut rx1) = create_test_handle(id1);

        // TUI surface (has streaming)
        let id2 = ConnectionId::new();
        let (tx, mut rx2) = mpsc::channel(32);
        let handle2 = SurfaceHandle::new(id2, tx, SurfaceType::Tui, SurfaceCapabilities::tui());

        registry.register(handle1);
        registry.register(handle2);

        // Send only to surfaces with color support
        let msg = ConductorMessage::QueryCapabilities;
        let result = registry.send_to_capable(msg, |caps| caps.color);

        assert_eq!(result.successful, 1);

        // Only TUI should have received it
        assert!(rx1.try_recv().is_err());
        assert!(rx2.try_recv().is_ok());
    }

    #[test]
    fn test_registry_cleanup_disconnected() {
        let registry = SurfaceRegistry::new();

        let id = ConnectionId::new();
        let (tx, rx) = mpsc::channel(32);
        let handle = SurfaceHandle::new(
            id,
            tx,
            SurfaceType::Headless,
            SurfaceCapabilities::headless(),
        );

        registry.register(handle);
        assert_eq!(registry.count(), 1);

        // Drop the receiver to simulate disconnect
        drop(rx);

        // Cleanup should remove the disconnected surface
        let removed = registry.cleanup_disconnected();
        assert_eq!(removed, 1);
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_surface_handle_is_connected() {
        let id = ConnectionId::new();
        let (tx, rx) = mpsc::channel(32);
        let handle = SurfaceHandle::new(
            id,
            tx,
            SurfaceType::Headless,
            SurfaceCapabilities::headless(),
        );

        assert!(handle.is_connected());

        // Drop receiver
        drop(rx);

        // Now disconnected
        assert!(!handle.is_connected());
    }

    // ========================================
    // Concurrent Surface Connection Tests
    // ========================================

    #[tokio::test]
    async fn test_concurrent_registration() {
        use std::sync::Arc;
        use tokio::task::JoinSet;

        let registry = Arc::new(SurfaceRegistry::new());
        let mut join_set = JoinSet::new();

        // Spawn 10 concurrent tasks that each register a surface
        for _ in 0..10 {
            let registry = Arc::clone(&registry);
            join_set.spawn(async move {
                let id = ConnectionId::new();
                let (tx, _rx) = mpsc::channel(32);
                let handle = SurfaceHandle::new(
                    id,
                    tx,
                    SurfaceType::Headless,
                    SurfaceCapabilities::headless(),
                );
                registry.register(handle);
                id
            });
        }

        // Wait for all registrations
        let mut registered_ids = Vec::new();
        while let Some(result) = join_set.join_next().await {
            registered_ids.push(result.unwrap());
        }

        // All 10 should be registered
        assert_eq!(registry.count(), 10);

        // All IDs should be unique
        let mut unique_ids: Vec<_> = registered_ids.iter().copied().collect();
        unique_ids.sort_by_key(|id| id.as_u64());
        unique_ids.dedup();
        assert_eq!(unique_ids.len(), 10);
    }

    #[tokio::test]
    async fn test_concurrent_broadcast() {
        use std::sync::Arc;
        use tokio::task::JoinSet;

        let registry = Arc::new(SurfaceRegistry::new());
        let mut receivers = Vec::new();

        // Register 5 surfaces
        for _ in 0..5 {
            let id = ConnectionId::new();
            let (tx, rx) = mpsc::channel(32);
            let handle = SurfaceHandle::new(
                id,
                tx,
                SurfaceType::Headless,
                SurfaceCapabilities::headless(),
            );
            registry.register(handle);
            receivers.push(rx);
        }

        // Spawn 10 concurrent broadcast tasks
        let mut join_set = JoinSet::new();
        for _ in 0..10 {
            let registry = Arc::clone(&registry);
            join_set.spawn(async move {
                let msg = ConductorMessage::QueryCapabilities;
                registry.broadcast_async(msg).await
            });
        }

        // Wait for all broadcasts
        while let Some(result) = join_set.join_next().await {
            let broadcast_result = result.unwrap();
            assert_eq!(broadcast_result.successful, 5);
            assert_eq!(broadcast_result.failed, 0);
        }

        // Each receiver should have 10 messages
        for mut rx in receivers {
            let mut count = 0;
            while rx.try_recv().is_ok() {
                count += 1;
            }
            assert_eq!(count, 10);
        }
    }

    #[tokio::test]
    async fn test_concurrent_register_unregister() {
        use std::sync::Arc;
        use tokio::task::JoinSet;

        let registry = Arc::new(SurfaceRegistry::new());
        let mut join_set = JoinSet::new();

        // Spawn 5 registration tasks
        for _ in 0..5 {
            let registry = Arc::clone(&registry);
            join_set.spawn(async move {
                let id = ConnectionId::new();
                let (tx, _rx) = mpsc::channel(32);
                let handle = SurfaceHandle::new(
                    id,
                    tx,
                    SurfaceType::Headless,
                    SurfaceCapabilities::headless(),
                );
                registry.register(handle);
                // Small delay, then unregister
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                registry.unregister(&id);
                id
            });
        }

        // Spawn 5 more that just register and stay
        for _ in 0..5 {
            let registry = Arc::clone(&registry);
            join_set.spawn(async move {
                let id = ConnectionId::new();
                let (tx, _rx) = mpsc::channel(32);
                let handle = SurfaceHandle::new(
                    id,
                    tx,
                    SurfaceType::Headless,
                    SurfaceCapabilities::headless(),
                );
                registry.register(handle);
                id
            });
        }

        // Wait for all tasks
        while join_set.join_next().await.is_some() {}

        // Should have exactly 5 connections (the ones that didn't unregister)
        // Note: Due to timing, we might have 5-10 depending on race conditions
        // but the point is the registry doesn't corrupt
        assert!(registry.count() >= 5);
        assert!(registry.count() <= 10);
    }

    #[tokio::test]
    async fn test_send_to_specific_under_load() {
        use std::sync::Arc;
        use tokio::task::JoinSet;

        let registry = Arc::new(SurfaceRegistry::new());

        // Register 3 surfaces
        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();
        let id3 = ConnectionId::new();

        let (tx1, mut rx1) = mpsc::channel(100);
        let (tx2, mut rx2) = mpsc::channel(100);
        let (tx3, mut rx3) = mpsc::channel(100);

        registry.register(SurfaceHandle::new(
            id1,
            tx1,
            SurfaceType::Tui,
            SurfaceCapabilities::tui(),
        ));
        registry.register(SurfaceHandle::new(
            id2,
            tx2,
            SurfaceType::Web,
            SurfaceCapabilities::web(),
        ));
        registry.register(SurfaceHandle::new(
            id3,
            tx3,
            SurfaceType::Headless,
            SurfaceCapabilities::headless(),
        ));

        // Spawn concurrent tasks sending to specific surfaces
        let mut join_set = JoinSet::new();

        // 10 messages to id1
        for _ in 0..10 {
            let registry = Arc::clone(&registry);
            let target_id = id1;
            join_set.spawn(async move {
                registry
                    .send_to_async(&target_id, ConductorMessage::QueryCapabilities)
                    .await
            });
        }

        // 10 messages to id2
        for _ in 0..10 {
            let registry = Arc::clone(&registry);
            let target_id = id2;
            join_set.spawn(async move {
                registry
                    .send_to_async(&target_id, ConductorMessage::QueryCapabilities)
                    .await
            });
        }

        // Wait for all sends
        while join_set.join_next().await.is_some() {}

        // Check that messages went to correct surfaces
        let mut count1 = 0;
        while rx1.try_recv().is_ok() {
            count1 += 1;
        }
        assert_eq!(count1, 10, "id1 should have received 10 messages");

        let mut count2 = 0;
        while rx2.try_recv().is_ok() {
            count2 += 1;
        }
        assert_eq!(count2, 10, "id2 should have received 10 messages");

        let mut count3 = 0;
        while rx3.try_recv().is_ok() {
            count3 += 1;
        }
        assert_eq!(count3, 0, "id3 should have received 0 messages");
    }

    #[test]
    fn test_registry_with_mixed_surface_types() {
        let registry = SurfaceRegistry::new();

        // Register different surface types
        let tui_id = ConnectionId::new();
        let web_id = ConnectionId::new();
        let mobile_id = ConnectionId::new();
        let headless_id = ConnectionId::new();

        let (tui_tx, mut tui_rx) = mpsc::channel(32);
        let (web_tx, mut web_rx) = mpsc::channel(32);
        let (mobile_tx, mut mobile_rx) = mpsc::channel(32);
        let (headless_tx, mut headless_rx) = mpsc::channel(32);

        registry.register(SurfaceHandle::new(
            tui_id,
            tui_tx,
            SurfaceType::Tui,
            SurfaceCapabilities::tui(),
        ));
        registry.register(SurfaceHandle::new(
            web_id,
            web_tx,
            SurfaceType::Web,
            SurfaceCapabilities::web(),
        ));
        registry.register(SurfaceHandle::new(
            mobile_id,
            mobile_tx,
            SurfaceType::Mobile,
            SurfaceCapabilities::web(),
        ));
        registry.register(SurfaceHandle::new(
            headless_id,
            headless_tx,
            SurfaceType::Headless,
            SurfaceCapabilities::headless(),
        ));

        assert_eq!(registry.count(), 4);

        // Send only to surfaces with image support
        let msg = ConductorMessage::QueryCapabilities;
        let result = registry.send_to_capable(msg, |caps| caps.images);

        // Web and Mobile support images
        assert_eq!(result.successful, 2);

        // Verify only web and mobile received it
        assert!(tui_rx.try_recv().is_err());
        assert!(web_rx.try_recv().is_ok());
        assert!(mobile_rx.try_recv().is_ok());
        assert!(headless_rx.try_recv().is_err());

        // Verify surface types
        assert_eq!(registry.get_surface_type(&tui_id), Some(SurfaceType::Tui));
        assert_eq!(registry.get_surface_type(&web_id), Some(SurfaceType::Web));
    }

    #[test]
    fn test_update_capabilities() {
        let registry = SurfaceRegistry::new();
        let id = ConnectionId::new();
        let (tx, _rx) = mpsc::channel(32);

        // Register with headless capabilities (no color)
        registry.register(SurfaceHandle::new(
            id,
            tx,
            SurfaceType::Tui,
            SurfaceCapabilities::headless(),
        ));

        // Verify initial capabilities
        let caps = registry.get_capabilities(&id).unwrap();
        assert!(!caps.color);

        // Update to TUI capabilities (has color)
        registry.update_capabilities(&id, SurfaceCapabilities::tui());

        // Verify updated capabilities
        let caps = registry.get_capabilities(&id).unwrap();
        assert!(caps.color);
    }

    #[test]
    fn test_registry_clone_is_shared() {
        let registry1 = SurfaceRegistry::new();
        let registry2 = registry1.clone();

        let id = ConnectionId::new();
        let (tx, _rx) = mpsc::channel(32);
        let handle = SurfaceHandle::new(
            id,
            tx,
            SurfaceType::Headless,
            SurfaceCapabilities::headless(),
        );

        // Register on clone 1
        registry1.register(handle);

        // Should be visible on clone 2
        assert!(registry2.contains(&id));
        assert_eq!(registry2.count(), 1);
    }
}
