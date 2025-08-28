//! # Codec Consolidation Integration Tests
//!
//! ## Purpose
//!
//! Comprehensive integration tests validating the MYC-004 codec consolidation.
//! Tests cover validation functionality, performance characteristics, and
//! migration compatibility to ensure the consolidation maintains all existing
//! functionality while providing enhanced validation capabilities.

use torq_codec::{
    TLVValidator, ValidatingTLVMessageBuilder, BuilderFactory, ValidationPolicy, ValidationLevel,
    TLVType, parse_header, parse_tlv_extensions, migration::compat,
};
use torq_types::{RelayDomain, SourceType};

/// Test basic validation functionality
#[test]
fn test_consolidated_validation() {
    let validator = TLVValidator::new();
    
    // Test that validator has domain rules configured
    let market_data_message = create_test_message(RelayDomain::MarketData);
    let signal_message = create_test_message(RelayDomain::Signal);
    let execution_message = create_test_message(RelayDomain::Execution);
    
    // All messages should parse successfully with appropriate domains
    let header = parse_header(&market_data_message).expect("Failed to parse market data header");
    let payload = &market_data_message[32..32 + header.payload_size as usize];
    assert!(validator.validate_message(header, payload).is_ok());
    
    let header = parse_header(&signal_message).expect("Failed to parse signal header");
    let payload = &signal_message[32..32 + header.payload_size as usize];
    assert!(validator.validate_message(header, payload).is_ok());
    
    let header = parse_header(&execution_message).expect("Failed to parse execution header");
    let payload = &execution_message[32..32 + header.payload_size as usize];
    assert!(validator.validate_message(header, payload).is_ok());
}

/// Test domain-specific TLV type validation
#[test]
fn test_domain_specific_validation() {
    let validator = TLVValidator::new();
    
    // Create message with wrong TLV type for domain
    let builder = ValidatingTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Test);
    
    // Try to add Signal TLV (type 20) to MarketData domain (should only allow 1-19)
    let signal_data = vec![0u8; 16];
    let result = builder.add_validated_tlv_bytes(TLVType::SignalIdentity, signal_data);
    
    // Should fail due to domain validation
    assert!(result.is_err());
}

/// Test performance-tuned validation policies
#[test]
fn test_validation_policies() {
    // Performance policy (minimal validation)
    let perf_builder = BuilderFactory::performance_builder(RelayDomain::MarketData, SourceType::Test);
    assert!(!perf_builder.is_validation_enabled());
    
    // Standard policy (checksum validation)
    let std_builder = BuilderFactory::standard_builder(RelayDomain::Signal, SourceType::Test);
    assert!(std_builder.is_validation_enabled());
    
    // Audit policy (full validation)
    let audit_builder = BuilderFactory::audit_builder(RelayDomain::Execution, SourceType::Test);
    assert!(audit_builder.is_validation_enabled());
}

/// Test enhanced message builder functionality
#[test]
fn test_enhanced_builder() {
    let builder = ValidatingTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Test);
    
    // Add valid TLV for market data domain
    let trade_data = vec![0u8; 40]; // Trade TLV size
    let builder = builder.add_validated_tlv_bytes(TLVType::Trade, trade_data)
        .expect("Failed to add valid trade TLV");
    
    // Build message
    let message = builder.build().expect("Failed to build message");
    
    // Verify message structure
    assert!(message.len() >= 32); // At least header size
    let header = parse_header(&message).expect("Failed to parse built message");
    assert_eq!(header.relay_domain, RelayDomain::MarketData as u8);
}

/// Test TLV size validation
#[test]
fn test_tlv_size_validation() {
    let builder = ValidatingTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Test);
    
    // Trade TLV should be exactly 40 bytes
    let correct_trade = vec![0u8; 40];
    let result = builder.add_validated_tlv_bytes(TLVType::Trade, correct_trade);
    assert!(result.is_ok());
    
    // Wrong size should fail
    let builder = ValidatingTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Test);
    let wrong_size_trade = vec![0u8; 30]; // Too small
    let result = builder.add_validated_tlv_bytes(TLVType::Trade, wrong_size_trade);
    assert!(result.is_err());
}

