#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! protocol_v2 = { path = "./protocol_v2" }
//! criterion = "0.5"
//! ```

use protocol_v2::{
    tlv::{build_message_direct, fast_timestamp::init_timestamp_system},
    tlv::market_data::TradeTLV,
    parse_header, parse_tlv_extensions,
    RelayDomain, SourceType, TLVType,
};
use std::time::{Instant, Duration};

fn main() {
    init_timestamp_system();
    println!("🔬 TLV Bottleneck Analysis");
    println!("=" .repeat(50));

    // Test different sizes
    let sizes = [64, 256, 1024, 4096, 16384];

    for size in sizes {
        println!("\n📊 Testing with payload size: {} bytes", size);
        profile_size(size);
    }

    println!("\n🔍 Profiling specific operations:");
    profile_operations();
}

fn profile_size(payload_size: usize) {
    let trade = TradeTLV {
        venue: 1,
        instrument_id: 123456789,
        price: 4500000000000,
        quantity: 1000000,
        side: 1,
        timestamp_ns: 1234567890123456789,
        trade_id: 987654321,
        metadata: 0,
    };

    // Build phase
    let start = Instant::now();
    let iterations = 100_000;

    for _ in 0..iterations {
        let msg = build_message_direct(
            RelayDomain::MarketData,
            SourceType::PolygonCollector,
            TLVType::Trade,
            &trade,
        ).unwrap();
        std::hint::black_box(msg);
    }
    let build_time = start.elapsed();

    // Parse phase
    let msg = build_message_direct(
        RelayDomain::MarketData,
        SourceType::PolygonCollector,
        TLVType::Trade,
        &trade,
    ).unwrap();

    let start = Instant::now();
    for _ in 0..iterations {
        let header = parse_header(&msg[..32]).unwrap();
        let tlvs = parse_tlv_extensions(&msg[32..32 + header.payload_size as usize]).unwrap();
        std::hint::black_box((header, tlvs));
    }
    let parse_time = start.elapsed();

    println!("  Build: {:?}/msg ({:.0} msg/s)",
        build_time / iterations,
        iterations as f64 / build_time.as_secs_f64());
    println!("  Parse: {:?}/msg ({:.0} msg/s)",
        parse_time / iterations,
        iterations as f64 / parse_time.as_secs_f64());
}

fn profile_operations() {
    let trade = TradeTLV {
        venue: 1,
        instrument_id: 123456789,
        price: 4500000000000,
        quantity: 1000000,
        side: 1,
        timestamp_ns: 1234567890123456789,
        trade_id: 987654321,
        metadata: 0,
    };

    // Profile individual operations
    let iterations = 1_000_000;

    // 1. Header construction only
    let start = Instant::now();
    for _ in 0..iterations {
        let mut header = [0u8; 32];
        header[0..4].copy_from_slice(&0xDEADBEEF_u32.to_le_bytes());
        std::hint::black_box(header);
    }
    let header_time = start.elapsed();
    println!("\n  Header construction: {:?}/op", header_time / iterations);

    // 2. TLV serialization only
    let start = Instant::now();
    for _ in 0..iterations {
        let bytes = unsafe {
            std::slice::from_raw_parts(
                &trade as *const _ as *const u8,
                std::mem::size_of::<TradeTLV>(),
            )
        };
        std::hint::black_box(bytes);
    }
    let serialize_time = start.elapsed();
    println!("  TLV serialization: {:?}/op", serialize_time / iterations);

    // 3. Checksum calculation
    let msg = build_message_direct(
        RelayDomain::MarketData,
        SourceType::PolygonCollector,
        TLVType::Trade,
        &trade,
    ).unwrap();

    let start = Instant::now();
    for _ in 0..iterations {
        let mut checksum = 0u32;
        for byte in &msg[..msg.len() - 4] {
            checksum = checksum.wrapping_add(*byte as u32);
        }
        std::hint::black_box(checksum);
    }
    let checksum_time = start.elapsed();
    println!("  Checksum calculation: {:?}/op", checksum_time / iterations);

    // 4. Memory allocation
    let start = Instant::now();
    for _ in 0..iterations {
        let vec = Vec::<u8>::with_capacity(256);
        std::hint::black_box(vec);
    }
    let alloc_time = start.elapsed();
    println!("  Vec allocation (256B): {:?}/op", alloc_time / iterations);
}
