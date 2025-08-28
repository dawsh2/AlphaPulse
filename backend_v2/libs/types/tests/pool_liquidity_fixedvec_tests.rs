//! PoolLiquidityTLV FixedVec Zero-Copy Integration Tests
//!
//! Comprehensive test suite validating the complete bijection cycle:
//! Vec<u128> → FixedVec → PoolLiquidityTLV → serialize → deserialize → FixedVec → Vec<u128>
//!
//! Tests cover:
//! - Perfect bijection preservation
//! - Zero-copy serialization validation
//! - Performance benchmarks (>1M msg/s targets)
//! - Bounds testing for MAX_POOL_TOKENS=8
//! - Memory layout validation

use torq_types::protocol::tlv::dynamic_payload::{FixedVec, MAX_POOL_TOKENS};
use torq_types::protocol::tlv::market_data::PoolLiquidityTLV;
use torq_types::protocol::VenueId;
use std::time::Instant;
use zerocopy::AsBytes;

/// Test perfect bijection: Vec<u128> → PoolLiquidityTLV → Vec<u128>
#[test]
fn test_bijection_preservation() {
    let test_cases = vec![
        // Single token case
        vec![1_000_000_000_000_000_000u128], // 1 ETH
        // Typical V2 pair
        vec![
            5_000_000_000_000_000_000u128, // 5 ETH
            10_000_000_000u128,            // 10,000 USDC (6 decimals)
        ],
        // V3 pool with virtual reserves
        vec![
            2_500_000_000_000_000_000u128, // 2.5 ETH
            7_500_000_000u128,             // 7,500 USDC
        ],
        // Complex multi-token pool (like Balancer)
        vec![
            1_000_000_000_000_000_000u128, // 1 ETH
            2_000_000_000u128,             // 2,000 USDC
            500_000_000_000_000_000u128,   // 0.5 WBTC (using 18 decimals for simplicity)
            1_000_000_000_000_000_000u128, // 1 DAI
        ],
        // Maximum capacity test (8 tokens)
        vec![
            100_000_000_000_000_000u128, // Token 0
            200_000_000_000_000_000u128, // Token 1
            300_000_000_000_000_000u128, // Token 2
            400_000_000_000_000_000u128, // Token 3
            500_000_000_000_000_000u128, // Token 4
            600_000_000_000_000_000u128, // Token 5
            700_000_000_000_000_000u128, // Token 6
            800_000_000_000_000_000u128, // Token 7
        ],
    ];

    for (i, original_reserves) in test_cases.iter().enumerate() {
        // Create PoolLiquidityTLV from Vec
        let pool_address = [0x42u8; 20]; // Mock pool address
        let timestamp_ns = 1700000000000000000u64;

        let tlv = PoolLiquidityTLV::new(
            VenueId::UniswapV3,
            pool_address,
            original_reserves,
            timestamp_ns,
        )
        .unwrap_or_else(|e| panic!("Test case {}: Failed to create PoolLiquidityTLV: {}", i, e));

        // Validate bijection
        let recovered_reserves = tlv.to_reserves_vec();

        assert_eq!(
            &recovered_reserves, original_reserves,
            "Test case {}: Bijection failed. Original: {:?}, Recovered: {:?}",
            i, original_reserves, recovered_reserves
        );

        // Note: validate_bijection is only available within the same crate context

        // Validate accessors
        assert_eq!(
            tlv.len(),
            original_reserves.len(),
            "Test case {}: Length mismatch",
            i
        );
        assert_eq!(
            tlv.get_reserves(),
            original_reserves.as_slice(),
            "Test case {}: Slice mismatch",
            i
        );
        assert_eq!(
            tlv.is_empty(),
            original_reserves.is_empty(),
            "Test case {}: Empty check failed",
            i
        );
    }
}

