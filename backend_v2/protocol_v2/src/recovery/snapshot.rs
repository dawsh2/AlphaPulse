//! Snapshot Implementation for Recovery
//!
//! Handles Snapshot TLV (Type 101) for large gap recovery

use crate::tlv::{TLVMessageBuilder, TLVType};
use crate::{ProtocolError, RelayDomain, SourceType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use zerocopy::{AsBytes, FromBytes, FromZeroes};

/// Snapshot TLV (Type 101, variable length)
/// Contains compressed state checkpoint data
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, AsBytes, FromBytes, FromZeroes)]
pub struct SnapshotTLVHeader {
    pub tlv_type: u8,           // 101
    pub tlv_length: u8,         // Length of header (remaining data is variable)
    pub snapshot_id: u32,       // Unique snapshot identifier
    pub sequence: u64,          // Sequence number this snapshot represents
    pub timestamp: u64,         // When snapshot was created (nanoseconds)
    pub compression_type: u8,   // Compression algorithm used
    pub checksum: u32,          // CRC32 of uncompressed data
    pub uncompressed_size: u32, // Size of data after decompression
    pub reserved: [u8; 3],      // Alignment padding
}

/// Compression types for snapshot data
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    None = 0,
    Zlib = 1,
    Lz4 = 2,
    Zstd = 3,
}

impl CompressionType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(CompressionType::None),
            1 => Some(CompressionType::Zlib),
            2 => Some(CompressionType::Lz4),
            3 => Some(CompressionType::Zstd),
            _ => None,
        }
    }
}

/// Snapshot data structure (before compression)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotData {
    pub version: u8,
    pub relay_domain: u8,
    pub sequence: u64,
    pub timestamp: u64,
    pub state: RelayState,
}

/// Relay state that can be snapshotted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayState {
    pub consumer_sequences: HashMap<u32, u64>, // consumer_id -> last_sequence
    pub message_buffer: Vec<BufferedMessage>,  // Recent messages for recovery
    pub active_instruments: HashMap<u64, InstrumentInfo>, // instrument_id -> metadata
    pub system_state: SystemState,
}

/// Buffered message for retransmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferedMessage {
    pub sequence: u64,
    pub timestamp: u64,
    pub data: Vec<u8>,
}

/// Instrument information in snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentInfo {
    pub instrument_id: u64,
    pub symbol: String,
    pub last_price: Option<i64>,
    pub last_update: u64,
}

/// System state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub uptime_ns: u64,
    pub message_count: u64,
    pub error_count: u32,
    pub last_heartbeat: u64,
}

impl SnapshotData {
    /// Create a new snapshot from relay state
    pub fn new(relay_domain: RelayDomain, sequence: u64, state: RelayState) -> Self {
        Self {
            version: 1,
            relay_domain: relay_domain as u8,
            sequence,
            timestamp: current_timestamp_ns(),
            state,
        }
    }

    /// Serialize to bytes
    pub fn serialize(&self) -> Result<Vec<u8>, ProtocolError> {
        serde_json::to_vec(self)
            .map_err(|e| ProtocolError::Recovery(format!("Serialization failed: {}", e)))
    }

    /// Deserialize from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self, ProtocolError> {
        serde_json::from_slice(data)
            .map_err(|e| ProtocolError::Recovery(format!("Deserialization failed: {}", e)))
    }

    /// Get the age of this snapshot in nanoseconds
    pub fn age_ns(&self) -> u64 {
        current_timestamp_ns().saturating_sub(self.timestamp)
    }

    /// Check if snapshot is still fresh (less than max_age_ns)
    pub fn is_fresh(&self, max_age_ns: u64) -> bool {
        self.age_ns() < max_age_ns
    }
}

/// Snapshot builder and manager
pub struct SnapshotBuilder {
    snapshot_id: u32,
    compression: CompressionType,
}

impl SnapshotBuilder {
    /// Create a new snapshot builder
    pub fn new(snapshot_id: u32, compression: CompressionType) -> Self {
        Self {
            snapshot_id,
            compression,
        }
    }

    /// Build a snapshot message
    pub fn build_snapshot(
        &self,
        relay_domain: RelayDomain,
        source: SourceType,
        sequence: u64,
        state: RelayState,
    ) -> Result<Vec<u8>, ProtocolError> {
        let snapshot_data = SnapshotData::new(relay_domain, sequence, state);
        let serialized_data = snapshot_data.serialize()?;

        // Compress data if needed
        let (compressed_data, actual_compression) = match self.compression {
            CompressionType::None => (serialized_data.clone(), CompressionType::None),
            CompressionType::Zlib => {
                // For now, just return uncompressed data
                // In production, use flate2 or similar
                (serialized_data.clone(), CompressionType::None)
            }
            _ => {
                // Other compression types not implemented yet
                (serialized_data.clone(), CompressionType::None)
            }
        };

        // Calculate checksum of uncompressed data
        let checksum = crc32fast::hash(&serialized_data);

        // Create snapshot header
        let header = SnapshotTLVHeader {
            tlv_type: TLVType::Snapshot as u8,
            tlv_length: std::mem::size_of::<SnapshotTLVHeader>() as u8,
            snapshot_id: self.snapshot_id,
            sequence,
            timestamp: snapshot_data.timestamp,
            compression_type: actual_compression as u8,
            checksum,
            uncompressed_size: serialized_data.len() as u32,
            reserved: [0; 3],
        };

        // Build message with extended TLV if needed
        let mut builder = TLVMessageBuilder::new(relay_domain, source);

        // Combine header and compressed data
        let mut payload = Vec::new();
        payload.extend_from_slice(header.as_bytes());
        payload.extend_from_slice(&compressed_data);

        if payload.len() <= 255 {
            builder = builder.add_tlv_bytes(TLVType::Snapshot, payload);
        } else {
            // For extended TLVs with Vec<u8>, use add_tlv_bytes which handles this case
            builder = builder.add_tlv_bytes(TLVType::Snapshot, payload);
        }

        Ok(builder.build())
    }
}

