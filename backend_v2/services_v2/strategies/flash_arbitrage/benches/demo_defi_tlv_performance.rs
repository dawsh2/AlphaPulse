//! Performance benchmarks for DemoDeFiArbitrageTLV system
//!
//! Validates that the new TLV system meets performance requirements:
//! - TLV serialization/deserialization: <1μs per message
//! - Signal output throughput: >10K arbitrage signals/second
//! - Dashboard conversion: <100μs per message
//! - End-to-end latency: <35μs (signal creation to relay output)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use protocol_v2::{
    tlv::builder::TLVMessageBuilder, tlv::demo_defi::DemoDeFiArbitrageTLV, tlv::types::TLVType,
    MessageHeader, PoolInstrumentId, RelayDomain, SourceType, VenueId,
};
use std::time::{SystemTime, UNIX_EPOCH};

const FLASH_ARBITRAGE_STRATEGY_ID: u16 = 21;

/// Create a realistic DemoDeFiArbitrageTLV for benchmarking
fn create_benchmark_arbitrage_tlv() -> DemoDeFiArbitrageTLV {
    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    // Create realistic pool IDs
    let usdc_token_id = 0xa0b86991c431aa73u64; // USDC
    let weth_token_id = 0xc02aaa39b223fe8du64; // WETH
    let pool_a = PoolInstrumentId::from_v2_pair(VenueId::UniswapV2, usdc_token_id, weth_token_id);
    let pool_b = PoolInstrumentId::from_v3_pair(VenueId::UniswapV3, usdc_token_id, weth_token_id);

    DemoDeFiArbitrageTLV::new(
        FLASH_ARBITRAGE_STRATEGY_ID,
        timestamp_ns,            // Use timestamp as signal ID
        95,                      // 95% confidence
        137,                     // Polygon chain ID
        25000000000i128,         // $250.00 expected profit (8 decimals)
        500000000000u128,        // $5000.00 required capital (8 decimals)
        2500000000000000000u128, // 0.0025 MATIC gas cost (18 decimals)
        VenueId::UniswapV2,      // Pool A venue
        pool_a,
        VenueId::UniswapV3, // Pool B venue
        pool_b,
        usdc_token_id,                               // Token in
        weth_token_id,                               // Token out
        100000000000u128,                            // 1000.00 USDC optimal amount (8 decimals)
        50,                                          // 0.5% slippage tolerance
        100,                                         // 100 Gwei max gas
        (timestamp_ns / 1_000_000_000) as u32 + 300, // Valid for 5 minutes
        200,                                         // High priority
        timestamp_ns,
    )
}

/// Benchmark TLV serialization performance
fn benchmark_tlv_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("tlv_serialization");
    group.throughput(Throughput::Elements(1));

    let arbitrage_tlv = create_benchmark_arbitrage_tlv();

    // Benchmark to_bytes() performance
    group.bench_function("serialize", |b| {
        b.iter(|| black_box(arbitrage_tlv.to_bytes()));
    });

    // Benchmark from_bytes() performance
    let serialized = arbitrage_tlv.to_bytes();
    group.bench_function("deserialize", |b| {
        b.iter(|| black_box(DemoDeFiArbitrageTLV::from_bytes(&serialized).unwrap()));
    });

    // Benchmark roundtrip performance
    group.bench_function("roundtrip", |b| {
        b.iter(|| {
            let bytes = black_box(arbitrage_tlv.to_bytes());
            black_box(DemoDeFiArbitrageTLV::from_bytes(&bytes).unwrap())
        });
    });

    group.finish();
}

/// Benchmark TLV message building performance
fn benchmark_tlv_message_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("tlv_message_building");
    group.throughput(Throughput::Elements(1));

    let arbitrage_tlv = create_benchmark_arbitrage_tlv();

    // Benchmark TLV message creation
    group.bench_function("to_tlv_message", |b| {
        b.iter(|| black_box(arbitrage_tlv.to_tlv_message()));
    });

    // Benchmark complete protocol message building
    group.bench_function("complete_protocol_message", |b| {
        b.iter(|| {
            let tlv_payload = black_box(arbitrage_tlv.to_bytes());
            black_box(
                TLVMessageBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy)
                    .add_tlv_bytes(TLVType::ExtendedTLV, tlv_payload)
                    .build(),
            )
        });
    });

    group.finish();
}

/// Benchmark Q64.64 conversion performance
fn benchmark_q64_conversions(c: &mut Criterion) {
    let mut group = c.benchmark_group("q64_conversions");
    group.throughput(Throughput::Elements(1));

    let arbitrage_tlv = create_benchmark_arbitrage_tlv();

    // Benchmark profit USD conversion
    group.bench_function("expected_profit_usd", |b| {
        b.iter(|| black_box(arbitrage_tlv.expected_profit_usd()));
    });

    // Benchmark capital USD conversion
    group.bench_function("required_capital_usd", |b| {
        b.iter(|| black_box(arbitrage_tlv.required_capital_usd()));
    });

    // Benchmark gas cost conversion
    group.bench_function("estimated_gas_cost_native", |b| {
        b.iter(|| black_box(arbitrage_tlv.estimated_gas_cost_native()));
    });

    // Benchmark slippage percentage
    group.bench_function("slippage_percentage", |b| {
        b.iter(|| black_box(arbitrage_tlv.slippage_percentage()));
    });

    group.finish();
}

