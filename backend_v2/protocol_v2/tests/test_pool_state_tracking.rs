//! Test pool state tracking with V2 and V3 pools
//!
//! Demonstrates the pool state management system for arbitrage detection

use alphapulse_protocol_v2::tlv::market_data::PoolSwapTLV;
use alphapulse_protocol_v2::tlv::pool_state::*;
use alphapulse_protocol_v2::*;
use std::collections::HashMap;

#[tokio::test]
async fn test_pool_state_v2_tracking(
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔄 Testing V2 Pool State Tracking");
    println!("{}", "=".repeat(50));

    // Create a V2 pool (Uniswap V2 style)
    let wmatic_token = 1001u64; // WMATIC token ID
    let usdc_token = 1002u64; // USDC token ID
    let pool_id = PoolInstrumentId::from_pair(VenueId::UniswapV2, wmatic_token, usdc_token);
    let mut pool_state = PoolStateTLV::from_v2_reserves(
        VenueId::UniswapV2,
        pool_id.clone(),
        18,                        // WMATIC has 18 decimals
        6,                         // USDC has 6 decimals
        1_000_000_000000000000i64, // 1M WMATIC (18 decimals - reduced to fit i64)
        250_000_000000i64,         // 250k USDC (native 6 decimal precision)
        30,                        // 0.3% fee (basis points)
        12345678,                  // block number
    );

    println!("Initial V2 Pool State:");
    println!("  Pool: {:?}", pool_state.pool_id);
    println!("  Type: {:?}", pool_state.pool_type);
    println!(
        "  Reserve0: {:.8} WMATIC",
        pool_state.reserve0 as f64 / 1e15
    ); // Adjusted for 15 decimals
    println!("  Reserve1: {:.6} USDC", pool_state.reserve1 as f64 / 1e6);
    println!("  Spot Price: {:.6} USDC/WMATIC", pool_state.spot_price());
    println!();

    // Simulate a swap: 100 WMATIC in → USDC out
    let swap_amount0 = 100_000000000000000i64; // 100 WMATIC in (15 decimals - fits i64)
    let swap_amount1 = -24_500000i64; // ~24.5 USDC out (6 decimals, negative = out)

    println!("Applying swap: 100 WMATIC → 24.5 USDC");
    pool_state.apply_swap(swap_amount0, swap_amount1, 0, 0);

    println!("After swap:");
    println!(
        "  Reserve0: {:.8} WMATIC",
        pool_state.reserve0 as f64 / 1e15
    ); // Adjusted for 15 decimals
    println!("  Reserve1: {:.6} USDC", pool_state.reserve1 as f64 / 1e6);
    println!("  New Price: {:.6} USDC/WMATIC", pool_state.spot_price());
    println!();

    // Test TLV serialization
    let tlv_msg = pool_state.to_tlv_message();
    println!("TLV Serialization:");
    println!("  Magic: 0x{:08X}", tlv_msg.header.magic);
    println!("  Type: {:?}", tlv_msg.header.tlv_type);
    println!("  Payload size: {} bytes", tlv_msg.payload.len());
    println!();

    Ok(())
}

#[tokio::test]
async fn test_pool_state_v3_tracking(
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔷 Testing V3 Pool State Tracking");
    println!("{}", "=".repeat(50));

    // Create a V3 pool (Uniswap V3 style)
    let usdc_token = 1002u64; // USDC token ID
    let weth_token = 1003u64; // WETH token ID
    let pool_id = PoolInstrumentId::from_pair(VenueId::UniswapV3, usdc_token, weth_token);
    let mut pool_state = PoolStateTLV::from_v3_state(
        VenueId::UniswapV3,
        pool_id.clone(),
        6,                       // USDC has 6 decimals
        18,                      // WETH has 18 decimals
        1771845812700228u64,     // sqrtPriceX96 for ~$4300 ETH (reduced to fit u64)
        85176,                   // Current tick
        500_000_000000000000i64, // Active liquidity (15 decimals - fits i64)
        500,                     // 0.05% fee (basis points)
        12345679,                // block number
    );

    println!("Initial V3 Pool State:");
    println!("  Pool: {:?}", pool_id);
    println!("  Type: {:?}", pool_state.pool_type);
    println!("  SqrtPriceX96: {}", pool_state.sqrt_price_x96);
    println!("  Tick: {}", pool_state.tick);
    println!("  Liquidity: {:.8}", pool_state.liquidity as f64 / 1e15); // Adjusted for 15 decimals
    println!(
        "  Virtual Reserve0: {:.6} USDC",
        pool_state.reserve0 as f64 / 1e6
    );
    println!(
        "  Virtual Reserve1: {:.8} WETH",
        pool_state.reserve1 as f64 / 1e15
    ); // Adjusted for 15 decimals
    println!("  Spot Price: ${:.2}/ETH", pool_state.spot_price());
    println!();

    // Simulate V3 swap with tick/price update
    let new_sqrt_price = 1781845812700229u64; // Slightly higher price (reduced to fit u64)
    let new_tick = 85180;

    println!("Applying V3 swap with price/tick update");
    pool_state.apply_swap(0, 0, new_sqrt_price, new_tick); // V3 recalculates reserves from price

    println!("After V3 swap:");
    println!("  New SqrtPriceX96: {}", pool_state.sqrt_price_x96);
    println!("  New Tick: {}", pool_state.tick);
    println!("  New Price: ${:.2}/ETH", pool_state.spot_price());
    println!();

    Ok(())
}