/// Snapshot parser and loader
pub struct SnapshotLoader;

impl SnapshotLoader {
    /// Parse snapshot from TLV payload
    pub fn parse_snapshot(
        payload: &[u8],
    ) -> Result<(SnapshotTLVHeader, SnapshotData), ProtocolError> {
        if payload.len() < std::mem::size_of::<SnapshotTLVHeader>() {
            return Err(ProtocolError::Recovery(
                "Snapshot payload too small".to_string(),
            ));
        }

        // Parse header
        let header = zerocopy::Ref::<_, SnapshotTLVHeader>::new(
            &payload[..std::mem::size_of::<SnapshotTLVHeader>()],
        )
        .ok_or_else(|| ProtocolError::Recovery("Invalid snapshot header".to_string()))?
        .into_ref();

        // Extract compressed data
        let compressed_data = &payload[std::mem::size_of::<SnapshotTLVHeader>()..];

        // Decompress data
        let decompressed_data = match CompressionType::from_u8(header.compression_type) {
            Some(CompressionType::None) => compressed_data.to_vec(),
            Some(CompressionType::Zlib) => {
                // In production, decompress with flate2
                compressed_data.to_vec()
            }
            _ => {
                return Err(ProtocolError::Recovery(format!(
                    "Unsupported compression type: {}",
                    header.compression_type
                )));
            }
        };

        // Verify checksum (avoid direct field reference for packed struct)
        let expected_checksum = header.checksum;
        let calculated_checksum = crc32fast::hash(&decompressed_data);
        if calculated_checksum != expected_checksum {
            return Err(ProtocolError::Recovery(format!(
                "Snapshot checksum mismatch: expected {:#x}, got {:#x}",
                expected_checksum, calculated_checksum
            )));
        }

        // Verify size (avoid direct field reference for packed struct)
        let expected_size = header.uncompressed_size as usize;
        if decompressed_data.len() != expected_size {
            return Err(ProtocolError::Recovery(format!(
                "Snapshot size mismatch: expected {}, got {}",
                expected_size,
                decompressed_data.len()
            )));
        }

        // Deserialize snapshot data
        let snapshot_data = SnapshotData::deserialize(&decompressed_data)?;

        Ok((*header, snapshot_data))
    }

    /// Apply snapshot to restore relay state
    pub fn apply_snapshot(snapshot_data: SnapshotData) -> Result<(u64, RelayState), ProtocolError> {
        // Validate snapshot version
        if snapshot_data.version != 1 {
            return Err(ProtocolError::Recovery(format!(
                "Unsupported snapshot version: {}",
                snapshot_data.version
            )));
        }

        // Check if snapshot is reasonably fresh (less than 1 hour old)
        let max_age = 60 * 60 * 1_000_000_000u64; // 1 hour in nanoseconds
        if !snapshot_data.is_fresh(max_age) {
            return Err(ProtocolError::Recovery(format!(
                "Snapshot too old: {} seconds",
                snapshot_data.age_ns() / 1_000_000_000
            )));
        }

        Ok((snapshot_data.sequence, snapshot_data.state))
    }
}

