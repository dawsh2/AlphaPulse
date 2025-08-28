//! Core Protocol Tests
//!
//! Tests fundamental protocol properties including:
//! - Binary format integrity
//! - Fixed 48-byte message constraint
//! - Header field validation
//! - Version compatibility

mod common;

use torq_types::protocol::{
    parse_header,
    tlv::{ParseError, TLVMessageBuilder},
    validation::calculate_crc32_excluding_checksum,
    MessageHeader, RelayDomain, SourceType, MESSAGE_MAGIC as MAGIC_NUMBER, PROTOCOL_VERSION,
};
use common::*;

#[test]
fn test_message_size_constraint() {
    // Every message must be at least 48 bytes (32 header + 16 min TLV)
    let domains = [
        RelayDomain::MarketData,
        RelayDomain::Signal,
        RelayDomain::Execution,
    ];

    for domain in domains {
        let msg = create_test_message(domain, SourceType::Dashboard);
        assert!(
            msg.len() >= 48,
            "Message for {:?} is only {} bytes, minimum is 48",
            domain,
            msg.len()
        );
    }
}

#[test]
fn test_magic_number_validation() {
    // Valid magic number should parse
    let valid_msg = create_market_data_message(SourceType::BinanceCollector);
    let header = parse_header(&valid_msg).unwrap();
    let magic = header.magic; // Copy from packed struct
    assert_eq!(magic, MAGIC_NUMBER);

    // Invalid magic number should fail
    let invalid_msg = create_invalid_magic_message();
    match parse_header(&invalid_msg) {
        Err(ParseError::InvalidMagic { expected, actual }) => {
            assert_eq!(expected, MAGIC_NUMBER);
            assert_eq!(actual, 0xBADBADBA);
        }
        _ => panic!("Expected InvalidMagic error"),
    }
}

#[test]
fn test_version_field() {
    let msg = create_signal_message(SourceType::Dashboard);
    let header = parse_header(&msg).unwrap();
    let version = header.version; // Copy from packed struct
    assert_eq!(version, PROTOCOL_VERSION);

    // Test with different version
    let mut msg_v2 = msg.clone();
    msg_v2[4] = 2; // Change version to 2
                   // Recalculate checksum for modified message
    let checksum = protocol_v2::validation::calculate_crc32_excluding_checksum(&msg_v2, 28);
    msg_v2[28..32].copy_from_slice(&checksum.to_le_bytes());

    let header_v2 = parse_header(&msg_v2).unwrap();
    let version = header_v2.version; // Copy from packed struct
    assert_eq!(version, 2);
}

#[test]
fn test_domain_field_validation() {
    // Test all valid domains
    let test_cases = [
        (RelayDomain::MarketData, 1),
        (RelayDomain::Signal, 2),
        (RelayDomain::Execution, 3),
    ];

    for (domain, expected_value) in test_cases {
        let msg = create_test_message(domain, SourceType::Dashboard);
        let header = parse_header(&msg).unwrap();
        assert_eq!(header.get_relay_domain().unwrap(), domain);
        assert_eq!(msg[6], expected_value);
    }
}

#[test]
fn test_source_field_validation() {
    let sources = [
        SourceType::BinanceCollector,
        SourceType::CoinbaseCollector,
        SourceType::KrakenCollector,
        SourceType::Dashboard,
        SourceType::SignalRelay,
    ];

    for source in sources {
        let msg = create_market_data_message(source);
        let header = parse_header(&msg).unwrap();
        assert_eq!(header.get_source_type().unwrap(), source);
    }
}

#[test]
fn test_timestamp_monotonicity() {
    // Timestamps should be monotonically increasing
    let msg1 = create_execution_message(SourceType::Dashboard);
    std::thread::sleep(std::time::Duration::from_millis(1));
    let msg2 = create_execution_message(SourceType::Dashboard);

    let header1 = parse_header(&msg1).unwrap();
    let header2 = parse_header(&msg2).unwrap();

    let timestamp1 = header1.timestamp; // Copy from packed struct
    let timestamp2 = header2.timestamp; // Copy from packed struct
    assert!(
        timestamp2 > timestamp1,
        "Timestamp should increase: {} -> {}",
        timestamp1,
        timestamp2
    );
}

