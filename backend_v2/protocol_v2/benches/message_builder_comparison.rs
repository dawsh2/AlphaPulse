//! Benchmark comparing legacy TLVMessageBuilder vs ZeroCopyTLVMessageBuilder

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use protocol_v2::{
    tlv::{
        builder::TLVMessageBuilder, market_data::TradeTLV,
        zero_copy_builder::ZeroCopyTLVMessageBuilder,
    },
    InstrumentId, RelayDomain, SourceType, TLVType, VenueId,
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

fn bench_zero_copy_builder(c: &mut Criterion) {
    let trade = create_trade_tlv();

    c.bench_function("zero_copy_builder_single_tlv", |b| {
        b.iter(|| {
            let message = ZeroCopyTLVMessageBuilder::new(
                RelayDomain::MarketData,
                SourceType::PolygonCollector,
            )
            .add_tlv_ref(TLVType::Trade, &trade)
            .build();
            criterion::black_box(message);
        })
    });
}

fn bench_zero_copy_builder_with_buffer(c: &mut Criterion) {
    let trade = create_trade_tlv();

    // Pre-allocate buffer
    let builder =
        ZeroCopyTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
            .add_tlv_ref(TLVType::Trade, &trade);
    let size = builder.calculate_size();
    let mut buffer = vec![0u8; size];

    c.bench_function("zero_copy_builder_with_buffer", |b| {
        b.iter(|| {
            let builder = ZeroCopyTLVMessageBuilder::new(
                RelayDomain::MarketData,
                SourceType::PolygonCollector,
            )
            .add_tlv_ref(TLVType::Trade, &trade);
            let size = builder.build_into_buffer(&mut buffer).unwrap();
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
            BenchmarkId::new("zero_copy_builder", tlv_count),
            &tlv_count,
            |b, &count| {
                b.iter(|| {
                    let mut builder = ZeroCopyTLVMessageBuilder::new(
                        RelayDomain::MarketData,
                        SourceType::PolygonCollector,
                    );
                    for i in 0..count {
                        builder = builder.add_tlv_ref(TLVType::Trade, &trades[i]);
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
    bench_zero_copy_builder,
    bench_zero_copy_builder_with_buffer,
    bench_multiple_tlvs
);
criterion_main!(benches);
