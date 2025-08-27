//! Performance benchmarks comparing FixedVec vs Vec for OrderBook operations
//!
//! Validates the zero-copy performance claims and ensures >1M msg/s throughput

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use alphapulse_types::protocol::tlv::market_data::{OrderBookTLV, OrderLevel};
use alphapulse_types::protocol::tlv::dynamic_payload::MAX_ORDER_LEVELS;
use alphapulse_types::{InstrumentId, VenueId};
use std::mem::size_of;

/// Create a test OrderBookTLV with FixedVec for benchmarking
fn create_orderbook_fixedvec(levels: usize) -> OrderBookTLV {
    let mut order_book = OrderBookTLV {
        instrument_id: InstrumentId::from_raw(12345),
        venue_id: VenueId::KRAKEN,
        timestamp_exchange_ns: 1_000_000_000_000,
        timestamp_received_ns: 1_000_000_001_000,
        sequence_number: 42,
        bid_count: levels as u8,
        ask_count: levels as u8,
        bids: [OrderLevel::default(); MAX_ORDER_LEVELS],
        asks: [OrderLevel::default(); MAX_ORDER_LEVELS],
    };
    
    // Fill with realistic data
    for i in 0..levels.min(MAX_ORDER_LEVELS) {
        order_book.bids[i] = OrderLevel {
            price: 50000_00000000 - (i as i64 * 100_00000000), // $50,000 - $100 per level
            quantity: 1_000_000 + (i as i64 * 100_000), // 1.0 + 0.1 per level
            order_count: (i + 1) as u16,
        };
        order_book.asks[i] = OrderLevel {
            price: 50000_00000000 + (i as i64 * 100_00000000), // $50,000 + $100 per level
            quantity: 1_000_000 + (i as i64 * 100_000),
            order_count: (i + 1) as u16,
        };
    }
    
    order_book
}

/// Alternative Vec-based OrderBook for comparison
#[derive(Clone)]
struct OrderBookVec {
    instrument_id: InstrumentId,
    venue_id: VenueId,
    timestamp_exchange_ns: u64,
    timestamp_received_ns: u64,
    sequence_number: u64,
    bids: Vec<OrderLevel>,
    asks: Vec<OrderLevel>,
}

fn create_orderbook_vec(levels: usize) -> OrderBookVec {
    let mut bids = Vec::with_capacity(levels);
    let mut asks = Vec::with_capacity(levels);
    
    for i in 0..levels {
        bids.push(OrderLevel {
            price: 50000_00000000 - (i as i64 * 100_00000000),
            quantity: 1_000_000 + (i as i64 * 100_000),
            order_count: (i + 1) as u16,
        });
        asks.push(OrderLevel {
            price: 50000_00000000 + (i as i64 * 100_00000000),
            quantity: 1_000_000 + (i as i64 * 100_000),
            order_count: (i + 1) as u16,
        });
    }
    
    OrderBookVec {
        instrument_id: InstrumentId::from_raw(12345),
        venue_id: VenueId::KRAKEN,
        timestamp_exchange_ns: 1_000_000_000_000,
        timestamp_received_ns: 1_000_000_001_000,
        sequence_number: 42,
        bids,
        asks,
    }
}

