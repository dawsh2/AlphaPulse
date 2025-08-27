//! Performance validation benchmarks for Generic Relay refactor
//!
//! This benchmark suite validates that the Generic + Trait architecture
//! maintains identical performance to the original relay implementations.
//!
//! Target requirements:
//! - Throughput: >1M msg/s construction, >1.6M msg/s parsing  
//! - Latency: <35μs forwarding per message
//! - Memory: 64KB buffer per connection (no additional allocations)

use alphapulse_relays::{
    create_validator, TopicConfig, TopicExtractionStrategy, TopicRegistry,
    ValidationPolicy,
};
use alphapulse_types::protocol::{MessageHeader, RelayDomain, MESSAGE_MAGIC};
use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
};
use std::time::{Duration, Instant};

const MESSAGE_SIZES: &[usize] = &[64, 256, 1024, 4096, 8192];
const CONNECTION_COUNTS: &[usize] = &[1, 10, 100, 500, 1000];

/// Benchmark message construction throughput (target: >1M msg/s)
fn bench_message_construction_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_construction");
    group.measurement_time(Duration::from_secs(10));
    
    for &size in MESSAGE_SIZES {
        group.throughput(Throughput::Elements(1000));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}B", size)),
            &size,
            |b, &size| {
                b.iter(|| {
                    // Simulate constructing 1000 messages
                    for i in 0..1000 {
                        let header = MessageHeader {
                            magic: MESSAGE_MAGIC,
                            relay_domain: RelayDomain::MarketData as u8,
                            version: 1,
                            source: 4, // Polygon
                            flags: 0,
                            sequence: i,
                            timestamp: 1000000000 + i as u64,
                            payload_size: size as u32,
                            checksum: 0xDEADBEEF,
                        };
                        black_box(header);
                    }
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark message parsing throughput (target: >1.6M msg/s)
fn bench_message_parsing_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_parsing");
    group.measurement_time(Duration::from_secs(10));
    
    // Create test messages for different sizes
    let test_messages: Vec<_> = MESSAGE_SIZES
        .iter()
        .map(|&size| {
            let header = MessageHeader {
                magic: MESSAGE_MAGIC,
                relay_domain: RelayDomain::MarketData as u8,
                version: 1,
                source: 4,
                flags: 0,
                sequence: 12345,
                timestamp: 1234567890,
                payload_size: size as u32,
                checksum: 0xDEADBEEF,
            };
            
            let header_bytes = unsafe {
                std::slice::from_raw_parts(
                    &header as *const _ as *const u8,
                    std::mem::size_of::<MessageHeader>(),
                )
            };
            
            let mut message = header_bytes.to_vec();
            message.extend_from_slice(&vec![0u8; size]); // Add payload
            message
        })
        .collect();
    
    for (i, &size) in MESSAGE_SIZES.iter().enumerate() {
        group.throughput(Throughput::Elements(1000));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}B", size)),
            &test_messages[i],
            |b, message| {
                b.iter(|| {
                    // Parse 1000 messages
                    for _ in 0..1000 {
                        let data = black_box(message);
                        if data.len() >= std::mem::size_of::<MessageHeader>() {
                            let header = unsafe { &*(data.as_ptr() as *const MessageHeader) };
                            black_box(
                                header.magic == MESSAGE_MAGIC
                                    && header.version == 1
                                    && header.relay_domain > 0
                                    && header.relay_domain <= 3,
                            );
                        }
                    }
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark per-message forwarding latency (target: <35μs)
fn bench_message_forwarding_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_forwarding_latency");
    group.measurement_time(Duration::from_secs(5));
    
    // Setup relay components
    let topic_config = TopicConfig {
        default: "default".to_string(),
        available: vec![
            "market_data_polygon".to_string(),
            "signals_arbitrage".to_string(),
            "execution_orders".to_string(),
        ],
        auto_discover: false,
        extraction_strategy: TopicExtractionStrategy::SourceType,
    };
    
    let registry = TopicRegistry::new(&topic_config).unwrap();
    let validation_policy = ValidationPolicy {
        checksum: false, // Performance mode
        audit: false,
        strict: false,
        max_message_size: Some(65536),
    };
    let validator = create_validator(&validation_policy);
    
    // Create test message
    let header = MessageHeader {
        magic: MESSAGE_MAGIC,
        relay_domain: RelayDomain::MarketData as u8,
        version: 1,
        source: 4, // Polygon
        flags: 0,
        sequence: 1,
        timestamp: 1000000000,
        payload_size: 1024,
        checksum: 0xDEADBEEF,
    };
    
    let payload = vec![0u8; 1024];
    
    // Benchmark single message processing latency
    group.bench_function("single_message_processing", |b| {
        b.iter(|| {
            let start = Instant::now();
            
            // 1. Validate message
            let validation_result = validator.validate(&header, &payload);
            black_box(validation_result);
            
            // 2. Extract topic
            let topic = registry
                .extract_topic(&header, None, &TopicExtractionStrategy::SourceType)
                .unwrap();
            
            // 3. Get subscribers  
            let subscribers = registry.get_subscribers(&topic);
            
            // 4. Simulate forwarding (measure just the routing overhead)
            black_box(subscribers.len());
            
            let elapsed = start.elapsed();
            black_box(elapsed);
        })
    });
    
    group.finish();
}

/// Benchmark concurrent connections scaling (target: 1000+ connections)
fn bench_concurrent_connections(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_connections");
    group.measurement_time(Duration::from_secs(10));
    
    for &conn_count in CONNECTION_COUNTS {
        // Setup topic registry with multiple subscribers per connection
        let topic_config = TopicConfig {
            default: "default".to_string(),
            available: vec!["market_data".to_string()],
            auto_discover: false,
            extraction_strategy: TopicExtractionStrategy::Fixed("market_data".to_string()),
        };
        
        let registry = TopicRegistry::new(&topic_config).unwrap();
        
        // Simulate connections by subscribing consumers
        for i in 0..conn_count {
            let consumer = alphapulse_relays::ConsumerId(format!("conn_{}", i));
            registry.subscribe(consumer, "market_data").unwrap();
        }
        
        let header = MessageHeader {
            magic: MESSAGE_MAGIC,
            relay_domain: RelayDomain::MarketData as u8,
            version: 1,
            source: 4,
            flags: 0,
            sequence: 1,
            timestamp: 1000000000,
            payload_size: 1024,
            checksum: 0xDEADBEEF,
        };
        
        group.throughput(Throughput::Elements(100)); // 100 messages per iteration
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_connections", conn_count)),
            &conn_count,
            |b, _| {
                b.iter(|| {
                    // Process 100 messages with current connection count
                    for seq in 0..100 {
                        let mut test_header = header;
                        test_header.sequence = seq;
                        
                        // Extract topic and get subscribers (simulating broadcast)
                        let topic = registry
                            .extract_topic(&test_header, None, &TopicExtractionStrategy::Fixed("market_data".to_string()))
                            .unwrap();
                        let subscribers = registry.get_subscribers(&topic);
                        
                        // Simulate sending to all subscribers
                        black_box(subscribers.len());
                    }
                })
            },
        );
        
        // Cleanup subscribers for next iteration
        for i in 0..conn_count {
            let consumer = alphapulse_relays::ConsumerId(format!("conn_{}", i));
            registry.unsubscribe_all(&consumer).unwrap();
        }
    }
    
    group.finish();
}

/// Benchmark memory allocation patterns (target: 64KB per connection)
fn bench_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");
    
    // Test buffer reuse efficiency
    group.bench_function("buffer_reuse_64kb", |b| {
        let buffer_size = 64 * 1024; // 64KB target
        let mut buffer = Vec::with_capacity(buffer_size);
        
        b.iter(|| {
            buffer.clear();
            
            // Simulate filling buffer with message data
            for i in 0..(buffer_size / 1024) {
                let header = MessageHeader {
                    magic: MESSAGE_MAGIC,
                    relay_domain: RelayDomain::MarketData as u8,
                    version: 1,
                    source: 4,
                    flags: 0,
                    sequence: i as u64,
                    timestamp: 1000000000 + i as u64,
                    payload_size: 1024,
                    checksum: 0xDEADBEEF,
                };
                
                let header_bytes = unsafe {
                    std::slice::from_raw_parts(
                        &header as *const _ as *const u8,
                        std::mem::size_of::<MessageHeader>(),
                    )
                };
                
                buffer.extend_from_slice(header_bytes);
                buffer.extend_from_slice(&vec![0u8; 1024 - std::mem::size_of::<MessageHeader>()]);
            }
            
            black_box(buffer.len());
        })
    });
    
    group.finish();
}

/// Comprehensive end-to-end performance test
fn bench_end_to_end_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end");
    group.measurement_time(Duration::from_secs(15));
    
    // Setup complete relay system
    let topic_config = TopicConfig {
        default: "default".to_string(),
        available: vec![
            "market_data_polygon".to_string(),
            "market_data_kraken".to_string(),
            "signals_arbitrage".to_string(),
        ],
        auto_discover: false,
        extraction_strategy: TopicExtractionStrategy::SourceType,
    };
    
    let registry = TopicRegistry::new(&topic_config).unwrap();
    let validation_policy = ValidationPolicy {
        checksum: true, // Reliability mode for real-world test
        audit: false,
        strict: false,
        max_message_size: Some(65536),
    };
    let validator = create_validator(&validation_policy);
    
    // Setup realistic subscriber load
    for i in 0..100 {
        let consumer = alphapulse_relays::ConsumerId(format!("polygon_consumer_{}", i));
        registry.subscribe(consumer, "market_data_polygon").unwrap();
    }
    
    for i in 0..50 {
        let consumer = alphapulse_relays::ConsumerId(format!("kraken_consumer_{}", i));  
        registry.subscribe(consumer, "market_data_kraken").unwrap();
    }
    
    for i in 0..25 {
        let consumer = alphapulse_relays::ConsumerId(format!("signal_consumer_{}", i));
        registry.subscribe(consumer, "signals_arbitrage").unwrap();
    }
    
    // Create realistic message mix
    let messages = vec![
        (MessageHeader {
            magic: MESSAGE_MAGIC,
            relay_domain: RelayDomain::MarketData as u8,
            version: 1,
            source: 4, // Polygon
            flags: 0,
            sequence: 1,
            timestamp: 1000000000,
            payload_size: 2048,
            checksum: crc32fast::hash(&vec![0u8; 2048]),
        }, vec![0u8; 2048]),
        (MessageHeader {
            magic: MESSAGE_MAGIC,
            relay_domain: RelayDomain::MarketData as u8,
            version: 1,
            source: 2, // Kraken
            flags: 0,
            sequence: 2,
            timestamp: 1000000001,
            payload_size: 1024,
            checksum: crc32fast::hash(&vec![1u8; 1024]),
        }, vec![1u8; 1024]),
        (MessageHeader {
            magic: MESSAGE_MAGIC,
            relay_domain: RelayDomain::Signal as u8,
            version: 1,
            source: 20, // Arbitrage
            flags: 0,
            sequence: 3,
            timestamp: 1000000002,
            payload_size: 512,
            checksum: crc32fast::hash(&vec![2u8; 512]),
        }, vec![2u8; 512]),
    ];
    
    // Benchmark complete message processing pipeline
    group.throughput(Throughput::Elements(1000)); // 1000 messages per iteration
    group.bench_function("realistic_message_mix", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                let (header, payload) = &messages[fastrand::usize(..messages.len())];
                
                // Complete processing pipeline
                // 1. Validate
                let _validation = validator.validate(header, payload);
                
                // 2. Extract topic  
                let topic = registry
                    .extract_topic(header, None, &TopicExtractionStrategy::SourceType)
                    .unwrap();
                
                // 3. Get subscribers
                let subscribers = registry.get_subscribers(&topic);
                
                // 4. Simulate forwarding to all subscribers
                black_box(subscribers.len());
            }
        })
    });
    
    group.finish();
}

criterion_group!(
    performance_validation,
    bench_message_construction_throughput,
    bench_message_parsing_throughput, 
    bench_message_forwarding_latency,
    bench_concurrent_connections,
    bench_memory_allocation,
    bench_end_to_end_performance
);

criterion_main!(performance_validation);