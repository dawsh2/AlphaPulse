//! Zero-copy TLV validation tests
//!
//! Validates that our TLV structures meet the size requirements for zero-copy serialization

use alphapulse_types::protocol::tlv::address::{AddressConversion, AddressExtraction};
use alphapulse_types::protocol::tlv::*;
use zerocopy::{AsBytes, FromBytes};

#[test]
fn verify_tlv_sizes() {
    use std::mem::size_of;

    // Runtime validation of sizes
    assert_eq!(size_of::<PoolSwapTLV>(), 208);
    assert_eq!(size_of::<PoolSyncTLV>(), 160);
    assert_eq!(size_of::<PoolMintTLV>(), 208);
    assert_eq!(size_of::<PoolBurnTLV>(), 208);
    assert_eq!(size_of::<PoolTickTLV>(), 64);
    assert_eq!(size_of::<PoolStateTLV>(), 192); // 189 + 3 padding
    assert_eq!(size_of::<QuoteTLV>(), 56); // 52 + 4 padding
    assert_eq!(size_of::<TradeTLV>(), 40); // 37 + 3 padding

    println!("All TLV sizes verified for zero-copy serialization!");
}

#[test]
fn verify_padding_is_zero() {
    use alphapulse_types::protocol::VenueId;

    let swap = PoolSwapTLV::new(
        [0x42u8; 20], // pool
        [0x43u8; 20], // token_in
        [0x44u8; 20], // token_out
        VenueId::Polygon,
        1000u128,      // amount_in
        900u128,       // amount_out
        5000u128,      // liquidity_after
        1234567890u64, // timestamp_ns
        12345u64,      // block_number
        100i32,        // tick_after
        18u8,          // amount_in_decimals
        6u8,           // amount_out_decimals
        12345u128,     // sqrt_price_x96_after
    );

    assert_eq!(swap._padding, [0u8; 8]);

    let sync = PoolSyncTLV::new(
        [0x42u8; 20], // pool
        [0x43u8; 20], // token0
        [0x44u8; 20], // token1
        VenueId::Polygon,
        1000u128,      // reserve0
        900u128,       // reserve1
        18u8,          // token0_decimals
        6u8,           // token1_decimals
        1234567890u64, // timestamp_ns
        12345u64,      // block_number
    );

    assert_eq!(sync._padding, [0u8; 12]);

    println!("All padding fields correctly initialized to zeros!");
}

#[test]
fn verify_zero_copy_works() {
    use alphapulse_types::protocol::VenueId;

    let sync = PoolSyncTLV::new(
        [0x42u8; 20], // pool
        [0x43u8; 20], // token0
        [0x44u8; 20], // token1
        VenueId::Polygon,
        1000u128,      // reserve0
        900u128,       // reserve1
        18u8,          // token0_decimals
        6u8,           // token1_decimals
        1234567890u64, // timestamp_ns
        12345u64,      // block_number
    );

    // Zero-copy serialization should work
    let _bytes: &[u8] = sync.as_bytes();

    // Zero-copy deserialization should work too
    let _sync_ref = PoolSyncTLV::ref_from(_bytes).expect("Zero-copy deserialization failed");

    println!("Zero-copy serialization/deserialization works!");
}

#[test]
fn verify_address_extraction() {
    use alphapulse_types::protocol::tlv::address::AddressExtraction;
    use alphapulse_types::protocol::VenueId;

    let original_pool = [0x42u8; 20];
    let sync = PoolSyncTLV::new(
        original_pool, // pool
        [0x43u8; 20],  // token0
        [0x44u8; 20],  // token1
        VenueId::Polygon,
        1000u128,      // reserve0
        900u128,       // reserve1
        18u8,          // token0_decimals
        6u8,           // token1_decimals
        1234567890u64, // timestamp_ns
        12345u64,      // block_number
    );

    // Should be able to extract the original address
    let extracted_pool = sync.pool_address.to_eth_address();
    assert_eq!(extracted_pool, original_pool);

    // Padding should be zeros
    assert!(sync.pool_address.validate_padding());

    println!("Address extraction working correctly!");
}

