# MYC-003: Broker Layer Implementation

## Status
- **Status**: pending
- **Assignee**: TBD
- **Estimated Effort**: 3 days
- **Priority**: High (core broker functionality)

## Description
Implement the core broker layer with topic-based message routing, TOML configuration system, and event loop management. This replaces the domain-specific relay architecture with a flexible, configurable broker that supports both fanout and queue routing patterns.

## Objectives
1. Implement Broker trait with topic-based publish/subscribe
2. Create TOML-based configuration system for routing rules
3. Implement fanout (broadcast) and queue (load-balanced) routing patterns  
4. Build main event loop with connection and subscription management
5. Ensure broker maintains >1M msg/s throughput performance

## Technical Approach

### Core Broker Implementation
```rust
// mycelium-broker/src/broker.rs
use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc};

pub struct MyceliumBroker {
    config: BrokerConfig,
    topics: HashMap<String, TopicHandler>,
    connections: ConnectionRegistry,
    metrics: BrokerMetrics,
}

#[async_trait]
impl Broker for MyceliumBroker {
    type Error = BrokerError;

    async fn publish(&self, topic: &str, data: &[u8]) -> Result<(), Self::Error> {
        let handler = self.topics.get(topic)
            .ok_or(BrokerError::TopicNotFound(topic.to_string()))?;
        
        // Increment metrics
        self.metrics.messages_published.fetch_add(1, Ordering::Relaxed);
        
        // Route based on topic configuration
        match handler.routing_type {
            RoutingType::Fanout => {
                handler.fanout_channel.send(data.to_vec())?;
            }
            RoutingType::Queue => {
                handler.queue_channel.send(data.to_vec()).await?;
            }
        }
        
        Ok(())
    }

    async fn subscribe(&self, connection_id: ConnectionId, topic: &str) -> Result<(), Self::Error> {
        let handler = self.topics.get(topic)
            .ok_or(BrokerError::TopicNotFound(topic.to_string()))?;
        
        self.connections.add_subscription(connection_id, topic, handler.clone())?;
        
        tracing::info!("Connection {} subscribed to topic '{}'", connection_id, topic);
        Ok(())
    }

    async fn unsubscribe(&self, connection_id: ConnectionId, topic: &str) -> Result<(), Self::Error> {
        self.connections.remove_subscription(connection_id, topic)?;
        
        tracing::info!("Connection {} unsubscribed from topic '{}'", connection_id, topic);
        Ok(())
    }
}
```

### Topic Management
```rust
// mycelium-broker/src/topic.rs
#[derive(Debug, Clone)]
pub struct TopicHandler {
    pub name: String,
    pub routing_type: RoutingType,
    pub fanout_channel: broadcast::Sender<Vec<u8>>,
    pub queue_channel: mpsc::Sender<Vec<u8>>,
    pub subscribers: Arc<RwLock<HashSet<ConnectionId>>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub enum RoutingType {
    /// Broadcast message to all subscribers
    Fanout,
    /// Load-balance message to one subscriber
    Queue,
}

impl TopicHandler {
    pub fn new(name: String, routing_type: RoutingType, capacity: usize) -> Self {
        let (fanout_tx, _) = broadcast::channel(capacity);
        let (queue_tx, queue_rx) = mpsc::channel(capacity);
        
        Self {
            name,
            routing_type,
            fanout_channel: fanout_tx,
            queue_channel: queue_tx,
            subscribers: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub async fn add_subscriber(&self, connection_id: ConnectionId) -> broadcast::Receiver<Vec<u8>> {
        self.subscribers.write().await.insert(connection_id);
        self.fanout_channel.subscribe()
    }

    pub async fn remove_subscriber(&self, connection_id: ConnectionId) {
        self.subscribers.write().await.remove(&connection_id);
    }
}
```

