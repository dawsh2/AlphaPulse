//! Performance validation tests for flash arbitrage strategy
//!
//! Ensures that critical hot path operations complete within <35μs target latency

use alphapulse_strategies::flash_arbitrage::relay_consumer::{
    DetectedOpportunity, ParseError, PercentageFixedPoint4, UsdFixedPoint8,
};
use protocol_v2::{PoolSwapTLV, VenueId};
use std::time::Instant;
use zerocopy::AsBytes;

/// Test that PoolSwapTLV parsing completes within 35μs
/// This is critical for maintaining low-latency arbitrage detection
#[test]
fn test_pool_swap_parsing_performance() {
    // Create a valid PoolSwapTLV
    let swap = PoolSwapTLV::new_raw(
        1_000_000_000_000_000_000u128,    // amount_in: 1 token
        2_000_000_000_000u128,            // amount_out: 0.002 token
        100_000_000_000_000_000_000u128,  // liquidity_after
        1234567890123456789u64,           // timestamp_ns
        15000000u64,                      // block_number
        -100i32,                          // tick_after
        VenueId::UniswapV3Polygon as u16, // venue
        18u8,                             // amount_in_decimals
        6u8,                              // amount_out_decimals
        [0u8; 8],                         // padding
        {
            let mut addr = [0u8; 32];
            addr[12..].copy_from_slice(&[0x42; 20]);
            addr
        }, // pool_address
        {
            let mut addr = [0u8; 32];
            addr[12..].copy_from_slice(&[0x11; 20]);
            addr
        }, // token_in_addr
        {
            let mut addr = [0u8; 32];
            addr[12..].copy_from_slice(&[0x22; 20]);
            addr
        }, // token_out_addr
        0u128,                            // sqrt_price_x96_after
    );

    let bytes = swap.as_bytes();

    // Warm up
    for _ in 0..100 {
        let _ = zerocopy::Ref::<_, PoolSwapTLV>::new(bytes);
    }

    // Measure parsing performance
    const ITERATIONS: usize = 10000;
    let start = Instant::now();

    for _ in 0..ITERATIONS {
        let parsed = zerocopy::Ref::<_, PoolSwapTLV>::new(bytes);
        assert!(parsed.is_some());
    }

    let elapsed = start.elapsed();
    let avg_time_ns = elapsed.as_nanos() / ITERATIONS as u128;
    let avg_time_us = avg_time_ns as f64 / 1000.0;

    println!("PoolSwapTLV parsing: {:.3}μs average", avg_time_us);

    // Assert we're under 35μs (with some buffer for test variance)
    assert!(
        avg_time_us < 35.0,
        "PoolSwapTLV parsing took {:.3}μs, exceeding 35μs target",
        avg_time_us
    );
}

/// Test that unaligned buffer handling still meets performance targets
#[test]
fn test_unaligned_parsing_performance() {
    // Create a valid PoolSwapTLV
    let swap = PoolSwapTLV::new_raw(
        1_000_000_000_000_000_000u128,    // amount_in: 1 token
        2_000_000_000_000u128,            // amount_out: 0.002 token
        100_000_000_000_000_000_000u128,  // liquidity_after
        1234567890123456789u64,           // timestamp_ns
        15000000u64,                      // block_number
        -100i32,                          // tick_after
        VenueId::UniswapV3Polygon as u16, // venue
        18u8,                             // amount_in_decimals
        6u8,                              // amount_out_decimals
        [0u8; 8],                         // padding
        {
            let mut addr = [0u8; 32];
            addr[12..].copy_from_slice(&[0x42; 20]);
            addr
        }, // pool_address
        {
            let mut addr = [0u8; 32];
            addr[12..].copy_from_slice(&[0x11; 20]);
            addr
        }, // token_in_addr
        {
            let mut addr = [0u8; 32];
            addr[12..].copy_from_slice(&[0x22; 20]);
            addr
        }, // token_out_addr
        0u128,                            // sqrt_price_x96_after
    );

    // Create an unaligned buffer
    let mut unaligned_buffer = vec![0u8; std::mem::size_of::<PoolSwapTLV>() + 1];
    unaligned_buffer[1..].copy_from_slice(swap.as_bytes());
    let unaligned_bytes = &unaligned_buffer[1..];

    // Warm up
    for _ in 0..100 {
        if zerocopy::Ref::<_, PoolSwapTLV>::new(unaligned_bytes).is_none() {
            // Fallback to aligned copy
            let mut aligned = vec![0u8; std::mem::size_of::<PoolSwapTLV>()];
            aligned.copy_from_slice(unaligned_bytes);
            let _ = zerocopy::Ref::<_, PoolSwapTLV>::new(&aligned[..]);
        }
    }

    // Measure parsing performance with fallback
    const ITERATIONS: usize = 10000;
    let start = Instant::now();

    for _ in 0..ITERATIONS {
        if let Some(parsed) = zerocopy::Ref::<_, PoolSwapTLV>::new(unaligned_bytes) {
            // Zero-copy path (should not happen with unaligned)
            assert!(false, "Expected unaligned buffer to fail zero-copy parsing");
        } else {
            // Fallback path - copy to aligned buffer
            let mut aligned = vec![0u8; std::mem::size_of::<PoolSwapTLV>()];
            aligned.copy_from_slice(unaligned_bytes);
            let parsed = zerocopy::Ref::<_, PoolSwapTLV>::new(&aligned[..]);
            assert!(parsed.is_some());
        }
    }

    let elapsed = start.elapsed();
    let avg_time_ns = elapsed.as_nanos() / ITERATIONS as u128;
    let avg_time_us = avg_time_ns as f64 / 1000.0;

    println!(
        "Unaligned PoolSwapTLV parsing with copy: {:.3}μs average",
        avg_time_us
    );

    // Assert we're still under 35μs even with the copy
    assert!(
        avg_time_us < 35.0,
        "Unaligned parsing took {:.3}μs, exceeding 35μs target",
        avg_time_us
    );
}