#[test]
fn test_invalid_padding_detection() {
    use alphapulse_types::protocol::tlv::address::AddressExtraction;

    let mut invalid_padded = [0u8; 32];
    invalid_padded[..20].copy_from_slice(&[0x42u8; 20]);
    invalid_padded[25] = 1; // Corrupt padding

    // Should detect corruption
    assert!(!invalid_padded.validate_padding());

    // Valid padding should pass
    let valid_padded = [0x42u8; 20].to_padded();
    assert!(valid_padded.validate_padding());

    println!("Invalid padding detection working correctly!");
}

#[test]
fn test_all_tlv_struct_zero_copy() {
    use alphapulse_types::protocol::{InstrumentId, VenueId};

    // Test TradeTLV
    let instrument_id = InstrumentId {
        venue: VenueId::Polygon as u16,
        asset_type: 1,
        reserved: 0,
        asset_id: 12345,
    };

    let trade = TradeTLV::from_instrument(
        VenueId::Polygon,
        instrument_id,
        100000000i64,   // $1.00 with 8 decimal places
        50000000000i64, // 500 tokens with 8 decimal places
        0u8,            // buy
        1234567890u64,
    );

    // Zero-copy serialization
    let trade_bytes: &[u8] = trade.as_bytes();
    assert_eq!(trade_bytes.len(), 40);

    // Zero-copy deserialization
    let trade_ref = TradeTLV::ref_from(trade_bytes).expect("TradeTLV deserialization failed");
    assert_eq!(*trade_ref, trade);

    // Test QuoteTLV
    let quote = QuoteTLV::from_instrument(
        VenueId::Polygon,
        instrument_id,
        99900000i64,  // $0.999 bid
        1000000i64,   // 10 tokens bid size
        100100000i64, // $1.001 ask
        2000000i64,   // 20 tokens ask size
        1234567890u64,
    );

    let quote_bytes: &[u8] = quote.as_bytes();
    assert_eq!(quote_bytes.len(), 56);

    let quote_ref = QuoteTLV::ref_from(quote_bytes).expect("QuoteTLV deserialization failed");
    assert_eq!(*quote_ref, quote);

    println!("All TLV structs support zero-copy operations!");
}

#[test]
fn test_performance_characteristics() {
    use alphapulse_types::protocol::VenueId;
    use std::time::Instant;

    // Test zero-copy performance characteristics
    let sync = PoolSyncTLV::new(
        [0x42u8; 20], // pool
        [0x43u8; 20], // token0
        [0x44u8; 20], // token1
        VenueId::Polygon,
        1000u128,      // reserve0
        900u128,       // reserve1
        18u8,          // token0_decimals
        6u8,           // token1_decimals
        1234567890u64, // timestamp_ns
        12345u64,      // block_number
    );

    // Measure zero-copy serialization
    let iterations = 10000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _bytes: &[u8] = sync.as_bytes();
        // Prevent optimization from eliminating the operation
        std::hint::black_box(_bytes);
    }

    let zero_copy_duration = start.elapsed();
    println!(
        "Zero-copy serialization: {} operations in {:?} ({:.2} ns/op)",
        iterations,
        zero_copy_duration,
        zero_copy_duration.as_nanos() as f64 / iterations as f64
    );

    // Measure zero-copy deserialization
    let bytes = sync.as_bytes();
    let start = Instant::now();

    for _ in 0..iterations {
        let _tlv_ref = PoolSyncTLV::ref_from(bytes).expect("Deserialization failed");
        std::hint::black_box(_tlv_ref);
    }

    let deser_duration = start.elapsed();
    println!(
        "Zero-copy deserialization: {} operations in {:?} ({:.2} ns/op)",
        iterations,
        deser_duration,
        deser_duration.as_nanos() as f64 / iterations as f64
    );

    // Verify performance is sub-microsecond per operation
    assert!(
        zero_copy_duration.as_nanos() / (iterations as u128) < 1000,
        "Zero-copy serialization should be < 1µs per operation"
    );
    assert!(
        deser_duration.as_nanos() / (iterations as u128) < 1000,
        "Zero-copy deserialization should be < 1µs per operation"
    );

    println!("Performance characteristics validated!");
}

