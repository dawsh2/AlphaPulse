//! Debug PoolSwapTLV Parsing Issues
//!
//! This tool helps debug why PoolSwapTLV parsing is failing in the relay consumer,
//! which causes the system to fall back to fake analysis data.

use torq_types::tlv::market_data::PoolSwapTLV;
use std::mem::size_of;
use zerocopy::{AsBytes, FromBytes};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 **POOLSWAP TLV PARSING DEBUG**");
    println!("=================================");

    // 1. Check struct size
    let expected_size = size_of::<PoolSwapTLV>();
    println!("📏 PoolSwapTLV struct size: {} bytes", expected_size);

    // 2. Check alignment requirements
    println!(
        "📐 PoolSwapTLV alignment: {} bytes",
        std::mem::align_of::<PoolSwapTLV>()
    );

    // 3. Create a test PoolSwapTLV to understand the structure
    let test_swap = PoolSwapTLV::new(
        [0x42u8; 20], // pool
        [0x43u8; 20], // token_in
        [0x44u8; 20], // token_out
        protocol_v2::VenueId::Polygon,
        1000u128,      // amount_in
        900u128,       // amount_out
        5000u128,      // liquidity_after
        1234567890u64, // timestamp_ns
        12345u64,      // block_number
        100i32,        // tick_after
        18u8,          // amount_in_decimals
        6u8,           // amount_out_decimals
        12345u128,     // sqrt_price_x96_after
    );

    // 4. Serialize it to bytes
    let serialized = test_swap.as_bytes();
    println!("✅ Test PoolSwapTLV serialized: {} bytes", serialized.len());

    if serialized.len() != expected_size {
        println!(
            "🚨 **SIZE MISMATCH**: serialized {} vs struct {}",
            serialized.len(),
            expected_size
        );
        return Ok(());
    }

    // 5. Try to parse it back
    match PoolSwapTLV::from_bytes(serialized) {
        Ok(parsed) => {
            println!("✅ Successfully parsed PoolSwapTLV back from bytes");
            println!("   - amount_in: {}", parsed.amount_in);
            println!("   - amount_out: {}", parsed.amount_out);
            println!("   - venue: {}", parsed.venue);
        }
        Err(e) => {
            println!("❌ **PARSING FAILED**: {}", e);
            println!("   This indicates the FromBytes implementation has issues");
        }
    }

    // 6. Try parsing with different alignment scenarios
    println!("\n🧪 **ALIGNMENT TESTING**");

    // Test unaligned parsing (common in network protocols)
    let mut unaligned_buffer = vec![0u8; serialized.len() + 1];
    unaligned_buffer[1..].copy_from_slice(serialized);

    match PoolSwapTLV::from_bytes(&unaligned_buffer[1..]) {
        Ok(_) => {
            println!("✅ Unaligned parsing works");
        }
        Err(e) => {
            println!("⚠️ Unaligned parsing failed: {}", e);
            println!("   This is expected - zerocopy requires proper alignment");
        }
    }

    // 7. Create a buffer that's too small to see the error message
    let small_buffer = vec![0u8; expected_size - 1];
    match PoolSwapTLV::from_bytes(&small_buffer) {
        Ok(_) => {
            println!("❌ **UNEXPECTED**: Small buffer parsing succeeded!");
        }
        Err(e) => {
            println!("✅ Small buffer correctly rejected: {}", e);
        }
    }

    // 8. Create a buffer that's too large to see if it works
    let mut large_buffer = vec![0u8; expected_size + 50];
    large_buffer[0..serialized.len()].copy_from_slice(serialized);
    match PoolSwapTLV::from_bytes(&large_buffer[0..expected_size]) {
        Ok(_) => {
            println!("✅ Large buffer (truncated to correct size) works");
        }
        Err(e) => {
            println!("❌ Large buffer (truncated) failed: {}", e);
        }
    }

    println!("\n📋 **PARSING RECOMMENDATIONS**");
    println!("1. Ensure TLV payload is exactly {} bytes", expected_size);
    println!("2. Check that the payload contains valid PoolSwapTLV data structure");
    println!("3. Verify the data is properly aligned in memory");
    println!("4. Consider using try_from_bytes() for better error handling");

    println!("\n💡 **POTENTIAL FIXES FOR RELAY CONSUMER**");
    println!("- Add detailed logging of payload size and alignment");
    println!("- Use PoolSwapTLV::try_from_bytes() instead of from_bytes()");
    println!("- Implement alignment correction if needed");
    println!("- Add hex dump of first/last bytes for debugging");

    Ok(())
}
