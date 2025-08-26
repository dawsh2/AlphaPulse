//! Performance benchmarks for typed ID system
//!
//! Verifies zero-cost abstraction property of typed IDs

use alphapulse_types::{OrderId, PoolId, SignalId, StrategyId};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

fn bench_raw_u64_operations(c: &mut Criterion) {
    c.bench_function("raw_u64_creation", |b| {
        b.iter(|| {
            let id = criterion::black_box(12345u64);
            criterion::black_box(id)
        })
    });

    c.bench_function("raw_u64_arithmetic", |b| {
        b.iter(|| {
            let id = criterion::black_box(12345u64);
            let next = id.wrapping_add(1);
            criterion::black_box(next)
        })
    });
}

fn bench_typed_id_operations(c: &mut Criterion) {
    c.bench_function("typed_id_creation", |b| {
        b.iter(|| {
            let id = OrderId::new(criterion::black_box(12345));
            criterion::black_box(id)
        })
    });

    c.bench_function("typed_id_arithmetic", |b| {
        b.iter(|| {
            let id = OrderId::new(criterion::black_box(12345));
            let next = id.next();
            criterion::black_box(next)
        })
    });

    c.bench_function("typed_id_inner_access", |b| {
        b.iter(|| {
            let id = OrderId::new(criterion::black_box(12345));
            let inner = id.inner();
            criterion::black_box(inner)
        })
    });
}

fn bench_typed_id_conversions(c: &mut Criterion) {
    c.bench_function("typed_id_from_u64", |b| {
        b.iter(|| {
            let raw = criterion::black_box(12345u64);
            let id = OrderId::from(raw);
            criterion::black_box(id)
        })
    });

    c.bench_function("typed_id_to_u64", |b| {
        b.iter(|| {
            let id = OrderId::new(criterion::black_box(12345));
            let raw: u64 = id.into();
            criterion::black_box(raw)
        })
    });
}

fn bench_mixed_typed_ids(c: &mut Criterion) {
    c.bench_function("mixed_typed_id_operations", |b| {
        b.iter(|| {
            let order = OrderId::new(criterion::black_box(1));
            let signal = SignalId::new(criterion::black_box(2));
            let strategy = StrategyId::new(criterion::black_box(3));
            let pool = PoolId::new(criterion::black_box(4));

            // Simulate some operations
            let order_next = order.next();
            let signal_inner = signal.inner();
            let strategy_null = strategy.is_null();
            let pool_display = format!("{}", pool);

            criterion::black_box((order_next, signal_inner, strategy_null, pool_display))
        })
    });
}

fn bench_serialization(c: &mut Criterion) {
    c.bench_function("typed_id_serialization", |b| {
        b.iter(|| {
            let id = OrderId::new(criterion::black_box(12345));
            let json = serde_json::to_string(&id).unwrap();
            criterion::black_box(json)
        })
    });

    c.bench_function("typed_id_deserialization", |b| {
        let json = "12345";
        b.iter(|| {
            let id: OrderId = serde_json::from_str(criterion::black_box(json)).unwrap();
            criterion::black_box(id)
        })
    });
}

fn bench_memory_layout(c: &mut Criterion) {
    c.bench_function("memory_size_verification", |b| {
        b.iter(|| {
            // Verify zero-cost abstraction at runtime
            let raw_size = std::mem::size_of::<u64>();
            let typed_size = std::mem::size_of::<OrderId>();
            let align_size = std::mem::align_of::<OrderId>();

            assert_eq!(raw_size, typed_size);
            assert_eq!(align_size, std::mem::align_of::<u64>());

            criterion::black_box((raw_size, typed_size, align_size))
        })
    });
}

criterion_group!(
    benches,
    bench_raw_u64_operations,
    bench_typed_id_operations,
    bench_typed_id_conversions,
    bench_mixed_typed_ids,
    bench_serialization,
    bench_memory_layout
);

criterion_main!(benches);