#[test]
fn test_struct_alignment_invariants() {
    use std::mem::{align_of, size_of};

    // Verify all structs have proper alignment
    assert_eq!(
        align_of::<PoolSwapTLV>(),
        16,
        "PoolSwapTLV should be 16-byte aligned"
    );
    assert_eq!(
        size_of::<PoolSwapTLV>(),
        208,
        "PoolSwapTLV should be exactly 208 bytes"
    );
    assert_eq!(
        size_of::<PoolSwapTLV>() % 16,
        0,
        "PoolSwapTLV size should be multiple of 16"
    );

    assert_eq!(
        align_of::<PoolSyncTLV>(),
        16,
        "PoolSyncTLV should be 16-byte aligned"
    );
    assert_eq!(
        size_of::<PoolSyncTLV>(),
        160,
        "PoolSyncTLV should be exactly 160 bytes"
    );
    assert_eq!(
        size_of::<PoolSyncTLV>() % 16,
        0,
        "PoolSyncTLV size should be multiple of 16"
    );

    assert_eq!(
        align_of::<PoolStateTLV>(),
        16,
        "PoolStateTLV should be 16-byte aligned"
    );
    assert_eq!(
        size_of::<PoolStateTLV>(),
        192,
        "PoolStateTLV should be exactly 192 bytes"
    );
    assert_eq!(
        size_of::<PoolStateTLV>() % 16,
        0,
        "PoolStateTLV size should be multiple of 16"
    );

    assert_eq!(
        align_of::<QuoteTLV>(),
        8,
        "QuoteTLV should be 8-byte aligned"
    );
    assert_eq!(
        size_of::<QuoteTLV>(),
        56,
        "QuoteTLV should be exactly 56 bytes"
    );
    assert_eq!(
        size_of::<QuoteTLV>() % 8,
        0,
        "QuoteTLV size should be multiple of 8"
    );

    assert_eq!(
        align_of::<TradeTLV>(),
        8,
        "TradeTLV should be 8-byte aligned"
    );
    assert_eq!(
        size_of::<TradeTLV>(),
        40,
        "TradeTLV should be exactly 40 bytes"
    );
    assert_eq!(
        size_of::<TradeTLV>() % 8,
        0,
        "TradeTLV size should be multiple of 8"
    );

    println!("All struct alignment invariants validated!");
}

#[test]
fn test_address_conversion_edge_cases() {
    use alphapulse_types::protocol::tlv::address::{AddressConversion, AddressExtraction};

    // Test zero address
    let zero_addr = [0u8; 20];
    let zero_padded = zero_addr.to_padded();
    assert_eq!(zero_padded, [0u8; 32]);
    assert_eq!(zero_padded.to_eth_address(), zero_addr);
    assert!(zero_padded.validate_padding());

    // Test maximum address
    let max_addr = [0xFFu8; 20];
    let max_padded = max_addr.to_padded();
    let expected_max_padded = {
        let mut padded = [0u8; 32];
        padded[..20].copy_from_slice(&max_addr);
        padded
    };
    assert_eq!(max_padded, expected_max_padded);
    assert_eq!(max_padded.to_eth_address(), max_addr);
    assert!(max_padded.validate_padding());

    // Test PaddedAddress wrapper
    let addr = [
        0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99, 0xAA, 0xBB, 0xCC,
    ];

    let padded_wrapper = PaddedAddress::from_eth(addr);
    assert!(padded_wrapper.is_valid());
    assert_eq!(padded_wrapper.as_eth(), addr);

    let raw_bytes: [u8; 32] = padded_wrapper.into();
    assert_eq!(&raw_bytes[..20], &addr[..]);
    assert_eq!(&raw_bytes[20..], &[0u8; 12]);

    println!("Address conversion edge cases validated!");
}
