//! Network Message Envelope
//!
//! Wire protocol implementation for network transport messages.
//! Provides efficient binary serialization with fixed headers and
//! variable-length payload support.

use crate::network::{CompressionType, EncryptionType};
use crate::time::fast_timestamp_ns;
use crate::{generate_message_id, Result, TransportError};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read, Write};

/// Network message envelope for wire protocol
///
/// This envelope provides transport-layer metadata for messages flowing through
/// the AlphaPulse network infrastructure. It includes timing information that
/// allows precise measurement of network latency and total system latency.
///
/// ## Timestamp Architecture
///
/// The envelope contains two transport-level timestamps:
/// - `sent_at_ns`: When the message entered the network transport layer
/// - `received_at_ns`: When the message was received at destination
///
/// These are separate from any business-level timestamps contained in the payload.
///
/// ## Latency Measurement
///
/// Use the provided methods to calculate different types of latency:
/// - Network latency: `received_at_ns - sent_at_ns`
/// - Total latency: `received_at_ns - business_event_timestamp`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEnvelope {
    /// Unique message identifier
    pub message_id: u64,
    /// Source node identifier
    pub source_node: String,
    /// Target node identifier
    pub target_node: String,
    /// Target actor identifier
    pub target_actor: String,

    // Transport-layer timing (infrastructure timestamps)
    /// When message entered the transport layer (nanoseconds since UNIX epoch)
    /// Set automatically when envelope is created
    pub sent_at_ns: u64,
    /// When message was received at destination (nanoseconds since UNIX epoch)
    /// Set automatically when envelope is received (0 until then)
    pub received_at_ns: u64,

    /// Compression type used for payload
    pub compression: CompressionType,
    /// Encryption type used for payload
    pub encryption: EncryptionType,
    /// Message priority level
    pub priority: u8,
    /// Message flags
    pub flags: MessageFlags,
    /// Payload size in bytes
    pub payload_size: u32,
    /// Message payload (contains serialized business messages)
    pub payload: Vec<u8>,
}

/// Message flags for special handling
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct MessageFlags {
    /// Request acknowledgment from receiver
    pub ack_requested: bool,
    /// Message is a heartbeat/keepalive
    pub heartbeat: bool,
    /// Message is fragmented (part of larger message)
    pub fragmented: bool,
    /// Last fragment in a fragmented message
    pub last_fragment: bool,
    /// Message requires ordered delivery
    pub ordered: bool,
    /// Message is compressed
    pub compressed: bool,
    /// Message is encrypted
    pub encrypted: bool,
    /// Reserved for future use
    pub reserved: bool,
}

impl MessageFlags {
    /// Convert flags to byte representation
    pub fn to_byte(&self) -> u8 {
        let mut byte = 0u8;
        if self.ack_requested {
            byte |= 0b0000_0001;
        }
        if self.heartbeat {
            byte |= 0b0000_0010;
        }
        if self.fragmented {
            byte |= 0b0000_0100;
        }
        if self.last_fragment {
            byte |= 0b0000_1000;
        }
        if self.ordered {
            byte |= 0b0001_0000;
        }
        if self.compressed {
            byte |= 0b0010_0000;
        }
        if self.encrypted {
            byte |= 0b0100_0000;
        }
        if self.reserved {
            byte |= 0b1000_0000;
        }
        byte
    }

    /// Create flags from byte representation
    pub fn from_byte(byte: u8) -> Self {
        Self {
            ack_requested: (byte & 0b0000_0001) != 0,
            heartbeat: (byte & 0b0000_0010) != 0,
            fragmented: (byte & 0b0000_0100) != 0,
            last_fragment: (byte & 0b0000_1000) != 0,
            ordered: (byte & 0b0001_0000) != 0,
            compressed: (byte & 0b0010_0000) != 0,
            encrypted: (byte & 0b0100_0000) != 0,
            reserved: (byte & 0b1000_0000) != 0,
        }
    }
}

/// Wire format constants
pub struct WireFormat;

impl WireFormat {
    /// Protocol version
    pub const VERSION: u8 = 1;
    /// Magic bytes for message validation
    pub const MAGIC: &'static [u8; 4] = b"ALPH";
    /// Fixed header size in bytes
    pub const HEADER_SIZE: usize = 32;
    /// Maximum message size (16MB)
    pub const MAX_MESSAGE_SIZE: u32 = 16 * 1024 * 1024;
    /// Maximum string length for node/actor names
    pub const MAX_STRING_LENGTH: u16 = 255;
}