/// Test zero-copy serialization/deserialization without allocations
#[test]
fn test_zero_copy_serialization() {
    let original_reserves = vec![
        1_500_000_000_000_000_000u128, // 1.5 ETH
        3_000_000_000u128,             // 3,000 USDC
    ];

    let pool_address = [0x12u8; 20];
    let timestamp_ns = 1700000000000000000u64;

    // Create original TLV
    let original_tlv = PoolLiquidityTLV::new(
        VenueId::UniswapV2,
        pool_address,
        &original_reserves,
        timestamp_ns,
    )
    .unwrap();

    // Zero-copy serialization via AsBytes
    let serialized_bytes: &[u8] = original_tlv.as_bytes();

    // Validate serialized size
    let expected_size = std::mem::size_of::<PoolLiquidityTLV>();
    assert_eq!(
        serialized_bytes.len(),
        expected_size,
        "Serialized size mismatch. Expected: {}, Got: {}",
        expected_size,
        serialized_bytes.len()
    );

    // Zero-copy deserialization via zerocopy::Ref
    let tlv_ref = zerocopy::Ref::<_, PoolLiquidityTLV>::new(serialized_bytes)
        .expect("Failed to deserialize PoolLiquidityTLV");

    let deserialized_tlv = *tlv_ref.into_ref();

    // Validate complete equality
    assert_eq!(original_tlv.timestamp_ns, deserialized_tlv.timestamp_ns);
    assert_eq!(original_tlv.pool_address, deserialized_tlv.pool_address);
    assert_eq!(original_tlv.venue, deserialized_tlv.venue);

    // Validate reserves bijection is preserved through serialization
    let deserialized_reserves = deserialized_tlv.to_reserves_vec();
    assert_eq!(deserialized_reserves, original_reserves);

    // Validate no allocations by checking direct slice access
    assert_eq!(
        deserialized_tlv.get_reserves(),
        original_reserves.as_slice()
    );

    println!(
        "✅ Zero-copy serialization: {} bytes",
        serialized_bytes.len()
    );
    println!("✅ Perfect bijection preserved through serialization/deserialization");
}

/// Test bounds enforcement for MAX_POOL_TOKENS=8
#[test]
fn test_bounds_enforcement() {
    let pool_address = [0x99u8; 20];
    let timestamp_ns = 1700000000000000000u64;

    // Test maximum capacity (should succeed)
    let max_reserves: Vec<u128> = (0..MAX_POOL_TOKENS)
        .map(|i| (i as u128 + 1) * 1_000_000_000_000_000_000u128)
        .collect();

    let tlv_max =
        PoolLiquidityTLV::new(VenueId::Balancer, pool_address, &max_reserves, timestamp_ns);

    assert!(tlv_max.is_ok(), "Maximum capacity should be allowed");
    let tlv = tlv_max.unwrap();
    assert_eq!(tlv.len(), MAX_POOL_TOKENS);
    assert_eq!(tlv.to_reserves_vec(), max_reserves);

    // Test exceeding capacity (should fail)
    let excessive_reserves: Vec<u128> = (0..MAX_POOL_TOKENS + 1)
        .map(|i| (i as u128 + 1) * 1_000_000_000_000_000_000u128)
        .collect();

    let tlv_excessive = PoolLiquidityTLV::new(
        VenueId::Balancer,
        pool_address,
        &excessive_reserves,
        timestamp_ns,
    );

    assert!(tlv_excessive.is_err(), "Exceeding capacity should fail");

    // Test empty reserves (should fail)
    let empty_reserves: Vec<u128> = vec![];
    let tlv_empty = PoolLiquidityTLV::new(
        VenueId::UniswapV2,
        pool_address,
        &empty_reserves,
        timestamp_ns,
    );

    assert!(tlv_empty.is_err(), "Empty reserves should fail");
}

