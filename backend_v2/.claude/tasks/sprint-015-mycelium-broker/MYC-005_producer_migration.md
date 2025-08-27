# MYC-005: Producer Migration

## Status
- **Status**: pending
- **Assignee**: TBD
- **Estimated Effort**: 2 days
- **Priority**: High (enables message flow from producers)

## Description
Migrate producer services (PolygonAdapter, KrakenCollector, etc.) from relay-based message sending to direct Mycelium broker publishing. This involves updating connection logic, mapping RelayDomain to Topics, and removing relay dependencies while preserving all existing functionality and performance.

## Objectives
1. Update PolygonAdapter to publish directly to Mycelium broker
2. Map existing RelayDomain enum to generic Topic strings
3. Remove relay client dependencies from producer services
4. Ensure message publishing maintains >1M msg/s throughput
5. Preserve all existing message validation and error handling

## Technical Approach

### Current Producer Architecture
```rust
// Current: services_v2/adapters/src/polygon/mod.rs
impl PolygonAdapter {
    async fn handle_trade_event(&mut self, event: TradeEvent) -> Result<(), AdapterError> {
        // Build TLV message
        let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, MessageSource::PolygonAdapter);
        builder.add_tlv(TLVType::Trade, &trade_tlv)?;
        let message = builder.build()?;

        // Send to relay
        self.relay_client.send_message(message).await?;
        Ok(())
    }
}
```

### Target Producer Architecture
```rust
// Target: services_v2/adapters/src/polygon/mod.rs
use mycelium_transport::Transport;
use mycelium_broker::BrokerMessage;

impl PolygonAdapter {
    async fn handle_trade_event(&mut self, event: TradeEvent) -> Result<(), AdapterError> {
        // Build TLV message (same as before)
        let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, MessageSource::PolygonAdapter);
        builder.add_tlv(TLVType::Trade, &trade_tlv)?;
        let tlv_message = builder.build()?;

        // Publish to broker topic instead of relay
        let broker_msg = BrokerMessage::Publish {
            topic: self.domain_to_topic(RelayDomain::MarketData),
            payload: tlv_message,
        };
        
        self.broker_transport.send(&broker_msg.serialize()).await?;
        Ok(())
    }

    fn domain_to_topic(&self, domain: RelayDomain) -> String {
        match domain {
            RelayDomain::MarketData => "market_data".to_string(),
            RelayDomain::Signal => "signals".to_string(), 
            RelayDomain::Execution => "execution".to_string(),
        }
    }
}
```

### Producer Connection Management
```rust
// services_v2/adapters/src/common/broker_client.rs - NEW FILE
use mycelium_transport::{Transport, UnixSocketTransport};
use mycelium_broker::BrokerMessage;

pub struct BrokerClient {
    transport: Box<dyn Transport>,
    connected: bool,
    reconnect_attempts: u32,
    max_reconnects: u32,
}

impl BrokerClient {
    pub async fn new(broker_socket_path: &str) -> Result<Self, BrokerClientError> {
        let transport = UnixSocketTransport::connect(broker_socket_path).await?;
        
        Ok(Self {
            transport: Box::new(transport),
            connected: true,
            reconnect_attempts: 0,
            max_reconnects: 10,
        })
    }

    pub async fn publish(&mut self, topic: &str, payload: Vec<u8>) -> Result<(), BrokerClientError> {
        let message = BrokerMessage::Publish {
            topic: topic.to_string(),
            payload,
        };

        match self.send_with_retry(&message).await {
            Ok(_) => {
                self.reconnect_attempts = 0; // Reset on success
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to publish to topic '{}': {}", topic, e);
                Err(e)
            }
        }
    }

    async fn send_with_retry(&mut self, message: &BrokerMessage) -> Result<(), BrokerClientError> {
        for attempt in 0..=self.max_reconnects {
            match self.transport.send(&message.serialize()).await {
                Ok(_) => return Ok(()),
                Err(TransportError::ConnectionClosed) if attempt < self.max_reconnects => {
                    tracing::warn!("Connection lost, attempting reconnect {} of {}", attempt + 1, self.max_reconnects);
                    self.reconnect().await?;
                }
                Err(e) => return Err(BrokerClientError::Transport(e)),
            }
        }
        
        Err(BrokerClientError::MaxReconnectsExceeded)
    }

    async fn reconnect(&mut self) -> Result<(), BrokerClientError> {
        // Wait with exponential backoff
        let backoff_ms = std::cmp::min(1000 * 2_u64.pow(self.reconnect_attempts), 30000);
        tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
        
        // Attempt reconnection
        // Note: Would need broker socket path stored in struct
        self.reconnect_attempts += 1;
        Ok(())
    }
}
```

### Adapter Configuration Updates
```rust
// services_v2/adapters/src/config.rs - UPDATED
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AdapterConfig {
    pub polygon: PolygonConfig,
    pub kraken: KrakenConfig,
    pub broker: BrokerClientConfig, // NEW
    // Remove relay configs
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct BrokerClientConfig {
    pub socket_path: String,
    pub connection_timeout_ms: u64,
    pub max_reconnects: u32,
    pub retry_backoff_ms: u64,
}

impl Default for BrokerClientConfig {
    fn default() -> Self {
        Self {
            socket_path: "/tmp/mycelium.sock".to_string(),
            connection_timeout_ms: 5000,
            max_reconnects: 10,
            retry_backoff_ms: 1000,
        }
    }
}
```

