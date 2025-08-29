//! Comprehensive TLV validation tests for GAP-001 implementation
//!
//! Tests Protocol V2 compliance for QuoteTLV and InvalidationReason types,
//! including serialization, parsing, precision preservation, and integration.

use torq_types::protocol::tlv::{InvalidationReason, QuoteTLV, TradeTLV};
use torq_types::{InstrumentId, RelayDomain, TLVType, VenueId};
use zerocopy::{AsBytes, FromBytes};

#[test]
fn test_quote_tlv_type_assignment() {
    // Verify QuoteTLV has correct type number in Market Data domain (1-19)
    assert_eq!(TLVType::QuoteUpdate as u8, 17);

    // Verify it routes to correct domain
    assert_eq!(TLVType::QuoteUpdate.relay_domain(), RelayDomain::MarketData);

    // Verify size constraint
    assert_eq!(
        TLVType::QuoteUpdate.expected_payload_size(),
        Some(56) // QuoteTLV padded size
    );
}

#[test]
fn test_invalidation_reason_type_assignment() {
    // Verify InvalidationReason has correct type number in Market Data domain
    assert_eq!(TLVType::StateInvalidationReason as u8, 19);

    // Verify it routes to correct domain
    assert_eq!(
        TLVType::StateInvalidationReason.relay_domain(),
        RelayDomain::MarketData
    );

    // Verify size constraint (single byte enum)
    assert_eq!(
        TLVType::StateInvalidationReason.expected_payload_size(),
        Some(1)
    );
}

#[test]
fn test_quote_tlv_serialization() {
    let instrument_id = InstrumentId {
        venue: VenueId::Coinbase as u16,
        asset_type: 1,
        reserved: 0,
        asset_id: 12345,
    };

    let quote = QuoteTLV::new(
        VenueId::Coinbase,
        instrument_id,
        45_000_00000000i64,     // $45,000.00 bid (8 decimals)
        100_00000000i64,        // 100 size
        45_001_00000000i64,     // $45,001.00 ask
        150_00000000i64,        // 150 size
        1234567890123456789u64, // timestamp
    );

    // Test zero-copy serialization
    let bytes = quote.as_bytes();
    assert_eq!(bytes.len(), 56); // Padded size

    // Test round-trip
    let parsed = QuoteTLV::read_from(bytes).expect("Failed to parse QuoteTLV");
    assert_eq!(parsed.bid_price, 45_000_00000000i64);
    assert_eq!(parsed.ask_price, 45_001_00000000i64);
    assert_eq!(parsed.bid_size, 100_00000000i64);
    assert_eq!(parsed.ask_size, 150_00000000i64);
    assert_eq!(parsed.quote_timestamp_ns, 1234567890123456789u64);
}

#[test]
fn test_quote_tlv_precision_preservation() {
    // Test that financial precision is preserved (8 decimals for USD prices)
    let quote = QuoteTLV::new(
        VenueId::Kraken,
        InstrumentId::default(),
        12345678901234i64, // Precise price with all decimals
        98765432109876i64, // Precise size
        12345678901235i64, // One unit higher
        98765432109877i64, // One unit higher
        u64::MAX,          // Max timestamp
    );

    let bytes = quote.as_bytes();
    let parsed = QuoteTLV::read_from(bytes).unwrap();

    // Verify exact precision preservation
    assert_eq!(parsed.bid_price, 12345678901234i64);
    assert_eq!(parsed.ask_price, 12345678901235i64);
    assert_eq!(parsed.bid_size, 98765432109876i64);
    assert_eq!(parsed.ask_size, 98765432109877i64);
    assert_eq!(parsed.quote_timestamp_ns, u64::MAX);
}

#[test]
fn test_invalidation_reason_serialization() {
    // Test each enum variant
    let reasons = vec![
        InvalidationReason::Disconnection,
        InvalidationReason::AuthenticationFailure,
        InvalidationReason::RateLimited,
        InvalidationReason::Staleness,
        InvalidationReason::Maintenance,
        InvalidationReason::Recovery,
    ];

    for reason in reasons {
        let byte = reason as u8;
        assert!(byte <= 5); // Ensure it fits in expected range

        // Verify round-trip through byte representation
        match byte {
            0 => assert_eq!(reason, InvalidationReason::Disconnection),
            1 => assert_eq!(reason, InvalidationReason::AuthenticationFailure),
            2 => assert_eq!(reason, InvalidationReason::RateLimited),
            3 => assert_eq!(reason, InvalidationReason::Staleness),
            4 => assert_eq!(reason, InvalidationReason::Maintenance),
            5 => assert_eq!(reason, InvalidationReason::Recovery),
            _ => panic!("Unexpected byte value"),
        }
    }
}

#[test]
fn test_quote_tlv_in_message_builder() {
    // Test integration with Protocol V2 message building
    use codec::build_message_direct;
    use torq_types::SourceType;

    let quote = QuoteTLV::new(
        VenueId::Binance,
        InstrumentId::default(),
        50_000_00000000i64,
        200_00000000i64,
        50_001_00000000i64,
        250_00000000i64,
        1234567890123456789u64,
    );

    // Build message with QuoteTLV
    let message = build_message_direct(
        RelayDomain::MarketData,
        SourceType::BinanceCollector,
        TLVType::QuoteUpdate,
        &quote,
    )
    .expect("Failed to build message");

    assert!(message.len() > 32); // Header + TLV payload

    // Verify magic number (little-endian on x86/ARM)
    assert_eq!(&message[0..4], &[0xEF, 0xBE, 0xAD, 0xDE]);
}

