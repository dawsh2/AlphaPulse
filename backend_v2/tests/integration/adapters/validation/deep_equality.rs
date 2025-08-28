//! Deep Equality Validation Tests
//!
//! Tests that deep equality validation detects any data corruption during
//! serialization/deserialization cycles (byte-for-byte identical validation).

use crate::fixtures::polygon;
use crate::*;
use torq_adapter_service::validate_equality;

#[test]
fn test_deep_equality_identical_tlvs() {
    // Test that identical TLVs pass deep equality validation
    let tlv1 = create_test_swap_tlv();
    let tlv2 = tlv1.clone();

    // Should pass deep equality
    assert!(
        validate_equality(&tlv1, &tlv2).is_ok(),
        "Identical TLVs should pass deep equality"
    );

    println!("✅ Deep equality passed for identical TLVs");
}

#[test]
fn test_deep_equality_detects_subtle_corruption() {
    // Test that deep equality detects even subtle differences
    let tlv1 = create_test_swap_tlv();
    let mut tlv2 = tlv1.clone();

    // Introduce subtle corruption
    tlv2.amount_in += 1; // Change by just 1 wei

    // Should detect the difference
    assert!(
        validate_equality(&tlv1, &tlv2).is_err(),
        "Should detect subtle amount corruption"
    );

    // Test other field corruptions
    tlv2 = tlv1.clone();
    tlv2.tick_after += 1;
    assert!(
        validate_equality(&tlv1, &tlv2).is_err(),
        "Should detect tick corruption"
    );

    tlv2 = tlv1.clone();
    tlv2.sqrt_price_x96_after[19] = tlv2.sqrt_price_x96_after[19].wrapping_add(1); // Modify last byte
    assert!(
        validate_equality(&tlv1, &tlv2).is_err(),
        "Should detect sqrt_price corruption"
    );

    tlv2 = tlv1.clone();
    tlv2.pool_address[0] = tlv2.pool_address[0].wrapping_add(1);
    assert!(
        validate_equality(&tlv1, &tlv2).is_err(),
        "Should detect pool_address corruption"
    );

    println!("✅ Deep equality correctly detected all subtle corruptions");
}

#[test]
fn test_serialization_roundtrip_deep_equality() {
    // Test that serialization + deserialization preserves deep equality
    let original = create_test_swap_tlv();

    // Serialize to bytes
    let bytes = original.to_bytes();
    assert!(!bytes.is_empty(), "Serialization should produce bytes");

    // Deserialize back
    let recovered = PoolSwapTLV::from_bytes(&bytes).expect("Deserialization should succeed");

    // Should be deeply equal
    assert!(
        validate_equality(&original, &recovered).is_ok(),
        "Serialization roundtrip should preserve deep equality"
    );

    // Re-serialize to verify byte-for-byte identical
    let original_bytes = original.to_bytes();
    let recovered_bytes = recovered.to_bytes();
    assert_eq!(
        original_bytes, recovered_bytes,
        "Re-serialization should produce identical bytes"
    );

    println!("✅ Serialization roundtrip preserved deep equality");
    println!("   Serialized size: {} bytes", bytes.len());
}

#[test]
fn test_multiple_roundtrip_deep_equality() {
    // Test deep equality through multiple serialization cycles
    let mut current = create_test_swap_tlv();
    let original = current.clone();

    // Perform multiple roundtrips
    for i in 1..=5 {
        // Serialize
        let bytes = current.to_bytes();

        // Deserialize
        let recovered = PoolSwapTLV::from_bytes(&bytes)
            .expect(&format!("Roundtrip {} deserialization should succeed", i));

        // Should be deeply equal to current
        assert!(
            validate_equality(&current, &recovered).is_ok(),
            "Roundtrip {} should preserve deep equality",
            i
        );

        // Should also be deeply equal to original
        assert!(
            validate_equality(&original, &recovered).is_ok(),
            "Roundtrip {} should preserve deep equality with original",
            i
        );

        current = recovered;
    }

    println!("✅ Multiple roundtrips preserved deep equality");
    println!("   Completed 5 serialization/deserialization cycles");
}

