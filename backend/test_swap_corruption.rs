use alphapulse_protocol::{MessageHeader, MessageType, SwapEventMessage, MAGIC_BYTE};
use zerocopy::AsBytes;
use std::fs::File;
use std::io::Write;

fn main() {
    // Create a SwapEvent message similar to what would be sent
    let mut swap_msg = SwapEventMessage::new_v2(
        1700000000000000000,  // timestamp_ns
        0x123456789ABCDEF0,   // pool_hash
        0x1111111111111111,   // token0_hash (WETH)
        0x2222222222222222,   // token1_hash (USDC)
        100000000,            // amount0_in (1.0 with 8 decimals)
        0,                    // amount1_in
        0,                    // amount0_out
        200000000,            // amount1_out (2.0 with 8 decimals)
    );
    
    // Create header
    let header = MessageHeader::new(MessageType::SwapEvent, SwapEventMessage::SIZE as u16, 1);
    
    // Convert to bytes
    let header_bytes = AsBytes::as_bytes(&header);
    let swap_bytes = AsBytes::as_bytes(&swap_msg);
    
    println!("Header size: {} bytes", header_bytes.len());
    println!("Swap size: {} bytes", swap_bytes.len());
    println!("Total size: {} bytes", header_bytes.len() + swap_bytes.len());
    println!("Expected: 8 + 128 = 136 bytes");
    
    // Check header magic
    println!("\nHeader bytes (first 8):");
    for (i, b) in header_bytes.iter().enumerate() {
        print!("{:02x} ", b);
    }
    println!();
    
    if header_bytes[0] != MAGIC_BYTE {
        println!("ERROR: Wrong magic byte! Expected 0xFE, got 0x{:02x}", header_bytes[0]);
    }
    
    // Check swap message structure
    println!("\nSwap bytes (first 32):");
    for (i, b) in swap_bytes.iter().take(32).enumerate() {
        print!("{:02x} ", b);
        if (i + 1) % 8 == 0 {
            println!();
        }
    }
    
    // Write to file for inspection
    let mut file = File::create("/tmp/swap_message.bin").unwrap();
    file.write_all(header_bytes).unwrap();
    file.write_all(swap_bytes).unwrap();
    println!("\nWrote binary message to /tmp/swap_message.bin");
    
    // Now test multiple messages to see if corruption happens
    println!("\n=== Testing multiple message sequence ===");
    let mut buffer = Vec::new();
    
    for seq in 1..=5 {
        let header = MessageHeader::new(MessageType::SwapEvent, SwapEventMessage::SIZE as u16, seq);
        buffer.extend_from_slice(AsBytes::as_bytes(&header));
        buffer.extend_from_slice(swap_bytes);
        println!("Added message #{}, buffer size: {} bytes", seq, buffer.len());
    }
    
    // Verify buffer integrity
    let mut offset = 0;
    while offset < buffer.len() {
        if buffer[offset] != MAGIC_BYTE {
            println!("ERROR at offset {}: Wrong magic byte 0x{:02x}", offset, buffer[offset]);
            println!("Context: {:02x?}", &buffer[offset.saturating_sub(8)..std::cmp::min(offset + 16, buffer.len())]);
            break;
        }
        offset += 136; // Move to next message
    }
    
    println!("\nTest complete.");
}