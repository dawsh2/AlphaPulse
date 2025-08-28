//! Integration tests for TLVMessageBuilder
//!
//! These tests exercise the public API from an external user's perspective,
//! focusing on realistic usage patterns and complete workflows.

use torq_codec::{TLVMessageBuilder, TLVType, VendorTLVBuilder};
use torq_types::protocol::message::header::MessageHeader;
use torq_types::{RelayDomain, SourceType};
use zerocopy::{AsBytes, FromBytes, FromZeroes};

// Test data structures for integration testing
#[repr(C)]
#[derive(AsBytes, FromBytes, FromZeroes, PartialEq, Eq, Debug)]
struct TestTradeTLV {
    instrument_id: u64,
    price: i64,
    volume: i64,
}

#[repr(C)]
#[derive(AsBytes, FromBytes, FromZeroes, PartialEq, Eq, Debug)]
struct TestSignalTLV {
    signal_id: u64,
    confidence: i32,
    flags: u32,
}

// Test utilities for creating predictable test data
mod test_builders {
    use super::*;

    pub fn valid_trade_tlv() -> TestTradeTLV {
        TestTradeTLV {
            instrument_id: 0x123456789ABCDEF0,
            price: 4500000000000, // $45,000.00 (8 decimal precision)
            volume: 100000000,    // 1.0 token (18 decimal precision)
        }
    }

    pub fn max_trade_tlv() -> TestTradeTLV {
        TestTradeTLV {
            instrument_id: u64::MAX,
            price: i64::MAX,
            volume: i64::MAX,
        }
    }

    pub fn zero_trade_tlv() -> TestTradeTLV {
        TestTradeTLV {
            instrument_id: 0,
            price: 0,
            volume: 0,
        }
    }

    pub fn valid_signal_tlv() -> TestSignalTLV {
        TestSignalTLV {
            signal_id: 0xDEADBEEFCAFEBABE,
            confidence: 95,
            flags: 0x01,
        }
    }
}

// Integration Tests - Testing public API workflows

#[test]
fn test_round_trip_precision_preservation() {
    // Test that we preserve exact precision through serialization roundtrip
    let original = test_builders::valid_trade_tlv();

    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .add_tlv(TLVType::Trade, &original)
        .build()
        .expect("Failed to build message");

    // Extract payload and verify it matches original
    let payload_offset = MessageHeader::SIZE + 2; // Skip header and TLV header
    let payload_bytes = &message[payload_offset..payload_offset + 24];

    let reconstructed =
        TestTradeTLV::read_from(payload_bytes).expect("Failed to reconstruct from bytes");

    assert_eq!(reconstructed, original);
    assert_eq!(reconstructed.price, 4500000000000); // Exact precision preserved
    assert_eq!(reconstructed.volume, 100000000); // Exact precision preserved
}

#[test]
fn test_extended_tlv_workflow() {
    // Test workflow with large payloads that require extended format
    let large_payload = vec![0x42u8; 1000];

    let message = TLVMessageBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy)
        .add_tlv_bytes(TLVType::SignalIdentity, large_payload.clone())
        .build()
        .expect("Failed to build extended message");

    // Verify message structure
    assert_eq!(message.len(), MessageHeader::SIZE + 5 + 1000);

    // Verify extended TLV format markers
    let tlv_start = MessageHeader::SIZE;
    assert_eq!(message[tlv_start], 255); // Extended marker
    assert_eq!(message[tlv_start + 1], 0); // Reserved byte
    assert_eq!(message[tlv_start + 2], TLVType::SignalIdentity as u8); // Actual type

    // Verify payload length in little-endian
    let length_bytes = [message[tlv_start + 3], message[tlv_start + 4]];
    assert_eq!(u16::from_le_bytes(length_bytes), 1000);
}

