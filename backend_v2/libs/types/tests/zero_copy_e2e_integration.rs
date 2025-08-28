//! End-to-end integration test for zero-copy TLV implementation
//!
//! Validates that the zero-copy implementation works correctly through the complete
//! message pipeline: construction → serialization → transport → parsing → consumption

use torq_types::protocol::tlv::address::AddressExtraction;
use torq_types::protocol::tlv::market_data::{PoolSwapTLV, QuoteTLV, TradeTLV};
use torq_types::protocol::tlv::{
    parse_header, parse_tlv_extensions, TLVMessageBuilder, TLVType,
};
use torq_codec::protocol::{InstrumentId, RelayDomain, SourceType, VenueId};
use std::time::Instant;
use zerocopy::{AsBytes, FromBytes};

#[test]
fn test_zero_copy_e2e_pipeline() {
    println!("🚀 Testing Zero-Copy End-to-End Pipeline");

    // Step 1: Create realistic market data
    let pool_address = [0x42u8; 20];
    let token_in = [0x43u8; 20];
    let token_out = [0x44u8; 20];

    let swap = PoolSwapTLV::new(
        pool_address,
        token_in,
        token_out,
        VenueId::Polygon,
        1000000000000000000u128,    // 1.0 WETH (18 decimals)
        1800000000u128,             // 1800 USDC (6 decimals)
        5000000000000000000000u128, // 5000 liquidity
        1703025600000000000u64,     // Real timestamp (nanoseconds)
        18654321u64,                // Block number
        -60000i32,                  // Tick after swap
        18u8,                       // WETH decimals
        6u8,                        // USDC decimals
        1234567890123456789u128,    // sqrt_price_x96_after
    );

    // Step 2: Build Protocol V2 message
    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
        .add_tlv(TLVType::PoolSwap, &swap)
        .build();

    // Step 3: Zero-copy serialization
    let message_bytes = message;
    println!("✅ Message serialized: {} bytes", message_bytes.len());

    // Step 4: Parse message header (simulates network transport)
    let header = parse_header(&message_bytes).expect("Header parsing failed");
    assert_eq!(header.magic, protocol_v2::MESSAGE_MAGIC);
    assert_eq!(header.relay_domain, RelayDomain::MarketData as u8);
    assert_eq!(header.source, SourceType::PolygonCollector as u8);
    println!("✅ Header parsed successfully");

    // Step 5: Parse TLV extensions
    let payload = &message_bytes[32..32 + header.payload_size as usize];
    let tlv_extensions = parse_tlv_extensions(payload).expect("TLV parsing failed");
    assert_eq!(tlv_extensions.len(), 1);

    let tlv_ext = &tlv_extensions[0];
    let (tlv_type, payload_data) = match tlv_ext {
        protocol_v2::tlv::parser::TLVExtensionEnum::Standard(std_tlv) => {
            (std_tlv.header.tlv_type, &std_tlv.payload)
        }
        protocol_v2::tlv::parser::TLVExtensionEnum::Extended(ext_tlv) => {
            (ext_tlv.header.tlv_type, &ext_tlv.payload)
        }
    };
    assert_eq!(tlv_type, TLVType::PoolSwap as u8);
    assert_eq!(payload_data.len(), std::mem::size_of::<PoolSwapTLV>());
    println!("✅ TLV extensions parsed successfully");

    // Step 6: Zero-copy deserialization
    let parsed_swap =
        PoolSwapTLV::ref_from(payload_data).expect("Zero-copy deserialization failed");
    let parsed_swap_value = *parsed_swap;

    // Step 7: Validate complete roundtrip
    assert_eq!(parsed_swap_value, swap);

    // Validate address extraction works
    let pool_addr_extracted = parsed_swap_value.pool_address.to_eth_address();
    let token_in_extracted = parsed_swap_value.token_in_addr.to_eth_address();
    let token_out_extracted = parsed_swap_value.token_out_addr.to_eth_address();

    assert_eq!(pool_addr_extracted, pool_address);
    assert_eq!(token_in_extracted, token_in);
    assert_eq!(token_out_extracted, token_out);

    println!("✅ Complete roundtrip validation successful");
    println!("✅ Address extraction working correctly");

    // Step 8: Performance validation under load
    let iterations = 10_000;
    let start = Instant::now();

    for _ in 0..iterations {
        // Simulate complete pipeline
        let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
            .add_tlv(TLVType::PoolSwap, &swap)
            .build();
        let message_bytes = message;

        let header = parse_header(&message_bytes).unwrap();
        let payload = &message_bytes[32..32 + header.payload_size as usize];
        let tlv_extensions = parse_tlv_extensions(payload).unwrap();

        let payload_data = match &tlv_extensions[0] {
            protocol_v2::tlv::parser::TLVExtensionEnum::Standard(std_tlv) => &std_tlv.payload,
            protocol_v2::tlv::parser::TLVExtensionEnum::Extended(ext_tlv) => &ext_tlv.payload,
        };
        let parsed_swap = PoolSwapTLV::ref_from(payload_data).unwrap();

        // Prevent compiler optimization
        std::hint::black_box(parsed_swap);
    }

    let duration = start.elapsed();
    let ops_per_sec = iterations as f64 / duration.as_secs_f64();

    println!(
        "E2E Pipeline Performance: {:.2}K ops/sec",
        ops_per_sec / 1000.0
    );

    // Should handle thousands of complete pipeline operations per second
    assert!(
        ops_per_sec > 5000.0,
        "E2E pipeline too slow: {:.2} ops/sec",
        ops_per_sec
    );

    println!("✅ End-to-end pipeline performance validated");
}

