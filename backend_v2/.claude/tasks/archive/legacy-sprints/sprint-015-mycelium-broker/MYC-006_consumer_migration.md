# MYC-006: Consumer Migration

## Status
- **Status**: pending
- **Assignee**: TBD
- **Estimated Effort**: 2 days
- **Priority**: High (completes message flow to consumers)

## Description
Migrate consumer services (FlashArbitrage strategy, Portfolio service, Dashboard, etc.) from relay-based message consumption to direct Mycelium broker subscription. This involves updating subscription logic, topic-based filtering, and message parsing while preserving all existing functionality and performance.

## Objectives
1. Update FlashArbitrage strategy to subscribe to broker topics
2. Migrate Portfolio and Dashboard services to broker subscription
3. Implement topic-based message filtering and routing
4. Ensure message consumption maintains >1.6M msg/s parsing performance
5. Preserve all existing message validation and processing logic

## Technical Approach

### Current Consumer Architecture
```rust
// Current: services_v2/strategies/flash_arbitrage/src/main.rs
impl FlashArbitrageStrategy {
    async fn run(&mut self) -> Result<(), StrategyError> {
        // Connect to multiple relays
        let market_data_client = RelayClient::connect(RelayDomain::MarketData).await?;
        let signal_client = RelayClient::connect(RelayDomain::Signal).await?;
        let execution_client = RelayClient::connect(RelayDomain::Execution).await?;

        loop {
            tokio::select! {
                msg = market_data_client.recv() => {
                    self.handle_market_data(msg?).await?;
                }
                msg = signal_client.recv() => {
                    self.handle_signal(msg?).await?;
                }
                msg = execution_client.recv() => {
                    self.handle_execution(msg?).await?;
                }
            }
        }
    }
}
```

### Target Consumer Architecture
```rust
// Target: services_v2/strategies/flash_arbitrage/src/main.rs
use mycelium_transport::Transport;
use mycelium_broker::BrokerMessage;

impl FlashArbitrageStrategy {
    async fn run(&mut self) -> Result<(), StrategyError> {
        // Single broker connection with multiple topic subscriptions
        let mut broker_client = BrokerSubscriber::new(&self.config.broker.socket_path).await?;
        
        // Subscribe to relevant topics
        broker_client.subscribe("market_data").await?;
        broker_client.subscribe("signals").await?;
        broker_client.subscribe("execution").await?;

        // Message processing loop
        while let Ok(broker_message) = broker_client.recv().await {
            match broker_message {
                BrokerMessage::Deliver { topic, payload } => {
                    self.route_message(&topic, &payload).await?;
                }
                BrokerMessage::SubscribeAck { topic } => {
                    tracing::info!("Subscribed to topic: {}", topic);
                }
                _ => {
                    tracing::warn!("Unexpected broker message type");
                }
            }
        }

        Ok(())
    }

    async fn route_message(&mut self, topic: &str, payload: &[u8]) -> Result<(), StrategyError> {
        // Parse TLV message (same as before)
        let parsed = self.parser.parse_message(payload)?;
        
        match topic {
            "market_data" => self.handle_market_data(parsed).await?,
            "signals" => self.handle_signal(parsed).await?,
            "execution" => self.handle_execution(parsed).await?,
            _ => {
                tracing::warn!("Unknown topic: {}", topic);
            }
        }
        
        Ok(())
    }
}
```

