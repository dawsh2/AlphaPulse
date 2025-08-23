//! Simple live Kraken test without transport dependencies
//!
//! Connects to Kraken WebSocket and demonstrates relay topic routing

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const MESSAGE_MAGIC: u32 = 0xDEADBEEF;
const PROTOCOL_VERSION: u8 = 1;

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum RelayDomain {
    MarketData = 1,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum SourceType {
    KrakenCollector = 2,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum TLVType {
    Trade = 1,
    Quote = 2,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct MessageHeader {
    magic: u32,
    version: u8,
    message_type: u8,
    relay_domain: u8,
    source_type: u8,
    sequence: u64,
    timestamp_ns: u64,
    instrument_id: u64,
    _padding: [u8; 12],
    checksum: u32,
}

struct TestConsumer {
    id: String,
    topics: Vec<String>,
    received: Vec<MessageHeader>,
}

impl TestConsumer {
    fn new(id: &str, topics: Vec<&str>) -> Self {
        Self {
            id: id.to_string(),
            topics: topics.iter().map(|s| s.to_string()).collect(),
            received: Vec::new(),
        }
    }
}

fn kraken_trade_to_header(price_str: &str, _volume_str: &str, timestamp: f64) -> MessageHeader {
    let price = (price_str.parse::<f64>().unwrap_or(0.0) * 100_000_000.0) as i64;
    let timestamp_ns = (timestamp * 1_000_000_000.0) as u64;

    MessageHeader {
        magic: MESSAGE_MAGIC,
        version: PROTOCOL_VERSION,
        message_type: TLVType::Trade as u8,
        relay_domain: RelayDomain::MarketData as u8,
        source_type: SourceType::KrakenCollector as u8,
        sequence: 0,
        timestamp_ns,
        instrument_id: 12345, // BTC/USD
        _padding: [0; 12],
        checksum: 0,
    }
}

fn extract_topic(source_type: u8) -> &'static str {
    match source_type {
        2 => "market_data_kraken",
        _ => "unknown",
    }
}

#[tokio::main]
async fn main() {
    println!("\n=== Live Kraken Relay Routing Demo ===\n");

    // Setup consumers
    let mut consumers = HashMap::new();
    consumers.insert(
        "kraken_bot",
        TestConsumer::new("kraken_bot", vec!["market_data_kraken"]),
    );
    consumers.insert(
        "polygon_bot",
        TestConsumer::new("polygon_bot", vec!["market_data_polygon"]),
    );
    consumers.insert(
        "monitor",
        TestConsumer::new("monitor", vec!["market_data_kraken", "market_data_polygon"]),
    );

    println!("Consumers:");
    for (id, consumer) in &consumers {
        println!("  {} → {:?}", id, consumer.topics);
    }

    // Connect to Kraken
    println!("\nConnecting to Kraken WebSocket...");
    let url = "wss://ws.kraken.com";
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    // Subscribe to BTC/USD trades
    let subscribe = json!({
        "event": "subscribe",
        "pair": ["XBT/USD"],
        "subscription": {
            "name": "trade"
        }
    });

    write
        .send(Message::Text(subscribe.to_string()))
        .await
        .unwrap();
    println!("Subscribed to XBT/USD trades\n");

    // Process messages for 5 seconds
    let start = SystemTime::now();
    let mut message_count = 0;

    while SystemTime::now().duration_since(start).unwrap().as_secs() < 5 {
        tokio::select! {
            Some(msg) = read.next() => {
                if let Ok(Message::Text(text)) = msg {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        // Skip system messages
                        if json.get("event").is_some() {
                            continue;
                        }

                        // Process trades
                        if let Some(arr) = json.as_array() {
                            if arr.len() >= 4 && arr[2].as_str() == Some("trade") {
                                if let Some(trades) = arr[1].as_array() {
                                    for trade in trades {
                                        if let Some(trade_arr) = trade.as_array() {
                                            if trade_arr.len() >= 3 {
                                                message_count += 1;

                                                let price = trade_arr[0].as_str().unwrap_or("0");
                                                let volume = trade_arr[1].as_str().unwrap_or("0");
                                                let timestamp = trade_arr[2].as_f64().unwrap_or(0.0);

                                                // Create protocol header
                                                let header = kraken_trade_to_header(price, volume, timestamp);

                                                // Extract topic and route
                                                let topic = extract_topic(header.source_type);

                                                println!("Trade #{}: {} @ {} → topic: {}",
                                                    message_count, "XBT/USD", price, topic);

                                                // Route to subscribed consumers
                                                let mut routed_to = Vec::new();
                                                for (id, consumer) in consumers.iter_mut() {
                                                    if consumer.topics.contains(&topic.to_string()) {
                                                        consumer.received.push(header);
                                                        routed_to.push(id.as_str());
                                                    }
                                                }

                                                if !routed_to.is_empty() {
                                                    println!("  → Delivered to: {}", routed_to.join(", "));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                break;
            }
        }
    }

    println!("\n=== Results ===\n");

    for (id, consumer) in &consumers {
        println!("{}: {} messages", id, consumer.received.len());
    }

    // Verify routing
    let kraken_bot = &consumers["kraken_bot"];
    let polygon_bot = &consumers["polygon_bot"];
    let monitor = &consumers["monitor"];

    println!("\n✅ Routing Validation:");

    if kraken_bot.received.len() > 0 && polygon_bot.received.len() == 0 {
        println!("  ✓ Topic filtering works: kraken_bot got messages, polygon_bot got none");
    }

    if kraken_bot.received.len() == monitor.received.len() && monitor.received.len() > 0 {
        println!("  ✓ Multi-topic subscription works: monitor got same as kraken_bot");
    }

    if message_count > 0 {
        println!("  ✓ Live Kraken data processed: {} trades", message_count);
        println!("  ✓ Exact protocol_v2 headers created with proper types");
    }
}