#[test]
fn test_multiple_tlvs_integration() {
    // Test realistic scenario with multiple TLV types
    let trade_data = test_builders::valid_trade_tlv();
    let signal_data = test_builders::valid_signal_tlv();

    let message = TLVMessageBuilder::new(RelayDomain::Execution, SourceType::ExecutionEngine)
        .add_tlv(TLVType::Trade, &trade_data)
        .add_tlv(TLVType::SignalIdentity, &signal_data)
        .with_sequence(12345)
        .with_flags(0x80)
        .build()
        .expect("Failed to build multi-TLV message");

    // Verify message structure
    let expected_size = MessageHeader::SIZE + (2 + 24) + (2 + 16);
    assert_eq!(message.len(), expected_size);

    // Verify header magic
    assert_eq!(&message[0..4], &[0xEF, 0xBE, 0xAD, 0xDE]);

    // Verify first TLV
    let first_tlv_offset = MessageHeader::SIZE;
    assert_eq!(message[first_tlv_offset], TLVType::Trade as u8);
    assert_eq!(message[first_tlv_offset + 1], 24); // Trade payload size

    // Verify second TLV
    let second_tlv_offset = first_tlv_offset + 2 + 24;
    assert_eq!(message[second_tlv_offset], TLVType::SignalIdentity as u8);
    assert_eq!(message[second_tlv_offset + 1], 16); // Signal payload size
}

#[test]
fn test_boundary_conditions() {
    // Test 255 byte boundary (should use standard format)
    let boundary_payload = vec![0xAAu8; 255];

    let message = TLVMessageBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy)
        .add_tlv_bytes(TLVType::SignalIdentity, boundary_payload)
        .build()
        .expect("Failed to build boundary message");

    assert_eq!(message.len(), MessageHeader::SIZE + 2 + 255);

    let tlv_start = MessageHeader::SIZE;
    assert_eq!(message[tlv_start], TLVType::SignalIdentity as u8);
    assert_eq!(message[tlv_start + 1], 255);

    // Test 256 bytes (should use extended format)
    let extended_payload = vec![0xBBu8; 256];

    let extended_message =
        TLVMessageBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy)
            .add_tlv_bytes(TLVType::SignalIdentity, extended_payload)
            .build()
            .expect("Failed to build extended boundary message");

    assert_eq!(extended_message.len(), MessageHeader::SIZE + 5 + 256);

    let ext_tlv_start = MessageHeader::SIZE;
    assert_eq!(extended_message[ext_tlv_start], 255); // Extended marker
}

#[test]
fn test_extreme_values() {
    // Test maximum values handling
    let max_trade = test_builders::max_trade_tlv();

    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .add_tlv(TLVType::Trade, &max_trade)
        .build()
        .expect("Failed to build message with max values");

    // Verify max values are preserved
    let payload_offset = MessageHeader::SIZE + 2;
    let payload_bytes = &message[payload_offset..payload_offset + 24];

    let reconstructed =
        TestTradeTLV::read_from(payload_bytes).expect("Failed to reconstruct max values");

    assert_eq!(reconstructed.instrument_id, u64::MAX);
    assert_eq!(reconstructed.price, i64::MAX);
    assert_eq!(reconstructed.volume, i64::MAX);

    // Test zero values handling
    let zero_trade = test_builders::zero_trade_tlv();

    let zero_message =
        TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
            .add_tlv(TLVType::Trade, &zero_trade)
            .build()
            .expect("Failed to build message with zero values");

    let zero_payload_offset = MessageHeader::SIZE + 2;
    let zero_payload_bytes = &zero_message[zero_payload_offset..zero_payload_offset + 24];

    let zero_reconstructed =
        TestTradeTLV::read_from(zero_payload_bytes).expect("Failed to reconstruct zero values");

    assert_eq!(zero_reconstructed.instrument_id, 0);
    assert_eq!(zero_reconstructed.price, 0);
    assert_eq!(zero_reconstructed.volume, 0);
}

#[test]
fn test_fluent_builder_api() {
    // Test that all builder methods can be chained fluently
    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .add_tlv(TLVType::Trade, &test_builders::valid_trade_tlv())
        .add_tlv_bytes(TLVType::GasPrice, vec![0u8; 32])
        .with_sequence(12345)
        .with_flags(0x80)
        .with_timestamp(1234567890123456789)
        .build()
        .expect("Fluent builder pattern failed");

    // Verify the message was built successfully
    assert!(message.len() > MessageHeader::SIZE);
}

