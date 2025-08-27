//! Integration Tests for Consolidated Network Crate
//!
//! Tests the complete integration of the unified network crate including:
//! - Protocol V2 validation and message flow
//! - Mycelium actor system with transport selection
//! - Hybrid transport configuration and routing
//! - Unix socket and network transports
//! - Performance characteristics and zero-copy benefits

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::timeout;

use torq_network::{
    Priority, Criticality, Reliability,
    mycelium::{
        ActorSystem, ActorBehavior, ActorRef,
        messages::{
            Message, MarketMessage, PoolSwapEvent, QuoteUpdate, SignalMessage, ExecutionMessage,
            ArbitrageSignal, RiskAlert, RiskAlertType, AlertSeverity
        },
        bundle::{ActorBundle, BundleConfiguration, DeploymentMode},
    },
    hybrid::{
        HybridTransport,
        config::{TransportConfig, ChannelConfig, TransportMode, RetryConfig},
    },
    protocol_v2::{Protocol, MessageHeader, DomainType, ValidationMode},
};
use async_trait::async_trait;

/// Integration test actor that processes all message types
#[derive(Debug)]
struct IntegrationTestActor {
    id: String,
    market_messages: Vec<MarketMessage>,
    signal_messages: Vec<SignalMessage>,
    execution_messages: Vec<ExecutionMessage>,
    total_processed: u64,
}

#[async_trait]
impl ActorBehavior for IntegrationTestActor {
    type Message = TestMessage;

    async fn handle(&mut self, msg: TestMessage) -> torq_network::Result<()> {
        self.total_processed += 1;
        
        match msg {
            TestMessage::Market(market_msg) => {
                self.market_messages.push(market_msg);
            }
            TestMessage::Signal(signal_msg) => {
                self.signal_messages.push(signal_msg);
            }
            TestMessage::Execution(exec_msg) => {
                self.execution_messages.push(exec_msg);
            }
        }
        
        Ok(())
    }

    async fn on_start(&mut self) -> torq_network::Result<()> {
        tracing::info!("Integration test actor {} started", self.id);
        Ok(())
    }
}

/// Test message wrapper for different Protocol V2 domains
#[derive(Debug, Clone)]
enum TestMessage {
    Market(MarketMessage),
    Signal(SignalMessage),
    Execution(ExecutionMessage),
}

impl Message for TestMessage {
    fn to_tlv(&self) -> torq_network::Result<Vec<u8>> {
        match self {
            TestMessage::Market(msg) => msg.to_tlv(),
            TestMessage::Signal(msg) => msg.to_tlv(),
            TestMessage::Execution(msg) => msg.to_tlv(),
        }
    }

    fn from_tlv(data: &[u8]) -> torq_network::Result<Self> {
        // Try to parse as different message types based on TLV type
        if let Ok(market_msg) = MarketMessage::from_tlv(data) {
            Ok(TestMessage::Market(market_msg))
        } else if let Ok(signal_msg) = SignalMessage::from_tlv(data) {
            Ok(TestMessage::Signal(signal_msg))
        } else if let Ok(exec_msg) = ExecutionMessage::from_tlv(data) {
            Ok(TestMessage::Execution(exec_msg))
        } else {
            Err(torq_network::TransportError::protocol("Unable to parse test message from TLV"))
        }
    }
}

