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

### Bidirectional Connection Architecture
**Current Implementation**: Direct socket-to-socket forwarding for maximum performance:
- All connections are bidirectional by default (read + write tasks)
- No service classification or timing heuristics
- Messages broadcast to all connected clients immediately
- Eliminates race conditions from timing-based connection detection

### Domain-Specific Performance Policies
Each relay binary optimized per domain requirements:
- `market_data_relay`: Direct broadcast, no checksum validation for >1M msg/s
- `signal_relay`: Topic-based pub-sub with checksum validation for >100K msg/s  
- `execution_relay`: Full validation + audit for >50K msg/s

**Note**: `market_data_relay.rs` uses direct broadcast (not topic-based routing) for maximum performance.

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

### Running Market Data Relay (Fixed Architecture)

```bash
# Production market data relay with direct broadcast
cargo run --release -p alphapulse-relays --bin market_data_relay

# Critical: Start services in this exact order
# 1. Market data relay (creates Unix socket)
# 2. polygon_publisher (connects and sends TLV messages)  
# 3. Dashboard (connects and consumes TLV messages)
```

### Running Other Relays (Topic-Based)

```bash
# Signal relay with topic-based pub-sub
cargo run --release --bin signal_relay

# Execution relay with full validation
cargo run --release --bin execution_relay
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