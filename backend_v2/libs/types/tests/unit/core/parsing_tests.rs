//! Core Parsing Unit Tests
//!
//! Tests for low-level parsing functions without TLV payload dependencies.

use protocol_v2::{parse_header, ParseError, MESSAGE_MAGIC};

#[test]
fn test_parse_header_insufficient_bytes() {
    // Header requires exactly 32 bytes
    let short_buffer = vec![0u8; 31]; // One byte short
    
    match parse_header(&short_buffer) {
        Err(ParseError::InsufficientBytes { required, available }) => {
            assert_eq!(required, 32);
            assert_eq!(available, 31);
        }
        _ => panic!("Should fail with InsufficientBytes"),
    }
}

#[test]
fn test_parse_header_exact_size() {
    // Create valid 32-byte header
    let mut buffer = vec![0u8; 32];
    
    // Set magic number (first 4 bytes, little endian)
    buffer[0..4].copy_from_slice(&MESSAGE_MAGIC.to_le_bytes());
    
    // Set version (byte 4)
    buffer[4] = 1; // PROTOCOL_VERSION
    
    // Set other required fields to valid values
    buffer[5] = 0; // RelayDomain::MarketData
    buffer[6..8].copy_from_slice(&0u16.to_le_bytes()); // SourceType
    buffer[8..16].copy_from_slice(&1u64.to_le_bytes()); // sequence
    buffer[16..24].copy_from_slice(&1000000000u64.to_le_bytes()); // timestamp
    buffer[24..28].copy_from_slice(&0u32.to_le_bytes()); // payload_size
    buffer[28..32].copy_from_slice(&0u32.to_le_bytes()); // checksum
    
    let result = parse_header(&buffer);
    assert!(result.is_ok(), "Valid header should parse successfully");
}

#[test]
fn test_parse_header_extra_bytes() {
    // Header parsing should work with extra bytes (for full message parsing)
    let mut buffer = vec![0u8; 64]; // Double the required size
    
    // Set magic number (first 4 bytes, little endian)
    buffer[0..4].copy_from_slice(&MESSAGE_MAGIC.to_le_bytes());
    
    // Set version (byte 4)
    buffer[4] = 1; // PROTOCOL_VERSION
    
    // Set other required fields to valid values
    buffer[5] = 0; // RelayDomain::MarketData
    buffer[6..8].copy_from_slice(&0u16.to_le_bytes()); // SourceType
    buffer[8..16].copy_from_slice(&1u64.to_le_bytes()); // sequence
    buffer[16..24].copy_from_slice(&1000000000u64.to_le_bytes()); // timestamp
    buffer[24..28].copy_from_slice(&0u32.to_le_bytes()); // payload_size
    buffer[28..32].copy_from_slice(&0u32.to_le_bytes()); // checksum
    
    let result = parse_header(&buffer);
    assert!(result.is_ok(), "Header parsing should work with extra bytes");
}

#[test]
fn test_parse_header_zero_buffer() {
    let empty_buffer: &[u8] = &[];
    
    match parse_header(empty_buffer) {
        Err(ParseError::InsufficientBytes { required, available }) => {
            assert_eq!(required, 32);
            assert_eq!(available, 0);
        }
        _ => panic!("Should fail with InsufficientBytes for empty buffer"),
    }
}

#[test]
fn test_header_parsing_endianness() {
    // Test that multi-byte fields are parsed correctly (little-endian)
    let mut buffer = vec![0u8; 32];
    
    // Magic: 0xDEADBEEF in little-endian bytes
    buffer[0..4].copy_from_slice(&MESSAGE_MAGIC.to_le_bytes());
    buffer[4] = 1; // version
    buffer[5] = 1; // domain
    
    // Source: 0x1234 in little-endian
    buffer[6] = 0x34;
    buffer[7] = 0x12;
    
    // Sequence: 0x123456789ABCDEF0 in little-endian  
    let sequence = 0x123456789ABCDEF0u64;
    buffer[8..16].copy_from_slice(&sequence.to_le_bytes());
    
    // Timestamp: similar test value
    let timestamp = 0xFEDCBA9876543210u64;
    buffer[16..24].copy_from_slice(&timestamp.to_le_bytes());
    
    // Payload size: 0x12345678
    let payload_size = 0x12345678u32;
    buffer[24..28].copy_from_slice(&payload_size.to_le_bytes());
    
    // Checksum: 0x87654321
    let checksum = 0x87654321u32;
    buffer[28..32].copy_from_slice(&checksum.to_le_bytes());
    
    let header = parse_header(&buffer).expect("Should parse successfully");
    
    assert_eq!(header.magic, MESSAGE_MAGIC);
    assert_eq!(header.sequence, sequence);
    assert_eq!(header.timestamp_ns, timestamp);
    assert_eq!(header.payload_size, payload_size);
    assert_eq!(header.checksum, checksum);
}

#[test]
fn test_parsing_performance_single_header() {
    // Micro-benchmark: Single header parsing should be < 1μs
    let mut buffer = vec![0u8; 32];
    buffer[0..4].copy_from_slice(&MESSAGE_MAGIC.to_le_bytes());
    buffer[4] = 1; // version
    buffer[5] = 0; // domain
    
    let start = std::time::Instant::now();
    let _ = parse_header(&buffer).expect("Should parse");
    let duration = start.elapsed();
    
    // Single header parse should be extremely fast
    assert!(duration.as_nanos() < 10_000, 
            "Header parsing took {}ns, should be < 10μs", duration.as_nanos());
}

#[test]
fn test_parsing_batch_headers() {
    // Test parsing multiple headers efficiently
    let mut buffer = vec![0u8; 32];
    buffer[0..4].copy_from_slice(&MESSAGE_MAGIC.to_le_bytes());
    buffer[4] = 1; // version
    buffer[5] = 0; // domain
    
    let iterations = 10_000;
    let start = std::time::Instant::now();
    
    for _ in 0..iterations {
        let _ = parse_header(&buffer).expect("Should parse");
    }
    
    let duration = start.elapsed();
    let per_header_ns = duration.as_nanos() / iterations;
    
    // Should maintain high throughput
    assert!(per_header_ns < 1_000, 
            "Per-header parsing took {}ns, should be < 1μs", per_header_ns);
}