#[tokio::test]
async fn test_full_protocol_v2_integration() {
    // Test the complete Protocol V2 integration across all domains
    tracing::info!("Starting Protocol V2 integration test");

    let protocol = Protocol::new();
    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    // Test Market Data Domain (TLV types 1-19)
    let pool_event = PoolSwapEvent {
        pool_address: [0x1f; 20], // Real Ethereum pool address
        token0_in: 1_500_000_000_000_000_000, // 1.5 WETH (18 decimals)
        token1_out: 3_000_000_000, // 3000 USDC (6 decimals)
        timestamp_ns,
        tx_hash: [0xde; 32],
        gas_used: 180_000,
    };

    let market_message = MarketMessage::Swap(Arc::new(pool_event.clone()));
    
    // Validate TLV serialization with Protocol V2 checksum
    let tlv_bytes = market_message.to_tlv().expect("Market message TLV serialization failed");
    let is_valid = protocol.validate_message(&tlv_bytes, ValidationMode::Strict)
        .expect("Protocol V2 validation failed");
    
    assert!(is_valid, "Market data TLV message should pass Protocol V2 validation");

    // Test deserialization
    let _deserialized = MarketMessage::from_tlv(&tlv_bytes)
        .expect("Market message TLV deserialization failed");

    // Test Signal Domain (TLV types 20-39) - create a test arbitrage signal
    let arbitrage_signal = ArbitrageSignal {
        signal_id: 12345,
        instrument_a: 98765,
        instrument_b: 54321,
        price_diff: 50_00000000, // $50.00 difference in 8-decimal
        volume: 1_000_000, // 1.0 unit
        confidence: 95, // 95% confidence
        timestamp_ns: timestamp_ns + 1000,
    };
    
    let signal_message = SignalMessage::Arbitrage(Arc::new(arbitrage_signal));
    let signal_tlv = signal_message.to_tlv().expect("Signal message TLV serialization failed");
    let signal_valid = protocol.validate_message(&signal_tlv, ValidationMode::Strict)
        .expect("Signal Protocol V2 validation failed");
    
    assert!(signal_valid, "Signal TLV message should pass Protocol V2 validation");

    tracing::info!("Protocol V2 integration test completed successfully");
}

#[tokio::test]
async fn test_mycelium_actor_system_integration() {
    // Test complete Mycelium actor system with transport selection
    tracing::info!("Starting Mycelium actor system integration test");

    let system = ActorSystem::new();
    
    // Create actors with different requirements
    let market_actor = IntegrationTestActor {
        id: "market_processor".to_string(),
        market_messages: vec![],
        signal_messages: vec![],
        execution_messages: vec![],
        total_processed: 0,
    };

    let signal_actor = IntegrationTestActor {
        id: "signal_processor".to_string(),
        market_messages: vec![],
        signal_messages: vec![],
        execution_messages: vec![],
        total_processed: 0,
    };

    // Spawn actors
    let market_ref: ActorRef<TestMessage> = system.spawn(market_actor).await
        .expect("Failed to spawn market actor");
    let signal_ref: ActorRef<TestMessage> = system.spawn(signal_actor).await
        .expect("Failed to spawn signal actor");

    // Create test messages with real Protocol V2 data
    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let pool_event = PoolSwapEvent {
        pool_address: [0xa1; 20],
        token0_in: 800_000_000_000_000_000, // 0.8 WETH
        token1_out: 1_600_000_000, // 1600 USDC
        timestamp_ns,
        tx_hash: [0xbc; 32],
        gas_used: 150_000,
    };

    let quote_update = QuoteUpdate {
        instrument_id: 67890,
        bid_price: 1875_50000000_i64, // $1875.50
        ask_price: 1876_00000000_i64, // $1876.00
        bid_size: 2_250_000, // 2.25 units
        ask_size: 1_750_000, // 1.75 units
        timestamp_ns: timestamp_ns + 500,
    };

    // Send messages to actors
    let market_message = TestMessage::Market(MarketMessage::Swap(Arc::new(pool_event)));
    
    let arbitrage_signal = ArbitrageSignal {
        signal_id: 67890,
        instrument_a: quote_update.instrument_id,
        instrument_b: quote_update.instrument_id + 1,
        price_diff: 25_00000000, // $25.00 difference
        volume: 750_000, // 0.75 units
        confidence: 88, // 88% confidence
        timestamp_ns: quote_update.timestamp_ns,
    };
    let signal_message = TestMessage::Signal(SignalMessage::Arbitrage(Arc::new(arbitrage_signal)));

    // Test priority message handling
    market_ref.send_with_priority(market_message, Priority::High).await
        .expect("Failed to send high priority market message");
    
    signal_ref.send_with_priority(signal_message, Priority::Critical).await
        .expect("Failed to send critical signal message");

    // Allow message processing
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify actor health and metrics
    assert!(market_ref.is_healthy(), "Market actor should be healthy");
    assert!(signal_ref.is_healthy(), "Signal actor should be healthy");

    let market_metrics = market_ref.metrics();
    let market_stats = market_metrics.get_stats();
    assert!(market_stats.local_sends >= 1, "Market actor should have received messages");
    assert!(market_stats.avg_local_latency_ns > 0.0, "Should measure actual latency");

    tracing::info!("Mycelium actor system integration test completed");
}