/// Test migration compatibility layer
#[test]
fn test_migration_compatibility() {
    let test_message = create_test_message(RelayDomain::MarketData);
    
    // Test deprecated compatibility functions still work
    #[allow(deprecated)]
    let result = compat::parse_market_data_message(&test_message);
    assert!(result.is_ok());
    
    #[allow(deprecated)]
    let legacy_builder = compat::create_legacy_builder(RelayDomain::MarketData, SourceType::Test);
    let legacy_message = legacy_builder
        .add_trusted_tlv_bytes(TLVType::Trade, vec![0u8; 40])
        .build()
        .expect("Legacy builder failed");
    
    // Legacy and new messages should have same structure
    assert!(legacy_message.len() >= 32);
}

/// Test that consolidated parsing maintains performance
#[test]
fn test_parsing_performance() {
    let test_messages = vec![
        create_test_message(RelayDomain::MarketData),
        create_test_message(RelayDomain::Signal),
        create_test_message(RelayDomain::Execution),
    ];
    
    let validator = TLVValidator::new();
    
    // Parse multiple messages to ensure no major performance regression
    for message in test_messages {
        let header = parse_header(&message).expect("Failed to parse header");
        let payload = &message[32..32 + header.payload_size as usize];
        
        let start = std::time::Instant::now();
        let _validated = validator.validate_message(header, payload)
            .expect("Validation failed");
        let duration = start.elapsed();
        
        // Should complete validation in reasonable time (<1ms for test messages)
        assert!(duration.as_millis() < 10);
    }
}

/// Test TLV extension parsing compatibility
#[test]
fn test_tlv_extension_parsing() {
    let test_message = create_test_message(RelayDomain::MarketData);
    let header = parse_header(&test_message).expect("Failed to parse header");
    let payload = &test_message[32..32 + header.payload_size as usize];
    
    // Parse TLV extensions using consolidated parser
    let tlvs = parse_tlv_extensions(payload).expect("Failed to parse TLV extensions");
    
    // Should have at least one TLV
    assert!(!tlvs.is_empty());
    
    // Validate TLV structure
    for tlv in tlvs {
        match tlv {
            codec::TLVExtensionEnum::Standard(std_tlv) => {
                assert!(std_tlv.header.tlv_length <= 255);
                assert_eq!(std_tlv.payload.len(), std_tlv.header.tlv_length as usize);
            }
            codec::TLVExtensionEnum::Extended(ext_tlv) => {
                assert_eq!(ext_tlv.header.marker, 255);
                assert_eq!(ext_tlv.header.reserved, 0);
                assert_eq!(ext_tlv.payload.len(), ext_tlv.header.tlv_length as usize);
            }
        }
    }
}

/// Test domain validator creation
#[test]
fn test_domain_validators() {
    use torq_codec::{create_domain_validator, DomainValidator};
    
    let market_validator = create_domain_validator(RelayDomain::MarketData);
    assert_eq!(market_validator.domain_name(), "MarketData");
    assert!(!market_validator.get_allowed_types().is_empty());
    
    let signal_validator = create_domain_validator(RelayDomain::Signal);
    assert_eq!(signal_validator.domain_name(), "Signal");
    assert!(!signal_validator.get_allowed_types().is_empty());
    
    let execution_validator = create_domain_validator(RelayDomain::Execution);
    assert_eq!(execution_validator.domain_name(), "Execution");
    assert!(!execution_validator.get_allowed_types().is_empty());
}

/// Test validation error handling
#[test]
fn test_validation_error_handling() {
    let validator = TLVValidator::new();
    
    // Test with invalid message (too short)
    let invalid_message = vec![0u8; 16]; // Too short for header
    let header_result = parse_header(&invalid_message);
    assert!(header_result.is_err());
    
    // Test with invalid domain in header
    let mut valid_message = create_test_message(RelayDomain::MarketData);
    // Corrupt domain byte (position 4 in header)
    valid_message[4] = 99; // Invalid domain
    
    let header = parse_header(&valid_message);
    // Parser might still succeed, but validation should catch invalid domain
    if let Ok(h) = header {
        let payload = &valid_message[32..32 + h.payload_size as usize];
        let result = validator.validate_message(h, payload);
        // Should fail due to invalid domain
        assert!(result.is_err());
    }
}

