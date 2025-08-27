//! Comprehensive error formatting tests for enhanced Protocol V2 error reporting
//!
//! Validates that enhanced error messages provide actionable debugging information
//! with proper Debug vs Display formatting for different use cases.

use alphapulse_codec::error::ProtocolError;

#[test]
fn test_message_too_small_formatting() {
    let error = ProtocolError::message_too_small(32, 16, "MessageHeader parsing");
    
    // Test Debug formatting (detailed for logging)
    let debug_output = format!("{:?}", error);
    assert!(debug_output.contains("MessageTooSmall"));
    assert!(debug_output.contains("32"));
    assert!(debug_output.contains("16"));
    assert!(debug_output.contains("MessageHeader parsing"));
    
    // Test Display formatting (human-readable)
    let display_output = format!("{}", error);
    assert!(display_output.contains("Message too small"));
    assert!(display_output.contains("need 32 bytes"));
    assert!(display_output.contains("got 16"));
    assert!(display_output.contains("context: MessageHeader parsing"));
}

#[test]
fn test_invalid_magic_formatting() {
    let error = ProtocolError::invalid_magic(0xDEADBEEF, 0x12345678, 0);
    
    let display_output = format!("{}", error);
    assert!(display_output.contains("Invalid magic number"));
    assert!(display_output.contains("0xdeadbeef"));  // lowercase hex
    assert!(display_output.contains("0x12345678"));
    assert!(display_output.contains("offset: 0"));
    assert!(display_output.contains("indicates:"));  // Diagnostic information
    
    // Test with uninitialized buffer
    let error2 = ProtocolError::invalid_magic(0xDEADBEEF, 0x00000000, 0);
    let display2 = format!("{}", error2);
    assert!(display2.contains("uninitialized buffer"));
    
    // Test with corrupted buffer
    let error3 = ProtocolError::invalid_magic(0xDEADBEEF, 0xFFFFFFFF, 0);
    let display3 = format!("{}", error3);
    assert!(display3.contains("corrupted buffer"));
    
    // Test with endianness issue
    let error4 = ProtocolError::invalid_magic(0xDEADBEEF, 0xEFBEADDE, 0);  // Byte-swapped
    let display4 = format!("{}", error4);
    assert!(display4.contains("byte order"));
}

#[test]
fn test_checksum_mismatch_formatting() {
    let error = ProtocolError::checksum_mismatch(0x12345678, 0x87654321, 1024, 5);
    
    let display_output = format!("{}", error);
    assert!(display_output.contains("Checksum mismatch"));
    assert!(display_output.contains("expected 0x12345678"));
    assert!(display_output.contains("calculated 0x87654321"));
    assert!(display_output.contains("message: 1024 bytes"));
    assert!(display_output.contains("tlvs: 5"));
    assert!(display_output.contains("cause:"));
    
    // Test different likely causes
    let error2 = ProtocolError::checksum_mismatch(0, 0x12345678, 512, 3);
    let display2 = format!("{}", error2);
    assert!(display2.contains("message created without checksum"));
    
    let error3 = ProtocolError::checksum_mismatch(0x12345678, 0, 512, 3);
    let display3 = format!("{}", error3);
    assert!(display3.contains("checksum calculation failed"));
}

#[test]
fn test_truncated_tlv_formatting() {
    let error = ProtocolError::truncated_tlv(100, 150, 42, 75);
    
    let display_output = format!("{}", error);
    assert!(display_output.contains("Truncated TLV"));
    assert!(display_output.contains("need 150 bytes"));
    assert!(display_output.contains("buffer has 100"));
    assert!(display_output.contains("TLV type 42"));
    assert!(display_output.contains("offset 75"));
    assert!(display_output.contains("action:"));
    
    // Test different suggested actions
    let error2 = ProtocolError::truncated_tlv(0, 50, 1, 0);
    let display2 = format!("{}", error2);
    assert!(display2.contains("message framing"));
    
    let error3 = ProtocolError::truncated_tlv(50, 1000, 1, 0);
    let display3 = format!("{}", error3);
    assert!(display3.contains("corrupted TLV length"));
}

