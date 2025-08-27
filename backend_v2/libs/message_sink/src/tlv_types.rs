use crate::SinkError;
use std::fmt::Debug;

// Import TLVType and RelayDomain from the canonical location in libs/types
use alphapulse_types::protocol::RelayDomain;
use alphapulse_types::protocol::tlv::types::TLVType;
use num_enum::TryFromPrimitive;

/// TLV message type safety - prevents mixing different protocol message types
#[derive(Debug, Clone)]
pub enum MessageType {
    /// Protocol V2 TLV message with validation
    TLV(TLVMessage),
    /// Raw bytes for non-TLV protocols
    Raw(Vec<u8>),
}

impl MessageType {
    /// Create TLV message type with validation
    pub fn tlv(tlv_message: TLVMessage) -> Result<Self, SinkError> {
        // Validate TLV structure
        tlv_message.validate()?;
        Ok(Self::TLV(tlv_message))
    }

    /// Create raw message type
    pub fn raw(payload: Vec<u8>) -> Self {
        Self::Raw(payload)
    }

    /// Get message size in bytes
    pub fn size(&self) -> usize {
        match self {
            MessageType::TLV(tlv) => tlv.total_size(),
            MessageType::Raw(bytes) => bytes.len(),
        }
    }

    /// Extract raw bytes for transmission
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            MessageType::TLV(tlv) => tlv.serialize(),
            MessageType::Raw(bytes) => bytes,
        }
    }

    /// Check if this is a TLV message
    pub fn is_tlv(&self) -> bool {
        matches!(self, MessageType::TLV(_))
    }

    /// Check if this is a raw message
    pub fn is_raw(&self) -> bool {
        matches!(self, MessageType::Raw(_))
    }

    /// Get TLV message if this is a TLV type
    pub fn as_tlv(&self) -> Option<&TLVMessage> {
        match self {
            MessageType::TLV(tlv) => Some(tlv),
            MessageType::Raw(_) => None,
        }
    }

    /// Get TLV type for routing
    pub fn tlv_type(&self) -> Option<TLVType> {
        self.as_tlv().map(|tlv| tlv.tlv_type())
    }

    /// Get message domain for relay routing
    pub fn relay_domain(&self) -> Option<RelayDomain> {
        self.tlv_type().map(|tlv_type| tlv_type.relay_domain())
    }
}

/// TLV message wrapper with validation
#[derive(Debug, Clone)]
pub struct TLVMessage {
    /// TLV type identifier
    pub tlv_type: TLVType,
    /// Message payload
    pub payload: Vec<u8>,
    /// Optional message header (32 bytes for Protocol V2)
    pub header: Option<MessageHeader>,
}

impl TLVMessage {
    /// Create new TLV message
    pub fn new(tlv_type: TLVType, payload: Vec<u8>) -> Self {
        Self {
            tlv_type,
            payload,
            header: None,
        }
    }

    /// Create TLV message with header
    pub fn with_header(tlv_type: TLVType, payload: Vec<u8>, header: MessageHeader) -> Self {
        Self {
            tlv_type,
            payload,
            header: Some(header),
        }
    }

    /// Get TLV type
    pub fn tlv_type(&self) -> TLVType {
        self.tlv_type
    }

    /// Get total message size (header + payload)
    pub fn total_size(&self) -> usize {
        let header_size = self.header.as_ref().map(|_| 32).unwrap_or(0);
        header_size + self.payload.len()
    }

    /// Validate TLV structure
    pub fn validate(&self) -> Result<(), SinkError> {
        // Check payload size matches expected for TLV type
        if let Some(expected_size) = self.tlv_type.expected_payload_size() {
            if self.payload.len() != expected_size {
                return Err(SinkError::invalid_config(format!(
                    "TLV payload size mismatch: expected {} bytes for {:?}, got {}",
                    expected_size,
                    self.tlv_type,
                    self.payload.len()
                )));
            }
        }

        // Validate minimum payload size for all TLV types
        // No TLV types allow empty payloads in Protocol V2
        if self.payload.is_empty() {
            return Err(SinkError::invalid_config(format!(
                "TLV type {:?} requires non-empty payload",
                self.tlv_type
            )));
        }

        // Validate header if present
        if let Some(header) = &self.header {
            header.validate()?;

            // Check header payload size matches actual payload
            if header.payload_size as usize != self.payload.len() {
                return Err(SinkError::invalid_config(format!(
                    "Header payload size {} doesn't match actual payload size {}",
                    header.payload_size,
                    self.payload.len()
                )));
            }
        }

        Ok(())
    }

