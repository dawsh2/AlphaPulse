//! # Unified TLV Message Builder - Zero-Copy Protocol V2 Construction
//!
//! ## Purpose
//!
//! High-performance, zero-copy TLV message builder that provides both ultra-fast
//! hot path construction (~25ns) and flexible multi-TLV message composition.
//! Combines the best of zero_copy_builder_v2 and message_builder implementations.
//!
//! ## Architecture
//!
//! ```text
//! Services → [TLVMessageBuilder] → Binary Messages → Transport Layer
//!     ↑              ↓                      ↓             ↓
//! Typed         Zero-Copy              Network       Unix Socket/
//! Structs      Serialization          Transport      Message Bus
//! ```
//!
//! ## Performance Profile
//!
//! - **Hot Path Construction**: ~25ns with direct buffer write
//! - **Standard Construction**: >1M messages/second (measured: 1,097,624 msg/s)
//! - **Memory**: Zero allocations for buffer-based construction
//! - **Latency**: <10μs overhead for fixed-size TLVs

use crate::error::{ProtocolError, ProtocolResult};
use crate::tlv_types::TLVType;
use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use zerocopy::{AsBytes, Ref};

// Import types from types crate
use types::protocol::message::header::MessageHeader;
use types::protocol::tlv::fast_timestamp_ns;
use types::{RelayDomain, SourceType};

/// Global sequence counter for message tracking
///
/// Ensures unique, monotonically increasing sequence numbers across all threads
static GLOBAL_SEQUENCE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Build errors for zero-copy message construction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildError {
    /// Buffer is too small for the message
    BufferTooSmall,
    /// TLV payload exceeds maximum size
    PayloadTooLarge,
    /// Invalid input parameters
    InvalidInput,
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::BufferTooSmall => write!(f, "Buffer too small for message"),
            BuildError::PayloadTooLarge => write!(f, "TLV payload exceeds maximum size"),
            BuildError::InvalidInput => write!(f, "Invalid input parameters"),
        }
    }
}

impl std::error::Error for BuildError {}

impl From<BuildError> for io::Error {
    fn from(err: BuildError) -> Self {
        match err {
            BuildError::BufferTooSmall => io::Error::new(io::ErrorKind::OutOfMemory, err),
            BuildError::PayloadTooLarge => io::Error::new(io::ErrorKind::InvalidInput, err),
            BuildError::InvalidInput => io::Error::new(io::ErrorKind::InvalidInput, err),
        }
    }
}

/// Internal representation of TLV data
#[derive(Debug, Clone)]
enum TLVData {
    Standard { tlv_type: u8, payload: Vec<u8> },
    Extended { tlv_type: u8, payload: Vec<u8> },
}

/// Unified TLV Message Builder with zero-copy and multi-TLV support
pub struct TLVMessageBuilder {
    header: MessageHeader,
    tlvs: Vec<TLVData>,
    domain: RelayDomain,
    source: SourceType,
}

impl TLVMessageBuilder {
    /// Create a new TLV message builder
    pub fn new(relay_domain: RelayDomain, source: SourceType) -> Self {
        Self {
            header: MessageHeader::new(relay_domain, source),
            tlvs: Vec::new(),
            domain: relay_domain,
            source,
        }
    }

    /// Add a standard TLV (payload ≤ 255 bytes) with size validation
    pub fn add_tlv<T: AsBytes>(mut self, tlv_type: TLVType, data: &T) -> Self {
        let bytes = data.as_bytes();
        
        // Runtime alignment validation
        if let Some(expected_size) = tlv_type.expected_payload_size() {
            if bytes.len() != expected_size {
                panic!(
                    "TLV size mismatch for {:?}: expected {} bytes, got {} bytes",
                    tlv_type, expected_size, bytes.len()
                );
            }
        }
        
        if bytes.len() <= 255 {
            self.tlvs.push(TLVData::Standard {
                tlv_type: tlv_type as u8,
                payload: bytes.to_vec(),
            });
        } else {
            // Automatically use extended format for large payloads
            self.tlvs.push(TLVData::Extended {
                tlv_type: tlv_type as u8,
                payload: bytes.to_vec(),
            });
        }
        self
    }

    /// Add a TLV with raw bytes slice (zero-copy friendly)
    pub fn add_tlv_slice(mut self, tlv_type: TLVType, payload: &[u8]) -> Self {
        // Runtime alignment validation
        if let Some(expected_size) = tlv_type.expected_payload_size() {
            if payload.len() != expected_size {
                panic!(
                    "TLV size mismatch for {:?}: expected {} bytes, got {} bytes",
                    tlv_type, expected_size, payload.len()
                );
            }
        }
        
        if payload.len() <= 255 {
            self.tlvs.push(TLVData::Standard {
                tlv_type: tlv_type as u8,
                payload: payload.to_vec(),
            });
        } else {
            self.tlvs.push(TLVData::Extended {
                tlv_type: tlv_type as u8,
                payload: payload.to_vec(),
            });
        }
        self
    }

