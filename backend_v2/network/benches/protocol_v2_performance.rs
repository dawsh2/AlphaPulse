//! Protocol V2 Performance Benchmarks
//!
//! Validates that TLV message parsing and construction meet the >1M msg/s
//! performance requirements for AlphaPulse trading system.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use std::time::{SystemTime, UNIX_EPOCH};
use torq_network::{ProtocolV2Validator, validate_timestamp_precision};

/// Create realistic Protocol V2 test message
fn create_test_message() -> Vec<u8> {
    let mut message = Vec::with_capacity(128);
    
    // 32-byte MessageHeader
    message.extend_from_slice(&0xDEADBEEF_u32.to_le_bytes()); // Magic
    message.extend_from_slice(&64_u32.to_le_bytes());          // Payload size
    message.push(1);                                           // Relay domain (Market Data)
    message.push(1);                                           // Source ID
    message.extend_from_slice(&[0u8; 2]);                      // Padding
    message.extend_from_slice(&12345_u32.to_le_bytes());       // Sequence
    
    // Current timestamp in nanoseconds
    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    message.extend_from_slice(&timestamp_ns.to_le_bytes());
    
    message.extend_from_slice(&[0u8; 4]); // Reserved
    
    // TLV payload (64 bytes) - create first
    let tlv_payload = create_market_data_tlv();
    
    // Add placeholder checksum and payload for size calculation
    message.extend_from_slice(&[0u8; 4]); // Placeholder checksum
    message.extend(&tlv_payload);
    
    // Calculate Protocol V2-compatible checksum over complete message
    let checksum = calculate_protocol_v2_checksum(&message);
    
    // Update checksum in header (bytes 28-31)
    let checksum_bytes = checksum.to_le_bytes();
    message[28] = checksum_bytes[0];
    message[29] = checksum_bytes[1]; 
    message[30] = checksum_bytes[2];
    message[31] = checksum_bytes[3];
    
    message
}

/// Create Market Data TLV (Type 1, within domain 1-19)
fn create_market_data_tlv() -> Vec<u8> {
    let mut tlv = Vec::new();
    
    // TLV Header: Type=1 (Trade), Length=56
    tlv.extend_from_slice(&1_u16.to_le_bytes());  // Type (Market Data domain)
    tlv.extend_from_slice(&56_u16.to_le_bytes()); // Length
    
    // Trade TLV payload (56 bytes)
    tlv.extend_from_slice(&[0x12; 20]);           // Pool address (20 bytes)
    tlv.extend_from_slice(&1_000_000_000_000_000_000_u64.to_le_bytes()); // 1 WETH (18 decimals)
    tlv.extend_from_slice(&2_000_000_000_u64.to_le_bytes());             // 2000 USDC (6 decimals)
    
    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    tlv.extend_from_slice(&timestamp_ns.to_le_bytes()); // Timestamp (8 bytes)
    
    tlv.extend_from_slice(&[0xab; 32]);           // Transaction hash (32 bytes - padding to 56 total)
    
    tlv
}

/// Calculate Protocol V2-compatible checksum for benchmarks
fn calculate_protocol_v2_checksum(message_bytes: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    
    // Protocol V2 checksum: header fields (excluding checksum) + payload
    hasher.update(&message_bytes[0..28]);
    
    // Include payload if present  
    if message_bytes.len() > 32 {
        hasher.update(&message_bytes[32..]);
    }
    
    // Protocol V2 integrity check: XOR with length factor
    let base_crc = hasher.finalize();
    let length_factor = (message_bytes.len() as u32).wrapping_mul(0xDEADBEEF);
    
    base_crc ^ length_factor
}

/// Benchmark Protocol V2 message validation performance
fn bench_protocol_v2_validation(c: &mut Criterion) {
    let validator = ProtocolV2Validator::new();
    let test_message = create_test_message();
    
    let mut group = c.benchmark_group("protocol_v2_validation");
    group.throughput(Throughput::Elements(1));
    
    group.bench_function("single_message_validation", |b| {
        b.iter(|| {
            black_box(validator.validate_message(black_box(&test_message)).unwrap())
        })
    });
    
    // Test batch validation for throughput measurement
    let messages: Vec<_> = (0..1000).map(|_| create_test_message()).collect();
    
    group.bench_function("batch_1000_messages", |b| {
        b.iter(|| {
            for message in &messages {
                black_box(validator.validate_message(black_box(message)).unwrap());
            }
        })
    });
    
    group.finish();
}

