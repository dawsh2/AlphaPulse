# AlphaPulse Protocol V2 - Message Types Reference

This document provides a comprehensive index of all TLV message types defined in the AlphaPulse Protocol V2.

## TLV Type Organization

TLV types are organized by relay domain to maintain clean separation and routing:

```rust
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
pub enum TLVType {
    // Types organized by domain for efficient routing
}
```

## Market Data Domain (Types 1-19)
*Routes through MarketDataRelay*

| Type | Name | Description | Size | Status |
|------|------|-------------|------|--------|
| 1 | Trade | Price, volume, side, timestamp | 37 bytes | ✅ Implemented |
| 2 | Quote | Bid/ask prices and sizes | 54 bytes | ✅ Implemented |
| 3 | OrderBook | Level data with prices/quantities | Variable | ✅ Implemented |
| 4 | InstrumentMeta | Symbol, decimals, venue info | Variable | ✅ Implemented |
| 5 | L2Snapshot | Complete order book snapshot | Variable | ✅ Implemented |
| 6 | L2Delta | Order book updates | Variable | ✅ Implemented |
| 7 | L2Reset | Order book reset signal | Variable | ✅ Implemented |
| 8 | PriceUpdate | Price change notification | Variable | ✅ Implemented |
| 9 | VolumeUpdate | Volume change notification | Variable | ✅ Implemented |
| 10 | PoolLiquidity | DEX pool liquidity state | Variable | ✅ Implemented |
| 11 | PoolSwap | DEX swap event with V3 state updates (full uint160 precision) | Variable | ✅ Implemented |
| 12 | PoolMint | Liquidity add event | Variable | ✅ Implemented |
| 13 | PoolBurn | Liquidity remove event | Variable | ✅ Implemented |
| 14 | PoolTick | Tick crossing event (V3) | Variable | ✅ Implemented |
| 15 | PoolState | Pool state snapshot (full state) | Variable | ✅ Implemented |
| 16 | PoolSync | V2 Sync event (complete reserves) | Variable | ✅ Implemented |
| 17-19 | *Reserved* | Future market data types | - | 📝 Reserved |

## Strategy Signal Domain (Types 20-39)
*Routes through SignalRelay*

| Type | Name | Description | Size | Status |
|------|------|-------------|------|--------|
| 20 | SignalIdentity | Strategy ID, signal ID, confidence | 16 bytes | ✅ Implemented |
| 21 | AssetCorrelation | Base/quote instrument correlation | 24 bytes | ✅ Implemented |
| 22 | Economics | Profit estimates, capital requirements | 32 bytes | ✅ Implemented |
| 23 | ExecutionAddresses | Token contracts, router addresses | 84 bytes | ✅ Implemented |
| 24 | VenueMetadata | Venue types, fees, direction flags | 12 bytes | ✅ Implemented |
| 25 | StateReference | Block numbers, validity windows | 24 bytes | ✅ Implemented |
| 26 | ExecutionControl | Flags, slippage, priority settings | 16 bytes | ✅ Implemented |
| 27 | PoolAddresses | DEX pool contracts for quoter calls | 44 bytes | ✅ Implemented |
| 28 | MEVBundle | Flashbots bundle preferences | 40 bytes | ✅ Implemented |
| 29 | TertiaryVenue | Third venue for triangular arbitrage | 24 bytes | ✅ Implemented |
| 30 | RiskParameters | Risk limits and thresholds | Variable | ✅ Implemented |
| 31 | PerformanceMetrics | Strategy performance data | Variable | ✅ Implemented |
| 32-39 | *Reserved* | Future strategy signal types | - | 📝 Reserved |

## Execution Domain (Types 40-59)
*Routes through ExecutionRelay*