#[tokio::test]
async fn test_actor_bundle_zero_copy_integration() {
    // Test ActorBundle with zero-copy communication
    tracing::info!("Starting ActorBundle zero-copy integration test");

    let mut bundle_config = BundleConfiguration::new("test_bundle");
    
    // Configure for shared memory deployment (zero-copy)
    let actor_ids = vec![
        torq_network::mycelium::registry::ActorId::new(),
        torq_network::mycelium::registry::ActorId::new(),
    ];
    
    bundle_config.deployment = DeploymentMode::SharedMemory { 
        channels: std::collections::HashMap::new() 
    };

    let mut bundle = ActorBundle::new(bundle_config);

    // Test zero-copy message sending
    let test_data = vec![1u8; 1024]; // 1KB test message
    let large_message = Arc::new(test_data);

    // This should use Arc::clone() instead of serialization
    bundle.send_local(&actor_ids[0], large_message.clone()).await
        .expect("Zero-copy send should succeed");

    let metrics = bundle.metrics();
    assert!(metrics.zero_copy_sends >= 1, "Should record zero-copy send");
    assert!(metrics.serialization_bytes_eliminated > 0, "Should eliminate serialization");

    tracing::info!("ActorBundle zero-copy integration completed");
}

#[tokio::test]
async fn test_hybrid_transport_configuration() {
    // Test hybrid transport with different configuration modes
    tracing::info!("Starting hybrid transport configuration test");

    // Create transport configuration
    let mut config = TransportConfig::default();
    
    // Ultra-low latency channel for critical signals
    let critical_channel = ChannelConfig::ultra_low_latency("critical_signals");
    config.add_channel(critical_channel);

    // Reliable delivery channel for audit logs
    let audit_channel = ChannelConfig::reliable_delivery("audit_logs");
    config.add_channel(audit_channel);

    // Validate configuration
    config.validate().expect("Transport configuration should be valid");

    // Create hybrid transport
    let hybrid = HybridTransport::new(config.clone()).await
        .expect("Failed to create hybrid transport");

    // Test configuration retrieval
    let critical_config = config.get_channel_config("critical_signals");
    assert_eq!(critical_config.mode, TransportMode::Direct);
    assert_eq!(critical_config.criticality, Criticality::UltraLowLatency);
    assert_eq!(critical_config.retry.max_attempts, 1); // No retries for ultra-low latency

    let audit_config = config.get_channel_config("audit_logs");
    assert_eq!(audit_config.mode, TransportMode::MessageQueue);
    assert_eq!(audit_config.reliability, Reliability::GuaranteedDelivery);
    assert!(audit_config.retry.max_attempts > 1); // Multiple retries for reliability

    // Test YAML serialization
    let yaml = config.to_yaml().expect("Should serialize to YAML");
    let parsed_config = TransportConfig::from_yaml(&yaml)
        .expect("Should parse from YAML");
    
    assert_eq!(config.default_mode, parsed_config.default_mode);

    tracing::info!("Hybrid transport configuration test completed");
}

