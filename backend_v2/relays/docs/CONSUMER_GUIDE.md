# Relay Consumer Implementation Guide

This guide explains how services should connect to and consume messages from the relay infrastructure.

## Architecture Overview

```
Producer Service → Relay (Broadcasting) → Consumer Services
                      ↓
               Topic Filtering
```

Services connect to relays as **consumers** without depending on the relay implementation. This maintains clean architectural boundaries.

## Consumer Implementation Pattern

### 1. Connection Setup

Services connect to relays via Unix domain sockets or TCP:

```rust
use tokio::net::UnixStream;
use alphapulse_protocol_v2::{MessageHeader, TLVMessage};

pub struct RelayConsumer {
    socket_path: String,
    stream: Option<UnixStream>,
}

impl RelayConsumer {
    pub async fn connect(&mut self) -> Result<()> {
        let stream = UnixStream::connect(&self.socket_path).await?;
        self.stream = Some(stream);
        
        // Send subscription request with topics
        self.subscribe_to_topics(vec!["market_data_polygon"]).await?;
        Ok(())
    }
}
```

### 2. Topic Subscription

Consumers tell the relay which topics they want:

```rust
async fn subscribe_to_topics(&mut self, topics: Vec<&str>) -> Result<()> {
    // Send subscription message to relay
    let subscription = SubscriptionRequest {
        consumer_id: self.consumer_id.clone(),
        topics: topics.into_iter().map(String::from).collect(),
    };
    
    // Serialize and send
    let data = serialize_subscription(&subscription)?;
    self.stream.write_all(&data).await?;
    Ok(())
}
```

### 3. Message Reception Loop

```rust
pub async fn run(&mut self) -> Result<()> {
    let mut buffer = vec![0u8; 65536];
    
    loop {
        // Read message from relay
        let n = self.stream.read(&mut buffer).await?;
        
        if n == 0 {
            // Connection closed, reconnect
            self.reconnect().await?;
            continue;
        }
        
        // Parse message header
        let header = self.parse_header(&buffer[..n])?;
        
        // Check if we care about this message
        if self.should_process(&header) {
            self.process_message(&buffer[..n]).await?;
        }
    }
}
```

### 4. Message Processing

Domain-specific message handling:

```rust
async fn process_message(&mut self, data: &[u8]) -> Result<()> {
    let header = parse_header(data)?;
    
    match header.message_type {
        TLVType::Trade => {
            let trade = TradeTLV::from_bytes(&data[HEADER_SIZE..])?;
            self.handle_trade(trade).await?;
        }
        TLVType::PoolSwap => {
            let swap = PoolSwapTLV::from_bytes(&data[HEADER_SIZE..])?;
            self.handle_pool_swap(swap).await?;
        }
        _ => {} // Ignore other message types
    }
    
    Ok(())
}
```

## Connection Configurations

### Market Data Relay
```rust
const MARKET_DATA_SOCKET: &str = "/tmp/alphapulse/market_data.sock";
const MARKET_DATA_TOPICS: &[&str] = &[
    "market_data_polygon",
    "market_data_uniswap_v2",
    "market_data_uniswap_v3",
];
```

### Signal Relay
```rust
const SIGNAL_SOCKET: &str = "/tmp/alphapulse/signals.sock";
const SIGNAL_TOPICS: &[&str] = &[
    "arbitrage_signals",
    "trend_signals",
];
```

### Execution Relay
```rust
const EXECUTION_SOCKET: &str = "/tmp/alphapulse/execution.sock";
const EXECUTION_TOPICS: &[&str] = &[
    "orders",
    "fills",
];
```

## Best Practices

### 1. Reconnection Logic

Always implement automatic reconnection:

```rust
async fn reconnect(&mut self) -> Result<()> {
    let mut attempts = 0;
    let max_attempts = 10;
    let mut delay = Duration::from_millis(100);
    
    while attempts < max_attempts {
        match UnixStream::connect(&self.socket_path).await {
            Ok(stream) => {
                self.stream = Some(stream);
                info!("Reconnected to relay");
                return Ok(());
            }
            Err(e) => {
                warn!("Reconnection attempt {} failed: {}", attempts, e);
                tokio::time::sleep(delay).await;
                delay *= 2; // Exponential backoff
                attempts += 1;
            }
        }
    }
    
    Err(anyhow!("Failed to reconnect after {} attempts", max_attempts))
}
```

