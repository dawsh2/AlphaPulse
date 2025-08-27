//! # MYC-004 Codec Consolidation Validation Demo
//!
//! Demonstrates the enhanced validation system with all critical fixes:
//! - Proper checksum validation per Protocol V2 spec
//! - Sequence number validation with gap detection
//! - Timestamp validation with bounds checking
//! - Zero-allocation parsing
//! - Configurable validation parameters

use codec::{
    ValidationConfig, EnhancedTLVValidator, SequenceTracker,
    TLVMessageBuilder, ValidatingTLVMessageBuilder,
};
use alphapulse_types::{RelayDomain, SourceType, protocol::tlv::types::TLVType};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    println!("🚀 MYC-004 Codec Consolidation Validation Demo");
    println!("===============================================\n");
    
    // 1. Demonstrate enhanced validation with all fixes
    demo_enhanced_validation();
    
    // 2. Demonstrate sequence tracking
    demo_sequence_validation();
    
    // 3. Demonstrate configurable validation
    demo_configurable_validation();
    
    // 4. Show performance comparison
    demo_performance();
    
    println!("✅ All critical fixes successfully demonstrated!");
}

fn demo_enhanced_validation() {
    println!("🔍 1. Enhanced Validation with All Critical Fixes");
    println!("   - Protocol V2 checksum validation");
    println!("   - Timestamp bounds checking");
    println!("   - Zero-copy parsing");
    
    let config = ValidationConfig::production();
    let validator = EnhancedTLVValidator::new(config);
    
    // Create a valid message using the enhanced builder
    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
        .add_tlv_bytes(TLVType::Trade, vec![0u8; 40])
        .build()
        .expect("Failed to build message");
    
    // Validate the message
    match validator.validate_message(&message) {
        Ok(validated) => {
            println!("   ✅ Message validation passed!");
            println!("   📊 Header checksum: 0x{:08x}", validated.header.checksum);
            println!("   📝 TLV extensions: {} found", validated.tlv_extensions.len());
        }
        Err(e) => {
            println!("   ❌ Message validation failed: {}", e);
        }
    }
    println!();
}

fn demo_sequence_validation() {
    println!("🔢 2. Sequence Number Validation with Gap Detection");
    
    let mut tracker = SequenceTracker::new(100);
    let max_gap = 10u64;
    let source = 1u8;
    
    // Normal sequence
    match tracker.validate_sequence(source, 100, max_gap) {
        Ok(_) => println!("   ✅ Sequence 100: OK (initial)"),
        Err(e) => println!("   ❌ Sequence 100: {}", e),
    }
    
    match tracker.validate_sequence(source, 101, max_gap) {
        Ok(_) => println!("   ✅ Sequence 101: OK (consecutive)"),
        Err(e) => println!("   ❌ Sequence 101: {}", e),
    }
    
    // Small gap (within tolerance)
    match tracker.validate_sequence(source, 105, max_gap) {
        Ok(_) => println!("   ✅ Sequence 105: OK (gap=3, within tolerance)"),
        Err(e) => println!("   ❌ Sequence 105: {}", e),
    }
    
    // Large gap (exceeds tolerance)
    match tracker.validate_sequence(source, 120, max_gap) {
        Ok(_) => println!("   ✅ Sequence 120: OK"),
        Err(e) => println!("   ⚠️  Sequence 120: {} (expected behavior)", e),
    }
    
    // Duplicate sequence
    match tracker.validate_sequence(source, 105, max_gap) {
        Ok(_) => println!("   ✅ Sequence 105 (duplicate): OK"),
        Err(e) => println!("   ⚠️  Sequence 105 (duplicate): {} (expected behavior)", e),
    }
    println!();
}

fn demo_configurable_validation() {
    println!("⚙️  3. Configurable Validation Parameters");
    
    // Production config (strict)
    let prod_config = ValidationConfig::production();
    println!("   📋 Production Config:");
    println!("      - Market data max size: {} bytes", prod_config.max_message_sizes.market_data);
    println!("      - Max sequence gap: {}", prod_config.sequence.max_sequence_gap);
    println!("      - Timestamp validation: {}", prod_config.timestamp.enforce_validation);
    
    // Development config (relaxed)
    let dev_config = ValidationConfig::development();
    println!("   🔧 Development Config:");
    println!("      - Market data max size: {} bytes", dev_config.max_message_sizes.market_data);
    println!("      - Max sequence gap: {}", dev_config.sequence.max_sequence_gap);
    println!("      - Timestamp validation: {}", dev_config.timestamp.enforce_validation);
    
    // Validate timestamp with current time
    let current_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    let prod_validator = EnhancedTLVValidator::new(prod_config);
    let dev_validator = EnhancedTLVValidator::new(dev_config);
    
    // Test with far future timestamp (should fail in production)
    let future_timestamp = current_ns + 10_000_000_000; // 10 seconds future
    
    match prod_validator.validate_timestamp(future_timestamp) {
        Ok(_) => println!("   ✅ Production validator: Future timestamp OK"),
        Err(e) => println!("   ⚠️  Production validator: {} (expected in strict mode)", e),
    }
    
    match dev_validator.validate_timestamp(future_timestamp) {
        Ok(_) => println!("   ✅ Development validator: Future timestamp OK (relaxed mode)"),
        Err(e) => println!("   ❌ Development validator: {}", e),
    }
    println!();
}

fn demo_performance() {
    println!("🚀 4. Performance Demonstration");
    
    // Create test message
    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
        .add_tlv_bytes(TLVType::Trade, vec![0u8; 40])
        .add_tlv_bytes(TLVType::Quote, vec![0u8; 52])
        .build()
        .expect("Failed to build message");
    
    // Performance mode (minimal validation)
    let perf_config = ValidationConfig::default();
    let perf_validator = EnhancedTLVValidator::new(perf_config);
    
    let iterations = 1000;
    let start = std::time::Instant::now();
    
    for _ in 0..iterations {
        let _ = perf_validator.validate_message(&message).unwrap();
    }
    
    let duration = start.elapsed();
    let validations_per_second = (iterations as f64) / duration.as_secs_f64();
    
    println!("   📈 Validation Performance:");
    println!("      - {} iterations in {:?}", iterations, duration);
    println!("      - {:.0} validations/second", validations_per_second);
    println!("      - {:.2} μs per validation", duration.as_micros() as f64 / iterations as f64);
    
    // Zero-copy parsing demonstration
    let payload = &message[32..];
    let start = std::time::Instant::now();
    
    for _ in 0..iterations {
        let _ = perf_validator.parse_tlv_zero_copy(payload, RelayDomain::MarketData).unwrap();
    }
    
    let duration = start.elapsed();
    let parses_per_second = (iterations as f64) / duration.as_secs_f64();
    
    println!("   🔄 Zero-Copy Parsing Performance:");
    println!("      - {} parses in {:?}", iterations, duration);
    println!("      - {:.0} parses/second", parses_per_second);
    println!("      - {:.2} μs per parse", duration.as_micros() as f64 / iterations as f64);
    println!();
}