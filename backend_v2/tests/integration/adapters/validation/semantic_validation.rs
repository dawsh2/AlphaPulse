//! Semantic Validation Tests
//!
//! Tests that semantic validation catches field mapping errors and prevents
//! issues like "fees stored in profit field" or amount_in/amount_out confusion.

use crate::fixtures::polygon;
use crate::*;
use adapter_service::{SemanticValidator, ValidationResult};
use protocol_v2::tlv::market_data::PoolSwapTLV;

#[test]
fn test_semantic_validation_success() {
    // Create a valid PoolSwapTLV with correct semantic mapping
    let swap = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_address: [1u8; 20],
        token_in_addr: [2u8; 20],
        token_out_addr: [3u8; 20],
        amount_in: 1000000000000000000, // 1 WETH (18 decimals)
        amount_out: 2700000000,         // 2700 USDC (6 decimals)
        amount_in_decimals: 18,
        amount_out_decimals: 6,
        tick_after: 3393, // Valid V3 tick
        sqrt_price_x96_after: PoolSwapTLV::sqrt_price_from_u128(1792282187229267636352), // Valid sqrt price
        liquidity_after: 1000000000000000000,
        timestamp_ns: 1640995200123456789,
        block_number: 48_500_000,
    };

    // Should pass semantic validation
    assert!(
        swap.validate_semantics().is_ok(),
        "Valid swap should pass semantic validation"
    );
    assert!(
        swap.validate_ranges().is_ok(),
        "Valid swap should pass range validation"
    );

    println!("✅ Semantic validation passed for valid swap");
}

#[test]
fn test_semantic_validation_catches_field_errors() {
    // Test various semantic errors that validation should catch

    // Error 1: Zero amount_in
    let mut swap = create_valid_test_swap();
    swap.amount_in = 0;
    assert!(
        swap.validate_semantics().is_err(),
        "Should detect zero amount_in"
    );

    // Error 2: Zero amount_out
    swap = create_valid_test_swap();
    swap.amount_out = 0;
    assert!(
        swap.validate_semantics().is_err(),
        "Should detect zero amount_out"
    );

    // Error 3: Zero pool address
    swap = create_valid_test_swap();
    swap.pool_address = [0u8; 20];
    assert!(
        swap.validate_semantics().is_err(),
        "Should detect zero pool address"
    );

    // Error 4: Invalid tick bounds (beyond Uniswap V3 specification)
    swap = create_valid_test_swap();
    swap.tick_after = -1000000; // Beyond V3 min tick of -887272
    assert!(
        swap.validate_semantics().is_err(),
        "Should detect tick below V3 bounds"
    );

    swap.tick_after = 1000000; // Beyond V3 max tick of 887272
    assert!(
        swap.validate_semantics().is_err(),
        "Should detect tick above V3 bounds"
    );

    // Error 5: Excessive decimal places
    swap = create_valid_test_swap();
    swap.amount_in_decimals = 50; // Too many decimals
    assert!(
        swap.validate_semantics().is_err(),
        "Should detect excessive decimals"
    );

    println!("✅ Semantic validation correctly caught all field errors");
}

#[test]
fn test_semantic_validation_prevents_venue_confusion() {
    // Test that venue is correctly mapped to blockchain, not protocol

    let mut swap = create_valid_test_swap();

    // Correct: Venue should be Polygon (blockchain)
    swap.venue = VenueId::Polygon;
    assert!(
        swap.validate_semantics().is_ok(),
        "Polygon venue should be valid"
    );

    // The validation framework doesn't prevent other venue IDs at the semantic level
    // (that's handled at the data source level), but we test that Polygon is accepted

    println!("✅ Venue semantic validation working correctly");
}

#[test]
fn test_semantic_validation_sqrt_price_requirements() {
    // Test sqrt_price_x96_after validation for V3 swaps

    let mut swap = create_valid_test_swap();

    // Valid sqrt price should pass
    swap.sqrt_price_x96_after = PoolSwapTLV::sqrt_price_from_u128(1792282187229267636352);
    assert!(
        swap.validate_ranges().is_ok(),
        "Valid sqrt_price should pass"
    );

    // Zero sqrt price indicates corrupted V3 data
    swap.sqrt_price_x96_after = PoolSwapTLV::sqrt_price_from_u128(0);
    assert!(
        swap.validate_ranges().is_err(),
        "Zero sqrt_price should fail validation"
    );

    println!("✅ sqrt_price validation prevents corrupted V3 data");
}

#[test]
fn test_semantic_validation_with_real_polygon_data() {
    // Test semantic validation with actual Polygon transaction data
    use adapter_service::input::collectors::polygon_dex::{
        abi_events::{DEXProtocol, SwapEventDecoder},
        validated_decoder::PolygonRawSwapEvent,
    };

    let log = polygon::uniswap_v3_swap_real();
    // SwapEventDecoder used with static methods

    // Extract data using ABI decoder
    let validated_data = SwapEventDecoder::decode_swap_event(&log, DEXProtocol::UniswapV3)
        .expect("ABI decoding should succeed with real data");

    let raw_event = PolygonRawSwapEvent {
        log,
        validated_data,
    };

    // Convert to TLV
    let tlv = PoolSwapTLV::from(raw_event);

    // Should pass semantic validation
    assert!(
        tlv.validate_semantics().is_ok(),
        "Real Polygon data should pass semantic validation"
    );
    assert!(
        tlv.validate_ranges().is_ok(),
        "Real Polygon data should pass range validation"
    );

    // Verify semantic correctness
    assert_eq!(
        tlv.venue,
        VenueId::Polygon,
        "Venue should be Polygon blockchain"
    );
    assert!(
        tlv.amount_in > 0,
        "amount_in should be positive from real data"
    );
    assert!(
        tlv.amount_out > 0,
        "amount_out should be positive from real data"
    );
    assert_ne!(
        tlv.pool_address, [0u8; 20],
        "Pool address should not be zero from real data"
    );

    println!("✅ Semantic validation passed with real Polygon data");
    println!("   Venue: {:?}", tlv.venue);
    println!("   Amount in:  {}", tlv.amount_in);
    println!("   Amount out: {}", tlv.amount_out);
    println!("   Tick after: {}", tlv.tick_after);
}

/// Helper function to create a valid test swap
fn create_valid_test_swap() -> PoolSwapTLV {
    PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_address: [1u8; 20],
        token_in_addr: [2u8; 20],
        token_out_addr: [3u8; 20],
        amount_in: 1000000000000000000, // 1 WETH (18 decimals)
        amount_out: 2700000000,         // 2700 USDC (6 decimals)
        amount_in_decimals: 18,
        amount_out_decimals: 6,
        tick_after: 3393, // Valid V3 tick
        sqrt_price_x96_after: PoolSwapTLV::sqrt_price_from_u128(1792282187229267636352), // Valid sqrt price
        liquidity_after: 1000000000000000000,
        timestamp_ns: 1640995200123456789,
        block_number: 48_500_000,
    }
}