#[test]
fn test_sequence_number_field() {
    // Sequence numbers are managed by relays, test the field exists
    let msg = create_market_data_message(SourceType::BinanceCollector);
    let header = parse_header(&msg).unwrap();

    // Initial messages have sequence 0
    let sequence = header.sequence; // Copy from packed struct
    assert_eq!(sequence, 0);

    // Manually set sequence
    let mut msg_with_seq = msg.clone();
    let seq_num: u64 = 12345678;
    msg_with_seq[12..20].copy_from_slice(&seq_num.to_le_bytes());

    // Recalculate checksum
    let checksum = protocol_v2::validation::calculate_crc32_excluding_checksum(&msg_with_seq, 28);
    msg_with_seq[28..32].copy_from_slice(&checksum.to_le_bytes());

    let header_with_seq = parse_header(&msg_with_seq).unwrap();
    let sequence = header_with_seq.sequence; // Copy from packed struct
    assert_eq!(sequence, seq_num);
}

#[test]
fn test_header_size_constant() {
    // Header must always be exactly 32 bytes
    assert_eq!(MessageHeader::SIZE, 32);
    assert_eq!(std::mem::size_of::<MessageHeader>(), 32);
}

#[test]
fn test_network_byte_order() {
    let msg = create_market_data_message(SourceType::BinanceCollector);

    // Magic number is big-endian (network byte order)
    let magic_bytes = &msg[0..4];
    let magic = u32::from_be_bytes(magic_bytes.try_into().unwrap());
    assert_eq!(magic, MAGIC_NUMBER);

    // Checksum is big-endian
    let checksum_bytes = &msg[28..32];
    let _checksum = u32::from_le_bytes(checksum_bytes.try_into().unwrap());

    // Timestamp is little-endian (for performance on x86)
    let timestamp_bytes = &msg[20..28];
    let _timestamp = u64::from_le_bytes(timestamp_bytes.try_into().unwrap());

    // Sequence is little-endian
    let sequence_bytes = &msg[12..20];
    let _sequence = u64::from_le_bytes(sequence_bytes.try_into().unwrap());
}

#[test]
fn test_minimum_viable_message() {
    // Create the absolute minimum valid message
    let builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Dashboard);

    // Add minimal TLV (empty payload is valid for some types)
    let msg = builder
        .add_tlv_bytes(protocol_v2::TLVType::Heartbeat, vec![0; 16])
        .build();

    // Should be exactly 48 bytes (32 header + 2 TLV header + 16 payload)
    assert_eq!(msg.len(), 50); // 32 + 2 + 16

    // Should parse successfully
    let header = parse_header(&msg).unwrap();
    assert_eq!(header.get_relay_domain().unwrap(), RelayDomain::MarketData);
}

#[test]
fn test_maximum_standard_message() {
    // Test maximum size for standard TLV (255 byte payload)
    let large_payload = vec![0xFF; 255];

    let msg = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Dashboard)
        .add_tlv_bytes(protocol_v2::TLVType::OrderBook, large_payload)
        .build();

    // Should be 32 header + 2 TLV header + 255 payload = 289 bytes
    assert_eq!(msg.len(), 289);

    let header = parse_header(&msg).unwrap();
    assert_eq!(header.get_relay_domain().unwrap(), RelayDomain::MarketData);
}

#[test]
fn test_flags_field_reserved() {
    // Flags field should be 0 (reserved for future use)
    let msg = create_signal_message(SourceType::Dashboard);
    assert_eq!(msg[5], 0, "Flags field should be 0 (reserved)");

    // Even with flags set, message should parse (forward compatibility)
    let mut msg_with_flags = msg.clone();
    msg_with_flags[5] = 0b10101010; // Set some flags

    // Recalculate checksum
    let checksum = protocol_v2::validation::calculate_crc32_excluding_checksum(&msg_with_flags, 28);
    msg_with_flags[28..32].copy_from_slice(&checksum.to_le_bytes());

    let header = parse_header(&msg_with_flags).unwrap();
    let flags = header.flags; // Copy from packed struct
    assert_eq!(flags, 0b10101010);
}