### Configuration System
```rust
// mycelium-config/src/config.rs
#[derive(Debug, Clone, serde::Deserialize)]
pub struct BrokerConfig {
    pub server: ServerConfig,
    pub topics: Vec<TopicConfig>,
    pub performance: PerformanceConfig,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,
    pub socket_path: String,
    pub max_connections: usize,
    pub connection_timeout_ms: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TopicConfig {
    pub name: String,
    pub routing: RoutingType,
    pub buffer_size: usize,
    pub max_message_size: usize,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PerformanceConfig {
    pub worker_threads: usize,
    pub channel_capacity: usize,
    pub batch_size: usize,
    pub flush_interval_ms: u64,
}

impl BrokerConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: BrokerConfig = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.server.max_connections == 0 {
            return Err(ConfigError::InvalidValue("max_connections must be > 0".into()));
        }
        
        if self.performance.worker_threads == 0 {
            return Err(ConfigError::InvalidValue("worker_threads must be > 0".into()));
        }
        
        // Validate topic names are unique
        let mut seen = HashSet::new();
        for topic in &self.topics {
            if !seen.insert(&topic.name) {
                return Err(ConfigError::DuplicateTopic(topic.name.clone()));
            }
        }
        
        Ok(())
    }
}
```

### Event Loop Implementation
```rust
// mycelium-broker/src/event_loop.rs
pub struct BrokerEventLoop {
    broker: Arc<MyceliumBroker>,
    connection_manager: ConnectionManager,
    shutdown_rx: mpsc::Receiver<()>,
}

impl BrokerEventLoop {
    pub async fn run(mut self) -> Result<(), BrokerError> {
        tracing::info!("Starting broker event loop");
        
        // Start connection acceptor
        let acceptor_handle = {
            let broker = Arc::clone(&self.broker);
            let mut conn_manager = self.connection_manager.clone();
            tokio::spawn(async move {
                Self::accept_connections(broker, conn_manager).await
            })
        };
        
        // Start message processors for each topic
        let mut processor_handles = Vec::new();
        for (topic_name, handler) in &self.broker.topics {
            let handle = Self::spawn_message_processor(
                topic_name.clone(),
                handler.clone(),
                self.connection_manager.clone()
            );
            processor_handles.push(handle);
        }
        
        // Wait for shutdown signal
        loop {
            tokio::select! {
                _ = self.shutdown_rx.recv() => {
                    tracing::info!("Received shutdown signal");
                    break;
                }
                _ = acceptor_handle => {
                    tracing::error!("Connection acceptor terminated unexpectedly");
                    break;
                }
            }
        }
        
        // Graceful shutdown
        acceptor_handle.abort();
        for handle in processor_handles {
            handle.abort();
        }
        
        tracing::info!("Broker event loop terminated");
        Ok(())
    }

    async fn accept_connections(
        broker: Arc<MyceliumBroker>,
        mut conn_manager: ConnectionManager
    ) -> Result<(), BrokerError> {
        while let Ok(transport) = conn_manager.accept().await {
            let broker_clone = Arc::clone(&broker);
            tokio::spawn(async move {
                Self::handle_connection(broker_clone, transport).await;
            });
        }
        Ok(())
    }

    async fn handle_connection(
        broker: Arc<MyceliumBroker>,
        transport: Box<dyn Transport>
    ) {
        let connection_id = ConnectionId::new();
        tracing::info!("New connection: {}", connection_id);
        
        loop {
            match transport.recv().await {
                Ok(data) => {
                    if let Err(e) = Self::process_message(
                        &broker, 
                        connection_id, 
                        &data,
                        &transport
                    ).await {
                        tracing::warn!("Message processing error: {}", e);
                    }
                }
                Err(TransportError::ConnectionClosed) => {
                    tracing::info!("Connection {} closed", connection_id);
                    break;
                }
                Err(e) => {
                    tracing::error!("Transport error on connection {}: {}", connection_id, e);
                    break;
                }
            }
        }
        
        broker.connections.remove_connection(connection_id).await;
    }

    async fn process_message(
        broker: &MyceliumBroker,
        connection_id: ConnectionId,
        data: &[u8],
        transport: &Box<dyn Transport>
    ) -> Result<(), BrokerError> {
        // Parse broker control message
        let message = BrokerMessage::parse(data)?;
        
        match message {
            BrokerMessage::Publish { topic, payload } => {
                broker.publish(&topic, &payload).await?;
            }
            BrokerMessage::Subscribe { topic } => {
                broker.subscribe(connection_id, &topic).await?;
                
                // Send acknowledgment
                let ack = BrokerMessage::SubscribeAck { topic };
                transport.send(&ack.serialize()).await?;
            }
            BrokerMessage::Unsubscribe { topic } => {
                broker.unsubscribe(connection_id, &topic).await?;
                
                // Send acknowledgment  
                let ack = BrokerMessage::UnsubscribeAck { topic };
                transport.send(&ack.serialize()).await?;
            }
        }
        
        Ok(())
    }
}
```

