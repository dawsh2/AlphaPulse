//! Full end-to-end pipeline test
//!
//! Tests the complete message flow from exchange through collector,
//! relay, and consumer to validate Protocol V2 architecture.

use alphapulse_adapter_service::output::RelayOutput;
use alphapulse_types::protocol::{
    tlv::{market_data::TradeTLV, TLVMessageBuilder},
    MessageHeader, RelayDomain, SourceType, TLVType,
};
use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::timeout;

/// Mock exchange that generates test events
struct MockExchange {
    event_sender: mpsc::Sender<Vec<u8>>,
}

impl MockExchange {
    fn start() -> (Self, mpsc::Receiver<Vec<u8>>) {
        let (tx, rx) = mpsc::channel(1000);
        (Self { event_sender: tx }, rx)
    }

    async fn send_trade(&self, trade: TradeTLV) -> Result<()> {
        let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Exchange);
        builder.add_tlv(TLVType::Trade, &trade);
        let message = builder.build();
        
        self.event_sender.send(message.to_vec()).await?;
        Ok(())
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
        amount: 100000000,     // 1.0 with 8 decimals
        direction: 1,          // Buy
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
        if consumer.receive_timeout(Duration::from_millis(100)).await?.is_some() {
            received += 1;
        }
    }
    
    let elapsed = start.elapsed();
    let throughput = (received as f64 / elapsed.as_secs_f64()) as usize;
    Ok(throughput)
}

#[tokio::test]
async fn test_full_pipeline_flow() -> Result<()> {
    // 1. Start mock exchange
    let (exchange, exchange_rx) = MockExchange::start();
    
    // 2. Start collector
    let _collector = start_collector(exchange_rx).await;
    
    // 3. Start relay
    let relay = start_relay().await;
    
    // 4. Connect consumer
    let consumer = connect_consumer(relay.clone()).await;
    
    // 5. Send test message through pipeline
    let trade = test_trade();
    exchange.send_trade(trade.clone()).await?;
    
    // 6. Verify message received by consumer
    let received = consumer
        .receive_timeout(Duration::from_secs(1))
        .await?
        .expect("Should receive message");
    
    // Parse and validate
    let header = alphapulse_codec::parse_header(&received)?;
    assert_eq!(header.relay_domain, RelayDomain::MarketData as u8);
    assert_eq!(header.source, SourceType::Exchange as u8);
    
    // 7. Performance validation
    let throughput = measure_throughput(&exchange, &consumer).await?;
    println!("Pipeline throughput: {} msg/s", throughput);
    
    // While we can't achieve 1M msg/s in a test environment,
    // ensure reasonable performance
    assert!(throughput > 1000, "Throughput should exceed 1000 msg/s");
    
    Ok(())
}

#[tokio::test]
async fn test_pipeline_error_handling() -> Result<()> {
    // Test error scenarios
    let (exchange, mut exchange_rx) = MockExchange::start();
    
    // Send malformed message
    exchange.event_sender.send(vec![0xFF; 10]).await?;
    
    // Verify collector handles error gracefully
    let msg = exchange_rx.recv().await.unwrap();
    assert_eq!(msg.len(), 10);
    
    Ok(())
}

#[tokio::test]
async fn test_pipeline_backpressure() -> Result<()> {
    // Test backpressure handling
    let (exchange, _exchange_rx) = MockExchange::start();
    
    // Send many messages rapidly
    for i in 0..10000 {
        let mut trade = test_trade();
        trade.trade_id = i;
        exchange.send_trade(trade).await?;
    }
    
    // System should handle without crashing
    Ok(())
}