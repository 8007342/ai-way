//! WebSocket Frame Adapter
//!
//! Converts between internal Conductor messages and WebSocket frames.
//! Handles text (JSON) and binary frame types, compression, and serialization.

use serde::{de::DeserializeOwned, Serialize};

/// WebSocket frame types
///
/// Maps to the standard WebSocket opcode types relevant for our use case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebSocketFrameType {
    /// Text frame (UTF-8 encoded, typically JSON)
    Text,
    /// Binary frame (raw bytes, for sprites or compressed data)
    Binary,
    /// Ping frame (connection health check)
    Ping,
    /// Pong frame (response to ping)
    Pong,
    /// Close frame (connection termination)
    Close,
}

impl WebSocketFrameType {
    /// Check if this is a data frame (text or binary)
    #[must_use]
    pub fn is_data(&self) -> bool {
        matches!(self, Self::Text | Self::Binary)
    }

    /// Check if this is a control frame (ping, pong, close)
    #[must_use]
    pub fn is_control(&self) -> bool {
        matches!(self, Self::Ping | Self::Pong | Self::Close)
    }
}

/// A WebSocket frame ready for transmission
///
/// This is the adapter's output format, containing the frame type
/// and serialized payload ready for the WebSocket library.
#[derive(Clone, Debug)]
pub struct WebSocketFrame {
    /// Frame type (text, binary, etc.)
    pub frame_type: WebSocketFrameType,
    /// Serialized payload
    pub payload: Vec<u8>,
    /// Whether the payload is compressed
    pub compressed: bool,
}

impl WebSocketFrame {
    /// Create a new text frame with JSON payload
    #[must_use]
    pub fn text(payload: Vec<u8>) -> Self {
        Self {
            frame_type: WebSocketFrameType::Text,
            payload,
            compressed: false,
        }
    }

    /// Create a new binary frame
    #[must_use]
    pub fn binary(payload: Vec<u8>) -> Self {
        Self {
            frame_type: WebSocketFrameType::Binary,
            payload,
            compressed: false,
        }
    }

    /// Create a ping frame
    #[must_use]
    pub fn ping(payload: Vec<u8>) -> Self {
        Self {
            frame_type: WebSocketFrameType::Ping,
            payload,
            compressed: false,
        }
    }

    /// Create a pong frame
    #[must_use]
    pub fn pong(payload: Vec<u8>) -> Self {
        Self {
            frame_type: WebSocketFrameType::Pong,
            payload,
            compressed: false,
        }
    }

    /// Create a close frame
    #[must_use]
    pub fn close(code: u16, reason: &str) -> Self {
        let mut payload = code.to_be_bytes().to_vec();
        payload.extend_from_slice(reason.as_bytes());
        Self {
            frame_type: WebSocketFrameType::Close,
            payload,
            compressed: false,
        }
    }

    /// Mark this frame as compressed
    #[must_use]
    pub fn with_compression(mut self) -> Self {
        self.compressed = true;
        self
    }

    /// Get the payload length
    #[must_use]
    pub fn len(&self) -> usize {
        self.payload.len()
    }

    /// Check if the payload is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.payload.is_empty()
    }
}

/// Errors that can occur during frame conversion
#[derive(Clone, Debug)]
pub enum FrameConversionError {
    /// JSON serialization failed
    SerializationError(String),
    /// JSON deserialization failed
    DeserializationError(String),
    /// Payload too large
    PayloadTooLarge {
        /// Actual size
        size: usize,
        /// Maximum allowed size
        max: usize,
    },
    /// Invalid frame type for operation
    InvalidFrameType(String),
    /// Compression error
    CompressionError(String),
    /// Decompression error
    DecompressionError(String),
    /// Invalid UTF-8 in text frame
    InvalidUtf8(String),
}

impl std::fmt::Display for FrameConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationError(msg) => write!(f, "Serialization error: {msg}"),
            Self::DeserializationError(msg) => write!(f, "Deserialization error: {msg}"),
            Self::PayloadTooLarge { size, max } => {
                write!(f, "Payload too large: {size} bytes (max: {max})")
            }
            Self::InvalidFrameType(msg) => write!(f, "Invalid frame type: {msg}"),
            Self::CompressionError(msg) => write!(f, "Compression error: {msg}"),
            Self::DecompressionError(msg) => write!(f, "Decompression error: {msg}"),
            Self::InvalidUtf8(msg) => write!(f, "Invalid UTF-8: {msg}"),
        }
    }
}

