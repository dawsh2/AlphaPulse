#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! alphapulse-protocol = { path = "protocol" }
//! zerocopy = "0.7"
//! tokio = { version = "1.0", features = ["full"] }
//! anyhow = "1.0"
//! ```

use alphapulse_protocol::{ArbitrageOpportunityMessage, InstrumentId, VenueId, AssetType, SourceType};
use std::os::unix::net::UnixStream;
use std::io::Write;
use zerocopy::AsBytes;

fn main() -> anyhow::Result<()> {
    println!("🚀 Creating test ArbitrageOpportunityMessage...");
    
    // Create bijective IDs for WETH/USDC pair
    let token0_id = InstrumentId {
        venue: VenueId::Polygon as u16,
        asset_type: AssetType::Token as u8,
        reserved: 0,
        asset_id: 0x1234567890abcdef, // WETH address hash
    };
    
    let token1_id = InstrumentId {
        venue: VenueId::Polygon as u16,
        asset_type: AssetType::Token as u8,
        reserved: 0,
        asset_id: 0x1234567890abcde0, // USDC address hash
    };
    
    let buy_pool_id = InstrumentId {
        venue: VenueId::Polygon as u16,
        asset_type: AssetType::Pool as u8,
        reserved: 0,
        asset_id: 0x5555555555555555, // Uniswap pool
    };
    
    let sell_pool_id = InstrumentId {
        venue: VenueId::Polygon as u16,
        asset_type: AssetType::Pool as u8,
        reserved: 0,
        asset_id: 0x6666666666666666, // SushiSwap pool
    };
    
    // Create arbitrage message with profitable trade
    let arb_msg = ArbitrageOpportunityMessage::new(
        token0_id,
        token1_id,
        buy_pool_id,
        sell_pool_id,
        150_000_000, // $1.50 buy price (8 decimal fixed point)
        152_000_000, // $1.52 sell price (8 decimal fixed point)
        100_000_000_000, // $1000 trade size (8 decimal fixed point)
        2_000_000_000, // $20 gross profit (8 decimal fixed point)
        250_000_000, // $2.50 gas fee (8 decimal fixed point)
        300_000_000, // $3.00 dex fees (8 decimal fixed point)
        50_000_000, // $0.50 slippage cost (8 decimal fixed point)
        1_400_000_000, // $14.00 net profit (8 decimal fixed point)
        14000, // 1.4% profit (4 decimal fixed point)
        950, // 95% confidence (3 decimal fixed point)
        true, // executable
        "WETH", // token0 symbol
        "USDC", // token1 symbol
        "UniswapV2", // buy exchange
        "SushiSwap", // sell exchange
        1755668400000000000, // timestamp in nanoseconds
        SourceType::Scanner,
    );
    
    // Convert to bytes
    let message_bytes = arb_msg.as_bytes();
    println!("✅ Created {}-byte ArbitrageOpportunityMessage", message_bytes.len());
    
    // Connect to relay as polygon exchange
    println!("🔌 Connecting to relay server as polygon exchange...");
    let mut stream = UnixStream::connect("/tmp/alphapulse/polygon.sock")?;
    
    // Send the message
    println!("📤 Sending ArbitrageOpportunityMessage to relay...");
    stream.write_all(message_bytes)?;
    stream.flush()?;
    
    println!("✅ Message sent successfully!");
    println!("📊 Arbitrage Details:");
    println!("   - Pair: WETH/USDC");
    println!("   - Buy Price: $1.50 (UniswapV2)");
    println!("   - Sell Price: $1.52 (SushiSwap)");
    println!("   - Trade Size: $1,000");
    println!("   - Gross Profit: $20.00");
    println!("   - Total Fees: $6.00 (gas: $2.50, dex: $3.00, slippage: $0.50)");
    println!("   - Net Profit: $14.00 (1.4%)");
    println!("   - Executable: Yes");
    println!("   - Confidence: 95%");
    
    // Keep connection open briefly
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    Ok(())
}