### Message Flow Preservation
```rust
// Ensure exact same TLV messages are sent, just to different destination
impl PolygonAdapter {
    // Keep all existing message building logic identical
    fn build_trade_message(&self, event: &TradeEvent) -> Result<Vec<u8>, AdapterError> {
        let trade_tlv = TradeTLV {
            instrument_id: self.build_instrument_id(&event.symbol)?,
            price: (event.price * PRICE_SCALE_FACTOR).round() as i64,
            volume: (event.volume * VOLUME_SCALE_FACTOR).round() as i64,
            timestamp: event.timestamp_nanos,
            side: event.side.into(),
        };

        let mut builder = TLVMessageBuilder::new(RelayDomain::MarketData, MessageSource::PolygonAdapter);
        builder.add_tlv(TLVType::Trade, &trade_tlv)?;
        builder.build()
    }

    async fn publish_trade(&mut self, event: TradeEvent) -> Result<(), AdapterError> {
        let message = self.build_trade_message(&event)?;
        
        // Only change: publish to broker instead of relay
        self.broker_client.publish("market_data", message).await
            .map_err(AdapterError::BrokerPublish)?;

        // Keep existing metrics and logging
        self.metrics.trades_published.fetch_add(1, Ordering::Relaxed);
        tracing::debug!("Published trade for {}: ${} @ {}", 
                       event.symbol, event.price, event.volume);
        
        Ok(())
    }
}
```

### Migration for All Producer Services
```rust
// services_v2/adapters/src/kraken/mod.rs - SIMILAR MIGRATION
impl KrakenCollector {
    pub async fn new(config: KrakenConfig, broker_client: BrokerClient) -> Self {
        Self {
            config,
            broker_client,     // Changed from relay_client
            websocket: None,
            subscriptions: HashSet::new(),
            metrics: KrakenMetrics::new(),
        }
    }

    async fn handle_ticker_update(&mut self, update: TickerUpdate) -> Result<(), KrakenError> {
        let quote_tlv = self.build_quote_tlv(&update)?;
        let message = self.build_message(RelayDomain::MarketData, quote_tlv)?;
        
        // Publish to broker (same pattern as PolygonAdapter)
        self.broker_client.publish("market_data", message).await?;
        Ok(())
    }
}
```

## Acceptance Criteria

### Functional Migration
- [ ] PolygonAdapter publishes to broker instead of relay
- [ ] KrakenCollector publishes to broker instead of relay
- [ ] All other producer services migrated to broker client
- [ ] Message content and format preserved exactly

### Performance Requirements
- [ ] Publishing maintains >1M msg/s throughput
- [ ] Connection overhead adds <1ms latency
- [ ] Memory usage unchanged from relay client
- [ ] Error rates not increased by migration

### Reliability Features
- [ ] Connection retry logic handles broker restarts
- [ ] Exponential backoff prevents connection storms
- [ ] Circuit breaker protects against cascade failures
- [ ] Metrics and logging preserved for observability