    /// Serialize TLV message to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(self.total_size());

        // Add header if present
        if let Some(header) = &self.header {
            result.extend_from_slice(&header.serialize());
        }

        // Add TLV type and length
        result.extend_from_slice(&(self.tlv_type as u16).to_le_bytes());
        result.extend_from_slice(&(self.payload.len() as u16).to_le_bytes());

        // Add payload
        result.extend_from_slice(&self.payload);

        result
    }

    /// Parse TLV message from bytes
    pub fn parse(bytes: &[u8]) -> Result<Self, SinkError> {
        if bytes.len() < 4 {
            return Err(SinkError::invalid_config(
                "TLV message too short".to_string(),
            ));
        }

        let tlv_type = u16::from_le_bytes([bytes[0], bytes[1]]);
        let payload_len = u16::from_le_bytes([bytes[2], bytes[3]]) as usize;

        if bytes.len() < 4 + payload_len {
            return Err(SinkError::invalid_config(
                "TLV payload truncated".to_string(),
            ));
        }

        let tlv_type = TLVType::try_from(tlv_type)?;
        let payload = bytes[4..4 + payload_len].to_vec();

        let message = Self::new(tlv_type, payload);
        message.validate()?;
        Ok(message)
    }
}

/// Protocol V2 message header (32 bytes)
#[derive(Debug, Clone)]
pub struct MessageHeader {
    /// Magic number for protocol identification
    pub magic: u32,
    /// Payload size in bytes
    pub payload_size: u32,
    /// Source relay domain
    pub relay_domain: u8,
    /// Message source identifier
    pub source: u8,
    /// Sequence number for ordering
    pub sequence: u64,
    /// Reserved bytes for future use
    pub reserved: [u8; 14],
}

impl MessageHeader {
    /// Protocol V2 magic number
    pub const MAGIC: u32 = 0xDEADBEEF;

    /// Header size in bytes
    pub const SIZE: usize = 32;

    /// Create new message header
    pub fn new(payload_size: u32, relay_domain: u8, source: u8, sequence: u64) -> Self {
        Self {
            magic: Self::MAGIC,
            payload_size,
            relay_domain,
            source,
            sequence,
            reserved: [0; 14],
        }
    }

    /// Validate header
    pub fn validate(&self) -> Result<(), SinkError> {
        if self.magic != Self::MAGIC {
            return Err(SinkError::invalid_config(format!(
                "Invalid magic number: expected 0x{:08X}, got 0x{:08X}",
                Self::MAGIC,
                self.magic
            )));
        }

        // Validate relay domain ranges
        if !matches!(self.relay_domain, 1..=3) {
            return Err(SinkError::invalid_config(format!(
                "Invalid relay domain: {} (must be 1-3)",
                self.relay_domain
            )));
        }

        Ok(())
    }

    /// Serialize header to bytes
    pub fn serialize(&self) -> [u8; 32] {
        let mut result = [0u8; 32];
        result[0..4].copy_from_slice(&self.magic.to_le_bytes());
        result[4..8].copy_from_slice(&self.payload_size.to_le_bytes());
        result[8] = self.relay_domain;
        result[9] = self.source;
        result[10..18].copy_from_slice(&self.sequence.to_le_bytes());
        result[18..32].copy_from_slice(&self.reserved);
        result
    }

    /// Parse header from bytes
    pub fn parse(bytes: &[u8]) -> Result<Self, SinkError> {
        if bytes.len() < 32 {
            return Err(SinkError::invalid_config("Header too short".to_string()));
        }

        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let payload_size = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let relay_domain = bytes[8];
        let source = bytes[9];
        let sequence = u64::from_le_bytes([
            bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15], bytes[16], bytes[17],
        ]);

        let mut reserved = [0u8; 14];
        reserved.copy_from_slice(&bytes[18..32]);

        let header = Self {
            magic,
            payload_size,
            relay_domain,
            source,
            sequence,
            reserved,
        };

        header.validate()?;
        Ok(header)
    }
}

// TLVType is now imported from alphapulse_types::protocol::tlv::types
// This provides the complete Protocol V2 type registry with all domains

// TLVType methods are now available from the imported type in libs/types
// The canonical implementation provides:
// - relay_domain() method for routing (replaces domain())
// - expected_payload_size() for fixed-size types
// - size_constraint() for comprehensive size validation

