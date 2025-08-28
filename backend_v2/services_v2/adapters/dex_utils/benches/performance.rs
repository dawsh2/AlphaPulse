//! Performance benchmarks for DEX ABI library
//!
//! Validates that the shared library maintains optimal performance
//! characteristics for high-frequency trading scenarios.

use torq_dex::abi::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use web3::types::{Bytes, Log, H160, H256, U256};

/// Create a realistic V3 swap log for benchmarking
fn create_v3_swap_log() -> Log {
    Log {
        address: H160::from_low_u64_be(0x1234567890abcdef), // V3-style address
        topics: vec![
            // Swap event signature
            H256::from_low_u64_be(0xc42079f94a635067),
            // sender (indexed)
            H256::from_low_u64_be(0x7a250d5630b4cf53),
            // recipient (indexed)
            H256::from_low_u64_be(0x68b3465833fb72a7),
        ],
        data: Bytes(vec![
            // amount0 (32 bytes) - negative for token0 out
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x8b, 0x04,
            0xe3, 0x2c, 0x8f, 0x60, // amount1 (32 bytes) - positive for token1 in
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5a, 0xf3, 0x10,
            0x7a, 0x40, 0x00, 0x00, // sqrtPriceX96 (20 bytes packed as 32 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x8b,
            0x7c, 0x3c, 0xb2, 0xd1, 0x4a, 0x3e, 0x8a, 0xf2, 0x1c, 0x5e, 0x7b, 0x9f, 0x8c, 0x1d,
            0x2a, 0x5f, 0x3b, 0x4c, // liquidity (16 bytes packed as 32 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x04, 0x56, 0x8b, 0xc1, 0x2d, 0x4f, 0x7a, 0x8e, 0x3b, 0x5c, 0x9d,
            0x1f, 0x2a, 0x6e, 0x8f, // tick (3 bytes packed as 32 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01, 0x2c, 0x40,
        ]),
        block_hash: Some(H256::random()),
        block_number: Some(18_000_000u64.into()),
        transaction_hash: Some(H256::random()),
        transaction_index: Some(0u64.into()),
        log_index: Some(0u64.into()),
        transaction_log_index: Some(0u64.into()),
        log_type: None,
        removed: Some(false),
    }
}

/// Create a realistic V2 swap log for benchmarking  
fn create_v2_swap_log() -> Log {
    Log {
        address: H160::from_low_u64_be(0x5c69bee701ef814a), // V2-style address
        topics: vec![
            // Swap event signature
            H256::from_low_u64_be(0xd78ad95fa46c994b),
            // sender (indexed)
            H256::from_low_u64_be(0x7a250d5630b4cf53),
            // to (indexed)
            H256::from_low_u64_be(0x68b3465833fb72a7),
        ],
        data: Bytes(vec![
            // amount0In (32 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5a, 0xf3, 0x10,
            0x7a, 0x40, 0x00, 0x00, // amount1In (32 bytes) - zero
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, // amount0Out (32 bytes) - zero
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, // amount1Out (32 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x74, 0xcb, 0x04,
            0x3c, 0x8f, 0x60, 0x00,
        ]),
        block_hash: Some(H256::random()),
        block_number: Some(18_000_000u64.into()),
        transaction_hash: Some(H256::random()),
        transaction_index: Some(0u64.into()),
        log_index: Some(0u64.into()),
        transaction_log_index: Some(0u64.into()),
        log_type: None,
        removed: Some(false),
    }
}

fn bench_protocol_detection(c: &mut Criterion) {
    let v3_log = create_v3_swap_log();
    let v2_log = create_v2_swap_log();

    c.bench_function("protocol_detection_v3", |b| {
        b.iter(|| black_box(detect_dex_protocol(&v3_log.address, &v3_log)))
    });

    c.bench_function("protocol_detection_v2", |b| {
        b.iter(|| black_box(detect_dex_protocol(&v2_log.address, &v2_log)))
    });
}