/// Benchmark signal output throughput
fn benchmark_signal_output_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("signal_output_throughput");

    // Test different batch sizes to find optimal throughput
    let batch_sizes = vec![1, 10, 100, 1000];

    for batch_size in batch_sizes {
        group.throughput(Throughput::Elements(batch_size as u64));

        group.bench_with_input(
            BenchmarkId::new("create_signals", batch_size),
            &batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    let mut signals = Vec::with_capacity(batch_size);
                    for i in 0..batch_size {
                        let mut arbitrage_tlv = create_benchmark_arbitrage_tlv();
                        arbitrage_tlv.signal_id = arbitrage_tlv.signal_id + i as u64;
                        signals.push(black_box(arbitrage_tlv.to_bytes()));
                    }
                    black_box(signals)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("serialize_messages", batch_size),
            &batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    let mut messages = Vec::with_capacity(batch_size);
                    for i in 0..batch_size {
                        let mut arbitrage_tlv = create_benchmark_arbitrage_tlv();
                        arbitrage_tlv.signal_id = arbitrage_tlv.signal_id + i as u64;
                        let tlv_payload = arbitrage_tlv.to_bytes();
                        let message_bytes = TLVMessageBuilder::new(
                            RelayDomain::Signal,
                            SourceType::ArbitrageStrategy,
                        )
                        .add_tlv_bytes(TLVType::ExtendedTLV, tlv_payload)
                        .build();
                        messages.push(black_box(message_bytes));
                    }
                    black_box(messages)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage characteristics
fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    // Test different numbers of simultaneous TLVs in memory
    let tlv_counts = vec![100, 1000, 10000];

    for count in tlv_counts {
        group.bench_with_input(
            BenchmarkId::new("allocate_tlvs", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let mut tlvs = Vec::with_capacity(count);
                    for i in 0..count {
                        let mut arbitrage_tlv = create_benchmark_arbitrage_tlv();
                        arbitrage_tlv.signal_id = arbitrage_tlv.signal_id + i as u64;
                        tlvs.push(black_box(arbitrage_tlv));
                    }
                    black_box(tlvs)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("serialize_bulk", count),
            &count,
            |b, &count| {
                // Pre-allocate TLVs
                let mut tlvs = Vec::with_capacity(count);
                for i in 0..count {
                    let mut arbitrage_tlv = create_benchmark_arbitrage_tlv();
                    arbitrage_tlv.signal_id = arbitrage_tlv.signal_id + i as u64;
                    tlvs.push(arbitrage_tlv);
                }

                b.iter(|| {
                    let mut serialized = Vec::with_capacity(count);
                    for tlv in &tlvs {
                        serialized.push(black_box(tlv.to_bytes()));
                    }
                    black_box(serialized)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark validation operations
fn benchmark_validation_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation");
    group.throughput(Throughput::Elements(1));

    let arbitrage_tlv = create_benchmark_arbitrage_tlv();
    let current_time = (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()) as u32;

    // Benchmark validity checking
    group.bench_function("is_valid", |b| {
        b.iter(|| black_box(arbitrage_tlv.is_valid(current_time)));
    });

    // Benchmark TLV message validation
    let tlv_message = arbitrage_tlv.to_tlv_message();
    group.bench_function("from_tlv_message", |b| {
        b.iter(|| black_box(DemoDeFiArbitrageTLV::from_tlv_message(&tlv_message).unwrap()));
    });

    group.finish();
}

/// Load test: simulate high-frequency arbitrage signal generation
fn load_test_arbitrage_signals(c: &mut Criterion) {
    let mut group = c.benchmark_group("load_test");
    group.sample_size(20); // Fewer samples for load test
    group.measurement_time(std::time::Duration::from_secs(10)); // 10 second measurement

    // Simulate 1 second of arbitrage signal generation at different frequencies
    let frequencies = vec![1000, 5000, 10000, 50000]; // signals per second

    for freq in frequencies {
        group.throughput(Throughput::Elements(freq as u64));

        group.bench_with_input(
            BenchmarkId::new("generate_signals_per_sec", freq),
            &freq,
            |b, &freq| {
                b.iter(|| {
                    let mut signals = Vec::with_capacity(freq);
                    for i in 0..freq {
                        let mut arbitrage_tlv = create_benchmark_arbitrage_tlv();
                        arbitrage_tlv.signal_id = arbitrage_tlv.signal_id + i as u64;
                        arbitrage_tlv.timestamp_ns = arbitrage_tlv.timestamp_ns + (i as u64 * 1000); // 1μs apart

                        let tlv_payload = arbitrage_tlv.to_bytes();
                        let message_bytes = TLVMessageBuilder::new(
                            RelayDomain::Signal,
                            SourceType::ArbitrageStrategy,
                        )
                        .add_tlv_bytes(TLVType::ExtendedTLV, tlv_payload)
                        .build();

                        signals.push(black_box(message_bytes));
                    }
                    black_box(signals)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_tlv_serialization,
    benchmark_tlv_message_building,
    benchmark_q64_conversions,
    benchmark_signal_output_throughput,
    benchmark_memory_usage,
    benchmark_validation_performance,
    load_test_arbitrage_signals
);

criterion_main!(benches);
