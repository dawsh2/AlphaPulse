//! Message Header Implementation
//!
//! The header is identical for all messages and contains routing and validation information.

use super::super::{ProtocolError, RelayDomain, SourceType, MESSAGE_MAGIC};
use crate::tlv::fast_timestamp_ns;
use zerocopy::{AsBytes, FromBytes, FromZeroes};

/// Message Header (32 bytes)
///
/// The header is identical for all messages and contains routing and validation information.
///
/// **CRITICAL**: Field ordering is carefully designed to achieve exactly 32 bytes
/// without padding. Fields are grouped by size (u64 → u32 → u8) to maintain
/// natural alignment. DO NOT REORDER without understanding padding implications.
///
/// ```text
/// ┌─────────────────┬─────────────────────────────────────┐
/// │ MessageHeader   │ TLV Payload                         │
/// │ (32 bytes)      │ (variable length)                   │
/// └─────────────────┴─────────────────────────────────────┘
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct MessageHeader {
    // CRITICAL: Magic MUST be first for immediate protocol identification (bytes 0-3)
    pub magic: u32, // 0xDEADBEEF (bytes 0-3)

    // Protocol metadata packed in remaining 4 bytes for alignment (bytes 4-7)
    pub relay_domain: u8, // Which relay handles this (1=market, 2=signal, 3=execution)
    pub version: u8,      // Protocol version
    pub source: u8,       // Source service type
    pub flags: u8,        // Compression, priority, etc.

    // Performance-critical fields - 8-byte aligned (bytes 8-23)
    pub sequence: u64,  // Monotonic sequence per source (bytes 8-15)
    pub timestamp: u64, // Nanoseconds since epoch (bytes 16-23)

    // Message metadata (bytes 24-31)
    pub payload_size: u32, // TLV payload bytes (bytes 24-27)
    pub checksum: u32,     // CRC32 of entire message (bytes 28-31)
}
// Total: EXACTLY 32 bytes with zero padding!

impl MessageHeader {
    /// Header size in bytes
    pub const SIZE: usize = 32;

    /// Create a new message header with ultra-fast timestamp
    ///
    /// Uses the global coarse clock + fine counter for ~5ns timestamp generation
    /// instead of SystemTime::now() which costs ~200ns per call.
    pub fn new(domain: RelayDomain, source: SourceType) -> Self {
        Self {
            // u64 fields first
            sequence: 0,
            timestamp: fast_timestamp_ns(), // ✅ ULTRA-FAST: ~5ns vs ~200ns
            // u32 fields
            magic: MESSAGE_MAGIC,
            payload_size: 0,
            checksum: 0, // Will be calculated when message is finalized
            // u8 fields
            relay_domain: domain as u8,
            version: crate::PROTOCOL_VERSION,
            source: source as u8,
            flags: 0,
        }
    }

    /// Validate the header format
    pub fn validate(&self) -> crate::Result<()> {
        if self.magic != MESSAGE_MAGIC {
            return Err(ProtocolError::Parse(crate::tlv::ParseError::InvalidMagic {
                expected: MESSAGE_MAGIC,
                actual: self.magic,
            }));
        }

        RelayDomain::try_from(self.relay_domain)
            .map_err(|_| ProtocolError::InvalidRelayDomain(self.relay_domain))?;

        SourceType::try_from(self.source).map_err(|_| {
            ProtocolError::Parse(crate::tlv::ParseError::UnknownSource(self.source))
        })?;

        Ok(())
    }

    /// Get the relay domain for this message
    pub fn get_relay_domain(&self) -> crate::Result<RelayDomain> {
        RelayDomain::try_from(self.relay_domain)
            .map_err(|_| ProtocolError::InvalidRelayDomain(self.relay_domain))
    }

    /// Get the source type for this message
    pub fn get_source_type(&self) -> crate::Result<SourceType> {
        SourceType::try_from(self.source)
            .map_err(|_| ProtocolError::Parse(crate::tlv::ParseError::UnknownSource(self.source)))
    }

    /// Set the sequence number (typically done by the relay)
    pub fn set_sequence(&mut self, seq: u64) {
        self.sequence = seq;
    }

    /// Set the payload size
    pub fn set_payload_size(&mut self, size: u32) {
        self.payload_size = size;
    }

