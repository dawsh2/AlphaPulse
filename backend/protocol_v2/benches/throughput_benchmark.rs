//! Throughput Benchmarks for Protocol V2 Relays
//! 
//! Measures sustained throughput under realistic load conditions.
//! Tests the difference between checksum validation vs skip for each domain.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use alphapulse_protocol_v2::{
    TLVType, RelayDomain, SourceType,
    tlv::TLVMessageBuilder,
    parse_header, MessageHeader,
};
use std::time::Duration;

/// Benchmark header parsing performance: fast vs full validation
fn bench_header_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("header_parsing");
    
    // Create test messages for each domain
    let market_msg = create_market_data_message();
    let signal_msg = create_signal_message();
    let execution_msg = create_execution_message();
    
    let test_cases = vec![
        ("market_data", market_msg),
        ("signal", signal_msg),
        ("execution", execution_msg),
    ];
    
    for (domain_name, message) in test_cases {
        // Benchmark full header parsing (with checksum validation)
        group.bench_with_input(
            BenchmarkId::new("full_parsing", domain_name),
            &message,
            |b, msg| {
                b.iter(|| {
                    black_box(parse_header(black_box(msg)).unwrap())
                })
            }
        );
        
        // Benchmark fast header parsing (no checksum validation)
        group.bench_with_input(
            BenchmarkId::new("fast_parsing", domain_name),
            &message,
            |b, msg| {
                b.iter(|| {
                    black_box(parse_header_fast(black_box(msg)).unwrap())
                })
            }
        );
    }
    
    group.finish();
}

/// Benchmark TLV construction vs processing overhead
fn bench_message_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_processing");
    group.measurement_time(Duration::from_secs(10));
    
    // Benchmark TLV construction only (baseline)
    group.bench_function("tlv_construction_only", |b| {
        b.iter(|| {
            let trade_payload = create_trade_payload(black_box(42));
            let _msg = TLVMessageBuilder::new(
                RelayDomain::MarketData,
                SourceType::BinanceCollector
            )
            .add_tlv_bytes(TLVType::Trade, trade_payload)
            .build();
            black_box(_msg)
        })
    });
    
    // Benchmark construction + market data relay processing
    group.bench_function("market_data_processing", |b| {
        b.iter(|| {
            let trade_payload = create_trade_payload(black_box(42));
            let msg = TLVMessageBuilder::new(
                RelayDomain::MarketData,
                SourceType::BinanceCollector
            )
            .add_tlv_bytes(TLVType::Trade, trade_payload)
            .build();
            
            // Simulate market data relay processing
            let _header = parse_header_fast(&msg).unwrap();
            black_box(_header)
        })
    });
    
    // Benchmark construction + signal relay processing (with checksum)
    group.bench_function("signal_processing", |b| {
        b.iter(|| {
            let signal_payload = create_signal_payload(black_box(42));
            let msg = TLVMessageBuilder::new(
                RelayDomain::Signal,
                SourceType::ArbitrageStrategy
            )
            .add_tlv_bytes(TLVType::SignalIdentity, signal_payload)
            .build();
            
            // Simulate signal relay processing (with checksum validation)
            let _header = parse_header(&msg).unwrap();
            black_box(_header)
        })
    });
    
    group.finish();
}

/// Benchmark sustained throughput simulation
fn bench_sustained_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("sustained_throughput");
    group.measurement_time(Duration::from_secs(15));
    
    // Test market data relay sustained throughput
    group.bench_function("market_data_sustained", |b| {
        b.iter(|| {
            let mut processed = 0u64;
            for i in 0..1000 {
                let trade_payload = create_trade_payload(i);
                let msg = TLVMessageBuilder::new(
                    RelayDomain::MarketData,
                    SourceType::BinanceCollector
                )
                .add_tlv_bytes(TLVType::Trade, trade_payload)
                .build();
                
                // Fast processing path
                if let Ok(_header) = parse_header_fast(&msg) {
                    processed += 1;
                }
            }
            black_box(processed)
        })
    });
    
    // Test signal relay sustained throughput  
    group.bench_function("signal_sustained", |b| {
        b.iter(|| {
            let mut processed = 0u64;
            for i in 0..1000 {
                let signal_payload = create_signal_payload(i);
                let msg = TLVMessageBuilder::new(
                    RelayDomain::Signal,
                    SourceType::ArbitrageStrategy
                )
                .add_tlv_bytes(TLVType::SignalIdentity, signal_payload)
                .build();
                
                // Full validation processing path
                if let Ok(_header) = parse_header(&msg) {
                    processed += 1;
                }
            }
            black_box(processed)
        })
    });
    
    group.finish();
}