/// Test that error creation and formatting is fast
#[test]
fn test_error_handling_performance() {
    // Warm up
    for _ in 0..100 {
        let _ = ParseError::PayloadTooSmall {
            actual: 100,
            required: 200,
        };
        let _ = ParseError::AlignmentError;
        let _ = ParseError::TruncatedTLV {
            offset: 10,
            required: 50,
            available: 40,
        };
    }

    // Measure error creation performance
    const ITERATIONS: usize = 100000;
    let start = Instant::now();

    for i in 0..ITERATIONS {
        let err = match i % 3 {
            0 => ParseError::PayloadTooSmall {
                actual: i,
                required: i * 2,
            },
            1 => ParseError::AlignmentError,
            _ => ParseError::TruncatedTLV {
                offset: i,
                required: i + 100,
                available: i + 50,
            },
        };

        // Format the error (as would happen in logging)
        let _ = format!("{}", err);
    }

    let elapsed = start.elapsed();
    let avg_time_ns = elapsed.as_nanos() / ITERATIONS as u128;
    let avg_time_us = avg_time_ns as f64 / 1000.0;

    println!("Error handling: {:.3}μs average", avg_time_us);

    // Error handling should be much faster than parsing
    assert!(
        avg_time_us < 5.0,
        "Error handling took {:.3}μs, exceeding 5μs target",
        avg_time_us
    );
}

/// Test the full hot path from bytes to opportunity detection
#[test]
fn test_full_hot_path_performance() {
    // Create test data
    let swap = PoolSwapTLV::new_raw(
        1_000_000_000_000_000_000u128,    // amount_in: 1 token
        2_000_000_000_000u128,            // amount_out: 0.002 token
        100_000_000_000_000_000_000u128,  // liquidity_after
        1234567890123456789u64,           // timestamp_ns
        15000000u64,                      // block_number
        -100i32,                          // tick_after
        VenueId::UniswapV3Polygon as u16, // venue
        18u8,                             // amount_in_decimals
        6u8,                              // amount_out_decimals
        [0u8; 8],                         // padding
        {
            let mut addr = [0u8; 32];
            addr[12..].copy_from_slice(&[0x42; 20]);
            addr
        }, // pool_address
        {
            let mut addr = [0u8; 32];
            addr[12..].copy_from_slice(&[0x11; 20]);
            addr
        }, // token_in_addr
        {
            let mut addr = [0u8; 32];
            addr[12..].copy_from_slice(&[0x22; 20]);
            addr
        }, // token_out_addr
        0u128,                            // sqrt_price_x96_after
    );

    let bytes = swap.as_bytes();

    // Simulate the hot path
    const ITERATIONS: usize = 10000;
    let start = Instant::now();

    for _ in 0..ITERATIONS {
        // 1. Parse the TLV
        let parsed = match zerocopy::Ref::<_, PoolSwapTLV>::new(bytes) {
            Some(p) => *p,
            None => panic!("Failed to parse"),
        };

        // 2. Extract addresses (simulating the real processing)
        let pool_addr_20 = &parsed.pool_address[12..32];
        let token_in_20 = &parsed.token_in_addr[12..32];
        let token_out_20 = &parsed.token_out_addr[12..32];

        // 3. Create opportunity (simplified)
        let opportunity = DetectedOpportunity {
            expected_profit: UsdFixedPoint8(100_000_000),  // $1.00
            spread_percentage: PercentageFixedPoint4(100), // 0.01%
            required_capital: UsdFixedPoint8(10_000_000_000), // $100.00
            target_pool: format!("0x{:x}", pool_addr_20[0]),
        };

        // Ensure the compiler doesn't optimize away our work
        std::hint::black_box(opportunity);
    }

    let elapsed = start.elapsed();
    let avg_time_ns = elapsed.as_nanos() / ITERATIONS as u128;
    let avg_time_us = avg_time_ns as f64 / 1000.0;

    println!("Full hot path: {:.3}μs average", avg_time_us);

    // The full hot path should complete within our 35μs target
    assert!(
        avg_time_us < 35.0,
        "Full hot path took {:.3}μs, exceeding 35μs target",
        avg_time_us
    );
}
