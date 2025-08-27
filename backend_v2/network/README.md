# torq-network

High-performance network transport system for AlphaPulse trading infrastructure. This crate consolidates topology management and transport protocols into a unified, performance-optimized solution.

## Crate Consolidation

This crate unifies three previously separate crates:
- `alphapulse-topology` → `topology` module  
- `alphapulse-transport` → `transport` and `hybrid` modules
- `alphapulse-network` → `network` module

## Features

- **Multiple Transport Modes**: Direct TCP/UDP, Message Queues (RabbitMQ/Kafka/Redis), Hybrid routing
- **Protocol V2 Integration**: TLV message validation with domain separation 
- **Topology Management**: Actor placement with NUMA awareness and resource optimization
- **Precision Handling**: DEX token and traditional exchange precision preservation
- **Zero-Copy Operations**: High-performance serialization optimizations
- **Comprehensive Error Handling**: Context-preserving error types with retry logic

## Migration Guide

### Import Changes

```rust
// OLD: Multiple separate crates
use alphapulse_topology::{TopologyConfig, TopologyResolver, Actor};
use alphapulse_transport::{TransportConfig, TransportMode, ProtocolType}; 
use alphapulse_network::{NetworkConfig, NetworkTransport};

// NEW: Single unified crate
use torq_network::{
    TopologyConfig, TopologyResolver, Actor,     // topology
    TransportConfig, TransportMode, ProtocolType, // transport  
    NetworkConfig, NetworkTransport,             // network
};
```

### Cargo.toml Changes

```toml
# OLD: Multiple dependencies
[dependencies]
alphapulse-topology = { path = "../network/topology" }
alphapulse-transport = { path = "../network/transport" } 
alphapulse-network = { path = "../network" }

# NEW: Single dependency
[dependencies]
torq-network = { path = "../network" }
```

### ChannelConfig Name Collision Resolution

The consolidation resolved a naming conflict between transport and topology `ChannelConfig` types:

```rust
// Explicit type selection (recommended)
use torq_network::{TransportChannelConfig, TopologyChannelConfig};

// Or use qualified imports
use torq_network::hybrid::ChannelConfig as TransportChannel;
use torq_network::topology::ChannelConfig as TopologyChannel;

// Default alias (backward compatible - uses transport version)
use torq_network::ChannelConfig; // Same as TransportChannelConfig
```

## Quick Start

```rust
use torq_network::{
    NetworkTransport, NetworkConfig, ProtocolType,
    TransportConfig, TransportMode, CompressionType
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Network transport configuration
    let network_config = NetworkConfig::default();
    let mut network = NetworkTransport::new(network_config).await?;
    network.start().await?;

    // Or use high-level transport abstraction
    let transport_config = TransportConfig {
        mode: TransportMode::Direct,
        protocol: Some(ProtocolType::Tcp),
        compression: CompressionType::Lz4,
        // ... other fields
    };
    
    // Send message to remote actor
    let message = b"market_data_update";
    network.send_to_actor("remote_node", "price_analyzer", message).await?;
    
    Ok(())
}
```

## Module Structure

```
torq-network/
├── topology/          # Actor placement and NUMA optimization
│   ├── actors/        # Actor definitions and resource requirements
│   ├── nodes/         # Node configuration and capabilities  
│   ├── runtime/       # Actor runtime and lifecycle management
│   └── resolver/      # Actor-to-node placement resolution
├── transport/         # Core transport abstractions
│   ├── hybrid/        # Hybrid routing for different message types
│   ├── direct/        # Direct peer-to-peer networking  
│   └── message_queue/ # Message queue backends
├── network/           # Low-level network protocols
│   ├── tcp/           # TCP connection management
│   ├── udp/           # UDP high-frequency messaging
│   └── security/      # TLS encryption and authentication
├── protocol_v2/       # Protocol V2 TLV message validation
├── precision/         # Financial precision handling
└── error/            # Unified error types
```

## Protocol V2 Support

Enable Protocol V2 integration for TLV message validation:

```toml
[dependencies]
torq-network = { path = "../network", features = ["protocol-integration"] }
```

```rust
use torq_network::{ProtocolV2Validator, validate_timestamp_precision};

let validator = ProtocolV2Validator::new();
let result = validator.validate_message(&message_bytes)?;

// Validate timestamp precision (nanoseconds required)
validate_timestamp_precision(timestamp_ns)?;
```

### TLV Domain Separation

- **Market Data (1-19)**: Price updates, order book changes, trades
- **Signal (20-39)**: Trading signals, analytics, strategy outputs  
- **Execution (40-79)**: Order placement, fills, portfolio updates

