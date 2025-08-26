//! MarketData Relay Integration Tests
//!
//! Tests for MarketData domain relay (TLV types 1-19):
//! - High-frequency message routing
//! - Consumer subscription management
//! - Performance under load
//! - Message filtering and routing

use std::time::Duration;
use tokio::{sync::mpsc, time::timeout};
use protocol_v2::{
    tlv::{TLVMessageBuilder, TradeTLV, QuoteTLV},
    RelayDomain, SourceType,
};

/// Integration test framework for relay testing
struct RelayTestFramework {
    market_data_relay: Option<tokio::process::Child>,
    test_socket_path: String,
}

impl RelayTestFramework {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let test_socket_path = format!("/tmp/test_market_data_{}.sock", uuid::Uuid::new_v4());
        
        // Start market data relay process for testing
        let relay_process = tokio::process::Command::new("cargo")
            .args(&["run", "--release", "--package", "alphapulse-market-data-relay", "--bin", "market_data_relay"])
            .env("MARKET_DATA_SOCKET", &test_socket_path)
            .env("RUST_LOG", "debug")
            .spawn()
            .expect("Failed to start market data relay");
        
        // Give relay time to start
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        Ok(Self {
            market_data_relay: Some(relay_process),
            test_socket_path,
        })
    }
    
    async fn connect_producer(&self) -> Result<TestProducer, Box<dyn std::error::Error>> {
        TestProducer::connect(&self.test_socket_path).await
    }
    
    async fn connect_consumer(&self) -> Result<TestConsumer, Box<dyn std::error::Error>> {
        TestConsumer::connect(&self.test_socket_path).await
    }
}

impl Drop for RelayTestFramework {
    fn drop(&mut self) {
        if let Some(mut process) = self.market_data_relay.take() {
            let _ = process.start_kill();
        }
        // Clean up socket file
        let _ = std::fs::remove_file(&self.test_socket_path);
    }
}

/// Test message producer
struct TestProducer {
    socket: tokio::net::UnixStream,
}

impl TestProducer {
    async fn connect(socket_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let socket = tokio::net::UnixStream::connect(socket_path).await?;
        Ok(Self { socket })
    }
    
    async fn send_trade_message(&mut self, trade: &TradeTLV) -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector);
        builder.add_trade_tlv(trade);
        let message = builder.build();
        
        tokio::io::AsyncWriteExt::write_all(&mut self.socket, &message).await?;
        Ok(())
    }
    
    async fn send_quote_message(&mut self, quote: &QuoteTLV) -> Result<(), Box<dyn std::error::Error>> {
        let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::KrakenCollector);
        builder.add_quote_tlv(quote);
        let message = builder.build();
        
        tokio::io::AsyncWriteExt::write_all(&mut self.socket, &message).await?;
        Ok(())
    }
}

/// Test message consumer
struct TestConsumer {
    receiver: mpsc::Receiver<Vec<u8>>,
    _handle: tokio::task::JoinHandle<()>,
}

impl TestConsumer {
    async fn connect(socket_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let socket = tokio::net::UnixStream::connect(socket_path).await?;
        let (tx, rx) = mpsc::channel(100);
        
        let handle = tokio::spawn(async move {
            let mut reader = socket;
            let mut buffer = [0u8; 4096];
            
            loop {
                match tokio::io::AsyncReadExt::read(&mut reader, &mut buffer).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let message = buffer[..n].to_vec();
                        if tx.send(message).await.is_err() {
                            break; // Receiver dropped
                        }
                    }
                    Err(_) => break, // Error reading
                }
            }
        });
        
        Ok(Self {
            receiver: rx,
            _handle: handle,
        })
    }
    
    async fn receive_message(&mut self) -> Option<Vec<u8>> {
        timeout(Duration::from_secs(5), self.receiver.recv())
            .await
            .ok()
            .flatten()
    }
}

#[tokio::test]
async fn test_market_data_relay_basic_routing() {
    let framework = RelayTestFramework::new().await
        .expect("Failed to create test framework");
    
    let mut producer = framework.connect_producer().await
        .expect("Failed to connect producer");
    let mut consumer = framework.connect_consumer().await
        .expect("Failed to connect consumer");
    
    // Create test trade message
    let trade = TradeTLV {
        instrument_id: 12345,
        price: 50000_00000000, // $50,000 in 8-decimal fixed-point
        quantity: 1_000000000000000000, // 1.0 token in native precision
        timestamp_ns: 1234567890123456789,
        side: 1, // Buy
        trade_id: 987654321,
    };
    
    // Send trade message
    producer.send_trade_message(&trade).await
        .expect("Failed to send trade message");
    
    // Receive and validate
    let received = consumer.receive_message().await
        .expect("Should receive trade message");
    
    // Parse received message
    let header = protocol_v2::parse_header(&received)
        .expect("Should parse header");
    
    assert_eq!(header.relay_domain, RelayDomain::MarketData);
    assert!(received.len() > 32); // Should have payload
}