impl NetworkEnvelope {
    /// Create a new network envelope
    pub fn new(
        source_node: String,
        target_node: String,
        target_actor: String,
        payload: Vec<u8>,
        compression: CompressionType,
        encryption: EncryptionType,
    ) -> Self {
        let mut flags = MessageFlags::default();
        flags.compressed = !matches!(compression, CompressionType::None);
        flags.encrypted = !matches!(encryption, EncryptionType::None);

        Self {
            message_id: generate_message_id(),
            source_node,
            target_node,
            target_actor,
            sent_at_ns: fast_timestamp_ns(),
            received_at_ns: 0, // Set when message is received
            compression,
            encryption,
            priority: 1, // Normal priority
            flags,
            payload_size: payload.len() as u32,
            payload,
        }
    }

    /// Create a heartbeat message
    pub fn heartbeat(source_node: String, target_node: String) -> Self {
        let mut envelope = Self::new(
            source_node,
            target_node,
            "heartbeat".to_string(),
            Vec::new(),
            CompressionType::None,
            EncryptionType::None,
        );
        envelope.flags.heartbeat = true;
        envelope.priority = 0; // High priority for heartbeats
        envelope
    }

    /// Create an acknowledgment message
    pub fn acknowledgment(
        source_node: String,
        target_node: String,
        original_message_id: u64,
    ) -> Self {
        let payload = original_message_id.to_le_bytes().to_vec();
        Self::new(
            source_node,
            target_node,
            "ack".to_string(),
            payload,
            CompressionType::None,
            EncryptionType::None,
        )
    }

    /// Serialize envelope to wire format
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        if self.payload_size > WireFormat::MAX_MESSAGE_SIZE {
            return Err(TransportError::protocol(format!(
                "Message size {} exceeds maximum {}",
                self.payload_size,
                WireFormat::MAX_MESSAGE_SIZE
            )));
        }

        let mut buffer = Vec::new();

        // Write fixed header (32 bytes)
        buffer.write_all(WireFormat::MAGIC)?; // 4 bytes: magic
        buffer.write_u8(WireFormat::VERSION)?; // 1 byte: version
        buffer.write_u8(self.priority)?; // 1 byte: priority
        buffer.write_u8(self.flags.to_byte())?; // 1 byte: flags
        buffer.write_u8(self.compression.to_byte())?; // 1 byte: compression
        buffer.write_u64::<LittleEndian>(self.message_id)?; // 8 bytes: message_id
        buffer.write_u64::<LittleEndian>(self.sent_at_ns)?; // 8 bytes: sent timestamp
        buffer.write_u32::<LittleEndian>(self.payload_size)?; // 4 bytes: payload_size
        buffer.write_u32::<LittleEndian>(0)?; // 4 bytes: reserved

        // Write variable-length strings
        Self::write_string(&mut buffer, &self.source_node)?;
        Self::write_string(&mut buffer, &self.target_node)?;
        Self::write_string(&mut buffer, &self.target_actor)?;
        Self::write_encryption_info(&mut buffer, &self.encryption)?;

        // Write payload
        buffer.write_all(&self.payload)?;

        // Add checksum for integrity verification
        let checksum = Self::calculate_checksum(&buffer);
        buffer.write_u32::<LittleEndian>(checksum)?;

