//! Performance validation script for DemoDeFiArbitrageTLV system
//!
//! Validates that the system meets the required performance characteristics:
//! - TLV roundtrip: <1μs per message
//! - Signal generation: >10K signals/second
//! - Memory efficiency: <1KB per signal average
//! - End-to-end latency validation

use protocol_v2::{
    tlv::builder::TLVMessageBuilder, tlv::demo_defi::DemoDeFiArbitrageTLV, tlv::types::TLVType,
    MessageHeader, PoolInstrumentId, RelayDomain, SourceType, VenueId,
};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const FLASH_ARBITRAGE_STRATEGY_ID: u16 = 21;

/// Create a realistic arbitrage opportunity for testing
fn create_test_arbitrage_tlv(signal_id: u64) -> DemoDeFiArbitrageTLV {
    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let usdc_token_id = 0xa0b86991c431aa73u64; // USDC
    let weth_token_id = 0xc02aaa39b223fe8du64; // WETH
    let pool_a = PoolInstrumentId::from_v2_pair(VenueId::UniswapV2, usdc_token_id, weth_token_id);
    let pool_b = PoolInstrumentId::from_v3_pair(VenueId::UniswapV3, usdc_token_id, weth_token_id);

    DemoDeFiArbitrageTLV::new(
        FLASH_ARBITRAGE_STRATEGY_ID,
        signal_id,
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

/// Test TLV serialization/deserialization performance
fn test_tlv_roundtrip_performance() -> (f64, bool) {
    println!("🧪 Testing TLV roundtrip performance...");

    let iterations = 100_000;
    let arbitrage_tlv = create_test_arbitrage_tlv(1);

    let start = Instant::now();

    for _i in 0..iterations {
        let bytes = arbitrage_tlv.to_bytes();
        let _recovered = DemoDeFiArbitrageTLV::from_bytes(&bytes).unwrap();
    }

    let duration = start.elapsed();
    let avg_nanos = duration.as_nanos() as f64 / iterations as f64;
    let avg_micros = avg_nanos / 1000.0;

    let passed = avg_micros < 1.0; // Target: <1μs per roundtrip

    println!("   Average roundtrip time: {:.3}μs per message", avg_micros);
    println!("   Performance target: <1.0μs");
    println!("   Result: {}", if passed { "✅ PASS" } else { "❌ FAIL" });

    (avg_micros, passed)
}

/// Test signal generation throughput
fn test_signal_generation_throughput() -> (f64, bool) {
    println!("🧪 Testing signal generation throughput...");

    let target_per_second = 10_000;
    let test_duration_secs = 1;
    let iterations = target_per_second * test_duration_secs;

    let start = Instant::now();

    for i in 0..iterations {
        let arbitrage_tlv = create_test_arbitrage_tlv(i as u64);
        let _tlv_payload = arbitrage_tlv.to_bytes();
        let _message_bytes =
            TLVMessageBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy)
                .add_tlv_bytes(TLVType::ExtendedTLV, _tlv_payload)
                .build();
    }

    let duration = start.elapsed();
    let signals_per_second = iterations as f64 / duration.as_secs_f64();

    let passed = signals_per_second >= target_per_second as f64;

    println!(
        "   Generated signals: {} in {:.3}s",
        iterations,
        duration.as_secs_f64()
    );
    println!("   Throughput: {:.0} signals/second", signals_per_second);
    println!(
        "   Performance target: ≥{} signals/second",
        target_per_second
    );
    println!("   Result: {}", if passed { "✅ PASS" } else { "❌ FAIL" });

    (signals_per_second, passed)
}

/// Test memory efficiency
fn test_memory_efficiency() -> (f64, bool) {
    println!("🧪 Testing memory efficiency...");

    let arbitrage_tlv = create_test_arbitrage_tlv(1);
    let serialized = arbitrage_tlv.to_bytes();
    let complete_message =
        TLVMessageBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy)
            .add_tlv_bytes(TLVType::ExtendedTLV, serialized.clone())
            .build();

    let tlv_size = serialized.len();
    let message_size = complete_message.len();
    let struct_size = std::mem::size_of::<DemoDeFiArbitrageTLV>();

    println!("   TLV struct size: {} bytes", struct_size);
    println!("   Serialized TLV size: {} bytes", tlv_size);
    println!("   Complete message size: {} bytes", message_size);

    let avg_memory_per_signal = message_size as f64;
    let target_max_bytes = 1024.0; // Target: <1KB per signal

    let passed = avg_memory_per_signal < target_max_bytes;

    println!("   Memory per signal: {:.0} bytes", avg_memory_per_signal);
    println!("   Performance target: <{:.0} bytes", target_max_bytes);
    println!("   Result: {}", if passed { "✅ PASS" } else { "❌ FAIL" });

    (avg_memory_per_signal, passed)
}