#[tokio::test]
async fn test_market_data_relay_high_frequency() {
    let framework = RelayTestFramework::new().await
        .expect("Failed to create test framework");
    
    let mut producer = framework.connect_producer().await
        .expect("Failed to connect producer");
    let mut consumer = framework.connect_consumer().await
        .expect("Failed to connect consumer");
    
    let message_count = 1000;
    let start_time = std::time::Instant::now();
    
    // Send high-frequency messages
    for i in 0..message_count {
        let trade = TradeTLV {
            instrument_id: 12345,
            price: 50000_00000000 + i as i64, // Varying price
            quantity: 1_000000000000000000,
            timestamp_ns: 1234567890123456789 + i as u64,
            side: (i % 2) as u8, // Alternating buy/sell
            trade_id: i as u64,
        };
        
        producer.send_trade_message(&trade).await
            .expect("Failed to send message");
    }
    
    // Receive all messages
    for i in 0..message_count {
        let received = timeout(Duration::from_secs(10), consumer.receive_message())
            .await
            .expect("Timeout waiting for message")
            .expect("Should receive message");
        
        let header = protocol_v2::parse_header(&received)
            .expect("Should parse header");
        assert_eq!(header.relay_domain, RelayDomain::MarketData);
        
        if i % 100 == 0 {
            println!("Received message {}/{}", i + 1, message_count);
        }
    }
    
    let duration = start_time.elapsed();
    let throughput = message_count as f64 / duration.as_secs_f64();
    
    println!("Market data relay throughput: {:.0} msg/s", throughput);
    
    // Should handle at least 10,000 messages per second
    assert!(throughput > 10_000.0, 
            "Throughput too low: {:.0} msg/s", throughput);
}

#[tokio::test]
async fn test_market_data_relay_multiple_consumers() {
    let framework = RelayTestFramework::new().await
        .expect("Failed to create test framework");
    
    let mut producer = framework.connect_producer().await
        .expect("Failed to connect producer");
    
    // Connect multiple consumers
    let mut consumer1 = framework.connect_consumer().await
        .expect("Failed to connect consumer 1");
    let mut consumer2 = framework.connect_consumer().await
        .expect("Failed to connect consumer 2");
    let mut consumer3 = framework.connect_consumer().await
        .expect("Failed to connect consumer 3");
    
    // Send test message
    let trade = TradeTLV {
        instrument_id: 99999,
        price: 1000_00000000, // $1,000
        quantity: 5_000000000000000000, // 5.0 tokens
        timestamp_ns: 1234567890123456789,
        side: 0, // Sell
        trade_id: 555555,
    };
    
    producer.send_trade_message(&trade).await
        .expect("Failed to send trade message");
    
    // All consumers should receive the same message
    let msg1 = consumer1.receive_message().await.expect("Consumer 1 should receive");
    let msg2 = consumer2.receive_message().await.expect("Consumer 2 should receive");
    let msg3 = consumer3.receive_message().await.expect("Consumer 3 should receive");
    
    // Messages should be identical
    assert_eq!(msg1, msg2, "Consumer 1 and 2 should receive identical messages");
    assert_eq!(msg2, msg3, "Consumer 2 and 3 should receive identical messages");
    
    // Parse and validate content
    let header = protocol_v2::parse_header(&msg1).expect("Should parse header");
    assert_eq!(header.relay_domain, RelayDomain::MarketData);
}

#[tokio::test]
async fn test_market_data_relay_message_ordering() {
    let framework = RelayTestFramework::new().await
        .expect("Failed to create test framework");
    
    let mut producer = framework.connect_producer().await
        .expect("Failed to connect producer");
    let mut consumer = framework.connect_consumer().await
        .expect("Failed to connect consumer");
    
    let message_count = 100;
    
    // Send messages with increasing sequence numbers
    for i in 0..message_count {
        let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector);
        builder.set_sequence(i as u64);
        
        let trade = TradeTLV {
            instrument_id: 12345,
            price: 50000_00000000,
            quantity: 1_000000000000000000,
            timestamp_ns: 1234567890123456789 + i as u64,
            side: 1,
            trade_id: i as u64,
        };
        
        builder.add_trade_tlv(&trade);
        let message = builder.build();
        
        tokio::io::AsyncWriteExt::write_all(&mut producer.socket, &message).await
            .expect("Failed to send message");
    }
    
    // Receive and validate ordering
    for expected_seq in 0..message_count {
        let received = consumer.receive_message().await
            .expect("Should receive message");
        
        let header = protocol_v2::parse_header(&received)
            .expect("Should parse header");
        
        assert_eq!(header.sequence, expected_seq as u64,
                  "Message sequence out of order. Expected {}, got {}", 
                  expected_seq, header.sequence);
    }
}

// Mock TLV types for testing (these would be defined in protocol_v2)
#[derive(Clone, Debug)]
struct TradeTLV {
    instrument_id: u64,
    price: i64,
    quantity: i64,
    timestamp_ns: u64,
    side: u8,
    trade_id: u64,
}

#[derive(Clone, Debug)]
struct QuoteTLV {
    instrument_id: u64,
    bid_price: i64,
    ask_price: i64,
    bid_quantity: i64,
    ask_quantity: i64,
    timestamp_ns: u64,
}

// Mock extensions for TLVMessageBuilder
trait TLVMessageBuilderExt {
    fn add_trade_tlv(&mut self, trade: &TradeTLV);
    fn add_quote_tlv(&mut self, quote: &QuoteTLV);
}

impl TLVMessageBuilderExt for TLVMessageBuilder {
    fn add_trade_tlv(&mut self, trade: &TradeTLV) {
        // Serialize trade to bytes (mock implementation)
        let trade_bytes = bincode::serialize(trade).expect("Failed to serialize trade");
        self.add_tlv(1, &trade_bytes); // TradeTLV type = 1
    }
    
    fn add_quote_tlv(&mut self, quote: &QuoteTLV) {
        // Serialize quote to bytes (mock implementation)
        let quote_bytes = bincode::serialize(quote).expect("Failed to serialize quote");
        self.add_tlv(2, &quote_bytes); // QuoteTLV type = 2
    }
}