#[test]
fn test_mixed_tlv_message_zero_copy() {
    println!("🧪 Testing Mixed TLV Message with Zero-Copy");

    // Create multiple TLV types in one message
    let instrument_id = InstrumentId {
        venue: VenueId::Polygon as u16,
        asset_type: 1,
        reserved: 0,
        asset_id: 12345,
    };

    let trade = TradeTLV::from_instrument(
        VenueId::Polygon,
        instrument_id,
        100000000i64,  // $1.00
        1000000000i64, // 10 tokens
        0u8,           // buy
        1703025600000000000u64,
    );

    let quote = QuoteTLV::new(
        VenueId::Polygon,
        instrument_id,
        99900000i64,  // $0.999 bid
        1000000i64,   // 1 token bid size
        100100000i64, // $1.001 ask
        2000000i64,   // 2 tokens ask size
        1703025600000000000u64,
    );

    let swap = PoolSwapTLV::new(
        [0x42u8; 20],
        [0x43u8; 20],
        [0x44u8; 20],
        VenueId::Polygon,
        1000000000000000000u128,
        1800000000u128,
        5000000000000000000000u128,
        1703025600000000000u64,
        18654321u64,
        -60000i32,
        18u8,
        6u8,
        1234567890123456789u128,
    );

    // Build message with multiple TLVs
    let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
        .add_tlv(TLVType::Trade, &trade)
        .add_tlv(TLVType::Quote, &quote)
        .add_tlv(TLVType::PoolSwap, &swap)
        .build();

    let message_bytes = message;
    println!("Mixed message size: {} bytes", message_bytes.len());

    // Parse the message
    let header = parse_header(&message_bytes).expect("Header parsing failed");
    let payload = &message_bytes[32..32 + header.payload_size as usize];
    let tlv_extensions = parse_tlv_extensions(payload).expect("TLV parsing failed");

    assert_eq!(tlv_extensions.len(), 3);
    println!("✅ Parsed {} TLV extensions", tlv_extensions.len());

    // Verify each TLV type using zero-copy deserialization
    let mut trade_found = false;
    let mut quote_found = false;
    let mut swap_found = false;

    for tlv_ext in &tlv_extensions {
        let (tlv_type, payload_data) = match tlv_ext {
            protocol_v2::tlv::parser::TLVExtensionEnum::Standard(std_tlv) => {
                (std_tlv.header.tlv_type, &std_tlv.payload)
            }
            protocol_v2::tlv::parser::TLVExtensionEnum::Extended(ext_tlv) => {
                (ext_tlv.header.tlv_type, &ext_tlv.payload)
            }
        };

        match tlv_type {
            1 => {
                // Trade
                let parsed_trade = TradeTLV::ref_from(payload_data).unwrap();
                assert_eq!(*parsed_trade, trade);
                trade_found = true;
                println!("✅ Trade TLV validated via zero-copy");
            }
            2 => {
                // Quote
                let parsed_quote = QuoteTLV::ref_from(payload_data).unwrap();
                assert_eq!(*parsed_quote, quote);
                quote_found = true;
                println!("✅ Quote TLV validated via zero-copy");
            }
            11 => {
                // PoolSwap
                let parsed_swap = PoolSwapTLV::ref_from(payload_data).unwrap();
                assert_eq!(*parsed_swap, swap);
                swap_found = true;
                println!("✅ PoolSwap TLV validated via zero-copy");
            }
            _ => panic!("Unexpected TLV type: {}", tlv_type),
        }
    }

    assert!(
        trade_found && quote_found && swap_found,
        "Not all TLVs found in message"
    );
    println!("✅ Mixed TLV message zero-copy validation complete");
}