fn bench_abi_construction(c: &mut Criterion) {
    c.bench_function("v2_swap_abi_construction", |b| {
        b.iter(|| black_box(uniswap_v2::swap_event()))
    });

    c.bench_function("v3_swap_abi_construction", |b| {
        b.iter(|| black_box(uniswap_v3::swap_event()))
    });

    c.bench_function("v2_mint_abi_construction", |b| {
        b.iter(|| black_box(uniswap_v2::mint_event()))
    });

    c.bench_function("v3_mint_abi_construction", |b| {
        b.iter(|| black_box(uniswap_v3::mint_event()))
    });
}

fn bench_value_conversion(c: &mut Criterion) {
    let normal_value = U256::from(1_000_000_000_000u64);
    let large_value = U256::from(i64::MAX) + U256::from(1000000);
    let zero_value = U256::zero();

    c.bench_function("safe_conversion_normal", |b| {
        b.iter(|| black_box(SwapEventDecoder::safe_u256_to_i64(normal_value).unwrap()))
    });

    c.bench_function("safe_conversion_overflow", |b| {
        b.iter(|| black_box(SwapEventDecoder::safe_u256_to_i64(large_value).unwrap()))
    });

    c.bench_function("safe_conversion_zero", |b| {
        b.iter(|| black_box(SwapEventDecoder::safe_u256_to_i64(zero_value).unwrap()))
    });
}

fn bench_complete_decoding_pipeline(c: &mut Criterion) {
    let v3_log = create_v3_swap_log();
    let v2_log = create_v2_swap_log();

    c.bench_function("complete_v3_swap_decode", |b| {
        b.iter(|| {
            let protocol = detect_dex_protocol(&v3_log.address, &v3_log);
            black_box(SwapEventDecoder::decode_swap_event(&v3_log, protocol))
        })
    });

    c.bench_function("complete_v2_swap_decode", |b| {
        b.iter(|| {
            let protocol = detect_dex_protocol(&v2_log.address, &v2_log);
            black_box(SwapEventDecoder::decode_swap_event(&v2_log, protocol))
        })
    });
}

fn bench_high_frequency_scenario(c: &mut Criterion) {
    // Simulate high-frequency trading scenario with multiple logs
    let logs = vec![
        create_v3_swap_log(),
        create_v2_swap_log(),
        create_v3_swap_log(),
        create_v2_swap_log(),
        create_v3_swap_log(),
    ];

    c.bench_function("batch_processing_5_logs", |b| {
        b.iter(|| {
            for log in &logs {
                let protocol = detect_dex_protocol(&log.address, &log);
                let _ = black_box(SwapEventDecoder::decode_swap_event(&log, protocol));
            }
        })
    });

    // Test with 100 logs to simulate real load
    let large_batch: Vec<_> = (0..100)
        .map(|i| {
            if i % 2 == 0 {
                create_v3_swap_log()
            } else {
                create_v2_swap_log()
            }
        })
        .collect();

    c.bench_function("batch_processing_100_logs", |b| {
        b.iter(|| {
            for log in &large_batch {
                let protocol = detect_dex_protocol(&log.address, &log);
                let _ = black_box(SwapEventDecoder::decode_swap_event(&log, protocol));
            }
        })
    });
}

fn bench_memory_usage(c: &mut Criterion) {
    // Benchmark struct creation to ensure minimal allocations
    c.bench_function("validated_swap_creation", |b| {
        b.iter(|| {
            black_box(ValidatedSwap {
                pool_address: [1u8; 20],
                amount_in: 1000000,
                amount_out: 950000,
                token_in_is_token0: true,
                sqrt_price_x96_after: 12345678901234567890u128,
                tick_after: -1000,
                liquidity_after: 9876543210123456u128,
                dex_protocol: DEXProtocol::UniswapV3,
            })
        })
    });

    c.bench_function("validated_mint_creation", |b| {
        b.iter(|| {
            black_box(ValidatedMint {
                pool_address: [2u8; 20],
                liquidity_provider: [3u8; 20],
                liquidity_delta: 500000,
                amount0: 1000000,
                amount1: 2000000,
                tick_lower: -100,
                tick_upper: 100,
                dex_protocol: DEXProtocol::UniswapV3,
            })
        })
    });
}

criterion_group!(
    benches,
    bench_protocol_detection,
    bench_abi_construction,
    bench_value_conversion,
    bench_complete_decoding_pipeline,
    bench_high_frequency_scenario,
    bench_memory_usage
);

criterion_main!(benches);
