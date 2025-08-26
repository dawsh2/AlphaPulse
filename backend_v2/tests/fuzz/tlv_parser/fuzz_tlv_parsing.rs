//! TLV Parser Fuzz Tests
//!
//! These tests feed random and malformed data to the TLV parser to ensure
//! it handles malicious or corrupted inputs gracefully without crashing
//! or causing security vulnerabilities.

use arbitrary::{Arbitrary, Unstructured};
use protocol_v2::{parse_header, tlv::parse_tlv_extensions, ParseError};

#[derive(Debug, Clone, Arbitrary)]
pub struct FuzzTLVMessage {
    pub header_bytes: [u8; 32],
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Arbitrary)]
pub struct FuzzTLVExtension {
    pub tlv_type: u16,
    pub reserved: u16,
    pub length: u16,
    pub padding: u16,
    pub payload: Vec<u8>,
}

/// Fuzz test for header parsing
pub fn fuzz_header_parsing(data: &[u8]) -> Result<(), String> {
    // Try to parse header from arbitrary data
    match parse_header(data) {
        Ok(header) => {
            // If parsing succeeds, validate the header makes sense
            if header.payload_size > 1_000_000 {
                return Err("Header claims impossibly large payload".to_string());
            }
            
            // Check magic number is reasonable (either valid or invalid)
            if header.magic != protocol_v2::MESSAGE_MAGIC && header.magic != 0 {
                // This is fine - invalid magic should be handled gracefully
            }
            
            Ok(())
        }
        Err(ParseError::InsufficientBytes { required, available }) => {
            // This is expected for short inputs
            if required > available && available < 32 {
                Ok(()) // Normal case
            } else {
                Err(format!("Unexpected insufficient bytes: required {}, available {}", required, available))
            }
        }
        Err(ParseError::InvalidMagic { expected: _, actual: _ }) => {
            // Expected for random data
            Ok(())
        }
        Err(ParseError::UnsupportedVersion { version: _ }) => {
            // Expected for random version bytes
            Ok(())
        }
        Err(e) => {
            Err(format!("Unexpected parse error: {:?}", e))
        }
    }
}

/// Fuzz test for TLV payload parsing
pub fn fuzz_tlv_payload_parsing(data: &[u8]) -> Result<(), String> {
    // Try to parse TLV extensions from arbitrary payload data
    match parse_tlv_extensions(data) {
        Ok(tlvs) => {
            // If parsing succeeds, validate TLV structure
            for tlv in tlvs {
                if tlv.header.length > 10_000 {
                    return Err("TLV claims impossibly large length".to_string());
                }
                
                if tlv.payload.len() != tlv.header.length as usize {
                    return Err("TLV payload length mismatch".to_string());
                }
            }
            Ok(())
        }
        Err(ParseError::MessageTooSmall { .. }) => {
            // Expected for malformed data
            Ok(())
        }
        Err(ParseError::PayloadTooLarge { .. }) => {
            // Expected for malformed TLV with excessive size claims
            Ok(())
        }
        Err(ParseError::TruncatedTLV { .. }) => {
            // Expected for incomplete TLV
            Ok(())
        }
        Err(e) => {
            Err(format!("Unexpected TLV parse error: {:?}", e))
        }
    }
}

/// Fuzz test for complete message parsing
pub fn fuzz_complete_message(data: &[u8]) -> Result<(), String> {
    if data.len() < 32 {
        // Too short for header
        return fuzz_header_parsing(data);
    }
    
    // Try to parse header
    let header = match parse_header(&data[..32]) {
        Ok(h) => h,
        Err(_) => return Ok(()), // Invalid header is fine
    };
    
    // Check if we have enough data for claimed payload
    let total_expected = 32 + header.payload_size as usize;
    if data.len() < total_expected {
        // Insufficient data for payload
        return Ok(());
    }
    
    // Try to parse payload
    let payload = &data[32..32 + header.payload_size as usize];
    fuzz_tlv_payload_parsing(payload)
}

