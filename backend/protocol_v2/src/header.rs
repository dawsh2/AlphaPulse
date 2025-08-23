//! Message Header Implementation
//! 
//! The header is identical for all messages and contains routing and validation information.

use crate::{MESSAGE_MAGIC, RelayDomain, SourceType, ProtocolError};
use zerocopy::{AsBytes, FromBytes, FromZeroes};
use std::time::{SystemTime, UNIX_EPOCH};

/// Message Header (32 bytes)
/// 
/// The header is identical for all messages and contains routing and validation information:
/// 
/// ```text
/// ┌─────────────────┬─────────────────────────────────────┐
/// │ MessageHeader   │ TLV Payload                         │
/// │ (32 bytes)      │ (variable length)                   │
/// └─────────────────┴─────────────────────────────────────┘
/// ```
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct MessageHeader {
    pub magic: u32,                 // 0xDEADBEEF
    pub relay_domain: u8,           // Which relay handles this (1=market, 2=signal, 3=execution)
    pub version: u8,                // Protocol version
    pub source: u8,                 // Source service type
    pub flags: u8,                  // Compression, priority, etc.
    pub payload_size: u32,          // TLV payload bytes
    pub sequence: u64,              // Monotonic sequence per source
    pub timestamp: u64,             // Nanoseconds since epoch
    pub checksum: u32,              // CRC32 of entire message
}

impl MessageHeader {
    /// Header size in bytes
    pub const SIZE: usize = 32;
    
    /// Create a new message header
    pub fn new(domain: RelayDomain, source: SourceType) -> Self {
        Self {
            magic: MESSAGE_MAGIC,
            relay_domain: domain as u8,
            version: crate::PROTOCOL_VERSION,
            source: source as u8,
            flags: 0,
            payload_size: 0,
            sequence: 0,
            timestamp: current_timestamp_ns(),
            checksum: 0, // Will be calculated when message is finalized
        }
    }
    
    /// Validate the header format
    pub fn validate(&self) -> crate::Result<()> {
        if self.magic != MESSAGE_MAGIC {
            return Err(ProtocolError::Parse(crate::tlv::ParseError::InvalidMagic { 
                expected: MESSAGE_MAGIC, 
                actual: self.magic 
            }));
        }
        
        RelayDomain::try_from(self.relay_domain)
            .map_err(|_| ProtocolError::InvalidRelayDomain(self.relay_domain))?;
        
        SourceType::try_from(self.source)
            .map_err(|_| ProtocolError::Parse(crate::tlv::ParseError::UnknownSource(self.source)))?;
        
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
        // CRC32 over entire message except checksum field (last 4 bytes)
        let checksum_offset = Self::SIZE - 4;
        let before_checksum = &full_message[..checksum_offset];
        let after_checksum = &full_message[Self::SIZE..];
        
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(before_checksum);
        hasher.update(after_checksum);
        self.checksum = hasher.finalize();
    }
    
    /// Verify the checksum against the full message
    pub fn verify_checksum(&self, full_message: &[u8]) -> bool {
        let checksum_offset = Self::SIZE - 4;
        let before_checksum = &full_message[..checksum_offset];
        let after_checksum = &full_message[Self::SIZE..];
        
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(before_checksum);
        hasher.update(after_checksum);
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

/// Get current timestamp in nanoseconds since Unix epoch
pub fn current_timestamp_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
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
        header.timestamp = current_timestamp_ns() - 1_000_000_000;
        
        let age = header.age_ns();
        assert!(age >= 1_000_000_000);  // At least 1 second
        assert!(age < 2_000_000_000);   // Less than 2 seconds (allowing for test execution time)
        
        assert!(header.is_older_than(500_000_000));  // 500ms
        assert!(!header.is_older_than(2_000_000_000)); // 2s
    }
}