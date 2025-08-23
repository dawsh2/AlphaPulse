# Message Bus Architecture

## Executive Summary

A shared memory message bus for high-performance inter-process communication between AlphaPulse services. Services communicate via lock-free ring buffers in shared memory, achieving microsecond-scale latency for local IPC while maintaining process isolation and fault tolerance.

## Core Design Principles

1. **Shared Memory IPC**: Lock-free ring buffers for inter-process communication
2. **Process Isolation**: Services run in separate processes for fault isolation
3. **Binary Protocol**: All messages use the TLV format defined in protocol.md
4. **Backpressure Handling**: Producer throttling when consumers can't keep up
5. **Observable**: Built-in metrics and tracing for production monitoring

## Architecture Overview

### Service Communication Model

```
┌─────────────────┐     Shared Memory      ┌─────────────────┐
│ Market Collector│────────────────────────►│ Strategy Engine │
│   (Producer)    │     Ring Buffer         │   (Consumer)    │
└─────────────────┘                         └─────────────────┘
         │                                           │
         │              ┌─────────────┐             │
         └─────────────►│ Analytics   │◄────────────┘
                        │  (Consumer)  │
                        └─────────────┘
```

### Memory Layout

```
Shared Memory Region (per channel):
┌──────────────────────────────────────────┐
│ Control Block (128 bytes, cache-aligned) │
├──────────────────────────────────────────┤
│ Ring Buffer (configurable, typically 1GB)│
│ ┌────────────────────────────────────┐   │
│ │ Message 0: [Header][TLV Payload]   │   │
│ │ Message 1: [Header][TLV Payload]   │   │
│ │ ...                                 │   │
│ └────────────────────────────────────┘   │
└──────────────────────────────────────────┘
```

## Implementation Design

### Control Block Structure

```rust
#[repr(C)]
pub struct ControlBlock {
    // Version and compatibility
    pub magic: u32,                    // 0xALPH0001
    pub version: u16,
    pub flags: u16,
    
    // Ring buffer metadata
    pub buffer_size: u64,              // Total size in bytes
    pub message_size_max: u32,         // Maximum message size
    pub alignment: u32,                // Memory alignment requirement
    
    // Producer state (cache line aligned)
    pub producer_position: AtomicU64,  // Current write position
    pub producer_sequence: AtomicU64,  // Monotonic sequence number
    
    // Consumer states (each cache line aligned)
    pub consumer_count: u32,
    pub consumers: [ConsumerState; MAX_CONSUMERS],
    
    // Statistics
    pub messages_written: AtomicU64,
    pub messages_dropped: AtomicU64,
    pub bytes_written: AtomicU64,
}

#[repr(C)]
pub struct ConsumerState {
    pub consumer_id: u32,
    pub active: AtomicBool,
    pub position: AtomicU64,           // Current read position
    pub last_sequence: AtomicU64,      // Last processed sequence
    pub lag_messages: AtomicU64,       // How far behind producer
}
```

### Message Envelope

```rust
/// Wrapper for messages with precise timing information
#[repr(C)]
pub struct MessageEnvelope {
    pub producer_sent_ns: u64,     // When producer wrote message
    pub bus_received_ns: u64,      // When bus accepted message  
    pub consumer_dequeued_ns: u64, // When consumer read message (set on read)
    pub payload_size: u32,
    pub payload: [u8],             // Flexible array member for TLV payload
}

impl MessageEnvelope {
    /// Calculate end-to-end latency
    pub fn total_latency_ns(&self) -> u64 {
        self.consumer_dequeued_ns.saturating_sub(self.producer_sent_ns)
    }
    
    /// Calculate bus processing latency
    pub fn bus_latency_ns(&self) -> u64 {
        self.bus_received_ns.saturating_sub(self.producer_sent_ns)
    }
    
    /// Calculate queue wait time
    pub fn queue_latency_ns(&self) -> u64 {
        self.consumer_dequeued_ns.saturating_sub(self.bus_received_ns)
    }
}
```

### Ring Buffer Operations