/// Get current timestamp in nanoseconds
fn current_timestamp_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tlv::parser::{find_tlv_by_type, parse_header};

    fn create_test_relay_state() -> RelayState {
        let mut consumer_sequences = HashMap::new();
        consumer_sequences.insert(1, 100);
        consumer_sequences.insert(2, 150);

        let buffered_messages = vec![
            BufferedMessage {
                sequence: 95,
                timestamp: current_timestamp_ns(),
                data: vec![1, 2, 3],
            },
            BufferedMessage {
                sequence: 96,
                timestamp: current_timestamp_ns(),
                data: vec![4, 5, 6],
            },
        ];

        let mut active_instruments = HashMap::new();
        active_instruments.insert(
            12345,
            InstrumentInfo {
                instrument_id: 12345,
                symbol: "BTC/USD".to_string(),
                last_price: Some(50000_00000000), // $50,000 with 8 decimals
                last_update: current_timestamp_ns(),
            },
        );

        RelayState {
            consumer_sequences,
            message_buffer: buffered_messages,
            active_instruments,
            system_state: SystemState {
                uptime_ns: 1000000000, // 1 second
                message_count: 10000,
                error_count: 5,
                last_heartbeat: current_timestamp_ns(),
            },
        }
    }

    #[test]
    fn test_snapshot_data_serialization() {
        let state = create_test_relay_state();
        let snapshot = SnapshotData::new(RelayDomain::MarketData, 200, state);

        // Serialize and deserialize
        let serialized = snapshot.serialize().unwrap();
        let deserialized = SnapshotData::deserialize(&serialized).unwrap();

        assert_eq!(snapshot.version, deserialized.version);
        assert_eq!(snapshot.relay_domain, deserialized.relay_domain);
        assert_eq!(snapshot.sequence, deserialized.sequence);
        assert_eq!(
            snapshot.state.consumer_sequences,
            deserialized.state.consumer_sequences
        );
        assert_eq!(
            snapshot.state.message_buffer.len(),
            deserialized.state.message_buffer.len()
        );
    }

    #[test]
    fn test_snapshot_builder() {
        let builder = SnapshotBuilder::new(12345, CompressionType::None);
        let state = create_test_relay_state();

        let message = builder
            .build_snapshot(RelayDomain::Signal, SourceType::SignalRelay, 300, state)
            .unwrap();

        // Parse the message
        let header = parse_header(&message).unwrap();
        assert_eq!(header.relay_domain, RelayDomain::Signal as u8);
        assert_eq!(header.source, SourceType::SignalRelay as u8);

        // Extract snapshot TLV
        let tlv_payload = &message[32..];
        let snapshot_payload = find_tlv_by_type(tlv_payload, TLVType::Snapshot as u8).unwrap();

        // Parse snapshot
        let (snap_header, snap_data) = SnapshotLoader::parse_snapshot(snapshot_payload).unwrap();

        let snapshot_id = snap_header.snapshot_id;
        let snap_seq = snap_header.sequence;
        assert_eq!(snapshot_id, 12345);
        assert_eq!(snap_seq, 300);
        assert_eq!(snap_data.sequence, 300);
        assert_eq!(snap_data.relay_domain, RelayDomain::Signal as u8);
    }

    #[test]
    fn test_snapshot_application() {
        let state = create_test_relay_state();
        let snapshot = SnapshotData::new(RelayDomain::Execution, 500, state.clone());

        let (sequence, restored_state) = SnapshotLoader::apply_snapshot(snapshot).unwrap();

        assert_eq!(sequence, 500);
        assert_eq!(restored_state.consumer_sequences, state.consumer_sequences);
        assert_eq!(
            restored_state.message_buffer.len(),
            state.message_buffer.len()
        );
        // Check active instruments match
        assert_eq!(
            restored_state.active_instruments.len(),
            state.active_instruments.len()
        );
        for (key, value) in &state.active_instruments {
            let restored_value = restored_state.active_instruments.get(key);
            assert!(restored_value.is_some(), "Missing instrument {}", key);
            let restored_value = restored_value.unwrap();
            assert_eq!(restored_value.instrument_id, value.instrument_id);
            assert_eq!(restored_value.symbol, value.symbol);
        }
    }

    #[test]
    fn test_snapshot_freshness() {
        let state = create_test_relay_state();
        let mut snapshot = SnapshotData::new(RelayDomain::MarketData, 100, state);

        // Fresh snapshot
        assert!(snapshot.is_fresh(60 * 1_000_000_000)); // 60 seconds

        // Make it old
        snapshot.timestamp = current_timestamp_ns() - (2 * 60 * 60 * 1_000_000_000); // 2 hours ago
        assert!(!snapshot.is_fresh(60 * 60 * 1_000_000_000)); // 1 hour max age

        // Should fail application
        let result = SnapshotLoader::apply_snapshot(snapshot);
        assert!(result.is_err());
    }

    #[test]
    fn test_checksum_validation() {
        let builder = SnapshotBuilder::new(999, CompressionType::None);
        let state = create_test_relay_state();

        let mut message = builder
            .build_snapshot(
                RelayDomain::MarketData,
                SourceType::MarketDataRelay,
                100,
                state,
            )
            .unwrap();

        // First parse the original message to get the snapshot
        let header = parse_header(&message).unwrap();
        let tlv_payload = &message[32..];
        let mut snapshot_payload = find_tlv_by_type(tlv_payload, TLVType::Snapshot as u8)
            .unwrap()
            .to_vec();

        // Now corrupt just the snapshot payload data (after its own header)
        // SnapshotTLVHeader is about 34 bytes, so corrupt after that
        let header_size = std::mem::size_of::<SnapshotTLVHeader>();
        if snapshot_payload.len() > header_size + 4 {
            snapshot_payload[header_size + 4] ^= 0xFF; // Corrupt actual data payload
        }

        // Parsing the corrupted snapshot should fail due to data checksum mismatch
        let result = SnapshotLoader::parse_snapshot(&snapshot_payload);
        assert!(
            result.is_err(),
            "Should fail due to corrupted snapshot data"
        );
    }
}