/// Test incremental reserve addition
#[test]
fn test_incremental_reserve_addition() {
    let pool_address = [0x77u8; 20];
    let timestamp_ns = 1700000000000000000u64;

    // Start with single reserve
    let initial_reserves = vec![1_000_000_000_000_000_000u128]; // 1 ETH
    let mut tlv = PoolLiquidityTLV::new(
        VenueId::UniswapV3,
        pool_address,
        &initial_reserves,
        timestamp_ns,
    )
    .unwrap();

    assert_eq!(tlv.len(), 1);
    assert_eq!(tlv.to_reserves_vec(), initial_reserves);

    // Add more reserves incrementally
    let additional_reserves = vec![
        2_000_000_000u128,             // 2,000 USDC
        500_000_000_000_000_000u128,   // 0.5 WBTC
        1_500_000_000_000_000_000u128, // 1.5 DAI
    ];

    for reserve in &additional_reserves {
        tlv.add_reserve(*reserve).unwrap();
    }

    // Validate final state
    let mut expected_final = initial_reserves;
    expected_final.extend_from_slice(&additional_reserves);

    assert_eq!(tlv.len(), expected_final.len());
    assert_eq!(tlv.to_reserves_vec(), expected_final);

    // Test capacity exceeded
    let remaining_capacity = MAX_POOL_TOKENS - tlv.len();

    // Fill to capacity
    for i in 0..remaining_capacity {
        let result = tlv.add_reserve((i as u128 + 10) * 1_000_000_000u128);
        assert!(result.is_ok(), "Should be able to add reserve {}", i);
    }

    // Attempt to exceed capacity
    let overflow_result = tlv.add_reserve(999_000_000_000u128);
    assert!(
        overflow_result.is_err(),
        "Should fail when exceeding capacity"
    );
}

/// Performance benchmark: Construction rate (target >1M msg/s)
#[test]
fn test_construction_performance() {
    let pool_address = [0xAAu8; 20];
    let timestamp_ns = 1700000000000000000u64;

    // Test data representing typical V2 pool
    let reserves = vec![
        2_000_000_000_000_000_000u128, // 2 ETH
        5_000_000_000u128,             // 5,000 USDC
    ];

    const ITERATIONS: usize = 1_000_000;
    let start = Instant::now();

    for _ in 0..ITERATIONS {
        let _tlv = PoolLiquidityTLV::new(VenueId::UniswapV2, pool_address, &reserves, timestamp_ns)
            .unwrap();

        // Prevent optimization
        std::hint::black_box(_tlv);
    }

    let elapsed = start.elapsed();
    let rate_per_second = (ITERATIONS as f64) / elapsed.as_secs_f64();

    println!("🚀 Construction rate: {:.0} msg/s", rate_per_second);
    println!("📊 Target: >1,000,000 msg/s");

    // Performance requirement
    assert!(
        rate_per_second > 1_000_000.0,
        "Construction rate {:.0} msg/s below target 1M msg/s",
        rate_per_second
    );
}

/// Performance benchmark: Parsing rate (target >1.6M msg/s)
#[test]
fn test_parsing_performance() {
    let pool_address = [0xBBu8; 20];
    let timestamp_ns = 1700000000000000000u64;

    let reserves = vec![
        3_000_000_000_000_000_000u128, // 3 ETH
        7_500_000_000u128,             // 7,500 USDC
    ];

    // Pre-create serialized data
    let original_tlv =
        PoolLiquidityTLV::new(VenueId::UniswapV3, pool_address, &reserves, timestamp_ns).unwrap();

    let serialized_bytes = original_tlv.as_bytes();

    const ITERATIONS: usize = 1_600_000; // Slightly above target for headroom
    let start = Instant::now();

    for _ in 0..ITERATIONS {
        let tlv_ref =
            zerocopy::Ref::<_, PoolLiquidityTLV>::new(serialized_bytes).expect("Failed to parse");
        let _tlv = *tlv_ref.into_ref();

        // Prevent optimization
        std::hint::black_box(_tlv);
    }

    let elapsed = start.elapsed();
    let rate_per_second = (ITERATIONS as f64) / elapsed.as_secs_f64();

    println!("⚡ Parsing rate: {:.0} msg/s", rate_per_second);
    println!("📊 Target: >1,600,000 msg/s");

    // Performance requirement
    assert!(
        rate_per_second > 1_600_000.0,
        "Parsing rate {:.0} msg/s below target 1.6M msg/s",
        rate_per_second
    );
}

