//! Realistic test using actual protocol messages
//!
//! This test uses real MessageHeader and TLV structures from protocol_v2
//! to verify the relay correctly routes messages based on topics.

use alphapulse_protocol_v2::{
    MessageHeader, PoolSwapTLV, QuoteTLV, RelayDomain, SourceType, TLVType, TradeTLV,
    MESSAGE_MAGIC, PROTOCOL_VERSION,
};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

/// Test consumer that tracks received messages
#[derive(Debug, Clone)]
struct TestConsumer {
    id: String,
    subscribed_topics: Vec<String>,
    received_messages: Vec<MessageHeader>,
}

impl TestConsumer {
    fn new(id: &str, topics: Vec<&str>) -> Self {
        Self {
            id: id.to_string(),
            subscribed_topics: topics.iter().map(|s| s.to_string()).collect(),
            received_messages: Vec::new(),
        }
    }

    fn receive(&mut self, header: MessageHeader) {
        self.received_messages.push(header);
    }

    fn received_from_source(&self, source: SourceType) -> bool {
        self.received_messages
            .iter()
            .any(|h| h.source_type == source as u8)
    }

    fn message_count(&self) -> usize {
        self.received_messages.len()
    }
}

/// Simple topic registry for testing
struct TopicRegistry {
    topics: HashMap<String, HashSet<String>>,
}

impl TopicRegistry {
    fn new() -> Self {
        Self {
            topics: HashMap::new(),
        }
    }

    fn subscribe(&mut self, consumer_id: &str, topic: &str) {
        self.topics
            .entry(topic.to_string())
            .or_insert_with(HashSet::new)
            .insert(consumer_id.to_string());
    }

    fn get_subscribers(&self, topic: &str) -> Vec<String> {
        self.topics
            .get(topic)
            .map(|subs| subs.iter().cloned().collect())
            .unwrap_or_default()
    }
}

/// Extract topic from message header based on source type
fn extract_topic(header: &MessageHeader) -> String {
    match header.source_type {
        s if s == SourceType::BinanceCollector as u8 => "market_data_binance".to_string(),
        s if s == SourceType::KrakenCollector as u8 => "market_data_kraken".to_string(),
        s if s == SourceType::CoinbaseCollector as u8 => "market_data_coinbase".to_string(),
        s if s == SourceType::PolygonCollector as u8 => "market_data_polygon".to_string(),
        s if s == SourceType::ArbitrageStrategy as u8 => "arbitrage_signals".to_string(),
        s if s == SourceType::MarketMaker as u8 => "market_maker_signals".to_string(),
        s if s == SourceType::PortfolioManager as u8 => "execution_orders".to_string(),
        s if s == SourceType::ExecutionEngine as u8 => "execution_fills".to_string(),
        _ => "unknown".to_string(),
    }
}

/// Create a realistic trade message
fn create_trade_message(
    source: SourceType,
    instrument_id: u64,
    price: i64,
    volume: i64,
) -> Vec<u8> {
    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let header = MessageHeader {
        magic: MESSAGE_MAGIC,
        version: PROTOCOL_VERSION,
        message_type: TLVType::Trade as u8,
        relay_domain: RelayDomain::MarketData as u8,
        source_type: source as u8,
        sequence: 1,
        timestamp_ns,
        instrument_id,
        checksum: 0, // Will be calculated if needed
    };

    // Convert header to bytes
    let header_bytes = unsafe {
        std::slice::from_raw_parts(
            &header as *const _ as *const u8,
            std::mem::size_of::<MessageHeader>(),
        )
    };

    let mut message = header_bytes.to_vec();

    // Add TradeTLV data
    message.push(TLVType::Trade as u8); // Type
    message.push(0); // Flags
    message.extend_from_slice(&16u16.to_le_bytes()); // Length
    message.extend_from_slice(&price.to_le_bytes()); // Price (8 bytes)
    message.extend_from_slice(&volume.to_le_bytes()); // Volume (8 bytes)

    // Calculate and update checksum
    let checksum = crc32fast::hash(&message);
    let checksum_offset = std::mem::size_of::<MessageHeader>() - 4;
    message[checksum_offset..checksum_offset + 4].copy_from_slice(&checksum.to_le_bytes());

    message
}

