//! # Critical Fixes Validation Tests
//!
//! Tests to validate all critical issues have been properly addressed:
//! 1. Proper checksum validation per Protocol V2 spec
//! 2. Sequence number validation with gap detection
//! 3. Timestamp validation with bounds checking
//! 4. Zero-allocation parsing in hot path
//! 5. Error context preservation
//! 6. Configurable validation parameters

use codec::{
    EnhancedTLVValidator, EnhancedValidationError, ValidationConfig,
    SequenceTracker, PoolDiscoveryQueue, TLVType,
    ValidatingTLVMessageBuilder, ValidationError,
};
use alphapulse_types::{RelayDomain, SourceType};
use std::time::{SystemTime, UNIX_EPOCH};

/// Test 1: Proper checksum validation per Protocol V2 spec
#[test]
fn test_checksum_validation_protocol_v2() {
    let config = ValidationConfig::production(); // Strict mode
    let validator = EnhancedTLVValidator::new(config);
    
    // Create a test message with correct Protocol V2 structure
    let mut message = create_test_message_with_checksum();
    
    // Validation should pass with correct checksum
    let result = validator.validate_message(&message);
    assert!(result.is_ok(), "Valid checksum should pass validation");
    
    // Corrupt the checksum field and test again
    corrupt_checksum(&mut message);
    let result = validator.validate_message(&message);
    assert!(matches!(result, Err(EnhancedValidationError::ChecksumMismatch { .. })));
    
    // Corrupt payload and test checksum detection
    message = create_test_message_with_checksum();
    corrupt_payload(&mut message);
    let result = validator.validate_message(&message);
    assert!(matches!(result, Err(EnhancedValidationError::ChecksumMismatch { .. })));
}

/// Test 2: Sequence number validation with gap detection
#[test]
fn test_sequence_validation_comprehensive() {
    let mut tracker = SequenceTracker::new(100);
    let max_gap = 10u64;
    
    // Test normal sequence progression
    assert!(tracker.validate_sequence(1, 100, max_gap).is_ok());
    assert!(tracker.validate_sequence(1, 101, max_gap).is_ok());
    assert!(tracker.validate_sequence(1, 102, max_gap).is_ok());
    
    // Test gap within tolerance
    assert!(tracker.validate_sequence(1, 105, max_gap).is_ok());
    
    // Test gap exceeding tolerance
    let result = tracker.validate_sequence(1, 120, max_gap);
    assert!(matches!(result, Err(EnhancedValidationError::SequenceGap { gap: 14, .. })));
    
    // Test duplicate sequence detection
    let result = tracker.validate_sequence(1, 105, max_gap);
    assert!(matches!(result, Err(EnhancedValidationError::DuplicateSequence { .. })));
    
    // Test backward sequence
    let result = tracker.validate_sequence(1, 103, max_gap);
    assert!(matches!(result, Err(EnhancedValidationError::DuplicateSequence { .. })));
    
    // Test different source doesn't interfere
    assert!(tracker.validate_sequence(2, 50, max_gap).is_ok());
    assert!(tracker.validate_sequence(2, 51, max_gap).is_ok());
}

/// Test 3: Timestamp validation with bounds checking
#[test]
fn test_timestamp_validation_bounds() {
    let config = ValidationConfig::production(); // Strict timestamp validation
    let validator = EnhancedTLVValidator::new(config);
    
    let current_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    // Current timestamp should be valid
    assert!(validator.validate_timestamp(current_ns).is_ok());
    
    // 1 second in future should be ok (within tolerance)
    assert!(validator.validate_timestamp(current_ns + 1_000_000_000).is_ok());
    
    // 5 seconds in future should fail (exceeds production tolerance)
    let result = validator.validate_timestamp(current_ns + 5_000_000_000);
    assert!(matches!(result, Err(EnhancedValidationError::InvalidTimestamp { .. })));
    
    // 10 seconds old should be ok
    assert!(validator.validate_timestamp(current_ns - 10_000_000_000).is_ok());
    
    // 45 seconds old should fail (exceeds production limit)
    let result = validator.validate_timestamp(current_ns - 45_000_000_000);
    assert!(matches!(result, Err(EnhancedValidationError::InvalidTimestamp { .. })));
}

/// Test 4: Zero-allocation parsing in hot path
#[test]
fn test_zero_allocation_parsing() {
    let config = ValidationConfig::default();
    let validator = EnhancedTLVValidator::new(config);
    
    // Create test message
    let message = create_test_message_market_data();
    let payload = &message[32..];
    
    // Parse with zero-copy method
    let result = validator.parse_tlv_zero_copy(payload, RelayDomain::MarketData);
    assert!(result.is_ok());
    
    let tlvs = result.unwrap();
    assert!(!tlvs.is_empty());
    
    // Verify TLV references point to original buffer
    for tlv in tlvs {
        match tlv {
            codec::TLVExtensionZeroCopy::Standard { payload, .. } => {
                // Verify payload is a slice reference, not an allocation
                assert!(payload.as_ptr() >= message.as_ptr());
                assert!(payload.as_ptr() < unsafe { message.as_ptr().add(message.len()) });
            }
            codec::TLVExtensionZeroCopy::Extended { payload, .. } => {
                // Same check for extended TLVs
                assert!(payload.as_ptr() >= message.as_ptr());
                assert!(payload.as_ptr() < unsafe { message.as_ptr().add(message.len()) });
            }
        }
    }
}