#[test]
fn test_buffer_operations() {
    // Test build_into_buffer workflow
    let trade_data = test_builders::valid_trade_tlv();
    let builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .add_tlv(TLVType::Trade, &trade_data);

    let mut buffer = vec![0u8; 1000];
    let size = builder
        .build_into_buffer(&mut buffer)
        .expect("Failed to build into buffer");

    assert_eq!(size, 58); // Expected message size
    assert_eq!(&buffer[0..4], &[0xEF, 0xBE, 0xAD, 0xDE]); // MESSAGE_MAGIC

    // Test buffer too small error
    let trade_data2 = test_builders::valid_trade_tlv();
    let builder2 = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .add_tlv(TLVType::Trade, &trade_data2);

    let mut small_buffer = vec![0u8; 10]; // Too small
    let result = builder2.build_into_buffer(&mut small_buffer);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Buffer too small"));
}

#[test]
fn test_build_and_send_workflow() {
    // Test build_and_send convenience method
    let trade_data = test_builders::valid_trade_tlv();
    let builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .add_tlv(TLVType::Trade, &trade_data);

    let mut sent_data = Vec::new();
    let send_fn = |data: &[u8]| -> Result<usize, std::io::Error> {
        sent_data.extend_from_slice(data);
        Ok(data.len())
    };

    let result = builder
        .build_and_send(send_fn)
        .expect("Failed to build and send");

    assert_eq!(result, 58); // Size returned by send_fn
    assert_eq!(sent_data.len(), 58);
    assert_eq!(&sent_data[0..4], &[0xEF, 0xBE, 0xAD, 0xDE]); // MESSAGE_MAGIC
}

#[test]
fn test_vendor_tlv_integration() {
    // Test vendor TLV builder workflow
    let vendor_data = [0x12, 0x34, 0x56, 0x78];

    let message = VendorTLVBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy)
        .add_vendor_tlv(200, &vendor_data)
        .build()
        .expect("Failed to build vendor TLV");

    // Verify vendor TLV structure
    let tlv_start = MessageHeader::SIZE;
    assert_eq!(message[tlv_start], 200); // Vendor type
    assert_eq!(message[tlv_start + 1], 4); // Payload size
    assert_eq!(&message[tlv_start + 2..tlv_start + 6], &vendor_data);

    // Test vendor to standard builder conversion
    let trade_data = test_builders::valid_trade_tlv();

    let mixed_message = VendorTLVBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy)
        .add_vendor_tlv(201, &[0x12, 0x34])
        .into_standard_builder()
        .add_tlv(TLVType::Trade, &trade_data)
        .build()
        .expect("Failed to build vendor+standard TLV");

    // Should contain both vendor and standard TLVs
    assert!(mixed_message.len() > MessageHeader::SIZE + 2 + 2 + 2 + 24);
}

#[test]
fn test_empty_message_workflow() {
    // Test building empty messages (just header)
    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .with_sequence(999)
        .build()
        .expect("Failed to build empty message");

    // Should just be the header
    assert_eq!(message.len(), MessageHeader::SIZE);
    assert_eq!(&message[0..4], &[0xEF, 0xBE, 0xAD, 0xDE]); // MESSAGE_MAGIC
}

#[test]
fn test_size_utility_methods() {
    // Test size checking utilities work correctly
    let builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::KrakenCollector)
        .add_tlv_bytes(TLVType::Trade, vec![0; 100]);

    assert_eq!(builder.payload_size(), 102); // 2 + 100
    assert_eq!(builder.tlv_count(), 1);
    assert!(!builder.would_exceed_size(200));
    assert!(builder.would_exceed_size(130)); // 32 + 102 = 134 > 130
}

#[test]
fn test_performance_with_many_tlvs() {
    // Test performance with realistic number of TLVs
    let mut builder = TLVMessageBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy);

    // Add 100 small TLVs (realistic batch size)
    for i in 0..100 {
        let payload = vec![i as u8; 10];
        builder = builder.add_tlv_bytes(TLVType::SignalIdentity, payload);
    }

    let message = builder
        .build()
        .expect("Failed to build message with many TLVs");

    // Each TLV: 2 byte header + 10 byte payload = 12 bytes
    // Total: 32 byte header + 100 * 12 = 1232 bytes
    assert_eq!(message.len(), MessageHeader::SIZE + 100 * 12);

    // Verify header is intact
    assert_eq!(&message[0..4], &[0xEF, 0xBE, 0xAD, 0xDE]); // MESSAGE_MAGIC
}
