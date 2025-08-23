# AlphaPulse Protocol V2 - TLV Universal Message Protocol

## Overview

This is the next-generation message protocol for AlphaPulse, implementing a universal TLV (Type-Length-Value) format with bijective instrument IDs and domain-specific relays.

## Architecture

```
┌─────────────────┬─────────────────────────────────────┐
│ MessageHeader   │ TLV Payload                         │
│ (32 bytes)      │ (variable length)                   │
└─────────────────┴─────────────────────────────────────┘
```

### Key Features

- **Universal TLV Format**: All messages use header + TLV payload
- **Bijective IDs**: Self-describing instrument identifiers
- **Domain Separation**: Market data (1-19), signals (20-39), execution (40-59)
- **Extended TLVs**: Type 255 supports >255 byte payloads
- **Recovery Protocol**: Automatic sequence gap handling
- **Zero-copy Parsing**: Direct memory access with proper alignment
- **Mixed Transport**: Unix sockets + message bus coexistence

### Protocol Domains

1. **Market Data Domain (Types 1-19)**
   - Routes through MarketDataRelay
   - Trade, Quote, OrderBook, InstrumentMeta TLVs

2. **Strategy Signal Domain (Types 20-39)**
   - Routes through SignalRelay
   - SignalIdentity, AssetCorrelation, Economics TLVs

3. **Execution Domain (Types 40-59)**
   - Routes through ExecutionRelay
   - OrderRequest, Fill, ExecutionReport TLVs

4. **System Domain (Types 100-109)**
   - Heartbeat, Snapshot, Error, ConfigUpdate TLVs

5. **Recovery Domain (Type 110+)**
   - RecoveryRequest for sequence gap handling

6. **Vendor/Private (Types 200-254)**
   - Custom extensions and experimental features

## Performance Targets (✅ ACHIEVED)

- **Message construction**: >1M msg/s ✅ **(1,097,624 msg/s measured)**
- **Message parsing**: >1.6M msg/s ✅ **(1,643,779 msg/s measured)**
- **InstrumentId operations**: >19M ops/s ✅ **(19,796,915 ops/s measured)**
- **Memory**: Zero-copy parsing with cache-friendly layouts ✅
- **Latency**: <35μs hot path, <100ms warm path ✅

## Directory Structure

```
protocol_v2/
├── src/
│   ├── lib.rs                        # Main exports and core types
│   ├── message/                      # Message header implementation
│   │   ├── mod.rs
│   │   └── header.rs                 # 32-byte MessageHeader struct
│   ├── tlv/                          # TLV parsing and types
│   │   ├── mod.rs
│   │   ├── parser.rs                 # Zero-copy TLV parsing logic
│   │   ├── builder.rs                # TLV message construction
│   │   ├── types.rs                  # TLV type definitions and routing
│   │   ├── extended.rs               # Type 255 extended TLVs
│   │   ├── market_data.rs            # Market data TLV structures
│   │   ├── pool_state.rs             # DEX pool state TLVs
│   │   └── relay_parser.rs           # Relay-specific parsing
│   ├── identifiers/                  # Bijective instrument IDs
│   │   ├── mod.rs
│   │   └── instrument/               # InstrumentId implementation
│   │       ├── mod.rs
│   │       ├── core.rs               # InstrumentId struct and methods
│   │       ├── venues.rs             # VenueId and AssetType enums
│   │       └── pairing.rs            # Pool pairing and construction
│   ├── relay/                        # Domain-specific relay servers
│   │   ├── mod.rs
│   │   ├── core.rs                   # Base relay functionality
│   │   ├── market_data_relay.rs      # Market data relay (types 1-19)
│   │   ├── signal_relay.rs           # Strategy signal relay (types 20-39)
│   │   ├── execution_relay.rs        # Execution relay (types 40-59)
│   │   ├── consumer_registry.rs      # Consumer tracking and management
│   │   └── io/                       # Transport implementations
│   │       ├── mod.rs
│   │       ├── unix_socket.rs        # Unix domain socket transport
│   │       └── message_bus.rs        # Future message bus support
│   ├── recovery/                     # Recovery protocol
│   │   ├── mod.rs
│   │   ├── request.rs                # RecoveryRequest handling
│   │   └── snapshot.rs               # Snapshot creation/restoration
│   ├── validation/                   # Message validation
│   │   ├── mod.rs
│   │   ├── checksum.rs               # CRC32 validation policies
│   │   └── bounds.rs                 # TLV bounds checking
│   └── bin/                          # Test and demo binaries
│       └── test_protocol.rs          # Comprehensive protocol tests
├── tests/                            # Integration and stress tests
│   ├── protocol_core.rs              # Core protocol functionality
│   ├── tlv_parsing.rs                # TLV parsing edge cases
│   ├── recovery.rs                   # Recovery protocol scenarios
│   ├── stress_concurrent.rs          # Concurrent load testing
│   ├── integration/                  # End-to-end integration tests
│   ├── validation/                   # Data validation and precision tests
│   └── debug/                        # Debug and diagnostic tools
├── ../docs/                          # Documentation (shared)
│   ├── protocol.md                   # Complete protocol specification
│   ├── message-types.md              # TLV message type reference
│   ├── PERFORMANCE_ANALYSIS.md       # Performance benchmarks
│   └── POOL_MESSAGES_DESIGN.md       # DEX pool message design
└── examples/                         # Usage examples (to be created)
    ├── basic_usage.rs
    ├── multi_relay.rs
    └── mixed_transport.rs
```

## Usage Examples

### Creating Messages

```rust
use alphapulse_protocol_v2::*;

// Create bijective instrument ID
let usdc_id = InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48")?;

// Build TLV message
let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::ExchangeCollector)
    .add_tlv(TLVType::Trade, &trade_tlv)
    .add_tlv(TLVType::InstrumentMeta, &instrument_tlv)
    .build();
```

### Multi-Relay Consumer

```rust
let dashboard = Dashboard::new()?;
dashboard.connect_all_relays().await?;

loop {
    tokio::select! {
        msg = dashboard.read_market_data() => { /* handle market data */ }
        msg = dashboard.read_signals() => { /* handle strategy signals */ }
        msg = dashboard.read_execution() => { /* handle execution updates */ }
    }
}
```

## Migration from Protocol V1

The protocol_v2 runs in parallel with the existing protocol during migration:

1. **Phase 1**: New services use protocol_v2
2. **Phase 2**: Gradual service migration with mixed transport
3. **Phase 3**: Legacy protocol deprecation
4. **Phase 4**: Protocol_v2 becomes the primary protocol

## Testing

```bash
# Run all tests
cargo test --workspace

# Run comprehensive protocol tests
cargo run --bin test_protocol

# Performance benchmarks (included in test_protocol)
cargo run --bin test_protocol --release

# Specific test categories
cargo test tlv_parsing        # TLV parsing tests
cargo test recovery          # Recovery protocol tests
cargo test stress           # Stress and concurrent tests
cargo test integration      # End-to-end integration tests
```

## Documentation

- **[protocol.md](../docs/protocol.md)** - Complete protocol specification with implementation details
- **[message-types.md](../docs/message-types.md)** - Comprehensive TLV message type reference
- **[PERFORMANCE_ANALYSIS.md](../docs/PERFORMANCE_ANALYSIS.md)** - Performance benchmarks and analysis
- **[POOL_MESSAGES_DESIGN.md](../docs/POOL_MESSAGES_DESIGN.md)** - DEX pool message design rationale