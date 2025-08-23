# AlphaPulse Transport System

High-performance network transport system for actor communication across nodes in the AlphaPulse trading infrastructure.

## Features

- **Direct Transport**: TCP/UDP/QUIC for ultra-low latency communication
- **Message Queues**: RabbitMQ/Kafka/Redis integration for reliability-critical channels  
- **Hybrid Routing**: Automatic selection between direct and MQ transport based on requirements
- **Topology Integration**: Seamless integration with AlphaPulse topology system
- **Security**: TLS and ChaCha20Poly1305 encryption support
- **Compression**: LZ4, Zstd, and Snappy compression options
- **Monitoring**: Comprehensive metrics and health monitoring

## Performance Targets

- **TCP Direct**: <5ms latency for inter-node communication
- **UDP Direct**: <1ms latency for trading signals  
- **Shared Memory**: <35μs for same-node communication (via topology integration)
- **Throughput**: >10,000 messages/second per connection

## Quick Start

```rust
use alphapulse_transport::{NetworkTransport, NetworkConfig, ProtocolType, CompressionType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create ultra-low latency configuration
    let config = NetworkConfig::ultra_low_latency();
    
    // Initialize transport
    let mut transport = NetworkTransport::new(config).await?;
    transport.start().await?;
    
    // Send message to remote actor
    let message = b"arbitrage_signal_data";
    transport.send_to_actor("execution_node", "order_executor", message).await?;
    
    Ok(())
}
```

## Architecture

```text
Actor A ─┬─ SharedMemory ──┬─ Actor B (same node, <35μs)
         │                 │
         └─ TCP/UDP ──────┬─ Actor C (different node, <5ms)
         │                │
         └─ MessageQueue ─┴─ Actor D (reliable delivery, >20ms)
```

## Transport Selection

Transport selection is automatic based on:
- **Actor placement** (same node vs different nodes)
- **Channel criticality** (latency vs reliability requirements)  
- **Network topology** (same datacenter vs cross-region)
- **Security requirements** (encrypted vs plain)

### Selection Examples

```yaml
# Ultra-low latency trading signals
arbitrage_signals:
  mode: direct
  protocol: udp
  compression: none
  encryption: none
  # Result: <1ms latency, best effort delivery

# High-throughput market data  
market_data:
  mode: direct
  protocol: tcp
  compression: lz4
  encryption: none
  # Result: <5ms latency, at-least-once delivery

# Reliable audit trail
audit_trail:
  mode: message_queue
  backend: rabbitmq
  compression: zstd
  encryption: tls
  # Result: >20ms latency, guaranteed delivery
```

## Configuration

### Direct TCP (Low Latency)

```yaml
# examples/direct_tcp.yaml
default_mode: direct

channels:
  arbitrage_signals:
    mode: direct
    criticality: ultra_low_latency
    max_message_size: 8192
    timeout: "100ms"
    retry:
      max_attempts: 1
```

### Hybrid (Best of Both)

```yaml  
# examples/hybrid_mq.yaml
default_mode: auto

channels:
  arbitrage_signals:
    mode: direct                    # Always direct for signals
  market_data:
    mode: direct_with_mq_fallback   # Direct with MQ backup
  audit_trail:
    mode: message_queue             # Always MQ for compliance
```

## Integration with Topology

The transport system integrates seamlessly with the AlphaPulse topology system:

```rust
use alphapulse_transport::topology_integration::TopologyIntegrationBuilder;

let integration = TopologyIntegrationBuilder::new()
    .load_topology_config("topology.yaml").await?
    .load_transport_config("transport.yaml").await?
    .build().await?;

// Automatic transport selection based on actor placement
let transport = integration.resolve_transport(
    "polygon_collector",    // Source actor
    "flash_arbitrage",      // Target actor  
    "market_data"          // Channel
).await?;
```

## Monitoring

### Built-in Metrics

- Message latency (p50, p95, p99)
- Throughput (messages/second, bytes/second)
- Error rates by category
- Connection pool health
- Circuit breaker states

### Health Checks

```rust
let health = transport.health_status();
println!("Direct transport healthy: {}", health.direct_healthy);
println!("MQ transport healthy: {}", health.mq_healthy);
println!("Overall health score: {:.2}", health.health_score());
```

## Benchmarks

Run performance benchmarks:

```bash
cargo bench --features "compression,encryption"
```

Key benchmark categories:
- **Serialization**: Message envelope encode/decode
- **Compression**: LZ4/Zstd/Snappy performance
- **Encryption**: ChaCha20Poly1305 throughput
- **End-to-End**: Complete message processing pipeline
- **Throughput**: Batch message processing

## Features

Enable optional features in `Cargo.toml`:

```toml
[dependencies]
alphapulse-transport = { 
    version = "0.1.0", 
    features = [
        "compression",    # LZ4, Zstd, Snappy compression
        "encryption",     # TLS, ChaCha20Poly1305 encryption
        "message-queues", # RabbitMQ, Kafka, Redis integration
        "monitoring",     # Prometheus metrics, tracing
        "quic"           # QUIC protocol support
    ]
}
```

## Testing

```bash
# Run all tests
cargo test --workspace

# Run tests with all features
cargo test --workspace --all-features

# Run specific test categories
cargo test --test network
cargo test --test topology_integration
```

## Development

### Project Structure

```
transport/
├── src/
│   ├── lib.rs                  # Public API
│   ├── error.rs                # Error types
│   ├── network/                # Direct network transport
│   │   ├── envelope.rs         # Wire protocol
│   │   ├── tcp.rs             # TCP implementation
│   │   ├── udp.rs             # UDP implementation
│   │   ├── compression.rs     # Compression engines
│   │   └── security.rs        # Encryption layers
│   ├── mq/                    # Message queue integration
│   ├── hybrid/                # Hybrid transport system
│   ├── topology_integration/  # Topology system integration
│   └── monitoring/            # Metrics and health monitoring
├── examples/                  # Configuration examples
├── tests/                     # Integration tests
└── benches/                   # Performance benchmarks
```

### Adding New Transport Protocols

1. Create protocol implementation in `src/network/`
2. Add configuration options to `NetworkConfig`
3. Update `NetworkTransport` to handle new protocol
4. Add tests and benchmarks
5. Update documentation

### Contributing

- Follow AlphaPulse coding standards
- Add comprehensive tests for new features
- Include performance benchmarks for transport changes
- Update documentation and examples
- Ensure all CI checks pass

## License

Licensed under the same terms as the main AlphaPulse project.