//! Integration tests for topic-based coarse filtering

use torq_relays::{
    ConsumerId, Relay, RelayConfig, RelayResult, TopicConfig, TopicExtractionStrategy,
    TopicRegistry,
};
use protocol_v2::{MessageHeader, MESSAGE_MAGIC};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Mock consumer that tracks received messages
#[derive(Clone)]
struct MockConsumer {
    id: ConsumerId,
    subscribed_topics: Vec<String>,
    received_messages: Arc<Mutex<Vec<TestMessage>>>,
}

#[derive(Debug, Clone)]
struct TestMessage {
    topic: String,
    source_type: u8,
    instrument_id: u64,
    data: Vec<u8>,
}

impl MockConsumer {
    fn new(id: &str, topics: Vec<String>) -> Self {
        Self {
            id: ConsumerId(id.to_string()),
            subscribed_topics: topics,
            received_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn message_count(&self) -> usize {
        self.received_messages.lock().await.len()
    }

    async fn has_message_from_source(&self, source_type: u8) -> bool {
        let messages = self.received_messages.lock().await;
        messages.iter().any(|m| m.source_type == source_type)
    }
}

/// Test that consumers only receive messages for their subscribed topics
#[tokio::test]
async fn test_topic_based_filtering() {
    // Create topic registry with test configuration
    let topic_config = TopicConfig {
        default: "default".to_string(),
        available: vec![
            "market_data_polygon".to_string(),
            "market_data_kraken".to_string(),
            "arbitrage_signals".to_string(),
        ],
        auto_discover: false,
        extraction_strategy: TopicExtractionStrategy::SourceType,
    };

    let registry = TopicRegistry::new(&topic_config).unwrap();

    // Create mock consumers with different topic subscriptions
    let polygon_consumer =
        MockConsumer::new("polygon_consumer", vec!["market_data_polygon".to_string()]);

    let kraken_consumer =
        MockConsumer::new("kraken_consumer", vec!["market_data_kraken".to_string()]);

    let all_market_consumer = MockConsumer::new(
        "all_market_consumer",
        vec![
            "market_data_polygon".to_string(),
            "market_data_kraken".to_string(),
        ],
    );

    let signal_consumer =
        MockConsumer::new("signal_consumer", vec!["arbitrage_signals".to_string()]);

    // Subscribe consumers to their topics
    registry
        .subscribe(polygon_consumer.id.clone(), "market_data_polygon")
        .unwrap();
    registry
        .subscribe(kraken_consumer.id.clone(), "market_data_kraken")
        .unwrap();
    registry
        .subscribe(all_market_consumer.id.clone(), "market_data_polygon")
        .unwrap();
    registry
        .subscribe(all_market_consumer.id.clone(), "market_data_kraken")
        .unwrap();
    registry
        .subscribe(signal_consumer.id.clone(), "arbitrage_signals")
        .unwrap();

    // Test Polygon market data message (source_type = 4)
    let polygon_header = MessageHeader {
        magic: MESSAGE_MAGIC,
        version: 1,
        message_type: 1, // Trade
        relay_domain: 1, // Market data
        source_type: 4,  // Polygon
        sequence: 1,
        timestamp_ns: 1000,
        instrument_id: 123,
        checksum: 0,
    };

    let topic = registry
        .extract_topic(&polygon_header, &TopicExtractionStrategy::SourceType)
        .unwrap();
    assert_eq!(topic, "market_data_polygon");

    let subscribers = registry.get_subscribers(&topic);
    assert_eq!(subscribers.len(), 2); // polygon_consumer and all_market_consumer
    assert!(subscribers.contains(&polygon_consumer.id));
    assert!(subscribers.contains(&all_market_consumer.id));
    assert!(!subscribers.contains(&kraken_consumer.id));
    assert!(!subscribers.contains(&signal_consumer.id));

    // Test Kraken market data message (source_type = 2)
    let kraken_header = MessageHeader {
        magic: MESSAGE_MAGIC,
        version: 1,
        message_type: 1, // Trade
        relay_domain: 1, // Market data
        source_type: 2,  // Kraken
        sequence: 2,
        timestamp_ns: 2000,
        instrument_id: 456,
        checksum: 0,
    };

    let topic = registry
        .extract_topic(&kraken_header, &TopicExtractionStrategy::SourceType)
        .unwrap();
    assert_eq!(topic, "market_data_kraken");

    let subscribers = registry.get_subscribers(&topic);
    assert_eq!(subscribers.len(), 2); // kraken_consumer and all_market_consumer
    assert!(subscribers.contains(&kraken_consumer.id));
    assert!(subscribers.contains(&all_market_consumer.id));
    assert!(!subscribers.contains(&polygon_consumer.id));
    assert!(!subscribers.contains(&signal_consumer.id));

    // Test arbitrage signal message (source_type = 20)
    let signal_header = MessageHeader {
        magic: MESSAGE_MAGIC,
        version: 1,
        message_type: 50, // Signal
        relay_domain: 2,  // Signals
        source_type: 20,  // Arbitrage strategy
        sequence: 3,
        timestamp_ns: 3000,
        instrument_id: 789,
        checksum: 0,
    };

    let topic = registry
        .extract_topic(&signal_header, &TopicExtractionStrategy::SourceType)
        .unwrap();
    assert_eq!(topic, "arbitrage_signals");

    let subscribers = registry.get_subscribers(&topic);
    assert_eq!(subscribers.len(), 1); // Only signal_consumer
    assert!(subscribers.contains(&signal_consumer.id));
    assert!(!subscribers.contains(&polygon_consumer.id));
    assert!(!subscribers.contains(&kraken_consumer.id));
    assert!(!subscribers.contains(&all_market_consumer.id));
}

/// Test venue-based topic extraction for DEX routing
#[tokio::test]
async fn test_venue_based_topic_extraction() {
    let topic_config = TopicConfig {
        default: "default".to_string(),
        available: vec![
            "market_data_uniswap_v2".to_string(),
            "market_data_uniswap_v3".to_string(),
            "market_data_sushiswap".to_string(),
        ],
        auto_discover: false,
        extraction_strategy: TopicExtractionStrategy::InstrumentVenue,
    };

    let registry = TopicRegistry::new(&topic_config).unwrap();

    // Create consumers for different DEX venues
    let uniswap_v2_consumer = ConsumerId("uniswap_v2_arb".to_string());
    let uniswap_v3_consumer = ConsumerId("uniswap_v3_arb".to_string());

    registry
        .subscribe(uniswap_v2_consumer.clone(), "market_data_uniswap_v2")
        .unwrap();
    registry
        .subscribe(uniswap_v3_consumer.clone(), "market_data_uniswap_v3")
        .unwrap();

    // Test Uniswap V2 message
    // Instrument ID format: [exchange:8][base:8][quote:8][type:8][venue:16][reserved:16]
    let uniswap_v2_instrument = 0x0000000100000000; // venue = 1 (Uniswap V2)

    let header = MessageHeader {
        magic: MESSAGE_MAGIC,
        version: 1,
        message_type: 1,
        relay_domain: 1,
        source_type: 4, // Polygon collector
        sequence: 1,
        timestamp_ns: 1000,
        instrument_id: uniswap_v2_instrument,
        checksum: 0,
    };

    let topic = registry
        .extract_topic(&header, &TopicExtractionStrategy::InstrumentVenue)
        .unwrap();
    assert_eq!(topic, "market_data_uniswap_v2");

    let subscribers = registry.get_subscribers(&topic);
    assert_eq!(subscribers.len(), 1);
    assert!(subscribers.contains(&uniswap_v2_consumer));
    assert!(!subscribers.contains(&uniswap_v3_consumer));

    // Test Uniswap V3 message
    let uniswap_v3_instrument = 0x0000000200000000; // venue = 2 (Uniswap V3)

    let header = MessageHeader {
        magic: MESSAGE_MAGIC,
        version: 1,
        message_type: 1,
        relay_domain: 1,
        source_type: 4,
        sequence: 2,
        timestamp_ns: 2000,
        instrument_id: uniswap_v3_instrument,
        checksum: 0,
    };

    let topic = registry
        .extract_topic(&header, &TopicExtractionStrategy::InstrumentVenue)
        .unwrap();
    assert_eq!(topic, "market_data_uniswap_v3");

    let subscribers = registry.get_subscribers(&topic);
    assert_eq!(subscribers.len(), 1);
    assert!(subscribers.contains(&uniswap_v3_consumer));
    assert!(!subscribers.contains(&uniswap_v2_consumer));
}

/// Test auto-discovery of new topics
#[tokio::test]
async fn test_topic_auto_discovery() {
    let topic_config = TopicConfig {
        default: "default".to_string(),
        available: vec!["existing_topic".to_string()],
        auto_discover: true, // Enable auto-discovery
        extraction_strategy: TopicExtractionStrategy::SourceType,
    };

    let registry = TopicRegistry::new(&topic_config).unwrap();

    // Subscribe to a new topic that doesn't exist yet
    let consumer = ConsumerId("explorer".to_string());

    // This should succeed with auto-discovery
    registry
        .subscribe(consumer.clone(), "new_discovered_topic")
        .unwrap();

    // Verify the topic was created
    let topics = registry.list_topics();
    assert!(topics.contains(&"new_discovered_topic".to_string()));

    // Verify subscription works
    let subscribers = registry.get_subscribers("new_discovered_topic");
    assert_eq!(subscribers.len(), 1);
    assert!(subscribers.contains(&consumer));
}

/// Test that fixed topic strategy sends all messages to same topic
#[tokio::test]
async fn test_fixed_topic_strategy() {
    let topic_config = TopicConfig {
        default: "default".to_string(),
        available: vec!["execution_all".to_string()],
        auto_discover: false,
        extraction_strategy: TopicExtractionStrategy::Fixed("execution_all".to_string()),
    };

    let registry = TopicRegistry::new(&topic_config).unwrap();

    // All messages should go to "execution_all" regardless of source
    let headers = vec![
        MessageHeader {
            magic: MESSAGE_MAGIC,
            version: 1,
            message_type: 60, // Order
            relay_domain: 3,  // Execution
            source_type: 40,  // Portfolio manager
            sequence: 1,
            timestamp_ns: 1000,
            instrument_id: 123,
            checksum: 0,
        },
        MessageHeader {
            magic: MESSAGE_MAGIC,
            version: 1,
            message_type: 61, // Fill
            relay_domain: 3,
            source_type: 42, // Execution engine
            sequence: 2,
            timestamp_ns: 2000,
            instrument_id: 456,
            checksum: 0,
        },
    ];

    for header in headers {
        let topic = registry
            .extract_topic(
                &header,
                &TopicExtractionStrategy::Fixed("execution_all".to_string()),
            )
            .unwrap();
        assert_eq!(topic, "execution_all");
    }
}

/// Test consumer subscription management
#[tokio::test]
async fn test_consumer_subscription_lifecycle() {
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
    let consumer = ConsumerId("test_consumer".to_string());

    // Subscribe to multiple topics
    registry.subscribe(consumer.clone(), "topic1").unwrap();
    registry.subscribe(consumer.clone(), "topic2").unwrap();

    // Verify subscriptions
    let consumer_topics = registry.get_consumer_topics(&consumer);
    assert_eq!(consumer_topics.len(), 2);
    assert!(consumer_topics.contains(&"topic1".to_string()));
    assert!(consumer_topics.contains(&"topic2".to_string()));

    // Verify topic subscribers
    assert_eq!(registry.subscriber_count("topic1"), 1);
    assert_eq!(registry.subscriber_count("topic2"), 1);
    assert_eq!(registry.subscriber_count("topic3"), 0);

    // Unsubscribe from one topic
    registry.unsubscribe(&consumer, "topic1").unwrap();

    // Verify partial unsubscription
    let consumer_topics = registry.get_consumer_topics(&consumer);
    assert_eq!(consumer_topics.len(), 1);
    assert!(!consumer_topics.contains(&"topic1".to_string()));
    assert!(consumer_topics.contains(&"topic2".to_string()));

    assert_eq!(registry.subscriber_count("topic1"), 0);
    assert_eq!(registry.subscriber_count("topic2"), 1);

    // Unsubscribe from all remaining topics
    registry.unsubscribe_all(&consumer).unwrap();

    // Verify complete unsubscription
    let consumer_topics = registry.get_consumer_topics(&consumer);
    assert_eq!(consumer_topics.len(), 0);
    assert_eq!(registry.subscriber_count("topic2"), 0);
}

/// Test topic registry statistics
#[tokio::test]
async fn test_topic_registry_stats() {
    let topic_config = TopicConfig {
        default: "default".to_string(),
        available: vec!["topic1".to_string(), "topic2".to_string()],
        auto_discover: true,
        extraction_strategy: TopicExtractionStrategy::SourceType,
    };

    let registry = TopicRegistry::new(&topic_config).unwrap();

    // Initial stats
    let stats = registry.stats();
    assert_eq!(stats.total_topics, 3); // 2 available + 1 default
    assert_eq!(stats.total_consumers, 0);
    assert_eq!(stats.total_subscriptions, 0);

    // Add consumers and subscriptions
    let consumer1 = ConsumerId("consumer1".to_string());
    let consumer2 = ConsumerId("consumer2".to_string());

    registry.subscribe(consumer1.clone(), "topic1").unwrap();
    registry.subscribe(consumer1.clone(), "topic2").unwrap();
    registry.subscribe(consumer2.clone(), "topic1").unwrap();
    registry.subscribe(consumer2.clone(), "new_topic").unwrap(); // Auto-discovered

    // Check updated stats
    let stats = registry.stats();
    assert_eq!(stats.total_topics, 4); // Added "new_topic"
    assert_eq!(stats.total_consumers, 2);
    assert_eq!(stats.total_subscriptions, 4);
}