impl std::error::Error for FrameConversionError {}

/// Adapter for converting between Conductor messages and WebSocket frames
///
/// This adapter handles:
/// - JSON serialization for text frames
/// - Binary encoding for sprite/image data
/// - Optional compression
/// - Size validation
///
/// # Example
///
/// ```ignore
/// use conductor_core::transport::websocket::WebSocketFrameAdapter;
/// use conductor_core::messages::ConductorMessage;
///
/// let adapter = WebSocketFrameAdapter::new();
///
/// // Convert message to WebSocket frame
/// let msg = ConductorMessage::State { state: ConductorState::Ready };
/// let frame = adapter.to_websocket_frame(&msg)?;
///
/// // Convert WebSocket frame back to message
/// let decoded: ConductorMessage = adapter.from_websocket_frame(&frame)?;
/// ```
#[derive(Debug)]
pub struct WebSocketFrameAdapter {
    /// Maximum message size (bytes)
    max_message_size: usize,
    /// Whether compression is enabled
    compression_enabled: bool,
    /// Compression threshold - messages smaller than this won't be compressed
    compression_threshold: usize,
}

impl Default for WebSocketFrameAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSocketFrameAdapter {
    /// Create a new frame adapter with default settings
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_message_size: 10 * 1024 * 1024, // 10 MB
            compression_enabled: false, // Disabled by default (TODO: enable with security review)
            compression_threshold: 1024, // Only compress messages > 1KB
        }
    }

    /// Create an adapter with custom settings
    #[must_use]
    pub fn with_settings(
        max_message_size: usize,
        compression_enabled: bool,
        compression_threshold: usize,
    ) -> Self {
        Self {
            max_message_size,
            compression_enabled,
            compression_threshold,
        }
    }

    /// Enable or disable compression
    #[must_use]
    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.compression_enabled = enabled;
        self
    }

    /// Set the maximum message size
    #[must_use]
    pub fn with_max_size(mut self, size: usize) -> Self {
        self.max_message_size = size;
        self
    }

    /// Convert a Conductor message to a WebSocket frame
    ///
    /// Serializes the message to JSON and wraps it in a text frame.
    /// If compression is enabled and the message exceeds the threshold,
    /// the payload will be compressed.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or the message is too large.
    pub fn to_websocket_frame<T: Serialize>(
        &self,
        message: &T,
    ) -> Result<WebSocketFrame, FrameConversionError> {
        // Serialize to JSON
        let json = serde_json::to_vec(message)
            .map_err(|e| FrameConversionError::SerializationError(e.to_string()))?;

        // Check size before any compression
        if json.len() > self.max_message_size {
            return Err(FrameConversionError::PayloadTooLarge {
                size: json.len(),
                max: self.max_message_size,
            });
        }

        // Compression is a TODO - just return uncompressed for now
        // Actual compression would use permessage-deflate at the WebSocket layer
        let (payload, compressed) =
            if self.compression_enabled && json.len() > self.compression_threshold {
                // TODO: Implement compression when security review is complete
                // For now, return uncompressed with a log note
                tracing::trace!(
                    size = json.len(),
                    "Compression requested but not yet implemented"
                );
                (json, false)
            } else {
                (json, false)
            };

        let mut frame = WebSocketFrame::text(payload);
        if compressed {
            frame = frame.with_compression();
        }

        Ok(frame)
    }

    /// Convert a WebSocket frame back to a Conductor message
    ///
    /// Deserializes the JSON payload from a text frame.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The frame is not a text frame
    /// - The payload is not valid UTF-8
    /// - JSON deserialization fails
    pub fn from_websocket_frame<T: DeserializeOwned>(
        &self,
        frame: &WebSocketFrame,
    ) -> Result<T, FrameConversionError> {
        // Validate frame type
        if frame.frame_type != WebSocketFrameType::Text
            && frame.frame_type != WebSocketFrameType::Binary
        {
            return Err(FrameConversionError::InvalidFrameType(format!(
                "Expected Text or Binary frame, got {:?}",
                frame.frame_type
            )));
        }

        // Handle decompression if needed
        let payload = if frame.compressed {
            // TODO: Implement decompression when security review is complete
            tracing::warn!("Received compressed frame but decompression not yet implemented");
            return Err(FrameConversionError::DecompressionError(
                "Compression not yet implemented".to_string(),
            ));
        } else {
            &frame.payload
        };

        // Check size
        if payload.len() > self.max_message_size {
            return Err(FrameConversionError::PayloadTooLarge {
                size: payload.len(),
                max: self.max_message_size,
            });
        }

        // For text frames, validate UTF-8
        if frame.frame_type == WebSocketFrameType::Text {
            let _text = std::str::from_utf8(payload)
                .map_err(|e| FrameConversionError::InvalidUtf8(e.to_string()))?;
        }

        // Deserialize JSON
        serde_json::from_slice(payload)
            .map_err(|e| FrameConversionError::DeserializationError(e.to_string()))
    }

    /// Create a binary frame from raw bytes
    ///
    /// Used for sprite data, images, or other binary content.
    ///
    /// # Errors
    ///
    /// Returns an error if the data exceeds the maximum size.
    pub fn to_binary_frame(&self, data: &[u8]) -> Result<WebSocketFrame, FrameConversionError> {
        if data.len() > self.max_message_size {
            return Err(FrameConversionError::PayloadTooLarge {
                size: data.len(),
                max: self.max_message_size,
            });
        }

        Ok(WebSocketFrame::binary(data.to_vec()))
    }

    /// Extract binary data from a frame
    ///
    /// # Errors
    ///
    /// Returns an error if the frame is not a binary frame.
    pub fn from_binary_frame(
        &self,
        frame: &WebSocketFrame,
    ) -> Result<Vec<u8>, FrameConversionError> {
        if frame.frame_type != WebSocketFrameType::Binary {
            return Err(FrameConversionError::InvalidFrameType(format!(
                "Expected Binary frame, got {:?}",
                frame.frame_type
            )));
        }

        if frame.payload.len() > self.max_message_size {
            return Err(FrameConversionError::PayloadTooLarge {
                size: frame.payload.len(),
                max: self.max_message_size,
            });
        }

        Ok(frame.payload.clone())
    }

    /// Get the maximum message size
    #[must_use]
    pub fn max_message_size(&self) -> usize {
        self.max_message_size
    }

    /// Check if compression is enabled
    #[must_use]
    pub fn compression_enabled(&self) -> bool {
        self.compression_enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct TestMessage {
        id: u32,
        content: String,
    }

    #[test]
    fn test_frame_type_classification() {
        assert!(WebSocketFrameType::Text.is_data());
        assert!(WebSocketFrameType::Binary.is_data());
        assert!(!WebSocketFrameType::Ping.is_data());

        assert!(WebSocketFrameType::Ping.is_control());
        assert!(WebSocketFrameType::Pong.is_control());
        assert!(WebSocketFrameType::Close.is_control());
        assert!(!WebSocketFrameType::Text.is_control());
    }

    #[test]
    fn test_websocket_frame_text() {
        let frame = WebSocketFrame::text(b"hello".to_vec());
        assert_eq!(frame.frame_type, WebSocketFrameType::Text);
        assert_eq!(frame.payload, b"hello");
        assert!(!frame.compressed);
        assert_eq!(frame.len(), 5);
        assert!(!frame.is_empty());
    }

    #[test]
    fn test_websocket_frame_binary() {
        let frame = WebSocketFrame::binary(vec![0x00, 0x01, 0x02]);
        assert_eq!(frame.frame_type, WebSocketFrameType::Binary);
        assert_eq!(frame.payload, vec![0x00, 0x01, 0x02]);
    }

    #[test]
    fn test_websocket_frame_close() {
        let frame = WebSocketFrame::close(1000, "normal closure");
        assert_eq!(frame.frame_type, WebSocketFrameType::Close);
        // First 2 bytes are the close code (1000 = 0x03E8)
        assert_eq!(frame.payload[0], 0x03);
        assert_eq!(frame.payload[1], 0xE8);
        // Rest is the reason
        assert_eq!(&frame.payload[2..], b"normal closure");
    }

    #[test]
    fn test_websocket_frame_with_compression() {
        let frame = WebSocketFrame::text(b"data".to_vec()).with_compression();
        assert!(frame.compressed);
    }

    #[test]
    fn test_adapter_roundtrip() {
        let adapter = WebSocketFrameAdapter::new();
        let msg = TestMessage {
            id: 42,
            content: "Hello, World!".to_string(),
        };

        let frame = adapter.to_websocket_frame(&msg).unwrap();
        assert_eq!(frame.frame_type, WebSocketFrameType::Text);

        let decoded: TestMessage = adapter.from_websocket_frame(&frame).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_adapter_size_limit() {
        let adapter = WebSocketFrameAdapter::new().with_max_size(100);

        let msg = TestMessage {
            id: 1,
            content: "x".repeat(200), // Will exceed 100 bytes when serialized
        };

        let result = adapter.to_websocket_frame(&msg);
        assert!(matches!(
            result,
            Err(FrameConversionError::PayloadTooLarge { .. })
        ));
    }

    #[test]
    fn test_adapter_invalid_frame_type() {
        let adapter = WebSocketFrameAdapter::new();
        let frame = WebSocketFrame::ping(vec![]);

        let result: Result<TestMessage, _> = adapter.from_websocket_frame(&frame);
        assert!(matches!(
            result,
            Err(FrameConversionError::InvalidFrameType(_))
        ));
    }

    #[test]
    fn test_adapter_invalid_json() {
        let adapter = WebSocketFrameAdapter::new();
        let frame = WebSocketFrame::text(b"not valid json".to_vec());

        let result: Result<TestMessage, _> = adapter.from_websocket_frame(&frame);
        assert!(matches!(
            result,
            Err(FrameConversionError::DeserializationError(_))
        ));
    }

    #[test]
    fn test_adapter_invalid_utf8() {
        let adapter = WebSocketFrameAdapter::new();
        // Invalid UTF-8 sequence
        let frame = WebSocketFrame::text(vec![0xFF, 0xFE]);

        let result: Result<TestMessage, _> = adapter.from_websocket_frame(&frame);
        assert!(matches!(result, Err(FrameConversionError::InvalidUtf8(_))));
    }

    #[test]
    fn test_binary_frame_roundtrip() {
        let adapter = WebSocketFrameAdapter::new();
        let data = vec![0x00, 0x01, 0x02, 0xFF];

        let frame = adapter.to_binary_frame(&data).unwrap();
        assert_eq!(frame.frame_type, WebSocketFrameType::Binary);

        let decoded = adapter.from_binary_frame(&frame).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_binary_frame_size_limit() {
        let adapter = WebSocketFrameAdapter::new().with_max_size(10);
        let data = vec![0u8; 100];

        let result = adapter.to_binary_frame(&data);
        assert!(matches!(
            result,
            Err(FrameConversionError::PayloadTooLarge { .. })
        ));
    }

    #[test]
    fn test_binary_frame_wrong_type() {
        let adapter = WebSocketFrameAdapter::new();
        let frame = WebSocketFrame::text(b"text data".to_vec());

        let result = adapter.from_binary_frame(&frame);
        assert!(matches!(
            result,
            Err(FrameConversionError::InvalidFrameType(_))
        ));
    }

    #[test]
    fn test_error_display() {
        let err = FrameConversionError::SerializationError("test error".to_string());
        assert!(err.to_string().contains("test error"));

        let err = FrameConversionError::PayloadTooLarge {
            size: 1000,
            max: 500,
        };
        let msg = err.to_string();
        assert!(msg.contains("1000"));
        assert!(msg.contains("500"));
    }

    #[test]
    fn test_adapter_settings() {
        let adapter = WebSocketFrameAdapter::with_settings(1024, true, 512);
        assert_eq!(adapter.max_message_size(), 1024);
        assert!(adapter.compression_enabled());
    }
}