/// Benchmark TLV parsing performance specifically
fn bench_tlv_parsing(c: &mut Criterion) {
    let validator = ProtocolV2Validator::new();
    let test_message = create_test_message();
    
    let mut group = c.benchmark_group("tlv_parsing");
    group.throughput(Throughput::Bytes(test_message.len() as u64));
    
    group.bench_function("header_parsing", |b| {
        b.iter(|| {
            // Simulate header parsing (first 32 bytes)
            let header_bytes = black_box(&test_message[..32]);
            let magic = u32::from_le_bytes([header_bytes[0], header_bytes[1], header_bytes[2], header_bytes[3]]);
            let payload_size = u32::from_le_bytes([header_bytes[4], header_bytes[5], header_bytes[6], header_bytes[7]]);
            let domain = header_bytes[8];
            black_box((magic, payload_size, domain))
        })
    });
    
    group.bench_function("tlv_payload_parsing", |b| {
        b.iter(|| {
            // Simulate TLV payload parsing
            let payload = black_box(&test_message[32..]);
            let mut offset = 0;
            let mut tlv_count = 0;
            
            while offset + 4 <= payload.len() {
                let tlv_type = u16::from_le_bytes([payload[offset], payload[offset + 1]]);
                let tlv_length = u16::from_le_bytes([payload[offset + 2], payload[offset + 3]]);
                
                if offset + 4 + tlv_length as usize > payload.len() {
                    break;
                }
                
                tlv_count += 1;
                offset += 4 + tlv_length as usize;
            }
            
            black_box(tlv_count)
        })
    });
    
    group.finish();
}

/// Benchmark timestamp precision validation
fn bench_timestamp_validation(c: &mut Criterion) {
    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    c.bench_function("timestamp_precision_validation", |b| {
        b.iter(|| {
            black_box(validate_timestamp_precision(black_box(timestamp_ns)).unwrap())
        })
    });
}

/// Benchmark domain validation performance
fn bench_domain_validation(c: &mut Criterion) {
    let validator = ProtocolV2Validator::new();
    
    let mut group = c.benchmark_group("domain_validation");
    
    group.bench_function("market_data_domain", |b| {
        b.iter(|| {
            // Market Data domain (1) should accept types 1-19
            for tlv_type in 1..=19 {
                black_box(validator.validate_tlv_domain(black_box(tlv_type), 1));
            }
        })
    });
    
    group.bench_function("cross_domain_validation", |b| {
        b.iter(|| {
            // Test all combinations for hot path
            for domain in 1..=3 {
                for tlv_type in 1..=79 {
                    black_box(validator.validate_tlv_domain(black_box(tlv_type), black_box(domain)));
                }
            }
        })
    });
    
    group.finish();
}

/// Performance requirement validation test
fn bench_throughput_requirement(c: &mut Criterion) {
    let validator = ProtocolV2Validator::new();
    let test_messages: Vec<_> = (0..10000).map(|_| create_test_message()).collect();
    
    let mut group = c.benchmark_group("throughput_validation");
    group.throughput(Throughput::Elements(10000));
    
    // Target: >1M msg/s = <1μs per message
    group.bench_function("10k_messages_throughput", |b| {
        b.iter(|| {
            let start = std::time::Instant::now();
            let mut validated = 0;
            
            for message in &test_messages {
                if validator.validate_message(black_box(message)).is_ok() {
                    validated += 1;
                }
            }
            
            let elapsed = start.elapsed();
            let messages_per_second = (validated as f64) / elapsed.as_secs_f64();
            
            // Ensure we meet >1M msg/s requirement
            assert!(
                messages_per_second > 1_000_000.0,
                "Performance requirement not met: {:.0} msg/s < 1M msg/s target",
                messages_per_second
            );
            
            black_box(validated)
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_protocol_v2_validation,
    bench_tlv_parsing,
    bench_timestamp_validation,
    bench_domain_validation,
    bench_throughput_requirement
);
criterion_main!(benches);