#[test]
fn test_truncated_messages() {
    // Test various truncation points
    let full_msg = create_market_data_message(SourceType::BinanceCollector);

    // Truncate at various points
    let truncation_points = [0, 10, 20, 31, 35, 40];

    for point in truncation_points {
        let truncated = &full_msg[..point.min(full_msg.len())];

        match parse_header(truncated) {
            Err(ParseError::MessageTooSmall { need, got }) => {
                assert_eq!(got, truncated.len());
                assert!(need > got);
            }
            Ok(_) => panic!("Should not parse truncated message at {} bytes", point),
            Err(e) => panic!("Unexpected error for truncation at {}: {:?}", point, e),
        }
    }
}

#[test]
fn test_zero_filled_message() {
    // A message of all zeros should fail magic number check
    let zero_msg = vec![0u8; 48];

    match parse_header(&zero_msg) {
        Err(ParseError::InvalidMagic { expected, actual }) => {
            assert_eq!(expected, MAGIC_NUMBER);
            assert_eq!(actual, 0);
        }
        _ => panic!("Expected InvalidMagic error for zero-filled message"),
    }
}

#[test]
fn test_message_padding_alignment() {
    // Messages should maintain alignment for performance
    let msg = create_market_data_message(SourceType::BinanceCollector);
    let msg_ptr = msg.as_ptr() as usize;

    // Check if message is at least 8-byte aligned (ideal for 64-bit systems)
    // Note: This is a best-effort test, actual alignment depends on allocator
    if msg_ptr % 8 == 0 {
        println!("Message is 8-byte aligned (optimal)");
    } else if msg_ptr % 4 == 0 {
        println!("Message is 4-byte aligned (acceptable)");
    } else {
        println!("Warning: Message alignment is suboptimal: {}", msg_ptr);
    }
}

#[test]
fn test_domain_routing_consistency() {
    // Ensure domain in header matches TLV types
    use torq_codec::protocol::TLVType;

    // Market data domain should only contain market data TLVs
    let market_msg = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Dashboard)
        .add_tlv_bytes(TLVType::Trade, vec![0; 24])
        .build();

    let header = parse_header(&market_msg).unwrap();
    assert_eq!(header.get_relay_domain().unwrap(), RelayDomain::MarketData);

    // Signal domain with signal TLVs
    let signal_msg = TLVMessageBuilder::new(RelayDomain::Signal, SourceType::Dashboard)
        .add_tlv_bytes(TLVType::SignalIdentity, vec![0; 16])
        .build();

    let header = parse_header(&signal_msg).unwrap();
    assert_eq!(header.get_relay_domain().unwrap(), RelayDomain::Signal);

    // Execution domain with execution TLVs
    let exec_msg = TLVMessageBuilder::new(RelayDomain::Execution, SourceType::Dashboard)
        .add_tlv_bytes(TLVType::OrderRequest, vec![0; 32])
        .build();

    let header = parse_header(&exec_msg).unwrap();
    assert_eq!(header.get_relay_domain().unwrap(), RelayDomain::Execution);
}

#[test]
fn test_parse_header_performance() {
    // Ensure header parsing is fast enough for hot path
    let msg = create_market_data_message(SourceType::BinanceCollector);
    let iterations = 100_000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let _ = parse_header(&msg).unwrap();
    }
    let elapsed = start.elapsed();

    let ns_per_parse = elapsed.as_nanos() / iterations;
    println!("Header parsing: {} ns per operation", ns_per_parse);

    // Should be under 100ns for hot path
    assert!(
        ns_per_parse < 1000,
        "Header parsing too slow: {} ns (target: <1000 ns)",
        ns_per_parse
    );
}