        Ok(buffer)
    }

    /// Deserialize envelope from wire format
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < WireFormat::HEADER_SIZE + 4 {
            // +4 for checksum
            return Err(TransportError::protocol("Message too short"));
        }

        let mut cursor = Cursor::new(data);

        // Verify checksum first
        let checksum_pos = data.len() - 4;
        let expected_checksum = u32::from_le_bytes([
            data[checksum_pos],
            data[checksum_pos + 1],
            data[checksum_pos + 2],
            data[checksum_pos + 3],
        ]);
        let actual_checksum = Self::calculate_checksum(&data[..checksum_pos]);

        if expected_checksum != actual_checksum {
            return Err(TransportError::protocol("Checksum mismatch"));
        }

        // Read fixed header
        let mut magic = [0u8; 4];
        cursor.read_exact(&mut magic)?;
        if magic != *WireFormat::MAGIC {
            return Err(TransportError::protocol("Invalid magic bytes"));
        }

        let version = cursor.read_u8()?;
        if version != WireFormat::VERSION {
            return Err(TransportError::protocol(format!(
                "Unsupported protocol version: {}",
                version
            )));
        }

        let priority = cursor.read_u8()?;
        let flags = MessageFlags::from_byte(cursor.read_u8()?);
        let compression = CompressionType::from_byte(cursor.read_u8()?)?;
        let message_id = cursor.read_u64::<LittleEndian>()?;
        let sent_at_ns = cursor.read_u64::<LittleEndian>()?;
        let payload_size = cursor.read_u32::<LittleEndian>()?;
        let _reserved = cursor.read_u32::<LittleEndian>()?;

        if payload_size > WireFormat::MAX_MESSAGE_SIZE {
            return Err(TransportError::protocol(format!(
                "Payload size {} exceeds maximum {}",
                payload_size,
                WireFormat::MAX_MESSAGE_SIZE
            )));
        }

        // Read variable-length strings
        let source_node = Self::read_string(&mut cursor)?;
        let target_node = Self::read_string(&mut cursor)?;
        let target_actor = Self::read_string(&mut cursor)?;
        let encryption = Self::read_encryption_info(&mut cursor)?;

        // Read payload
        let mut payload = vec![0u8; payload_size as usize];
        cursor.read_exact(&mut payload)?;

        Ok(NetworkEnvelope {
            message_id,
            source_node,
            target_node,
            target_actor,
            sent_at_ns,
            received_at_ns: 0, // Will be set when message is actually received
            compression,
            encryption,
            priority,
            flags,
            payload_size,
            payload,
        })
    }

    /// Mark message as received and set the received timestamp
    ///
    /// This should be called automatically by the transport layer when
    /// a message is received at its destination.
    pub fn mark_received(&mut self) {
        self.received_at_ns = fast_timestamp_ns();
    }

    /// Mark message as received with a specific timestamp
    ///
    /// This is useful for testing or when the received timestamp
    /// needs to be set to a specific value.
    pub fn mark_received_at(&mut self, timestamp_ns: u64) {
        self.received_at_ns = timestamp_ns;
    }

    /// Calculate network latency in nanoseconds
    ///
    /// This measures how long the message spent in transit through the
    /// AlphaPulse network infrastructure (received_at_ns - sent_at_ns).
    ///
    /// Returns 0 if the message hasn't been received yet.
    pub fn network_latency_ns(&self) -> u64 {
        if self.received_at_ns == 0 {
            0
        } else {
            self.received_at_ns.saturating_sub(self.sent_at_ns)
        }
    }

    /// Calculate total system latency from a business event timestamp
    ///
    /// This measures the total time from when a business event occurred
    /// (e.g., trade execution on exchange) to when the message was received.
    ///
    /// ## Parameters
    /// - `business_event_timestamp_ns`: When the business event occurred
    ///
    /// ## Returns
    /// Total latency in nanoseconds, or 0 if message hasn't been received yet.
    ///
    /// ## Example
    /// ```rust
    /// // For a trade message with execution timestamp from exchange
    /// let trade_tlv = TradeTLV { execution_timestamp_ns: 1234567890, /* ... */ };
    /// let total_latency = envelope.total_latency_from_event_ns(trade_tlv.execution_timestamp_ns);
    /// ```
    pub fn total_latency_from_event_ns(&self, business_event_timestamp_ns: u64) -> u64 {
        if self.received_at_ns == 0 {
            0
        } else {
            self.received_at_ns
                .saturating_sub(business_event_timestamp_ns)
        }
    }

    /// Get message age since it was sent (current time - sent_at_ns)
    ///
    /// This is useful for detecting stale messages or measuring how long
    /// a message has been in the system.
    pub fn age_nanos(&self) -> u64 {
        fast_timestamp_ns().saturating_sub(self.sent_at_ns)
    }

    /// Get message age in milliseconds
    pub fn age_millis(&self) -> f64 {
        self.age_nanos() as f64 / 1_000_000.0
    }

    /// Check if message is a heartbeat
    pub fn is_heartbeat(&self) -> bool {
        self.flags.heartbeat
    }

    /// Check if message requires acknowledgment
    pub fn requires_ack(&self) -> bool {
        self.flags.ack_requested
    }

    /// Check if message is fragmented
    pub fn is_fragmented(&self) -> bool {
        self.flags.fragmented
    }

    /// Write length-prefixed string to buffer
    fn write_string(buffer: &mut Vec<u8>, s: &str) -> Result<()> {
        let bytes = s.as_bytes();
        if bytes.len() > WireFormat::MAX_STRING_LENGTH as usize {
            return Err(TransportError::protocol(format!(
                "String too long: {} > {}",
                bytes.len(),
                WireFormat::MAX_STRING_LENGTH
            )));
        }

        buffer.write_u8(bytes.len() as u8)?;
        buffer.write_all(bytes)?;
        Ok(())
    }

    /// Read length-prefixed string from cursor
    fn read_string(cursor: &mut Cursor<&[u8]>) -> Result<String> {
        let len = cursor.read_u8()? as usize;
        let mut bytes = vec![0u8; len];
        cursor.read_exact(&mut bytes)?;
        String::from_utf8(bytes)
            .map_err(|e| TransportError::protocol(format!("Invalid UTF-8 string: {}", e)))
    }

    /// Write encryption information to buffer
    fn write_encryption_info(buffer: &mut Vec<u8>, encryption: &EncryptionType) -> Result<()> {
        buffer.write_u8(encryption.to_byte())?;

        // Write encryption-specific data if needed
        match encryption {
            EncryptionType::None => {}
            EncryptionType::Tls => {}
            EncryptionType::ChaCha20Poly1305 { .. } => {
                // Could write nonce or other encryption metadata here
            }
        }

        Ok(())
    }

    /// Read encryption information from cursor
    fn read_encryption_info(cursor: &mut Cursor<&[u8]>) -> Result<EncryptionType> {
        let encryption_byte = cursor.read_u8()?;
        EncryptionType::from_byte(encryption_byte)
    }

    /// Calculate CRC32 checksum for message integrity
    fn calculate_checksum(data: &[u8]) -> u32 {
        // Simple CRC32 implementation
        // In production, use a proper CRC32 library like `crc32fast`
        let mut crc = 0xFFFFFFFFu32;
        for &byte in data {
            crc ^= byte as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xEDB88320;
                } else {
                    crc >>= 1;
                }
            }
        }
        !crc
    }

    /// Validate envelope integrity
    pub fn validate(&self) -> Result<()> {
        // Check string lengths
        if self.source_node.len() > WireFormat::MAX_STRING_LENGTH as usize {
            return Err(TransportError::protocol("Source node name too long"));
        }
        if self.target_node.len() > WireFormat::MAX_STRING_LENGTH as usize {
            return Err(TransportError::protocol("Target node name too long"));
        }
        if self.target_actor.len() > WireFormat::MAX_STRING_LENGTH as usize {
            return Err(TransportError::protocol("Target actor name too long"));
        }

        // Check payload size
        if self.payload_size as usize != self.payload.len() {
            return Err(TransportError::protocol("Payload size mismatch"));
        }
        if self.payload_size > WireFormat::MAX_MESSAGE_SIZE {
            return Err(TransportError::protocol("Payload too large"));
        }

        // Check timestamp is reasonable (not too far in future/past)
        let now = fast_timestamp_ns();
        let age = now.saturating_sub(self.sent_at_ns);
        if age > 300_000_000_000 {
            // 5 minutes in nanoseconds
            return Err(TransportError::protocol("Message too old"));
        }
        if self.sent_at_ns > now + 60_000_000_000 {
            // 1 minute in future
            return Err(TransportError::protocol("Message timestamp in future"));
        }

        Ok(())
    }
}

