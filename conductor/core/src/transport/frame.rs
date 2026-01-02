//! Frame Protocol
//!
//! Wire format for Conductor-Surface messages using length-prefixed JSON with
//! CRC32 checksum for integrity verification.
//!
//! # Frame Format
//!
//! ```text
//! +----------------+----------------+------------------------------------------+
//! | Length (4)     | Checksum (4)   | JSON Payload (variable)                  |
//! | big-endian u32 | CRC32          | ConductorMessage or SurfaceEvent         |
//! +----------------+----------------+------------------------------------------+
//! ```
//!
//! The Length field contains the size of the JSON payload only (not including the checksum).
//! The Checksum is the CRC32 hash of the JSON payload.
//!
//! # Security
//!
//! - Maximum frame size is enforced to prevent memory exhaustion
//! - Length field is validated before allocating buffer
//! - CRC32 checksum detects data corruption in transit

use serde::{de::DeserializeOwned, Serialize};

use super::TransportError;

/// Maximum frame size (10 MB)
///
/// This prevents memory exhaustion from malicious or corrupted frames.
pub const MAX_FRAME_SIZE: usize = 10 * 1024 * 1024;

/// Minimum buffer capacity for decoder
const MIN_BUFFER_CAPACITY: usize = 4096;

/// Frame header size: 4 bytes length + 4 bytes checksum
const HEADER_SIZE: usize = 8;

/// Compute CRC32 checksum for payload
#[inline]
fn compute_checksum(payload: &[u8]) -> u32 {
    crc32fast::hash(payload)
}

/// Encode a message to a length-prefixed frame with CRC32 checksum
///
/// # Frame Format
///
/// `[Length(4)][Checksum(4)][JSON Payload]`
///
/// # Errors
///
/// Returns `TransportError::SerializationError` if:
/// - JSON serialization fails
/// - Resulting frame exceeds `MAX_FRAME_SIZE`
pub fn encode<T: Serialize>(msg: &T) -> Result<Vec<u8>, TransportError> {
    let json =
        serde_json::to_vec(msg).map_err(|e| TransportError::SerializationError(e.to_string()))?;

    if json.len() > MAX_FRAME_SIZE {
        return Err(TransportError::SerializationError(format!(
            "Frame too large: {} bytes (max: {})",
            json.len(),
            MAX_FRAME_SIZE
        )));
    }

    let len = json.len() as u32;
    let checksum = compute_checksum(&json);

    let mut buf = Vec::with_capacity(HEADER_SIZE + json.len());
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(&checksum.to_be_bytes());
    buf.extend_from_slice(&json);
    Ok(buf)
}

/// Encoder for streaming frame output
#[derive(Debug, Default)]
pub struct FrameEncoder;

impl FrameEncoder {
    /// Create a new encoder
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn available(&self) -> usize {
        self.buffer.len() - self.read_pos
    }

