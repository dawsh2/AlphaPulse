# Torq Relay Infrastructure

High-performance message relay infrastructure for Protocol V2 TLV messages with domain-specific routing and validation.

## Architecture Overview

```
relays/
├── core/                    # Core relay infrastructure (shared)
├── domains/                 # Domain-specific implementations
├── config/                  # Configuration management
├── testing/                 # Organized testing framework
├── docs/                    # Documentation
└── deployment/             # Deployment utilities
```

## Core Infrastructure

**Location**: `relays/core/`

Shared infrastructure components used by all relay domains:

- **Transport Adapters**: Unix socket, TCP, and topology-based transport
- **Validation Policies**: Performance, reliability, and security validation modes  
- **Topic Registry**: Pub-sub routing with wildcard support
- **Message Construction**: Protocol V2 TLV message building utilities
- **Configuration**: Comprehensive configuration management

## Domain Implementations

### Market Data Relay (`domains/market_data/`)

- **TLV Types**: 1-19 (trades, quotes, order book updates)
- **Performance**: >1M messages/second target
- **Validation**: Minimal (performance-optimized)
- **Socket**: `/tmp/torq/market_data.sock`

### Signal Relay (`domains/signal/`) 

- **TLV Types**: 20-39 (arbitrage signals, strategy outputs)
- **Performance**: >100K messages/second target  
- **Validation**: Standard (CRC32 checksums)
- **Socket**: `/tmp/torq/signals.sock`

### Execution Relay (`domains/execution/`)

- **TLV Types**: 40-79 (order execution, trade confirmations)
- **Performance**: >50K messages/second target
- **Validation**: Audit (full validation + logging)
- **Socket**: `/tmp/torq/execution.sock`

## Configuration

**Location**: `relays/config/`

Environment-specific configuration with templates:

```
config/
├── environments/
│   ├── development/         # Development settings
│   ├── staging/            # Staging environment  
│   └── production/         # Production tuning
└── templates/              # Configuration templates
```

## Testing Framework

**Location**: `relays/testing/`

Organized testing with clear separation:

- **Unit Tests**: `testing/unit/` - Domain logic and utilities
- **Local Integration**: `testing/integration/local/` - Component integration
- **Live Integration**: `testing/integration/live/` - External service tests
- **Benchmarks**: `testing/benchmarks/` - Performance validation
- **Fixtures**: `testing/fixtures/` - Test data and mocks

## Building

```bash
# Build all relay components
cargo build --workspace

# Build specific domain
cargo build -p torq-relay-market-data

# Run tests
cargo test --workspace

# Run benchmarks
cargo bench
```

## Deployment

**Location**: `relays/deployment/`

Production deployment utilities:

- **Docker**: Container definitions and compose files
- **Kubernetes**: Production K8s manifests  
- **Scripts**: Deployment and management automation

## Performance Targets

| Relay Domain | Throughput | Latency | Validation |
|--------------|------------|---------|------------|
| Market Data  | >1M msg/s  | <5μs    | Minimal    |
| Signal       | >100K msg/s| <10μs   | CRC32      |
| Execution    | >50K msg/s | <20μs   | Full Audit |

## Protocol V2 Compliance

All relays maintain strict Protocol V2 compliance:

- **32-byte MessageHeader** with proper relay domain identification
- **TLV payload format** with domain-specific type ranges
- **Bijective InstrumentID** support for asset identification
- **Nanosecond timestamps** with no precision truncation
- **Native precision** preservation for DEX operations

## Next Steps

1. **Fix build issues** in workspace dependencies
2. **Complete domain binary implementations** in separate relay crates
3. **Add monitoring and metrics** collection
4. **Implement deployment automation**
5. **Add comprehensive documentation** for each component

## Contributing

- Follow existing code organization patterns
- Add tests for new functionality  
- Update configuration templates as needed
- Document performance impact of changes
- Maintain Protocol V2 compliance at all times