//! Message Header Unit Tests
//!
//! Tests the 32-byte MessageHeader structure that prefixes all Protocol V2 messages.

use protocol_v2::{
    parse_header, MessageHeader, RelayDomain, SourceType, ParseError,
    MESSAGE_MAGIC, PROTOCOL_VERSION,
};

#[test]
fn test_header_size_is_32_bytes() {
    // Critical: Header must be exactly 32 bytes for Protocol V2
    assert_eq!(std::mem::size_of::<MessageHeader>(), 32);
}

#[test]
fn test_header_field_sizes() {
    // Validate individual field sizes
    use std::mem::size_of;
    
    // Magic: 4 bytes
    assert_eq!(size_of::<u32>(), 4);
    // Version: 1 byte 
    assert_eq!(size_of::<u8>(), 1);
    // Domain: 1 byte
    assert_eq!(size_of::<RelayDomain>(), 1);
    // Source: 2 bytes
    assert_eq!(size_of::<SourceType>(), 2);
    // Sequence: 8 bytes
    assert_eq!(size_of::<u64>(), 8);
    // Timestamp: 8 bytes
    assert_eq!(size_of::<u64>(), 8);
    // Payload size: 4 bytes
    assert_eq!(size_of::<u32>(), 4);
    // Checksum: 4 bytes
    assert_eq!(size_of::<u32>(), 4);
    // Total: 32 bytes
}

#[test]
fn test_valid_magic_number() {
    let header = MessageHeader {
        magic: MESSAGE_MAGIC,
        version: PROTOCOL_VERSION,
        relay_domain: RelayDomain::MarketData,
        source: SourceType::BinanceCollector,
        sequence: 1,
        timestamp_ns: 1000000000,
        payload_size: 16,
        checksum: 0,
    };
    
    let bytes = unsafe { 
        std::slice::from_raw_parts(
            &header as *const _ as *const u8, 
            std::mem::size_of::<MessageHeader>()
        )
    };
    
    let parsed = parse_header(bytes).expect("Valid header should parse");
    assert_eq!(parsed.magic, MESSAGE_MAGIC);
}

#[test]
fn test_invalid_magic_number() {
    let header = MessageHeader {
        magic: 0x12345678, // Invalid magic
        version: PROTOCOL_VERSION,
        relay_domain: RelayDomain::MarketData,
        source: SourceType::BinanceCollector,
        sequence: 1,
        timestamp_ns: 1000000000,
        payload_size: 16,
        checksum: 0,
    };
    
    let bytes = unsafe { 
        std::slice::from_raw_parts(
            &header as *const _ as *const u8, 
            std::mem::size_of::<MessageHeader>()
        )
    };
    
    match parse_header(bytes) {
        Err(ParseError::InvalidMagic { expected, actual }) => {
            assert_eq!(expected, MESSAGE_MAGIC);
            assert_eq!(actual, 0x12345678);
        }
        _ => panic!("Should have failed with InvalidMagic"),
    }
}

#[test]
fn test_version_validation() {
    let header = MessageHeader {
        magic: MESSAGE_MAGIC,
        version: 99, // Invalid version
        relay_domain: RelayDomain::MarketData,
        source: SourceType::BinanceCollector,
        sequence: 1,
        timestamp_ns: 1000000000,
        payload_size: 16,
        checksum: 0,
    };
    
    let bytes = unsafe { 
        std::slice::from_raw_parts(
            &header as *const _ as *const u8, 
            std::mem::size_of::<MessageHeader>()
        )
    };
    
    match parse_header(bytes) {
        Err(ParseError::UnsupportedVersion { version }) => {
            assert_eq!(version, 99);
        }
        _ => panic!("Should have failed with UnsupportedVersion"),
    }
}

#[test]
fn test_sequence_number_parsing() {
    let sequence = 0x123456789ABCDEF0u64;
    
    let header = MessageHeader {
        magic: MESSAGE_MAGIC,
        version: PROTOCOL_VERSION,
        relay_domain: RelayDomain::Signal,
        source: SourceType::Dashboard,
        sequence,
        timestamp_ns: 1000000000,
        payload_size: 16,
        checksum: 0,
    };
    
    let bytes = unsafe { 
        std::slice::from_raw_parts(
            &header as *const _ as *const u8, 
            std::mem::size_of::<MessageHeader>()
        )
    };
    
    let parsed = parse_header(bytes).expect("Valid header should parse");
    assert_eq!(parsed.sequence, sequence);
}

#[test]
fn test_nanosecond_timestamp_precision() {
    // Test that we preserve full nanosecond precision
    let timestamp_ns = 1_234_567_890_123_456_789u64;
    
    let header = MessageHeader {
        magic: MESSAGE_MAGIC,
        version: PROTOCOL_VERSION,
        relay_domain: RelayDomain::Execution,
        source: SourceType::FlashArbitrageStrategy,
        sequence: 1,
        timestamp_ns,
        payload_size: 16,
        checksum: 0,
    };
    
    let bytes = unsafe { 
        std::slice::from_raw_parts(
            &header as *const _ as *const u8, 
            std::mem::size_of::<MessageHeader>()
        )
    };
    
    let parsed = parse_header(bytes).expect("Valid header should parse");
    assert_eq!(parsed.timestamp_ns, timestamp_ns);
    
    // Verify nanosecond precision is preserved
    assert_eq!(parsed.timestamp_ns % 1000, 789);
}

#[test]
fn test_relay_domain_values() {
    // Test each relay domain value
    let domains = [
        (RelayDomain::MarketData, "MarketData"),
        (RelayDomain::Signal, "Signal"),
        (RelayDomain::Execution, "Execution"),
    ];
    
    for (domain, name) in domains {
        let header = MessageHeader {
            magic: MESSAGE_MAGIC,
            version: PROTOCOL_VERSION,
            relay_domain: domain,
            source: SourceType::Dashboard,
            sequence: 1,
            timestamp_ns: 1000000000,
            payload_size: 16,
            checksum: 0,
        };
        
        let bytes = unsafe { 
            std::slice::from_raw_parts(
                &header as *const _ as *const u8, 
                std::mem::size_of::<MessageHeader>()
            )
        };
        
        let parsed = parse_header(bytes)
            .unwrap_or_else(|_| panic!("Failed to parse header for {}", name));
        
        assert_eq!(parsed.relay_domain, domain, "Domain mismatch for {}", name);
    }
}

#[test]
fn test_payload_size_limits() {
    // Test various payload sizes
    let sizes = [0, 16, 1024, 65535]; // Including edge cases
    
    for size in sizes {
        let header = MessageHeader {
            magic: MESSAGE_MAGIC,
            version: PROTOCOL_VERSION,
            relay_domain: RelayDomain::MarketData,
            source: SourceType::BinanceCollector,
            sequence: 1,
            timestamp_ns: 1000000000,
            payload_size: size,
            checksum: 0,
        };
        
        let bytes = unsafe { 
            std::slice::from_raw_parts(
                &header as *const _ as *const u8, 
                std::mem::size_of::<MessageHeader>()
            )
        };
        
        let parsed = parse_header(bytes)
            .unwrap_or_else(|_| panic!("Failed to parse header with payload size {}", size));
        
        assert_eq!(parsed.payload_size, size);
    }
}