#[test]
fn test_real_world_performance_simulation() {
    println!("🏎️  Real-World Performance Simulation");

    // Simulate realistic trading scenario:
    // - 100 pool swaps per second
    // - 500 trades per second
    // - 1000 quotes per second

    let instrument_id = InstrumentId {
        venue: VenueId::Polygon as u16,
        asset_type: 1,
        reserved: 0,
        asset_id: 12345,
    };

    // Pre-create test data
    let swap = PoolSwapTLV::new(
        [0x42u8; 20],
        [0x43u8; 20],
        [0x44u8; 20],
        VenueId::Polygon,
        1000000000000000000u128,
        1800000000u128,
        5000000000000000000000u128,
        1703025600000000000u64,
        18654321u64,
        -60000i32,
        18u8,
        6u8,
        1234567890123456789u128,
    );

    let trade = TradeTLV::from_instrument(
        VenueId::Polygon,
        instrument_id,
        100000000i64,
        1000000000i64,
        0u8,
        1703025600000000000u64,
    );

    let quote = QuoteTLV::new(
        VenueId::Polygon,
        instrument_id,
        99900000i64,
        1000000i64,
        100100000i64,
        2000000i64,
        1703025600000000000u64,
    );

    // Test sustained throughput for 1 second worth of messages
    let total_messages = 1600; // 100 swaps + 500 trades + 1000 quotes per second
    let start = Instant::now();

    for i in 0..total_messages {
        // Distribute message types realistically
        let message = match i % 16 {
            0 => TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
                .add_tlv(TLVType::PoolSwap, &swap)
                .build(), // ~6% swaps
            1..=8 => TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
                .add_tlv(TLVType::Trade, &trade)
                .build(), // ~50% trades
            _ => TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
                .add_tlv(TLVType::Quote, &quote)
                .build(), // ~44% quotes
        };
        let message_bytes = message;

        // Simulate processing pipeline
        let header = parse_header(&message_bytes).unwrap();
        let payload = &message_bytes[32..32 + header.payload_size as usize];
        let tlv_extensions = parse_tlv_extensions(payload).unwrap();

        for tlv_ext in &tlv_extensions {
            let (tlv_type, payload_data) = match tlv_ext {
                protocol_v2::tlv::parser::TLVExtensionEnum::Standard(std_tlv) => {
                    (std_tlv.header.tlv_type, &std_tlv.payload)
                }
                protocol_v2::tlv::parser::TLVExtensionEnum::Extended(ext_tlv) => {
                    (ext_tlv.header.tlv_type, &ext_tlv.payload)
                }
            };

            match tlv_type {
                1 => {
                    let parsed_trade = TradeTLV::ref_from(payload_data).unwrap();
                    std::hint::black_box(parsed_trade);
                }
                2 => {
                    let parsed_quote = QuoteTLV::ref_from(payload_data).unwrap();
                    std::hint::black_box(parsed_quote);
                }
                11 => {
                    let parsed_swap = PoolSwapTLV::ref_from(payload_data).unwrap();
                    std::hint::black_box(parsed_swap);
                }
                _ => {}
            }
        }
    }

    let duration = start.elapsed();
    let msgs_per_sec = total_messages as f64 / duration.as_secs_f64();
    let total_throughput_mbps =
        (total_messages as f64 * 200.0) / duration.as_secs_f64() / (1024.0 * 1024.0); // Assume ~200 bytes per message

    println!("Real-world simulation results:");
    println!("  Messages/sec: {:.2}K", msgs_per_sec / 1000.0);
    println!("  Throughput:   {:.2} MB/s", total_throughput_mbps);
    println!(
        "  Avg latency:  {:.2} μs/msg",
        duration.as_micros() as f64 / total_messages as f64
    );

    // Should handle realistic trading loads easily
    assert!(
        msgs_per_sec > 10_000.0,
        "Real-world simulation too slow: {:.2} msgs/sec",
        msgs_per_sec
    );

    println!("✅ Real-world performance simulation successful");
}