| Type | Name | Description | Size | Status |
|------|------|-------------|------|--------|
| 40 | OrderRequest | Order type, quantity, limits | 32 bytes | ✅ Implemented |
| 41 | OrderStatus | Fill status, remaining quantity | 24 bytes | ✅ Implemented |
| 42 | Fill | Execution price, quantity, fees | 32 bytes | ✅ Implemented |
| 43 | OrderCancel | Cancel request with reason | 16 bytes | ✅ Implemented |
| 44 | OrderModify | Modification parameters | 24 bytes | ✅ Implemented |
| 45 | ExecutionReport | Complete execution summary | 48 bytes | ✅ Implemented |
| 46 | Portfolio | Portfolio composition data | Variable | ✅ Implemented |
| 47 | Position | Individual position data | Variable | ✅ Implemented |
| 48 | Balance | Account balance information | Variable | ✅ Implemented |
| 49 | TradeConfirmation | Trade confirmation details | Variable | ✅ Implemented |
| 50-59 | *Reserved* | Future execution types | - | 📝 Reserved |

## Portfolio-Risk Domain (Types 60-79)
*Routes through ExecutionRelay for state consistency*

| Type | Name | Description | Size | Path |
|------|------|-------------|------|------|
| 60 | RiskDecision | Risk approval for managed strategies | 48 bytes | Risk-Managed |
| 61 | PositionUpdate | Portfolio state changes | 48 bytes | Risk-Managed |
| 62 | FlashLoanResult | Post-execution report from self-contained strategy | 32 bytes | Self-Contained |
| 63 | PostTradeAnalytics | Execution results for analysis | 40 bytes | Both |
| 64 | PositionQuery | Request current positions | 24 bytes | Risk-Managed |
| 65 | RiskMetrics | Current risk calculations | 64 bytes | Risk-Managed |
| 66 | CircuitBreaker | Emergency control activation | 16 bytes | Both |
| 67 | StrategyRegistration | Strategy type declaration | 24 bytes | Both |
| 68-79 | *Reserved* | Future portfolio/risk types | - | - |

## System Domain (Types 100-109)
*Direct connections or SystemRelay*

| Type | Name | Description | Size | Status |
|------|------|-------------|------|--------|
| 100 | Heartbeat | Service health and timestamp | 16 bytes | ✅ Implemented |
| 101 | Snapshot | State checkpoint data | Variable | ✅ Implemented |
| 102 | Error | Error codes and descriptions | Variable | ✅ Implemented |
| 103 | ConfigUpdate | Configuration changes | Variable | ✅ Implemented |
| 104 | ServiceDiscovery | Service registration/discovery | Variable | ✅ Implemented |
| 105 | MetricsReport | Performance and health metrics | Variable | ✅ Implemented |
| 106 | StateInvalidation | State reset/invalidation signal | Variable | ✅ Implemented |
| 107-109 | *Reserved* | Future system types | - | 📝 Reserved |

## Recovery Domain (Types 110-119)

| Type | Name | Description | Size | Status |
|------|------|-------------|------|--------|
| 110 | RecoveryRequest | Request missing sequence range | 18 bytes | ✅ Implemented |
| 111 | RecoveryResponse | Response with missing data | Variable | ✅ Implemented |
| 112 | SequenceSync | Sequence number synchronization | Variable | ✅ Implemented |
| 113-119 | *Reserved* | Future recovery types | - | 📝 Reserved |

## Extended and Vendor Ranges

### TraceContext TLV (Type 120)
| Type | Name | Description | Size | Status |
|------|------|-------------|------|--------|
| 120 | TraceContext | Distributed tracing context | 26 bytes | ✅ Implemented |

### Vendor/Private Range (Types 200-254)
| Type | Name | Description | Size | Usage |
|------|------|-------------|------|--------|
| 200 | CustomMetrics | Vendor-specific metrics | Variable | Example |
| 201 | ExperimentalSignal | Experimental signal types | Variable | Example |
| 202 | ProprietaryData | Proprietary vendor data | Variable | Example |
| 203-254 | *Available* | Vendor-specific extensions | Variable | Open |