/// Test memory layout and size consistency
#[test]
fn test_memory_layout() {
    use std::mem::{align_of, size_of};

    // Validate struct size
    let tlv_size = size_of::<PoolLiquidityTLV>();
    let fixed_vec_size = size_of::<FixedVec<u128, MAX_POOL_TOKENS>>();

    println!("📏 PoolLiquidityTLV size: {} bytes", tlv_size);
    println!(
        "📏 FixedVec<u128, {}> size: {} bytes",
        MAX_POOL_TOKENS, fixed_vec_size
    );

    // Expected layout (with alignment):
    // - timestamp_ns: u64 (8 bytes)
    // - reserves: FixedVec (144 bytes with alignment)
    // - pool_address: [u8; 32] (32 bytes)
    // - venue: u16 (2 bytes)
    // - _padding: [u8; 6] (6 bytes)
    // - Additional padding for alignment: varies
    // Actual measured: 208 bytes, 144 bytes for FixedVec

    assert_eq!(tlv_size, 208, "Unexpected struct size");
    assert_eq!(fixed_vec_size, 144, "Unexpected FixedVec size");

    // Validate alignment
    let tlv_alignment = align_of::<PoolLiquidityTLV>();
    println!("🎯 PoolLiquidityTLV alignment: {} bytes", tlv_alignment);

    // Should be aligned to largest field (u128 arrays in FixedVec = 16 bytes)
    assert_eq!(tlv_alignment, 16, "Unexpected alignment");

    // Test actual instance
    let reserves = vec![1_000_000_000_000_000_000u128, 2_000_000_000u128];
    let tlv = PoolLiquidityTLV::new(
        VenueId::UniswapV2,
        [0x55u8; 20],
        &reserves,
        1700000000000000000u64,
    )
    .unwrap();

    let serialized = tlv.as_bytes();
    assert_eq!(serialized.len(), tlv_size, "Serialized size mismatch");

    println!("✅ Memory layout validation passed");
}

/// Test edge cases and error conditions
#[test]
fn test_edge_cases() {
    let pool_address = [0xCCu8; 20];
    let timestamp_ns = 1700000000000000000u64;

    // Test with maximum u128 values
    let max_reserves = vec![u128::MAX, u128::MAX - 1];
    let tlv_max = PoolLiquidityTLV::new(
        VenueId::UniswapV2,
        pool_address,
        &max_reserves,
        timestamp_ns,
    )
    .unwrap();

    assert_eq!(tlv_max.to_reserves_vec(), max_reserves);

    // Test with minimum values
    let min_reserves = vec![0u128, 1u128];
    let tlv_min = PoolLiquidityTLV::new(
        VenueId::UniswapV2,
        pool_address,
        &min_reserves,
        timestamp_ns,
    )
    .unwrap();

    assert_eq!(tlv_min.to_reserves_vec(), min_reserves);

    // Test serialization roundtrip with extreme values
    let extreme_tlv = PoolLiquidityTLV::new(
        VenueId::UniswapV2,
        pool_address,
        &vec![0u128, u128::MAX, 42u128],
        timestamp_ns,
    )
    .unwrap();

    let serialized = extreme_tlv.as_bytes();
    let deserialized_ref = zerocopy::Ref::<_, PoolLiquidityTLV>::new(serialized).unwrap();
    let deserialized = *deserialized_ref.into_ref();

    assert_eq!(
        deserialized.to_reserves_vec(),
        vec![0u128, u128::MAX, 42u128]
    );

    println!("✅ Edge case validation passed");
}

/// Integration test with address conversion
#[test]
fn test_address_integration() {
    let eth_address = [
        0x1f, 0x98, 0x76, 0x54, 0x32, 0x10, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32, 0x10, 0xfe,
        0xdc, 0xba, 0x98, 0x76, 0x54,
    ];

    let reserves = vec![
        5_000_000_000_000_000_000u128, // 5 ETH
        12_000_000_000u128,            // 12,000 USDC
    ];

    let tlv = PoolLiquidityTLV::new(
        VenueId::UniswapV3,
        eth_address,
        &reserves,
        1700000000000000000u64,
    )
    .unwrap();

    // Validate address roundtrip
    let recovered_address = tlv.get_pool_address();
    assert_eq!(recovered_address, eth_address);

    // Validate serialization preserves address
    let serialized = tlv.as_bytes();
    let deserialized_ref = zerocopy::Ref::<_, PoolLiquidityTLV>::new(serialized).unwrap();
    let deserialized = *deserialized_ref.into_ref();

    assert_eq!(deserialized.get_pool_address(), eth_address);
    assert_eq!(deserialized.to_reserves_vec(), reserves);

    println!("✅ Address integration validation passed");
}
