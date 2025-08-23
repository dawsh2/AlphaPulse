//! Benchmarks for AMM math calculations

use alphapulse_flash_arbitrage::math::{V2Math, V2PoolState, V3Math, V3PoolState};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn benchmark_v2_output_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("v2_output");

    let amounts = vec![
        ("small", dec!(1)),
        ("medium", dec!(100)),
        ("large", dec!(10000)),
    ];

    for (name, amount) in amounts {
        group.bench_with_input(
            BenchmarkId::new("calculate_output", name),
            &amount,
            |b, &amount_in| {
                b.iter(|| {
                    V2Math::calculate_output_amount(
                        black_box(amount_in),
                        black_box(dec!(10000)),
                        black_box(dec!(20000000)),
                        black_box(30),
                    )
                });
            },
        );
    }

    group.finish();
}

fn benchmark_v2_optimal_arbitrage(c: &mut Criterion) {
    c.bench_function("v2_optimal_arbitrage", |b| {
        let pool_a = V2PoolState {
            reserve_in: dec!(1000),
            reserve_out: dec!(2000000),
            fee_bps: 30,
        };

        let pool_b = V2PoolState {
            reserve_in: dec!(1950000),
            reserve_out: dec!(1000),
            fee_bps: 30,
        };

        b.iter(|| {
            V2Math::calculate_optimal_arbitrage_amount(black_box(&pool_a), black_box(&pool_b))
        });
    });
}

fn benchmark_v3_swap_calculation(c: &mut Criterion) {
    c.bench_function("v3_swap_within_tick", |b| {
        let pool = V3PoolState {
            liquidity: 1_000_000_000_000,
            sqrt_price_x96: 79228162514264337593543950336,
            current_tick: 0,
            fee_pips: 3000,
        };

        b.iter(|| {
            V3Math::calculate_output_amount(black_box(1000000), black_box(&pool), black_box(true))
        });
    });
}

fn benchmark_decimal_sqrt(c: &mut Criterion) {
    let mut group = c.benchmark_group("decimal_sqrt");

    let values = vec![
        ("small", dec!(2)),
        ("medium", dec!(100)),
        ("large", dec!(1000000)),
    ];

    for (name, value) in values {
        group.bench_with_input(BenchmarkId::new("sqrt", name), &value, |b, &val| {
            b.iter(|| {
                // This is a private function, so we test indirectly
                V2Math::calculate_optimal_arbitrage_amount(
                    black_box(&V2PoolState {
                        reserve_in: val,
                        reserve_out: val * dec!(2),
                        fee_bps: 30,
                    }),
                    black_box(&V2PoolState {
                        reserve_in: val * dec!(2),
                        reserve_out: val,
                        fee_bps: 30,
                    }),
                )
            });
        });
    }

    group.finish();
}

fn benchmark_price_impact(c: &mut Criterion) {
    let mut group = c.benchmark_group("price_impact");

    group.bench_function("v2_price_impact", |b| {
        b.iter(|| {
            V2Math::calculate_price_impact(
                black_box(dec!(100)),
                black_box(dec!(10000)),
                black_box(dec!(20000000)),
            )
        });
    });

    group.bench_function("v3_price_impact", |b| {
        let pool = V3PoolState {
            liquidity: 10_000_000_000_000,
            sqrt_price_x96: 79228162514264337593543950336,
            current_tick: 0,
            fee_pips: 500,
        };

        b.iter(|| {
            V3Math::calculate_price_impact(
                black_box(100_000_000),
                black_box(&pool),
                black_box(true),
            )
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_v2_output_calculation,
    benchmark_v2_optimal_arbitrage,
    benchmark_v3_swap_calculation,
    benchmark_decimal_sqrt,
    benchmark_price_impact
);

criterion_main!(benches);
