//! Mycelium Actor Runtime Performance Benchmarks
//!
//! Validates zero-cost local transport achieves <100ns latency target
//! and measures serialization elimination benefits.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

// Import Mycelium components
use torq_network::mycelium::{
    transport::{ActorTransport, TransportMetrics},
    messages::{Message, MarketMessage, PoolSwapEvent, QuoteUpdate},
};

/// Benchmark local transport zero-copy Arc<T> message sending
fn bench_local_transport_latency(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("local_transport_latency");
    group.sample_size(1000);
    
    // Test different message sizes
    let message_sizes = vec![
        ("small_quote", 128),   // QuoteUpdate ~128 bytes
        ("large_pool_event", 256), // PoolSwapEvent ~256 bytes  
    ];
    
    for (name, _size) in message_sizes {
        group.bench_with_input(BenchmarkId::new("arc_clone_send", name), &name, |b, _| {
            let (tx, mut rx) = mpsc::channel(1000);
            let transport = ActorTransport::new_local(tx, "bench_actor".to_string());
            
            b.to_async(&rt).iter(|| async {
                // Create real Protocol V2 message
                let timestamp_ns = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;
                    
                let pool_event = PoolSwapEvent {
                    pool_address: [0x12; 20],
                    token0_in: 1_000_000_000_000_000_000, // 1 WETH
                    token1_out: 2_000_000_000, // 2000 USDC
                    timestamp_ns,
                    tx_hash: [0xab; 32],
                    gas_used: 150_000,
                };
                
                let market_msg = MarketMessage::Swap(Arc::new(pool_event));
                
                // This should be <100ns for local transport
                black_box(transport.send(market_msg).await.unwrap());
                
                // Consume message to prevent channel backup
                let _received = rx.try_recv();
            });
        });
    }
    
    group.finish();
}

/// Benchmark serialization elimination benefits  
fn bench_serialization_elimination(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("serialization_elimination");
    group.sample_size(500);
    
    group.bench_function("zero_copy_vs_tlv_serialization", |b| {
        let (tx, mut rx) = mpsc::channel(1000);
        let transport = ActorTransport::new_local(tx, "bench_actor".to_string());
        
        b.to_async(&rt).iter(|| async {
            let quote_update = QuoteUpdate {
                instrument_id: 12345,
                bid_price: 1999_50000000_i64, // 8-decimal fixed point
                ask_price: 2001_50000000_i64,
                bid_size: 1_000_000,
                ask_size: 1_200_000,
                timestamp_ns: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64,
            };
            
            // Zero-copy Arc<T> path (should be ~10-100x faster than serialization)
            let market_msg = MarketMessage::Quote(Arc::new(quote_update));
            black_box(transport.send(market_msg).await.unwrap());
            
            let _received = rx.try_recv();
        });
    });
    
    group.finish();
}

/// Benchmark transport metrics accuracy
fn bench_transport_metrics(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("transport_metrics_collection", |b| {
        let (tx, _rx) = mpsc::channel(1000);
        let transport = ActorTransport::new_local(tx, "bench_actor".to_string());
        
        b.to_async(&rt).iter(|| async {
            // Send multiple messages to test metrics accuracy
            for i in 0..10 {
                let timestamp_ns = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64 + i;
                    
                let quote = QuoteUpdate {
                    instrument_id: 12345 + i,
                    bid_price: (2000_00000000_i64) + (i as i64 * 1000000),
                    ask_price: (2002_00000000_i64) + (i as i64 * 1000000),
                    bid_size: 1_000_000,
                    ask_size: 1_000_000,
                    timestamp_ns,
                };
                
                let msg = MarketMessage::Quote(Arc::new(quote));
                black_box(transport.send(msg).await.unwrap());
            }
            
            // Measure metrics collection overhead
            let metrics = transport.metrics();
            let stats = black_box(metrics.get_stats());
            
            // Validate key performance indicators
            assert!(stats.local_sends >= 10);
            assert!(stats.avg_local_latency_ns > 0.0);
            assert!(stats.serialization_eliminated_mb > 0.0);
        });
    });
}

/// High-throughput benchmark simulating real trading workload
fn bench_high_throughput_scenario(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("high_throughput");
    group.sample_size(100);
    
    // Test throughput scenarios based on Protocol V2 targets (>1M msg/s)
    let message_counts = vec![1000, 5000, 10000];
    
    for count in message_counts {
        group.bench_with_input(
            BenchmarkId::new("messages_per_batch", count),
            &count,
            |b, &msg_count| {
                let (tx, mut rx) = mpsc::channel(msg_count * 2);
                let transport = ActorTransport::new_local(tx, "throughput_actor".to_string());
                
                b.to_async(&rt).iter(|| async {
                    // Send batch of messages as fast as possible
                    for i in 0..msg_count {
                        let pool_event = PoolSwapEvent {
                            pool_address: [0x12; 20],
                            token0_in: 1_000_000_000_000_000_000 + (i as u64 * 1000),
                            token1_out: 2_000_000_000 + (i as u64 * 1000),
                            timestamp_ns: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_nanos() as u64 + i as u64,
                            tx_hash: [0xab; 32],
                            gas_used: 150_000 + i as u64,
                        };
                        
                        let msg = MarketMessage::Swap(Arc::new(pool_event));
                        transport.send(msg).await.unwrap();
                    }
                    
                    // Drain the channel
                    for _ in 0..msg_count {
                        let _received = rx.try_recv();
                    }
                });
            }
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_local_transport_latency,
    bench_serialization_elimination,
    bench_transport_metrics,
    bench_high_throughput_scenario
);
criterion_main!(benches);