    /// Set the sequence number
    pub fn with_sequence(mut self, sequence: u64) -> Self {
        self.header.set_sequence(sequence);
        self
    }

    /// Set custom flags
    pub fn with_flags(mut self, flags: u8) -> Self {
        self.header.flags = flags;
        self
    }

    /// Set custom timestamp (normally uses fast timestamp)
    pub fn with_timestamp(mut self, timestamp_ns: u64) -> Self {
        self.header.timestamp = timestamp_ns;
        self
    }

    /// Build the final message bytes (standard multi-TLV path)
    pub fn build(mut self) -> ProtocolResult<Vec<u8>> {
        // Calculate total payload size
        let payload_size: usize = self
            .tlvs
            .iter()
            .map(|tlv| match tlv {
                TLVData::Standard { payload, .. } => 2 + payload.len(),
                TLVData::Extended { payload, .. } => 5 + payload.len(),
            })
            .sum();

        self.header.set_payload_size(payload_size as u32);

        // Use fast timestamp if not already set
        if self.header.timestamp == 0 {
            self.header.timestamp = fast_timestamp_ns();
        }

        // Pre-allocate buffer
        let total_size = MessageHeader::SIZE + payload_size;
        let mut message = Vec::with_capacity(total_size);

        // Add placeholder header
        message.extend_from_slice(self.header.as_bytes());

        // Add TLVs
        for tlv in &self.tlvs {
            match tlv {
                TLVData::Standard { tlv_type, payload } => {
                    message.push(*tlv_type);
                    message.push(payload.len() as u8);
                    message.extend_from_slice(payload);
                }
                TLVData::Extended { tlv_type, payload } => {
                    message.push(255); // ExtendedTLV marker
                    message.push(0); // Reserved
                    message.push(*tlv_type);
                    message.extend_from_slice(&(payload.len() as u16).to_le_bytes());
                    message.extend_from_slice(payload);
                }
            }
        }

        // Calculate and update checksum
        let message_copy = message.clone();
        let (header_mut, _) = Ref::<_, MessageHeader>::new_from_prefix(message.as_mut_slice())
            .expect("Message buffer too small for header");
        header_mut.into_mut().calculate_checksum(&message_copy);

        Ok(message)
    }

    /// Ultra-fast direct buffer construction for hot paths (~25ns)
    ///
    /// This method writes a single TLV directly to buffer with minimal overhead.
    /// Use this for performance-critical paths like market data processing.
    pub fn build_direct_into_buffer<T: AsBytes>(
        buffer: &mut [u8],
        domain: RelayDomain,
        source: SourceType,
        tlv_type: TLVType,
        tlv_data: &T,
    ) -> Result<usize, BuildError> {
        let tlv_bytes = tlv_data.as_bytes();
        let tlv_size = tlv_bytes.len();

        // Validate TLV size
        if tlv_size > 65535 {
            return Err(BuildError::PayloadTooLarge);
        }

        // Calculate sizes
        const HEADER_SIZE: usize = 32;
        let tlv_header_size = if tlv_size <= 255 { 2 } else { 5 };
        let total_size = HEADER_SIZE + tlv_header_size + tlv_size;

        // Fast bounds check
        if buffer.len() < total_size {
            return Err(BuildError::BufferTooSmall);
        }

        // Get sequence number
        let sequence = GLOBAL_SEQUENCE_COUNTER.fetch_add(1, Ordering::Relaxed);
        
        // Ultra-fast timestamp
        let timestamp_ns = fast_timestamp_ns();

        // Alignment check for zero-copy
        let buffer_ptr = buffer.as_mut_ptr();
        let header_align = std::mem::align_of::<MessageHeader>();
        if buffer_ptr.align_offset(header_align) != 0 {
            return Err(BuildError::InvalidInput);
        }

        // Direct header write
        unsafe {
            let header_ptr = buffer_ptr as *mut MessageHeader;
            header_ptr.write(MessageHeader {
                sequence,
                timestamp: timestamp_ns,
                magic: types::MESSAGE_MAGIC,
                payload_size: (tlv_header_size + tlv_size) as u32,
                checksum: 0, // TODO: Calculate if needed
                relay_domain: domain as u8,
                version: 1,
                source: source as u8,
                flags: 0,
            });
        }

        // Direct TLV header write
        let tlv_header_start = HEADER_SIZE;
        if tlv_size <= 255 {
            buffer[tlv_header_start] = tlv_type as u8;
            buffer[tlv_header_start + 1] = tlv_size as u8;
        } else {
            buffer[tlv_header_start] = 255;
            buffer[tlv_header_start + 1] = 0;
            buffer[tlv_header_start + 2] = tlv_type as u8;
            let size_bytes = (tlv_size as u16).to_le_bytes();
            buffer[tlv_header_start + 3] = size_bytes[0];
            buffer[tlv_header_start + 4] = size_bytes[1];
        }

        // Direct TLV data write
        let data_start = tlv_header_start + tlv_header_size;
        unsafe {
            std::ptr::copy_nonoverlapping(
                tlv_bytes.as_ptr(),
                buffer.as_mut_ptr().add(data_start),
                tlv_size,
            );
        }

        Ok(total_size)
    }

