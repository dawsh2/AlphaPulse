//! Performance benchmark for Decimal arithmetic in hot path
//!
//! Tests the performance impact of using Decimal instead of f64
//! in the arbitrage detection hot path to ensure <35μs latency target

use alphapulse_flash_arbitrage::config::DetectorConfig;
use alphapulse_flash_arbitrage::detector::OpportunityDetector;
use alphapulse_state_market::PoolStateManager;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;

fn bench_decimal_arithmetic(c: &mut Criterion) {
    c.bench_function("decimal_profit_calculation", |b| {
        let trade_size = dec!(2000.0);
        let net_profit = dec!(60.0);
        let gas_cost = dec!(3.0);

        b.iter(|| {
            // Simulate the profit margin calculation from detector.rs
            let spread_percentage = if trade_size > Decimal::ZERO {
                (net_profit + gas_cost) / trade_size * Decimal::from(100)
            } else {
                Decimal::ZERO
            };

            let profit_margin = if trade_size > Decimal::ZERO {
                (net_profit / trade_size) * Decimal::from(100)
            } else {
                Decimal::ZERO
            };

            black_box((spread_percentage, profit_margin))
        });
    });

    c.bench_function("f64_profit_calculation_baseline", |b| {
        let trade_size = 2000.0_f64;
        let net_profit = 60.0_f64;
        let gas_cost = 3.0_f64;

        b.iter(|| {
            // Same calculation with f64 for comparison
            let spread_percentage = if trade_size > 0.0 {
                (net_profit + gas_cost) / trade_size * 100.0
            } else {
                0.0
            };

            let profit_margin = if trade_size > 0.0 {
                (net_profit / trade_size) * 100.0
            } else {
                0.0
            };

            black_box((spread_percentage, profit_margin))
        });
    });

    c.bench_function("decimal_conversion_overhead", |b| {
        let decimal_val = dec!(1234.5678);

        b.iter(|| {
            // Test the overhead of converting to f64 for display
            let f64_val = decimal_val.to_f64().unwrap_or(0.0);
            let rounded = decimal_val.round_dp(2);
            black_box((f64_val, rounded))
        });
    });

    c.bench_function("decimal_comparison_operations", |b| {
        let val1 = dec!(10.5);
        let val2 = dec!(10.0);
        let threshold = dec!(0.5);

        b.iter(|| {
            // Test comparison operations used in guards
            let is_greater = val1 > val2;
            let is_profitable = (val1 - val2) > threshold;
            let margin_check = val1 > Decimal::from(10);
            black_box((is_greater, is_profitable, margin_check))
        });
    });
}

fn bench_full_detection_path(c: &mut Criterion) {
    // Benchmark a more complete detection path
    c.bench_function("full_arbitrage_calculation_decimal", |b| {
        b.iter(|| {
            // Simulate the full calculation path with Decimal
            let optimal_position_amount = dec!(1000.0);
            let expected_profit = dec!(50.0);
            let gas_cost = dec!(3.0);
            let slippage_bps = 25u32;

            let trade_size_usd = optimal_position_amount;
            let net_profit = expected_profit;

            let spread_percentage = if trade_size_usd > Decimal::ZERO {
                (net_profit + gas_cost) / trade_size_usd * Decimal::from(100)
            } else {
                Decimal::ZERO
            };

            let profit_margin = if trade_size_usd > Decimal::ZERO {
                (net_profit / trade_size_usd) * Decimal::from(100)
            } else {
                Decimal::ZERO
            };

            // Profitability guard
            let is_suspicious = profit_margin > dec!(10.0);

            black_box((
                spread_percentage,
                profit_margin,
                is_suspicious,
                slippage_bps,
            ))
        });
    });
}

criterion_group!(benches, bench_decimal_arithmetic, bench_full_detection_path);
criterion_main!(benches);