// TryFrom<u8> is implemented in the canonical TLVType via num_enum::TryFromPrimitive
// We provide a u16 to u8 conversion wrapper for backward compatibility
impl TryFrom<u16> for TLVType {
    type Error = SinkError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if value > 255 {
            return Err(SinkError::invalid_config(format!(
                "TLV type {} exceeds u8 range",
                value
            )));
        }
        
        let type_u8 = value as u8;
        TLVType::try_from(type_u8).map_err(|_| {
            SinkError::invalid_config(format!("Unknown TLV type: {}", value))
        })
    }
}

// MessageDomain is replaced by RelayDomain from libs/types
// RelayDomain provides the same functionality with consistent domain mapping

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tlv_message_creation() {
        let payload = vec![1, 2, 3, 4];
        let tlv_msg = TLVMessage::new(TLVType::Quote, payload.clone());

        assert_eq!(tlv_msg.tlv_type(), TLVType::Quote);
        assert_eq!(tlv_msg.payload, payload);
        assert_eq!(tlv_msg.total_size(), 4);
    }

    #[test]
    fn test_tlv_message_validation() {
        // Valid quote message (52 bytes expected as per Protocol V2)
        let valid_payload = vec![0u8; 52];
        let valid_msg = TLVMessage::new(TLVType::Quote, valid_payload);
        assert!(valid_msg.validate().is_ok());

        // Invalid quote message (wrong size)
        let invalid_payload = vec![0u8; 10];
        let invalid_msg = TLVMessage::new(TLVType::Quote, invalid_payload);
        assert!(invalid_msg.validate().is_err());

        // Variable size message (should be valid)
        let variable_payload = vec![0u8; 100];
        let variable_msg = TLVMessage::new(TLVType::OrderBook, variable_payload);
        assert!(variable_msg.validate().is_ok());
    }

    #[test]
    fn test_message_header() {
        let header = MessageHeader::new(100, 1, 42, 12345);

        assert_eq!(header.magic, MessageHeader::MAGIC);
        assert_eq!(header.payload_size, 100);
        assert_eq!(header.relay_domain, 1);
        assert_eq!(header.source, 42);
        assert_eq!(header.sequence, 12345);

        assert!(header.validate().is_ok());
    }

    #[test]
    fn test_header_serialization() {
        let original = MessageHeader::new(200, 2, 50, 67890);
        let bytes = original.serialize();
        let parsed = MessageHeader::parse(&bytes).unwrap();

        assert_eq!(original.magic, parsed.magic);
        assert_eq!(original.payload_size, parsed.payload_size);
        assert_eq!(original.relay_domain, parsed.relay_domain);
        assert_eq!(original.source, parsed.source);
        assert_eq!(original.sequence, parsed.sequence);
    }

    #[test]
    fn test_tlv_serialization() {
        let payload = vec![1, 2, 3, 4, 5];
        let tlv_msg = TLVMessage::new(TLVType::Trade, payload);

        let bytes = tlv_msg.serialize();
        let parsed = TLVMessage::parse(&bytes).unwrap();

        assert_eq!(tlv_msg.tlv_type, parsed.tlv_type);
        assert_eq!(tlv_msg.payload, parsed.payload);
    }

    #[test]
    fn test_message_type() {
        let tlv_msg = TLVMessage::new(TLVType::OrderRequest, vec![0u8; 32]);
        let msg_type = MessageType::tlv(tlv_msg).unwrap();

        assert!(msg_type.is_tlv());
        assert!(!msg_type.is_raw());
        assert_eq!(msg_type.tlv_type(), Some(TLVType::OrderRequest));
        assert_eq!(msg_type.relay_domain(), Some(RelayDomain::Execution));
        assert_eq!(msg_type.size(), 32);
    }

    #[test]
    fn test_tlv_type_domains() {
        assert_eq!(TLVType::Trade.relay_domain(), RelayDomain::MarketData);
        assert_eq!(TLVType::Quote.relay_domain(), RelayDomain::MarketData);
        assert_eq!(TLVType::SignalIdentity.relay_domain(), RelayDomain::Signal);
        assert_eq!(TLVType::OrderRequest.relay_domain(), RelayDomain::Execution);
    }

    #[test]
    fn test_invalid_magic_number() {
        let mut header = MessageHeader::new(100, 1, 42, 12345);
        header.magic = 0x12345678; // Wrong magic

        assert!(header.validate().is_err());
    }

    #[test]
    fn test_invalid_relay_domain() {
        let header = MessageHeader::new(100, 99, 42, 12345); // Invalid domain
        assert!(header.validate().is_err());
    }
}