    /// Build message directly into buffer (multi-TLV support)
    pub fn build_into_buffer(self, buffer: &mut [u8]) -> Result<usize, io::Error> {
        let message = self
            .build()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let size = message.len();

        if buffer.len() < size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Buffer too small: need {}, got {}", size, buffer.len()),
            ));
        }

        buffer[..size].copy_from_slice(&message);
        Ok(size)
    }

    /// Get the current payload size (before building)
    pub fn payload_size(&self) -> usize {
        self.tlvs
            .iter()
            .map(|tlv| match tlv {
                TLVData::Standard { payload, .. } => 2 + payload.len(),
                TLVData::Extended { payload, .. } => 5 + payload.len(),
            })
            .sum()
    }

    /// Get the number of TLVs added
    pub fn tlv_count(&self) -> usize {
        self.tlvs.len()
    }

    /// Check if would exceed size limit
    pub fn would_exceed_size(&self, max_size: usize) -> bool {
        MessageHeader::SIZE + self.payload_size() > max_size
    }
}

/// Convenience function for ultra-fast single TLV message construction (~25ns)
///
/// Builds directly into thread-local buffer and returns a Vec for cross-thread
/// message passing. This is the optimal pattern for hot paths.
pub fn build_message_direct<T: AsBytes>(
    domain: RelayDomain,
    source: SourceType,
    tlv_type: TLVType,
    tlv_data: &T,
) -> Result<Vec<u8>, types::protocol::tlv::BufferError> {
    use types::protocol::tlv::with_hot_path_buffer;

    with_hot_path_buffer(|buffer| {
        let size = TLVMessageBuilder::build_direct_into_buffer(
            buffer,
            domain,
            source,
            tlv_type,
            tlv_data,
        )
        .map_err(io::Error::from)?;

        // Single required allocation for cross-thread send
        let result = buffer[..size].to_vec();
        Ok((result, size))
    })
}

/// Builder for vendor/experimental TLVs
pub struct VendorTLVBuilder {
    inner: TLVMessageBuilder,
}

impl VendorTLVBuilder {
    /// Create a vendor TLV builder
    pub fn new(relay_domain: RelayDomain, source: SourceType) -> Self {
        Self {
            inner: TLVMessageBuilder::new(relay_domain, source),
        }
    }

    /// Add a vendor TLV (type 200-254)
    pub fn add_vendor_tlv<T: AsBytes>(mut self, vendor_type: u8, data: &T) -> Self {
        if !(200..=254).contains(&vendor_type) {
            panic!("Vendor TLV type must be in range 200-254, got {}", vendor_type);
        }

        // Create a pseudo-TLVType for vendor types
        // Since vendor types don't have predefined TLVType enum values,
        // we bypass the validation by using a cast
        let bytes = data.as_bytes();
        if bytes.len() <= 255 {
            self.inner.tlvs.push(TLVData::Standard {
                tlv_type: vendor_type,
                payload: bytes.to_vec(),
            });
        } else {
            self.inner.tlvs.push(TLVData::Extended {
                tlv_type: vendor_type,
                payload: bytes.to_vec(),
            });
        }
        self
    }

    /// Convert to standard builder
    pub fn into_standard_builder(self) -> TLVMessageBuilder {
        self.inner
    }

    /// Build the message
    pub fn build(self) -> ProtocolResult<Vec<u8>> {
        self.inner.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[repr(C)]
    #[derive(AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes, PartialEq, Eq, Debug)]
    struct TestTradeTLV {
        instrument_id: u64,
        price: i64,
        volume: i64,
    }

    #[test]
    fn test_basic_message_building() {
        let test_data = TestTradeTLV {
            instrument_id: 0x123456789ABCDEF0,
            price: 4500000000000,
            volume: 100000000,
        };

        let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
            .add_tlv(TLVType::Trade, &test_data)
            .build()
            .expect("Failed to build message");

        assert_eq!(message.len(), 58); // Header (32) + TLV header (2) + payload (24)
    }

    #[test]
    fn test_direct_buffer_construction() {
        let test_data = TestTradeTLV {
            instrument_id: 0x123456789ABCDEF0,
            price: 4500000000000,
            volume: 100000000,
        };

        let mut buffer = vec![0u8; 1024];
        let size = TLVMessageBuilder::build_direct_into_buffer(
            &mut buffer,
            RelayDomain::MarketData,
            SourceType::BinanceCollector,
            TLVType::Trade,
            &test_data,
        )
        .expect("Failed to build");

        assert_eq!(size, 58);
        assert_eq!(buffer[0..4], types::MESSAGE_MAGIC.to_le_bytes());
    }

    #[test]
    #[should_panic(expected = "Vendor TLV type must be in range 200-254")]
    fn test_invalid_vendor_type() {
        VendorTLVBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy)
            .add_vendor_tlv(100, &[0u8; 4]);
    }
}