#[test]
fn test_deep_equality_with_real_polygon_data() {
    // Test deep equality with actual Polygon transaction data
    use torq_adapter_service::input::collectors::polygon_dex::{
        abi_events::{DEXProtocol, SwapEventDecoder},
        validated_decoder::PolygonRawSwapEvent,
    };

    let log = polygon::uniswap_v3_swap_real();
    // SwapEventDecoder used with static methods

    // Parse with ABI decoder
    let validated_data = SwapEventDecoder::decode_swap_event(&log, DEXProtocol::UniswapV3)
        .expect("ABI decoding should succeed");

    let raw_event = PolygonRawSwapEvent {
        log,
        validated_data,
    };

    // Convert to TLV
    let original = PoolSwapTLV::from(raw_event);

    // Test serialization roundtrip
    let bytes = original.to_bytes();
    let recovered =
        PoolSwapTLV::from_bytes(&bytes).expect("Real data deserialization should succeed");

    // Should pass deep equality
    assert!(
        validate_equality(&original, &recovered).is_ok(),
        "Real Polygon data should preserve deep equality through roundtrip"
    );

    // Verify specific fields are preserved
    assert_eq!(original.venue, recovered.venue, "Venue should be preserved");
    assert_eq!(
        original.pool_address, recovered.pool_address,
        "Pool address should be preserved"
    );
    assert_eq!(
        original.amount_in, recovered.amount_in,
        "Amount in should be preserved"
    );
    assert_eq!(
        original.amount_out, recovered.amount_out,
        "Amount out should be preserved"
    );
    assert_eq!(
        original.tick_after, recovered.tick_after,
        "Tick should be preserved"
    );
    assert_eq!(
        original.sqrt_price_x96_after, recovered.sqrt_price_x96_after,
        "sqrt_price should be preserved"
    );

    println!("✅ Deep equality validated with real Polygon data");
    println!("   Amount in preserved:  {}", recovered.amount_in);
    println!("   Amount out preserved: {}", recovered.amount_out);
    println!("   Tick preserved:       {}", recovered.tick_after);
}

#[test]
fn test_deep_equality_hash_consistency() {
    // Test that hash-based equality checking is consistent
    let tlv = create_test_swap_tlv();

    // Hash should be consistent across calls
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher1 = DefaultHasher::new();
    tlv.hash(&mut hasher1);
    let hash1 = hasher1.finish();

    let mut hasher2 = DefaultHasher::new();
    tlv.hash(&mut hasher2);
    let hash2 = hasher2.finish();

    assert_eq!(hash1, hash2, "Hash should be consistent");

    // Hash should change with data changes
    let mut modified = tlv.clone();
    modified.amount_in += 1;

    let mut hasher3 = DefaultHasher::new();
    modified.hash(&mut hasher3);
    let hash3 = hasher3.finish();

    assert_ne!(hash1, hash3, "Hash should change with data modification");

    println!("✅ Hash-based equality checking is consistent");
}

/// Helper function to create a test PoolSwapTLV
fn create_test_swap_tlv() -> PoolSwapTLV {
    PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_address: [
            0x45, 0xdd, 0xa9, 0xcb, 0x7c, 0x25, 0x13, 0x1d, 0xf2, 0x68, 0x51, 0x51, 0x31, 0xf6,
            0x47, 0xd7, 0x26, 0xf5, 0x06, 0x08,
        ],
        token_in_addr: [
            0xc0, 0x2a, 0xaa, 0x39, 0xb2, 0x23, 0xfe, 0x8d, 0x0a, 0x0e, 0x5c, 0x4f, 0x27, 0xea,
            0xd9, 0x08, 0x3c, 0x75, 0x6c, 0xc2,
        ],
        token_out_addr: [
            0x27, 0x91, 0xbc, 0xa1, 0xf2, 0xde, 0x4a, 0xef, 0x14, 0x41, 0x35, 0x8c, 0x71, 0x9c,
            0xf4, 0x3a, 0x0e, 0x97, 0xa9, 0xc5,
        ],
        amount_in: 10000000000000000000, // 10 WETH
        amount_out: 27000000000,         // 27,000 USDC
        amount_in_decimals: 18,
        amount_out_decimals: 6,
        tick_after: 3393,
        sqrt_price_x96_after: PoolSwapTLV::sqrt_price_from_u128(1792282187229267636352),
        liquidity_after: 1000000000000000000,
        timestamp_ns: 1640995200123456789,
        block_number: 48_500_000,
    }
}
