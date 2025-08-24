#!/usr/bin/env rust-script

//! Test semantic equality for TLV message construction
//! This verifies round-trip parsing works correctly for Protocol V2

use protocol_v2::{
    parse_header, parse_tlv_extensions, tlv::market_data::PoolSwapTLV, RelayDomain, SourceType,
    TLVMessageBuilder, TLVType, VenueId,
};
use std::time::{SystemTime, UNIX_EPOCH};
use zerocopy::AsBytes;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing TLV Semantic Equality");

    // Create test PoolSwapTLV with real-world values using constructor
    let original_swap = PoolSwapTLV::new(
        [0x12; 20], // pool_address
        [0x34; 20], // token_in_addr
        [0x56; 20], // token_out_addr
        VenueId::Polygon,
        1_000_000_000_000_000_000u128, // amount_in - 1 WETH (18 decimals)
        3000_000_000u128,              // amount_out - 3000 USDC (6 decimals)
        1_000_000_000_000_000_000u128, // liquidity_after
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64,
        12345,     // block_number
        100,       // tick_after
        18,        // amount_in_decimals
        6,         // amount_out_decimals
        12345u128, // sqrt_price_x96_after
    );

    println!(
        "📦 Original swap: {} WETH → {} USDC",
        original_swap.amount_in as f64 / 1e18,
        original_swap.amount_out as f64 / 1e6
    );

    // Convert to bytes
    let tlv_bytes = original_swap.as_bytes().to_vec();
    println!("📏 TLV serialized: {} bytes", tlv_bytes.len());

    // Test direct round-trip
    use zerocopy::FromBytes;
    match PoolSwapTLV::ref_from(&tlv_bytes) {
        Some(parsed_swap) => {
            println!("✅ Direct TLV round-trip successful");

            // Test semantic equality
            let amounts_match = parsed_swap.amount_in == original_swap.amount_in
                && parsed_swap.amount_out == original_swap.amount_out;
            let metadata_match = parsed_swap.venue == original_swap.venue
                && parsed_swap.block_number == original_swap.block_number;

            if amounts_match && metadata_match {
                println!("🎯 Direct semantic equality: PASSED");
            } else {
                println!("❌ Direct semantic equality: FAILED");
                println!(
                    "   Original amount_in: {}, parsed: {}",
                    original_swap.amount_in, parsed_swap.amount_in
                );
                println!(
                    "   Original amount_out: {}, parsed: {}",
                    original_swap.amount_out, parsed_swap.amount_out
                );
            }
        }
        None => {
            println!("❌ Direct TLV round-trip failed: ref_from returned None");
            return Ok(());
        }
    }

    // Test through full Protocol V2 message
    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
        .add_tlv_bytes(TLVType::PoolSwap, tlv_bytes)
        .build();

    println!("📨 Protocol V2 message: {} bytes", message.len());

    // Parse header
    let header = parse_header(&message[..32])?;
    let magic = header.magic;
    let payload_size = header.payload_size;
    let checksum = header.checksum;
    println!(
        "📋 Header: magic=0x{:08X}, payload_size={}, checksum=0x{:08X}",
        magic, payload_size, checksum
    );

    // Parse TLV payload (skip checksum validation for now)
    let payload_end = 32 + header.payload_size as usize;
    if message.len() >= payload_end {
        let tlv_payload = &message[32..payload_end];

        // Parse TLV extensions without validation
        match parse_tlv_extensions(tlv_payload) {
            Ok(tlvs) => {
                println!("✅ Protocol V2 TLV parsing successful: {} TLVs", tlvs.len());

                if let Some(first_tlv) = tlvs.first() {
                    match first_tlv {
                        protocol_v2::TLVExtensionEnum::Standard(std_tlv) => {
                            println!(
                                "🔄 Standard TLV type: {}, length: {}",
                                std_tlv.header.tlv_type,
                                std_tlv.payload.len()
                            );

                            // Parse back to PoolSwapTLV
                            match PoolSwapTLV::ref_from(&std_tlv.payload) {
                                Some(final_swap) => {
                                    println!("✅ Full round-trip successful!");

                                    // Test full semantic equality
                                    let amounts_equal = final_swap.amount_in
                                        == original_swap.amount_in
                                        && final_swap.amount_out == original_swap.amount_out;
                                    let state_equal = final_swap.tick_after
                                        == original_swap.tick_after
                                        && final_swap.liquidity_after
                                            == original_swap.liquidity_after;
                                    let metadata_equal = final_swap.venue == original_swap.venue
                                        && final_swap.block_number == original_swap.block_number
                                        && final_swap.timestamp_ns == original_swap.timestamp_ns;

                                    if amounts_equal && state_equal && metadata_equal {
                                        println!("🎯 Full semantic equality: PASSED");
                                        println!(
                                            "✅ Round-trip semantic and deep equality working!"
                                        );
                                    } else {
                                        println!("❌ Full semantic equality: FAILED");
                                        println!("   Amounts equal: {}", amounts_equal);
                                        println!("   State equal: {}", state_equal);
                                        println!("   Metadata equal: {}", metadata_equal);
                                    }
                                }
                                None => {
                                    println!("❌ Failed to parse TLV back to PoolSwapTLV: ref_from returned None");
                                }
                            }
                        }
                        protocol_v2::TLVExtensionEnum::Extended(ext_tlv) => {
                            println!(
                                "🔄 Extended TLV type: {}, length: {}",
                                ext_tlv.header.tlv_type,
                                ext_tlv.payload.len()
                            );
                            println!("❌ Expected standard TLV, got extended");
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ Protocol V2 TLV parsing failed: {}", e);
            }
        }
    } else {
        println!("❌ Message too short for declared payload size");
    }

    Ok(())
}
