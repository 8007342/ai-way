//! Frame Protocol
//!
//! Wire format for Conductor-Surface messages using length-prefixed JSON.
//!
//! # Frame Format
//!
//! ```text
//! +----------------+------------------------------------------+
//! | Length (4)     | JSON Payload (variable)                  |
//! | big-endian u32 | ConductorMessage or SurfaceEvent         |
//! +----------------+------------------------------------------+
//! ```
//!
//! # Security
//!
//! - Maximum frame size is enforced to prevent memory exhaustion
//! - Length field is validated before allocating buffer

use serde::{de::DeserializeOwned, Serialize};

use super::TransportError;

/// Maximum frame size (10 MB)
///
/// This prevents memory exhaustion from malicious or corrupted frames.
pub const MAX_FRAME_SIZE: usize = 10 * 1024 * 1024;

/// Minimum buffer capacity for decoder
const MIN_BUFFER_CAPACITY: usize = 4096;

/// Encode a message to a length-prefixed frame
///
/// # Errors
///
/// Returns `TransportError::SerializationError` if:
/// - JSON serialization fails
/// - Resulting frame exceeds `MAX_FRAME_SIZE`
pub fn encode<T: Serialize>(msg: &T) -> Result<Vec<u8>, TransportError> {
    let json = serde_json::to_vec(msg)
        .map_err(|e| TransportError::SerializationError(e.to_string()))?;

    if json.len() > MAX_FRAME_SIZE {
        return Err(TransportError::SerializationError(format!(
            "Frame too large: {} bytes (max: {})",
            json.len(),
            MAX_FRAME_SIZE
        )));
    }

    let len = json.len() as u32;
    let mut buf = Vec::with_capacity(4 + json.len());
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(&json);
    Ok(buf)
}

/// Encoder for streaming frame output
#[derive(Debug, Default)]
pub struct FrameEncoder;

impl FrameEncoder {
    /// Create a new encoder
    pub fn new() -> Self {
        Self
    }

    /// Encode a message to bytes
    pub fn encode<T: Serialize>(&self, msg: &T) -> Result<Vec<u8>, TransportError> {
        encode(msg)
    }
}

/// Decoder state machine for streaming frame parsing
///
/// Buffers incoming bytes and yields complete messages.
#[derive(Debug)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
    /// Position where we've consumed up to
    read_pos: usize,
}

impl Default for FrameDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameDecoder {
    /// Create a new decoder with default buffer capacity
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(MIN_BUFFER_CAPACITY),
            read_pos: 0,
        }
    }

    /// Append bytes to the buffer
    pub fn push(&mut self, data: &[u8]) {
        // Compact buffer if we've consumed a lot
        if self.read_pos > self.buffer.len() / 2 && self.read_pos > MIN_BUFFER_CAPACITY {
            self.buffer.drain(..self.read_pos);
            self.read_pos = 0;
        }
        self.buffer.extend_from_slice(data);
    }

    /// Get the number of bytes available in the buffer
    pub fn available(&self) -> usize {
        self.buffer.len() - self.read_pos
    }

    /// Try to decode the next frame
    ///
    /// Returns:
    /// - `Ok(Some(msg))` if a complete frame was decoded
    /// - `Ok(None)` if more data is needed
    /// - `Err(...)` if frame is invalid
    pub fn decode<T: DeserializeOwned>(&mut self) -> Result<Option<T>, TransportError> {
        let available = self.available();

        // Need at least 4 bytes for length
        if available < 4 {
            return Ok(None);
        }

        // Read length (big-endian u32)
        let len_bytes = &self.buffer[self.read_pos..self.read_pos + 4];
        let len = u32::from_be_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]])
            as usize;

        // Validate frame size
        if len > MAX_FRAME_SIZE {
            return Err(TransportError::SerializationError(format!(
                "Frame size {} exceeds maximum {}",
                len, MAX_FRAME_SIZE
            )));
        }

        // Need more data for payload
        if available < 4 + len {
            return Ok(None);
        }

        // Extract and parse payload
        let payload_start = self.read_pos + 4;
        let payload_end = payload_start + len;
        let payload = &self.buffer[payload_start..payload_end];

        let msg = serde_json::from_slice(payload)
            .map_err(|e| TransportError::SerializationError(e.to_string()))?;

        // Advance read position
        self.read_pos = payload_end;

        Ok(Some(msg))
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.read_pos = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestMessage {
        content: String,
        number: u32,
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let msg = TestMessage {
            content: "Hello, world!".to_string(),
            number: 42,
        };

        let encoded = encode(&msg).unwrap();
        assert!(encoded.len() > 4); // At least length prefix

        let mut decoder = FrameDecoder::new();
        decoder.push(&encoded);

        let decoded: TestMessage = decoder.decode().unwrap().unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_decode_partial_length() {
        let mut decoder = FrameDecoder::new();
        decoder.push(&[0, 0]); // Only 2 bytes

        let result: Result<Option<TestMessage>, _> = decoder.decode();
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn test_decode_partial_payload() {
        let msg = TestMessage {
            content: "test".to_string(),
            number: 1,
        };

        let encoded = encode(&msg).unwrap();

        let mut decoder = FrameDecoder::new();
        // Push only first half
        decoder.push(&encoded[..encoded.len() / 2]);

        let result: Result<Option<TestMessage>, _> = decoder.decode();
        assert!(matches!(result, Ok(None)));

        // Push rest
        decoder.push(&encoded[encoded.len() / 2..]);

        let decoded: TestMessage = decoder.decode().unwrap().unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_decode_multiple_frames() {
        let msg1 = TestMessage {
            content: "first".to_string(),
            number: 1,
        };
        let msg2 = TestMessage {
            content: "second".to_string(),
            number: 2,
        };

        let mut encoded = encode(&msg1).unwrap();
        encoded.extend(encode(&msg2).unwrap());

        let mut decoder = FrameDecoder::new();
        decoder.push(&encoded);

        let decoded1: TestMessage = decoder.decode().unwrap().unwrap();
        let decoded2: TestMessage = decoder.decode().unwrap().unwrap();
        let no_more: Option<TestMessage> = decoder.decode().unwrap();

        assert_eq!(decoded1, msg1);
        assert_eq!(decoded2, msg2);
        assert!(no_more.is_none());
    }

    #[test]
    fn test_encode_too_large() {
        // Create a message that will exceed max size when serialized
        let large_content = "x".repeat(MAX_FRAME_SIZE + 1);
        let msg = TestMessage {
            content: large_content,
            number: 0,
        };

        let result = encode(&msg);
        assert!(matches!(result, Err(TransportError::SerializationError(_))));
    }

    #[test]
    fn test_decode_invalid_json() {
        let mut decoder = FrameDecoder::new();

        // Valid length prefix but invalid JSON
        let invalid_json = b"not valid json";
        let len = (invalid_json.len() as u32).to_be_bytes();

        decoder.push(&len);
        decoder.push(invalid_json);

        let result: Result<Option<TestMessage>, _> = decoder.decode();
        assert!(matches!(result, Err(TransportError::SerializationError(_))));
    }

    #[test]
    fn test_decode_frame_too_large() {
        let mut decoder = FrameDecoder::new();

        // Claim a frame size larger than max
        let huge_len = ((MAX_FRAME_SIZE + 1) as u32).to_be_bytes();
        decoder.push(&huge_len);

        let result: Result<Option<TestMessage>, _> = decoder.decode();
        assert!(matches!(result, Err(TransportError::SerializationError(_))));
    }
}