/// Fuzz test for TLV construction and parsing round-trip
pub fn fuzz_tlv_roundtrip(message: FuzzTLVMessage) -> Result<(), String> {
    use protocol_v2::tlv::TLVMessageBuilder;
    use protocol_v2::{RelayDomain, SourceType};
    
    // Create a TLV message builder
    let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Dashboard);
    
    // Add arbitrary TLV data (but keep sizes reasonable)
    let mut total_payload_size = 0;
    let mut payload_chunks = Vec::new();
    
    let mut data_iter = message.payload.chunks(100); // Limit chunk size
    for (i, chunk) in data_iter.enumerate() {
        if i > 50 || total_payload_size > 5000 {
            break; // Limit total size to prevent resource exhaustion
        }
        
        let tlv_type = (i % 79) as u16 + 1; // Valid TLV type range 1-79
        builder.add_tlv(tlv_type, chunk);
        payload_chunks.push((tlv_type, chunk.to_vec()));
        total_payload_size += chunk.len() + 8; // 8 bytes for TLV header
    }
    
    // Build the message
    let constructed_message = builder.build();
    
    // Try to parse it back
    let parsed_header = parse_header(&constructed_message)
        .map_err(|e| format!("Failed to parse constructed header: {:?}", e))?;
    
    if constructed_message.len() < 32 + parsed_header.payload_size as usize {
        return Err("Constructed message shorter than header claims".to_string());
    }
    
    let payload = &constructed_message[32..32 + parsed_header.payload_size as usize];
    let parsed_tlvs = parse_tlv_extensions(payload)
        .map_err(|e| format!("Failed to parse constructed TLVs: {:?}", e))?;
    
    // Verify we got back the same number of TLVs
    if parsed_tlvs.len() != payload_chunks.len() {
        return Err(format!("TLV count mismatch: constructed {}, parsed {}", 
                          payload_chunks.len(), parsed_tlvs.len()));
    }
    
    Ok(())
}

/// Test that ensures parser doesn't crash on pathological inputs
pub fn fuzz_parser_stability(data: &[u8]) -> Result<(), String> {
    // Test various parsing scenarios that shouldn't crash
    
    // 1. Very long inputs
    if data.len() > 100_000 {
        return Ok(()); // Skip very large inputs to avoid resource exhaustion
    }
    
    // 2. Repeated patterns that might cause infinite loops
    let _ = fuzz_complete_message(data);
    
    // 3. All zeros
    if data.iter().all(|&b| b == 0) {
        let _ = fuzz_complete_message(data);
    }
    
    // 4. All ones
    if data.iter().all(|&b| b == 0xFF) {
        let _ = fuzz_complete_message(data);
    }
    
    // 5. Alternating patterns
    let alternating: Vec<u8> = (0..data.len()).map(|i| if i % 2 == 0 { 0x55 } else { 0xAA }).collect();
    let _ = fuzz_complete_message(&alternating);
    
    Ok(())
}

#[cfg(test)]
mod fuzz_test_runner {
    use super::*;
    use arbitrary::Unstructured;
    
    #[test]
    fn test_fuzz_header_parsing_basic() {
        // Test with various basic inputs
        let test_cases = [
            vec![], // Empty
            vec![0u8; 16], // Too short
            vec![0u8; 32], // Minimum size
            vec![0xFFu8; 32], // All ones
            vec![0x55u8; 32], // Pattern
        ];
        
        for (i, test_case) in test_cases.iter().enumerate() {
            match fuzz_header_parsing(test_case) {
                Ok(()) => {}, // Good
                Err(e) => panic!("Fuzz test {} failed: {}", i, e),
            }
        }
    }
    
    #[test]
    fn test_fuzz_tlv_payload_parsing_basic() {
        let test_cases = [
            vec![], // Empty payload
            vec![0u8; 8], // Minimum TLV size
            vec![1, 0, 4, 0, 0, 0, 0, 0, 0x01, 0x02, 0x03, 0x04], // Valid single TLV
            vec![0xFFu8; 100], // Malformed data
        ];
        
        for (i, test_case) in test_cases.iter().enumerate() {
            match fuzz_tlv_payload_parsing(test_case) {
                Ok(()) => {},
                Err(e) => panic!("TLV fuzz test {} failed: {}", i, e),
            }
        }
    }
    