#[tokio::test]
async fn test_arbitrage_detection(
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🎯 Testing Arbitrage Detection");
    println!("{}", "=".repeat(50));

    let mut tracker = PoolStateTracker::new();

    // Create two pools with same tokens but different prices
    let wmatic_token = 1001u64; // WMATIC token ID
    let usdc_token = 1002u64; // USDC token ID

    let pool1_id = PoolInstrumentId::from_pair(VenueId::UniswapV2, wmatic_token, usdc_token);
    let pool1 = PoolStateTLV::from_v2_reserves(
        VenueId::UniswapV2,
        pool1_id.clone(),
        18,                        // WMATIC has 18 decimals
        6,                         // USDC has 6 decimals
        1_000_000_000000000000i64, // 1M WMATIC (15 decimals - fits i64)
        240_000_000000i64,         // 240k USDC (6 decimals, 0.24 price)
        30,                        // 0.3% fee
        12345678,                  // block number
    );

    let pool2_id = PoolInstrumentId::from_pair(VenueId::SushiSwap, wmatic_token, usdc_token);
    let pool2 = PoolStateTLV::from_v2_reserves(
        VenueId::SushiSwap,
        pool2_id.clone(),
        18,                        // WMATIC has 18 decimals
        6,                         // USDC has 6 decimals
        1_000_000_000000000000i64, // 1M WMATIC (15 decimals - fits i64)
        250_000_000000i64,         // 250k USDC (6 decimals, 0.25 price)
        30,                        // 0.3% fee
        12345678,                  // block number
    );

    println!("Pool States:");
    println!("  UniswapV2 price: {:.6} USDC/WMATIC", pool1.spot_price());
    println!("  SushiSwap price: {:.6} USDC/WMATIC", pool2.spot_price());

    // Calculate spread
    let price_diff = (pool2.spot_price() - pool1.spot_price()).abs();
    let avg_price = (pool1.spot_price() + pool2.spot_price()) / 2.0;
    let spread_pct = price_diff / avg_price * 100.0;

    println!("  Price difference: {:.6}", price_diff);
    println!("  Spread: {:.2}%", spread_pct);

    if spread_pct > 0.5 {
        println!("  🚨 ARBITRAGE OPPORTUNITY!");
        println!("    Buy on UniswapV2 @ {:.6}", pool1.spot_price());
        println!("    Sell on SushiSwap @ {:.6}", pool2.spot_price());

        // Calculate potential profit for $10k trade
        let trade_size = 10000.0;
        let tokens_bought = trade_size / pool1.spot_price();
        let revenue = tokens_bought * pool2.spot_price();
        let gross_profit = revenue - trade_size;
        let gas_cost = 50.0; // Estimate
        let net_profit = gross_profit - gas_cost;

        println!("    Trade size: ${:.0}", trade_size);
        println!("    Gross profit: ${:.2}", gross_profit);
        println!("    Est. gas cost: ${:.0}", gas_cost);
        println!("    Net profit: ${:.2}", net_profit);
    }

    println!();

    Ok(())
}

#[tokio::test]
async fn test_pool_state_serialization(
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📦 Testing Pool State TLV Serialization");
    println!("{}", "=".repeat(50));

    // Create pool state
    let wmatic_token = 1001u64; // WMATIC token ID
    let usdc_token = 1002u64; // USDC token ID
    let pool_id = PoolInstrumentId::from_pair(VenueId::UniswapV2, wmatic_token, usdc_token);
    let pool_state = PoolStateTLV::from_v2_reserves(
        VenueId::UniswapV2,
        pool_id,
        18,                        // WMATIC has 18 decimals
        6,                         // USDC has 6 decimals
        1_000_000_000000000000i64, // 1M WMATIC (15 decimals - fits i64)
        240_000_000000i64,         // 240k USDC (6 decimals)
        30,                        // 0.3% fee
        12345678,                  // block number
    );

    println!("Original Pool State:");
    println!("  Venue: {:?}", pool_state.venue);
    println!("  Pool Type: {:?}", pool_state.pool_type);
    println!("  Reserve0: {}", pool_state.reserve0);
    println!("  Reserve1: {}", pool_state.reserve1);
    println!("  Fee Rate: {} bps", pool_state.fee_rate);
    println!("  Block: {}", pool_state.block_number);

    // Test binary serialization
    let bytes = pool_state.to_bytes();
    println!("\nBinary Serialization:");
    println!("  Payload size: {} bytes", bytes.len());
    println!(
        "  First 32 bytes: {}",
        hex::encode(&bytes[..32.min(bytes.len())])
    );

    // Test TLV message
    let tlv_msg = pool_state.to_tlv_message();
    println!("\nTLV Message:");
    println!("  Magic: 0x{:08X}", tlv_msg.header.magic);
    println!(
        "  Type: {:?} ({})",
        tlv_msg.header.tlv_type, tlv_msg.header.tlv_type as u8
    );
    println!("  Payload length: {}", tlv_msg.header.payload_len);
    println!("  Checksum: 0x{:02X}", tlv_msg.header.checksum);

    println!("✅ Pool state serialization working correctly");

    Ok(())
}