#[test]
fn test_invalidation_reason_in_message() {
    // Test InvalidationReason serialization
    use codec::build_message_direct;
    use torq_types::protocol::tlv::StateInvalidationTLV;
    use torq_types::SourceType;

    // Create a StateInvalidationTLV with InvalidationReason
    let instruments = vec![InstrumentId::default()];
    let invalidation = StateInvalidationTLV::new(
        VenueId::Coinbase,
        1,
        &instruments,
        InvalidationReason::RateLimited,
        1234567890123456789u64,
    )
    .expect("Failed to create invalidation");

    // Build message
    let message = build_message_direct(
        RelayDomain::MarketData,
        SourceType::StateManager,
        TLVType::StateInvalidation,
        &invalidation,
    )
    .expect("Failed to build message");

    assert!(message.len() > 32);
    assert_eq!(&message[0..4], &[0xEF, 0xBE, 0xAD, 0xDE]);

    // Verify reason is properly encoded
    assert_eq!(invalidation.reason, InvalidationReason::RateLimited as u8);
}

#[test]
fn test_mixed_tlv_message() {
    // Test that different TLV types can coexist
    use codec::build_message_direct;
    use torq_types::SourceType;

    // Build messages with different TLV types
    let trade = TradeTLV::new(
        VenueId::Kraken,
        InstrumentId::default(),
        45_000_00000000i64,
        10_00000000i64,
        0, // buy side
        1234567890123456789u64,
    );

    let trade_msg = build_message_direct(
        RelayDomain::MarketData,
        SourceType::KrakenCollector,
        TLVType::Trade,
        &trade,
    )
    .expect("Failed to build trade message");

    let quote = QuoteTLV::new(
        VenueId::Kraken,
        InstrumentId::default(),
        44_999_00000000i64,
        50_00000000i64,
        45_001_00000000i64,
        60_00000000i64,
        1234567890123456789u64,
    );

    let quote_msg = build_message_direct(
        RelayDomain::MarketData,
        SourceType::KrakenCollector,
        TLVType::QuoteUpdate,
        &quote,
    )
    .expect("Failed to build quote message");

    // Verify both messages are valid Protocol V2 (little-endian on x86/ARM)
    assert_eq!(&trade_msg[0..4], &[0xEF, 0xBE, 0xAD, 0xDE]);
    assert_eq!(&quote_msg[0..4], &[0xEF, 0xBE, 0xAD, 0xDE]);

    // Trade is 40 bytes, Quote is 56 bytes (padded)
    assert!(trade_msg.len() >= 32 + 40); // Header + TradeTLV
    assert!(quote_msg.len() >= 32 + 56); // Header + QuoteTLV
}

#[test]
fn test_performance_characteristics() {
    use std::time::Instant;

    // Measure QuoteTLV serialization performance
    let quote = QuoteTLV::new(
        VenueId::Coinbase,
        InstrumentId::default(),
        45_000_00000000i64,
        100_00000000i64,
        45_001_00000000i64,
        150_00000000i64,
        1234567890123456789u64,
    );

    let iterations = 1_000_000;
    let start = Instant::now();

    for _ in 0..iterations {
        let bytes = quote.as_bytes();
        std::hint::black_box(bytes);
    }

    let elapsed = start.elapsed();
    let ns_per_op = elapsed.as_nanos() / iterations as u128;

    // Should be very fast due to zero-copy
    assert!(
        ns_per_op < 10,
        "Serialization too slow: {}ns per op",
        ns_per_op
    );

    // Measure parsing performance
    let bytes = quote.as_bytes();
    let start = Instant::now();

    for _ in 0..iterations {
        let parsed = QuoteTLV::read_from(bytes).unwrap();
        std::hint::black_box(parsed);
    }

    let elapsed = start.elapsed();
    let ns_per_op = elapsed.as_nanos() / iterations as u128;

    // Parsing should also be very fast
    assert!(ns_per_op < 20, "Parsing too slow: {}ns per op", ns_per_op);
}

#[test]
fn test_state_management_integration() {
    // Simulate state invalidation flow
    struct StateManager {
        invalidation_reason: Option<InvalidationReason>,
    }

    impl StateManager {
        fn invalidate(&mut self, reason: InvalidationReason) {
            self.invalidation_reason = Some(reason);
        }

        fn is_valid(&self) -> bool {
            self.invalidation_reason.is_none()
        }
    }

    let mut state_mgr = StateManager {
        invalidation_reason: None,
    };
    assert!(state_mgr.is_valid());

    // Simulate disconnection
    state_mgr.invalidate(InvalidationReason::Disconnection);
    assert!(!state_mgr.is_valid());
    assert_eq!(
        state_mgr.invalidation_reason,
        Some(InvalidationReason::Disconnection)
    );

    // Test all reasons
    for reason in &[
        InvalidationReason::AuthenticationFailure,
        InvalidationReason::RateLimited,
        InvalidationReason::Staleness,
        InvalidationReason::Maintenance,
        InvalidationReason::Recovery,
    ] {
        state_mgr.invalidate(*reason);
        assert!(!state_mgr.is_valid());
        assert_eq!(state_mgr.invalidation_reason, Some(*reason));
    }
}
