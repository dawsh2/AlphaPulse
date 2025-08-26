//! Benchmark for TLVMessageBuilder performance with typed IDs
//!
//! Tests TLV message construction performance and compares raw vs typed ID usage

use alphapulse_types::{
    protocol::tlv::{builder::TLVMessageBuilder, market_data::TradeTLV},
    InstrumentId,
    OrderId,
    RelayDomain,
    SignalId,
    SourceType,
    StrategyId, // Add typed IDs for testing
    TLVType,
    VenueId,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

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

fn bench_tlv_builder_single_message(c: &mut Criterion) {
    let trade = create_trade_tlv();

    c.bench_function("tlv_builder_single_message", |b| {
        b.iter(|| {
            let message =
                TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
                    .add_tlv(TLVType::Trade, &trade)
                    .build();
            criterion::black_box(message);
        })
    });
}

fn bench_tlv_builder_multiple_messages(c: &mut Criterion) {
    let trade = create_trade_tlv();

    c.bench_function("tlv_builder_multiple_messages", |b| {
        b.iter(|| {
            let message1 =
                TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
                    .add_tlv(TLVType::Trade, &trade)
                    .build();
            let message2 =
                TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
                    .add_tlv(TLVType::Trade, &trade)
                    .build();
            criterion::black_box((message1, message2));
        })
    });
}

fn bench_typed_id_operations(c: &mut Criterion) {
    c.bench_function("typed_id_operations_in_context", |b| {
        b.iter(|| {
            // Simulate using typed IDs in message construction context
            let order = OrderId::new(criterion::black_box(12345));
            let signal = SignalId::new(criterion::black_box(67890));
            let strategy = StrategyId::new(criterion::black_box(1001));

            // Simulate operations that would be used with TLV messages
            let order_inner = order.inner();
            let signal_next = signal.next();
            let strategy_display = format!("{}", strategy);

            criterion::black_box((order_inner, signal_next, strategy_display));
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
            BenchmarkId::new("tlv_builder", tlv_count),
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
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_tlv_builder_single_message,
    bench_tlv_builder_multiple_messages,
    bench_typed_id_operations,
    bench_multiple_tlvs
);
criterion_main!(benches);
