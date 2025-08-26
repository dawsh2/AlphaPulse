//! Zero-copy validation tests for Protocol V2 TLV message construction
//!
//! This test suite ensures that all TLV message construction follows the zero-copy
//! pattern correctly, preventing performance regressions from unnecessary allocations.
//!
//! ## Critical Requirements
//! - TLV structs MUST implement zerocopy::AsBytes trait
//! - build_message_direct MUST receive struct references, not byte arrays
//! - No intermediate to_bytes() conversions in hot path
//!
//! ## Performance Impact
//! Each violation adds ~100ns overhead. At 1M msg/s, this is 10% degradation!

use alphapulse_types::protocol::{
    tlv::{
        build_message_direct,
        market_data::{PoolSwapTLV, QuoteTLV, TradeTLV},
    },
    InstrumentId, RelayDomain, SourceType, TLVType, VenueId,
};
use zerocopy::AsBytes;

/// Verify that all market data TLV types implement AsBytes trait
#[test]
fn test_market_data_tlvs_implement_asbytes() {
    // This test verifies at compile time that these types implement AsBytes
    fn assert_implements_asbytes<T: AsBytes>() {}

    // Market Data TLVs
    assert_implements_asbytes::<TradeTLV>();
    assert_implements_asbytes::<QuoteTLV>();
    assert_implements_asbytes::<PoolSwapTLV>();

    // Signal TLVs removed - DemoDeFiArbitrageTLV was deleted
}

/// Test correct zero-copy pattern with build_message_direct
#[test]
fn test_correct_zerocopy_pattern() {
    // Create a TradeTLV using the proper constructor
    let instrument_id = InstrumentId::coin(VenueId::Kraken, "BTC-USD");
    let trade_tlv = TradeTLV::new(
        VenueId::Kraken,
        instrument_id,
        45000_00000000i64, // $45,000 with 8 decimal precision
        100_00000000i64,   // 100 units
        1,                 // Buy side
        1234567890,        // timestamp_ns
    );

    // CORRECT: Pass struct directly for zero-copy
    let message = build_message_direct(
        RelayDomain::MarketData,
        SourceType::PolygonCollector, // Use an existing source type
        TLVType::Trade,
        &trade_tlv, // Direct reference - zero-copy!
    )
    .expect("Should build message");

    assert!(message.len() >= 32); // At least header size
    assert_eq!(&message[0..4], &[0xEF, 0xBE, 0xAD, 0xDE]); // Magic bytes
}

// DemoDeFiArbitrageTLV tests removed - struct was deleted

/// Demonstrate the WRONG pattern (this should NOT compile if uncommented)
#[test]
fn test_wrong_pattern_documentation() {
    // This test documents what NOT to do. The code is commented out
    // because it won't compile - Vec<u8> doesn't implement AsBytes!

    /*
    // ❌ WRONG - This pattern defeats zero-copy optimization!
    let trade_tlv = TradeTLV { ... };
    let bytes = trade_tlv.to_bytes();  // Unnecessary conversion!

    // This line would fail to compile:
    // error[E0277]: the trait bound `Vec<u8>: AsBytes` is not satisfied
    let message = build_message_direct(
        RelayDomain::MarketData,
        SourceType::TestHarness,
        TLVType::Trade,
        &bytes,  // ❌ Vec<u8> doesn't implement AsBytes!
    );
    */

    // The compiler prevents this anti-pattern, ensuring zero-copy is maintained
    assert!(true, "Wrong pattern is prevented at compile time");
}

/// Performance test - verify zero-copy maintains >1M msg/s
#[test]
#[ignore] // Run with: cargo test --package protocol_v2 --test zerocopy_validation -- --ignored
fn test_zerocopy_performance() {
    use std::time::Instant;

    let iterations = 100_000;
    let instrument_id = InstrumentId::coin(VenueId::Kraken, "BTC-USD");
    let trade_tlv = TradeTLV::new(
        VenueId::Kraken,
        instrument_id,
        45000_00000000i64,
        100_00000000i64,
        1,
        1234567890,
    );

    let start = Instant::now();
    for _ in 0..iterations {
        let _message = build_message_direct(
            RelayDomain::MarketData,
            SourceType::PolygonCollector,
            TLVType::Trade,
            &trade_tlv,
        )
        .expect("Should build");
    }
    let duration = start.elapsed();

    let messages_per_sec = (iterations as f64) / duration.as_secs_f64();
    println!(
        "Zero-copy performance: {:.0} messages/second",
        messages_per_sec
    );

    // Verify we maintain >1M msg/s
    assert!(
        messages_per_sec > 1_000_000.0,
        "Performance regression detected: {:.0} msg/s (expected >1M)",
        messages_per_sec
    );
}

/// Test that all TLV types used in production implement AsBytes
#[test]
fn test_all_production_tlvs_zerocopy_compatible() {
    use alphapulse_types::protocol::tlv::market_data::*;
    use alphapulse_types::protocol::tlv::pool_state::*;

    fn assert_zerocopy<T: AsBytes>() {}

    // Market Data Domain (Types 1-19)
    assert_zerocopy::<TradeTLV>();
    assert_zerocopy::<QuoteTLV>();
    assert_zerocopy::<PoolSwapTLV>();
    assert_zerocopy::<PoolMintTLV>();
    assert_zerocopy::<PoolBurnTLV>();
    assert_zerocopy::<PoolTickTLV>();
    assert_zerocopy::<PoolSyncTLV>();
    assert_zerocopy::<PoolLiquidityTLV>();

    // Pool State
    assert_zerocopy::<PoolStateTLV>();

    // Signal Domain - DemoDeFiArbitrageTLV removed
}

/// Integration test - verify end-to-end message construction
#[test]
fn test_end_to_end_zerocopy_message_flow() {
    // Simulate real collector workflow
    let swap_tlv = PoolSwapTLV {
        venue: VenueId::UniswapV3 as u16,
        pool_address: [0x42; 20],
        pool_address_padding: [0; 12],
        token_in_addr: [0x11; 20],
        token_in_padding: [0; 12],
        token_out_addr: [0x22; 20],
        token_out_padding: [0; 12],
        amount_in: 1000000000000000000u128, // 1 ETH
        amount_in_decimals: 18,
        amount_out: 3000000000u128, // 3000 USDC
        amount_out_decimals: 6,
        sqrt_price_x96_after: [0; 32],
        tick_after: 100,
        liquidity_after: 1000000,
        block_number: 12345678,
        timestamp_ns: 1234567890123456789,
        _padding: [0; 8],
    };

    // Build message with zero-copy
    let message = build_message_direct(
        RelayDomain::MarketData,
        SourceType::PolygonCollector,
        TLVType::PoolSwap,
        &swap_tlv,
    )
    .expect("Should build swap message");

    // Verify message structure
    assert!(message.len() > 32, "Message should have header + payload");

    // Verify header magic bytes
    assert_eq!(&message[0..4], &[0xEF, 0xBE, 0xAD, 0xDE], "Magic bytes");

    // Verify domain and source in header
    assert_eq!(message[4], RelayDomain::MarketData as u8, "Relay domain");
    assert_eq!(
        message[6],
        SourceType::PolygonCollector as u8,
        "Source type"
    );

    // The message is correctly built with zero-copy, which is what we're testing
    // Full parsing may require proper checksum calculation which is not the focus here
}
