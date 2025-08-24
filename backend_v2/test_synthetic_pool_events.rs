#!/usr/bin/env cargo run --bin
//! Test synthetic pool event injection to verify pipeline
//! 
//! This bypasses the polygon collector and injects fake pool events
//! directly into the relay to test if Relay → Dashboard → Frontend works.

use protocol_v2::{
    TLVMessageBuilder, TLVType, RelayDomain, SourceType,
    tlv::market_data::{PoolSwapTLV, PoolSyncTLV}
};
use tokio::net::UnixStream;
use tokio::io::AsyncWriteExt;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing synthetic pool event injection...");
    
    // Connect to relay socket
    let mut stream = UnixStream::connect("/tmp/alphapulse/market_data.sock")
        .await
        .context("Failed to connect to market data relay")?;
    
    println!("✅ Connected to market data relay");
    
    // Create synthetic pool swap event
    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos() as u64;
    
    let pool_swap = PoolSwapTLV {
        venue_name: [b'P', b'o', b'l', b'y', b'g', b'o', b'n', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        pool_address: [0x45, 0xdd, 0xa9, 0xcb, 0x7c, 0x25, 0x13, 0x1d, 0xf2, 0x68, 0x51, 0x51, 0x31, 0xf6, 0x47, 0xd7, 0x26, 0xf5, 0x06, 0x08], // USDC/WETH pool
        token0_symbol: [b'U', b'S', b'D', b'C', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        token1_symbol: [b'W', b'E', b'T', b'H', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        amount0_delta: -1000000,  // -1 USDC (6 decimals)
        amount1_delta: 250000000000000, // +0.00025 WETH (18 decimals) 
        sqrt_price_x96: 79228162514264337593543950336_u128, // ~1 USDC per ETH
        liquidity: 1000000000000,
        tick: -276000,
        fee_paid: 500, // 0.5 USDC fee
        timestamp: timestamp_ns,
        block_number: 12345678,
        log_index: 42,
    };
    
    // Build TLV message
    let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonDEX as u8);
    builder.add_tlv(TLVType::PoolSwap, &pool_swap)?;
    let message = builder.build();
    
    println!("📤 Sending synthetic pool swap event...");
    stream.write_all(&message).await?;
    
    // Wait a moment
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    // Create synthetic pool sync event
    let pool_sync = PoolSyncTLV {
        venue_name: [b'P', b'o', b'l', b'y', b'g', b'o', b'n', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        pool_address: [0x45, 0xdd, 0xa9, 0xcb, 0x7c, 0x25, 0x13, 0x1d, 0xf2, 0x68, 0x51, 0x51, 0x31, 0xf6, 0x47, 0xd7, 0x26, 0xf5, 0x06, 0x08],
        token0_symbol: [b'U', b'S', b'D', b'C', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        token1_symbol: [b'W', b'E', b'T', b'H', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        reserve0: 5000000000000, // 5M USDC
        reserve1: 1250000000000000000000, // 1,250 WETH  
        timestamp: timestamp_ns + 1000,
        block_number: 12345679,
        log_index: 43,
    };
    
    // Build second TLV message
    let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonDEX as u8);
    builder.add_tlv(TLVType::PoolSync, &pool_sync)?;
    let message = builder.build();
    
    println!("📤 Sending synthetic pool sync event...");
    stream.write_all(&message).await?;
    
    println!("✅ Synthetic pool events sent!");
    println!("📱 Check frontend at http://localhost:5177 for pool events");
    println!("🔍 Check WebSocket test script for message receipt");
    
    Ok(())
}