### Broker Subscriber Client
```rust
// services_v2/common/src/broker_subscriber.rs - NEW FILE
use mycelium_transport::{Transport, UnixSocketTransport};
use mycelium_broker::BrokerMessage;

pub struct BrokerSubscriber {
    transport: Box<dyn Transport>,
    subscriptions: HashSet<String>,
    message_buffer: VecDeque<BrokerMessage>,
    parser: TLVParser,
}

impl BrokerSubscriber {
    pub async fn new(broker_socket_path: &str) -> Result<Self, BrokerSubscriberError> {
        let transport = UnixSocketTransport::connect(broker_socket_path).await?;
        
        Ok(Self {
            transport: Box::new(transport),
            subscriptions: HashSet::new(),
            message_buffer: VecDeque::new(),
            parser: TLVParser::new(),
        })
    }

    pub async fn subscribe(&mut self, topic: &str) -> Result<(), BrokerSubscriberError> {
        let subscribe_msg = BrokerMessage::Subscribe {
            topic: topic.to_string(),
        };
        
        self.transport.send(&subscribe_msg.serialize()).await?;
        self.subscriptions.insert(topic.to_string());
        
        tracing::info!("Sent subscription request for topic: {}", topic);
        Ok(())
    }

    pub async fn unsubscribe(&mut self, topic: &str) -> Result<(), BrokerSubscriberError> {
        let unsubscribe_msg = BrokerMessage::Unsubscribe {
            topic: topic.to_string(),
        };
        
        self.transport.send(&unsubscribe_msg.serialize()).await?;
        self.subscriptions.remove(topic);
        
        tracing::info!("Sent unsubscription request for topic: {}", topic);
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<BrokerMessage, BrokerSubscriberError> {
        // Return buffered message if available
        if let Some(message) = self.message_buffer.pop_front() {
            return Ok(message);
        }

        // Receive new data from transport
        let data = self.transport.recv().await?;
        let message = BrokerMessage::parse(&data)?;

        Ok(message)
    }

    pub async fn recv_with_timeout(&mut self, timeout: Duration) -> Result<BrokerMessage, BrokerSubscriberError> {
        tokio::time::timeout(timeout, self.recv()).await
            .map_err(|_| BrokerSubscriberError::Timeout)?
    }

    // Selective message filtering based on TLV types
    pub async fn recv_filtered<F>(&mut self, filter: F) -> Result<BrokerMessage, BrokerSubscriberError>
    where
        F: Fn(&str, &[u8]) -> bool,
    {
        loop {
            let message = self.recv().await?;
            
            if let BrokerMessage::Deliver { ref topic, ref payload } = message {
                if filter(topic, payload) {
                    return Ok(message);
                }
                // Continue loop if message doesn't match filter
            } else {
                // Non-data messages always returned
                return Ok(message);
            }
        }
    }
}
```

### Topic-Based Message Filtering
```rust
// services_v2/strategies/flash_arbitrage/src/message_filter.rs - NEW FILE
pub struct MessageFilter {
    interested_tlv_types: HashSet<TLVType>,
    interested_instruments: HashSet<InstrumentId>,
    parser: TLVParser,
}

impl MessageFilter {
    pub fn new() -> Self {
        let mut filter = Self {
            interested_tlv_types: HashSet::new(),
            interested_instruments: HashSet::new(),
            parser: TLVParser::new(),
        };

        // Configure for flash arbitrage strategy
        filter.interested_tlv_types.insert(TLVType::Trade);
        filter.interested_tlv_types.insert(TLVType::Quote);
        filter.interested_tlv_types.insert(TLVType::ArbitrageSignal);
        
        filter
    }

    pub fn is_relevant(&self, topic: &str, payload: &[u8]) -> bool {
        // Quick topic-level filtering
        match topic {
            "market_data" | "signals" => {
                // Parse and check TLV content
                self.is_tlv_relevant(payload)
            }
            "execution" => {
                // Always process execution messages for this strategy
                true
            }
            _ => false,
        }
    }

    fn is_tlv_relevant(&self, payload: &[u8]) -> bool {
        // Fast path: check if any TLV types match without full parsing
        match self.parser.parse_message(payload) {
            Ok(parsed) => {
                for tlv in &parsed.tlv_extensions {
                    if let Ok(tlv_type) = TLVType::try_from(tlv.header.tlv_type) {
                        if self.interested_tlv_types.contains(&tlv_type) {
                            return true;
                        }
                    }
                }
                false
            }
            Err(_) => {
                // If parsing fails, let the main handler deal with it
                true
            }
        }
    }

    pub fn add_instrument(&mut self, instrument_id: InstrumentId) {
        self.interested_instruments.insert(instrument_id);
    }

    pub fn remove_instrument(&mut self, instrument_id: InstrumentId) {
        self.interested_instruments.remove(&instrument_id);
    }
}
```