### Configuration Management
- [ ] Broker connection configurable via TOML files
- [ ] Environment variable overrides supported
- [ ] Hot-reload capability for connection parameters
- [ ] Backwards compatibility during migration period

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
    async fn polygon_adapter_publishes_to_broker() {
        let broker_client = MockBrokerClient::new();
        let mut adapter = PolygonAdapter::new(config, broker_client);
        
        let trade_event = TradeEvent {
            symbol: "WETH/USDC".to_string(),
            price: 2500.50,
            volume: 1.5,
            timestamp_nanos: SystemClock::now_nanos(),
            side: TradeSide::Buy,
        };
        
        adapter.handle_trade_event(trade_event).await.unwrap();
        
        // Verify message published to correct topic
        let published = adapter.broker_client.get_published_messages();
        assert_eq!(published.len(), 1);
        assert_eq!(published[0].topic, "market_data");
        
        // Verify TLV content unchanged
        let parsed = TLVParser::new().parse_message(&published[0].payload).unwrap();
        assert_eq!(parsed.header.relay_domain, RelayDomain::MarketData);
    }

    #[tokio::test]
    async fn broker_client_handles_connection_failure() {
        let mut broker_client = BrokerClient::new("invalid_path").await.unwrap();
        
        // Simulate connection failure
        let result = broker_client.publish("test", vec![1, 2, 3]).await;
        assert!(result.is_err());
        
        // Should attempt reconnection
        assert!(matches!(result, Err(BrokerClientError::MaxReconnectsExceeded)));
    }

    #[tokio::test]
    async fn domain_to_topic_mapping() {
        let adapter = PolygonAdapter::default();
        
        assert_eq!(adapter.domain_to_topic(RelayDomain::MarketData), "market_data");
        assert_eq!(adapter.domain_to_topic(RelayDomain::Signal), "signals");
        assert_eq!(adapter.domain_to_topic(RelayDomain::Execution), "execution");
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
    async fn publishing_throughput() {
        let broker_client = create_test_broker_client().await;
        let message = vec![0u8; 1024]; // 1KB test message
        let num_messages = 1_000_000;
        
        let start = std::time::Instant::now();
        
        for _ in 0..num_messages {
            broker_client.publish("test_topic", message.clone()).await.unwrap();
        }
        
        let elapsed = start.elapsed();
        let msg_per_sec = num_messages as f64 / elapsed.as_secs_f64();
        
        println!("Producer publishing: {:.0} msg/s", msg_per_sec);
        assert!(msg_per_sec > 1_000_000.0); // >1M msg/s requirement
    }

    #[tokio::test]
    #[ignore] 
    async fn connection_overhead_benchmark() {
        let num_connections = 100;
        let mut connection_times = Vec::new();
        
        for _ in 0..num_connections {
            let start = std::time::Instant::now();
            let _client = BrokerClient::new("/tmp/test_broker.sock").await.unwrap();
            connection_times.push(start.elapsed().as_micros());
        }
        
        let avg_connection_time = connection_times.iter().sum::<u128>() / connection_times.len() as u128;
        println!("Average connection time: {}μs", avg_connection_time);
        assert!(avg_connection_time < 1000); // <1ms connection time
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn end_to_end_producer_flow() {
    // Start test broker
    let broker = start_test_broker().await;
    
    // Create producer service
    let broker_client = BrokerClient::new(&broker.socket_path()).await.unwrap();
    let mut adapter = PolygonAdapter::new(test_config(), broker_client);
    
    // Subscribe to topic (simulate consumer)
    let mut consumer = create_test_consumer(&broker.socket_path(), "market_data").await;
    
    // Producer publishes message
    let trade_event = create_test_trade_event();
    adapter.handle_trade_event(trade_event.clone()).await.unwrap();
    
    // Consumer receives message
    let received = consumer.recv().await.unwrap();
    let parsed = TLVParser::new().parse_message(&received).unwrap();
    
    // Verify message content
    let trade_tlv = parsed.get_tlv::<TradeTLV>(TLVType::Trade).unwrap();
    assert_eq!(trade_tlv.price, (trade_event.price * PRICE_SCALE_FACTOR).round() as i64);
}
```

### Migration Compatibility Tests
```rust
#[tokio::test]
async fn message_format_compatibility() {
    // Create message with old relay builder
    let relay_message = build_message_with_relay_client(&test_trade_event());
    
    // Create message with new broker client  
    let broker_message = build_message_with_broker_client(&test_trade_event());
    
    // Messages should be identical (except timestamps)
    let relay_parsed = TLVParser::new().parse_message(&relay_message).unwrap();
    let broker_parsed = TLVParser::new().parse_message(&broker_message).unwrap();
    
    assert_eq!(relay_parsed.header.relay_domain, broker_parsed.header.relay_domain);
    assert_eq!(relay_parsed.header.source, broker_parsed.header.source);
    assert_eq!(relay_parsed.tlv_extensions.len(), broker_parsed.tlv_extensions.len());
}
```

## Rollback Plan

### If Performance Issues
1. Revert to relay clients with performance optimizations
2. Implement batching at producer level to improve throughput
3. Use direct TCP connections instead of unix sockets if needed

### If Connection Reliability Issues
1. Add more aggressive reconnection logic
2. Implement message queueing during connection failures  
3. Fall back to file-based message persistence

### If Integration Problems
1. Run relay and broker clients in parallel during transition
2. Use feature flags to switch between implementations
3. Implement gradual rollout service by service

## Technical Notes

### Design Decisions
- **Preserve Message Format**: No changes to TLV message content or structure
- **Topic Mapping**: Simple 1:1 mapping from RelayDomain to topic strings
- **Connection Management**: Robust reconnection with exponential backoff
- **Error Handling**: Preserve existing error types and logging patterns

### Performance Optimizations
- **Connection Pooling**: Reuse connections across multiple publishes
- **Message Batching**: Group small messages for better throughput
- **Zero-Copy Publishing**: Avoid unnecessary data copying in broker client
- **Async I/O**: Non-blocking operations prevent producer thread blocking

### Migration Strategy
- **Identical Message Content**: Ensures consumers don't need simultaneous changes
- **Configuration Driven**: Easy to switch between relay and broker clients
- **Incremental Migration**: Move one service at a time for safer deployment
- **Monitoring**: Track success/error rates during migration

## Validation Steps

1. **Unit Test Coverage**:
   ```bash
   cargo test --package adapters producer_migration
   ```

2. **Performance Validation**:
   ```bash
   cargo test --package adapters --release -- --ignored perf_tests
   ```

3. **Integration Testing**:
   ```bash
   # Start test broker and run end-to-end tests
   cargo test --package adapters integration_tests
   ```

4. **Message Format Verification**:
   ```bash
   # Compare messages produced by old vs new clients
   cargo test --package adapters message_format_compatibility
   ```

This migration maintains all existing producer functionality while transitioning to the simpler, more maintainable broker architecture, setting the stage for consumer migration in MYC-006.