//! In-Process Transport
//!
//! Direct channel-based communication for embedded mode.
//! This is used when the TUI embeds the Conductor directly (no process separation).
//!
//! # Usage
//!
//! ```ignore
//! let (transport, event_rx, msg_tx) = InProcessTransport::new_pair();
//!
//! // Give event_rx and msg_tx to the Conductor
//! // Use transport in the Surface
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::events::SurfaceEvent;
use crate::messages::ConductorMessage;

use super::traits::{SurfaceTransport, TransportError};

/// In-process transport using tokio channels
///
/// Provides zero-copy, zero-serialization communication when the
/// Conductor is embedded directly in the Surface process.
pub struct InProcessTransport {
    /// Channel to send events to Conductor
    event_tx: mpsc::Sender<SurfaceEvent>,
    /// Channel to receive messages from Conductor
    msg_rx: mpsc::Receiver<ConductorMessage>,
    /// Connection state
    connected: Arc<AtomicBool>,
}

impl InProcessTransport {
    /// Create a new in-process transport pair
    ///
    /// Returns:
    /// - `InProcessTransport`: Use this in the Surface
    /// - `mpsc::Receiver<SurfaceEvent>`: Conductor receives events here
    /// - `mpsc::Sender<ConductorMessage>`: Conductor sends messages here
    ///
    /// # Example
    ///
    /// ```ignore
    /// let (transport, event_rx, msg_tx) = InProcessTransport::new_pair();
    ///
    /// // Create Conductor with msg_tx
    /// let conductor = Conductor::new(backend, config, msg_tx);
    ///
    /// // Spawn task to forward events to conductor
    /// tokio::spawn(async move {
    ///     while let Some(event) = event_rx.recv().await {
    ///         conductor.handle_event(event).await.ok();
    ///     }
    /// });
    ///
    /// // Use transport in Surface
    /// transport.send(SurfaceEvent::Connected { ... }).await?;
    /// ```
    #[must_use]
    pub fn new_pair() -> (
        Self,
        mpsc::Receiver<SurfaceEvent>,
        mpsc::Sender<ConductorMessage>,
    ) {
        let (event_tx, event_rx) = mpsc::channel(100);
        let (msg_tx, msg_rx) = mpsc::channel(100);

        let transport = Self {
            event_tx,
            msg_rx,
            connected: Arc::new(AtomicBool::new(true)),
        };

        (transport, event_rx, msg_tx)
    }

    /// Create with custom channel capacity
    #[must_use]
    pub fn new_pair_with_capacity(
        capacity: usize,
    ) -> (
        Self,
        mpsc::Receiver<SurfaceEvent>,
        mpsc::Sender<ConductorMessage>,
    ) {
        let (event_tx, event_rx) = mpsc::channel(capacity);
        let (msg_tx, msg_rx) = mpsc::channel(capacity);

        let transport = Self {
            event_tx,
            msg_rx,
            connected: Arc::new(AtomicBool::new(true)),
        };

        (transport, event_rx, msg_tx)
    }
}

#[async_trait]
impl SurfaceTransport for InProcessTransport {
    async fn connect(&mut self) -> Result<(), TransportError> {
        self.connected.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self, event: SurfaceEvent) -> Result<(), TransportError> {
        if !self.connected.load(Ordering::SeqCst) {
            return Err(TransportError::InvalidState(
                "Transport not connected".to_string(),
            ));
        }

        self.event_tx
            .send(event)
            .await
            .map_err(|_| TransportError::SendFailed("Channel closed".to_string()))
    }

    async fn recv(&mut self) -> Result<ConductorMessage, TransportError> {
        self.msg_rx
            .recv()
            .await
            .ok_or(TransportError::ConnectionClosed)
    }

    fn try_recv(&mut self) -> Option<ConductorMessage> {
        self.msg_rx.try_recv().ok()
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_process_roundtrip() {
        use crate::events::{SurfaceCapabilities, SurfaceType};
        use crate::messages::{ConductorState, EventId};
        use crate::ConductorMessage;

        let (mut transport, mut event_rx, msg_tx) = InProcessTransport::new_pair();

        // Send an event
        let event = SurfaceEvent::Connected {
            event_id: EventId("test".to_string()),
            surface_type: SurfaceType::Tui,
            capabilities: SurfaceCapabilities::tui(),
        };

        transport.send(event.clone()).await.unwrap();

        // Receive the event
        let received_event = event_rx.recv().await.unwrap();
        assert!(matches!(received_event, SurfaceEvent::Connected { .. }));

        // Send a message back
        let msg = ConductorMessage::State {
            state: ConductorState::Ready,
        };
        msg_tx.send(msg).await.unwrap();

        // Receive the message
        let received_msg = transport.recv().await.unwrap();
        assert!(matches!(
            received_msg,
            ConductorMessage::State {
                state: ConductorState::Ready
            }
        ));
    }

    #[tokio::test]
    async fn test_in_process_try_recv() {
        use crate::messages::ConductorState;
        use crate::ConductorMessage;

        let (mut transport, _event_rx, msg_tx) = InProcessTransport::new_pair();

        // No message yet
        assert!(transport.try_recv().is_none());

        // Send a message
        let msg = ConductorMessage::State {
            state: ConductorState::Ready,
        };
        msg_tx.send(msg).await.unwrap();

        // Now we should get it
        let received = transport.try_recv();
        assert!(received.is_some());
    }

    #[tokio::test]
    async fn test_in_process_disconnect() {
        use crate::events::{SurfaceCapabilities, SurfaceType};
        use crate::messages::EventId;

        let (mut transport, _event_rx, _msg_tx) = InProcessTransport::new_pair();

        assert!(transport.is_connected());

        transport.disconnect().await.unwrap();
        assert!(!transport.is_connected());

        // Sending after disconnect should fail
        let event = SurfaceEvent::Connected {
            event_id: EventId("test".to_string()),
            surface_type: SurfaceType::Tui,
            capabilities: SurfaceCapabilities::tui(),
        };

        let result = transport.send(event).await;
        assert!(matches!(result, Err(TransportError::InvalidState(_))));

        // Reconnect
        transport.connect().await.unwrap();
        assert!(transport.is_connected());
    }

    #[tokio::test]
    async fn test_in_process_channel_closed() {
        let (transport, event_rx, _msg_tx) = InProcessTransport::new_pair();

        // Drop the event receiver
        drop(event_rx);

        // Sending should fail
        let event = SurfaceEvent::QuitRequested {
            event_id: crate::messages::EventId("test".to_string()),
        };

        let result = transport.send(event).await;
        assert!(matches!(result, Err(TransportError::SendFailed(_))));
    }
}
