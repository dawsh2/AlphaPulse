//! Four-Step Validation Pipeline Tests
//!
//! Tests the complete validation framework with real exchange data:
//! 1. Raw Data Parsing Validation
//! 2. TLV Serialization Validation  
//! 3. TLV Deserialization Validation
//! 4. Semantic & Deep Equality Validation

use crate::fixtures::polygon;
use crate::*;
use adapter_service::input::collectors::polygon_dex::{
    abi_events::{DEXProtocol, SwapEventDecoder},
    validated_decoder::{PolygonRawSwapEvent, ValidatedPolygonDecoder},
};
use adapter_service::{
    validate_equality, validate_tlv_deserialization, validate_tlv_serialization, RawDataValidator,
};

#[test]
fn test_complete_four_step_validation_success() {
    // Use real Polygon Uniswap V3 data
    let log = polygon::uniswap_v3_swap_real();
    let decoder = ValidatedPolygonDecoder::new();

    // This should pass all four validation steps
    let result = decoder.decode_and_validate(&log, DEXProtocol::UniswapV3);

    assert!(
        result.is_ok(),
        "Four-step validation should succeed with real data: {:?}",
        result
    );

    let tlv = result.unwrap();

    // Verify semantic correctness
    assert_eq!(tlv.venue, VenueId::Polygon, "Venue should be Polygon");
    assert!(
        tlv.amount_in > 0,
        "amount_in should be positive: {}",
        tlv.amount_in
    );
    assert!(
        tlv.amount_out > 0,
        "amount_out should be positive: {}",
        tlv.amount_out
    );
    assert_ne!(
        tlv.pool_address, [0u8; 20],
        "Pool address should not be zero"
    );
    assert!(
        tlv.tick_after >= -887272 && tlv.tick_after <= 887272,
        "Tick should be within V3 bounds: {}",
        tlv.tick_after
    );

    println!("✅ Four-step validation passed:");
    println!(
        "   Amount in:  {} (decimals: {})",
        tlv.amount_in, tlv.amount_in_decimals
    );
    println!(
        "   Amount out: {} (decimals: {})",
        tlv.amount_out, tlv.amount_out_decimals
    );
    println!("   Tick after: {}", tlv.tick_after);
    println!("   Pool:       0x{}", hex::encode(tlv.pool_address));
}

#[test]
fn test_four_step_validation_detects_corruption() {
    // Use corrupted data that should fail validation
    let invalid_log = polygon::invalid_swap_corrupted_data();
    let decoder = ValidatedPolygonDecoder::new();

    // This should fail at some validation step
    let result = decoder.decode_and_validate(&invalid_log, DEXProtocol::UniswapV3);

    assert!(
        result.is_err(),
        "Four-step validation should detect corrupted data"
    );

    let error = result.unwrap_err();
    println!("✅ Validation correctly detected corruption: {}", error);

    // Error should indicate validation failure
    let error_msg = error.to_string();
    assert!(
        error_msg.contains("Validation failed") || error_msg.contains("validation"),
        "Error should mention validation: {}",
        error_msg
    );
}

