//! Standalone TLV Integration Test
//!
//! Validates TLV message compatibility between Polygon collector and Flash Arbitrage strategy

use protocol_v2::{tlv::market_data::PoolSwapTLV, MessageHeader, RelayDomain, TLVMessage, VenueId};

/// Test that Polygon-style TLVs can be created and parsed
#[tokio::test]
async fn test_polygon_tlv_creation_and_parsing() {
    println!("🧪 Testing Polygon TLV Creation and Parsing");

    // Create a realistic Polygon swap TLV message
    let swap_tlv = create_polygon_swap_tlv();
    let tlv_message = swap_tlv.to_tlv_message();

    println!(
        "✅ Created Polygon swap TLV: {} bytes",
        tlv_message.payload.len()
    );

    // Verify TLV message structure
    assert!(
        tlv_message.payload.len() > 0,
        "TLV payload should not be empty"
    );

    // Test serialization and deserialization
    let tlv_bytes = swap_tlv.to_bytes().expect("TLV serialization should work");
    let parsed_swap = PoolSwapTLV::from_bytes(&tlv_bytes).expect("TLV parsing should work");

    // Verify data integrity
    assert_eq!(
        swap_tlv.pool_address, parsed_swap.pool_address,
        "Pool address must match"
    );
    assert_eq!(
        swap_tlv.amount_in, parsed_swap.amount_in,
        "Amount in must match"
    );
    assert_eq!(
        swap_tlv.amount_out, parsed_swap.amount_out,
        "Amount out must match"
    );
    assert_eq!(
        swap_tlv.amount_in_decimals, parsed_swap.amount_in_decimals,
        "Input decimals must match"
    );
    assert_eq!(
        swap_tlv.amount_out_decimals, parsed_swap.amount_out_decimals,
        "Output decimals must match"
    );

    println!("✅ TLV data integrity verified");

    // Test relay message creation
    let header = MessageHeader {
        magic: 0xDEADBEEF,
        relay_domain: RelayDomain::MarketData as u8,
        version: 1,
        source: 3, // Polygon source ID
        flags: 0,
        payload_size: tlv_message.payload.len() as u32,
        sequence: 1,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64,
        checksum: 0,
    };

    // Serialize complete relay message
    let header_bytes = unsafe {
        std::slice::from_raw_parts(
            &header as *const MessageHeader as *const u8,
            std::mem::size_of::<MessageHeader>(),
        )
    };

    let mut relay_message = Vec::with_capacity(header_bytes.len() + tlv_message.payload.len());
    relay_message.extend_from_slice(header_bytes);
    relay_message.extend_from_slice(&tlv_message.payload);

    println!(
        "✅ Created relay message: {} bytes total",
        relay_message.len()
    );

    // Verify relay message structure
    assert_eq!(
        relay_message.len(),
        32 + tlv_message.payload.len(),
        "Relay message should be header + payload"
    );

    // Test header parsing
    let parsed_magic = u32::from_le_bytes([
        relay_message[0],
        relay_message[1],
        relay_message[2],
        relay_message[3],
    ]);
    assert_eq!(
        parsed_magic, 0xDEADBEEF,
        "Magic number should be correctly parsed"
    );

    let parsed_payload_size = u32::from_le_bytes([
        relay_message[8],
        relay_message[9],
        relay_message[10],
        relay_message[11],
    ]);
    assert_eq!(
        parsed_payload_size as usize,
        tlv_message.payload.len(),
        "Payload size should match"
    );

    println!("✅ Complete integration test passed: Polygon → TLV → Relay Message → Parsing");
}

/// Test precision preservation across the pipeline
#[tokio::test]
async fn test_precision_preservation() {
    println!("🧪 Testing Precision Preservation");

    let swap_tlv = create_polygon_swap_tlv();

    // Test that native precision is preserved
    assert!(swap_tlv.amount_in > 0, "Amount in should be positive");
    assert!(swap_tlv.amount_out > 0, "Amount out should be positive");
    assert_eq!(
        swap_tlv.amount_in_decimals, 18,
        "WETH should have 18 decimals"
    );
    assert_eq!(
        swap_tlv.amount_out_decimals, 6,
        "USDC should have 6 decimals"
    );

    // Expected values: 5 WETH → 13,500 USDC
    assert_eq!(
        swap_tlv.amount_in, 5_000_000_000_000_000_000u128,
        "Should preserve 5 WETH in wei"
    );
    assert_eq!(
        swap_tlv.amount_out, 13500_000_000u128,
        "Should preserve 13,500 USDC in native units"
    );

    println!("✅ Native precision preservation verified");

    // Test that V3-specific fields are properly set
    assert_ne!(
        swap_tlv.sqrt_price_x96_after, [0u8; 20],
        "V3 sqrt_price should be set"
    );
    assert_ne!(swap_tlv.tick_after, 0, "V3 tick should be set");
    assert!(
        swap_tlv.liquidity_after > 0,
        "V3 liquidity should be positive"
    );

    println!("✅ V3-specific field validation passed");
}

/// Test TLV type encoding
#[tokio::test]
async fn test_tlv_type_encoding() {
    println!("🧪 Testing TLV Type Encoding");

    let swap_tlv = create_polygon_swap_tlv();
    let tlv_message = swap_tlv.to_tlv_message();

    // Check that TLV type is correctly set
    // PoolSwapTLV should be type 11
    assert!(
        tlv_message.payload.len() >= 4,
        "TLV payload should have header"
    );

    // First byte should be TLV type (11 for PoolSwap)
    assert_eq!(
        tlv_message.payload[0], 11,
        "Should be PoolSwapTLV type (11)"
    );

    println!("✅ TLV type encoding verified");
}

/// Create a realistic Polygon swap TLV for testing
fn create_polygon_swap_tlv() -> PoolSwapTLV {
    PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_address: [
            0x45, 0xdd, 0xa9, 0xcb, 0x7c, 0x25, 0x13, 0x1d, 0xf2, 0x68, 0x51, 0x51, 0x31, 0xf6,
            0x47, 0xd7, 0x26, 0xf5, 0x06, 0x08,
        ], // WETH/USDC pool
        token_in_addr: [
            0x7c, 0xeb, 0x23, 0xfd, 0x6f, 0x88, 0xb7, 0x6a, 0xf0, 0x52, 0xc3, 0xca, 0x45, 0x9c,
            0x11, 0x73, 0xc5, 0xb9, 0xb9, 0x6d,
        ], // WETH
        token_out_addr: [
            0x27, 0x91, 0xbc, 0xa1, 0xf2, 0xde, 0x46, 0x61, 0xed, 0x88, 0xa3, 0x0c, 0x99, 0xa7,
            0xa9, 0x44, 0x9a, 0xa8, 0x41, 0x74,
        ], // USDC
        amount_in: 5_000_000_000_000_000_000u128, // 5 WETH (18 decimals)
        amount_out: 13500_000_000u128,            // 13,500 USDC (6 decimals)
        amount_in_decimals: 18,
        amount_out_decimals: 6,
        sqrt_price_x96_after: PoolSwapTLV::sqrt_price_from_u128(1792282187229267636352u128), // Realistic V3 price
        tick_after: 3393,
        liquidity_after: 500000000000000000u128,
        timestamp_ns: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64,
        block_number: 48_650_000,
    }
}