### Service Configuration Updates
```rust
// services_v2/strategies/flash_arbitrage/src/config.rs - UPDATED
#[derive(Debug, Clone, serde::Deserialize)]
pub struct FlashArbitrageConfig {
    pub strategy: StrategyConfig,
    pub broker: BrokerSubscriberConfig, // NEW: replaces relay configs
    pub risk_management: RiskConfig,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct BrokerSubscriberConfig {
    pub socket_path: String,
    pub topics: Vec<String>,
    pub message_buffer_size: usize,
    pub subscription_timeout_ms: u64,
    pub reconnect_attempts: u32,
}

impl Default for BrokerSubscriberConfig {
    fn default() -> Self {
        Self {
            socket_path: "/tmp/mycelium.sock".to_string(),
            topics: vec!["market_data".to_string(), "signals".to_string(), "execution".to_string()],
            message_buffer_size: 10000,
            subscription_timeout_ms: 5000,
            reconnect_attempts: 10,
        }
    }
}
```

### Multi-Service Consumer Pattern
```rust
// services_v2/dashboard/src/message_handler.rs
impl DashboardService {
    pub async fn start_message_consumer(&mut self) -> Result<(), DashboardError> {
        let mut subscriber = BrokerSubscriber::new(&self.config.broker.socket_path).await?;
        
        // Dashboard interested in all message types for monitoring
        subscriber.subscribe("market_data").await?;
        subscriber.subscribe("signals").await?;
        subscriber.subscribe("execution").await?;

        let message_handler = Arc::clone(&self.message_handler);
        
        tokio::spawn(async move {
            while let Ok(broker_message) = subscriber.recv().await {
                if let BrokerMessage::Deliver { topic, payload } = broker_message {
                    // Route to appropriate dashboard component
                    match topic.as_str() {
                        "market_data" => {
                            message_handler.handle_market_data(&payload).await;
                        }
                        "signals" => {
                            message_handler.handle_signal(&payload).await;
                        }
                        "execution" => {
                            message_handler.handle_execution(&payload).await;
                        }
                        _ => {}
                    }
                }
            }
        });

        Ok(())
    }
}
```

### Performance-Optimized Consumer
```rust
// For high-throughput consumers
impl HighPerformanceConsumer {
    pub async fn run_optimized(&mut self) -> Result<(), ConsumerError> {
        let mut subscriber = BrokerSubscriber::new(&self.config.broker.socket_path).await?;
        subscriber.subscribe("market_data").await?;

        // Pre-allocate parsing buffers
        let mut parse_buffer = Vec::with_capacity(4096);
        
        loop {
            match subscriber.recv().await {
                Ok(BrokerMessage::Deliver { topic: _, payload }) => {
                    // Zero-copy parsing where possible
                    match self.parser.parse_message(&payload) {
                        Ok(parsed) => {
                            self.process_message_fast(&parsed).await?;
                        }
                        Err(e) => {
                            tracing::warn!("Parse error: {}", e);
                            continue;
                        }
                    }
                }
                Ok(_) => {
                    // Handle other message types (acks, etc.)
                    continue;
                }
                Err(e) => {
                    tracing::error!("Subscriber error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}
```

## Acceptance Criteria

### Functional Migration
- [ ] FlashArbitrage strategy subscribes to broker topics
- [ ] Portfolio service migrated to broker subscription
- [ ] Dashboard service receives all message types via broker
- [ ] All consumers preserve exact message processing logic

### Performance Requirements
- [ ] Message parsing maintains >1.6M msg/s throughput
- [ ] Topic filtering adds <2μs overhead per message
- [ ] Subscription setup completes in <100ms
- [ ] Memory usage unchanged from relay client

### Reliability Features
- [ ] Subscription acknowledgments handled correctly
- [ ] Topic unsubscription works reliably
- [ ] Connection failures trigger proper reconnection
- [ ] Message ordering preserved within topics

### Message Processing
- [ ] TLV message parsing identical to relay version
- [ ] Message validation and error handling preserved
- [ ] Topic routing logic correctly distributes messages
- [ ] Filtered subscription reduces unnecessary processing

