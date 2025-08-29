//! Integration tests for TLV message parsing
//!
//! These tests focus on end-to-end parsing workflows and realistic message validation scenarios.

use codec::{
    extract_tlv_payload, find_tlv_by_type, parse_header, parse_tlv_extensions, validate_tlv_size,
    ProtocolError, TLVMessageBuilder, TLVType,
};
use types::protocol::message::header::MessageHeader;
use types::{RelayDomain, SourceType, MESSAGE_MAGIC};
use zerocopy::{AsBytes, FromBytes, FromZeroes};

#[repr(C)]
#[derive(AsBytes, FromBytes, FromZeroes, PartialEq, Eq, Debug, Copy, Clone)]
struct TestTradeTLV {
    instrument_id: u64, // 8 bytes
    price: i64,         // 8 bytes
    volume: i64,        // 8 bytes
    timestamp: u64,     // 8 bytes
    flags: u64,         // 8 bytes - Total: 40 bytes
}

#[test]
fn test_complete_message_parse_workflow() {
    // Build a complete message
    let trade_data = TestTradeTLV {
        instrument_id: 0x123456789ABCDEF0,
        price: 4500000000000,
        volume: 100000000,
        timestamp: 1234567890123456789,
        flags: 0x0001, // buy order
    };

    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .add_tlv(TLVType::Trade, &trade_data)
        .with_sequence(42)
        .build()
        .expect("Failed to build test message");

    // Parse header
    let header = parse_header(&message).expect("Failed to parse header");
    assert_eq!(header.magic, MESSAGE_MAGIC);
    assert_eq!(header.relay_domain, RelayDomain::MarketData as u8);
    assert_eq!(header.source, SourceType::BinanceCollector as u8);
    assert_eq!(header.sequence, 42);

    // Parse TLV payload
    let tlv_payload = &message[MessageHeader::SIZE..];
    let extensions = parse_tlv_extensions(tlv_payload).expect("Failed to parse TLV extensions");

    assert_eq!(extensions.len(), 1);

    // Verify TLV content
    match &extensions[0] {
        codec::TLVExtensionEnum::Standard(tlv) => {
            assert_eq!(tlv.header.tlv_type, TLVType::Trade as u8);
            assert_eq!(tlv.header.tlv_length, 40);
            assert_eq!(tlv.payload.len(), 40);

            // Verify payload data
            let parsed_trade =
                TestTradeTLV::read_from(&tlv.payload).expect("Failed to parse trade data");
            assert_eq!(parsed_trade, trade_data);
        }
        _ => panic!("Expected standard TLV format"),
    }
}

#[test]
fn test_extended_tlv_parsing() {
    // Build extended TLV message
    let large_payload = vec![0x42u8; 1000];
    let message = TLVMessageBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy)
        .add_tlv_bytes(TLVType::OrderBook, large_payload.clone())
        .build()
        .expect("Failed to build extended message");

    // Parse header
    let header = parse_header(&message).expect("Failed to parse header");
    assert_eq!(header.relay_domain, RelayDomain::Signal as u8);

    // Parse TLV payload
    let tlv_payload = &message[MessageHeader::SIZE..];
    let extensions = parse_tlv_extensions(tlv_payload).expect("Failed to parse extended TLV");

    assert_eq!(extensions.len(), 1);

    // Verify extended TLV content
    match &extensions[0] {
        codec::TLVExtensionEnum::Extended(tlv) => {
            assert_eq!(tlv.header.marker, 255);
            assert_eq!(tlv.header.reserved, 0);
            assert_eq!(tlv.header.tlv_type, TLVType::OrderBook as u8);
            let tlv_length = tlv.header.tlv_length;
            assert_eq!(tlv_length, 1000);
            assert_eq!(tlv.payload, large_payload);
        }
        _ => panic!("Expected extended TLV format"),
    }
}

