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

## Performance Targets

- **Market data**: 1M+ messages/second
- **Strategy signals**: 100K messages/second  
- **Execution orders**: 10K messages/second
- **Memory**: Zero-copy parsing with cache-friendly layouts
- **Latency**: <35μs hot path, <100ms warm path

## Directory Structure

```
protocol_v2/
├── src/
│   ├── lib.rs                 # Main exports
│   ├── header.rs              # 32-byte message header
│   ├── tlv/                   # TLV parsing and types
│   │   ├── mod.rs
│   │   ├── parser.rs          # TLV parsing logic
│   │   ├── builder.rs         # TLV message construction
│   │   ├── types.rs           # TLV type definitions
│   │   └── extended.rs        # Type 255 extended TLVs
│   ├── instrument_id/         # Bijective instrument IDs
│   │   ├── mod.rs
│   │   ├── core.rs           # InstrumentId struct
│   │   ├── venues.rs         # Venue and asset type enums
│   │   └── pairing.rs        # Cantor pairing for pools
│   ├── recovery/              # Recovery protocol
│   │   ├── mod.rs
│   │   ├── request.rs        # RecoveryRequest handling
│   │   └── snapshot.rs       # Snapshot creation/restoration
│   ├── transport/             # Mixed transport modes
│   │   ├── mod.rs
│   │   ├── unix_socket.rs    # Unix domain sockets
│   │   └── message_bus.rs    # Future message bus support
│   ├── validation/            # Message validation
│   │   ├── mod.rs
│   │   ├── checksum.rs       # CRC32 validation
│   │   └── bounds.rs         # Bounds checking
│   └── bin/                  # Test binaries
│       ├── tlv_test.rs       # TLV parsing tests
│       └── recovery_demo.rs  # Recovery protocol demo
├── tests/                     # Integration tests
│   ├── bijective_tests.rs    # Bijective ID properties
│   ├── recovery_tests.rs     # Recovery scenarios
│   └── performance_tests.rs  # Performance benchmarks
└── examples/                  # Usage examples
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

# Performance benchmarks
cargo bench

# TLV parsing tests
cargo run --bin tlv_test

# Recovery protocol demo  
cargo run --bin recovery_demo
```