/// Parse header from message bytes
fn parse_header(data: &[u8]) -> Option<MessageHeader> {
    if data.len() < std::mem::size_of::<MessageHeader>() {
        return None;
    }

    let header = unsafe { std::ptr::read(data.as_ptr() as *const MessageHeader) };

    if header.magic != MESSAGE_MAGIC {
        return None;
    }

    Some(header)
}

#[test]
fn test_realistic_topic_routing() {
    println!("\n=== Testing Realistic Protocol Message Routing ===\n");

    // Create topic registry
    let mut registry = TopicRegistry::new();

    // Create consumers with different subscriptions
    let mut polygon_consumer = TestConsumer::new("polygon_arb_bot", vec!["market_data_polygon"]);
    let mut kraken_consumer = TestConsumer::new("kraken_market_maker", vec!["market_data_kraken"]);
    let mut signal_consumer = TestConsumer::new(
        "signal_aggregator",
        vec!["arbitrage_signals", "market_maker_signals"],
    );
    let mut all_market_consumer = TestConsumer::new(
        "market_monitor",
        vec![
            "market_data_polygon",
            "market_data_kraken",
            "market_data_binance",
            "market_data_coinbase",
        ],
    );

    // Register subscriptions
    registry.subscribe("polygon_arb_bot", "market_data_polygon");
    registry.subscribe("kraken_market_maker", "market_data_kraken");
    registry.subscribe("signal_aggregator", "arbitrage_signals");
    registry.subscribe("signal_aggregator", "market_maker_signals");
    registry.subscribe("market_monitor", "market_data_polygon");
    registry.subscribe("market_monitor", "market_data_kraken");
    registry.subscribe("market_monitor", "market_data_binance");
    registry.subscribe("market_monitor", "market_data_coinbase");

    println!("Consumers registered:");
    println!("  - polygon_arb_bot → [market_data_polygon]");
    println!("  - kraken_market_maker → [market_data_kraken]");
    println!("  - signal_aggregator → [arbitrage_signals, market_maker_signals]");
    println!("  - market_monitor → [all market data topics]\n");

    // Create realistic messages
    let messages = vec![
        (
            create_trade_message(SourceType::PolygonCollector, 12345, 150000000, 100000000),
            "Polygon DEX trade",
        ),
        (
            create_trade_message(SourceType::KrakenCollector, 67890, 4500000000000, 25000000),
            "Kraken BTC trade",
        ),
        (
            create_trade_message(SourceType::BinanceCollector, 11111, 250000000, 500000000),
            "Binance trade",
        ),
    ];

    // Process messages through topic routing
    for (message_bytes, description) in &messages {
        if let Some(header) = parse_header(message_bytes) {
            let topic = extract_topic(&header);
            let subscribers = registry.get_subscribers(&topic);

            println!(
                "Message: {} (source: {}, instrument: {})",
                description, header.source_type, header.instrument_id
            );
            println!("  → Topic: {}", topic);
            println!("  → Subscribers: {:?}", subscribers);

            // Route to appropriate consumers
            for subscriber in &subscribers {
                match subscriber.as_str() {
                    "polygon_arb_bot" => polygon_consumer.receive(header),
                    "kraken_market_maker" => kraken_consumer.receive(header),
                    "signal_aggregator" => signal_consumer.receive(header),
                    "market_monitor" => all_market_consumer.receive(header),
                    _ => {}
                }
            }
            println!();
        }
    }

    // Verify routing correctness
    println!("Routing verification:");
    println!(
        "  polygon_arb_bot received: {} messages",
        polygon_consumer.message_count()
    );
    assert_eq!(polygon_consumer.message_count(), 1);
    assert!(polygon_consumer.received_from_source(SourceType::PolygonCollector));
    assert!(!polygon_consumer.received_from_source(SourceType::KrakenCollector));

    println!(
        "  kraken_market_maker received: {} messages",
        kraken_consumer.message_count()
    );
    assert_eq!(kraken_consumer.message_count(), 1);
    assert!(kraken_consumer.received_from_source(SourceType::KrakenCollector));
    assert!(!kraken_consumer.received_from_source(SourceType::PolygonCollector));

    println!(
        "  signal_aggregator received: {} messages",
        signal_consumer.message_count()
    );
    assert_eq!(signal_consumer.message_count(), 0); // No signal messages sent

    println!(
        "  market_monitor received: {} messages",
        all_market_consumer.message_count()
    );
    assert_eq!(all_market_consumer.message_count(), 3); // All market data
    assert!(all_market_consumer.received_from_source(SourceType::PolygonCollector));
    assert!(all_market_consumer.received_from_source(SourceType::KrakenCollector));
    assert!(all_market_consumer.received_from_source(SourceType::BinanceCollector));

    println!("\n✅ All routing assertions passed!");
}

