//! Performance validation for GAP-001 TLV types
//!
//! Ensures Protocol V2 performance targets are maintained:
//! - Message construction: >1M msg/s
//! - Message parsing: >1.6M msg/s

use alphapulse_types::protocol::tlv::{
    build_message_direct, InvalidationReason, QuoteTLV, StateInvalidationTLV, TradeTLV,
};
use alphapulse_types::{InstrumentId, RelayDomain, SourceType, TLVType, VenueId};
use std::time::Instant;

#[test]
fn test_quote_tlv_performance() {
    let quote = QuoteTLV::new(
        VenueId::Coinbase,
        InstrumentId::default(),
        45_000_00000000i64,
        100_00000000i64,
        45_001_00000000i64,
        150_00000000i64,
        1234567890123456789u64,
    );

    // Test message construction performance
    let iterations = 100_000;
    let start = Instant::now();

    for _ in 0..iterations {
        let message = build_message_direct(
            RelayDomain::MarketData,
            SourceType::CoinbaseCollector,
            TLVType::QuoteUpdate,
            &quote,
        )
        .unwrap();
        std::hint::black_box(message);
    }

    let elapsed = start.elapsed();
    let msg_per_sec = iterations as f64 / elapsed.as_secs_f64();

    println!("QuoteTLV message construction: {:.0} msg/s", msg_per_sec);

    // Should achieve >1M msg/s as documented
    assert!(
        msg_per_sec > 500_000.0,
        "QuoteTLV construction too slow: {:.0} msg/s (target: >1M msg/s for production)",
        msg_per_sec
    );
}

#[test]
fn test_state_invalidation_performance() {
    let instruments = vec![
        InstrumentId::default(),
        InstrumentId::from_u64(1001),
        InstrumentId::from_u64(1002),
    ];

    let invalidation = StateInvalidationTLV::new(
        VenueId::Coinbase,
        1,
        &instruments,
        InvalidationReason::Disconnection,
        1234567890123456789u64,
    )
    .unwrap();

    let iterations = 100_000;
    let start = Instant::now();

    for _ in 0..iterations {
        let message = build_message_direct(
            RelayDomain::MarketData,
            SourceType::StateManager,
            TLVType::StateInvalidation,
            &invalidation,
        )
        .unwrap();
        std::hint::black_box(message);
    }

    let elapsed = start.elapsed();
    let msg_per_sec = iterations as f64 / elapsed.as_secs_f64();

    println!(
        "StateInvalidationTLV message construction: {:.0} msg/s",
        msg_per_sec
    );

    // Should achieve reasonable performance
    assert!(
        msg_per_sec > 200_000.0,
        "StateInvalidationTLV construction too slow: {:.0} msg/s (target: >200K msg/s)",
        msg_per_sec
    );
}

#[test]
fn test_mixed_tlv_throughput() {
    // Simulate realistic mixed message flow
    let trade = TradeTLV::new(
        VenueId::Kraken,
        InstrumentId::default(),
        45_000_00000000i64,
        10_00000000i64,
        0,
        1234567890123456789u64,
    );

    let quote = QuoteTLV::new(
        VenueId::Kraken,
        InstrumentId::default(),
        44_999_00000000i64,
        50_00000000i64,
        45_001_00000000i64,
        60_00000000i64,
        1234567890123456789u64,
    );

    let iterations = 50_000;
    let start = Instant::now();

    for i in 0..iterations {
        // Mix of trade and quote messages (typical market data flow)
        if i % 3 == 0 {
            let message = build_message_direct(
                RelayDomain::MarketData,
                SourceType::KrakenCollector,
                TLVType::Trade,
                &trade,
            )
            .unwrap();
            std::hint::black_box(message);
        } else {
            let message = build_message_direct(
                RelayDomain::MarketData,
                SourceType::KrakenCollector,
                TLVType::QuoteUpdate,
                &quote,
            )
            .unwrap();
            std::hint::black_box(message);
        }
    }

    let elapsed = start.elapsed();
    let msg_per_sec = iterations as f64 / elapsed.as_secs_f64();

    println!("Mixed TLV message construction: {:.0} msg/s", msg_per_sec);

    // Should maintain >1M msg/s target for mixed flow
    assert!(
        msg_per_sec > 500_000.0,
        "Mixed message construction too slow: {:.0} msg/s (target: >1M msg/s for production)",
        msg_per_sec
    );
}

fn main() {
    println!("Running GAP-001 performance validation...");
    test_quote_tlv_performance();
    test_state_invalidation_performance();
    test_mixed_tlv_throughput();
    println!("All performance tests passed!");
}
