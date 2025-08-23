// Test script to verify TLV serialization fix
use alphapulse_protocol_v2::tlv::market_data::PoolSwapTLV;
use alphapulse_protocol_v2::VenueId;

fn main() {
    // Create a test PoolSwapTLV
    let swap = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_address: [1u8; 20],
        token_in_addr: [2u8; 20],
        token_out_addr: [3u8; 20],
        amount_in: 1000000000000000000, // 1 token (18 decimals)
        amount_out: 2000000000, // 2000 tokens (6 decimals)  
        amount_in_decimals: 18,
        amount_out_decimals: 6,
        tick_after: 123456,
        sqrt_price_x96_after: 1000000000000000000000000,
        liquidity_after: 5000000000000000000,
        timestamp_ns: 1000000000000000000,
        block_number: 45000000,
    };

    println!("Original PoolSwapTLV: {:?}", swap);

    // Test the struct's native serialization (should be 146 bytes)
    let native_bytes = swap.to_bytes();
    println!("Native serialization: {} bytes", native_bytes.len());
    
    // Test deserialization
    match PoolSwapTLV::from_bytes(&native_bytes) {
        Ok(recovered) => {
            println!("Native deserialization: SUCCESS");
            println!("Values match: {}", swap == recovered);
        },
        Err(e) => println!("Native deserialization failed: {}", e),
    }

    println!("✅ TLV serialization test complete");
}