#[test]
fn test_individual_validation_steps() {
    // Test each step of the validation pipeline individually
    let log = polygon::uniswap_v3_swap_real();
    // SwapEventDecoder is a unit struct, use static methods directly

    // Step 1: Extract semantic data with ABI decoder
    let validated_data = SwapEventDecoder::decode_swap_event(&log, DEXProtocol::UniswapV3)
        .expect("ABI decoding should succeed with real data");

    let raw_event = PolygonRawSwapEvent {
        log: log.clone(),
        validated_data,
    };

    // Test Step 1: Raw data validation
    println!("Testing Step 1: Raw data validation...");
    assert!(
        raw_event.validate_required_fields().is_ok(),
        "Step 1a: Required fields validation failed"
    );
    assert!(
        raw_event.validate_types_against_spec().is_ok(),
        "Step 1b: Type spec validation failed"
    );
    assert!(
        raw_event.validate_field_ranges().is_ok(),
        "Step 1c: Field ranges validation failed"
    );
    assert!(
        raw_event.validate_precision_preserved().is_ok(),
        "Step 1d: Precision validation failed"
    );
    println!("✅ Step 1 passed");

    // Convert to TLV
    let original_tlv = PoolSwapTLV::from(raw_event);

    // Test Step 2: TLV serialization validation
    println!("Testing Step 2: TLV serialization validation...");
    let serialization_result = validate_tlv_serialization(&original_tlv);
    assert!(
        serialization_result.is_ok(),
        "Step 2: TLV serialization validation failed"
    );
    let bytes = serialization_result.unwrap();
    assert!(
        !bytes.is_empty(),
        "Step 2: Serialized bytes should not be empty"
    );
    println!("✅ Step 2 passed, {} bytes serialized", bytes.len());

    // Test Step 3: TLV deserialization validation
    println!("Testing Step 3: TLV deserialization validation...");
    let deserialization_result: ValidationResult<PoolSwapTLV> =
        validate_tlv_deserialization(&bytes);
    assert!(
        deserialization_result.is_ok(),
        "Step 3: TLV deserialization validation failed"
    );
    let recovered_tlv = deserialization_result.unwrap();
    println!("✅ Step 3 passed");

    // Test Step 4: Deep equality validation
    println!("Testing Step 4: Deep equality validation...");
    let equality_result = validate_equality(&original_tlv, &recovered_tlv);
    assert!(
        equality_result.is_ok(),
        "Step 4: Deep equality validation failed: {:?}",
        equality_result
    );
    println!("✅ Step 4 passed");

    println!("✅ All four validation steps passed individually");
}

#[test]
fn test_validation_performance_benchmark() {
    // Test validation performance with multiple real events
    let logs = vec![
        polygon::uniswap_v3_swap_real(),
        polygon::quickswap_v2_swap_real(),
        polygon::uniswap_v3_swap_real(),   // Duplicate for volume
        polygon::quickswap_v2_swap_real(), // Duplicate for volume
        polygon::uniswap_v3_swap_real(),
    ];

    let decoder = ValidatedPolygonDecoder::new();
    let start = std::time::Instant::now();

    // Test V3 events
    let v3_results = logs
        .iter()
        .filter(|_| true) // Use all for now
        .take(3)
        .map(|log| decoder.decode_and_validate(log, DEXProtocol::UniswapV3))
        .collect::<Vec<_>>();

    // Test V2 events
    let v2_results = logs
        .iter()
        .skip(1)
        .take(2)
        .map(|log| decoder.decode_and_validate(log, DEXProtocol::UniswapV2))
        .collect::<Vec<_>>();

    let duration = start.elapsed();
    let total_events = v3_results.len() + v2_results.len();

    // Count successful validations
    let v3_success = v3_results.iter().filter(|r| r.is_ok()).count();
    let v2_success = v2_results.iter().filter(|r| r.is_ok()).count();
    let total_success = v3_success + v2_success;

    println!("✅ Validation Performance Benchmark:");
    println!("   Total events:     {}", total_events);
    println!("   Successful:       {}", total_success);
    println!(
        "   V3 events:        {} ({} successful)",
        v3_results.len(),
        v3_success
    );
    println!(
        "   V2 events:        {} ({} successful)",
        v2_results.len(),
        v2_success
    );
    println!("   Total time:       {:?}", duration);
    println!("   Average per event: {:?}", duration / total_events as u32);

    // Performance requirements from config
    let avg_per_event = duration / total_events as u32;
    assert!(
        avg_per_event.as_millis() < config::MAX_VALIDATION_TIME_MS as u128,
        "Validation should be fast (<{}ms per event), got {:?}",
        config::MAX_VALIDATION_TIME_MS,
        avg_per_event
    );

    // Most should succeed with real data
    assert!(
        total_success >= total_events / 2,
        "At least half of validations should succeed with real data"
    );
}