## Precision Handling

Financial calculations require strict precision handling:

```rust
use torq_network::{TokenAmount, ExchangePrice, validate_precision};

// DEX tokens: preserve native precision
let weth = TokenAmount::new_weth(1_500_000_000_000_000_000); // 1.5 WETH (18 decimals)
let usdc = TokenAmount::new_usdc(2_000_000); // 2.0 USDC (6 decimals)

// Traditional exchanges: 8-decimal fixed-point USD prices  
let btc_price = ExchangePrice::from_usd(4_500_000_000_000); // $45,000.00

// Validate precision consistency
validate_precision(&weth, &btc_price)?;
```

## Performance Characteristics

- **TCP Direct**: <5ms latency for inter-node communication
- **UDP Direct**: <1ms latency for trading signals  
- **Shared Memory**: <35μs for same-node communication
- **Throughput**: >10,000 messages/second per connection
- **Protocol V2**: >1M msg/s construction, >1.6M msg/s parsing

## Transport Selection

The system automatically selects optimal transport based on:

- **Distance**: Same node (shared memory) vs different nodes (network)
- **Criticality**: Ultra-low latency, low latency, standard, high latency
- **Reliability**: Best effort, at-least-once, exactly-once, guaranteed delivery
- **Security**: Plain vs encrypted channels

```rust
use torq_network::{EndpointConfig, Criticality, Reliability};

// Ultra-low latency trading signals
let config = EndpointConfig::ultra_low_latency();

// High-throughput market data  
let config = EndpointConfig::high_throughput();

// Guaranteed delivery for compliance
let config = EndpointConfig::guaranteed_delivery();
```

## Feature Flags

```toml
[dependencies]
torq-network = { 
    path = "../network", 
    features = [
        "protocol-integration",  # Enable Protocol V2 TLV validation
        "numa-optimization",     # Enable NUMA-aware actor placement
        "quic",                 # Enable QUIC transport protocol
        "compression",          # Enable LZ4/Zstd/Snappy compression
        "encryption",           # Enable TLS and ChaCha20Poly1305
        "monitoring",           # Enable Prometheus metrics
        "message-queues",       # Enable RabbitMQ/Kafka/Redis backends
    ]
}
```

## Error Handling

The crate provides comprehensive error handling with context preservation:

```rust
use torq_network::{TransportError, TopologyError, Result};

// All topology errors convert to transport errors with context
let topology_error = TopologyError::ActorNotFound { actor: "test".to_string() };
let transport_error: TransportError = topology_error.into();

// Check error properties
if transport_error.is_retryable() {
    // Retry the operation
}

// Categorize for metrics
let category = transport_error.category(); // "topology", "network", "protocol", etc.
```

## Examples

### Basic Network Transport
```rust
use torq_network::{NetworkConfig, NetworkTransport};

let config = NetworkConfig::default();
let mut transport = NetworkTransport::new(config).await?;
transport.start().await?;
transport.send_to_actor("node2", "market_analyzer", b"price_update").await?;
```

### Topology Integration
```rust
use torq_network::{TopologyConfig, TopologyResolver};
use std::collections::HashMap;

let config = TopologyConfig {
    version: "1.0.0".to_string(),
    actors: HashMap::new(),
    nodes: HashMap::new(),
    inter_node: None,
};

let resolver = TopologyResolver::new(config.actors);
let node = resolver.resolve_actor_node("price_analyzer")?;
```

### Protocol V2 Validation
```rust
#[cfg(feature = "protocol-integration")]
use torq_network::ProtocolV2Validator;

let validator = ProtocolV2Validator::new();
let result = validator.validate_message(&raw_message)?;

println!("Message validation: {}", result.summary());
assert!(result.is_valid());
```

## Testing

Run tests with all features:

```bash
cargo test --all-features
```

Run consolidation integration tests:

```bash  
cargo test --test consolidation_integration
```

## Documentation

Generate documentation with all features:

```bash
cargo doc --all-features --open
```

## License

Same as AlphaPulse backend - see project root for license information.

## Migration Checklist

When migrating from separate crates:

- [ ] Update `Cargo.toml` to use single `torq-network` dependency
- [ ] Replace separate crate imports with unified imports  
- [ ] Handle `ChannelConfig` name collision if using both transport and topology
- [ ] Enable appropriate feature flags for Protocol V2 and other optional features
- [ ] Update any hardcoded crate names in documentation or comments
- [ ] Test that all functionality works as expected
- [ ] Update CI/CD configurations to use new crate structure