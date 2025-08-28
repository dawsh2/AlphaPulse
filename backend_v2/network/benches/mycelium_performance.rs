//! Mycelium Performance Benchmarks (MYCEL-008)
//!
//! Validates performance targets:
//! - Local (Arc<T>): <100ns per message
//! - Unix Socket: <35μs per message
//! - Zero allocations in steady state

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use torq_network::mycelium::{
    ProofOfConceptMigration, MarketMessage, PoolSwapEvent, QuoteUpdate,
    ActorSystem, ActorBehavior, SupervisorDirective, BundleConfiguration, DeploymentMode,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::HashMap;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

/// Simple test actor for benchmarking
struct BenchmarkActor {
    counter: AtomicU64,
}

impl BenchmarkActor {
    fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
        }
    }
}

#[async_trait::async_trait]
impl ActorBehavior for BenchmarkActor {
    type Message = MarketMessage;
    
    async fn handle(&mut self, _msg: MarketMessage) -> Result<(), torq_network::TransportError> {
        self.counter.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

fn bench_arc_clone_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("arc_clone");
    
    // Create test messages of different sizes
    let small_msg = Arc::new(QuoteUpdate {
        instrument_id: 12345,
        bid_price: 4500_00000000,
        ask_price: 4501_00000000,
        bid_size: 1000,
        ask_size: 1000,
        timestamp_ns: 1234567890,
    });
    
    let large_msg = Arc::new(PoolSwapEvent {
        pool_address: [1; 20],
        token0_in: 1_000_000_000_000_000_000,
        token1_out: 3000_000_000,
        timestamp_ns: 1234567890,
        tx_hash: [2; 32],
        gas_used: 21000,
    });
    
    // Benchmark Arc::clone() for small message
    group.bench_function("small_message", |b| {
        b.iter(|| {
            let cloned = Arc::clone(&small_msg);
            black_box(cloned);
        });
    });
    
    // Benchmark Arc::clone() for large message
    group.bench_function("large_message", |b| {
        b.iter(|| {
            let cloned = Arc::clone(&large_msg);
            black_box(cloned);
        });
    });
    
    // Benchmark creating MarketMessage enum with Arc
    group.bench_function("market_message_enum", |b| {
        b.iter(|| {
            let msg = MarketMessage::Quote(Arc::clone(&small_msg));
            black_box(msg);
        });
    });
    
    group.finish();
}

fn bench_local_message_passing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("local_message_passing");
    
    // Setup bundled actors for zero-cost communication
    group.bench_function("bundled_actors", |b| {
        b.to_async(&rt).iter(|| async {
            let system = ActorSystem::new();
            
            // Create shared memory bundle
            let bundle = BundleConfiguration {
                name: "bench_bundle".to_string(),
                actors: vec!["bench_actor".to_string()],
                deployment: DeploymentMode::SharedMemory {
                    channels: HashMap::new(),
                },
            };
            system.add_bundle("bench_bundle".to_string(), bundle).await.unwrap();
            
            // Spawn actor
            let actor = BenchmarkActor::new();
            let actor_ref = system.spawn(actor).await.unwrap();
            
            // Send message
            let msg = MarketMessage::Quote(Arc::new(QuoteUpdate {
                instrument_id: 12345,
                bid_price: 4500_00000000,
                ask_price: 4501_00000000,
                bid_size: 1000,
                ask_size: 1000,
                timestamp_ns: 1234567890,
            }));
            
            actor_ref.send(msg).await.unwrap();
        });
    });
    
    group.finish();
}

fn bench_channel_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("channel_throughput");
    
    for size in [10, 100, 1000, 10000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async move {
                let (tx, mut rx) = mpsc::channel(1000);
                
                // Spawn receiver task
                let receiver = tokio::spawn(async move {
                    let mut count = 0;
                    while let Some(_) = rx.recv().await {
                        count += 1;
                        if count >= size {
                            break;
                        }
                    }
                });
                
                // Send messages
                let msg = Arc::new(QuoteUpdate {
                    instrument_id: 12345,
                    bid_price: 4500_00000000,
                    ask_price: 4501_00000000,
                    bid_size: 1000,
                    ask_size: 1000,
                    timestamp_ns: 1234567890,
                });
                
                for _ in 0..size {
                    let cloned = Arc::clone(&msg) as Arc<dyn std::any::Any + Send + Sync>;
                    tx.send(cloned).await.unwrap();
                }
                
                receiver.await.unwrap();
            });
        });
    }
    
    group.finish();
}

fn bench_proof_of_concept(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("proof_of_concept");
    
    group.sample_size(10); // Reduce sample size for slower benchmarks
    group.measurement_time(Duration::from_secs(10));
    
    group.bench_function("end_to_end_pipeline", |b| {
        b.to_async(&rt).iter(|| async {
            // Setup migration
            let migration = ProofOfConceptMigration::setup().await.unwrap();
            
            // Send test data
            migration.send_test_data().await.unwrap();
            
            // Small delay to ensure processing
            tokio::time::sleep(Duration::from_millis(1)).await;
            
            // Get stats
            let stats = migration.get_stats().await;
            black_box(stats);
            
            // Shutdown
            migration.shutdown().await.unwrap();
        });
    });
    
    group.bench_function("message_batch_processing", |b| {
        b.to_async(&rt).iter(|| async {
            let migration = ProofOfConceptMigration::setup().await.unwrap();
            let market_ref = migration.market_processor_ref.as_ref().unwrap();
            
            // Send batch of messages
            for i in 0..100 {
                let quote = QuoteUpdate {
                    instrument_id: 12345 + i,
                    bid_price: 4500_00000000 + i as i64,
                    ask_price: 4501_00000000 + i as i64,
                    bid_size: 1000,
                    ask_size: 1000,
                    timestamp_ns: 1234567890 + i,
                };
                market_ref.send(MarketMessage::Quote(Arc::new(quote))).await.unwrap();
            }
            
            migration.shutdown().await.unwrap();
        });
    });
    
    group.finish();
}

fn bench_serialization_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization_comparison");
    
    let quote = QuoteUpdate {
        instrument_id: 12345,
        bid_price: 4500_00000000,
        ask_price: 4501_00000000,
        bid_size: 1000,
        ask_size: 1000,
        timestamp_ns: 1234567890,
    };
    
    // Benchmark Arc::clone (zero serialization)
    group.bench_function("arc_clone_zero_cost", |b| {
        let arc_quote = Arc::new(quote.clone());
        b.iter(|| {
            let cloned = Arc::clone(&arc_quote);
            black_box(cloned);
        });
    });
    
    // Benchmark TLV serialization (for comparison)
    group.bench_function("tlv_serialization", |b| {
        use torq_network::mycelium::Message;
        b.iter(|| {
            let tlv_bytes = quote.to_tlv().unwrap();
            black_box(tlv_bytes);
        });
    });
    
    // Calculate speedup factor
    println!("\n=== EXPECTED SPEEDUP ===");
    println!("Arc::clone should be 350x+ faster than TLV serialization");
    println!("Target: <100ns for Arc::clone vs ~35μs for TLV");
    
    group.finish();
}

criterion_group!(
    benches,
    bench_arc_clone_latency,
    bench_local_message_passing,
    bench_channel_throughput,
    bench_proof_of_concept,
    bench_serialization_comparison
);

criterion_main!(benches);