#[test]
fn test_multiple_tlv_parsing() {
    // Build message with multiple TLVs
    let trade_data = TestTradeTLV {
        instrument_id: 0x123456789ABCDEF0,
        price: 4500000000000,
        volume: 100000000,
        timestamp: 1234567890123456789,
        flags: 0x0001, // buy order
    };

    let signal_data = vec![0xAAu8; 16];

    let message = TLVMessageBuilder::new(RelayDomain::Execution, SourceType::ExecutionEngine)
        .add_tlv(TLVType::Trade, &trade_data)
        .add_tlv_bytes(TLVType::SignalIdentity, signal_data.clone())
        .add_tlv_bytes(TLVType::GasPrice, vec![0xBBu8; 32])
        .build()
        .expect("Failed to build multi-TLV message");

    // Parse TLVs
    let tlv_payload = &message[MessageHeader::SIZE..];
    let extensions = parse_tlv_extensions(tlv_payload).expect("Failed to parse multiple TLVs");

    assert_eq!(extensions.len(), 3);

    // Verify each TLV
    for (i, extension) in extensions.iter().enumerate() {
        match extension {
            codec::TLVExtensionEnum::Standard(tlv) => match i {
                0 => {
                    assert_eq!(tlv.header.tlv_type, TLVType::Trade as u8);
                    assert_eq!(tlv.payload.len(), 40);
                }
                1 => {
                    assert_eq!(tlv.header.tlv_type, TLVType::SignalIdentity as u8);
                    assert_eq!(tlv.payload, signal_data);
                }
                2 => {
                    assert_eq!(tlv.header.tlv_type, TLVType::GasPrice as u8);
                    assert_eq!(tlv.payload, vec![0xBBu8; 32]);
                }
                _ => panic!("Unexpected TLV index"),
            },
            _ => panic!("Expected standard TLV format"),
        }
    }
}

#[test]
fn test_tlv_lookup_by_type() {
    // Build message with multiple TLV types
    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::KrakenCollector)
        .add_tlv_bytes(TLVType::Trade, vec![0x01u8; 40])
        .add_tlv_bytes(TLVType::Quote, vec![0x02u8; 52])
        .add_tlv_bytes(TLVType::GasPrice, vec![0x03u8; 32])
        .build()
        .expect("Failed to build lookup test message");

    let tlv_payload = &message[MessageHeader::SIZE..];

    // Test successful lookups
    let trade_payload =
        find_tlv_by_type(tlv_payload, TLVType::Trade as u8).expect("Failed to find Trade TLV");
    assert_eq!(trade_payload, vec![0x01u8; 40]);

    let quote_payload =
        find_tlv_by_type(tlv_payload, TLVType::Quote as u8).expect("Failed to find Quote TLV");
    assert_eq!(quote_payload, vec![0x02u8; 52]);

    let gas_payload = find_tlv_by_type(tlv_payload, TLVType::GasPrice as u8)
        .expect("Failed to find GasPrice TLV");
    assert_eq!(gas_payload, vec![0x03u8; 32]);

    // Test failed lookup
    let not_found = find_tlv_by_type(tlv_payload, TLVType::OrderBook as u8);
    assert!(not_found.is_none());
}

#[test]
fn test_type_safe_payload_extraction() {
    // Build message with trade data
    let original_trade = TestTradeTLV {
        instrument_id: 0xFEDCBA9876543210,
        price: 5500000000000,
        volume: 200000000,
        timestamp: 9876543210987654321,
        flags: 0x0002, // sell order
    };

    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .add_tlv(TLVType::Trade, &original_trade)
        .build()
        .expect("Failed to build trade message");

    let tlv_payload = &message[MessageHeader::SIZE..];

    // Extract trade using type-safe method
    let extracted_trade: TestTradeTLV = extract_tlv_payload(tlv_payload, TLVType::Trade)
        .expect("Failed to extract trade payload")
        .expect("Trade TLV not found");

    assert_eq!(extracted_trade, original_trade);

    // Test extraction of non-existent type
    let no_quote: Option<TestTradeTLV> =
        extract_tlv_payload(tlv_payload, TLVType::Quote).expect("Failed to extract quote payload");
    assert!(no_quote.is_none());
}

#[test]
fn test_parsing_error_conditions() {
    // Test message too small
    let tiny_message = vec![0x01, 0x02, 0x03];
    let result = parse_header(&tiny_message);
    assert!(matches!(result, Err(ProtocolError::MessageTooSmall { .. })));

    // Test invalid magic number
    let mut bad_magic = vec![0u8; 32];
    bad_magic[0..4].copy_from_slice(&[0x00, 0x11, 0x22, 0x33]); // Wrong magic
    let result = parse_header(&bad_magic);
    assert!(matches!(result, Err(ProtocolError::InvalidMagic { .. })));

    // Test truncated TLV
    let truncated_tlv = vec![
        TLVType::Trade as u8, // Type
        100,                  // Claims 100 bytes
        0x01,
        0x02, // But only has 2 bytes
    ];
    let result = parse_tlv_extensions(&truncated_tlv);
    assert!(matches!(result, Err(ProtocolError::TruncatedTLV { .. })));
}