### 2. Buffering and Backpressure

Handle message bursts gracefully:

```rust
pub struct BufferedConsumer {
    buffer: VecDeque<Message>,
    max_buffer_size: usize,
}

impl BufferedConsumer {
    async fn process_with_backpressure(&mut self) -> Result<()> {
        if self.buffer.len() > self.max_buffer_size {
            // Apply backpressure - slow down consumption
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        // Process messages in batches
        let batch_size = 100.min(self.buffer.len());
        for _ in 0..batch_size {
            if let Some(msg) = self.buffer.pop_front() {
                self.process_message(msg).await?;
            }
        }
        
        Ok(())
    }
}
```

### 3. Metrics and Monitoring

Track consumer health:

```rust
pub struct ConsumerMetrics {
    messages_received: u64,
    messages_processed: u64,
    messages_dropped: u64,
    reconnections: u64,
    last_message_time: Instant,
}

impl ConsumerMetrics {
    pub fn is_healthy(&self) -> bool {
        // Check if receiving messages
        self.last_message_time.elapsed() < Duration::from_secs(30)
    }
}
```

### 4. Topic-Based Filtering

Only subscribe to needed topics:

```rust
// Good: Specific topics
consumer.subscribe(&["market_data_polygon", "market_data_uniswap_v2"]);

// Bad: Subscribe to everything
consumer.subscribe(&["market_data_all"]);  // Too broad
```

## Example: Flash Arbitrage Consumer

Complete example for arbitrage strategy:

```rust
use alphapulse_protocol_v2::{PoolSwapTLV, TradeTLV};
use rust_decimal::Decimal;

pub struct ArbitrageConsumer {
    relay_socket: String,
    pool_states: HashMap<u64, PoolState>,
    detector: ArbitrageDetector,
}

impl ArbitrageConsumer {
    pub async fn run(&mut self) -> Result<()> {
        // Connect to market data relay
        self.connect().await?;
        
        // Subscribe to DEX topics
        self.subscribe(&[
            "market_data_uniswap_v2",
            "market_data_uniswap_v3",
            "market_data_sushiswap",
        ]).await?;
        
        // Process messages
        loop {
            match self.receive_message().await {
                Ok(msg) => {
                    if let Some(opportunity) = self.process_for_arbitrage(msg).await? {
                        self.execute_arbitrage(opportunity).await?;
                    }
                }
                Err(e) => {
                    error!("Consumer error: {}", e);
                    self.reconnect().await?;
                }
            }
        }
    }
    
    async fn process_for_arbitrage(&mut self, msg: Message) -> Result<Option<Opportunity>> {
        // Update pool state
        self.update_pool_state(&msg)?;
        
        // Check for arbitrage
        self.detector.check_opportunity(&self.pool_states)
    }
}
```

## Testing Your Consumer

### Unit Tests
```rust
#[tokio::test]
async fn test_consumer_subscription() {
    let mut consumer = TestConsumer::new();
    consumer.subscribe(&["test_topic"]).await.unwrap();
    assert_eq!(consumer.subscribed_topics(), vec!["test_topic"]);
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_consumer_with_relay() {
    // Start test relay
    let relay = start_test_relay().await;
    
    // Create consumer
    let mut consumer = RelayConsumer::new(relay.socket_path());
    consumer.connect().await.unwrap();
    
    // Send test message through relay
    relay.broadcast_message(test_message()).await;
    
    // Verify consumer receives it
    let msg = consumer.receive_message().await.unwrap();
    assert_eq!(msg.header.message_type, 1);
}
```

## Common Issues and Solutions

### Issue: Not Receiving Messages
- Check topic subscription matches message source
- Verify relay is running and socket path is correct
- Check relay domain filtering

### Issue: Message Parse Errors
- Ensure using same protocol version as relay
- Check for message corruption
- Verify endianness handling

### Issue: High Latency
- Check buffer sizes and backpressure
- Monitor network/socket performance
- Consider using performance mode relay config

## Summary

Key principles for relay consumers:
1. **No relay dependencies** - Only depend on protocol library
2. **Topic-based subscription** - Only get relevant messages
3. **Robust reconnection** - Handle network issues gracefully
4. **Domain-specific processing** - Parse only needed message types
5. **Performance monitoring** - Track metrics and health