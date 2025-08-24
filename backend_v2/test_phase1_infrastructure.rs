// Quick test to validate Phase 1 hot path buffer infrastructure

use std::time::Instant;

#[path = "protocol_v2/src/lib.rs"]
mod protocol_v2;

use protocol_v2::tlv::{
    with_hot_path_buffer, with_signal_buffer, ZeroCopyTLVMessageBuilder,
    TradeTLV, TLVType,
};
use protocol_v2::{RelayDomain, SourceType, VenueId, InstrumentId};

fn main() {
    println!("Testing Phase 1: Thread-Local Buffer Infrastructure");
    
    // Test 1: Basic buffer functionality
    println!("\n1. Testing basic hot path buffer...");
    let result = with_hot_path_buffer(|buffer| {
        buffer[0] = 0xDE;
        buffer[1] = 0xAD; 
        buffer[2] = 0xBE;
        buffer[3] = 0xEF;
        Ok((42, 4))
    });
    assert_eq!(result.unwrap(), 42);
    println!("✓ Basic buffer operations work");

    // Test 2: Zero-copy message construction
    println!("\n2. Testing zero-copy message construction...");
    let trade = TradeTLV::new(
        VenueId::Polygon,
        InstrumentId {
            venue: VenueId::Polygon as u16,
            asset_type: 1,
            reserved: 0,
            asset_id: 12345,
        },
        100_000_000,  // price
        50_000_000,   // volume
        0,            // side (buy)
        1234567890,   // timestamp
    );

    let message_bytes = ZeroCopyTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
        .add_tlv_ref(TLVType::Trade, &trade)
        .build_with_hot_path_buffer()
        .unwrap();

    assert!(!message_bytes.is_empty());
    assert!(message_bytes.len() > 32); // Header + TLV payload
    println!("✓ Zero-copy message construction works");

    // Test 3: Performance measurement
    println!("\n3. Testing performance (target: <100ns)...");
    
    // Warm up the buffer
    for _ in 0..100 {
        let _ = with_hot_path_buffer(|buffer| {
            let builder = ZeroCopyTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
                .add_tlv_ref(TLVType::Trade, &trade);
            let size = builder.build_into_buffer(buffer).unwrap();
            Ok(((), size))
        });
    }

    // Measure performance
    let iterations = 10_000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        let _ = with_hot_path_buffer(|buffer| {
            let builder = ZeroCopyTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
                .add_tlv_ref(TLVType::Trade, &trade);
            let size = builder.build_into_buffer(buffer).unwrap();
            std::hint::black_box(size);
            Ok(((), size))
        }).unwrap();
    }
    
    let duration = start.elapsed();
    let ns_per_op = duration.as_nanos() as f64 / iterations as f64;
    
    println!("Performance: {:.2} ns/op", ns_per_op);
    if ns_per_op < 100.0 {
        println!("✓ Performance target met (<100ns)");
    } else {
        println!("⚠ Performance target not met: {:.2}ns > 100ns", ns_per_op);
    }

    // Test 4: Build and send pattern
    println!("\n4. Testing build and send pattern...");
    let mut sent_size = 0;
    
    let result = ZeroCopyTLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .add_tlv_ref(TLVType::Trade, &trade)
        .build_and_send(|message_bytes| {
            sent_size = message_bytes.len();
            // Simulate sending
            Ok(message_bytes.len())
        });
    
    assert!(result.is_ok());
    assert!(sent_size > 32);
    println!("✓ Build and send pattern works");

    println!("\n🎉 Phase 1 Infrastructure Test Complete!");
    println!("All core functionality verified and ready for migration.");
}