```rust
pub struct RingBuffer {
    control: *mut ControlBlock,
    buffer: *mut u8,
    size: usize,
}

impl RingBuffer {
    /// Get current time in nanoseconds
    fn current_nanos() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
    
    /// Producer: Write message to ring buffer with timestamp
    pub fn write(&self, message: &[u8]) -> Result<u64, BusError> {
        let payload_size = message.len();
        let envelope_size = std::mem::size_of::<MessageEnvelope>() + payload_size;
        
        if payload_size > self.control.message_size_max as usize {
            return Err(BusError::MessageTooLarge);
        }
        
        // Create envelope with timing information
        let envelope = MessageEnvelope {
            producer_sent_ns: current_nanos(),
            bus_received_ns: 0,  // Set when bus processes
            consumer_dequeued_ns: 0,  // Set when consumer reads
            payload_size: payload_size as u32,
            payload: message.to_vec(),  // In practice, would use zero-copy
        };
        
        // Acquire write position atomically
        let position = self.control.producer_position
            .fetch_add(envelope_size as u64, Ordering::AcqRel);
        
        // Mark bus received time
        envelope.bus_received_ns = current_nanos();
        
        // Check for ring wrap
        let offset = (position % self.size as u64) as usize;
        
        // Copy envelope to buffer (handles wrap-around)
        self.copy_envelope_with_wrap(&envelope, offset);
        
        // Update sequence number
        let sequence = self.control.producer_sequence
            .fetch_add(1, Ordering::Release);
        
        Ok(sequence)
    }
    
    /// Consumer: Read next message with timing metadata
    pub fn read(&self, consumer_id: u32) -> Result<(Vec<u8>, MessageEnvelope), BusError> {
        let consumer = &self.control.consumers[consumer_id as usize];
        
        // Get current read position
        let position = consumer.position.load(Ordering::Acquire);
        let producer_pos = self.control.producer_position.load(Ordering::Acquire);
        
        // Check if data available
        if position >= producer_pos {
            return Err(BusError::NoDataAvailable);
        }
        
        // Read envelope first
        let offset = (position % self.size as u64) as usize;
        let mut envelope = self.read_envelope(offset)?;
        
        // Mark consumer dequeue time
        envelope.consumer_dequeued_ns = current_nanos();
        
        // Read payload
        let payload = self.read_payload(offset, envelope.payload_size as usize)?;
        
        // Update consumer position
        let total_size = std::mem::size_of::<MessageEnvelope>() + envelope.payload_size as usize;
        consumer.position.fetch_add(total_size as u64, Ordering::Release);
        consumer.last_sequence.fetch_add(1, Ordering::Release);
        
        // Update lag tracking
        let lag = producer_pos - position - total_size as u64;
        consumer.lag_messages.store(lag / total_size as u64, Ordering::Relaxed);
        
        Ok((payload, envelope))
    }
}
```

### Backpressure Handling

```rust
pub struct BackpressureStrategy {
    pub mode: BackpressureMode,
    pub high_water_mark: f32,  // e.g., 0.8 = 80% full
    pub low_water_mark: f32,   // e.g., 0.6 = 60% full
}

pub enum BackpressureMode {
    /// Drop new messages when buffer is full
    DropNewest,
    
    /// Drop oldest messages to make room
    DropOldest,
    
    /// Block producer until space available
    BlockProducer { timeout_ms: u32 },
    
    /// Dynamically slow producer based on consumer lag
    AdaptiveThrottle {
        min_delay_us: u32,
        max_delay_us: u32,
    },
    
    /// Circuit breaker for cascade failure protection
    CircuitBreaker {
        failure_threshold: u32,      // Number of failures before opening
        recovery_timeout: Duration,  // Time before attempting recovery
        failure_window: Duration,    // Time window for counting failures
    },
}

impl RingBuffer {
    fn check_backpressure(&self) -> BackpressureAction {
        // Calculate buffer utilization
        let slowest_consumer = self.find_slowest_consumer();
        let lag = self.control.producer_position.load(Ordering::Acquire) 
                - slowest_consumer.position.load(Ordering::Acquire);
        let utilization = lag as f32 / self.size as f32;
        
        match self.backpressure.mode {
            BackpressureMode::AdaptiveThrottle { min_delay_us, max_delay_us } => {
                if utilization > self.backpressure.high_water_mark {
                    // Calculate delay based on utilization
                    let delay = min_delay_us + 
                        ((max_delay_us - min_delay_us) as f32 * utilization) as u32;
                    BackpressureAction::Delay(delay)
                } else {
                    BackpressureAction::Proceed
                }
            }
            BackpressureMode::CircuitBreaker { failure_threshold, recovery_timeout, .. } => {
                if self.circuit_breaker.is_open() {
                    BackpressureAction::Reject
                } else if self.circuit_breaker.failure_count >= failure_threshold {
                    self.circuit_breaker.open(recovery_timeout);
                    BackpressureAction::Reject
                } else {
                    BackpressureAction::Proceed
                }
            }
            // ... other modes
        }
    }
}
```

