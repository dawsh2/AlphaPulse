# AlphaPulse V2: Detailed Architecture Guide

## Table of Contents
1. [Protocol V2 Specification](#protocol-v2-specification)
2. [Component Deep Dives](#component-deep-dives)
3. [Message Type Registry](#message-type-registry)
4. [Performance Architecture](#performance-architecture)
5. [Fault Tolerance & Recovery](#fault-tolerance--recovery)
6. [Testing Strategy](#testing-strategy)

## Protocol V2 Specification

### Message Header Structure (32 bytes)

```rust
#[repr(C)]
pub struct MessageHeader {
    magic: u32,           // 0xDEADBEEF - identifies AlphaPulse messages
    version: u8,          // Protocol version (currently 1)
    relay_domain: u8,     // Target relay domain
    source: u16,          // Source service identifier
    
    sequence: u64,        // Monotonic sequence number per source
    timestamp: u64,       // Nanosecond Unix timestamp
    
    payload_size: u32,    // Size of TLV payload in bytes
    checksum: u32,        // CRC32 of entire message
}
```

### TLV Extension Format

#### Standard TLV (≤255 bytes)
```
┌─────────────┬─────────────┬─────────────────────┐
│ Type (1B)   │ Length (1B) │ Value (0-255B)      │
└─────────────┴─────────────┴─────────────────────┘
```

#### Extended TLV (>255 bytes)
```
┌─────┬─────┬─────┬──────────────┬─────────────────┐
│ 255 │ 0   │Type │ Length (2B)  │ Value (0-64KB)  │
└─────┴─────┴─────┴──────────────┴─────────────────┘
```

### TLV Type Domains

| Domain | Type Range | Purpose | Example Types |
|--------|------------|---------|---------------|
| Market Data | 1-19 | Real-time market events | Trade, Quote, OrderBook |
| Signals | 20-39 | Trading signals & indicators | ArbitrageSignal, Momentum |
| Execution | 40-79 | Order & execution messages | OrderRequest, Fill, Cancel |
| System | 80-99 | Infrastructure & monitoring | Heartbeat, StatusReport |
| Reserved | 100-254 | Future expansion | - |
| Extended | 255 | Extended format marker | - |

## Component Deep Dives

### 1. Exchange Adapters (`services_v2/adapters/`)

Each adapter follows a standardized pattern:

```rust
pub struct ExchangeAdapter {
    // Connection management
    websocket: WebSocketClient,
    
    // Message building
    message_builder: TLVMessageBuilder,
    
    // Relay connection
    relay_socket: UnixSocket,
    
    // Metrics
    metrics: AdapterMetrics,
}

impl ExchangeAdapter {
    pub async fn run(&mut self) {
        loop {
            // 1. Receive exchange data
            let raw_data = self.websocket.receive().await?;
            
            // 2. Parse and validate
            let event = self.parse_exchange_format(raw_data)?;
            
            // 3. Convert to TLV
            let tlv_message = self.build_tlv_message(event)?;
            
            // 4. Send to relay
            self.relay_socket.send(tlv_message).await?;
            
            // 5. Update metrics
            self.metrics.record_message();
        }
    }
}
```

#### Adapter Responsibilities
- **Connection Management**: WebSocket/REST connections with reconnection logic
- **Data Normalization**: Convert exchange-specific formats to TLV
- **Precision Preservation**: Maintain native token precision
- **Error Recovery**: Handle connection drops, invalid data
- **Metrics Collection**: Latency, throughput, error rates

### 2. Relay Architecture (`relays/`)

Relays are high-performance message routers:

```rust
pub struct GenericRelay<T: RelayDomain> {
    // Message reception
    server_socket: UnixSocketServer,
    
    // Subscriber management
    subscribers: HashMap<SubscriberId, Sender>,
    
    // Message validation
    validator: MessageValidator<T>,
    
    // Performance monitoring
    metrics: RelayMetrics,
}
```

#### Relay Characteristics
- **Domain Separation**: Each relay handles specific TLV type ranges
- **Zero-Copy Broadcasting**: Messages forwarded without parsing payload
- **Subscriber Management**: Dynamic subscription/unsubscription
- **Back-pressure Handling**: Slow consumers don't block fast ones
- **Metrics & Monitoring**: Message rates, latency distribution

### 3. Trading Strategies (`services_v2/strategies/`)

Strategies implement the core trading logic:

```rust
pub trait TradingStrategy {
    type Signal;
    
    // Process market data
    fn on_market_data(&mut self, data: TradeTLV) -> Option<Self::Signal>;
    
    // Risk management
    fn check_risk_limits(&self, signal: &Self::Signal) -> bool;
    
    // Generate execution instructions
    fn to_order_request(&self, signal: Self::Signal) -> OrderRequestTLV;
}
```

#### Strategy Components
- **Signal Generation**: Identify trading opportunities
- **Risk Management**: Position limits, exposure controls
- **Execution Planning**: Order sizing, venue selection
- **Performance Tracking**: P&L, win rate, slippage

## Message Type Registry

### Market Data Domain (1-19)

| Type | Name | Size | Description |
|------|------|------|-------------|
| 1 | TradeTLV | 40B | Executed trade event |
| 2 | QuoteTLV | 52B | Best bid/ask update |
| 3 | OrderBookTLV | Variable | Full orderbook snapshot |
| 4 | PoolSwapTLV | 60-200B | DEX swap event |
| 5 | PoolMintTLV | 50-180B | Liquidity addition |
| 6 | PoolBurnTLV | 50-180B | Liquidity removal |

### Signal Domain (20-39)

| Type | Name | Size | Description |
|------|------|------|-------------|
| 20 | SignalIdentity | 16B | Signal metadata |
| 21 | ArbitrageSignal | Variable | Arbitrage opportunity |
| 22 | MomentumSignal | 32B | Momentum indicator |
| 23 | LiquidationAlert | 48B | Liquidation risk warning |

### Execution Domain (40-79)

| Type | Name | Size | Description |
|------|------|------|-------------|
| 40 | OrderRequest | 60-100B | New order submission |
| 41 | OrderAck | 32B | Order acknowledgment |
| 42 | Fill | 56B | Execution report |
| 43 | Cancel | 24B | Order cancellation |
| 44 | GasPrice | 32B | Gas price update |

## Performance Architecture

### Zero-Copy Design
```rust
// Messages parsed in-place
let header = zerocopy::Ref::<_, MessageHeader>::new(&buffer)?;
let tlv_data = &buffer[32..32 + header.payload_size];
```

### Memory Layout Optimization
- **Aligned Structures**: All TLV types are properly aligned
- **Cache-Friendly**: Hot data fits in L1/L2 cache
- **Minimal Allocations**: Pre-allocated buffers reused

### Concurrency Model
- **Actor Pattern**: Each service runs independently
- **Message Passing**: No shared mutable state
- **Lock-Free Queues**: SPSC/MPSC channels for communication
- **Thread Affinity**: Services pinned to CPU cores

### Benchmarking Results

| Operation | Throughput | Latency (p50) | Latency (p99) |
|-----------|------------|---------------|---------------|
| Message Build | 1.09M/s | 0.9μs | 1.2μs |
| Message Parse | 1.64M/s | 0.6μs | 0.8μs |
| Relay Forward | 2.1M/s | 0.4μs | 0.7μs |
| E2E Hot Path | 850K/s | 32μs | 48μs |

## Fault Tolerance & Recovery

### Service Isolation
- **Process Boundaries**: Services run as separate processes
- **Failure Isolation**: One service crash doesn't affect others
- **Automatic Restart**: Systemd/supervisor monitors and restarts

### Message Reliability
- **Sequence Numbers**: Detect gaps and request retransmission
- **Checksums**: Detect corruption during transport
- **Buffering**: Temporary storage during relay unavailability

### State Recovery
```rust
pub struct RecoveryManager {
    // Snapshot current state
    fn snapshot(&self) -> StateSnapshot;
    
    // Restore from snapshot
    fn restore(snapshot: StateSnapshot) -> Self;
    
    // Replay messages from sequence
    fn replay_from(sequence: u64);
}
```

### Circuit Breakers
- **Rate Limiting**: Prevent message floods
- **Error Thresholds**: Disable misbehaving components
- **Gradual Recovery**: Slowly increase load after issues

## Testing Strategy

### Unit Testing
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_tlv_serialization() {
        let trade = TradeTLV { /* ... */ };
        let bytes = trade.as_bytes();
        let decoded = TradeTLV::from_bytes(bytes).unwrap();
        assert_eq!(trade, decoded);
    }
}
```

### Integration Testing
- **Protocol Tests**: Round-trip message serialization
- **Relay Tests**: Multi-subscriber broadcasting
- **Adapter Tests**: Mock exchange connections

### End-to-End Testing
```bash
# Replay historical data through system
cargo test --package tests --test replay_historical

# Stress testing with synthetic load
cargo test --package tests --test stress_test
```

### Property-Based Testing
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_message_roundtrip(
        price in 0i64..=i64::MAX,
        volume in 0i64..=i64::MAX
    ) {
        let trade = TradeTLV { price, volume, /* ... */ };
        let bytes = serialize(trade);
        let decoded = deserialize(bytes);
        prop_assert_eq!(trade, decoded);
    }
}
```

## Network Topology

### Local Deployment
```
┌──────────────────────────────────────────────┐
│                Host Machine                  │
├──────────────────────────────────────────────┤
│  Adapters  │  Relays  │  Strategies  │  API  │
├──────────────────────────────────────────────┤
│         Unix Domain Sockets (IPC)            │
└──────────────────────────────────────────────┘
```

### Distributed Deployment
```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Adapter    │     │   Relay     │     │  Strategy   │
│   Node 1    │────▶│   Node 2    │────▶│   Node 3    │
└─────────────┘     └─────────────┘     └─────────────┘
       TCP/TLS            TCP/TLS            TCP/TLS
```

## Security Architecture

### Network Security
- **Unix Sockets**: Local-only communication
- **TLS 1.3**: Encrypted external connections
- **Authentication**: Service-to-service auth tokens

### Data Integrity
- **Checksums**: Detect tampering/corruption
- **Sequence Numbers**: Prevent replay attacks
- **Timestamps**: Detect stale messages

### Operational Security
- **Least Privilege**: Services run with minimal permissions
- **Secret Management**: Environment variables, not in code
- **Audit Logging**: All trades and signals logged

## Monitoring & Observability

### Metrics Collection
```rust
pub struct ServiceMetrics {
    messages_processed: Counter,
    processing_latency: Histogram,
    errors: Counter,
    active_connections: Gauge,
}
```

### Log Aggregation
- **Structured Logging**: JSON format for parsing
- **Log Levels**: DEBUG, INFO, WARN, ERROR
- **Correlation IDs**: Trace requests across services

### Health Checks
```rust
#[derive(Serialize)]
pub struct HealthStatus {
    status: ServiceStatus,
    uptime: Duration,
    message_rate: f64,
    error_rate: f64,
    dependencies: Vec<DependencyHealth>,
}
```

## Configuration Management

### Environment-Based Config
```yaml
# config/production.yaml
market_data_relay:
  socket_path: /var/run/alphapulse/market_data.sock
  buffer_size: 65536
  max_subscribers: 1000

adapters:
  polygon:
    rpc_url: ${POLYGON_RPC_URL}
    ws_url: wss://polygon-mainnet.g.alchemy.com/v2/${API_KEY}
```

### Feature Flags
```rust
#[cfg(feature = "replay-mode")]
fn use_replay_data() { /* ... */ }

#[cfg(not(feature = "replay-mode"))]  
fn use_live_data() { /* ... */ }
```

## Development Best Practices

### Code Organization
- **Single Responsibility**: Each module has one clear purpose
- **Dependency Injection**: Services receive dependencies
- **Error Handling**: Result types, no panics in production
- **Documentation**: Every public API documented

### Performance Guidelines
- **Measure First**: Profile before optimizing
- **Avoid Allocations**: Reuse buffers in hot paths
- **Batch Operations**: Process multiple items together
- **Cache Wisely**: Only cache frequently accessed data

### Testing Requirements
- **Unit Test Coverage**: >80% for critical paths
- **Integration Tests**: All service boundaries
- **Performance Tests**: Detect regression
- **Fuzzing**: Protocol parsing robustness

---

*For implementation details, see service-specific README files in their respective directories.*