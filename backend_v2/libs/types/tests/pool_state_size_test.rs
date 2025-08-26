//! Test to verify PoolStateTLV size remains 192 bytes after padding changes

use alphapulse_types::protocol::tlv::pool_state::PoolStateTLV;

#[test]
fn test_poolstate_tlv_size_unchanged() {
    let size = std::mem::size_of::<PoolStateTLV>();
    println!("PoolStateTLV size: {} bytes", size);

    // Expected: 192 bytes total (must match original for Protocol V2 wire compatibility)
    // Breakdown after explicit padding changes:
    // - u128 fields: 16 * 4 = 64 bytes (reserve0, reserve1, sqrt_price_x96, liquidity)
    // - u64 fields: 8 * 2 = 16 bytes (block_number, timestamp_ns)
    // - u32 fields: 4 * 2 = 8 bytes (tick, fee_rate)
    // - u16 fields: 2 * 1 = 2 bytes (venue)
    // - u8 fields: 1 * 6 = 6 bytes (pool_type, token0_decimals, token1_decimals, _padding[3])
    // - special fields (now with explicit padding):
    //   - pool_address: 20 bytes + pool_address_padding: 12 bytes = 32 bytes
    //   - token0_addr: 20 bytes + token0_padding: 12 bytes = 32 bytes
    //   - token1_addr: 20 bytes + token1_padding: 12 bytes = 32 bytes
    //   - Total special: 96 bytes
    // Total: 64 + 16 + 8 + 2 + 6 + 96 = 192 bytes

    assert_eq!(size, 192,
        "PoolStateTLV size changed! Expected 192 bytes for Protocol V2 wire format compatibility, got {} bytes",
        size);
}
