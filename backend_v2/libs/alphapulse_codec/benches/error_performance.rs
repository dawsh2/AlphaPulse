//! Performance validation for enhanced error reporting
//! 
//! Ensures that error enhancements do not impact happy path performance.
//! The enhanced error types should only add overhead when errors occur,
//! not during successful parsing operations.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use alphapulse_codec::{
    parser::parse_header,
    error::ProtocolError,
};
use alphapulse_types::{
    protocol::message::header::MessageHeader,
    MESSAGE_MAGIC, RelayDomain, SourceType
};
use std::mem::size_of;

/// Create a valid 32-byte message header for testing
fn create_valid_header() -> Vec<u8> {
    let header = MessageHeader {
        magic: MESSAGE_MAGIC,
        relay_domain: RelayDomain::MarketData as u8,
        version: 1,
        source: SourceType::PolygonCollector as u8,
        flags: 0,
        sequence: 12345,
        timestamp: 1000000000000u64,
        payload_size: 0,
        checksum: 0,
    };
    
    // Convert to bytes
    let header_bytes = unsafe {
        std::slice::from_raw_parts(
            &header as *const MessageHeader as *const u8,
            size_of::<MessageHeader>()
        )
    };
    
    header_bytes.to_vec()
}

/// Create an invalid message for error path benchmarking
fn create_invalid_message() -> Vec<u8> {
    vec![0x01, 0x02, 0x03, 0x04] // Too small for header
}

/// Benchmark happy path: parsing valid headers (should be unaffected by error enhancements)
fn bench_happy_path_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("happy_path_parsing");
    
    let valid_header = create_valid_header();
    
    group.bench_function("parse_valid_header", |b| {
        b.iter(|| {
            // This should succeed and should not be affected by error enhancements
            let result = parse_header(black_box(&valid_header));
            black_box(result);
        });
    });
    
    group.finish();
}

/// Benchmark error path: ensure enhanced errors don't add excessive overhead
fn bench_error_path_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_path_parsing");
    
    let invalid_message = create_invalid_message();
    
    group.bench_function("parse_header_error", |b| {
        b.iter(|| {
            let result = parse_header(black_box(&invalid_message));
            match result {
                Err(ProtocolError::MessageTooSmall { .. }) => {},
                _ => panic!("Expected MessageTooSmall error"),
            }
        });
    });
    
    group.finish();
}

/// Benchmark error creation and formatting overhead
fn bench_error_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_creation");
    
    group.bench_function("create_message_too_small", |b| {
        b.iter(|| {
            let error = ProtocolError::message_too_small(
                black_box(32),
                black_box(16), 
                black_box("test context")
            );
            black_box(error);
        });
    });
    
    group.bench_function("create_checksum_mismatch", |b| {
        b.iter(|| {
            let error = ProtocolError::checksum_mismatch(
                black_box(0x12345678),
                black_box(0x87654321),
                black_box(1024),
                black_box(5)
            );
            black_box(error);
        });
    });
    
    group.bench_function("create_truncated_tlv", |b| {
        b.iter(|| {
            let error = ProtocolError::truncated_tlv(
                black_box(100),
                black_box(150),
                black_box(42),
                black_box(75)
            );
            black_box(error);
        });
    });
    
    group.finish();
}

/// Benchmark error formatting performance
fn bench_error_formatting(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_formatting");
    
    let error = ProtocolError::checksum_mismatch(0x12345678, 0x87654321, 1024, 5);
    
    group.bench_function("format_display", |b| {
        b.iter(|| {
            let formatted = format!("{}", black_box(&error));
            black_box(formatted);
        });
    });
    
    group.bench_function("format_debug", |b| {
        b.iter(|| {
            let formatted = format!("{:?}", black_box(&error));
            black_box(formatted);
        });
    });
    
    group.finish();
}

/// Benchmark comparison: simple vs enhanced errors
fn bench_error_overhead_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_overhead_comparison");
    
    // Simulate simple error creation (baseline)
    group.bench_function("simple_error", |b| {
        b.iter(|| {
            #[derive(Debug)]
            enum SimpleError {
                TruncatedTLV,
            }
            let error = SimpleError::TruncatedTLV;
            black_box(error);
        });
    });
    
    // Enhanced error creation
    group.bench_function("enhanced_error", |b| {
        b.iter(|| {
            let error = ProtocolError::truncated_tlv(100, 150, 42, 75);
            black_box(error);
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_happy_path_parsing,
    bench_error_path_parsing,
    bench_error_creation,
    bench_error_formatting,
    bench_error_overhead_comparison
);
criterion_main!(benches);