## Dependencies
- **Upstream**: MYC-002 (Transport Layer), MYC-003 (Broker Layer), MYC-004 (Codec Consolidation)
- **Downstream**: MYC-007 (Integration Testing)
- **External**: None (internal migration)

## Testing Requirements

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn broker_subscriber_basic_functionality() {
        let mut subscriber = MockBrokerSubscriber::new();
        
        // Test subscription
        subscriber.subscribe("test_topic").await.unwrap();
        assert!(subscriber.subscriptions.contains("test_topic"));
        
        // Test message reception
        let test_message = BrokerMessage::Deliver {
            topic: "test_topic".to_string(),
            payload: vec![1, 2, 3, 4],
        };
        
        subscriber.inject_message(test_message.clone());
        let received = subscriber.recv().await.unwrap();
        assert_eq!(received, test_message);
    }

    #[tokio::test]
    async fn message_filter_relevance_check() {
        let filter = MessageFilter::new();
        
        // Create test TLV message with Trade type
        let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, MessageSource::Test);
        builder.add_tlv(TLVType::Trade, &TradeTLV::default()).unwrap();
        let message = builder.build().unwrap();
        
        // Should be relevant for market data topic
        assert!(filter.is_relevant("market_data", &message));
        
        // Should not be relevant for unknown topic
        assert!(!filter.is_relevant("unknown", &message));
    }

    #[tokio::test]
    async fn flash_arbitrage_message_routing() {
        let mut strategy = FlashArbitrageStrategy::new(test_config()).await.unwrap();
        
        // Test market data routing
        let trade_message = create_test_trade_message();
        strategy.route_message("market_data", &trade_message).await.unwrap();
        
        // Verify appropriate handler was called
        assert_eq!(strategy.test_counters.market_data_handled, 1);
        
        // Test signal routing
        let signal_message = create_test_signal_message();
        strategy.route_message("signals", &signal_message).await.unwrap();
        assert_eq!(strategy.test_counters.signals_handled, 1);
    }
}
```

### Performance Tests
```rust
#[cfg(test)]
mod perf_tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn consumer_parsing_performance() {
        let mut subscriber = create_test_subscriber().await;
        let test_message = create_1kb_tlv_message();
        let num_messages = 1_000_000;
        
        let start = std::time::Instant::now();
        
        for _ in 0..num_messages {
            subscriber.inject_message(BrokerMessage::Deliver {
                topic: "test".to_string(),
                payload: test_message.clone(),
            });
            
            let received = subscriber.recv().await.unwrap();
            if let BrokerMessage::Deliver { payload, .. } = received {
                // Parse message (this is what we're benchmarking)
                let _parsed = TLVParser::new().parse_message(&payload).unwrap();
            }
        }
        
        let elapsed = start.elapsed();
        let parse_rate = num_messages as f64 / elapsed.as_secs_f64();
        
