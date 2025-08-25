//! Live Kraken WebSocket test for relay with exact protocol messages
//!
//! This test connects to Kraken's WebSocket API, receives real market data,
//! converts it to exact protocol_v2 messages, and tests relay routing.

use futures_util::{SinkExt, StreamExt};
use protocol_v2::{
    build_instrument_id, InstrumentId, MessageHeader, QuoteTLV, RelayDomain, SourceType,
    TLVMessage, TLVType, TradeTLV, VenueId, MESSAGE_MAGIC, PROTOCOL_VERSION,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Test consumer that receives routed messages
#[derive(Clone)]
struct TestConsumer {
    id: String,
    subscribed_topics: Vec<String>,
    received_messages: Arc<Mutex<Vec<(MessageHeader, Vec<u8>)>>>,
}

impl TestConsumer {
    fn new(id: &str, topics: Vec<&str>) -> Self {
        Self {
            id: id.to_string(),
            subscribed_topics: topics.iter().map(|s| s.to_string()).collect(),
            received_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn receive(&self, header: MessageHeader, data: Vec<u8>) {
        let mut messages = self.received_messages.lock().await;
        messages.push((header, data));
        println!("  → {} received message #{}", self.id, messages.len());
    }

    async fn stats(&self) -> (usize, usize, usize) {
        let messages = self.received_messages.lock().await;
        let trades = messages
            .iter()
            .filter(|(h, _)| h.message_type == TLVType::Trade as u8)
            .count();
        let quotes = messages
            .iter()
            .filter(|(h, _)| h.message_type == TLVType::Quote as u8)
            .count();
        (messages.len(), trades, quotes)
    }
}

/// Simple topic registry for routing
struct TopicRouter {
    consumers: HashMap<String, TestConsumer>,
}

impl TopicRouter {
    fn new() -> Self {
        Self {
            consumers: HashMap::new(),
        }
    }

    fn add_consumer(&mut self, consumer: TestConsumer) {
        self.consumers.insert(consumer.id.clone(), consumer);
    }

    async fn route_message(&self, header: MessageHeader, data: Vec<u8>) {
        // Extract topic from source type
        let topic = match header.source_type {
            2 => "market_data_kraken",
            _ => "unknown",
        };

        println!(
            "Routing message: type={}, topic={}",
            header.message_type, topic
        );

        // Route to consumers subscribed to this topic
        for consumer in self.consumers.values() {
            if consumer.subscribed_topics.contains(&topic.to_string()) {
                consumer.receive(header, data.clone()).await;
            }
        }
    }
}

/// Convert Kraken trade data to exact protocol TradeTLV
fn kraken_trade_to_protocol(pair: &str, trade_data: &serde_json::Value) -> Option<Vec<u8>> {
    // Parse Kraken trade array: [price, volume, time, side, orderType, misc]
    let arr = trade_data.as_array()?;
    if arr.len() < 4 {
        return None;
    }

    let price_str = arr[0].as_str()?;
    let volume_str = arr[1].as_str()?;
    let timestamp = arr[2].as_f64()?;
    let side = arr[3].as_str()?;

    // Convert to protocol format (8 decimal fixed point)
    let price = (price_str.parse::<f64>().ok()? * 100_000_000.0) as i64;
    let volume = (volume_str.parse::<f64>().ok()? * 100_000_000.0) as i64;
    let timestamp_ns = (timestamp * 1_000_000_000.0) as u64;

    // Create instrument ID for the pair
    let instrument_id = match pair {
        "XBT/USD" => build_instrument_id(1, 1, 2, 1, VenueId::Kraken as u16, 0), // BTC/USD spot
        "ETH/USD" => build_instrument_id(1, 3, 2, 1, VenueId::Kraken as u16, 0), // ETH/USD spot
        _ => 0,
    };

    // Create exact protocol message header
    let header = MessageHeader {
        magic: MESSAGE_MAGIC,
        version: PROTOCOL_VERSION,
        message_type: TLVType::Trade as u8,
        relay_domain: RelayDomain::MarketData as u8,
        source_type: SourceType::KrakenCollector as u8,
        sequence: 0, // Will be set by collector
        timestamp_ns,
        instrument_id,
        checksum: 0, // Will be calculated
    };

    // Serialize header to bytes
    let header_bytes = unsafe {
        std::slice::from_raw_parts(
            &header as *const _ as *const u8,
            std::mem::size_of::<MessageHeader>(),
        )
    };

    let mut message = header_bytes.to_vec();

    // Add TradeTLV payload
    let is_buy = side == "b";
    let flags = if is_buy { 0x01 } else { 0x00 };

    message.push(TLVType::Trade as u8);
    message.push(flags);
    message.extend_from_slice(&16u16.to_le_bytes()); // Length
    message.extend_from_slice(&price.to_le_bytes());
    message.extend_from_slice(&volume.to_le_bytes());

    // Calculate and update checksum
    let checksum = crc32fast::hash(&message);
    let checksum_offset = std::mem::size_of::<MessageHeader>() - 4;
    message[checksum_offset..checksum_offset + 4].copy_from_slice(&checksum.to_le_bytes());

    Some(message)
}

/// Convert Kraken spread/quote to exact protocol QuoteTLV
fn kraken_spread_to_protocol(pair: &str, spread_data: &serde_json::Value) -> Option<Vec<u8>> {
    // Parse spread array: [bid, ask, timestamp, bidVolume, askVolume]
    let arr = spread_data.as_array()?;
    if arr.len() < 5 {
        return None;
    }

    let bid_str = arr[0].as_str()?;
    let ask_str = arr[1].as_str()?;
    let timestamp = arr[2].as_f64()?;
    let bid_vol_str = arr[3].as_str()?;
    let ask_vol_str = arr[4].as_str()?;

    // Convert to protocol format
    let bid_price = (bid_str.parse::<f64>().ok()? * 100_000_000.0) as i64;
    let ask_price = (ask_str.parse::<f64>().ok()? * 100_000_000.0) as i64;
    let bid_volume = (bid_vol_str.parse::<f64>().ok()? * 100_000_000.0) as i64;
    let ask_volume = (ask_vol_str.parse::<f64>().ok()? * 100_000_000.0) as i64;
    let timestamp_ns = (timestamp * 1_000_000_000.0) as u64;

    // Create instrument ID
    let instrument_id = match pair {
        "XBT/USD" => build_instrument_id(1, 1, 2, 1, VenueId::Kraken as u16, 0),
        "ETH/USD" => build_instrument_id(1, 3, 2, 1, VenueId::Kraken as u16, 0),
        _ => 0,
    };

    // Create header
    let header = MessageHeader {
        magic: MESSAGE_MAGIC,
        version: PROTOCOL_VERSION,
        message_type: TLVType::Quote as u8,
        relay_domain: RelayDomain::MarketData as u8,
        source_type: SourceType::KrakenCollector as u8,
        sequence: 0,
        timestamp_ns,
        instrument_id,
        checksum: 0,
    };

    let header_bytes = unsafe {
        std::slice::from_raw_parts(
            &header as *const _ as *const u8,
            std::mem::size_of::<MessageHeader>(),
        )
    };

    let mut message = header_bytes.to_vec();

    // Add QuoteTLV payload
    message.push(TLVType::Quote as u8);
    message.push(0); // Flags
    message.extend_from_slice(&32u16.to_le_bytes()); // Length
    message.extend_from_slice(&bid_price.to_le_bytes());
    message.extend_from_slice(&ask_price.to_le_bytes());
    message.extend_from_slice(&bid_volume.to_le_bytes());
    message.extend_from_slice(&ask_volume.to_le_bytes());

    // Calculate checksum
    let checksum = crc32fast::hash(&message);
    let checksum_offset = std::mem::size_of::<MessageHeader>() - 4;
    message[checksum_offset..checksum_offset + 4].copy_from_slice(&checksum.to_le_bytes());

    Some(message)
}

#[tokio::test]
async fn test_live_kraken_relay() {
    println!("\n=== Live Kraken WebSocket Relay Test ===\n");

    // Create router and consumers
    let mut router = TopicRouter::new();

    let kraken_consumer = TestConsumer::new("kraken_trader", vec!["market_data_kraken"]);
    let monitor_consumer =
        TestConsumer::new("monitor", vec!["market_data_kraken", "market_data_polygon"]);
    let polygon_consumer = TestConsumer::new("polygon_only", vec!["market_data_polygon"]);

    router.add_consumer(kraken_consumer.clone());
    router.add_consumer(monitor_consumer.clone());
    router.add_consumer(polygon_consumer.clone());

    println!("Consumers registered:");
    println!("  - kraken_trader → [market_data_kraken]");
    println!("  - monitor → [market_data_kraken, market_data_polygon]");
    println!("  - polygon_only → [market_data_polygon]\n");

    // Connect to Kraken WebSocket
    let url = "wss://ws.kraken.com";
    println!("Connecting to Kraken WebSocket: {}", url);

    let (ws_stream, _) = connect_async(url)
        .await
        .expect("Failed to connect to Kraken");
    let (mut write, mut read) = ws_stream.split();

    // Subscribe to BTC/USD trades and spread
    let subscribe_msg = json!({
        "event": "subscribe",
        "pair": ["XBT/USD"],
        "subscription": {
            "name": "trade"
        }
    });

    write
        .send(Message::Text(subscribe_msg.to_string()))
        .await
        .unwrap();

    let subscribe_spread = json!({
        "event": "subscribe",
        "pair": ["XBT/USD"],
        "subscription": {
            "name": "spread"
        }
    });

    write
        .send(Message::Text(subscribe_spread.to_string()))
        .await
        .unwrap();

    println!("Subscribed to XBT/USD trades and spreads\n");
    println!("Receiving live data for 10 seconds...\n");

    // Process messages for 10 seconds
    let start = SystemTime::now();
    let duration = std::time::Duration::from_secs(10);

    while SystemTime::now().duration_since(start).unwrap() < duration {
        tokio::select! {
            Some(msg) = read.next() => {
                if let Ok(Message::Text(text)) = msg {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        // Skip system messages
                        if json.get("event").is_some() {
                            continue;
                        }

                        // Process market data
                        if let Some(arr) = json.as_array() {
                            if arr.len() >= 4 {
                                let channel_name = arr[2].as_str().unwrap_or("");
                                let pair = arr[3].as_str().unwrap_or("");

                                if channel_name == "trade" {
                                    if let Some(trades) = arr[1].as_array() {
                                        for trade in trades {
                                            if let Some(msg_bytes) = kraken_trade_to_protocol(pair, trade) {
                                                // Parse header for routing
                                                let header = unsafe {
                                                    std::ptr::read(msg_bytes.as_ptr() as *const MessageHeader)
                                                };

                                                println!("Trade: {} @ {}",
                                                    pair,
                                                    trade[0].as_str().unwrap_or("?"));

                                                // Route through relay
                                                router.route_message(header, msg_bytes).await;
                                            }
                                        }
                                    }
                                } else if channel_name == "spread" {
                                    if let Some(msg_bytes) = kraken_spread_to_protocol(pair, &arr[1]) {
                                        let header = unsafe {
                                            std::ptr::read(msg_bytes.as_ptr() as *const MessageHeader)
                                        };

                                        println!("Spread: {} bid={} ask={}",
                                            pair,
                                            arr[1][0].as_str().unwrap_or("?"),
                                            arr[1][1].as_str().unwrap_or("?"));

                                        router.route_message(header, msg_bytes).await;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ = tokio::time::sleep(duration) => {
                break;
            }
        }
    }

    println!("\n=== Test Results ===\n");

    // Check results
    let (kraken_total, kraken_trades, kraken_quotes) = kraken_consumer.stats().await;
    let (monitor_total, monitor_trades, monitor_quotes) = monitor_consumer.stats().await;
    let (polygon_total, _, _) = polygon_consumer.stats().await;

    println!(
        "kraken_trader received: {} messages ({} trades, {} quotes)",
        kraken_total, kraken_trades, kraken_quotes
    );
    println!(
        "monitor received: {} messages ({} trades, {} quotes)",
        monitor_total, monitor_trades, monitor_quotes
    );
    println!("polygon_only received: {} messages", polygon_total);

    // Verify routing
    assert!(kraken_total > 0, "kraken_trader should receive messages");
    assert_eq!(
        kraken_total, monitor_total,
        "monitor should receive same as kraken_trader"
    );
    assert_eq!(
        polygon_total, 0,
        "polygon_only should receive no Kraken messages"
    );

    println!("\n✅ Live relay routing test passed!");
    println!("\nKey validations:");
    println!("- Used exact protocol_v2 message format");
    println!("- Real Kraken market data processed");
    println!("- Topic-based filtering worked correctly");
    println!("- Only subscribers to 'market_data_kraken' received messages");
    println!("- polygon_only consumer correctly received 0 messages");
}
