#!/usr/bin/env rust-script
//! Debug TLV Sizes - Measure actual serialization sizes
//!
//! This temporary debugging script measures the actual sizes of TLV messages
//! to identify the discrepancy between theoretical and observed message sizes.

use protocol_v2::{
    tlv::{demo_defi::DemoDeFiArbitrageTLV, TLVMessage},
    message::header::MessageHeader,
    RelayDomain, VenueId,
};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 **DEBUGGING TLV MESSAGE SIZES**");
    println!("=====================================");

    // 1. Measure struct size
    let struct_size = std::mem::size_of::<DemoDeFiArbitrageTLV>();
    println!("📏 DemoDeFiArbitrageTLV struct size: {} bytes", struct_size);

    // 2. Create a sample TLV
    let now_ns = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
    
    let arbitrage_tlv = DemoDeFiArbitrageTLV::new_with_addresses(
        21, // strategy_id: Flash Arbitrage
        12345, // signal_id
        85, // confidence
        137, // chain_id: Polygon
        100_000_000_000_000_000_000i128, // expected_profit_q: $100 in Q64.64
        1_000_000_000_000_000_000_000u128, // required_capital_q: $1000 in Q64.64
        5_000_000_000_000_000_000u128, // estimated_gas_cost_q: $5 in Q64.64
        VenueId::UniswapV3, // venue_a
        [0x45, 0xdd, 0xa9, 0xcb, 0x7c, 0x25, 0x13, 0x1d, 0xf2, 0x68, 0x51, 0x51, 0x31, 0xf6, 0x47, 0xd7, 0x26, 0xf5, 0x06, 0x08], // pool_a
        VenueId::SushiSwap, // venue_b
        [0x1f, 0x98, 0x43, 0x17, 0x01, 0x16, 0x6c, 0xc1, 0xba, 0x90, 0xa2, 0x3f, 0xc0, 0xe2, 0x13, 0x55, 0x67, 0xd4, 0xc7, 0x4a], // pool_b
        0x7ceB23fD6bC0adDB, // token_in: WETH (truncated)
        0x2791Bca1f2de4661, // token_out: USDC (truncated)
        500_000_000_000_000_000_000u128, // optimal_amount_q: $500 in Q64.64
        50, // slippage_tolerance: 0.5%
        30, // max_gas_price_gwei
        (now_ns / 1_000_000_000) as u32 + 300, // valid_until: 5 minutes from now
        1, // priority
        now_ns, // timestamp_ns
    );

    // 3. Measure serialization methods
    
    // Method 1: Direct struct bytes
    let struct_bytes = unsafe {
        std::slice::from_raw_parts(
            &arbitrage_tlv as *const DemoDeFiArbitrageTLV as *const u8,
            struct_size,
        )
    };
    println!("📦 Direct struct serialization: {} bytes", struct_bytes.len());

    // Method 2: zerocopy AsBytes
    use zerocopy::AsBytes;
    let zerocopy_bytes = arbitrage_tlv.as_bytes();
    println!("🔄 zerocopy::AsBytes serialization: {} bytes", zerocopy_bytes.len());

    // Method 3: Manual TLV message construction
    let tlv_message = TLVMessage {
        tlv_type: 255, // ExtendedTLV type for DemoDeFiArbitrageTLV
        payload: zerocopy_bytes.to_vec(),
    };
    
    let tlv_payload_size = tlv_message.payload.len();
    println!("📋 TLV payload size: {} bytes", tlv_payload_size);

    // 4. Calculate total message size (header + payload)
    let header_size = std::mem::size_of::<MessageHeader>();
    let total_message_size = header_size + tlv_payload_size;
    
    println!("\n📊 **SIZE BREAKDOWN**");
    println!("- MessageHeader: {} bytes", header_size);
    println!("- TLV payload: {} bytes", tlv_payload_size);
    println!("- **Total message**: {} bytes", total_message_size);
    
    // 5. Compare with observed sizes
    println!("\n⚠️ **SIZE COMPARISON**");
    println!("- Observed individual: 261 bytes");
    println!("- Observed concatenated: 522 bytes (2×261)");
    println!("- Calculated total: {} bytes", total_message_size);
    println!("- Expected by consumer: 258 bytes");
    
    let discrepancy = total_message_size as i32 - 261i32;
    if discrepancy != 0 {
        println!("🚨 **DISCREPANCY**: {} bytes difference!", discrepancy.abs());
    } else {
        println!("✅ **MATCH**: Calculated size matches observed!");
    }

    // 6. Test TLV parsing
    println!("\n🧪 **PARSING TEST**");
    match DemoDeFiArbitrageTLV::read_from(zerocopy_bytes) {
        Ok(parsed) => {
            println!("✅ TLV parsing successful");
            println!("   - Strategy ID: {}", parsed.strategy_id);
            println!("   - Signal ID: {}", parsed.signal_id);
            println!("   - Confidence: {}%", parsed.confidence);
        }
        Err(e) => {
            println!("❌ TLV parsing failed: {:?}", e);
        }
    }

    Ok(())
}