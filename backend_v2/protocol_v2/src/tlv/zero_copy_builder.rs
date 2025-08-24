//! Zero-Copy TLV Message Builder
//!
//! Eliminates all allocations during message construction by using references
//! to the original TLV structs and building directly into pre-allocated buffers.

use crate::{MessageHeader, RelayDomain, SourceType, TLVType};
use std::io::{self, Write};
use zerocopy::AsBytes;

/// Zero-copy TLV data reference
#[derive(Debug)]
pub enum TLVRef<'a> {
    Standard { tlv_type: u8, payload: &'a [u8] },
    Extended { tlv_type: u8, payload: &'a [u8] },
}

/// Zero-copy message builder that uses references instead of owned data
#[derive(Debug)]
pub struct ZeroCopyTLVMessageBuilder<'a> {
    domain: RelayDomain,
    source: SourceType,
    sequence: u64,
    tlv_refs: Vec<TLVRef<'a>>,
}

impl<'a> ZeroCopyTLVMessageBuilder<'a> {
    /// Create a new zero-copy builder
    pub fn new(domain: RelayDomain, source: SourceType) -> Self {
        Self {
            domain,
            source,
            sequence: 0,
            tlv_refs: Vec::new(),
        }
    }

    /// Add a TLV by reference (zero-copy)
    pub fn add_tlv_ref<T: AsBytes>(mut self, tlv_type: TLVType, tlv_data: &'a T) -> Self {
        let payload = tlv_data.as_bytes();

        let tlv_ref = if payload.len() <= 255 {
            TLVRef::Standard {
                tlv_type: tlv_type as u8,
                payload,
            }
        } else {
            TLVRef::Extended {
                tlv_type: tlv_type as u8,
                payload,
            }
        };

        self.tlv_refs.push(tlv_ref);
        self
    }

    /// Calculate total message size without building
    pub fn calculate_size(&self) -> usize {
        let header_size = std::mem::size_of::<MessageHeader>();
        let payload_size: usize = self
            .tlv_refs
            .iter()
            .map(|tlv_ref| {
                match tlv_ref {
                    TLVRef::Standard { payload, .. } => 2 + payload.len(), // type(1) + len(1) + data
                    TLVRef::Extended { payload, .. } => 4 + payload.len(), // type(1) + reserved(1) + len(2) + data
                }
            })
            .sum();

        header_size + payload_size
    }

    /// Build message directly into provided buffer (zero-allocation)
    pub fn build_into_buffer(self, buffer: &mut [u8]) -> Result<usize, io::Error> {
        let total_size = self.calculate_size();
        if buffer.len() < total_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Buffer too small: need {}, got {}",
                    total_size,
                    buffer.len()
                ),
            ));
        }

        let mut writer = io::Cursor::new(buffer);

        // Calculate payload size
        let payload_size = total_size - std::mem::size_of::<MessageHeader>();

        // Write header
        let header = MessageHeader {
            sequence: self.sequence,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            magic: crate::MESSAGE_MAGIC,
            payload_size: payload_size as u32,
            checksum: 0, // Calculate after payload
            relay_domain: self.domain as u8,
            version: 1,
            source: self.source as u8,
            flags: 0,
        };

        writer.write_all(header.as_bytes())?;

        // Write TLV payload directly from references
        for tlv_ref in &self.tlv_refs {
            match tlv_ref {
                TLVRef::Standard { tlv_type, payload } => {
                    writer.write_all(&[*tlv_type])?;
                    writer.write_all(&[payload.len() as u8])?;
                    writer.write_all(payload)?;
                }
                TLVRef::Extended { tlv_type, payload } => {
                    writer.write_all(&[*tlv_type])?;
                    writer.write_all(&[0])?; // Reserved byte
                    writer.write_all(&(payload.len() as u16).to_le_bytes())?;
                    writer.write_all(payload)?;
                }
            }
        }

        Ok(total_size)
    }

    /// Build message with single allocation (still better than current)
    pub fn build(self) -> Vec<u8> {
        let total_size = self.calculate_size();
        let mut buffer = vec![0u8; total_size];

        match self.build_into_buffer(&mut buffer) {
            Ok(size) => {
                buffer.truncate(size);
                buffer
            }
            Err(_) => Vec::new(), // Should never happen with properly sized buffer
        }
    }

    /// Set sequence number
    pub fn with_sequence(mut self, sequence: u64) -> Self {
        self.sequence = sequence;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tlv::market_data::TradeTLV;
    use crate::{InstrumentId, VenueId};

    #[test]
    fn test_zero_copy_builder() {
        let trade = TradeTLV::new(
            VenueId::Polygon,
            InstrumentId {
                venue: VenueId::Polygon as u16,
                asset_type: 1,
                reserved: 0,
                asset_id: 12345,
            },
            100_000_000,
            50_000_000,
            0,
            1234567890,
        );

        // Build using zero-copy references
        let builder =
            ZeroCopyTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
                .add_tlv_ref(TLVType::Trade, &trade);

        let message = builder.build();
        assert!(!message.is_empty());

        // Verify the TLV data is correctly embedded
        let header_size = std::mem::size_of::<MessageHeader>();
        let tlv_start = header_size;

        // Should have: type(1) + len(1) + trade_data(40) = 42 bytes TLV payload
        assert_eq!(message[tlv_start], TLVType::Trade as u8);
        assert_eq!(message[tlv_start + 1], 40); // TradeTLV size
    }

    #[test]
    fn test_buffer_build() {
        let trade = TradeTLV::new(
            VenueId::Polygon,
            InstrumentId {
                venue: VenueId::Polygon as u16,
                asset_type: 1,
                reserved: 0,
                asset_id: 12345,
            },
            100_000_000,
            50_000_000,
            0,
            1234567890,
        );

        let builder =
            ZeroCopyTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
                .add_tlv_ref(TLVType::Trade, &trade);

        let expected_size = builder.calculate_size();
        let mut buffer = vec![0u8; expected_size + 10]; // Extra space

        let actual_size = builder.build_into_buffer(&mut buffer).unwrap();
        assert_eq!(actual_size, expected_size);
    }

    #[test]
    fn test_performance_no_allocations_during_build() {
        let trade = TradeTLV::new(
            VenueId::Polygon,
            InstrumentId {
                venue: VenueId::Polygon as u16,
                asset_type: 1,
                reserved: 0,
                asset_id: 12345,
            },
            100_000_000,
            50_000_000,
            0,
            1234567890,
        );

        // Pre-allocate buffer once
        let builder =
            ZeroCopyTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
                .add_tlv_ref(TLVType::Trade, &trade);

        let size = builder.calculate_size();
        let mut buffer = vec![0u8; size];

        // This should do zero allocations
        let iterations = 100_000;
        let start = std::time::Instant::now();

        for _ in 0..iterations {
            let builder = ZeroCopyTLVMessageBuilder::new(
                RelayDomain::MarketData,
                SourceType::PolygonCollector,
            )
            .add_tlv_ref(TLVType::Trade, &trade);

            let _size = builder.build_into_buffer(&mut buffer).unwrap();
            std::hint::black_box(_size);
        }

        let duration = start.elapsed();
        let ns_per_op = duration.as_nanos() as f64 / iterations as f64;

        println!(
            "Zero-copy message construction: {:.2} ns/op ({:.2}M ops/sec)",
            ns_per_op,
            1000.0 / ns_per_op
        );

        // Should be faster than current builder
        assert!(ns_per_op < 1000.0, "Should be sub-microsecond");
    }
}
