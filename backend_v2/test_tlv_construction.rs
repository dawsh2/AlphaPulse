#!/usr/bin/env rust-script

//! Simple test to verify TLV message construction and parsing

use protocol_v2::{
    tlv::market_data::PoolSwapTLV,
    parse_header, parse_tlv_extensions,
    RelayDomain, SourceType, TLVMessageBuilder, TLVType, VenueId,
};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing TLV Construction and Round-trip Validation");
    
    // Create a simple PoolSwapTLV
    let swap_tlv = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_address: [0x12; 20],
        token_in_addr: [0x34; 20],
        token_out_addr: [0x56; 20],
        amount_in: 1000_000_000_000_000_000u128, // 1 WETH (18 decimals)
        amount_out: 3000_000_000u128,            // 3000 USDC (6 decimals)
        amount_in_decimals: 18,
        amount_out_decimals: 6,
        sqrt_price_x96_after: [0x78; 20],
        tick_after: 100,
        liquidity_after: 1_000_000_000_000_000_000u128,
        timestamp_ns: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64,
        block_number: 12345,
    };
    
    println!("📦 Created PoolSwapTLV: {} WETH → {} USDC", 
             swap_tlv.amount_in as f64 / 1e18, 
             swap_tlv.amount_out as f64 / 1e6);
    
    // Convert to bytes
    let tlv_bytes = swap_tlv.to_bytes();
    println!("📏 TLV bytes length: {}", tlv_bytes.len());
    
    // Build complete Protocol V2 message
    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
        .add_tlv_bytes(TLVType::PoolSwap, &tlv_bytes)
        .build();
    
    println!("📨 Built message: {} bytes total", message.len());
    
    // Parse header back
    let header = parse_header(&message[..32])?;
    println!("📋 Header parsed:");
    println!("   Magic: 0x{:08X}", header.magic);
    println!("   Payload size: {}", header.payload_size);
    println!("   Relay domain: {:?}", header.relay_domain);
    println!("   Source: {}", header.source);
    println!("   Checksum: 0x{:08X}", header.checksum);
    
    // Parse TLV payload
    let payload_end = 32 + header.payload_size as usize;
    if message.len() >= payload_end {
        let tlv_payload = &message[32..payload_end];
        println!("📦 TLV payload: {} bytes", tlv_payload.len());
        
        match parse_tlv_extensions(tlv_payload) {
            Ok(tlvs) => {
                println!("✅ TLV parsing successful: {} TLVs found", tlvs.len());
                
                // Test round-trip
                if let Some(first_tlv) = tlvs.first() {
                    println!("🔄 First TLV type: {}", first_tlv.header.tlv_type);
                    println!("🔄 First TLV length: {}", first_tlv.header.tlv_length);
                    
                    // Try to parse back to PoolSwapTLV
                    match PoolSwapTLV::from_bytes(&first_tlv.payload) {
                        Ok(parsed_swap) => {
                            println!("✅ Round-trip successful!");
                            println!("   Original amount_in: {}", swap_tlv.amount_in);
                            println!("   Parsed amount_in: {}", parsed_swap.amount_in);
                            
                            if parsed_swap.amount_in == swap_tlv.amount_in && 
                               parsed_swap.amount_out == swap_tlv.amount_out {
                                println!("🎯 Semantic equality: PASSED");
                            } else {
                                println!("❌ Semantic equality: FAILED");
                            }
                        }
                        Err(e) => {
                            println!("❌ Failed to parse back to PoolSwapTLV: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ TLV parsing failed: {}", e);
            }
        }
    } else {
        println!("❌ Message too short for declared payload size");
    }
    
    Ok(())
}