        println!("Consumer parsing: {:.0} msg/s", parse_rate);
        assert!(parse_rate > 1_600_000.0); // >1.6M msg/s requirement
    }

    #[tokio::test]
    #[ignore]
    async fn topic_filtering_overhead() {
        let filter = MessageFilter::new();
        let test_messages = create_mixed_message_types();
        let num_iterations = 100_000;
        
        let start = std::time::Instant::now();
        
        for _ in 0..num_iterations {
            for message in &test_messages {
                filter.is_relevant("market_data", message);
            }
        }
        
        let elapsed = start.elapsed();
        let filter_rate = (num_iterations * test_messages.len()) as f64 / elapsed.as_secs_f64();
        
        println!("Filter rate: {:.0} msg/s", filter_rate);
        // Filtering should add <2μs per message
        let avg_time_per_filter = elapsed / (num_iterations * test_messages.len() as u32);
        assert!(avg_time_per_filter.as_micros() < 2);
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn end_to_end_consumer_flow() {
    // Start test broker
    let broker = start_test_broker().await;
    
    // Start producer (from MYC-005 tests)
    let producer = start_test_producer(&broker.socket_path()).await;
    
    // Create consumer
    let mut consumer = BrokerSubscriber::new(&broker.socket_path()).await.unwrap();
    consumer.subscribe("market_data").await.unwrap();
    
    // Wait for subscription acknowledgment
    let ack = consumer.recv().await.unwrap();
    assert!(matches!(ack, BrokerMessage::SubscribeAck { .. }));
    
    // Producer sends message
    let test_trade = create_test_trade_event();
    producer.publish_trade(test_trade.clone()).await.unwrap();
    
    // Consumer receives message
    let received = consumer.recv().await.unwrap();
    if let BrokerMessage::Deliver { topic, payload } = received {
        assert_eq!(topic, "market_data");
        
        let parsed = TLVParser::new().parse_message(&payload).unwrap();
        let trade_tlv = parsed.get_tlv::<TradeTLV>(TLVType::Trade).unwrap();
        assert_eq!(trade_tlv.price, (test_trade.price * PRICE_SCALE_FACTOR).round() as i64);
    } else {
        panic!("Expected Deliver message");
    }
}

#[tokio::test]
async fn multiple_consumer_fanout() {
    let broker = start_test_broker().await;
    let producer = start_test_producer(&broker.socket_path()).await;
    
    // Create multiple consumers for same topic
    let mut consumer1 = BrokerSubscriber::new(&broker.socket_path()).await.unwrap();
    let mut consumer2 = BrokerSubscriber::new(&broker.socket_path()).await.unwrap();
    
    consumer1.subscribe("market_data").await.unwrap();
    consumer2.subscribe("market_data").await.unwrap();
    
    // Skip subscription acks
    consumer1.recv().await.unwrap();
    consumer2.recv().await.unwrap();
    
    // Producer sends one message
    producer.publish_trade(create_test_trade_event()).await.unwrap();
    
    // Both consumers should receive the message (fanout)
    let msg1 = consumer1.recv().await.unwrap();
    let msg2 = consumer2.recv().await.unwrap();
    
    assert!(matches!(msg1, BrokerMessage::Deliver { .. }));
    assert!(matches!(msg2, BrokerMessage::Deliver { .. }));
}
```

## Rollback Plan

### If Performance Issues
1. Revert to relay clients with optimizations
2. Implement message batching at consumer level
3. Use more aggressive message filtering to reduce processing

### If Subscription Reliability Issues
1. Add more robust subscription acknowledgment handling
2. Implement periodic subscription health checks
3. Fall back to polling-based message consumption

### If Message Ordering Issues
1. Add sequence number validation in consumer
2. Implement message ordering guarantees at broker level
3. Use single-threaded message processing if necessary

## Technical Notes

### Design Decisions
- **Single Connection**: One broker connection instead of multiple relay connections
- **Topic-Based Routing**: Flexible message filtering based on topics
- **Unified Subscriber**: Reusable client for all consumer services
- **Message Filtering**: Optional optimization for high-throughput consumers

### Performance Optimizations
- **Buffer Pre-allocation**: Reuse parsing buffers to reduce allocations
- **Zero-Copy Parsing**: Minimize data copying during message processing
- **Selective Filtering**: Skip irrelevant messages before full parsing
- **Async Processing**: Non-blocking message consumption

### Migration Strategy
- **Preserve Message Processing**: No changes to actual business logic
- **Topic Mapping**: Simple mapping from relay domains to topics
- **Configuration Driven**: Easy to switch between implementations
- **Gradual Rollout**: Service-by-service migration for safety

## Validation Steps

1. **Unit Test Coverage**:
   ```bash
   cargo test --package strategies consumer_migration
   cargo test --package dashboard consumer_migration
   ```

2. **Performance Validation**:
   ```bash
   cargo test --package strategies --release -- --ignored perf_tests
   ```

3. **Integration Testing**:
   ```bash
   # End-to-end message flow tests
   cargo test --workspace integration_consumer_flow
   ```

4. **Service-Specific Testing**:
   ```bash
   # Test each consumer service individually
   cargo test --package flash_arbitrage broker_integration
   cargo test --package portfolio broker_integration
   cargo test --package dashboard broker_integration
   ```

This migration completes the transition from relay-based to broker-based architecture while maintaining all existing consumer functionality and performance characteristics, enabling the integration testing phase in MYC-007.