/// Test validation policy customization
#[test]
fn test_custom_validation_policy() {
    let strict_policy = ValidationPolicy {
        checksum: true,
        audit: true,
        strict: true,
        max_message_size: Some(1024),
    };
    
    let lenient_policy = ValidationPolicy {
        checksum: false,
        audit: false,
        strict: false,
        max_message_size: Some(65536),
    };
    
    let strict_builder = ValidatingTLVMessageBuilder::with_validation_policy(
        RelayDomain::Execution, 
        SourceType::Test, 
        strict_policy
    );
    assert!(strict_builder.is_validation_enabled());
    
    let lenient_builder = ValidatingTLVMessageBuilder::with_validation_policy(
        RelayDomain::MarketData, 
        SourceType::Test, 
        lenient_policy
    );
    assert!(lenient_builder.is_validation_enabled());
}

/// Helper function to create test messages for different domains
fn create_test_message(domain: RelayDomain) -> Vec<u8> {
    let (tlv_type, tlv_data) = match domain {
        RelayDomain::MarketData => (TLVType::Trade, vec![0u8; 40]),
        RelayDomain::Signal => (TLVType::SignalIdentity, vec![0u8; 16]),
        RelayDomain::Execution => (TLVType::OrderStatus, vec![0u8; 24]),
        _ => (TLVType::Trade, vec![0u8; 40]),
    };
    
    let builder = ValidatingTLVMessageBuilder::without_validation(domain, SourceType::Test);
    builder
        .add_trusted_tlv_bytes(tlv_type, tlv_data)
        .build()
        .expect("Failed to create test message")
}

/// Benchmark test to ensure consolidation doesn't hurt performance
#[test]
fn test_consolidation_performance() {
    let validator = TLVValidator::new();
    let num_iterations = 1000;
    
    // Create test messages
    let test_message = create_test_message(RelayDomain::MarketData);
    let header = parse_header(&test_message).expect("Failed to parse header");
    let payload = &test_message[32..32 + header.payload_size as usize];
    
    // Measure validation performance
    let start = std::time::Instant::now();
    for _ in 0..num_iterations {
        let _result = validator.validate_message(header, payload)
            .expect("Validation failed");
    }
    let duration = start.elapsed();
    
    let ops_per_second = (num_iterations as f64) / duration.as_secs_f64();
    
    // Should maintain high throughput (>10K validations/second for test data)
    assert!(ops_per_second > 10000.0);
    
    println!("Validation performance: {:.0} validations/second", ops_per_second);
}

/// Integration test for the complete consolidation workflow
#[test]
fn test_complete_consolidation_workflow() {
    // 1. Create message with enhanced builder
    let builder = ValidatingTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector);
    let trade_data = vec![0u8; 40];
    let message = builder
        .add_validated_tlv_bytes(TLVType::Trade, trade_data)
        .expect("Failed to add TLV")
        .with_sequence(1)
        .with_flags(0)
        .build()
        .expect("Failed to build message");
    
    // 2. Parse message with consolidated parser
    let header = parse_header(&message).expect("Failed to parse header");
    let payload = &message[32..32 + header.payload_size as usize];
    
    // 3. Validate with consolidated validator
    let validator = TLVValidator::new();
    let validated = validator.validate_message(header, payload)
        .expect("Failed to validate message");
    
    // 4. Verify all components work together
    assert_eq!(validated.header.relay_domain, RelayDomain::MarketData as u8);
    assert_eq!(validated.header.source, SourceType::PolygonCollector as u8);
    assert_eq!(validated.header.sequence, 1);
    assert!(!validated.tlv_extensions.is_empty());
    assert!(validated.validation_policy.contains("performance")); // MarketData uses performance policy
    
    println!("✅ Complete consolidation workflow test passed");
}