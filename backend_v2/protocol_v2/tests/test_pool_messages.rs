//! Test suite for pool message serialization/deserialization
//!
//! Ensures all modified TLV structures work correctly

use alphapulse_protocol_v2::{
    tlv::market_data::{PoolBurnTLV, PoolMintTLV, PoolSwapTLV, PoolSyncTLV, TLVMessage, TLVType},
    PoolInstrumentId, PoolProtocol, VenueId,
};

#[test]
fn test_pool_swap_tlv_with_v3_fields() {
    // Create a V3 swap with state updates
    let pool_id = PoolInstrumentId::from_v3_pair(VenueId::Polygon, 1234, 5678);

    let swap = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        token_in: 1234,
        token_out: 5678,
        amount_in: 1000_00000000,                  // 1000 tokens
        amount_out: 2000_00000000,                 // 2000 tokens
        amount_in_decimals: 18,                    // Token in has 18 decimals
        amount_out_decimals: 6,                    // Token out has 6 decimals
        sqrt_price_x96_after: 7922816251426433759, // sqrt(1) * 2^96 truncated to fit u64
        tick_after: 0,
        liquidity_after: 1000000_00000000,
        timestamp_ns: 1234567890,
        block_number: 1000,
    };

    // Serialize
    let bytes = swap.to_bytes();

    // Deserialize
    let decoded = PoolSwapTLV::from_bytes(&bytes).expect("Failed to deserialize");

    // Verify all fields
    assert_eq!(decoded.venue, swap.venue);
    assert_eq!(decoded.pool_id, swap.pool_id);
    assert_eq!(decoded.token_in, swap.token_in);
    assert_eq!(decoded.token_out, swap.token_out);
    assert_eq!(decoded.amount_in, swap.amount_in);
    assert_eq!(decoded.amount_out, swap.amount_out);
    assert_eq!(decoded.amount_in_decimals, swap.amount_in_decimals);
    assert_eq!(decoded.amount_out_decimals, swap.amount_out_decimals);
    assert_eq!(decoded.sqrt_price_x96_after, swap.sqrt_price_x96_after);
    assert_eq!(decoded.tick_after, swap.tick_after);
    assert_eq!(decoded.liquidity_after, swap.liquidity_after);
    assert_eq!(decoded.timestamp_ns, swap.timestamp_ns);
    assert_eq!(decoded.block_number, swap.block_number);
}

#[test]
fn test_pool_swap_tlv_v2_compatibility() {
    // V2 swap should have zero values for V3 fields
    let pool_id = PoolInstrumentId::from_v2_pair(VenueId::Polygon, 1234, 5678);

    let swap = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        token_in: 1234,
        token_out: 5678,
        amount_in: 1000_00000000,
        amount_out: 2000_00000000,
        amount_in_decimals: 18,
        amount_out_decimals: 6,
        sqrt_price_x96_after: 0, // V2 doesn't have this
        tick_after: 0,           // V2 doesn't have this
        liquidity_after: 0,      // V2 doesn't track this
        timestamp_ns: 1234567890,
        block_number: 1000,
    };

    let bytes = swap.to_bytes();
    let decoded = PoolSwapTLV::from_bytes(&bytes).expect("Failed to deserialize");

    assert_eq!(decoded.sqrt_price_x96_after, 0);
    assert_eq!(decoded.tick_after, 0);
    assert_eq!(decoded.liquidity_after, 0);
}

#[test]
fn test_pool_sync_tlv() {
    let pool_id = PoolInstrumentId::from_v2_pair(VenueId::Polygon, 1234, 5678);

    let sync = PoolSyncTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        reserve0: 1000000_00000000, // 1M tokens
        reserve1: 2000000_00000000, // 2M tokens
        timestamp_ns: 1234567890,
        block_number: 1000,
    };

    // Serialize
    let bytes = sync.to_bytes();

    // Deserialize
    let decoded = PoolSyncTLV::from_bytes(&bytes).expect("Failed to deserialize");

    assert_eq!(decoded.venue, sync.venue);
    assert_eq!(decoded.pool_id, sync.pool_id);
    assert_eq!(decoded.reserve0, sync.reserve0);
    assert_eq!(decoded.reserve1, sync.reserve1);
    assert_eq!(decoded.timestamp_ns, sync.timestamp_ns);
    assert_eq!(decoded.block_number, sync.block_number);
}

#[test]
fn test_pool_mint_burn_with_v3_ticks() {
    let pool_id = PoolInstrumentId::from_v3_pair(VenueId::Polygon, 1234, 5678);

    // Test Mint
    let mint = PoolMintTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        provider: 0xDEADBEEF,
        tick_lower: -887272, // V3 tick range
        tick_upper: 887272,
        liquidity_delta: 1000_00000000,
        amount0: 500_00000000,
        amount1: 500_00000000,
        timestamp_ns: 1234567890,
    };

    let bytes = mint.to_bytes();
    let decoded = PoolMintTLV::from_bytes(&bytes).expect("Failed to deserialize mint");

    assert_eq!(decoded.tick_lower, mint.tick_lower);
    assert_eq!(decoded.tick_upper, mint.tick_upper);
    assert_eq!(decoded.liquidity_delta, mint.liquidity_delta);

    // Test Burn
    let burn = PoolBurnTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        provider: 0xDEADBEEF,
        tick_lower: -887272,
        tick_upper: 887272,
        liquidity_delta: -500_00000000, // Negative for removal
        amount0: 250_00000000,
        amount1: 250_00000000,
        timestamp_ns: 1234567890,
    };

    let bytes = burn.to_bytes();
    let decoded = PoolBurnTLV::from_bytes(&bytes).expect("Failed to deserialize burn");

    assert_eq!(decoded.liquidity_delta, burn.liquidity_delta);
    assert!(decoded.liquidity_delta < 0); // Verify it's negative
}

