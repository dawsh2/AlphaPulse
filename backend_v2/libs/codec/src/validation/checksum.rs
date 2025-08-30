//! CRC32 Checksum Validation
//!
//! Fast hardware-accelerated checksum validation for message integrity

/// Calculate CRC32 checksum for a message
pub fn calculate_crc32(data: &[u8]) -> u32 {
    crc32fast::hash(data)
}

/// Calculate CRC32 checksum excluding the checksum field itself
/// Used for message validation where the checksum is embedded in the message
pub fn calculate_crc32_excluding_checksum(data: &[u8], checksum_offset: usize) -> u32 {
    let mut hasher = crc32fast::Hasher::new();

    // Hash everything before the checksum field
    hasher.update(&data[..checksum_offset]);

    // Hash everything after the checksum field (if any)
    let checksum_end = checksum_offset + 4; // CRC32 is 4 bytes
    if checksum_end < data.len() {
        hasher.update(&data[checksum_end..]);
    }

    hasher.finalize()
}

/// Verify a message checksum
pub fn verify_message_checksum(
    data: &[u8],
    expected_checksum: u32,
    checksum_offset: usize,
) -> bool {
    let calculated = calculate_crc32_excluding_checksum(data, checksum_offset);
    calculated == expected_checksum
}

/// Calculate and embed checksum into a mutable message buffer
pub fn embed_checksum(data: &mut [u8], checksum_offset: usize) {
    // Zero out the checksum field first
    if checksum_offset + 4 <= data.len() {
        data[checksum_offset..checksum_offset + 4].fill(0);

        // Calculate checksum excluding the checksum field itself
        let checksum = calculate_crc32_excluding_checksum(data, checksum_offset);

        // Embed checksum as little-endian bytes
        data[checksum_offset..checksum_offset + 4].copy_from_slice(&checksum.to_le_bytes());
    }
}

/// Streaming checksum calculator for large messages
pub struct StreamingChecksum {
    hasher: crc32fast::Hasher,
}

impl StreamingChecksum {
    pub fn new() -> Self {
        Self {
            hasher: crc32fast::Hasher::new(),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    pub fn finalize(self) -> u32 {
        self.hasher.finalize()
    }

    pub fn reset(&mut self) {
        self.hasher = crc32fast::Hasher::new();
    }
}

impl Default for StreamingChecksum {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_checksum() {
        let data = b"Hello, world!";
        let checksum = calculate_crc32(data);

        // CRC32 should be deterministic
        assert_eq!(checksum, calculate_crc32(data));

        // Different data should produce different checksums (very likely)
        let other_data = b"Hello, World!"; // Different capitalization
        assert_ne!(checksum, calculate_crc32(other_data));
    }

    #[test]
    fn test_checksum_excluding_field() {
        // Create a message with embedded checksum
        let mut message = vec![
            0x01, 0x02, 0x03, 0x04, // data before checksum
            0x00, 0x00, 0x00, 0x00, // checksum field (placeholder)
            0x05, 0x06, 0x07, 0x08,
        ]; // data after checksum

        let checksum_offset = 4;

        // Calculate checksum excluding the checksum field
        let checksum = calculate_crc32_excluding_checksum(&message, checksum_offset);

        // Embed the checksum
        message[checksum_offset..checksum_offset + 4].copy_from_slice(&checksum.to_le_bytes());

        // Verify it
        assert!(verify_message_checksum(&message, checksum, checksum_offset));

        // Corrupt the message and verify it fails
        message[0] = 0xFF;
        assert!(!verify_message_checksum(
            &message,
            checksum,
            checksum_offset
        ));
    }

    #[test]
    fn test_embed_checksum() {
        let mut message = vec![
            0x01, 0x02, 0x03, 0x04, // data
            0xFF, 0xFF, 0xFF, 0xFF, // checksum field (will be overwritten)
            0x05, 0x06,
        ]; // more data

        let checksum_offset = 4;
        embed_checksum(&mut message, checksum_offset);

        // Extract the embedded checksum
        let embedded_checksum = u32::from_le_bytes([
            message[checksum_offset],
            message[checksum_offset + 1],
            message[checksum_offset + 2],
            message[checksum_offset + 3],
        ]);

        // Verify it's correct
        assert!(verify_message_checksum(
            &message,
            embedded_checksum,
            checksum_offset
        ));
    }

    #[test]
    fn test_streaming_checksum() {
        let data1 = b"Hello, ";
        let data2 = b"world!";
        let combined = b"Hello, world!";

        // Calculate streaming checksum
        let mut streaming = StreamingChecksum::new();
        streaming.update(data1);
        streaming.update(data2);
        let streaming_result = streaming.finalize();

        // Should match single calculation
        let direct_result = calculate_crc32(combined);
        assert_eq!(streaming_result, direct_result);
    }

    #[test]
    fn test_streaming_reset() {
        let data = b"test data";

        let mut hasher = StreamingChecksum::new();
        hasher.update(data);
        let first_result = hasher.finalize();

        let mut hasher = StreamingChecksum::new();
        hasher.update(b"other data");
        hasher.reset();
        hasher.update(data);
        let second_result = hasher.finalize();

        assert_eq!(first_result, second_result);
    }
}