    /// Try to decode the next frame
    ///
    /// Returns:
    /// - `Ok(Some(msg))` if a complete frame was decoded
    /// - `Ok(None)` if more data is needed
    /// - `Err(TransportError::ChecksumMismatch)` if checksum verification fails
    /// - `Err(...)` if frame is invalid
    pub fn decode<T: DeserializeOwned>(&mut self) -> Result<Option<T>, TransportError> {
        let available = self.available();

        // Need at least 8 bytes for header (length + checksum)
        if available < HEADER_SIZE {
            return Ok(None);
        }

        // Read length (big-endian u32)
        let len_bytes = &self.buffer[self.read_pos..self.read_pos + 4];
        let len =
            u32::from_be_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;

        // Validate frame size
        if len > MAX_FRAME_SIZE {
            return Err(TransportError::SerializationError(format!(
                "Frame size {len} exceeds maximum {MAX_FRAME_SIZE}"
            )));
        }

        // Need more data for checksum + payload
        if available < HEADER_SIZE + len {
            return Ok(None);
        }

        // Read checksum (big-endian u32)
        let checksum_bytes = &self.buffer[self.read_pos + 4..self.read_pos + 8];
        let expected_checksum = u32::from_be_bytes([
            checksum_bytes[0],
            checksum_bytes[1],
            checksum_bytes[2],
            checksum_bytes[3],
        ]);

        // Extract payload
        let payload_start = self.read_pos + HEADER_SIZE;
        let payload_end = payload_start + len;
        let payload = &self.buffer[payload_start..payload_end];

        // Verify checksum
        let actual_checksum = compute_checksum(payload);
        if actual_checksum != expected_checksum {
            return Err(TransportError::ChecksumMismatch {
                expected: expected_checksum,
                actual: actual_checksum,
            });
        }

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
        assert!(encoded.len() > HEADER_SIZE); // At least header (length + checksum)

        let mut decoder = FrameDecoder::new();
        decoder.push(&encoded);

        let decoded: TestMessage = decoder.decode().unwrap().unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_decode_partial_header() {
        let mut decoder = FrameDecoder::new();
        decoder.push(&[0, 0, 0, 5]); // Only length, no checksum

        let result: Result<Option<TestMessage>, _> = decoder.decode();
        assert!(matches!(result, Ok(None)));

        // Also test with even less data
        let mut decoder2 = FrameDecoder::new();
        decoder2.push(&[0, 0]); // Only 2 bytes
        let result2: Result<Option<TestMessage>, _> = decoder2.decode();
        assert!(matches!(result2, Ok(None)));
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

        // Valid length prefix and checksum but invalid JSON
        let invalid_json = b"not valid json";
        let len = (invalid_json.len() as u32).to_be_bytes();
        let checksum = compute_checksum(invalid_json).to_be_bytes();

        decoder.push(&len);
        decoder.push(&checksum);
        decoder.push(invalid_json);

        let result: Result<Option<TestMessage>, _> = decoder.decode();
        assert!(matches!(result, Err(TransportError::SerializationError(_))));
    }

    #[test]
    fn test_decode_frame_too_large() {
        let mut decoder = FrameDecoder::new();

        // Claim a frame size larger than max (include dummy checksum)
        let huge_len = ((MAX_FRAME_SIZE + 1) as u32).to_be_bytes();
        let dummy_checksum = [0u8; 4];
        decoder.push(&huge_len);
        decoder.push(&dummy_checksum);

        let result: Result<Option<TestMessage>, _> = decoder.decode();
        assert!(matches!(result, Err(TransportError::SerializationError(_))));
    }

    #[test]
    fn test_checksum_valid() {
        // Test that a valid frame with correct CRC32 checksum decodes successfully
        let msg = TestMessage {
            content: "checksum test".to_string(),
            number: 99,
        };

        let encoded = encode(&msg).unwrap();

        // Verify frame structure: [length: 4][checksum: 4][payload]
        assert!(encoded.len() >= HEADER_SIZE);

        // Extract and verify checksum is present
        let payload_len =
            u32::from_be_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]) as usize;
        assert_eq!(payload_len, encoded.len() - HEADER_SIZE);

        // Decode and verify message integrity
        let mut decoder = FrameDecoder::new();
        decoder.push(&encoded);
        let decoded: TestMessage = decoder.decode().unwrap().unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_checksum_mismatch() {
        // Test that corrupted payload is detected via checksum mismatch
        let mut decoder = FrameDecoder::new();

        // Valid JSON payload
        let valid_json = b"{\"content\":\"test\",\"number\":1}";
        let len = (valid_json.len() as u32).to_be_bytes();
        // Intentionally wrong checksum
        let wrong_checksum = 0xDEADBEEFu32.to_be_bytes();

        decoder.push(&len);
        decoder.push(&wrong_checksum);
        decoder.push(valid_json);

        let result: Result<Option<TestMessage>, _> = decoder.decode();
        assert!(matches!(
            result,
            Err(TransportError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn test_checksum_deterministic() {
        let msg = TestMessage {
            content: "deterministic".to_string(),
            number: 123,
        };

        let encoded1 = encode(&msg).unwrap();
        let encoded2 = encode(&msg).unwrap();

        // Same message should produce identical frames (including checksum)
        assert_eq!(encoded1, encoded2);
    }

    #[test]
    fn test_checksum_changes_with_payload() {
        let msg1 = TestMessage {
            content: "hello".to_string(),
            number: 1,
        };
        let msg2 = TestMessage {
            content: "world".to_string(),
            number: 1,
        };

        let encoded1 = encode(&msg1).unwrap();
        let encoded2 = encode(&msg2).unwrap();

        // Extract checksums (bytes 4-8)
        let checksum1 = &encoded1[4..8];
        let checksum2 = &encoded2[4..8];

        // Different payloads should produce different checksums
        assert_ne!(checksum1, checksum2);
    }
}