#[test]
fn test_pool_instrument_id_with_protocol() {
    // Test V2 pool
    let v2_pool = PoolInstrumentId::from_v2_pair(VenueId::Polygon, 1234, 5678);
    assert_eq!(v2_pool.get_pool_protocol(), PoolProtocol::V2);
    assert!(v2_pool.is_v2());
    assert!(!v2_pool.is_v3());

    // Test V3 pool
    let v3_pool = PoolInstrumentId::from_v3_pair(VenueId::Polygon, 1234, 5678);
    assert_eq!(v3_pool.get_pool_protocol(), PoolProtocol::V3);
    assert!(!v3_pool.is_v2());
    assert!(v3_pool.is_v3());

    // Test that same tokens but different protocols produce different hashes
    assert_ne!(v2_pool.fast_hash, v3_pool.fast_hash);
}

#[test]
fn test_tlv_message_with_new_types() {
    let pool_id = PoolInstrumentId::from_v2_pair(VenueId::Polygon, 1234, 5678);

    // Test PoolSync TLV message
    let sync = PoolSyncTLV {
        venue: VenueId::Polygon,
        pool_id,
        reserve0: 1000000_00000000,
        reserve1: 2000000_00000000,
        timestamp_ns: 1234567890,
        block_number: 1000,
    };

    let msg = sync.to_tlv_message();
    assert_eq!(msg.header.tlv_type, TLVType::PoolSync);
    assert_eq!(msg.header.magic, 0xDEADBEEF);

    // Verify checksum is calculated
    assert_ne!(msg.header.checksum, 0);

    // Verify we can parse the message back
    let parsed = PoolSyncTLV::from_tlv_message(&msg).expect("Failed to parse TLV message");
    assert_eq!(parsed.reserve0, sync.reserve0);
    assert_eq!(parsed.reserve1, sync.reserve1);
}

#[test]
fn test_edge_cases() {
    let pool_id = PoolInstrumentId::from_v3_pair(VenueId::Polygon, 1234, 5678);

    // Test with maximum values
    let swap = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id,
        token_in: u64::MAX,
        token_out: u64::MAX,
        amount_in: i64::MAX,
        amount_out: i64::MAX,
        amount_in_decimals: 18,
        amount_out_decimals: 6,
        sqrt_price_x96_after: u64::MAX,
        tick_after: i32::MAX,
        liquidity_after: i64::MAX,
        timestamp_ns: u64::MAX,
        block_number: u64::MAX,
    };

    let bytes = swap.to_bytes();
    let decoded = PoolSwapTLV::from_bytes(&bytes).expect("Failed to deserialize max values");

    assert_eq!(decoded.amount_in, i64::MAX);
    assert_eq!(decoded.sqrt_price_x96_after, u64::MAX);
    assert_eq!(decoded.tick_after, i32::MAX);

    // Test with minimum/negative values
    let swap_min = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id: PoolInstrumentId::from_v3_pair(VenueId::Polygon, 0, 1),
        token_in: 0,
        token_out: 0,
        amount_in: i64::MIN,
        amount_out: i64::MIN,
        amount_in_decimals: 0,
        amount_out_decimals: 0,
        sqrt_price_x96_after: 0,
        tick_after: i32::MIN,
        liquidity_after: 0,
        timestamp_ns: 0,
        block_number: 0,
    };

    let bytes = swap_min.to_bytes();
    let decoded = PoolSwapTLV::from_bytes(&bytes).expect("Failed to deserialize min values");

    assert_eq!(decoded.tick_after, i32::MIN);
    assert_eq!(decoded.amount_in, i64::MIN);
}

#[test]
fn test_backward_compatibility() {
    // Ensure old code can at least partially read new messages
    // by checking that the basic fields are in the expected positions

    let pool_id = PoolInstrumentId::from_v2_pair(VenueId::Polygon, 1234, 5678);

    let swap = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id,
        token_in: 1234,
        token_out: 5678,
        amount_in: 1000_00000000,
        amount_out: 2000_00000000,
        amount_in_decimals: 18,
        amount_out_decimals: 6,
        sqrt_price_x96_after: 0,
        tick_after: 0,
        liquidity_after: 0,
        timestamp_ns: 1234567890,
        block_number: 1000,
    };

    let bytes = swap.to_bytes();

    // Check that venue is at the beginning
    let venue_bytes = &bytes[0..2];
    let venue = u16::from_le_bytes([venue_bytes[0], venue_bytes[1]]);
    assert_eq!(venue, VenueId::Polygon as u16);

    // The rest of the structure depends on PoolInstrumentId serialization
    // which should maintain backward compatibility
}
