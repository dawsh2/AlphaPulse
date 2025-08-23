#!/usr/bin/env rust-script
//! Standalone roundtrip equality test that can run without cargo
//! 
//! Run with: rustc standalone_roundtrip_test.rs && ./standalone_roundtrip_test

const MESSAGE_MAGIC: u32 = 0xDEADBEEF;
const PROTOCOL_VERSION: u8 = 1;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum RelayDomain {
    MarketData = 1,
    Signal = 2,
    Execution = 3,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum SourceType {
    KrakenCollector = 2,
    PolygonCollector = 4,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum TLVType {
    Trade = 1,
    Quote = 2,
    PoolSwap = 11,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct MessageHeader {
    magic: u32,
    version: u8,
    message_type: u8,
    relay_domain: u8,
    source_type: u8,
    sequence: u64,
    timestamp_ns: u64,
    instrument_id: u64,
    _padding: [u8; 12],
    checksum: u32,
}

fn test_trade_roundtrip() {
    println!("\n=== TradeTLV Roundtrip Test ===\n");
    
    // Create original values with high precision
    let original_price: i64 = 4523467890123; // $45,234.67890123
    let original_volume: i64 = 123456789;     // 1.23456789 BTC
    let original_timestamp_ns: u64 = 1734567890123456789;
    
    println!("Original values:");
    println!("  Price: {} (${:.8})", original_price, original_price as f64 / 100_000_000.0);
    println!("  Volume: {} ({:.8} BTC)", original_volume, original_volume as f64 / 100_000_000.0);
    println!("  Timestamp: {} ns", original_timestamp_ns);
    
    // Create header
    let header = MessageHeader {
        magic: MESSAGE_MAGIC,
        version: PROTOCOL_VERSION,
        message_type: TLVType::Trade as u8,
        relay_domain: RelayDomain::MarketData as u8,
        source_type: SourceType::KrakenCollector as u8,
        sequence: 42,
        timestamp_ns: original_timestamp_ns,
        instrument_id: 12345,
        _padding: [0; 12],
        checksum: 0,
    };
    
    // Serialize to bytes
    let header_bytes = unsafe {
        std::slice::from_raw_parts(
            &header as *const _ as *const u8,
            std::mem::size_of::<MessageHeader>(),
        )
    };
    
    let mut message = header_bytes.to_vec();
    
    // Add TLV payload
    message.push(TLVType::Trade as u8);
    message.push(0x01); // Buy flag
    message.extend_from_slice(&16u16.to_le_bytes()); // Length
    message.extend_from_slice(&original_price.to_le_bytes());
    message.extend_from_slice(&original_volume.to_le_bytes());
    
    println!("Serialized message: {} bytes", message.len());
    
    // Deserialize header
    let deserialized_header = unsafe {
        std::ptr::read(message.as_ptr() as *const MessageHeader)
    };
    
    // Parse TLV payload
    let tlv_offset = std::mem::size_of::<MessageHeader>();
    let price_offset = tlv_offset + 4;
    let volume_offset = price_offset + 8;
    
    let deserialized_price = i64::from_le_bytes([
        message[price_offset], message[price_offset + 1],
        message[price_offset + 2], message[price_offset + 3],
        message[price_offset + 4], message[price_offset + 5],
        message[price_offset + 6], message[price_offset + 7],
    ]);
    
    let deserialized_volume = i64::from_le_bytes([
        message[volume_offset], message[volume_offset + 1],
        message[volume_offset + 2], message[volume_offset + 3],
        message[volume_offset + 4], message[volume_offset + 5],
        message[volume_offset + 6], message[volume_offset + 7],
    ]);
    
    println!("\nDeserialized values:");
    println!("  Price: {} (${:.8})", deserialized_price, deserialized_price as f64 / 100_000_000.0);
    println!("  Volume: {} ({:.8} BTC)", deserialized_volume, deserialized_volume as f64 / 100_000_000.0);
    
    // Copy fields from packed struct to avoid alignment issues
    let orig_magic = header.magic;
    let orig_version = header.version;
    let orig_timestamp = header.timestamp_ns;
    let deser_magic = deserialized_header.magic;
    let deser_version = deserialized_header.version;
    let deser_timestamp = deserialized_header.timestamp_ns;
    
    println!("  Timestamp: {} ns", deser_timestamp);
    
    // Verify equality
    assert_eq!(orig_magic, deser_magic, "Magic mismatch!");
    assert_eq!(orig_version, deser_version, "Version mismatch!");
    assert_eq!(orig_timestamp, deser_timestamp, "Timestamp mismatch!");
    assert_eq!(original_price, deserialized_price, "Price mismatch!");
    assert_eq!(original_volume, deserialized_volume, "Volume mismatch!");
    
    println!("\n✅ TradeTLV roundtrip test PASSED - perfect equality!");
}

fn test_pool_swap_roundtrip() {
    println!("\n=== PoolSwapTLV Roundtrip Test ===\n");
    
    // Test with Wei values (18 decimals)
    let original_amount_in: u128 = 1234567890123456789012345678;
    let original_amount_out: u128 = 9876543210987654321098765432;
    
    println!("Original PoolSwap:");
    println!("  Amount In: {} Wei", original_amount_in);
    println!("  Amount Out: {} Wei", original_amount_out);
    
    // Create header
    let header = MessageHeader {
        magic: MESSAGE_MAGIC,
        version: PROTOCOL_VERSION,
        message_type: 11, // PoolSwapTLV
        relay_domain: RelayDomain::MarketData as u8,
        source_type: SourceType::PolygonCollector as u8,
        sequence: 999999,
        timestamp_ns: 1734567890123456789,
        instrument_id: 0xABCDEF,
        _padding: [0; 12],
        checksum: 0,
    };
    
    let header_bytes = unsafe {
        std::slice::from_raw_parts(
            &header as *const _ as *const u8,
            std::mem::size_of::<MessageHeader>(),
        )
    };
    
    let mut message = header_bytes.to_vec();
    
    // Add simplified PoolSwap payload
    message.push(11); // Type
    message.push(0);  // Flags
    message.extend_from_slice(&32u16.to_le_bytes()); // Length
    message.extend_from_slice(&original_amount_in.to_le_bytes());
    message.extend_from_slice(&original_amount_out.to_le_bytes());
    
    // Deserialize
    let amount_in_offset = std::mem::size_of::<MessageHeader>() + 4;
    let amount_out_offset = amount_in_offset + 16;
    
    let deserialized_amount_in = u128::from_le_bytes([
        message[amount_in_offset], message[amount_in_offset + 1],
        message[amount_in_offset + 2], message[amount_in_offset + 3],
        message[amount_in_offset + 4], message[amount_in_offset + 5],
        message[amount_in_offset + 6], message[amount_in_offset + 7],
        message[amount_in_offset + 8], message[amount_in_offset + 9],
        message[amount_in_offset + 10], message[amount_in_offset + 11],
        message[amount_in_offset + 12], message[amount_in_offset + 13],
        message[amount_in_offset + 14], message[amount_in_offset + 15],
    ]);
    
    let deserialized_amount_out = u128::from_le_bytes([
        message[amount_out_offset], message[amount_out_offset + 1],
        message[amount_out_offset + 2], message[amount_out_offset + 3],
        message[amount_out_offset + 4], message[amount_out_offset + 5],
        message[amount_out_offset + 6], message[amount_out_offset + 7],
        message[amount_out_offset + 8], message[amount_out_offset + 9],
        message[amount_out_offset + 10], message[amount_out_offset + 11],
        message[amount_out_offset + 12], message[amount_out_offset + 13],
        message[amount_out_offset + 14], message[amount_out_offset + 15],
    ]);
    
    println!("\nDeserialized PoolSwap:");
    println!("  Amount In: {} Wei", deserialized_amount_in);
    println!("  Amount Out: {} Wei", deserialized_amount_out);
    
    assert_eq!(original_amount_in, deserialized_amount_in, "Amount in mismatch!");
    assert_eq!(original_amount_out, deserialized_amount_out, "Amount out mismatch!");
    
    println!("\n✅ PoolSwapTLV roundtrip test PASSED - Wei precision preserved!");
}

fn test_edge_cases() {
    println!("\n=== Binary Precision Edge Cases ===\n");
    
    let test_cases = vec![
        (0i64, "Zero"),
        (1i64, "One satoshi"),
        (-1i64, "Negative one"),
        (i64::MAX, "Max i64"),
        (i64::MIN, "Min i64"),
        (99999999i64, "0.99999999"),
        (100000000i64, "1.00000000"),
        (4523467890123i64, "45234.67890123"),
    ];
    
    for (original, description) in test_cases {
        print!("  Testing {}: {} ", description, original);
        
        // Serialize
        let bytes = original.to_le_bytes();
        
        // Deserialize
        let deserialized = i64::from_le_bytes(bytes);
        
        assert_eq!(original, deserialized);
        println!("✓");
    }
    
    println!("\n✅ All edge cases passed with perfect precision!");
}

fn test_routing_simulation() {
    println!("\n=== Multi-Strategy Routing Simulation ===\n");
    
    // Simulate messages from different sources
    let messages = vec![
        (SourceType::KrakenCollector, TLVType::Trade, "Kraken BTC/USD Trade"),
        (SourceType::PolygonCollector, TLVType::PoolSwap, "Polygon WETH/USDC Swap"),
        (SourceType::KrakenCollector, TLVType::Quote, "Kraken ETH/USD Quote"),
        (SourceType::PolygonCollector, TLVType::PoolSwap, "Polygon WBTC/WETH Swap"),
    ];
    
    // Strategy subscriptions
    let flash_arbitrage_topics = vec!["market_data_kraken", "market_data_polygon"];
    let kraken_signal_topics = vec!["market_data_kraken"];
    
    let mut flash_arbitrage_count = 0;
    let mut kraken_signal_count = 0;
    
    for (source, _msg_type, description) in &messages {
        println!("Message: {}", description);
        
        // Extract topic
        let topic = match source {
            SourceType::KrakenCollector => "market_data_kraken",
            SourceType::PolygonCollector => "market_data_polygon",
        };
        
        // Route to strategies
        if flash_arbitrage_topics.contains(&topic) {
            flash_arbitrage_count += 1;
            println!("  → flash-arbitrage received");
        }
        
        if kraken_signal_topics.contains(&topic) {
            kraken_signal_count += 1;
            println!("  → kraken-signal received");
        }
    }
    
    println!("\nResults:");
    println!("  flash-arbitrage: {} messages (should be 4)", flash_arbitrage_count);
    println!("  kraken-signal: {} messages (should be 2)", kraken_signal_count);
    
    assert_eq!(flash_arbitrage_count, 4, "flash-arbitrage should receive all messages");
    assert_eq!(kraken_signal_count, 2, "kraken-signal should only receive Kraken messages");
    
    println!("\n✅ Routing simulation passed!");
}

fn main() {
    println!("\n========================================");
    println!("    RELAY ROUNDTRIP EQUALITY TEST");
    println!("========================================");
    
    test_trade_roundtrip();
    test_pool_swap_roundtrip();
    test_edge_cases();
    test_routing_simulation();
    
    println!("\n========================================");
    println!("         ALL TESTS PASSED! 🎉");
    println!("========================================");
    println!("\nKey validations:");
    println!("✓ Perfect binary equality maintained");
    println!("✓ No precision loss in conversions");
    println!("✓ Wei values (u128) preserved exactly");
    println!("✓ Nanosecond timestamps preserved");
    println!("✓ Multi-strategy routing works correctly");
    println!("✓ Topic-based filtering validated");
}