### Extended TLV (Type 255)
| Type | Name | Description | Length Field | Status |
|------|------|-------------|--------------|--------|
| 255 | ExtendedTLV | Large payload support (>255 bytes) | u16/u32 | ✅ Implemented |

**Extended TLV Format:**
```
┌─────┬─────┬─────┬─────┬─────────────┐
│ 255 │ 0   │ T   │ L   │ Value       │
│ 1B  │ 1B  │ 1B  │ 2B  │ L bytes     │
└─────┴─────┴─────┴─────┴─────────────┘
```

## Production Validation Framework

### DEX Data Validation Pipeline

All DeFi pool events undergo comprehensive validation before TLV creation:

```rust
// Production validator with pool registry integration
let validator = ProductionPolygonValidator::new(rpc_url, cache_dir, chain_id).await?;

// Four-step validation framework:
// 1. ABI decoding with ethabi
// 2. TLV serialization 
// 3. TLV deserialization
// 4. Deep equality verification
let validated_event = validator.validate_production_swap(&log, dex_protocol).await?;

// Convert to TLV with full precision preservation
let pool_swap_tlv = PoolSwapTLV::from(validated_event);
// sqrt_price_x96_after: [u8; 20] for full uint160 precision
// Token decimals validated from on-chain contracts
```

### Pool Registry Validation

```rust
// Pool information validated against factory deployments
let pool_info = validator.query_pool_info_from_chain(pool_address).await?;
// Token decimals queried from ERC20 contracts
// Pool type detected via interface queries (V2/V3)
// Results cached for performance
```

## Message Construction Examples

### Market Data Message
```rust
let trade = TradeTLV::new(
    VenueId::Binance,
    instrument_id,
    price,
    volume,
    side,
    timestamp_ns
);

let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
    .add_tlv(TLVType::Trade, &trade)
    .build();
```

### Strategy Signal Message
```rust
let signal = SignalIdentityTLV { /* ... */ };
let economics = EconomicsTLV { /* ... */ };

let message = TLVMessageBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy)
    .add_tlv(TLVType::SignalIdentity, &signal)
    .add_tlv(TLVType::Economics, &economics)
    .build();
```

### Execution Message
```rust
let order = OrderRequestTLV { /* ... */ };

let message = TLVMessageBuilder::new(RelayDomain::Execution, SourceType::PortfolioManager)
    .add_tlv(TLVType::OrderRequest, &order)
    .build();
```

## Usage Guidelines

### TLV Type Selection
1. **Use the correct domain** - Types 1-19 for market data, 20-39 for signals, etc.
2. **Check size constraints** - Fixed-size TLVs must match expected sizes
3. **Reserve unknown types** - Forward compatibility requires graceful unknown type handling
4. **Vendor extensions** - Use 200-254 range for proprietary extensions

### Performance Considerations
- **Fixed-size TLVs** are faster to parse (no bounds checking needed)
- **Variable-size TLVs** offer flexibility but require length validation
- **Extended TLVs** (Type 255) have additional overhead for large payloads
- **Production validation** adds ~200μs per event for comprehensive safety checks
- **Pool cache hits** reduce validation overhead to <10μs for known pools

### Routing Behavior
- TLV type determines relay domain automatically
- Multiple TLVs in one message route to the same domain
- Cross-domain communication requires separate messages

## Implementation Status

### ✅ Production Ready
All core TLV types (1-120) are fully implemented with:
- Zero-copy serialization/deserialization
- Comprehensive test coverage
- Performance benchmarks >1M msg/s
- Robust error handling
- Production validation framework for DeFi data
- Full precision preservation (uint160 for sqrt_price_x96)
- Pool registry integration with on-chain validation

### 📝 Reserved for Future
Types marked as "Reserved" are allocated but not yet implemented, ensuring future extensibility without breaking changes.

### 🔧 Vendor Extensions
The vendor range (200-254) is available for custom extensions and experimental features.