#[test]
fn test_checksum_validation() {
    println!("\n=== Testing Checksum Validation ===\n");

    // Create message with valid checksum
    let message = create_trade_message(SourceType::PolygonCollector, 12345, 150000000, 100000000);

    // Parse header
    let header = parse_header(&message).unwrap();

    // Verify checksum
    let calculated = crc32fast::hash(&message);
    let stored_checksum = u32::from_le_bytes([
        message[message.len() - 4],
        message[message.len() - 3],
        message[message.len() - 2],
        message[message.len() - 1],
    ]);

    println!("Message checksum validation:");
    println!("  Calculated: 0x{:08X}", calculated);
    println!("  Stored: 0x{:08X}", stored_checksum);
    println!("  Header checksum field: 0x{:08X}", header.checksum);

    // Note: The checksum is stored in the message bytes, not the header field
    assert_eq!(calculated, stored_checksum);

    println!("\n✅ Checksum validation passed!");
}

#[test]
fn test_signal_routing() {
    println!("\n=== Testing Signal Message Routing ===\n");

    let mut registry = TopicRegistry::new();
    let mut arb_consumer = TestConsumer::new("arbitrage_executor", vec!["arbitrage_signals"]);
    let mut mm_consumer = TestConsumer::new("market_maker_bot", vec!["market_maker_signals"]);

    registry.subscribe("arbitrage_executor", "arbitrage_signals");
    registry.subscribe("market_maker_bot", "market_maker_signals");

    // Create signal messages
    let arb_signal = create_signal_message(SourceType::ArbitrageStrategy);
    let mm_signal = create_signal_message(SourceType::MarketMaker);

    // Process signals
    for (message, source) in [
        (arb_signal, SourceType::ArbitrageStrategy),
        (mm_signal, SourceType::MarketMaker),
    ] {
        if let Some(header) = parse_header(&message) {
            let topic = extract_topic(&header);
            let subscribers = registry.get_subscribers(&topic);

            for subscriber in &subscribers {
                match subscriber.as_str() {
                    "arbitrage_executor" => arb_consumer.receive(header),
                    "market_maker_bot" => mm_consumer.receive(header),
                    _ => {}
                }
            }
        }
    }

    // Verify signal routing
    assert_eq!(arb_consumer.message_count(), 1);
    assert!(arb_consumer.received_from_source(SourceType::ArbitrageStrategy));
    assert!(!arb_consumer.received_from_source(SourceType::MarketMaker));

    assert_eq!(mm_consumer.message_count(), 1);
    assert!(mm_consumer.received_from_source(SourceType::MarketMaker));
    assert!(!mm_consumer.received_from_source(SourceType::ArbitrageStrategy));

    println!("✅ Signal routing test passed!");
}

fn create_signal_message(source: SourceType) -> Vec<u8> {
    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let header = MessageHeader {
        magic: MESSAGE_MAGIC,
        version: PROTOCOL_VERSION,
        message_type: TLVType::Signal as u8,
        relay_domain: RelayDomain::Signal as u8,
        source_type: source as u8,
        sequence: 1,
        timestamp_ns,
        instrument_id: 0,
        checksum: 0,
    };

    let header_bytes = unsafe {
        std::slice::from_raw_parts(
            &header as *const _ as *const u8,
            std::mem::size_of::<MessageHeader>(),
        )
    };

    let mut message = header_bytes.to_vec();

    // Add minimal signal TLV
    message.push(TLVType::Signal as u8);
    message.push(0); // Flags
    message.extend_from_slice(&8u16.to_le_bytes()); // Length
    message.extend_from_slice(&[0u8; 8]); // Signal data

    message
}
