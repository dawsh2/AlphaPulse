# AlphaPulse Relay Infrastructure

High-performance message routing layer that sits between the transport infrastructure (`infra/`) and application services (`services_v2/`).

## Architecture

Relays implement the application-aware routing logic, parsing message headers to determine routing based on domain and topic. They are a special class of foundational service that acts as the system's central nervous system.

```
services_v2 (producers) → relays (routing) → services_v2 (consumers)
                              ↓
                         infra/transport
```

## Design Principles

### Configuration Over Code
A single generic `Relay` implementation is configured per domain:
- `market_data.toml`: Domain 1, no checksum validation for >1M msg/s
- `signal.toml`: Domain 2, checksum validation for >100K msg/s  
- `execution.toml`: Domain 3, full validation + audit for >50K msg/s

### Topic-Based Pub-Sub
Relays perform coarse-grained filtering by topic:
- Producers publish to topics (e.g., "market_data_polygon")
- Relays route to topic subscribers only
- Consumers perform fine-grained filtering

### Transport Agnostic
Relays use the `infra/transport` layer, supporting:
- Unix domain sockets (same machine, <35μs)
- TCP/UDP (network, <5ms)
- Message queues (reliability, >20ms)
- Configuration determines transport, not code

## Performance Targets

| Relay Type | Target Throughput | Validation Policy | Latency |
|------------|------------------|-------------------|----------|
| Market Data | >1M msg/s | No checksum | <35μs |
| Signal | >100K msg/s | Checksum enabled | <100μs |
| Execution | >50K msg/s | Checksum + audit | <200μs |

## Usage

### Running a Relay

```bash
# Production relay with specific config
cargo run --release --bin relay -- --config config/market_data.toml

# Development mode with debug output
cargo run --bin relay_dev -- --type market_data --log-level debug
```

### Configuration Example

```toml
# config/market_data.toml
[relay]
domain = 1
name = "market_data"

[transport]
mode = "unix_socket"
path = "/tmp/alphapulse/market_data.sock"

[validation]
checksum = false  # Skip for performance
audit = false

[topics]
default = "market_data_all"
available = [
    "market_data_polygon",
    "market_data_ethereum",
    "market_data_kraken"
]

[performance]
buffer_size = 65536
max_connections = 1000
```

## Topic Routing

Messages are routed based on topics extracted from message metadata:

1. **Producer** sends message with topic "market_data_polygon"
2. **Relay** checks subscriber registry for that topic
3. **Relay** forwards only to subscribed consumers
4. **Consumer** performs fine filtering (e.g., specific trading pairs)

This provides efficient data distribution without overwhelming consumers.

## Integration with Transport

Relays leverage the `infra/transport` system for all communication:

```rust
use alphapulse_transport::TopologyIntegration;

let transport = TopologyIntegration::create_transport(
    &config.transport
).await?;
```

Transport selection (unix socket, TCP, QUIC) is purely configuration-driven.

## Development

### Directory Structure
```
relays/
├── src/
│   ├── lib.rs          # Public API
│   ├── relay.rs        # Generic relay implementation
│   ├── config.rs       # Configuration management
│   ├── topics.rs       # Topic-based routing
│   └── validation.rs   # Domain-specific validation
├── config/             # Relay configurations
├── bin/               # Binary entry points
└── benches/           # Performance benchmarks
```

### Testing
```bash
# Run all tests
cargo test

# Run benchmarks
cargo bench

# Test with specific config
cargo test --test config_validation
```

## Migration from Old Architecture

This relay infrastructure replaces:
- `services_v2/relays/` - Old service-based relays
- `protocol_v2/src/relay/` - Protocol-embedded relay code

The new architecture provides:
- Clear separation of concerns
- Configuration-driven behavior
- Topic-based routing
- Transport flexibility
- Better performance through proper layering