/// Test 5: Error context preservation
#[test]
fn test_error_context_preservation() {
    let builder = ValidatingTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Test);
    
    // Try to add invalid TLV type for domain
    let signal_data = vec![0u8; 16];
    let result = builder.add_validated_tlv_bytes(TLVType::SignalIdentity, signal_data);
    
    // Verify error contains specific context
    match result {
        Err(ValidationError::InvalidTLVForDomain { tlv_type: 20, domain }) => {
            assert_eq!(domain, RelayDomain::MarketData);
        }
        other => panic!("Expected InvalidTLVForDomain error, got {:?}", other),
    }
}

/// Test 6: Configurable validation parameters
#[test]
fn test_configurable_validation() {
    // Test production config (strict)
    let prod_config = ValidationConfig::production();
    assert_eq!(prod_config.max_message_sizes.market_data, 2048);
    assert_eq!(prod_config.sequence.max_sequence_gap, 50);
    assert!(prod_config.timestamp.enforce_validation);
    
    // Test development config (relaxed)
    let dev_config = ValidationConfig::development();
    assert_eq!(dev_config.max_message_sizes.market_data, 8192);
    assert_eq!(dev_config.sequence.max_sequence_gap, 1000);
    assert!(!dev_config.timestamp.enforce_validation);
    
    // Test environment override
    std::env::set_var("ALPHAPULSE_MAX_MESSAGE_SIZE_MARKET", "1024");
    let env_config = ValidationConfig::from_env();
    assert_eq!(env_config.max_message_sizes.market_data, 1024);
    std::env::remove_var("ALPHAPULSE_MAX_MESSAGE_SIZE_MARKET");
}

/// Test 7: Pool discovery queue mechanism
#[tokio::test]
async fn test_pool_discovery_queue() {
    let (queue, mut receiver) = PoolDiscoveryQueue::new();
    
    let config = ValidationConfig::default();
    let validator = EnhancedTLVValidator::new(config).with_pool_discovery(queue);
    
    // Create message with unknown pool
    let message = create_message_with_pool([0xAB; 20]);
    
    // Validation should queue the unknown pool
    let result = validator.validate_message(&message);
    assert!(matches!(result, Err(EnhancedValidationError::UnknownPool { .. })));
    
    // Check that pool was queued
    let queued_pool = receiver.recv().await.unwrap();
    assert_eq!(queued_pool, [0xAB; 20]);
    
    // Add pool as known and retry
    validator.add_known_pool([0xAB; 20]);
    let result = validator.validate_message(&message);
    assert!(result.is_ok());
}

/// Test 8: Enhanced builder timestamp validation
#[test]
fn test_enhanced_builder_timestamp_validation() {
    let builder = ValidatingTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Test);
    
    let current_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    // Valid timestamp should work
    let result = builder.with_timestamp(current_ns);
    assert!(result.is_ok());
    
    // Invalid timestamp should fail
    let builder = ValidatingTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Test);
    let future_timestamp = current_ns + 10_000_000_000; // 10 seconds future
    let result = builder.with_timestamp(future_timestamp);
    assert!(result.is_err());
}

/// Test 9: Performance regression check
#[test]
fn test_performance_no_regression() {
    let config = ValidationConfig::default();
    let validator = EnhancedTLVValidator::new(config);
    
    let message = create_test_message_market_data();
    let iterations = 1000;
    
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let _ = validator.validate_message(&message).unwrap();
    }
    let duration = start.elapsed();
    
    let validations_per_second = (iterations as f64) / duration.as_secs_f64();
    
    // Should maintain >10K validations/second for test messages
    assert!(validations_per_second > 10_000.0, 
        "Performance regression: only {:.0} validations/second", validations_per_second);
    
    println!("Enhanced validation performance: {:.0} validations/second", validations_per_second);
}

// Helper functions

fn create_test_message_with_checksum() -> Vec<u8> {
    use codec::TLVMessageBuilder;
    
    let builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Test);
    builder
        .add_tlv_bytes(TLVType::Trade, vec![0u8; 40])
        .build()
        .expect("Failed to create test message")
}

fn create_test_message_market_data() -> Vec<u8> {
    create_test_message_with_checksum()
}

fn create_message_with_pool(pool_addr: [u8; 20]) -> Vec<u8> {
    use codec::TLVMessageBuilder;
    
    // Create a PoolSwap TLV with the pool address
    let mut pool_swap_data = vec![0u8; 60];
    pool_swap_data[..20].copy_from_slice(&pool_addr); // Pool address at start
    
    let builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Test);
    builder
        .add_tlv_bytes(TLVType::PoolSwap, pool_swap_data)
        .build()
        .expect("Failed to create pool message")
}

fn corrupt_checksum(message: &mut [u8]) {
    // Checksum is at offset 28-31 in the header
    if message.len() >= 32 {
        message[28] ^= 0xFF; // Flip bits to corrupt checksum
    }
}

fn corrupt_payload(message: &mut [u8]) {
    // Corrupt first byte of payload
    if message.len() > 32 {
        message[32] ^= 0xFF;
    }
}

/// Integration test combining all fixes
#[test]
fn test_complete_validation_workflow() {
    let config = ValidationConfig::production();
    let (queue, _receiver) = PoolDiscoveryQueue::new();
    let validator = EnhancedTLVValidator::new(config).with_pool_discovery(queue);
    
    // Create a proper message
    let message = create_test_message_with_checksum();
    
    // Full validation should pass
    let result = validator.validate_message(&message);
    assert!(result.is_ok());
    
    let validated = result.unwrap();
    assert!(!validated.tlv_extensions.is_empty());
    
    println!("✅ Complete validation workflow test passed");
}