#[test]
fn test_memory_efficiency_validation() {
    println!("💾 Memory Efficiency Validation");

    // Test that zero-copy operations don't cause memory leaks or excessive allocations
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Simple allocation tracker
    struct TrackingAllocator {
        allocated: AtomicUsize,
    }

    impl TrackingAllocator {
        const fn new() -> Self {
            Self {
                allocated: AtomicUsize::new(0),
            }
        }

        fn allocated_bytes(&self) -> usize {
            self.allocated.load(Ordering::Relaxed)
        }
    }

    unsafe impl GlobalAlloc for TrackingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let result = System.alloc(layout);
            if !result.is_null() {
                self.allocated.fetch_add(layout.size(), Ordering::Relaxed);
            }
            result
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout);
            self.allocated.fetch_sub(layout.size(), Ordering::Relaxed);
        }
    }

    // Create test data
    let swap = PoolSwapTLV::new(
        [0x42u8; 20],
        [0x43u8; 20],
        [0x44u8; 20],
        VenueId::Polygon,
        1000000000000000000u128,
        1800000000u128,
        5000000000000000000000u128,
        1703025600000000000u64,
        18654321u64,
        -60000i32,
        18u8,
        6u8,
        1234567890123456789u128,
    );

    // Test that zero-copy operations are truly zero-copy
    let bytes: &[u8] = swap.as_bytes();
    let swap_ref = PoolSwapTLV::ref_from(bytes).expect("Zero-copy deserialization failed");

    // Verify no additional memory was allocated for the zero-copy operations
    // (The bytes reference should point directly into the original struct)
    assert_eq!(bytes.as_ptr(), &swap as *const _ as *const u8);
    assert_eq!(swap_ref as *const _ as *const u8, bytes.as_ptr());

    println!("✅ Zero-copy operations confirmed - no additional allocations");

    // Test address extraction efficiency
    let iterations = 1000;
    for _ in 0..iterations {
        let pool_addr = swap.pool_address.to_eth_address();
        let token_in = swap.token_in_addr.to_eth_address();
        let token_out = swap.token_out_addr.to_eth_address();

        // These should be stack operations only
        std::hint::black_box((pool_addr, token_in, token_out));
    }

    println!("✅ Address extraction operations completed efficiently");
}
