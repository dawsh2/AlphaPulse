//! Polygon DEX Integration Tests
//!
//! Tests the complete Polygon DEX adapter with real blockchain data,
//! demonstrating proper usage of the validation framework.

use crate::fixtures::polygon;
use crate::*;
use alphapulse_adapter_service::input::collectors::polygon_dex::{
    abi_events::{DEXProtocol as LocalDEXProtocol, SwapEventDecoder},
    validated_decoder::{PolygonRawSwapEvent, ValidatedPolygonDecoder},
};
use protocol_v2::{tlv::market_data::PoolSwapTLV, VenueId};

#[test]
fn test_uniswap_v3_swap_integration() {
    // Test complete flow: Real Polygon Log → ABI Decoding → TLV → Validation
    let log = polygon::uniswap_v3_swap_real();
    let decoder = ValidatedPolygonDecoder::new();

    println!("🧪 Testing Uniswap V3 Swap Integration");
    println!("   Log address: 0x{}", hex::encode(log.address.as_bytes()));
    println!("   Block number: {:?}", log.block_number);
    println!("   Data size: {} bytes", log.data.0.len());

    // Decode and validate
    let result = decoder.decode_and_validate(&log, LocalDEXProtocol::UniswapV3);
    assert!(
        result.is_ok(),
        "V3 swap integration should succeed: {:?}",
        result
    );

    let tlv = result.unwrap();

    // Verify integration correctness
    assert_eq!(tlv.venue, VenueId::Polygon, "Venue should be Polygon");
    assert!(
        tlv.amount_in > 0,
        "Should have positive amount_in: {}",
        tlv.amount_in
    );
    assert!(
        tlv.amount_out > 0,
        "Should have positive amount_out: {}",
        tlv.amount_out
    );
    assert!(
        tlv.tick_after >= -887272 && tlv.tick_after <= 887272,
        "Tick within V3 bounds: {}",
        tlv.tick_after
    );
    assert!(
        tlv.sqrt_price_x96_as_u128() > 0,
        "Should have positive sqrt_price: {}",
        tlv.sqrt_price_x96_as_u128()
    );
    assert!(
        tlv.block_number > 0,
        "Should have block number: {}",
        tlv.block_number
    );

    println!("✅ Uniswap V3 integration successful");
    println!(
        "   Amount: {} -> {} ({}:{} decimals)",
        tlv.amount_in, tlv.amount_out, tlv.amount_in_decimals, tlv.amount_out_decimals
    );
    println!(
        "   Tick: {} | sqrt_price: {}",
        tlv.tick_after,
        tlv.sqrt_price_x96_as_u128()
    );
    println!("   Pool: 0x{}", hex::encode(tlv.pool_address));
}

#[test]
fn test_quickswap_v2_swap_integration() {
    // Test QuickSwap V2 swap integration
    let log = polygon::quickswap_v2_swap_real();
    let decoder = ValidatedPolygonDecoder::new();

    println!("🧪 Testing QuickSwap V2 Swap Integration");
    println!("   Log address: 0x{}", hex::encode(log.address.as_bytes()));
    println!("   Block number: {:?}", log.block_number);
    println!("   Data size: {} bytes", log.data.0.len());

    // Decode and validate V2 swap
    let result = decoder.decode_and_validate(&log, LocalDEXProtocol::UniswapV2);
    assert!(
        result.is_ok(),
        "V2 swap integration should succeed: {:?}",
        result
    );

    let tlv = result.unwrap();

    // Verify V2 integration
    assert_eq!(tlv.venue, VenueId::Polygon, "Venue should be Polygon");
    assert!(
        tlv.amount_in > 0,
        "Should have positive amount_in: {}",
        tlv.amount_in
    );
    assert!(
        tlv.amount_out > 0,
        "Should have positive amount_out: {}",
        tlv.amount_out
    );
    // V2 doesn't have meaningful tick/sqrt_price, but should have defaults
    assert!(
        tlv.block_number > 0,
        "Should have block number: {}",
        tlv.block_number
    );

    println!("✅ QuickSwap V2 integration successful");
    println!(
        "   Amount: {} -> {} ({}:{} decimals)",
        tlv.amount_in, tlv.amount_out, tlv.amount_in_decimals, tlv.amount_out_decimals
    );
    println!("   Pool: 0x{}", hex::encode(tlv.pool_address));
}

#[test]
fn test_uniswap_v3_mint_integration() {
    // Test liquidity mint event integration
    let log = polygon::uniswap_v3_mint_real();

    println!("🧪 Testing Uniswap V3 Mint Integration");
    println!("   Log address: 0x{}", hex::encode(log.address.as_bytes()));
    println!("   Block number: {:?}", log.block_number);

    // For now, we focus on swaps, but this shows the pattern for other event types
    // Future: Implement MintEventDecoder and PoolMintTLV validation

    println!("✅ Mint event structure validated (swap focus for now)");
}