#[test]
fn test_size_validation() {
    // Test fixed size validation
    assert!(validate_tlv_size(TLVType::Trade as u8, 40).is_ok());
    assert!(validate_tlv_size(TLVType::Trade as u8, 39).is_err());
    assert!(validate_tlv_size(TLVType::Trade as u8, 41).is_err());

    // Test variable size (should always pass)
    assert!(validate_tlv_size(TLVType::OrderBook as u8, 10).is_ok());
    assert!(validate_tlv_size(TLVType::OrderBook as u8, 1000).is_ok());
    assert!(validate_tlv_size(TLVType::OrderBook as u8, 100000).is_ok());
}

#[test]
fn test_boundary_parsing() {
    // Test 255 byte boundary (standard format)
    let boundary_payload = vec![0xAAu8; 255];
    let message = TLVMessageBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy)
        .add_tlv_bytes(TLVType::OrderBook, boundary_payload.clone())
        .build()
        .expect("Failed to build boundary message");

    let tlv_payload = &message[MessageHeader::SIZE..];
    let extensions = parse_tlv_extensions(tlv_payload).expect("Failed to parse boundary TLV");

    assert_eq!(extensions.len(), 1);
    match &extensions[0] {
        codec::TLVExtensionEnum::Standard(tlv) => {
            assert_eq!(tlv.header.tlv_length, 255);
            assert_eq!(tlv.payload, boundary_payload);
        }
        _ => panic!("Expected standard format for 255 bytes"),
    }

    // Test 256 bytes (extended format)
    let extended_payload = vec![0xBBu8; 256];
    let extended_message =
        TLVMessageBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy)
            .add_tlv_bytes(TLVType::OrderBook, extended_payload.clone())
            .build()
            .expect("Failed to build extended boundary message");

    let ext_tlv_payload = &extended_message[MessageHeader::SIZE..];
    let ext_extensions =
        parse_tlv_extensions(ext_tlv_payload).expect("Failed to parse extended boundary TLV");

    assert_eq!(ext_extensions.len(), 1);
    match &ext_extensions[0] {
        codec::TLVExtensionEnum::Extended(tlv) => {
            let tlv_length = tlv.header.tlv_length;
            assert_eq!(tlv_length, 256);
            assert_eq!(tlv.payload, extended_payload);
        }
        _ => panic!("Expected extended format for 256 bytes"),
    }
}

#[test]
fn test_mixed_format_parsing() {
    // Test message with both standard and extended TLVs
    let mut builder = TLVMessageBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy);

    // Add standard TLV
    builder = builder.add_tlv_bytes(TLVType::SignalIdentity, vec![0x01u8; 16]);
    // Add extended TLV
    builder = builder.add_tlv_bytes(TLVType::OrderBook, vec![0x02u8; 500]);
    // Add another standard TLV
    builder = builder.add_tlv_bytes(TLVType::GasPrice, vec![0x03u8; 32]);

    let message = builder
        .build()
        .expect("Failed to build mixed format message");
    let tlv_payload = &message[MessageHeader::SIZE..];
    let extensions = parse_tlv_extensions(tlv_payload).expect("Failed to parse mixed formats");

    assert_eq!(extensions.len(), 3);

    // Verify formats
    match &extensions[0] {
        codec::TLVExtensionEnum::Standard(tlv) => {
            assert_eq!(tlv.header.tlv_type, TLVType::SignalIdentity as u8);
            assert_eq!(tlv.payload, vec![0x01u8; 16]);
        }
        _ => panic!("Expected standard format"),
    }

    match &extensions[1] {
        codec::TLVExtensionEnum::Extended(tlv) => {
            assert_eq!(tlv.header.tlv_type, TLVType::OrderBook as u8);
            assert_eq!(tlv.payload, vec![0x02u8; 500]);
        }
        _ => panic!("Expected extended format"),
    }

    match &extensions[2] {
        codec::TLVExtensionEnum::Standard(tlv) => {
            assert_eq!(tlv.header.tlv_type, TLVType::GasPrice as u8);
            assert_eq!(tlv.payload, vec![0x03u8; 32]);
        }
        _ => panic!("Expected standard format"),
    }
}

#[test]
fn test_checksum_validation() {
    // Build valid message
    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .add_tlv_bytes(TLVType::Trade, vec![0xAAu8; 40])
        .build()
        .expect("Failed to build valid message");

    // Should parse successfully with valid checksum
    let header = parse_header(&message).expect("Valid message should parse");
    assert_eq!(header.magic, MESSAGE_MAGIC);

    // Corrupt the checksum and verify it fails
    let mut corrupted_message = message.clone();
    corrupted_message[28] ^= 0xFF; // Flip bits in checksum field

    let result = parse_header(&corrupted_message);
    assert!(matches!(
        result,
        Err(ProtocolError::ChecksumMismatch { .. })
    ));
}