### Example Configuration
```toml
# broker.toml
[server]
bind_address = "127.0.0.1"
socket_path = "/tmp/mycelium.sock"
max_connections = 1000
connection_timeout_ms = 30000

[[topics]]
name = "market_data"
routing = "Fanout"           # Broadcast to all subscribers
buffer_size = 1000000        # 1M message buffer
max_message_size = 65536     # 64KB max message

[[topics]]
name = "signals"
routing = "Fanout"
buffer_size = 100000
max_message_size = 32768

[[topics]]
name = "execution"
routing = "Queue"            # Load balance across subscribers
buffer_size = 50000
max_message_size = 16384

[performance]
worker_threads = 8
channel_capacity = 100000
batch_size = 1000
flush_interval_ms = 1
```

## Acceptance Criteria

### Core Functionality
- [ ] Broker implements publish/subscribe with topic routing
- [ ] TOML configuration loads and validates correctly
- [ ] Fanout routing broadcasts to all subscribers
- [ ] Queue routing load-balances across subscribers

### Performance Requirements
- [ ] Broker maintains >1M msg/s throughput under load
- [ ] Message routing adds <5μs latency overhead
- [ ] Memory usage scales linearly with subscriber count
- [ ] Configuration hot-reload without service restart

### Reliability Features
- [ ] Graceful connection failure handling
- [ ] Topic subscription/unsubscription works reliably
- [ ] Broker survives subscriber disconnections
- [ ] Message ordering preserved within topics

### Integration Points
- [ ] Compatible with transport layer from MYC-002
- [ ] Configuration system supports environment variable overrides
- [ ] Metrics integrate with existing monitoring infrastructure
- [ ] Error handling propagates to service layers

## Dependencies
- **Upstream**: MYC-001 (Platform Foundation), MYC-002 (Transport Layer)
- **Downstream**: MYC-005 (Producer Migration), MYC-006 (Consumer Migration)
- **External**: tokio for async runtime, serde/toml for configuration

