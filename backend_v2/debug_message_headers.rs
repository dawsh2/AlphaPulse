//! Debug utility to inspect TLV message headers
use protocol_v2::{
    tlv::{TLVMessageBuilder, TLVType, PoolSwapTLV},
    RelayDomain, SourceType, parse_header, MESSAGE_MAGIC
};

fn main() {
    println!("🔍 Debugging TLV Message Header Construction");
    
    // Create a simple PoolSwapTLV like the Polygon collector would
    let pool_swap = PoolSwapTLV {
        pool_id: [0u8; 32],
        token_in: [1u8; 20],
        token_out: [2u8; 20],
        amount_in: 1000000000000000000u64,  // 1.0 ETH
        amount_out: 2000000000u64,           // 2000 USDC  
        gas_used: 21000,
        gas_price: 20000000000u64,
        block_number: 12345,
        transaction_hash: [3u8; 32],
        log_index: 1,
        timestamp_ns: 1640995200000000000u64,
    };
    
    // Build message like Polygon collector
    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
        .add_tlv(TLVType::PoolSwap, &pool_swap)
        .build();
    
    println!("📦 Built message: {} bytes", message.len());
    println!("🔍 First 32 bytes (header): {:?}", &message[..32]);
    
    // Parse header to inspect fields
    match parse_header(&message) {
        Ok(header) => {
            println!("✅ Header parsed successfully:");
            println!("  sequence: {}", header.sequence);
            println!("  timestamp: {} ({} ns)", header.timestamp, 
                     if header.timestamp > 0 { "valid" } else { "ZERO!" });
            println!("  magic: 0x{:08x} (expected: 0x{:08x})", header.magic, MESSAGE_MAGIC);
            println!("  payload_size: {}", header.payload_size);
            println!("  relay_domain: {}", header.relay_domain);
            println!("  source: {}", header.source);
        },
        Err(e) => {
            println!("❌ Header parsing failed: {}", e);
        }
    }
    
    // Show raw bytes for debugging
    println!("\n🔍 Raw header bytes:");
    for (i, byte) in message[..32].iter().enumerate() {
        if i % 8 == 0 {
            print!("\n  [{:2}..{:2}]: ", i, i+7);
        }
        print!("{:02x} ", byte);
    }
    println!();
}