#[test]
fn test_unknown_tlv_type_formatting() {
    let error = ProtocolError::UnknownTLVType { tlv_type: 99 };
    
    let display_output = format!("{}", error);
    assert!(display_output.contains("Unknown TLV type 99"));
    assert!(display_output.contains("1-19 (MarketData)"));
    assert!(display_output.contains("20-39 (Signals)"));
    assert!(display_output.contains("40-79 (Execution)"));
    assert!(display_output.contains("80-99 (System)"));
}

#[test]
fn test_invalid_extended_tlv_formatting() {
    let error = ProtocolError::invalid_extended_tlv(5, 0xFF00, 0x1234);
    
    let display_output = format!("{}", error);
    assert!(display_output.contains("Invalid extended TLV format"));
    assert!(display_output.contains("offset 5"));
    assert!(display_output.contains("expected marker 0xff00"));
    assert!(display_output.contains("got 0x1234"));
    assert!(display_output.contains("check:"));
    assert!(display_output.contains("0xFF00 marker"));
}

#[test]
fn test_payload_too_large_formatting() {
    let error = ProtocolError::payload_too_large(100000, 65535, 200);
    
    let display_output = format!("{}", error);
    assert!(display_output.contains("TLV payload too large"));
    assert!(display_output.contains("100000 bytes"));
    assert!(display_output.contains("exceeds limit 65535"));
    assert!(display_output.contains("type 200"));
    assert!(display_output.contains("consider:"));
    
    // Test large vs moderate size recommendations
    let error2 = ProtocolError::payload_too_large(1000000, 65535, 1);  // Very large
    let display2 = format!("{}", error2);
    assert!(display2.contains("corrupted length field"));
    
    let error3 = ProtocolError::payload_too_large(70000, 65535, 1);    // Moderately large
    let display3 = format!("{}", error3);
    assert!(display3.contains("fragmentation"));
}

#[test]
fn test_message_too_large_formatting() {
    let error = ProtocolError::MessageTooLarge {
        size: 100000,
        max: 65535,
        payload_size: 99968,
        tlv_count: 10,
    };
    
    let display_output = format!("{}", error);
    assert!(display_output.contains("Message too large"));
    assert!(display_output.contains("100000 bytes"));
    assert!(display_output.contains("exceeds maximum 65535"));
    assert!(display_output.contains("payload: 99968"));
    assert!(display_output.contains("tlvs: 10"));
}

#[test]
fn test_payload_size_mismatch_formatting() {
    let error = ProtocolError::PayloadSizeMismatch {
        tlv_type: 42,
        expected: 168,
        got: 150,
        struct_name: "ArbitrageSignalTLV".to_string(),
    };
    
    let display_output = format!("{}", error);
    assert!(display_output.contains("TLV payload size mismatch"));
    assert!(display_output.contains("type 42"));
    assert!(display_output.contains("expected 168 bytes"));
    assert!(display_output.contains("got 150"));
    assert!(display_output.contains("struct: ArbitrageSignalTLV"));
}

#[test]
fn test_invalid_payload_formatting() {
    let error = ProtocolError::invalid_payload(
        25,
        64,
        "Invalid timestamp: zero value not allowed",
        1024,
    );
    
    let display_output = format!("{}", error);
    assert!(display_output.contains("Invalid TLV payload"));
    assert!(display_output.contains("type 25"));
    assert!(display_output.contains("offset 64"));
    assert!(display_output.contains("Invalid timestamp"));
    assert!(display_output.contains("buffer: 1024 bytes"));
}

#[test]
fn test_unsupported_version_formatting() {
    let error = ProtocolError::UnsupportedVersion {
        version: 99,
        supported_versions: "1, 2, 3".to_string(),
    };
    
    let display_output = format!("{}", error);
    assert!(display_output.contains("Unsupported TLV version 99"));
    assert!(display_output.contains("supported versions are 1, 2, 3"));
}

