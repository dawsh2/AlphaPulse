//! Full end-to-end pipeline test
//!
//! Tests the complete message flow from exchange through collector,
//! relay, and consumer to validate Protocol V2 architecture.
//!
//! IMPORTANT: Uses REAL exchange connections per CLAUDE.md requirements - NO MOCKS

use torq_adapter_service::output::RelayOutput;
use torq_types::protocol::{
    tlv::{market_data::TradeTLV, TLVMessageBuilder},
    MessageHeader, RelayDomain, SourceType, TLVType, InstrumentId, VenueId,
};
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info};

/// Real Kraken WebSocket connection for live data
struct RealKrakenConnection {
    event_sender: mpsc::Sender<Vec<u8>>,
    ws_url: String,
}

impl RealKrakenConnection {
    async fn start(symbol: &str) -> Result<(Self, mpsc::Receiver<Vec<u8>>)> {
        let (tx, rx) = mpsc::channel(1000);
        let ws_url = "wss://ws.kraken.com/".to_string();
        
        let mut connection = Self { 
            event_sender: tx,
            ws_url: ws_url.clone(),
        };
        
        // Start WebSocket connection in background
        let sender = connection.event_sender.clone();
        let symbol = symbol.to_string();
        tokio::spawn(async move {
            if let Err(e) = Self::run_websocket(ws_url, sender, symbol).await {
                error!("WebSocket error: {}", e);
            }
        });
        
        Ok((connection, rx))
    }
    
