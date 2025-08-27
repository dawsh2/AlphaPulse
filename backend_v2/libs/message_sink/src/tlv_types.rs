use crate::SinkError;
use std::fmt::Debug;

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
    pub fn message_domain(&self) -> Option<MessageDomain> {
        self.tlv_type().map(|tlv_type| tlv_type.domain())
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
        if self.payload.is_empty() && !self.tlv_type.allows_empty_payload() {
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

/// TLV type enumeration for Protocol V2
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum TLVType {
    // Market Data Domain (1-19)
    Trade = 1,
    Quote = 2,
    OrderBook = 3,
    OHLC = 4,
    Volume = 5,

    // Signal Domain (20-39)
    SignalIdentity = 20,
    ArbitrageSignal = 21,
    TrendSignal = 22,

    // Execution Domain (40-79)
    ExecutionOrder = 40,
    ExecutionResult = 41,
    PositionUpdate = 42,
}

impl TLVType {
    /// Get message domain for this TLV type
    pub fn domain(&self) -> MessageDomain {
        match *self as u16 {
            1..=19 => MessageDomain::MarketData,
            20..=39 => MessageDomain::Signals,
            40..=79 => MessageDomain::Execution,
            _ => MessageDomain::Unknown,
        }
    }

    /// Get expected payload size for this TLV type
    pub fn expected_payload_size(&self) -> Option<usize> {
        match self {
            TLVType::Trade => Some(32),          // Fixed size trade structure
            TLVType::Quote => Some(24),          // Fixed size quote structure
            TLVType::ExecutionOrder => Some(48), // Fixed size order structure
            TLVType::SignalIdentity => Some(16), // Fixed size signal ID
            // Variable size types - return minimum expected size
            TLVType::OrderBook => Some(8), // Min: header + at least one entry
            TLVType::ArbitrageSignal => Some(20), // Min: pair addresses
            TLVType::TrendSignal => Some(12), // Min: signal data
            TLVType::OHLC => Some(40),     // Fixed: open,high,low,close,volume
            TLVType::Volume => Some(16),   // Fixed: buy,sell volumes
            TLVType::ExecutionResult => Some(32), // Fixed: result structure
            TLVType::PositionUpdate => Some(24), // Fixed: position data
        }
    }

    /// Get minimum payload size for validation
    pub fn minimum_payload_size(&self) -> usize {
        match self {
            // All TLV types should have at least some data
            TLVType::OrderBook | TLVType::ArbitrageSignal | TLVType::TrendSignal => 4,
            _ => self.expected_payload_size().unwrap_or(1),
        }
    }

    /// Check if this TLV type allows empty payloads
    pub fn allows_empty_payload(&self) -> bool {
        // Most TLV types require non-empty payloads
        false
    }
}

impl TryFrom<u16> for TLVType {
    type Error = SinkError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(TLVType::Trade),
            2 => Ok(TLVType::Quote),
            3 => Ok(TLVType::OrderBook),
            4 => Ok(TLVType::OHLC),
            5 => Ok(TLVType::Volume),
            20 => Ok(TLVType::SignalIdentity),
            21 => Ok(TLVType::ArbitrageSignal),
            22 => Ok(TLVType::TrendSignal),
            40 => Ok(TLVType::ExecutionOrder),
            41 => Ok(TLVType::ExecutionResult),
            42 => Ok(TLVType::PositionUpdate),
            _ => Err(SinkError::invalid_config(format!(
                "Unknown TLV type: {}",
                value
            ))),
        }
    }
}

/// Message domains for relay routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MessageDomain {
    Unknown = 0,
    MarketData = 1,
    Signals = 2,
    Execution = 3,
}

impl From<u8> for MessageDomain {
    fn from(value: u8) -> Self {
        match value {
            1 => MessageDomain::MarketData,
            2 => MessageDomain::Signals,
            3 => MessageDomain::Execution,
            _ => MessageDomain::Unknown,
        }
    }
}

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
        // Valid quote message (24 bytes expected)
        let valid_payload = vec![0u8; 24];
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
        let tlv_msg = TLVMessage::new(TLVType::ExecutionOrder, vec![0u8; 48]);
        let msg_type = MessageType::tlv(tlv_msg).unwrap();

        assert!(msg_type.is_tlv());
        assert!(!msg_type.is_raw());
        assert_eq!(msg_type.tlv_type(), Some(TLVType::ExecutionOrder));
        assert_eq!(msg_type.message_domain(), Some(MessageDomain::Execution));
        assert_eq!(msg_type.size(), 48);
    }

    #[test]
    fn test_tlv_type_domains() {
        assert_eq!(TLVType::Trade.domain(), MessageDomain::MarketData);
        assert_eq!(TLVType::Quote.domain(), MessageDomain::MarketData);
        assert_eq!(TLVType::SignalIdentity.domain(), MessageDomain::Signals);
        assert_eq!(TLVType::ExecutionOrder.domain(), MessageDomain::Execution);
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
