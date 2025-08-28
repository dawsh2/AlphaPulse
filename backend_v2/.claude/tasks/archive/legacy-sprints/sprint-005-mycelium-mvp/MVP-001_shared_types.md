# MVP-001: Shared Message Types Migration

## Task Overview
**Sprint**: 005-mycelium-mvp
**Priority**: CRITICAL
**Estimate**: 6 hours
**Status**: TODO
**Goal**: Move protocol message definitions to libs/types for sharing across actors

## Problem
Message types are currently scattered across services. For zero-cost actor communication, we need shared type definitions that can be wrapped in Arc<T> and passed between actors without serialization.

## Solution
Create a centralized `libs/types` library containing all message definitions organized by domain.

## Implementation

### Directory Structure
```
libs/types/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── market.rs      # Market data messages
│   ├── signals.rs     # Trading signals
│   ├── execution.rs   # Execution messages
│   └── common.rs      # Common types
```

### Cargo.toml
```toml
[package]
name = "torq-types"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
bytes = "1.5"
rust_decimal = "1.33"

[dev-dependencies]
tokio = { version = "1.35", features = ["full"] }
```

### Market Messages (market.rs)
```rust
use std::sync::Arc;
use serde::{Serialize, Deserialize};

/// Pool swap event from DEX
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PoolSwapEvent {
    pub pool_address: [u8; 20],
    pub token0: [u8; 20],
    pub token1: [u8; 20],
    pub amount0_in: i128,
    pub amount1_in: i128,
    pub amount0_out: i128,
    pub amount1_out: i128,
    pub sqrt_price_x96: u128,
    pub liquidity: u128,
    pub tick: i32,
    pub timestamp_ns: u64,
}

/// Quote update from CEX
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuoteUpdate {
    pub instrument_id: u64,
    pub bid_price: i64,  // Fixed-point 8 decimals
    pub ask_price: i64,
    pub bid_size: i64,
    pub ask_size: i64,
    pub timestamp_ns: u64,
}

/// Order book snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookSnapshot {
    pub instrument_id: u64,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub timestamp_ns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: i64,
    pub quantity: i64,
}

/// Aggregated market messages
#[derive(Debug, Clone)]
pub enum MarketMessage {
    Swap(Arc<PoolSwapEvent>),
    Quote(Arc<QuoteUpdate>),
    OrderBook(Arc<OrderBookSnapshot>),
    Volume(Arc<VolumeSnapshot>),
}

impl MarketMessage {
    /// Get timestamp from any market message
    pub fn timestamp_ns(&self) -> u64 {
        match self {
            Self::Swap(e) => e.timestamp_ns,
            Self::Quote(q) => q.timestamp_ns,
            Self::OrderBook(ob) => ob.timestamp_ns,
            Self::Volume(v) => v.timestamp_ns,
        }
    }
}
```

### Signal Messages (signals.rs)
```rust
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use rust_decimal::Decimal;

/// Arbitrage opportunity signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageSignal {
    pub signal_id: u64,
    pub opportunity_type: ArbitrageType,
    pub profit_usd: Decimal,
    pub capital_required: Decimal,
    pub gas_cost_estimate: Decimal,
    pub confidence: u8,
    pub pools: Vec<PoolInfo>,
    pub timestamp_ns: u64,
    pub expiry_ns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArbitrageType {
    Triangle,
    CrossVenue,
    FlashLoan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub address: [u8; 20],
    pub venue: String,
    pub reserves: (u128, u128),
}

/// Momentum trading signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MomentumSignal {
    pub signal_id: u64,
    pub instrument_id: u64,
    pub direction: Direction,
    pub strength: f64,
    pub entry_price: i64,
    pub target_price: i64,
    pub stop_loss: i64,
    pub timestamp_ns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Direction {
    Long,
    Short,
}

/// Aggregated signal messages
#[derive(Debug, Clone)]
pub enum SignalMessage {
    Arbitrage(Arc<ArbitrageSignal>),
    Momentum(Arc<MomentumSignal>),
    Liquidation(Arc<LiquidationSignal>),
}
```

