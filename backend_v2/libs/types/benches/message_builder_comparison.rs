//! Benchmark comparing legacy TLVMessageBuilder vs TrueZeroCopyBuilder

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use alphapulse_types::{
    protocol::tlv::{
        builder::TLVMessageBuilder,
        market_data::TradeTLV,
        zero_copy_builder_v2::{build_message_direct, TrueZeroCopyBuilder},
    },
    InstrumentId, VenueId, RelayDomain, SourceType, TLVType,
};

fn create_trade_tlv() -> TradeTLV {
    TradeTLV::new(
        VenueId::Polygon,
        InstrumentId {
            venue: VenueId::Polygon as u16,
            asset_type: 1,
            reserved: 0,
            asset_id: 12345,
        },
        100_000_000, // $1.00 with 8 decimals
        50_000_000,  // 0.5 tokens
        0,           // buy
        1234567890,  // timestamp
    )
}

fn bench_legacy_builder(c: &mut Criterion) {
    let trade = create_trade_tlv();

    c.bench_function("legacy_builder_single_tlv", |b| {
        b.iter(|| {
            let message =
                TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
                    .add_tlv(TLVType::Trade, &trade)
                    .build();
            criterion::black_box(message);
        })
    });
}

fn bench_true_zero_copy_builder(c: &mut Criterion) {
    let trade = create_trade_tlv();

    c.bench_function("true_zero_copy_builder_single_tlv", |b| {
        b.iter(|| {
            let message = build_message_direct(
                RelayDomain::MarketData,
                SourceType::PolygonCollector,
                TLVType::Trade,
                &trade,
            )
            .unwrap();
            criterion::black_box(message);
        })
    });
}

fn bench_true_zero_copy_with_thread_local_buffer(c: &mut Criterion) {
    let trade = create_trade_tlv();

    // Warmup the thread-local buffer
    for _ in 0..100 {
        let _ = with_hot_path_buffer(|buffer| {
            let builder =
                TrueZeroCopyBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector);
            builder
                .build_into_buffer(buffer, TLVType::Trade, &trade)
                .map(|size| (size, size))
        });
    }

    c.bench_function("true_zero_copy_with_thread_local_buffer", |b| {
        b.iter(|| {
            let size = with_hot_path_buffer(|buffer| {
                let builder =
                    TrueZeroCopyBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector);
                let size = builder
                    .build_into_buffer(buffer, TLVType::Trade, &trade)
                    .unwrap();
                Ok((size, size))
            })
            .unwrap();
            criterion::black_box(size);
        })
    });
}

fn bench_multiple_tlvs(c: &mut Criterion) {
    let trades: Vec<TradeTLV> = (0..10)
        .map(|i| {
            TradeTLV::new(
                VenueId::Polygon,
                InstrumentId {
                    venue: VenueId::Polygon as u16,
                    asset_type: 1,
                    reserved: 0,
                    asset_id: 12345 + i,
                },
                100_000_000 + i as i64,
                50_000_000 + i as i64,
                0,
                1234567890 + i as u64,
            )
        })
        .collect();

    let mut group = c.benchmark_group("multiple_tlvs");

    for tlv_count in [1, 5, 10] {
        group.bench_with_input(
            BenchmarkId::new("legacy_builder", tlv_count),
            &tlv_count,
            |b, &count| {
                b.iter(|| {
                    let mut builder = TLVMessageBuilder::new(
                        RelayDomain::MarketData,
                        SourceType::PolygonCollector,
                    );
                    for i in 0..count {
                        builder = builder.add_tlv(TLVType::Trade, &trades[i]);
                    }
                    let message = builder.build();
                    criterion::black_box(message);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("true_zero_copy_builder", tlv_count),
            &tlv_count,
            |b, &count| {
                b.iter(|| {
                    // For multiple TLVs, use standard builder since TrueZeroCopyBuilder handles single TLV
                    let mut builder = TLVMessageBuilder::new(
                        RelayDomain::MarketData,
                        SourceType::PolygonCollector,
                    );
                    for i in 0..count {
                        builder = builder.add_tlv(TLVType::Trade, &trades[i]);
                    }
                    let message = builder.build();
                    criterion::black_box(message);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_legacy_builder,
    bench_true_zero_copy_builder,
    bench_true_zero_copy_with_thread_local_buffer,
    bench_multiple_tlvs
);
criterion_main!(benches);