## Channel Configuration

### Channel Types

```rust
pub enum ChannelType {
    /// Single producer, single consumer (fastest)
    SPSC {
        producer: ServiceId,
        consumer: ServiceId,
    },
    
    /// Single producer, multiple consumers
    SPMC {
        producer: ServiceId,
        consumers: Vec<ServiceId>,
    },
    
    /// Multiple producers, single consumer
    MPSC {
        producers: Vec<ServiceId>,
        consumer: ServiceId,
    },
    
    /// Multiple producers, multiple consumers (slowest)
    MPMC {
        producers: Vec<ServiceId>,
        consumers: Vec<ServiceId>,
    },
}

pub struct ChannelConfig {
    pub name: String,
    pub channel_type: ChannelType,
    pub buffer_size: usize,
    pub message_size_max: usize,
    pub backpressure: BackpressureStrategy,
    pub persistence: Option<PersistenceConfig>,
    
    // Memory optimization
    pub huge_pages: bool,           // Use 2MB/1GB huge pages for buffer
    pub numa_node: Option<u8>,      // Pin to specific NUMA node
    pub prefault: bool,             // Pre-fault pages to avoid page faults
}
```

### Standard Channels

```toml
# config/message_bus.toml

[channels.market_data]
type = "SPMC"
producer = "polygon_collector"
consumers = ["arbitrage_strategy", "analytics", "dashboard"]
buffer_size = "1GB"
message_size_max = "8KB"
huge_pages = true               # Use 2MB huge pages for performance
numa_node = 0                   # Pin to NUMA node 0
prefault = true                 # Pre-fault pages to avoid runtime faults
backpressure.mode = "DropOldest"

[channels.signals]
type = "MPSC"
producers = ["arbitrage_strategy", "ml_strategy", "momentum_strategy"]
consumer = "execution_coordinator"
buffer_size = "256MB"
message_size_max = "4KB"
backpressure.mode = "AdaptiveThrottle"
backpressure.min_delay_us = 10
backpressure.max_delay_us = 1000

[channels.execution_results]
type = "SPMC"
producer = "execution_coordinator"
consumers = ["risk_monitor", "analytics", "dashboard"]
buffer_size = "128MB"
message_size_max = "2KB"
huge_pages = true
numa_node = 0
prefault = false                # Less critical channel, save memory
backpressure.mode = "BlockProducer"
backpressure.timeout_ms = 100

[channels.critical_signals]
type = "SPSC"
producer = "flash_arbitrage"
consumer = "execution_engine"
buffer_size = "64MB"
message_size_max = "4KB"
huge_pages = true
numa_node = 0
prefault = true
# Circuit breaker for cascade failure protection
backpressure.mode = "CircuitBreaker"
backpressure.failure_threshold = 5      # Open after 5 failures
backpressure.recovery_timeout_ms = 5000 # Try recovery after 5 seconds
backpressure.failure_window_ms = 1000   # Count failures within 1 second window
```

## Service Integration

### Producer Example

```rust
use message_bus::{MessageBus, TLVMessage};

pub struct MarketDataCollector {
    bus: MessageBus,
    channel: ChannelHandle,
}

impl MarketDataCollector {
    pub async fn process_trade(&self, trade: TradeTLV) -> Result<()> {
        // Build TLV message (from protocol.md)
        let message = TLVMessageBuilder::new(MARKET_DATA_DOMAIN, POLYGON_COLLECTOR)
            .add_tlv(TLVType::Trade, &trade)
            .add_tlv(TLVType::TraceContext, &self.trace_ctx)
            .build();
        
        // Write to message bus
        let sequence = self.bus.write(&self.channel, message.as_bytes())?;
        
        // Update metrics
        self.metrics.messages_sent.increment();
        
        Ok(())
    }
}
```

### Consumer Example

```rust
pub struct ArbitrageStrategy {
    bus: MessageBus,
    channel: ChannelHandle,
    consumer_id: u32,
}

impl ArbitrageStrategy {
    pub async fn run(&mut self) -> Result<()> {
        loop {
            // Read from message bus
            match self.bus.read(&self.channel, self.consumer_id) {
                Ok(data) => {
                    let message = TLVMessage::from_bytes(&data)?;
                    self.process_message(message).await?;
                }
                Err(BusError::NoDataAvailable) => {
                    // No messages, sleep briefly
                    tokio::time::sleep(Duration::from_micros(100)).await;
                }
                Err(e) => {
                    error!("Bus read error: {}", e);
                }
            }
        }
    }
}
```