## Testing Requirements

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn broker_fanout_routing() {
        let config = BrokerConfig::default();
        let broker = MyceliumBroker::new(config).await.unwrap();
        
        // Create topic with fanout routing
        broker.create_topic("test_fanout", RoutingType::Fanout).await.unwrap();
        
        // Subscribe two connections
        let conn1 = ConnectionId::new();
        let conn2 = ConnectionId::new();
        broker.subscribe(conn1, "test_fanout").await.unwrap();
        broker.subscribe(conn2, "test_fanout").await.unwrap();
        
        // Publish message
        let message = b"fanout test";
        broker.publish("test_fanout", message).await.unwrap();
        
        // Both connections should receive the message
        // (Test implementation depends on connection mock)
    }

    #[tokio::test]
    async fn broker_queue_routing() {
        let config = BrokerConfig::default();
        let broker = MyceliumBroker::new(config).await.unwrap();
        
        // Create topic with queue routing
        broker.create_topic("test_queue", RoutingType::Queue).await.unwrap();
        
        // Subscribe two connections
        let conn1 = ConnectionId::new();
        let conn2 = ConnectionId::new();
        broker.subscribe(conn1, "test_queue").await.unwrap();
        broker.subscribe(conn2, "test_queue").await.unwrap();
        
        // Publish multiple messages
        for i in 0..100 {
            let message = format!("queue message {}", i);
            broker.publish("test_queue", message.as_bytes()).await.unwrap();
        }
        
        // Messages should be distributed between connections
        // (Load balancing verification)
    }

    #[test]
    fn config_validation() {
        let config_toml = r#"
        [server]
        bind_address = "127.0.0.1"
        socket_path = "/tmp/test.sock"
        max_connections = 100
        connection_timeout_ms = 5000

        [[topics]]
        name = "valid_topic"
        routing = "Fanout"
        buffer_size = 1000
        max_message_size = 1024

        [performance]
        worker_threads = 4
        channel_capacity = 1000
        batch_size = 100
        flush_interval_ms = 1
        "#;
        
        let config: BrokerConfig = toml::from_str(config_toml).unwrap();
        assert!(config.validate().is_ok());
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
    async fn throughput_benchmark() {
        let config = BrokerConfig::high_performance();
        let broker = MyceliumBroker::new(config).await.unwrap();
        
        broker.create_topic("perf_test", RoutingType::Fanout).await.unwrap();
        
        let num_messages = 1_000_000;
        let message = vec![0u8; 1024]; // 1KB message
        
        let start = std::time::Instant::now();
        
        for _ in 0..num_messages {
            broker.publish("perf_test", &message).await.unwrap();
        }
        
        let elapsed = start.elapsed();
        let msg_per_sec = num_messages as f64 / elapsed.as_secs_f64();
        
        println!("Broker throughput: {:.0} msg/s", msg_per_sec);
        assert!(msg_per_sec > 1_000_000.0);
    }

    #[tokio::test]
    #[ignore]
    async fn latency_benchmark() {
        let broker = create_test_broker().await;
        
        let mut latencies = Vec::new();
        let message = b"latency test";
        
        for _ in 0..10000 {
            let start = std::time::Instant::now();
            broker.publish("test_topic", message).await.unwrap();
            latencies.push(start.elapsed().as_micros());
        }
        
        latencies.sort();
        let p99 = latencies[latencies.len() * 99 / 100];
        
        println!("P99 publish latency: {}μs", p99);
        assert!(p99 < 10); // <10μs P99 latency
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn end_to_end_message_flow() {
    // Start broker
    let config = BrokerConfig::test_config();
    let broker = MyceliumBroker::new(config.clone()).await.unwrap();
    let event_loop = BrokerEventLoop::new(broker);
    
    let broker_handle = tokio::spawn(async move {
        event_loop.run().await
    });
    
    // Connect producer and consumer
    let producer_transport = UnixSocketTransport::connect(&config.server.socket_path).await.unwrap();
    let consumer_transport = UnixSocketTransport::connect(&config.server.socket_path).await.unwrap();
    
    // Consumer subscribes to topic
    let subscribe_msg = BrokerMessage::Subscribe { topic: "test".to_string() };
    consumer_transport.send(&subscribe_msg.serialize()).await.unwrap();
    
    // Producer publishes message
    let test_data = b"integration test message";
    let publish_msg = BrokerMessage::Publish {
        topic: "test".to_string(),
        payload: test_data.to_vec(),
    };
    producer_transport.send(&publish_msg.serialize()).await.unwrap();
    
    // Consumer receives message
    let received = consumer_transport.recv().await.unwrap();
    let received_msg = BrokerMessage::parse(&received).unwrap();
    
    if let BrokerMessage::Deliver { payload, .. } = received_msg {
        assert_eq!(payload, test_data);
    } else {
        panic!("Expected Deliver message");
    }
}
```

## Rollback Plan

### If Performance Issues
1. Simplify routing logic to direct message passing
2. Remove batching/buffering if it adds too much latency
3. Use lock-free data structures for hot paths

### If Configuration Complexity
1. Hard-code topic configurations instead of TOML
2. Remove hot-reload functionality  
3. Use environment variables for simple configuration

### If Reliability Issues
1. Add more conservative error handling and retries
2. Implement circuit breaker patterns for failing connections
3. Add explicit connection health checks

## Technical Notes

### Design Decisions
- **Topic-Based Routing**: More flexible than domain-specific relays
- **TOML Configuration**: Human-readable and supports comments
- **Async Event Loop**: Handles high concurrency efficiently
- **Separate Fanout/Queue Channels**: Optimized for different routing patterns

### Performance Optimizations
- **Channel Pre-allocation**: Avoids runtime allocation overhead
- **Batch Processing**: Groups multiple messages for efficiency
- **Zero-Copy Message Routing**: Avoids unnecessary data copying
- **Connection Pooling**: Reduces setup/teardown overhead

### Reliability Features
- **Graceful Shutdown**: Ensures no message loss on termination
- **Connection Health Monitoring**: Detects and handles failed connections
- **Topic Validation**: Prevents invalid routing configurations
- **Backpressure Handling**: Prevents memory exhaustion under high load

## Validation Steps

1. **Configuration Validation**:
   ```bash
   cargo test --package mycelium-config
   ```

2. **Broker Functionality**:
   ```bash
   cargo test --package mycelium-broker
   ```

3. **Performance Testing**:
   ```bash
   cargo test --package mycelium-broker --release -- --ignored
   ```

4. **Integration Validation**:
   ```bash
   cargo test --workspace integration
   ```

This broker layer provides the flexible, high-performance message routing needed to replace the domain-specific relay architecture while maintaining Torq's strict performance and reliability requirements.