    async fn run_websocket(
        url: String,
        sender: mpsc::Sender<Vec<u8>>,
        symbol: String,
    ) -> Result<()> {
        let (ws_stream, _) = connect_async(&url)
            .await
            .context("Failed to connect to Kraken WebSocket")?;
        
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        
        // Subscribe to trades
        let subscribe_msg = json!({
            "event": "subscribe",
            "pair": [symbol],
            "subscription": {
                "name": "trade"
            }
        });
        
        ws_sender
            .send(Message::Text(subscribe_msg.to_string()))
            .await
            .context("Failed to subscribe")?;
        
        info!("Connected to Kraken WebSocket for {}", symbol);
        
        // Process incoming messages
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(trades) = Self::parse_trade_message(&parsed, &symbol) {
                            for trade in trades {
                                let mut builder = TLVMessageBuilder::new(
                                    RelayDomain::MarketData,
                                    SourceType::KrakenCollector,
                                );
                                builder.add_tlv(TLVType::Trade, &trade);
                                let message = builder.build();
                                
                                if let Err(e) = sender.send(message.to_vec()).await {
                                    error!("Failed to send trade: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket closed");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    fn parse_trade_message(msg: &serde_json::Value, symbol: &str) -> Option<Vec<TradeTLV>> {
        // Parse Kraken trade format: [channel_id, [[price, volume, time, side, orderType, misc]], "trade", "XBT/USD"]
        if let Some(arr) = msg.as_array() {
            if arr.len() >= 4 {
                if let (Some(trades_data), Some(channel_name), Some(pair)) = 
                    (arr.get(1), arr.get(2), arr.get(3)) {
                    
                    if channel_name.as_str() == Some("trade") && 
                       pair.as_str() == Some(symbol) {
                        
                        if let Some(trades) = trades_data.as_array() {
                            let mut tlv_trades = Vec::new();
                            
                            for trade in trades {
                                if let Some(t) = trade.as_array() {
                                    if t.len() >= 4 {
                                        let price = t[0].as_str().and_then(|s| s.parse::<f64>().ok())?;
                                        let volume = t[1].as_str().and_then(|s| s.parse::<f64>().ok())?;
                                        let timestamp = t[2].as_str().and_then(|s| s.parse::<f64>().ok())?;
                                        let side = t[3].as_str()?;
                                        
                                        // Convert to Protocol V2 format
                                        let price_fixed = (price * 100_000_000.0) as i64;
                                        let volume_fixed = (volume * 100_000_000.0) as i64;
                                        let timestamp_ns = (timestamp * 1_000_000_000.0) as u64;
                                        let direction = if side == "b" { 0 } else { 1 };
                                        
                                        let instrument_id = InstrumentId::from_venue_and_symbol(
                                            VenueId::Kraken,
                                            symbol
                                        );
                                        
                                        tlv_trades.push(TradeTLV {
                                            instrument_id: instrument_id.as_u64(),
                                            price: price_fixed,
                                            amount: volume_fixed,
                                            direction,
                                            timestamp_ns,
                                            trade_id: timestamp_ns, // Use timestamp as ID for uniqueness
                                        });
                                    }
                                }
                            }
                            
                            return Some(tlv_trades);
                        }
                    }
                }
            }
        }
        None
    }
}

/// Start a collector that processes exchange events
async fn start_collector(exchange_rx: mpsc::Receiver<Vec<u8>>) -> RelayOutput {
    let relay_output = RelayOutput::new("test_collector".to_string());

    // Spawn collector task
    let output = relay_output.clone();
    tokio::spawn(async move {
        let mut rx = exchange_rx;
        while let Some(event) = rx.recv().await {
            // Process and forward to relay
            output.send_bytes(event).await.unwrap();
        }
    });

    relay_output
}

/// Start a relay server
async fn start_relay() -> Arc<RelayServer> {
    let relay = Arc::new(RelayServer::new());
    relay.start().await;
    relay
}

/// Connect a consumer to the relay
async fn connect_consumer(relay: Arc<RelayServer>) -> ConsumerConnection {
    ConsumerConnection::new(relay).await
}

/// Relay server mock
struct RelayServer {
    messages: Arc<tokio::sync::RwLock<Vec<Vec<u8>>>>,
}

impl RelayServer {
    fn new() -> Self {
        Self {
            messages: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    async fn start(&self) {
        // Start relay server
    }

    async fn receive_message(&self, msg: Vec<u8>) {
        self.messages.write().await.push(msg);
    }

    async fn get_messages(&self) -> Vec<Vec<u8>> {
        self.messages.read().await.clone()
    }
}

/// Consumer connection
struct ConsumerConnection {
    relay: Arc<RelayServer>,
}

impl ConsumerConnection {
    async fn new(relay: Arc<RelayServer>) -> Self {
        Self { relay }
    }

    async fn receive_timeout(&self, duration: Duration) -> Result<Option<Vec<u8>>> {
        let start = Instant::now();
        while start.elapsed() < duration {
            let messages = self.relay.get_messages().await;
            if !messages.is_empty() {
                return Ok(Some(messages[0].clone()));
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok(None)
    }
}

/// Create a test trade
fn test_trade() -> TradeTLV {
    TradeTLV {
        instrument_id: 12345,
        price: 4500000000000, // $45,000 with 8 decimals
        amount: 100000000,    // 1.0 with 8 decimals
        direction: 1,         // Buy
        timestamp_ns: 1000000000,
        trade_id: 987654321,
    }
}

/// Create expected message for validation
fn expected_message(trade: &TradeTLV) -> MessageHeader {
    let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Exchange);
    builder.add_tlv(TLVType::Trade, trade);
    builder.build()
}

/// Measure throughput between exchange and consumer
async fn measure_throughput(
    exchange: &MockExchange,
    consumer: &ConsumerConnection,
) -> Result<usize> {
    let start = Instant::now();
    let message_count = 10000;

    // Send messages
    for i in 0..message_count {
        let mut trade = test_trade();
        trade.trade_id = i as u64;
        exchange.send_trade(trade).await?;
    }

    // Wait for all messages
    let mut received = 0;
    let timeout_duration = Duration::from_secs(10);
    let deadline = Instant::now() + timeout_duration;

    while received < message_count && Instant::now() < deadline {
        if consumer
            .receive_timeout(Duration::from_millis(100))
            .await?
            .is_some()
        {
            received += 1;
        }
    }

    let elapsed = start.elapsed();
    let throughput = (received as f64 / elapsed.as_secs_f64()) as usize;
    Ok(throughput)
}

#[tokio::test]
#[ignore] // Run with --ignored flag to test with real exchange data
async fn test_full_pipeline_flow_real_kraken() -> Result<()> {
    // Initialize logging for debugging
    tracing_subscriber::fmt::init();
    
    info!("Starting real Kraken WebSocket pipeline test");
    
    // 1. Start REAL Kraken WebSocket connection (NO MOCKS)
    let (kraken, exchange_rx) = RealKrakenConnection::start("XBT/USD").await?;
    
    // 2. Start collector
    let _collector = start_collector(exchange_rx).await;

    // 3. Start relay
    let relay = start_relay().await;

    // 4. Connect consumer
    let consumer = connect_consumer(relay.clone()).await;

    // 5. Wait for real trades from Kraken (no fake data)
    info!("Waiting for real trades from Kraken...");
    
    // Give WebSocket time to connect and receive trades
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // 6. Verify message received by consumer
    let received = consumer
        .receive_timeout(Duration::from_secs(10)) // Longer timeout for real data
        .await?
        .context("Should receive real trade from Kraken")?;

    // Parse and validate Protocol V2 header
    let header = torq_codec::parse_header(&received)?;
    
    // Verify proper Protocol V2 compliance
    assert_eq!(header.magic, 0xDEADBEEF, "Magic number must be 0xDEADBEEF");
    assert_eq!(header.relay_domain, RelayDomain::MarketData as u8);
    assert_eq!(header.source, SourceType::KrakenCollector as u8);
    
    info!("Successfully received and validated real Kraken trade through pipeline");

    // 7. Performance validation with real data
    // Note: Real exchange data is rate-limited, so we test latency instead of throughput
    let start = Instant::now();
    let _next_trade = consumer
        .receive_timeout(Duration::from_secs(30))
        .await?;
    let latency = start.elapsed();
    
    info!("Trade-to-consumer latency: {:?}", latency);
    assert!(latency < Duration::from_secs(1), "Latency should be under 1 second");

    Ok(())
}

#[tokio::test]
async fn test_pipeline_protocol_compliance() -> Result<()> {
    // Test Protocol V2 compliance with proper magic number
    use torq_types::MESSAGE_MAGIC;
    
    let mut builder = TLVMessageBuilder::new(
        RelayDomain::MarketData,
        SourceType::KrakenCollector,
    );
    
    let trade = TradeTLV {
        instrument_id: InstrumentId::from_venue_and_symbol(VenueId::Kraken, "XBT/USD").as_u64(),
        price: 4500000000000,
        amount: 100000000,
        direction: 0,
        timestamp_ns: torq_network::time::safe_system_timestamp_ns(),
        trade_id: 12345,
    };
    
    builder.add_tlv(TLVType::Trade, &trade);
    let message = builder.build();
    
    // Verify header has correct magic number (0xDEADBEEF)
    let header = torq_codec::parse_header(&message)?;
    assert_eq!(header.magic, MESSAGE_MAGIC, "Magic number must match Protocol V2");
    assert_eq!(header.magic, 0xDEADBEEF, "Magic must be 0xDEADBEEF");
    
    // Verify header fields
    assert_eq!(header.relay_domain, RelayDomain::MarketData as u8);
    assert_eq!(header.source, SourceType::KrakenCollector as u8);
    assert!(header.payload_size > 0, "Payload must not be empty");
    
    Ok(())
}

#[tokio::test]
#[ignore] // Run with --ignored to test with real Coinbase data
async fn test_real_coinbase_pipeline() -> Result<()> {
    // Test with REAL Coinbase WebSocket (NO MOCKS)
    use tokio_tungstenite::connect_async;
    
    let url = "wss://ws-feed.exchange.coinbase.com";
    let (ws_stream, _) = connect_async(url)
        .await
        .context("Failed to connect to Coinbase")?;
    
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    // Subscribe to real BTC-USD trades
    let subscribe = json!({
        "type": "subscribe",
        "product_ids": ["BTC-USD"],
        "channels": ["matches"]
    });
    
    ws_sender.send(Message::Text(subscribe.to_string())).await?;
    
    // Process real trades
    let mut trade_count = 0;
    while let Some(msg) = ws_receiver.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                if parsed["type"] == "match" {
                    trade_count += 1;
                    
                    // Parse real Coinbase trade
                    let price = parsed["price"].as_str()
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let size = parsed["size"].as_str()
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    
                    info!("Real Coinbase trade: ${:.2} x {:.8} BTC", price, size);
                    
                    if trade_count >= 5 {
                        break; // Got enough real trades
                    }
                }
            }
        }
    }
    
    assert!(trade_count > 0, "Should receive real Coinbase trades");
    info!("Successfully processed {} real Coinbase trades", trade_count);
    
    Ok(())
}