## Performance Characteristics

### Realistic Latency Targets

| Operation | Target | Conditions |
|-----------|--------|------------|
| SPSC Write | < 1 μs | Same NUMA node, no contention |
| SPSC Read | < 1 μs | Data in cache |
| MPMC Write | < 5 μs | With synchronization overhead |
| MPMC Read | < 5 μs | With consumer coordination |
| End-to-end | < 10 μs | Producer write → Consumer process |

### Throughput Targets

| Channel Type | Messages/sec | Bandwidth | Conditions |
|--------------|--------------|-----------|------------|
| SPSC | 10M | 10 GB/s | 1KB messages, single core |
| SPMC | 5M | 5 GB/s | 2 consumers |
| MPSC | 3M | 3 GB/s | 3 producers |
| MPMC | 1M | 1 GB/s | Multiple producers/consumers |

## Monitoring and Observability

### Built-in Metrics

```rust
pub struct ChannelMetrics {
    // Throughput metrics
    pub messages_written: Counter,
    pub messages_read: HashMap<ConsumerId, Counter>,
    pub bytes_written: Counter,
    pub bytes_read: HashMap<ConsumerId, Counter>,
    
    // Latency metrics
    pub write_latency_ns: Histogram,
    pub read_latency_ns: HashMap<ConsumerId, Histogram>,
    pub end_to_end_latency_ns: Histogram,
    
    // Buffer metrics
    pub buffer_utilization: Gauge,
    pub consumer_lag: HashMap<ConsumerId, Gauge>,
    pub backpressure_events: Counter,
    pub messages_dropped: Counter,
    
    // Error metrics
    pub write_errors: Counter,
    pub read_errors: HashMap<ConsumerId, Counter>,
}
```

### Health Monitoring

```rust
impl MessageBus {
    pub fn health_check(&self) -> HealthStatus {
        let mut status = HealthStatus::Healthy;
        
        for (name, channel) in &self.channels {
            // Check buffer utilization
            if channel.utilization() > 0.9 {
                status = HealthStatus::Degraded;
                warn!("Channel {} buffer >90% full", name);
            }
            
            // Check consumer lag
            for consumer in channel.consumers() {
                if consumer.lag_messages() > 10000 {
                    status = HealthStatus::Degraded;
                    warn!("Consumer {} lagging by {} messages", 
                          consumer.id, consumer.lag_messages());
                }
            }
            
            // Check for stalled consumers
            if consumer.seconds_since_last_read() > 60 {
                status = HealthStatus::Unhealthy;
                error!("Consumer {} appears stalled", consumer.id);
            }
        }
        
        status
    }
}
```

## Fault Tolerance

### Consumer Failure Handling

- Automatic detection of stalled/crashed consumers
- Configurable timeout before marking consumer as failed
- Optional automatic consumer restart
- Lag monitoring and alerting

### Producer Failure Handling

- Multiple producer support for critical channels
- Automatic failover to backup producer
- Sequence number gaps indicate lost messages

### Recovery Mechanisms

```rust
pub struct RecoveryConfig {
    /// Persist messages to disk for recovery
    pub enable_persistence: bool,
    pub persistence_path: PathBuf,
    
    /// Replay messages on consumer restart
    pub enable_replay: bool,
    pub replay_window: Duration,
    
    /// Automatic consumer restart
    pub auto_restart_consumers: bool,
    pub restart_delay: Duration,
}
```

## Future Extensions

### Planned Enhancements

1. **Network Transport**: TCP/UDP transport for distributed deployments
2. **Compression**: Optional message compression for bandwidth optimization
3. **Encryption**: TLS for network transport, optional for shared memory
4. **Topic Routing**: Pub/sub topic filtering within channels
5. **Priority Queues**: Multiple priority levels within a channel

### Protocol Compatibility

The message bus is designed to transport TLV messages as defined in protocol.md without modification. Any future protocol changes will be transparent to the bus layer.

## Summary

This message bus provides:
- **Realistic performance**: Microsecond latency, millions of messages/second
- **Production ready**: Backpressure, monitoring, fault tolerance
- **Clean integration**: Works with existing TLV protocol
- **Extensible**: Clear path for distributed deployment

The design prioritizes simplicity, reliability, and realistic performance targets suitable for production deployment.