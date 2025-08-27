//! Benchmarks for relay throughput performance
//!
//! Target performance:
//! - Market Data Relay: >1M messages/second
//! - Signal Relay: >100K messages/second  
//! - Execution Relay: >50K messages/second

use alphapulse_types::protocol::{MessageHeader, MESSAGE_MAGIC};
use alphapulse_relays::{
    create_validator, ConsumerId, MessageValidator, TopicConfig, TopicExtractionStrategy,
    TopicRegistry, ValidationPolicy,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;

/// Benchmark topic extraction performance
fn bench_topic_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("topic_extraction");

    // Setup topic registry
    let topic_config = TopicConfig {
        default: "default".to_string(),
        available: vec![
            "market_data_polygon".to_string(),
            "market_data_kraken".to_string(),
            "market_data_binance".to_string(),
            "arbitrage_signals".to_string(),
            "execution_orders".to_string(),
        ],
        auto_discover: false,
        extraction_strategy: TopicExtractionStrategy::SourceType,
    };

    let registry = TopicRegistry::new(&topic_config).unwrap();

    // Create test headers with different source types
    let headers = vec![
        MessageHeader {
            magic: MESSAGE_MAGIC,
            relay_domain: 1,
            version: 1,
            source: 4, // Polygon
            flags: 0,
            sequence: 1,
            timestamp: 1000,
            payload_size: 0,
            checksum: 0,
        },
        MessageHeader {
            magic: MESSAGE_MAGIC,
            relay_domain: 1,
            version: 1,
            source: 2, // Kraken
            flags: 0,
            sequence: 2,
            timestamp: 2000,
            payload_size: 0,
            checksum: 0,
        },
        MessageHeader {
            magic: MESSAGE_MAGIC,
            relay_domain: 2,
            version: 1,
            source: 20, // Arbitrage
            flags: 0,
            sequence: 3,
            timestamp: 3000,
            payload_size: 0,
            checksum: 0,
        },
    ];

    // Benchmark source type extraction
    group.bench_function("source_type", |b| {
        let mut idx = 0;
        b.iter(|| {
            let header = &headers[idx % headers.len()];
            idx += 1;
            black_box(registry.extract_topic(header, None, &TopicExtractionStrategy::SourceType))
        })
    });

    // Benchmark venue extraction
    group.bench_function("venue_based", |b| {
        let mut idx = 0;
        b.iter(|| {
            let header = &headers[idx % headers.len()];
            idx += 1;
            black_box(registry.extract_topic(header, None, &TopicExtractionStrategy::InstrumentVenue))
        })
    });

    // Benchmark fixed topic (fastest)
    group.bench_function("fixed", |b| {
        let mut idx = 0;
        b.iter(|| {
            let header = &headers[idx % headers.len()];
            idx += 1;
            black_box(
                registry
                    .extract_topic(header, None, &TopicExtractionStrategy::Fixed("fixed".to_string())),
            )
        })
    });

    group.finish();
}

/// Benchmark validation performance for different policies
fn bench_validation_policies(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation");

    // Create test data
    let data = vec![0u8; 1024]; // 1KB message
    let checksum = crc32fast::hash(&data);

    let header = MessageHeader {
        magic: MESSAGE_MAGIC,
        relay_domain: 1,
        version: 1,
        source: 4,
        flags: 0,
        sequence: 1,
        timestamp: 1000,
        payload_size: data.len() as u32,
        checksum,
    };

    // Performance validator (no checksum)
    let perf_policy = ValidationPolicy {
        checksum: false,
        audit: false,
        strict: false,
        max_message_size: Some(65536),
    };
    let perf_validator = create_validator(&perf_policy);

    group.bench_function("performance_mode", |b| {
        b.iter(|| black_box(perf_validator.validate(&header, &data)))
    });

    // Reliability validator (checksum validation)
    let reliability_policy = ValidationPolicy {
        checksum: true,
        audit: false,
        strict: false,
        max_message_size: Some(65536),
    };
    let reliability_validator = create_validator(&reliability_policy);

    group.bench_function("reliability_mode", |b| {
        b.iter(|| black_box(reliability_validator.validate(&header, &data)))
    });

    // Security validator (checksum + audit)
    let security_policy = ValidationPolicy {
        checksum: true,
        audit: true,
        strict: true,
        max_message_size: Some(65536),
    };
    let security_validator = create_validator(&security_policy);

    group.bench_function("security_mode", |b| {
        b.iter(|| black_box(security_validator.validate(&header, &data)))
    });

    group.finish();
}