/// Test Q64.64 conversion performance
fn test_q64_conversion_performance() -> (f64, bool) {
    println!("🧪 Testing Q64.64 conversion performance...");

    let iterations = 1_000_000;
    let arbitrage_tlv = create_test_arbitrage_tlv(1);

    let start = Instant::now();

    for _i in 0..iterations {
        let _profit = arbitrage_tlv.expected_profit_usd();
        let _capital = arbitrage_tlv.required_capital_usd();
        let _gas = arbitrage_tlv.estimated_gas_cost_native();
        let _slippage = arbitrage_tlv.slippage_percentage();
    }

    let duration = start.elapsed();
    let avg_nanos = duration.as_nanos() as f64 / (iterations * 4) as f64; // 4 conversions per iteration

    let passed = avg_nanos < 100.0; // Target: <100ns per conversion

    println!("   Total conversions: {}", iterations * 4);
    println!("   Average conversion time: {:.1}ns", avg_nanos);
    println!("   Performance target: <100ns per conversion");
    println!("   Result: {}", if passed { "✅ PASS" } else { "❌ FAIL" });

    (avg_nanos, passed)
}

/// Test validation performance
fn test_validation_performance() -> (f64, bool) {
    println!("🧪 Testing validation performance...");

    let iterations = 100_000;
    let arbitrage_tlv = create_test_arbitrage_tlv(1);
    let current_time = (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()) as u32;

    let start = Instant::now();

    for _i in 0..iterations {
        let _is_valid = arbitrage_tlv.is_valid(current_time);
    }

    let duration = start.elapsed();
    let avg_nanos = duration.as_nanos() as f64 / iterations as f64;

    let passed = avg_nanos < 50.0; // Target: <50ns per validation

    println!("   Validations performed: {}", iterations);
    println!("   Average validation time: {:.1}ns", avg_nanos);
    println!("   Performance target: <50ns per validation");
    println!("   Result: {}", if passed { "✅ PASS" } else { "❌ FAIL" });

    (avg_nanos, passed)
}

/// Test end-to-end latency
fn test_end_to_end_latency() -> (f64, bool) {
    println!("🧪 Testing end-to-end latency...");

    let iterations = 10_000;
    let mut total_latency_nanos = 0u128;

    for i in 0..iterations {
        let start = Instant::now();

        // Full pipeline: create TLV -> serialize -> build message -> deserialize -> validate
        let arbitrage_tlv = create_test_arbitrage_tlv(i as u64);
        let tlv_payload = arbitrage_tlv.to_bytes();
        let message_bytes =
            TLVMessageBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy)
                .add_tlv_bytes(TLVType::ExtendedTLV, tlv_payload.clone())
                .build();
        let _recovered_tlv = DemoDeFiArbitrageTLV::from_bytes(&tlv_payload).unwrap();
        let _is_valid = arbitrage_tlv.is_valid((arbitrage_tlv.timestamp_ns / 1_000_000_000) as u32);

        let latency = start.elapsed();
        total_latency_nanos += latency.as_nanos();
    }

    let avg_latency_nanos = total_latency_nanos as f64 / iterations as f64;
    let avg_latency_micros = avg_latency_nanos / 1000.0;

    let passed = avg_latency_micros < 35.0; // Target: <35μs end-to-end

    println!("   End-to-end operations: {}", iterations);
    println!("   Average latency: {:.1}μs", avg_latency_micros);
    println!("   Performance target: <35.0μs");
    println!("   Result: {}", if passed { "✅ PASS" } else { "❌ FAIL" });

    (avg_latency_micros, passed)
}

fn main() {
    println!("🚀 DemoDeFiArbitrageTLV Performance Validation");
    println!("================================================");
    println!();

    let mut all_passed = true;
    let mut results = Vec::new();

    // Run all performance tests
    let (roundtrip_micros, roundtrip_passed) = test_tlv_roundtrip_performance();
    results.push(("TLV Roundtrip", roundtrip_micros, "μs", roundtrip_passed));
    all_passed &= roundtrip_passed;
    println!();

    let (throughput, throughput_passed) = test_signal_generation_throughput();
    results.push((
        "Signal Throughput",
        throughput,
        "signals/sec",
        throughput_passed,
    ));
    all_passed &= throughput_passed;
    println!();

    let (memory_bytes, memory_passed) = test_memory_efficiency();
    results.push(("Memory per Signal", memory_bytes, "bytes", memory_passed));
    all_passed &= memory_passed;
    println!();

    let (conversion_nanos, conversion_passed) = test_q64_conversion_performance();
    results.push(("Q64 Conversion", conversion_nanos, "ns", conversion_passed));
    all_passed &= conversion_passed;
    println!();

    let (validation_nanos, validation_passed) = test_validation_performance();
    results.push(("Validation", validation_nanos, "ns", validation_passed));
    all_passed &= validation_passed;
    println!();

    let (e2e_micros, e2e_passed) = test_end_to_end_latency();
    results.push(("End-to-End Latency", e2e_micros, "μs", e2e_passed));
    all_passed &= e2e_passed;
    println!();

    // Print summary
    println!("📊 Performance Validation Summary");
    println!("==================================");
    for (test_name, value, unit, passed) in results {
        let status = if passed { "✅ PASS" } else { "❌ FAIL" };
        println!("{:<20} {:<10.1} {:<12} {}", test_name, value, unit, status);
    }
    println!();

    if all_passed {
        println!("🎯 ALL PERFORMANCE TESTS PASSED!");
        println!("   The DemoDeFiArbitrageTLV system meets all performance requirements.");
        std::process::exit(0);
    } else {
        println!("⚠️  SOME PERFORMANCE TESTS FAILED!");
        println!("   Review the failed tests and optimize as needed.");
        std::process::exit(1);
    }
}
