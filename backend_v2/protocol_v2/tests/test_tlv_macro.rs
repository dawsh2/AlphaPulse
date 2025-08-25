//! Tests for the TLV macro generation
//!
//! Validates that the define_tlv! macro correctly generates TLV structures

use protocol_v2::define_tlv;
use protocol_v2::tlv::market_data::{PoolSwapTLV, QuoteTLV, TradeTLV};
use protocol_v2::{InstrumentId, VenueId};
use zerocopy::{AsBytes, FromBytes};

#[test]
fn test_trade_tlv_macro_generated() {
    // Create a TradeTLV using the macro-generated structure
    let token0 = InstrumentId::polygon_token("0x0000000000000000000000000000000000000002").unwrap();
    let token1 = InstrumentId::polygon_token("0x0000000000000000000000000000000000000003").unwrap();
    let instrument_id = InstrumentId::pool(VenueId::Polygon, token0, token1);

    let trade = TradeTLV::from_instrument(
        VenueId::Polygon,
        instrument_id,
        100000000,  // price
        50000000,   // volume
        0,          // side
        1234567890, // timestamp
    );

    // Test that fields are correctly set (copy fields to avoid alignment issues)
    let asset_id = trade.asset_id;
    let price = trade.price;
    let volume = trade.volume;
    let timestamp_ns = trade.timestamp_ns;
    let venue_id = trade.venue_id;
    let side = trade.side;

    let expected_asset_id = instrument_id.asset_id;
    assert_eq!(asset_id, expected_asset_id);
    assert_eq!(price, 100000000);
    assert_eq!(volume, 50000000);
    assert_eq!(timestamp_ns, 1234567890);
    assert_eq!(venue_id, VenueId::Polygon as u16);
    assert_eq!(side, 0);

    // Test zero-copy serialization
    let bytes = trade.as_bytes();
    assert_eq!(bytes.len(), std::mem::size_of::<TradeTLV>());

    // Test zero-copy deserialization
    let parsed = TradeTLV::from_bytes(bytes).unwrap();
    assert_eq!(parsed, trade);
}

#[test]
fn test_quote_tlv_macro_generated() {
    let token0 = InstrumentId::polygon_token("0x0000000000000000000000000000000000000002").unwrap();
    let token1 = InstrumentId::polygon_token("0x0000000000000000000000000000000000000003").unwrap();
    let instrument_id = InstrumentId::pool(VenueId::Polygon, token0, token1);

    let quote = QuoteTLV::from_instrument(
        VenueId::Polygon,
        instrument_id,
        99000000,   // bid_price
        10000000,   // bid_size
        101000000,  // ask_price
        20000000,   // ask_size
        1234567890, // timestamp
    );

    // Test that fields are correctly set (copy fields to avoid alignment issues)
    let asset_id = quote.asset_id;
    let bid_price = quote.bid_price;
    let bid_size = quote.bid_size;
    let ask_price = quote.ask_price;
    let ask_size = quote.ask_size;
    let timestamp_ns = quote.timestamp_ns;

    let expected_asset_id = instrument_id.asset_id;
    assert_eq!(asset_id, expected_asset_id);
    assert_eq!(bid_price, 99000000);
    assert_eq!(bid_size, 10000000);
    assert_eq!(ask_price, 101000000);
    assert_eq!(ask_size, 20000000);
    assert_eq!(timestamp_ns, 1234567890);

    // Test zero-copy serialization
    let bytes = quote.as_bytes();
    assert_eq!(bytes.len(), std::mem::size_of::<QuoteTLV>());

    // Test zero-copy deserialization
    let parsed = QuoteTLV::from_bytes(bytes).unwrap();
    assert_eq!(parsed, quote);
}

#[test]
fn test_pool_swap_tlv_macro_generated() {
    let swap = PoolSwapTLV::from_addresses(
        [1u8; 20], // pool
        [2u8; 20], // token_in
        [3u8; 20], // token_out
        VenueId::Polygon,
        1000000000000000000u128, // amount_in
        2000000000u128,          // amount_out
        500000000000000000u128,  // liquidity_after
        1234567890,              // timestamp_ns
        12345,                   // block_number
        100,                     // tick_after
        18,                      // amount_in_decimals
        6,                       // amount_out_decimals
        123456789u128,           // sqrt_price_x96_after
    );

    // Test that fields are correctly set
    assert_eq!(swap.amount_in, 1000000000000000000u128);
    assert_eq!(swap.amount_out, 2000000000u128);
    assert_eq!(swap.liquidity_after, 500000000000000000u128);
    assert_eq!(swap.timestamp_ns, 1234567890);
    assert_eq!(swap.block_number, 12345);
    assert_eq!(swap.tick_after, 100);
    assert_eq!(swap.venue, VenueId::Polygon as u16);

    // Test zero-copy serialization
    let bytes = swap.as_bytes();
    assert_eq!(bytes.len(), std::mem::size_of::<PoolSwapTLV>());

    // Test that we can parse it back
    let parsed = PoolSwapTLV::from_bytes(bytes).unwrap();
    assert_eq!(parsed, swap);
}

#[test]
fn test_tlv_sizes_are_aligned() {
    // Ensure all TLV structs are properly aligned for zero-copy
    assert_eq!(
        std::mem::size_of::<TradeTLV>() % 8,
        0,
        "TradeTLV must be 8-byte aligned"
    );
    assert_eq!(
        std::mem::size_of::<QuoteTLV>() % 8,
        0,
        "QuoteTLV must be 8-byte aligned"
    );
    assert_eq!(
        std::mem::size_of::<PoolSwapTLV>() % 8,
        0,
        "PoolSwapTLV must be 8-byte aligned"
    );
}