/// Benchmark subscriber lookup performance
fn bench_subscriber_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("subscriber_lookup");

    let topic_config = TopicConfig {
        default: "default".to_string(),
        available: vec![
            "topic1".to_string(),
            "topic2".to_string(),
            "topic3".to_string(),
        ],
        auto_discover: false,
        extraction_strategy: TopicExtractionStrategy::SourceType,
    };

    let registry = TopicRegistry::new(&topic_config).unwrap();

    // Add varying numbers of subscribers
    for num_subscribers in [1, 10, 100, 1000].iter() {
        // Subscribe consumers to topic1
        for i in 0..*num_subscribers {
            let consumer = ConsumerId(format!("consumer_{}", i));
            registry.subscribe(consumer, "topic1").unwrap();
        }

        group.throughput(Throughput::Elements(*num_subscribers as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_subscribers),
            num_subscribers,
            |b, _| b.iter(|| black_box(registry.get_subscribers("topic1"))),
        );

        // Clean up for next iteration
        for i in 0..*num_subscribers {
            let consumer = ConsumerId(format!("consumer_{}", i));
            registry.unsubscribe_all(&consumer).unwrap();
        }
    }

    group.finish();
}

/// Benchmark header parsing performance
fn bench_header_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("header_parsing");

    // Create a valid message with header
    let header = MessageHeader {
        magic: MESSAGE_MAGIC,
        relay_domain: 1,
        version: 1,
        source: 4,
        flags: 0,
        sequence: 12345,
        timestamp: 1234567890,
        payload_size: 100,
        checksum: 0xDEADBEEF,
    };

    let header_bytes = unsafe {
        std::slice::from_raw_parts(
            &header as *const _ as *const u8,
            std::mem::size_of::<MessageHeader>(),
        )
    };

    let mut message = header_bytes.to_vec();
    message.extend_from_slice(&[0u8; 100]); // Add payload

    // Benchmark zero-copy header parsing
    group.bench_function("zero_copy_parse", |b| {
        b.iter(|| {
            let data = black_box(&message);
            if data.len() >= std::mem::size_of::<MessageHeader>() {
                let header = unsafe { &*(data.as_ptr() as *const MessageHeader) };
                black_box(header.magic == MESSAGE_MAGIC);
            }
        })
    });

    // Benchmark with magic number validation
    group.bench_function("with_validation", |b| {
        b.iter(|| {
            let data = black_box(&message);
            if data.len() >= std::mem::size_of::<MessageHeader>() {
                let header = unsafe { &*(data.as_ptr() as *const MessageHeader) };
                black_box(
                    header.magic == MESSAGE_MAGIC
                        && header.version == 1
                        && header.relay_domain > 0
                        && header.relay_domain <= 3,
                );
            }
        })
    });

    group.finish();
}

/// Benchmark end-to-end message routing
fn bench_message_routing(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_routing");
    group.measurement_time(Duration::from_secs(10));

    // Setup
    let topic_config = TopicConfig {
        default: "default".to_string(),
        available: vec![
            "market_data_polygon".to_string(),
            "market_data_kraken".to_string(),
        ],
        auto_discover: false,
        extraction_strategy: TopicExtractionStrategy::SourceType,
    };

    let registry = TopicRegistry::new(&topic_config).unwrap();

    // Subscribe consumers
    for i in 0..100 {
        let consumer = ConsumerId(format!("polygon_consumer_{}", i));
        registry.subscribe(consumer, "market_data_polygon").unwrap();
    }

    for i in 0..50 {
        let consumer = ConsumerId(format!("kraken_consumer_{}", i));
        registry.subscribe(consumer, "market_data_kraken").unwrap();
    }

    // Create test messages
    let polygon_header = MessageHeader {
        magic: MESSAGE_MAGIC,
        relay_domain: 1,
        version: 1,
        source: 4, // Polygon
        flags: 0,
        sequence: 1,
        timestamp: 1000,
        payload_size: 0,
        checksum: 0,
    };

    let kraken_header = MessageHeader {
        magic: MESSAGE_MAGIC,
        relay_domain: 1,
        version: 1,
        source: 2, // Kraken
        flags: 0,
        sequence: 2,
        timestamp: 2000,
        payload_size: 0,
        checksum: 0,
    };

    let headers = vec![polygon_header, kraken_header];

    // Benchmark complete routing operation
    group.bench_function("full_routing", |b| {
        let mut idx = 0;
        b.iter(|| {
            let header = &headers[idx % headers.len()];
            idx += 1;

            // Extract topic
            let topic = registry
                .extract_topic(header, None, &TopicExtractionStrategy::SourceType)
                .unwrap();

            // Get subscribers
            let subscribers = registry.get_subscribers(&topic);

            // Simulate sending to subscribers
            black_box(subscribers.len());
        })
    });

    group.finish();
}

/// Benchmark checksum calculation performance
fn bench_checksum_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("checksum");

    for size in [64, 256, 1024, 4096, 16384].iter() {
        let data = vec![0u8; *size];

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| black_box(crc32fast::hash(data)))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_topic_extraction,
    bench_validation_policies,
    bench_subscriber_lookup,
    bench_header_parsing,
    bench_message_routing,
    bench_checksum_performance
);

criterion_main!(benches);