### Common Types (common.rs)
```rust
use serde::{Serialize, Deserialize};

/// Fixed-point decimal with 8 decimal places
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct FixedPoint8(pub i64);

impl FixedPoint8 {
    pub const SCALE: i64 = 100_000_000;
    
    pub fn from_float(value: f64) -> Self {
        Self((value * Self::SCALE as f64) as i64)
    }
    
    pub fn to_float(&self) -> f64 {
        self.0 as f64 / Self::SCALE as f64
    }
}

/// Unique actor identifier
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorId(pub uuid::Uuid);

impl ActorId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}
```

### Migration from Protocol V2
```rust
// Add conversion traits for backward compatibility
use protocol_v2::tlv::{TradeTLV, PoolSwapTLV};

impl From<&TradeTLV> for QuoteUpdate {
    fn from(tlv: &TradeTLV) -> Self {
        QuoteUpdate {
            instrument_id: tlv.instrument_id,
            bid_price: tlv.bid_price,
            ask_price: tlv.ask_price,
            bid_size: tlv.bid_quantity,
            ask_size: tlv.ask_quantity,
            timestamp_ns: tlv.timestamp_ns,
        }
    }
}

impl From<&PoolSwapTLV> for PoolSwapEvent {
    fn from(tlv: &PoolSwapTLV) -> Self {
        PoolSwapEvent {
            pool_address: tlv.pool_address,
            token0: tlv.token0,
            token1: tlv.token1,
            amount0_in: tlv.amount0_in,
            amount1_in: tlv.amount1_in,
            amount0_out: tlv.amount0_out,
            amount1_out: tlv.amount1_out,
            sqrt_price_x96: tlv.sqrt_price_x96,
            liquidity: tlv.liquidity,
            tick: tlv.tick,
            timestamp_ns: tlv.timestamp_ns,
        }
    }
}
```

## TDD Test Cases

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_size() {
        // Verify Arc adds minimal overhead
        assert_eq!(std::mem::size_of::<Arc<PoolSwapEvent>>(), 8);
        assert_eq!(std::mem::size_of::<Arc<ArbitrageSignal>>(), 8);
    }

    #[test]
    fn test_enum_size() {
        // Verify enums are pointer-sized
        assert_eq!(std::mem::size_of::<MarketMessage>(), 16);
        assert_eq!(std::mem::size_of::<SignalMessage>(), 16);
    }

    #[test]
    fn test_message_timestamp() {
        let swap = Arc::new(PoolSwapEvent {
            timestamp_ns: 123456789,
            ..Default::default()
        });
        
        let msg = MarketMessage::Swap(swap);
        assert_eq!(msg.timestamp_ns(), 123456789);
    }

    #[test]
    fn test_fixed_point_conversion() {
        let fp = FixedPoint8::from_float(123.45678901);
        assert_eq!(fp.0, 12345678901);
        assert!((fp.to_float() - 123.45678901).abs() < 0.000001);
    }

    #[test]
    fn test_tlv_compatibility() {
        // Verify conversion from Protocol V2
        let tlv = PoolSwapTLV { /* ... */ };
        let event: PoolSwapEvent = (&tlv).into();
        assert_eq!(event.pool_address, tlv.pool_address);
    }

    #[test]
    fn test_arc_sharing() {
        let signal = Arc::new(ArbitrageSignal { /* ... */ });
        let msg1 = SignalMessage::Arbitrage(signal.clone());
        let msg2 = SignalMessage::Arbitrage(signal.clone());
        
        // Verify both point to same allocation
        assert_eq!(Arc::strong_count(&signal), 3);
    }
}
```

## Migration Checklist
- [ ] Create libs/types directory structure
- [ ] Move market message types
- [ ] Move signal message types
- [ ] Move execution message types
- [ ] Add Protocol V2 conversion traits
- [ ] Update existing services to import from libs/types
- [ ] Run tests to verify no breaking changes
- [ ] Update documentation

## Dependencies to Update
```toml
# In services that use these types
[dependencies]
torq-types = { path = "../../libs/types" }
```

## Definition of Done
- All shared message types in libs/types
- No circular dependencies
- Protocol V2 compatibility maintained
- Tests demonstrate Arc sharing
- Services updated to use shared types
- Zero breaking changes to existing code