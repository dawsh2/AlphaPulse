# Protocol V2 TLV System Maintenance Guide

## Overview

This document outlines critical maintenance procedures for the AlphaPulse Protocol V2 TLV (Type-Length-Value) message system. The TLV system is the core binary protocol that handles >1M messages/second with strict precision and performance requirements.

**⚠️ CRITICAL**: This system handles real financial data. Incorrect maintenance can cause precision loss, data corruption, or system failures.

## System Architecture Refresh

```
Exchanges → Collectors (Rust) → Binary Protocol → Relay → Bridge → Dashboard
         WebSocket            48-byte messages   Unix Socket  JSON    WebSocket
```

- **Collectors**: Convert exchange data to Protocol V2 binary TLV messages
- **Relays**: Route messages by domain (Market Data, Signals, Execution)
- **TLV Format**: Fixed 32-byte header + variable TLV payload
- **Precision**: 8 decimal places for all financial values

## Critical Files Requiring Maintenance

### 1. TLV Type Registry (`protocol_v2/src/tlv/types.rs`)

**What**: Central registry of all TLV message types (1-255)

**Critical Sections**:
```rust
#[repr(u8)]
pub enum TLVType {
    // Market Data Domain (1-19)
    Trade = 1,
    Quote = 2,
    OrderBook = 3,
    // ...
}

pub fn expected_payload_size(&self) -> Option<usize> {
    match self {
        TLVType::Trade => Some(37),  // MUST match TradeTLV size exactly
        TLVType::Quote => Some(52),  // MUST match QuoteTLV size exactly
        // ...
    }
}
```

**Maintenance**:
- ✅ Never reuse type numbers
- ✅ Update `expected_payload_size()` when struct sizes change
- ✅ Maintain domain boundaries (1-19 = Market Data, 20-39 = Signals, etc.)
- ✅ Document all changes in `message-types.md`

### 2. TLV Structure Definitions (`protocol_v2/src/tlv/market_data.rs`)

**What**: Binary struct layouts for zero-copy serialization

**Critical Requirements**:
```rust
/// CRITICAL: Field order and sizes must never change without migration
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, AsBytes, FromBytes, FromZeroes)]
pub struct QuoteTLV {
    pub venue_id: u16,         // 2 bytes - VenueId as primitive
    pub asset_type: u8,        // 1 byte - AssetType as primitive  
    pub reserved: u8,          // 1 byte - Reserved for alignment
    pub asset_id: u64,         // 8 bytes - Asset identifier
    pub bid_price: i64,        // 8 bytes - Fixed-point, 8 decimals
    pub bid_size: i64,         // 8 bytes - Fixed-point, 8 decimals
    pub ask_price: i64,        // 8 bytes - Fixed-point, 8 decimals
    pub ask_size: i64,         // 8 bytes - Fixed-point, 8 decimals
    pub timestamp_ns: u64,     // 8 bytes - Nanoseconds since epoch
}
// Total: 52 bytes - MUST match expected_payload_size()
```

**Maintenance Rules**:
- ✅ **NEVER** reorder fields in existing structs
- ✅ **NEVER** change field types without full migration
- ✅ **ALWAYS** use `#[repr(C, packed)]`
- ✅ **ALWAYS** derive `AsBytes, FromBytes, FromZeroes`
- ✅ **ALWAYS** use fixed-point i64 for prices/volumes (8 decimals)
- ✅ **ALWAYS** use u64 nanoseconds for timestamps

### 3. Message Type Reference (`docs/message-types.md`)

**What**: Human-readable registry of all TLV types

**Maintenance**:
- ✅ Update when adding new TLV types
- ✅ Mark implementation status: ✅ Implemented, 🚧 Planned, 📝 Reserved
- ✅ Document size and routing behavior
- ✅ Include usage examples

### 4. Exchange Collectors (`services_v2/adapters/src/input/collectors/`)

**What**: Convert exchange-specific data to Protocol V2 TLV

**Critical Mappings**:
```rust
// Kraken-specific - MAINTAIN CONSISTENCY
let instrument_id = match pair {
    "XBT/USD" => InstrumentId::stock(venue_id, "BTCUSD"),
    "ETH/USD" => InstrumentId::stock(venue_id, "ETHUSD"),
    _ => return Err(KrakenError::InvalidMessageFormat(format!("Unsupported pair: {}", pair))),
};

// Fixed-point conversion - CRITICAL PRECISION
let price_fixed = (price_float * 100_000_000.0) as i64;  // 8 decimals
```

**Maintenance**:
- ✅ Keep exchange symbol mappings consistent
- ✅ Always convert to 8-decimal fixed-point
- ✅ Use proper InstrumentId construction methods
- ✅ Handle all exchange-specific edge cases

## Adding New TLV Message Types

### Checklist for New TLV Type

When adding a new message type (e.g., `OrderBookDepthTLV`):

1. **Plan the Type Number**
   ```rust
   // Add to TLVType enum with unique number in correct domain
   OrderBookDepth = 4,  // Market Data domain (1-19)
   ```