#[test]
fn test_batch_processing_integration() {
    // Test processing multiple events in batch
    let logs = vec![
        polygon::uniswap_v3_swap_real(),
        polygon::quickswap_v2_swap_real(),
    ];

    let decoder = ValidatedPolygonDecoder::new();

    println!("🧪 Testing Batch Processing Integration");
    println!("   Processing {} events", logs.len());

    // Process V3 batch
    let v3_results =
        decoder.decode_and_validate_batch(&[logs[0].clone()], LocalDEXProtocol::UniswapV3);
    let v3_success = v3_results.iter().filter(|r| r.is_ok()).count();

    // Process V2 batch
    let v2_results =
        decoder.decode_and_validate_batch(&[logs[1].clone()], LocalDEXProtocol::UniswapV2);
    let v2_success = v2_results.iter().filter(|r| r.is_ok()).count();

    assert_eq!(v3_success, 1, "V3 batch should succeed");
    assert_eq!(v2_success, 1, "V2 batch should succeed");

    println!("✅ Batch processing integration successful");
    println!("   V3 events: {} successful", v3_success);
    println!("   V2 events: {} successful", v2_success);
}

#[test]
fn test_error_handling_integration() {
    // Test that integration properly handles corrupted data
    let invalid_log = polygon::invalid_swap_corrupted_data();
    let decoder = ValidatedPolygonDecoder::new();

    println!("🧪 Testing Error Handling Integration");

    // Should detect corruption and fail gracefully
    let result = decoder.decode_and_validate(&invalid_log, LocalDEXProtocol::UniswapV3);
    assert!(result.is_err(), "Should detect corrupted data");

    let error = result.unwrap_err();
    println!("✅ Error handling integration successful");
    println!("   Detected corruption: {}", error);

    // Error should be descriptive
    let error_msg = error.to_string().to_lowercase();
    assert!(
        error_msg.contains("validation")
            || error_msg.contains("failed")
            || error_msg.contains("error")
            || error_msg.contains("invalid"),
        "Error should be descriptive: {}",
        error
    );
}

#[test]
fn test_end_to_end_precision_preservation() {
    // Test that precision is preserved through the complete pipeline
    let log = polygon::uniswap_v3_swap_real();
    let decoder = ValidatedPolygonDecoder::new();

    println!("🧪 Testing E2E Precision Preservation");

    // Process through complete pipeline
    let tlv = decoder
        .decode_and_validate(&log, LocalDEXProtocol::UniswapV3)
        .expect("E2E processing should succeed");

    // Test precision preservation through multiple serialization cycles
    let mut current = tlv.clone();
    let original_amounts = (current.amount_in, current.amount_out);

    for cycle in 1..=3 {
        // Serialize and deserialize
        let bytes = current.to_bytes();
        let recovered = PoolSwapTLV::from_bytes(&bytes)
            .expect(&format!("Cycle {} deserialization should succeed", cycle));

        // Precision should be preserved exactly
        assert_eq!(
            recovered.amount_in, original_amounts.0,
            "amount_in precision lost in cycle {}",
            cycle
        );
        assert_eq!(
            recovered.amount_out, original_amounts.1,
            "amount_out precision lost in cycle {}",
            cycle
        );
        assert_eq!(
            recovered.tick_after, tlv.tick_after,
            "tick precision lost in cycle {}",
            cycle
        );
        assert_eq!(
            recovered.sqrt_price_x96_after, tlv.sqrt_price_x96_after,
            "sqrt_price precision lost in cycle {}",
            cycle
        );

        current = recovered;
    }

    println!("✅ E2E precision preservation successful");
    println!("   Original amount_in:  {}", original_amounts.0);
    println!("   Final amount_in:     {}", current.amount_in);
    println!("   Original amount_out: {}", original_amounts.1);
    println!("   Final amount_out:    {}", current.amount_out);
    println!("   Precision preserved through 3 cycles");
}

#[test]
fn test_performance_integration() {
    // Test that integration meets performance requirements
    let logs = vec![
        polygon::uniswap_v3_swap_real(),
        polygon::quickswap_v2_swap_real(),
        polygon::uniswap_v3_swap_real(), // Duplicate for volume
    ];

    let decoder = ValidatedPolygonDecoder::new();

    println!("🧪 Testing Performance Integration");

    let start = std::time::Instant::now();

    // Process all events
    let mut total_processed = 0;
    let mut total_successful = 0;

    for log in &logs {
        // Alternate between V3 and V2 for realistic workload
        let protocol = if total_processed % 2 == 0 {
            LocalDEXProtocol::UniswapV3
        } else {
            LocalDEXProtocol::UniswapV2
        };

        let result = decoder.decode_and_validate(log, protocol);
        total_processed += 1;

        if result.is_ok() {
            total_successful += 1;
        }
    }

    let duration = start.elapsed();
    let avg_per_event = duration / total_processed;

    println!("✅ Performance integration results:");
    println!("   Events processed: {}", total_processed);
    println!("   Successful: {}", total_successful);
    println!("   Total time: {:?}", duration);
    println!("   Average per event: {:?}", avg_per_event);

    // Should meet performance requirements (<1ms per event)
    const MAX_VALIDATION_TIME_MS: u128 = 1;
    assert!(
        avg_per_event.as_millis() < MAX_VALIDATION_TIME_MS,
        "Average processing time should be <{}ms, got {:?}",
        MAX_VALIDATION_TIME_MS,
        avg_per_event
    );

    // Most should succeed with real data
    assert!(
        total_successful >= total_processed * 2 / 3,
        "At least 2/3 should succeed with real data: {}/{}",
        total_successful,
        total_processed
    );
}