#[tokio::test]
async fn test_unix_socket_transport_integration() {
    // Test Unix domain socket transport for cross-process communication
    tracing::info!("Starting Unix socket transport integration test");

    use tempfile::tempdir;
    use torq_network::network::unix::{UnixSocketTransport, UnixSocketConfig};

    let temp_dir = tempdir().expect("Failed to create temp directory");
    let socket_path = temp_dir.path().join("integration_test.sock");

    let config = UnixSocketConfig {
        path: socket_path.clone(),
        buffer_size: 64 * 1024,
        max_message_size: 1024 * 1024,
        cleanup_on_drop: true,
    };

    // Create server
    let mut server = UnixSocketTransport::new(config.clone())
        .expect("Failed to create Unix socket server");
    
    server.bind().await.expect("Failed to bind Unix socket");

    // Test client connection and message exchange
    let server_handle = tokio::spawn(async move {
        let mut conn = server.accept().await.expect("Failed to accept connection");
        
        // Receive message
        let received_data = conn.receive().await.expect("Failed to receive data");
        assert_eq!(&received_data[..], b"Integration test message");
        
        // Send response
        conn.send(b"Server response").await.expect("Failed to send response");
        
        server.shutdown().await.expect("Failed to shutdown server");
    });

    // Give server time to start listening
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Connect client
    let mut client = UnixSocketTransport::connect(&socket_path).await
        .expect("Failed to connect Unix socket client");
    
    // Send message
    client.send(b"Integration test message").await
        .expect("Failed to send message");
    
    // Receive response
    let response = client.receive().await.expect("Failed to receive response");
    assert_eq!(&response[..], b"Server response");

    // Clean up
    client.close().await.expect("Failed to close client");
    server_handle.await.expect("Server task failed");

    tracing::info!("Unix socket transport integration test completed");
}

#[tokio::test]
async fn test_performance_characteristics() {
    // Test performance characteristics of the consolidated network crate
    tracing::info!("Starting performance characteristics test");

    let start_time = std::time::Instant::now();
    let system = ActorSystem::new();
    
    // Create high-throughput test actor
    let test_actor = IntegrationTestActor {
        id: "performance_test".to_string(),
        market_messages: vec![],
        signal_messages: vec![],
        execution_messages: vec![],
        total_processed: 0,
    };

    let actor_ref: ActorRef<TestMessage> = system.spawn(test_actor).await
        .expect("Failed to spawn performance test actor");

    // Send batch of messages to measure throughput
    let message_count = 1000;
    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    for i in 0..message_count {
        let pool_event = PoolSwapEvent {
            pool_address: [0xf0 | (i as u8 % 16); 20],
            token0_in: 1_000_000_000_000_000_000 + (i as u64 * 1000),
            token1_out: 2_000_000_000 + (i as u64 * 1000),
            timestamp_ns: timestamp_ns + i as u64,
            tx_hash: [(0xa0 + (i as u8 % 16)) as u8; 32],
            gas_used: 150_000 + (i as u64 * 100),
        };

        let message = TestMessage::Market(MarketMessage::Swap(Arc::new(pool_event)));
        actor_ref.send(message).await.expect("Failed to send performance test message");
    }

    // Wait for processing to complete
    tokio::time::sleep(Duration::from_millis(100)).await;

    let elapsed = start_time.elapsed();
    let messages_per_second = message_count as f64 / elapsed.as_secs_f64();

    tracing::info!(
        "Performance test: {} messages in {:.2}ms ({:.0} msg/s)",
        message_count,
        elapsed.as_millis(),
        messages_per_second
    );

    // Verify metrics
    let metrics = actor_ref.metrics();
    let stats = metrics.get_stats();
    
    assert!(stats.local_sends >= message_count, "Should record all local sends");
    assert!(stats.avg_local_latency_ns < 50_000.0, "Average latency should be under 50μs");
    assert!(stats.serialization_eliminated_mb > 0.0, "Should benefit from zero-copy optimization");

    // Check system metrics
    let system_metrics = system.metrics();
    let avg_processing = system_metrics.avg_processing_time_ns();
    assert!(avg_processing < 10_000.0, "Average processing time should be under 10μs");

    tracing::info!(
        "Performance metrics: avg_latency={:.2}ns, serialization_eliminated={:.4}MB, processing_time={:.2}ns",
        stats.avg_local_latency_ns,
        stats.serialization_eliminated_mb,
        avg_processing
    );

    tracing::info!("Performance characteristics test completed");
}