2. **Design the Binary Structure**
   ```rust
   #[repr(C, packed)]
   #[derive(Debug, Clone, Copy, PartialEq, AsBytes, FromBytes, FromZeroes)]
   pub struct OrderBookDepthTLV {
       pub venue_id: u16,           // 2 bytes
       pub instrument_id: [u8; 12], // 12 bytes - InstrumentId as bytes
       pub levels: u8,              // 1 byte - number of levels
       pub reserved: [u8; 3],       // 3 bytes - padding for alignment
       // ... level data
   }
   ```

3. **Update Expected Size**
   ```rust
   TLVType::OrderBookDepth => Some(calculate_size_here),
   ```

4. **Add Constructor and Methods**
   ```rust
   impl OrderBookDepthTLV {
       pub fn new(venue: VenueId, instrument: InstrumentId, levels: u8) -> Self { ... }
       pub fn to_tlv_message(&self) -> TLVMessage { ... }
       pub fn from_bytes(data: &[u8]) -> Result<Self, String> { ... }
   }
   ```

5. **Write Tests**
   ```rust
   #[test]
   fn test_orderbook_depth_roundtrip() {
       let depth = OrderBookDepthTLV::new(...);
       let bytes = depth.as_bytes().to_vec();
       let recovered = OrderBookDepthTLV::from_bytes(&bytes).unwrap();
       assert_eq!(depth, recovered);
   }
   ```

6. **Update Documentation**
   - Add to `message-types.md`
   - Update collector integration docs
   - Add usage examples

## Financial Precision Requirements

### Fixed-Point Decimal Convention

**Rule**: ALL financial values use 8-decimal fixed-point representation

```rust
// ✅ CORRECT: String "123.45678900" → i64: 12345678900
let price_str = "123.45678900";
let price_decimal = Decimal::from_str_exact(price_str)?;
let price_fixed = (price_decimal.to_f64().unwrap() * 100_000_000.0) as i64;

// ❌ WRONG: Using floating point directly
let price_float: f64 = 123.45678900;  // Precision loss risk!
```

**Validation**:
```rust
// Always verify precision in tests
assert_eq!(price_fixed, 12345678900i64);
```

## Performance Requirements

### Benchmarks to Maintain

**Target Performance**:
- Message construction: >1M msg/s
- Message parsing: >1.6M msg/s  
- InstrumentId operations: >19M ops/s
- Memory usage: <50MB per service

**Monitoring**:
```bash
# Run performance tests regularly
cargo run --bin test_protocol --release

# Expected output:
# ⚡ Message construction: 825264 msg/s
# ⚡ Message parsing: 1786498 msg/s
# ⚡ InstrumentId operations: 18375877 ops/s
```

**Performance Regression Detection**:
- Run benchmarks before/after changes
- Flag >10% performance degradation
- Profile hot paths with `cargo flamegraph`

## Binary Compatibility Management

### Breaking Changes

**Never Break**:
- Field order in existing structs
- Field types in existing structs  
- Magic numbers (0xDEADBEEF)
- Message header format
- TLV type number assignments

**Safe Changes**:
- Adding new TLV types (use new numbers)
- Adding new fields to END of structs (with version bump)
- Extending reserved ranges

**Migration Process**:
1. Version bump in protocol header
2. Support both old and new formats
3. Gradual rollout with compatibility layer
4. Remove old format after full deployment

### Struct Evolution Example

```rust
// V1 - Original
pub struct TradeV1 {
    pub price: i64,
    pub volume: i64,
}

// V2 - Safe addition at end
pub struct TradeV2 {
    pub price: i64,
    pub volume: i64,
    pub fees: i64,        // NEW - added at end
    pub version: u8,      // Track version
}
```

## Testing Requirements

### Critical Tests to Maintain

1. **Size Validation**
   ```rust
   #[test]
   fn test_tlv_sizes() {
       assert_eq!(size_of::<TradeTLV>(), 37);
       assert_eq!(size_of::<QuoteTLV>(), 52);
       // Add for every fixed-size TLV
   }
   ```

2. **Serialization Roundtrip**
   ```rust
   #[test]
   fn test_roundtrip() {
       let tlv = QuoteTLV::new(...);
       let bytes = tlv.as_bytes().to_vec();
       let recovered = QuoteTLV::from_bytes(&bytes).unwrap();
       assert_eq!(tlv, recovered);
   }
   ```

3. **Precision Preservation**
   ```rust
   #[test]
   fn test_precision() {
       let price_str = "123.45678901";  // 8+ decimals
       let fixed = convert_to_fixed_point(price_str);
       let back = convert_from_fixed_point(fixed);
       assert_eq!(back, "123.45678901");  // No precision loss
   }
   ```

4. **Exchange Integration**
   ```rust
   #[test]
   fn test_kraken_conversion() {
       let kraken_trade = parse_kraken_message("...");
       let tlv = convert_to_trade_tlv(kraken_trade);
       // Verify all fields converted correctly
   }
   ```

### Test Automation

```bash
# Run before every commit
cargo test --lib
cargo run --bin test_protocol

# Performance regression check
cargo bench --baseline main

# Integration test with real exchange data
cargo test --test live_kraken_simple
```