impl CompressionType {
    /// Convert compression type to byte representation
    pub fn to_byte(&self) -> u8 {
        match self {
            CompressionType::None => 0,
            CompressionType::Lz4 => 1,
            CompressionType::Zstd => 2,
            CompressionType::Snappy => 3,
        }
    }

    /// Create compression type from byte representation
    pub fn from_byte(byte: u8) -> Result<Self> {
        match byte {
            0 => Ok(CompressionType::None),
            1 => Ok(CompressionType::Lz4),
            2 => Ok(CompressionType::Zstd),
            3 => Ok(CompressionType::Snappy),
            _ => Err(TransportError::protocol(format!(
                "Unknown compression type: {}",
                byte
            ))),
        }
    }
}

impl EncryptionType {
    /// Convert encryption type to byte representation
    pub fn to_byte(&self) -> u8 {
        match self {
            EncryptionType::None => 0,
            EncryptionType::Tls => 1,
            EncryptionType::ChaCha20Poly1305 { .. } => 2,
        }
    }

    /// Create encryption type from byte representation
    pub fn from_byte(byte: u8) -> Result<Self> {
        match byte {
            0 => Ok(EncryptionType::None),
            1 => Ok(EncryptionType::Tls),
            2 => Ok(EncryptionType::ChaCha20Poly1305 {
                key: [0u8; 32], // Will be set by security layer
            }),
            _ => Err(TransportError::protocol(format!(
                "Unknown encryption type: {}",
                byte
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_flags_serialization() {
        let flags = MessageFlags {
            ack_requested: true,
            heartbeat: false,
            fragmented: true,
            last_fragment: false,
            ordered: true,
            compressed: false,
            encrypted: true,
            reserved: false,
        };

        let byte = flags.to_byte();
        let restored = MessageFlags::from_byte(byte);

        assert_eq!(flags.ack_requested, restored.ack_requested);
        assert_eq!(flags.fragmented, restored.fragmented);
        assert_eq!(flags.ordered, restored.ordered);
        assert_eq!(flags.encrypted, restored.encrypted);
    }

    #[test]
    fn test_envelope_serialization() {
        let envelope = NetworkEnvelope::new(
            "node1".to_string(),
            "node2".to_string(),
            "actor1".to_string(),
            b"test payload".to_vec(),
            CompressionType::Lz4,
            EncryptionType::None,
        );

        let bytes = envelope.to_bytes().unwrap();
        let restored = NetworkEnvelope::from_bytes(&bytes).unwrap();

        assert_eq!(envelope.message_id, restored.message_id);
        assert_eq!(envelope.source_node, restored.source_node);
        assert_eq!(envelope.target_node, restored.target_node);
        assert_eq!(envelope.target_actor, restored.target_actor);
        assert_eq!(envelope.payload, restored.payload);
    }

    #[test]
    fn test_heartbeat_message() {
        let heartbeat = NetworkEnvelope::heartbeat("node1".to_string(), "node2".to_string());

        assert!(heartbeat.is_heartbeat());
        assert_eq!(heartbeat.priority, 0);
        assert_eq!(heartbeat.target_actor, "heartbeat");
    }

    #[test]
    fn test_checksum_validation() {
        let envelope = NetworkEnvelope::new(
            "node1".to_string(),
            "node2".to_string(),
            "actor1".to_string(),
            b"test".to_vec(),
            CompressionType::None,
            EncryptionType::None,
        );

        let mut bytes = envelope.to_bytes().unwrap();

        // Corrupt the checksum
        let len = bytes.len();
        bytes[len - 1] ^= 0xFF;

        let result = NetworkEnvelope::from_bytes(&bytes);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Checksum mismatch"));
    }

    #[test]
    fn test_envelope_validation() {
        let mut envelope = NetworkEnvelope::new(
            "node1".to_string(),
            "node2".to_string(),
            "actor1".to_string(),
            vec![0u8; 1000],
            CompressionType::None,
            EncryptionType::None,
        );

        // Valid envelope
        assert!(envelope.validate().is_ok());

        // Invalid: payload size mismatch
        envelope.payload_size = 500;
        assert!(envelope.validate().is_err());

        // Invalid: source node name too long
        envelope.payload_size = envelope.payload.len() as u32;
        envelope.source_node = "x".repeat(300);
        assert!(envelope.validate().is_err());
    }

    #[test]
    fn test_age_calculation() {
        let envelope = NetworkEnvelope::new(
            "node1".to_string(),
            "node2".to_string(),
            "actor1".to_string(),
            Vec::new(),
            CompressionType::None,
            EncryptionType::None,
        );

        // Age should be very small for a just-created message
        assert!(envelope.age_millis() < 100.0);
    }

    #[test]
    fn test_compression_type_conversion() {
        assert_eq!(CompressionType::None.to_byte(), 0);
        assert_eq!(CompressionType::Lz4.to_byte(), 1);
        assert_eq!(CompressionType::Zstd.to_byte(), 2);
        assert_eq!(CompressionType::Snappy.to_byte(), 3);

        assert!(matches!(
            CompressionType::from_byte(0).unwrap(),
            CompressionType::None
        ));
        assert!(matches!(
            CompressionType::from_byte(1).unwrap(),
            CompressionType::Lz4
        ));
        assert!(CompressionType::from_byte(255).is_err());
    }
}