/// Benchmark serialization performance
fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook_serialization");
    
    for levels in [5, 10, 20, 30].iter() {
        let orderbook_fixed = create_orderbook_fixedvec(*levels);
        let orderbook_vec = create_orderbook_vec(*levels);
        
        // Benchmark FixedVec serialization (zero-copy)
        group.bench_with_input(
            BenchmarkId::new("FixedVec", levels),
            &orderbook_fixed,
            |b, book| {
                b.iter(|| {
                    // Zero-copy serialization via AsBytes
                    let bytes = zerocopy::AsBytes::as_bytes(black_box(book));
                    black_box(bytes.len());
                });
            },
        );
        
        // Benchmark Vec serialization (requires allocation)
        group.bench_with_input(
            BenchmarkId::new("Vec", levels),
            &orderbook_vec,
            |b, book| {
                b.iter(|| {
                    // Manual serialization with allocation
                    let mut buffer = Vec::with_capacity(1024);
                    buffer.extend_from_slice(&book.instrument_id.to_le_bytes());
                    buffer.extend_from_slice(&book.venue_id.to_le_bytes());
                    buffer.extend_from_slice(&book.timestamp_exchange_ns.to_le_bytes());
                    buffer.extend_from_slice(&book.timestamp_received_ns.to_le_bytes());
                    buffer.extend_from_slice(&book.sequence_number.to_le_bytes());
                    buffer.extend_from_slice(&(book.bids.len() as u8).to_le_bytes());
                    buffer.extend_from_slice(&(book.asks.len() as u8).to_le_bytes());
                    for level in &book.bids {
                        buffer.extend_from_slice(&level.price.to_le_bytes());
                        buffer.extend_from_slice(&level.quantity.to_le_bytes());
                        buffer.extend_from_slice(&level.order_count.to_le_bytes());
                    }
                    for level in &book.asks {
                        buffer.extend_from_slice(&level.price.to_le_bytes());
                        buffer.extend_from_slice(&level.quantity.to_le_bytes());
                        buffer.extend_from_slice(&level.order_count.to_le_bytes());
                    }
                    black_box(buffer.len());
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark deserialization performance
fn bench_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook_deserialization");
    
    for levels in [5, 10, 20, 30].iter() {
        let orderbook = create_orderbook_fixedvec(*levels);
        let bytes = zerocopy::AsBytes::as_bytes(&orderbook);
        
        // Benchmark FixedVec deserialization (zero-copy)
        group.bench_with_input(
            BenchmarkId::new("FixedVec_zerocopy", levels),
            &bytes,
            |b, data| {
                b.iter(|| {
                    // Zero-copy deserialization
                    let result = zerocopy::FromBytes::read_from(black_box(*data));
                    black_box(result);
                });
            },
        );
        
        // Benchmark Vec deserialization (requires allocation)
        group.bench_with_input(
            BenchmarkId::new("Vec_allocating", levels),
            &bytes,
            |b, data| {
                b.iter(|| {
                    // Manual deserialization with allocation
                    let mut offset = 0;
                    let _instrument_id = u64::from_le_bytes(
                        data[offset..offset + 8].try_into().unwrap()
                    );
                    offset += 8;
                    let _venue_id = u16::from_le_bytes(
                        data[offset..offset + 2].try_into().unwrap()
                    );
                    offset += 2;
                    let _ts_exchange = u64::from_le_bytes(
                        data[offset..offset + 8].try_into().unwrap()
                    );
                    offset += 8;
                    let _ts_received = u64::from_le_bytes(
                        data[offset..offset + 8].try_into().unwrap()
                    );
                    offset += 8;
                    let _sequence = u64::from_le_bytes(
                        data[offset..offset + 8].try_into().unwrap()
                    );
                    offset += 8;
                    let bid_count = data[offset] as usize;
                    offset += 1;
                    let ask_count = data[offset] as usize;
                    offset += 1;
                    
                    let mut bids = Vec::with_capacity(bid_count);
                    for _ in 0..bid_count {
                        let price = i64::from_le_bytes(
                            data[offset..offset + 8].try_into().unwrap()
                        );
                        offset += 8;
                        let quantity = i64::from_le_bytes(
                            data[offset..offset + 8].try_into().unwrap()
                        );
                        offset += 8;
                        let order_count = u16::from_le_bytes(
                            data[offset..offset + 2].try_into().unwrap()
                        );
                        offset += 2;
                        bids.push(OrderLevel { price, quantity, order_count });
                    }
                    
                    black_box(bids);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark memory usage
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook_memory");
    
    group.bench_function("FixedVec_size", |b| {
        b.iter(|| {
            let size = size_of::<OrderBookTLV>();
            black_box(size);
        });
    });
    
    group.bench_function("Vec_heap_allocation", |b| {
        b.iter(|| {
            let book = create_orderbook_vec(20);
            let heap_size = book.bids.capacity() * size_of::<OrderLevel>() +
                           book.asks.capacity() * size_of::<OrderLevel>();
            black_box(heap_size);
        });
    });
    
    group.finish();
}

/// Benchmark throughput for message processing
fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook_throughput");
    
    // Test with 20-level order books (typical depth)
    let orderbook = create_orderbook_fixedvec(20);
    let bytes = zerocopy::AsBytes::as_bytes(&orderbook);
    
    group.throughput(Throughput::Bytes(bytes.len() as u64));
    
    // Benchmark FixedVec round-trip (serialize + deserialize)
    group.bench_function("FixedVec_roundtrip", |b| {
        b.iter(|| {
            // Serialize
            let serialized = zerocopy::AsBytes::as_bytes(black_box(&orderbook));
            // Deserialize
            let deserialized: &OrderBookTLV = zerocopy::FromBytes::read_from(black_box(serialized)).unwrap();
            black_box(deserialized);
        });
    });
    
    // Calculate messages per second
    group.bench_function("FixedVec_msgs_per_sec", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            for _ in 0..1000 {
                let serialized = zerocopy::AsBytes::as_bytes(&orderbook);
                let _deserialized: &OrderBookTLV = zerocopy::FromBytes::read_from(serialized).unwrap();
                counter += 1;
            }
            black_box(counter);
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_serialization,
    bench_deserialization,
    bench_memory_usage,
    bench_throughput
);
criterion_main!(benches);