#[test]
fn test_relay_domain_mismatch_formatting() {
    let error = ProtocolError::RelayDomainMismatch {
        expected: 1,
        got: 2,
        expected_name: "MarketData".to_string(),
        got_name: "Signals".to_string(),
    };
    
    let display_output = format!("{}", error);
    assert!(display_output.contains("Relay domain mismatch"));
    assert!(display_output.contains("expected 1 (MarketData)"));
    assert!(display_output.contains("got 2 (Signals)"));
    assert!(display_output.contains("route to correct relay"));
}

#[test]
fn test_parse_error_formatting() {
    let error = ProtocolError::parse_error(
        128,
        "Invalid field alignment",
        2048,
        "parsing OrderBookTLV entries",
    );
    
    let display_output = format!("{}", error);
    assert!(display_output.contains("Parse error at byte 128"));
    assert!(display_output.contains("Invalid field alignment"));
    assert!(display_output.contains("buffer: 2048 bytes"));
    assert!(display_output.contains("context: parsing OrderBookTLV"));
}

#[test]
fn test_error_chain_compatibility() {
    // Test that errors work with standard Error trait patterns
    let error = ProtocolError::message_too_small(32, 16, "test");
    
    // Should implement Error trait
    let _: &dyn std::error::Error = &error;
    
    // Should be cloneable and comparable
    let error2 = error.clone();
    assert_eq!(error, error2);
    
    // Should work with Result patterns
    let result: Result<(), ProtocolError> = Err(error);
    assert!(result.is_err());
}

#[test]
fn test_error_diagnostic_quality() {
    // Test that error messages provide actionable information
    
    // Magic number error should help identify the issue
    let magic_error = ProtocolError::invalid_magic(0xDEADBEEF, 0xEFBEADDE, 0);
    let magic_msg = format!("{}", magic_error);
    assert!(magic_msg.contains("byte order") || magic_msg.contains("endianness"));
    
    // Truncation error should suggest next steps
    let truncation_error = ProtocolError::truncated_tlv(0, 100, 1, 0);
    let truncation_msg = format!("{}", truncation_error);
    assert!(truncation_msg.contains("framing") || truncation_msg.contains("socket"));
    
    // Checksum error should indicate likely cause
    let checksum_error = ProtocolError::checksum_mismatch(0, 0x12345678, 1024, 3);
    let checksum_msg = format!("{}", checksum_error);
    assert!(checksum_msg.contains("without checksum") || checksum_msg.contains("failed"));
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    
    /// Test that enhanced errors integrate properly with error handling chains
    #[test]
    fn test_error_propagation_chain() {
        fn parse_mock_message() -> Result<(), ProtocolError> {
            // Simulate a parsing failure
            Err(ProtocolError::message_too_small(32, 16, "mock parsing"))
        }
        
        fn handle_message() -> Result<String, String> {
            parse_mock_message()
                .map(|_| "success".to_string())
                .map_err(|e| format!("Parse failed: {}", e))
        }
        
        let result = handle_message();
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Parse failed"));
        assert!(error_msg.contains("Message too small"));
        assert!(error_msg.contains("need 32 bytes"));
        assert!(error_msg.contains("mock parsing"));
    }
    
    /// Test that error context helps in production debugging scenarios
    #[test]
    fn test_production_debugging_scenarios() {
        // Scenario 1: Network corruption
        let network_error = ProtocolError::checksum_mismatch(0x12345678, 0x87654321, 1024, 3);
        let log_msg = format!("Network error: {}", network_error);
        assert!(log_msg.contains("data corruption"));
        
        // Scenario 2: Version mismatch
        let version_error = ProtocolError::UnsupportedVersion {
            version: 5,
            supported_versions: "1, 2, 3".to_string(),
        };
        let log_msg2 = format!("Protocol error: {}", version_error);
        assert!(log_msg2.contains("version 5"));
        
        // Scenario 3: Buffer management
        let buffer_error = ProtocolError::truncated_tlv(100, 1000, 42, 50);
        let log_msg3 = format!("Buffer error: {}", buffer_error);
        assert!(log_msg3.contains("corrupted") || log_msg3.contains("retry"));
    }
}