    /// Calculate and set the checksum for the entire message
    pub fn calculate_checksum(&mut self, full_message: &[u8]) {
        self.checksum = 0;
        // CRC32 over entire message except checksum field (bytes 28-31)
        let checksum_offset = 28; // checksum field starts at byte 28
        let before_checksum = &full_message[..checksum_offset];
        let after_checksum = &full_message[checksum_offset + 4..Self::SIZE]; // skip 4 checksum bytes
        let payload = &full_message[Self::SIZE..];

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(before_checksum);
        hasher.update(after_checksum);
        hasher.update(payload);
        self.checksum = hasher.finalize();
    }

    /// Verify the checksum against the full message
    pub fn verify_checksum(&self, full_message: &[u8]) -> bool {
        let checksum_offset = 28; // checksum field starts at byte 28
        let before_checksum = &full_message[..checksum_offset];
        let after_checksum = &full_message[checksum_offset + 4..Self::SIZE]; // skip 4 checksum bytes
        let payload = &full_message[Self::SIZE..];

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(before_checksum);
        hasher.update(after_checksum);
        hasher.update(payload);
        let calculated = hasher.finalize();

        calculated == self.checksum
    }

    /// Get age of this message in nanoseconds
    pub fn age_ns(&self) -> u64 {
        current_timestamp_ns().saturating_sub(self.timestamp)
    }

    /// Check if this message is older than the given duration
    pub fn is_older_than(&self, max_age_ns: u64) -> bool {
        self.age_ns() > max_age_ns
    }
}

/// Get current timestamp in nanoseconds since Unix epoch (ultra-fast)
///
/// Uses the global coarse clock system for ~5ns performance instead of
/// SystemTime::now() which costs ~200ns. Maintains ±10μs accuracy.
pub fn current_timestamp_ns() -> u64 {
    fast_timestamp_ns()
}

/// Get precise system timestamp (fallback for critical operations)
///
/// Uses SystemTime::now() for perfect accuracy at the cost of ~200ns latency.
/// Use this sparingly for critical operations requiring perfect timestamp accuracy.
pub fn precise_timestamp_ns() -> u64 {
    // Use safe conversion from time module
    // Prevents silent truncation on overflow - will panic if timestamp overflows u64
    alphapulse_time::safe_system_timestamp_ns()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_size() {
        assert_eq!(std::mem::size_of::<MessageHeader>(), MessageHeader::SIZE);
        assert_eq!(MessageHeader::SIZE, 32);
    }

    #[test]
    fn test_header_creation() {
        let header = MessageHeader::new(RelayDomain::MarketData, SourceType::BinanceCollector);

        let magic = header.magic;
        let relay_domain = header.relay_domain;
        let source = header.source;
        let version = header.version;
        let timestamp = header.timestamp;
        assert_eq!(magic, MESSAGE_MAGIC);
        assert_eq!(relay_domain, RelayDomain::MarketData as u8);
        assert_eq!(source, SourceType::BinanceCollector as u8);
        assert_eq!(version, crate::PROTOCOL_VERSION);
        assert!(timestamp > 0);
    }

    #[test]
    fn test_header_validation() {
        let mut header = MessageHeader::new(RelayDomain::Signal, SourceType::ArbitrageStrategy);
        assert!(header.validate().is_ok());

        // Test invalid magic
        header.magic = 0x12345678;
        assert!(header.validate().is_err());

        // Fix magic, test invalid domain
        header.magic = MESSAGE_MAGIC;
        header.relay_domain = 99;
        assert!(header.validate().is_err());
    }

    #[test]
    fn test_checksum_calculation() {
        let header = MessageHeader::new(RelayDomain::Execution, SourceType::ExecutionEngine);
        let message_bytes = header.as_bytes();

        let mut header_mut = header;
        header_mut.calculate_checksum(message_bytes);

        // Checksum should now be non-zero
        let checksum = header_mut.checksum;
        assert_ne!(checksum, 0);

        // Verification should pass
        let message_with_checksum = header_mut.as_bytes();
        assert!(header_mut.verify_checksum(message_with_checksum));
    }

    #[test]
    fn test_age_calculation() {
        let mut header = MessageHeader::new(RelayDomain::MarketData, SourceType::KrakenCollector);

        // Set timestamp to 1 second ago
        header.timestamp = precise_timestamp_ns() - 1_000_000_000;

        let age = header.age_ns();
        assert!(age >= 1_000_000_000); // At least 1 second
        assert!(age < 2_000_000_000); // Less than 2 seconds (allowing for test execution time)

        assert!(header.is_older_than(500_000_000)); // 500ms
        assert!(!header.is_older_than(2_000_000_000)); // 2s
    }
}
