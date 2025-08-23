# AlphaPulse Protocol V2 - Message Types Reference

**⚠️ This file is auto-generated from `protocol_v2/src/tlv/types.rs` - DO NOT EDIT MANUALLY**

This document provides a comprehensive index of all TLV message types defined in the AlphaPulse Protocol V2.

## Overview

- **Total Types**: 47 implemented
- **Market Data**: 17 types (1-19)
- **Strategy Signals**: 12 types (20-39)
- **Execution**: 10 types (40-59)
- **System**: 8 types (100-119)

## Market Data Domain (Types 1-19)
*Routes through MarketDataRelay*

| Type | Name | Description | Size | Status |
|------|------|-------------|------|---------|
| 1 | Trade | Individual trade execution with price, volume, side, timestamp | 37 bytes | ✅ Implemented |
| 2 | Quote | Bid/ask quote update with current best prices and sizes | 52 bytes | ✅ Implemented |
| 3 | OrderBook | Multiple price levels with quantities for market depth | Variable | ✅ Implemented |
| 4 | InstrumentMeta | TLV message type - see documentation for details | Variable | ✅ Implemented |
| 5 | L2Snapshot | TLV message type - see documentation for details | Variable | ✅ Implemented |
| 6 | L2Delta | TLV message type - see documentation for details | Variable | ✅ Implemented |
| 7 | L2Reset | TLV message type - see documentation for details | Variable | ✅ Implemented |
| 8 | PriceUpdate | TLV message type - see documentation for details | Variable | ✅ Implemented |
| 9 | VolumeUpdate | TLV message type - see documentation for details | Variable | ✅ Implemented |
| 10 | PoolLiquidity | TLV message type - see documentation for details | 20-300 bytes | ✅ Implemented |
| 11 | PoolSwap | DEX swap event with V3 state updates and reserves | 60-200 bytes | ✅ Implemented |
| 12 | PoolMint | TLV message type - see documentation for details | 50-180 bytes | ✅ Implemented |
| 13 | PoolBurn | TLV message type - see documentation for details | 50-180 bytes | ✅ Implemented |
| 14 | PoolTick | TLV message type - see documentation for details | 30-120 bytes | ✅ Implemented |
| 15 | PoolState | TLV message type - see documentation for details | 60-200 bytes | ✅ Implemented |
| 16 | PoolSync | TLV message type - see documentation for details | 40-150 bytes | ✅ Implemented |
| 255 | ExtendedTLV | TLV message type - see documentation for details | Variable | 🔄 Extended |

## Strategy Signal Domain (Types 20-39)
*Routes through SignalRelay*

| Type | Name | Description | Size | Status |
|------|------|-------------|------|---------|
| 20 | SignalIdentity | Strategy identification with signal ID and confidence | 16 bytes | ✅ Implemented |
| 21 | AssetCorrelation | TLV message type - see documentation for details | 24 bytes | ✅ Implemented |
| 22 | Economics | Profit estimates and capital requirements for execution | 32 bytes | ✅ Implemented |
| 23 | ExecutionAddresses | TLV message type - see documentation for details | 84 bytes | ✅ Implemented |
| 24 | VenueMetadata | TLV message type - see documentation for details | 12 bytes | ✅ Implemented |
| 25 | StateReference | TLV message type - see documentation for details | 24 bytes | ✅ Implemented |
| 26 | ExecutionControl | TLV message type - see documentation for details | 16 bytes | ✅ Implemented |
| 27 | PoolAddresses | TLV message type - see documentation for details | 44 bytes | ✅ Implemented |
| 28 | MEVBundle | TLV message type - see documentation for details | 40 bytes | ✅ Implemented |
| 29 | TertiaryVenue | TLV message type - see documentation for details | 24 bytes | ✅ Implemented |
| 30 | RiskParameters | TLV message type - see documentation for details | 24-512 bytes | ✅ Implemented |
| 31 | PerformanceMetrics | TLV message type - see documentation for details | 32-1024 bytes | ✅ Implemented |

## Execution Domain (Types 40-59)
*Routes through ExecutionRelay*

| Type | Name | Description | Size | Status |
|------|------|-------------|------|---------|
| 40 | OrderRequest | Order placement request with type, quantity, limits | 32 bytes | ✅ Implemented |
| 41 | OrderStatus | TLV message type - see documentation for details | 24 bytes | ✅ Implemented |
| 42 | Fill | Execution confirmation with actual price, quantity, fees | 32 bytes | ✅ Implemented |
| 43 | OrderCancel | TLV message type - see documentation for details | 16 bytes | ✅ Implemented |
| 44 | OrderModify | TLV message type - see documentation for details | 24 bytes | ✅ Implemented |
| 45 | ExecutionReport | TLV message type - see documentation for details | 48 bytes | ✅ Implemented |
| 46 | Portfolio | TLV message type - see documentation for details | 32-2048 bytes | ✅ Implemented |
| 47 | Position | TLV message type - see documentation for details | 24-512 bytes | ✅ Implemented |
| 48 | Balance | TLV message type - see documentation for details | 16-256 bytes | ✅ Implemented |
| 49 | TradeConfirmation | TLV message type - see documentation for details | 32-256 bytes | ✅ Implemented |

## System Domain (Types 100-119)
*Routes through SystemRelay*

| Type | Name | Description | Size | Status |
|------|------|-------------|------|---------|
| 100 | Heartbeat | Service health check with timestamp and status | 16 bytes | ✅ Implemented |
| 101 | Snapshot | TLV message type - see documentation for details | 32-1024 bytes | ✅ Implemented |
| 102 | Error | TLV message type - see documentation for details | 16-512 bytes | ✅ Implemented |
| 103 | ConfigUpdate | TLV message type - see documentation for details | 20-2048 bytes | ✅ Implemented |
| 104 | ServiceDiscovery | TLV message type - see documentation for details | 24-512 bytes | ✅ Implemented |
| 110 | RecoveryRequest | TLV message type - see documentation for details | 18 bytes | ✅ Implemented |
| 111 | RecoveryResponse | TLV message type - see documentation for details | 20-1024 bytes | ✅ Implemented |
| 112 | SequenceSync | TLV message type - see documentation for details | 16-256 bytes | ✅ Implemented |

## Usage Examples

### Querying Types by Domain
```rust
use alphapulse_protocol_v2::tlv::TLVType;
use alphapulse_protocol_v2::RelayDomain;

// Get all market data types
let market_types = TLVType::types_in_domain(RelayDomain::MarketData);
for tlv_type in market_types {
    let info = tlv_type.type_info();
    println!("{}: {}", info.name, info.description);
}
```

### Type Information API
```rust
let trade_info = TLVType::Trade.type_info();
println!("Type {}: {} bytes", trade_info.type_number, 
         match trade_info.size_constraint {
             TLVSizeConstraint::Fixed(size) => size.to_string(),
             _ => "Variable".to_string()
         });
```

---
*Generated automatically from code*