#[tokio::test]
async fn test_financial_data_integrity() {
    // Test financial data integrity with Protocol V2 checksums
    tracing::info!("Starting financial data integrity test");

    let protocol = Protocol::new();
    
    // Create financial message with precise values
    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let quote = QuoteUpdate {
        instrument_id: 98765,
        bid_price: 50_000_00000000_i64, // $50,000.00 (Bitcoin price in 8-decimal)
        ask_price: 50_001_50000000_i64, // $50,001.50
        bid_size: 500_000, // 0.5 BTC
        ask_size: 300_000, // 0.3 BTC
        timestamp_ns,
    };

    // Serialize to TLV - create a risk alert signal
    let risk_alert = RiskAlert {
        alert_id: 12345,
        alert_type: RiskAlertType::PositionLimit,
        severity: AlertSeverity::Critical,
        description: "Position size exceeds limit".to_string(),
        timestamp_ns,
    };
    
    let original_tlv = SignalMessage::RiskAlert(Arc::new(risk_alert)).to_tlv()
        .expect("Failed to serialize signal to TLV");

    // Validate original data
    assert!(protocol.validate_message(&original_tlv, ValidationMode::Strict)
        .expect("Validation failed"), "Original message should be valid");

    // Test corruption detection (critical for financial data)
    let mut corrupted_tlv = original_tlv.clone();
    if corrupted_tlv.len() > 50 {
        // Corrupt a single byte representing financial value (simulates 1 wei corruption)
        corrupted_tlv[48] = corrupted_tlv[48].wrapping_add(1);
    }

    // Corrupted message should be detected
    let corrupted_valid = protocol.validate_message(&corrupted_tlv, ValidationMode::Strict)
        .expect("Validation of corrupted message failed");
    
    assert!(!corrupted_valid, "Corrupted financial data should be detected");

    // Test precision preservation in round-trip
    let deserialized = SignalMessage::from_tlv(&original_tlv)
        .expect("Failed to deserialize TLV");
    
    // Verify it deserializes to the correct RiskAlert
    if let SignalMessage::RiskAlert(deserialized_alert) = deserialized {
        assert_eq!(deserialized_alert.alert_id, 12345, "Alert ID must be preserved");
        assert_eq!(deserialized_alert.severity, AlertSeverity::Critical, "Severity must be preserved");
        assert_eq!(deserialized_alert.timestamp_ns, timestamp_ns, "Timestamp precision must be preserved");
    } else {
        panic!("Deserialized message should be a RiskAlert");
    }

    tracing::info!("Financial data integrity test completed successfully");
}

#[tokio::test]
async fn test_error_handling_and_recovery() {
    // Test error handling and recovery mechanisms
    tracing::info!("Starting error handling and recovery test");

    let system = ActorSystem::new();
    
    // Create actor that can handle errors
    let test_actor = IntegrationTestActor {
        id: "error_test".to_string(),
        market_messages: vec![],
        signal_messages: vec![],
        execution_messages: vec![],
        total_processed: 0,
    };

    let actor_ref: ActorRef<TestMessage> = system.spawn(test_actor).await
        .expect("Failed to spawn error test actor");

    // Test timeout handling
    let pool_event = PoolSwapEvent {
        pool_address: [0x01; 20],
        token0_in: 100_000_000_000_000_000, // 0.1 WETH
        token1_out: 200_000_000, // 200 USDC
        timestamp_ns: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64,
        tx_hash: [0x01; 32],
        gas_used: 150_000,
    };
    
    let result = timeout(
        Duration::from_millis(100),
        actor_ref.send(TestMessage::Market(MarketMessage::Swap(Arc::new(pool_event))))
    ).await;

    assert!(result.is_ok(), "Message send should complete within timeout");

    // Test invalid TLV data handling
    let invalid_tlv_data = vec![0xFF; 10]; // Invalid TLV data
    let parse_result = TestMessage::from_tlv(&invalid_tlv_data);
    assert!(parse_result.is_err(), "Invalid TLV data should be rejected");

    // Test transport error handling
    assert!(actor_ref.is_healthy(), "Actor should remain healthy after error tests");

    tracing::info!("Error handling and recovery test completed");
}