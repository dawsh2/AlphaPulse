//! Debug Message Sizes - Comprehensive size analysis tool
//!
//! This debugging binary traces the exact serialization process to identify
//! where the discrepancy between calculated and observed sizes occurs.

use protocol_v2::{
    tlv::{
        demo_defi::{ArbitrageConfig, DemoDeFiArbitrageTLV},
        builder::TLVMessageBuilder,
        TLVType,
    },
    message::header::MessageHeader,
    RelayDomain, SourceType, VenueId,
};
use std::time::{SystemTime, UNIX_EPOCH};
use zerocopy::AsBytes;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 **COMPREHENSIVE MESSAGE SIZE ANALYSIS**");
    println!("==========================================");

    // Create test data identical to what's used in signal_output.rs
    let now_ns = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
    
    let pool_32: [u8; 32] = {
        let mut pool = [0u8; 32];
        // Copy 20-byte address into first 20 bytes, leaving 12 bytes zero-padding
        pool[0..20].copy_from_slice(&[
            0x45, 0xdd, 0xa9, 0xcb, 0x7c, 0x25, 0x13, 0x1d, 0xf2, 0x68, 
            0x51, 0x51, 0x31, 0xf6, 0x47, 0xd7, 0x26, 0xf5, 0x06, 0x08
        ]);
        pool
    };

    // Recreate exact TLV from signal_output.rs
    let demo_tlv = DemoDeFiArbitrageTLV::new(ArbitrageConfig {
        strategy_id: 21, // FLASH_ARBITRAGE_STRATEGY_ID
        signal_id: 12345,
        confidence: 85,
        chain_id: 137, // Polygon
        expected_profit_q: (100.0 * (1u128 << 64) as f64) as i128, // $100 profit
        required_capital_q: (1000.0 * (1u128 << 64) as f64) as u128, // $1000 capital
        estimated_gas_cost_q: (2.50 * (1u128 << 64) as f64) as u128, // $2.50 gas
        venue_a: VenueId::QuickSwap,
        pool_a: pool_32,
        venue_b: VenueId::SushiSwapPolygon,
        pool_b: pool_32,
        token_in: 0x2791bca1f2de4661u64,  // USDC
        token_out: 0x0d500b1d8e8ef31eu64, // WMATIC
        optimal_amount_q: (1000.0 * (1u128 << 64) as f64) as u128,
        slippage_tolerance: 100, // 1%
        max_gas_price_gwei: 20,
        valid_until: (now_ns / 1_000_000_000) as u32 + 300,
        priority: 1,
        timestamp_ns: now_ns,
    });

    println!("📊 **STRUCT SIZE ANALYSIS**");
    println!("- DemoDeFiArbitrageTLV struct: {} bytes", std::mem::size_of::<DemoDeFiArbitrageTLV>());
    println!("- MessageHeader struct: {} bytes", std::mem::size_of::<MessageHeader>());
    
    // Test direct serialization
    let struct_bytes = demo_tlv.as_bytes();
    println!("- AsBytes serialization: {} bytes", struct_bytes.len());

    println!("\n📋 **TLV BUILDER ANALYSIS**");
    
    // Create builder exactly like signal_output.rs
    let builder = TLVMessageBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy)
        .add_extended_tlv(TLVType::ExtendedTLV, &demo_tlv);

    // Get payload size before building
    let payload_size = builder.payload_size();
    println!("- Builder payload_size(): {} bytes", payload_size);
    
    // Expected breakdown:
    println!("- Extended TLV header: 5 bytes");
    println!("- DemoDeFiArbitrageTLV data: {} bytes", struct_bytes.len());
    println!("- Expected payload total: {} bytes", 5 + struct_bytes.len());
    
    if payload_size != 5 + struct_bytes.len() {
        println!("⚠️ **PAYLOAD SIZE MISMATCH**: {} vs {}", payload_size, 5 + struct_bytes.len());
    }

    // Build the message
    let message_bytes = builder.build();
    
    println!("\n🔧 **BUILT MESSAGE ANALYSIS**");
    println!("- Total message length: {} bytes", message_bytes.len());
    println!("- Expected total: {} bytes", 32 + payload_size);
    
    if message_bytes.len() != 32 + payload_size {
        println!("⚠️ **TOTAL SIZE MISMATCH**: {} vs {}", message_bytes.len(), 32 + payload_size);
    }
    
    // Parse the actual message to verify structure
    println!("\n🔍 **MESSAGE STRUCTURE VALIDATION**");
    
    if message_bytes.len() >= 32 {
        // Parse header
        let header_slice = &message_bytes[0..32];
        let header = unsafe {
            &*(header_slice.as_ptr() as *const MessageHeader)
        };
        
        println!("- Header magic: 0x{:08X}", header.magic);
        println!("- Header payload_size: {} bytes", header.payload_size);
        println!("- Header calculated total: {} bytes", 32 + header.payload_size as usize);
        
        if message_bytes.len() != 32 + header.payload_size as usize {
            println!("⚠️ **HEADER MISMATCH**: message {} vs header {}", 
                     message_bytes.len(), 32 + header.payload_size as usize);
        }

        // Analyze payload structure
        if message_bytes.len() > 32 {
            let payload_slice = &message_bytes[32..];
            println!("- Actual payload length: {} bytes", payload_slice.len());
            
            if payload_slice.len() >= 5 {
                println!("- Extended TLV marker: {} (should be 255)", payload_slice[0]);
                println!("- Extended TLV reserved: {} (should be 0)", payload_slice[1]); 
                println!("- Extended TLV type: {}", payload_slice[2]);
                let tlv_length = u16::from_le_bytes([payload_slice[3], payload_slice[4]]);
                println!("- Extended TLV length: {} bytes", tlv_length);
                println!("- Extended TLV data starts at byte 5");
                
                let expected_payload_size = 5 + tlv_length as usize;
                if payload_slice.len() != expected_payload_size {
                    println!("⚠️ **TLV LENGTH MISMATCH**: payload {} vs expected {}", 
                             payload_slice.len(), expected_payload_size);
                }
                
                if tlv_length as usize != struct_bytes.len() {
                    println!("⚠️ **STRUCT SIZE MISMATCH**: TLV length {} vs struct {}", 
                             tlv_length, struct_bytes.len());
                    
                    // Show first few bytes for debugging
                    if payload_slice.len() > 5 {
                        let data_slice = &payload_slice[5..];
                        println!("- First 16 bytes of TLV data: {:02x?}", 
                                &data_slice[..std::cmp::min(16, data_slice.len())]);
                    }
                }
            }
        }
    }

    println!("\n📈 **SUMMARY**");
    println!("- Observed in logs: 261 bytes");
    println!("- Built message: {} bytes", message_bytes.len());
    println!("- Discrepancy: {} bytes", 
             message_bytes.len() as i32 - 261i32);

    if message_bytes.len() == 261 {
        println!("✅ **MATCH**: Built message matches observed size!");
    } else {
        println!("❌ **SIZE BUG FOUND**: Investigate further");
    }

    Ok(())
}