## Exchange-Specific Maintenance

### Kraken Collector

**Critical Mappings**:
```rust
// Symbol mapping - UPDATE when Kraken adds pairs
"XBT/USD" => InstrumentId::stock(VenueId::Kraken, "BTCUSD"),
"ETH/USD" => InstrumentId::stock(VenueId::Kraken, "ETHUSD"),
"ADA/USD" => InstrumentId::stock(VenueId::Kraken, "ADAUSD"),
```

**Data Format Handling**:
```rust
// Kraken trade format: [price, volume, time, side, orderType, misc]
// MAINTAIN: Array position mappings
let price_str = trade_array[0].as_str()?;      // Position 0
let volume_str = trade_array[1].as_str()?;     // Position 1  
let time_str = trade_array[2].as_str()?;       // Position 2
let side_str = trade_array[3].as_str()?;       // Position 3
```

**Timestamp Conversion**:
```rust
// Kraken: seconds.microseconds → nanoseconds
let time_seconds = time_str.parse::<f64>()?;
let timestamp_ns = (time_seconds * 1_000_000_000.0) as u64;
```

### Adding New Exchange

1. **Study Exchange API**
   - WebSocket message formats
   - Symbol naming conventions
   - Timestamp formats
   - Decimal precision

2. **Create Collector**
   ```rust
   pub struct NewExchangeCollector {
       config: NewExchangeConfig,
       output_tx: mpsc::Sender<Vec<u8>>,  // Protocol V2 binary
   }
   ```

3. **Map Symbols**
   ```rust
   fn map_symbol(exchange_symbol: &str) -> Result<InstrumentId> {
       match exchange_symbol {
           "BTCUSDT" => Ok(InstrumentId::stock(VenueId::NewExchange, "BTCUSD")),
           // ... map all supported pairs
       }
   }
   ```

## Error Handling and Debugging

### Common Issues

1. **Size Mismatch**
   ```
   Error: PayloadTooLarge { size: 24 }
   Fix: Update expected_payload_size() to match struct
   ```

2. **Precision Loss**
   ```
   Error: Price 123.456789 became 123.45678
   Fix: Use Decimal::from_str_exact() not f64
   ```

3. **Alignment Issues**
   ```
   Error: Packed field access
   Fix: Copy field to local variable before comparison
   ```

### Debugging Tools

```bash
# Message inspection
RUST_LOG=debug cargo run --bin collector

# Binary analysis
hexdump -C message.bin

# Performance profiling
cargo flamegraph --bin collector

# Memory usage
valgrind --tool=massif ./target/release/collector
```

## Version and Release Management

### Pre-Release Checklist

- [ ] All tests passing (cargo test --workspace)
- [ ] Performance benchmarks within targets
- [ ] No precision loss in financial calculations
- [ ] Exchange mappings updated
- [ ] Documentation updated
- [ ] Breaking changes documented
- [ ] Migration path tested

### Release Notes Template

```markdown
## Protocol V2.X.Y Release

### New Features
- Added OrderBookDepthTLV (Type 4) for L2 market data
- Enhanced Kraken collector with new trading pairs

### Breaking Changes
- None (backward compatible)

### Performance
- Message construction: 1.1M msg/s (+10% improvement)
- Memory usage: 45MB (-10% optimization)

### Bug Fixes
- Fixed precision loss in small decimal conversions
- Corrected timestamp handling for exchange X

### Migration Required
- None for this release
```

## Emergency Procedures

### Data Corruption Recovery

1. **Stop all services**
2. **Identify corruption scope**
3. **Restore from last known good state**
4. **Replay missed messages if possible**
5. **Validate data integrity before restart**

### Performance Degradation

1. **Monitor key metrics**
2. **Profile hot paths**
3. **Check for memory leaks**
4. **Verify no precision loss**
5. **Rollback if necessary**

### Schema Migration Emergency

1. **Never deploy breaking changes without migration**
2. **Always maintain compatibility layer**
3. **Test migration with production data volume**
4. **Have rollback plan ready**

## Contact and Escalation

For critical issues:
- Binary compatibility concerns → Architecture team
- Financial precision issues → Risk management
- Performance degradation → DevOps team
- Exchange API changes → Data team

## Appendix: Quick Reference

### File Locations
- TLV Types: `protocol_v2/src/tlv/types.rs`
- Market Data TLVs: `protocol_v2/src/tlv/market_data.rs`
- Collectors: `services_v2/adapters/src/input/collectors/`
- Tests: `protocol_v2/tests/` and `src/*/tests.rs`
- Docs: `docs/protocol.md`, `docs/message-types.md`

### Key Commands
```bash
# Test everything
cargo test --workspace

# Performance check  
cargo run --bin test_protocol --release

# Add new TLV type
# 1. Edit types.rs
# 2. Add struct in market_data.rs
# 3. Update expected_payload_size()
# 4. Add tests
# 5. Update message-types.md

# Debug TLV parsing
RUST_LOG=alphapulse_protocol_v2::tlv=debug cargo test
```