/// Helper functions for creating test data
fn create_market_data_message() -> Vec<u8> {
    let trade_payload = create_trade_payload(12345);
    TLVMessageBuilder::new(
        RelayDomain::MarketData,
        SourceType::BinanceCollector
    )
    .add_tlv_bytes(TLVType::Trade, trade_payload)
    .build()
}

fn create_signal_message() -> Vec<u8> {
    let signal_payload = create_signal_payload(67890);
    TLVMessageBuilder::new(
        RelayDomain::Signal,
        SourceType::ArbitrageStrategy
    )
    .add_tlv_bytes(TLVType::SignalIdentity, signal_payload)
    .build()
}

fn create_execution_message() -> Vec<u8> {
    let order_payload = vec![
        0x35, 0x81, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // order_id: 98765
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, // instrument_id
        0x01, // side: 1
        0x01, // order_type: 1
        0x00, 0xe1, 0xf5, 0x05, 0x00, 0x00, 0x00, 0x00, // quantity
        0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, // price
    ];
    
    TLVMessageBuilder::new(
        RelayDomain::Execution,
        SourceType::ExecutionEngine
    )
    .add_tlv_bytes(TLVType::OrderRequest, order_payload)
    .build()
}

fn create_trade_payload(variant: usize) -> Vec<u8> {
    vec![
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, // instrument_id
        (variant & 0xFF) as u8, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, // price
        0x00, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x00, // volume
        0x01, // side
        ((variant + 10000) & 0xFF) as u8, 0x39, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // trade_id
    ]
}

fn create_signal_payload(variant: usize) -> Vec<u8> {
    vec![
        0x19, // signal_type: 25
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, // instrument_id
        (75 + (variant % 25)) as u8, // strength (varying)
        0x01, // direction: 1
        (variant & 0xFF) as u8, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x00, // timestamp_ns
        0x00, 0x01, 0x02, // metadata
    ]
}

/// Fast header parsing implementation for benchmarking
fn parse_header_fast(data: &[u8]) -> Result<&MessageHeader, alphapulse_protocol_v2::ProtocolError> {
    if data.len() < MessageHeader::SIZE {
        return Err(alphapulse_protocol_v2::ProtocolError::Parse(
            alphapulse_protocol_v2::ParseError::MessageTooSmall {
                need: MessageHeader::SIZE,
                got: data.len(),
            }
        ));
    }
    
    let header_bytes = &data[..MessageHeader::SIZE];
    let header = zerocopy::Ref::<_, MessageHeader>::new(header_bytes)
        .ok_or(alphapulse_protocol_v2::ProtocolError::Parse(
            alphapulse_protocol_v2::ParseError::MessageTooSmall {
                need: MessageHeader::SIZE,
                got: data.len(),
            }
        ))?
        .into_ref();
    
    // Only validate magic number - skip checksum for performance
    if header.magic != alphapulse_protocol_v2::MESSAGE_MAGIC {
        return Err(alphapulse_protocol_v2::ProtocolError::Parse(
            alphapulse_protocol_v2::ParseError::InvalidMagic {
                expected: alphapulse_protocol_v2::MESSAGE_MAGIC,
                actual: header.magic,
            }
        ));
    }
    
    Ok(header)
}

criterion_group!(
    benches,
    bench_header_parsing,
    bench_message_processing,
    bench_sustained_throughput
);
criterion_main!(benches);