    #[test]
    fn test_fuzz_parser_stability_basic() {
        let test_cases = [
            vec![0u8; 1000], // Zeros
            vec![0xFFu8; 1000], // Ones  
            (0..1000u8).collect::<Vec<u8>>(), // Sequential
        ];
        
        for (i, test_case) in test_cases.iter().enumerate() {
            match fuzz_parser_stability(test_case) {
                Ok(()) => {},
                Err(e) => panic!("Parser stability test {} failed: {}", i, e),
            }
        }
    }
    
    #[test]
    fn test_fuzz_known_bad_inputs() {
        // Test inputs that have historically caused issues
        
        // 1. TLV length longer than available data
        let bad_tlv = vec![
            1, 0,           // type = 1
            255, 255,       // length = 65535 (way too long)
            0, 0, 0, 0,     // reserved + padding
            1, 2, 3, 4,     // only 4 bytes of data
        ];
        
        let _ = fuzz_tlv_payload_parsing(&bad_tlv); // Should not crash
        
        // 2. Header claiming huge payload
        let mut bad_header = vec![0u8; 32];
        bad_header[0..4].copy_from_slice(&protocol_v2::MESSAGE_MAGIC.to_le_bytes());
        bad_header[4] = 1; // version
        bad_header[24..28].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // huge payload
        
        let _ = fuzz_header_parsing(&bad_header); // Should not crash
        
        // 3. Nested TLVs with circular references (if supported)
        // This would be implementation-specific
    }
    
    #[test]
    fn test_resource_exhaustion_protection() {
        // Ensure parser doesn't use excessive resources
        
        use std::time::{Duration, Instant};
        
        // 1. Very large claimed payload size
        let mut message = vec![0u8; 32];
        message[0..4].copy_from_slice(&protocol_v2::MESSAGE_MAGIC.to_le_bytes());
        message[4] = 1; // version
        message[24..28].copy_from_slice(&1000000u32.to_le_bytes()); // 1MB payload claim
        
        let start = Instant::now();
        let _ = fuzz_complete_message(&message);
        let duration = start.elapsed();
        
        assert!(duration < Duration::from_millis(100), 
               "Parser took too long: {:?}", duration);
        
        // 2. Many small TLVs
        let mut many_tlvs = Vec::new();
        for i in 0..1000u16 {
            many_tlvs.extend_from_slice(&[
                (i % 79 + 1) as u8, ((i % 79 + 1) >> 8) as u8, // type
                4, 0,       // length = 4
                0, 0, 0, 0, // reserved + padding
                1, 2, 3, 4, // payload
            ]);
        }
        
        let start = Instant::now();
        let _ = fuzz_tlv_payload_parsing(&many_tlvs);
        let duration = start.elapsed();
        
        assert!(duration < Duration::from_millis(500),
               "TLV parsing took too long: {:?}", duration);
    }
}

// Integration with cargo-fuzz (if available)
#[cfg(feature = "fuzz")]
pub mod cargo_fuzz_targets {
    use super::*;
    
    /// Fuzz target for libfuzzer
    pub fn fuzz_target_header(data: &[u8]) {
        let _ = fuzz_header_parsing(data);
    }
    
    /// Fuzz target for TLV parsing
    pub fn fuzz_target_tlv(data: &[u8]) {
        let _ = fuzz_tlv_payload_parsing(data);
    }
    
    /// Fuzz target for complete messages
    pub fn fuzz_target_message(data: &[u8]) {
        let _ = fuzz_complete_message(data);
    }
    
    /// Fuzz target for roundtrip testing
    pub fn fuzz_target_roundtrip(data: &[u8]) {
        if let Ok(mut unstructured) = Unstructured::new(data) {
            if let Ok(message) = FuzzTLVMessage::arbitrary(&mut unstructured) {
                let _ = fuzz_tlv_roundtrip(message);
            }
        }
    }
}