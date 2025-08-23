# AlphaPulse Registry System - Technical Overview

## Executive Summary

The Registry System is a **unified data management layer** that acts as the "brain" for tracking and managing all tradeable instruments across DeFi, CEX, and TradFi markets. Think of it as a high-performance database that knows about every asset, every trading venue, and how they all relate to each other - enabling intelligent cross-market trading decisions in microseconds.

## What Problem Does It Solve?

Currently, AlphaPulse has excellent infrastructure for processing market data through the relay system, but lacks a centralized way to:
- Track the same asset across multiple exchanges (e.g., Apple stock trades on 8+ exchanges with different tickers)
- Automatically discover new trading instruments as they appear
- Handle hash collisions when generating deterministic IDs
- Support trading strategies beyond simple arbitrage
- Create synthetic instruments from multiple components
- Route orders to the cheapest/fastest execution venue

The Registry solves all of these problems while maintaining the existing <35μs latency requirements.

## Core Components

### 1. **Instrument Registry** (`01-core/`)
The master database of all tradeable assets.

**What it tracks:**
- Tokens (USDC, ETH, etc.) with blockchain addresses
- Stocks (AAPL, TSLA) with ISIN/CUSIP identifiers  
- Futures, Options, ETFs, Currencies
- Synthetic instruments (basket trades, indices)

**Key Features:**
- Deterministic ID generation using Blake3 hashing
- Collision detection and mitigation
- Cross-exchange tracking via ISIN (e.g., find Apple on all 8 exchanges it trades on)
- Sub-microsecond lookups using lock-free data structures

### 2. **Venue Registry** (`02-venues/`)
Tracks all trading venues (exchanges, DEXs, brokers).

**What it manages:**
- Exchange connectivity details (APIs, WebSockets, FIX)
- Fee structures and volume discounts
- Trading hours and market status
- Performance metrics (liquidity, latency, reliability)
- Smart order routing decisions

### 3. **Event System** (`04-events/`)
Pub/sub system for reactive trading strategies.

**Event Types:**
- New instrument discoveries
- Price updates across venues
- Arbitrage opportunities detected
- Hash collisions (critical alerts)
- Venue status changes

**Subscription Patterns:**
- By asset type (all stocks, all tokens on Polygon)
- By venue (all Binance instruments)
- By opportunity (arbitrage > 50 bps)
- By ISIN pattern (all US securities starting with "US")

### 4. **Binary Protocol** (`05-binary-protocol/`)
Ultra-efficient serialization for registry messages.

**Message Types:**
- Fixed-size messages (128 bytes for instruments)
- Variable-size with header validation
- Zero-copy deserialization
- CRC32 checksums
- Compression support (LZ4, Snappy, Zstd)

### 5. **Cross-Exchange Tracking** (`06-cross-exchange/`)
Enables trading the same asset across multiple venues.

**Example - Apple Inc:**
```
ISIN: US0378331005
- NASDAQ: AAPL (primary, highest liquidity)
- NYSE: AAPL  
- XETRA: APC (Germany)
- LSE: 0R2V (London)
- TSE: AAPL (Tokyo)
- HKEX: 0865 (Hong Kong)
```

The registry tracks all of these as the SAME instrument, enabling cross-market arbitrage.

## How Services Access the Registry

### Integration with Existing Architecture

Your current flow remains unchanged:
```
Collectors → Unix Sockets → Relay → Consumers
```

The Registry enhances this by adding a data enrichment layer:
```
Collectors → Unix Sockets → Relay → [Registry Lookup] → Enhanced Data → Strategies
```

### Access Patterns

#### 1. **Direct Lookup** (Most Common)
```rust
// Service wants to know about an instrument
let instrument = registry.get_by_id(instrument_id)?;
let venues = registry.find_all_venues_for_instrument(instrument_id)?;
```

#### 2. **Event Subscription**
```rust
// Service subscribes to specific patterns
let subscriber = registry.subscribe(SubscriptionPattern::AllTokens { 
    blockchain: Blockchain::Polygon 
});

// Receive events asynchronously
while let Ok(event) = subscriber.recv().await {
    match event {
        RegistryEvent::InstrumentAdded { .. } => handle_new_token(),
        RegistryEvent::ArbitrageDetected { .. } => execute_arbitrage(),
        _ => {}
    }
}
```

#### 3. **Unix Socket Protocol**
Services communicate with the registry using the existing Unix socket infrastructure:
```rust
// Send registration message
let msg = InstrumentRegistrationMessage { ... };
socket.send(msg.serialize())?;

// Receive updates
let update = socket.receive::<PriceUpdateMessage>()?;
```

#### 4. **Shared Memory** (Ultra-Low Latency)
For hot-path operations, the registry uses memory-mapped buffers:
```rust
// Zero-copy read from shared memory
let instrument = registry.mmap_lookup(instrument_id)?;
```

## Practical Benefits

### 1. **Automatic Instrument Discovery**
As new tokens are created or stocks are listed, the registry automatically discovers and tracks them.

### 2. **Cross-Market Opportunities**
Find arbitrage between:
- Same stock on different exchanges (AAPL on NASDAQ vs XETRA)
- Same token on different DEXs (USDC on Uniswap vs SushiSwap)
- Spot vs futures markets

### 3. **Smart Order Routing**
Automatically route orders to the best venue based on:
- Lowest fees
- Highest liquidity
- Fastest execution
- Best price

### 4. **Universal Strategy Support**
Beyond arbitrage, enables:
- Market making across venues
- Statistical arbitrage
- Cross-asset correlation trading
- Index rebalancing

### 5. **Risk Management**
- Detect and prevent hash collisions
- Track instrument relationships
- Monitor venue health
- Aggregate exposure across markets

## Performance Characteristics

| Operation | Latency | Throughput |
|-----------|---------|------------|
| ID Lookup | <35μs | 1M+ ops/sec |
| ISIN Cross-Reference | <50μs | 500k ops/sec |
| Event Dispatch | <10μs | 100k events/sec |
| Binary Serialization | <5μs | 2M+ msgs/sec |
| Instrument Registration | <100μs | 10k ops/sec |

## Implementation Approach

### Phase 1: Core Registry (Week 1)
- Deploy alongside existing relay server
- Start registering current instruments
- No changes to existing services

### Phase 2: Service Integration (Week 2)
- Add registry lookups to collectors
- Enable ISIN tracking
- Start publishing events

### Phase 3: Advanced Features (Week 3)
- Cross-exchange arbitrage detection
- Smart order routing
- Synthetic instruments

## Key Design Decisions

### Why Deterministic IDs?
- Consistent across restarts
- No database required
- Enable distributed operation
- Predictable and debuggable

### Why Lock-Free Data Structures?
- No mutex contention
- Predictable latency
- Scale to millions of lookups/sec
- Safe concurrent access

### Why Binary Protocol?
- Fixed-size messages for predictable parsing
- Zero-copy deserialization
- Minimal bandwidth usage
- Direct memory mapping support

### Why Event-Driven?
- Reactive strategies respond instantly
- Selective subscriptions reduce noise
- Decoupled components
- Natural fit with existing relay architecture

## Summary

The Registry System transforms AlphaPulse from a data relay pipeline into an intelligent trading platform that understands relationships between all instruments and venues. It's the foundation for:

1. **Scaling beyond arbitrage** into any trading strategy
2. **Trading across all markets** (crypto, stocks, futures, forex)
3. **Intelligent order routing** for best execution
4. **Automatic discovery** of new opportunities
5. **Institutional-grade** instrument management

All while maintaining your existing ultra-low latency architecture and adding minimal overhead (<35μs for lookups).

## Questions This Addresses

**Q: How do services know what instrument ID 0x1234ABCD refers to?**
A: Registry lookup provides full instrument details, including symbol, type, decimals, and venue information.

**Q: How do we track AAPL across NASDAQ, NYSE, and European exchanges?**
A: ISIN-based tracking links all instances as the same underlying asset.

**Q: What happens when two different instruments hash to the same ID?**
A: Collision detection alerts operators and provides fallback IDs.

**Q: How do we add new types of instruments (like options)?**
A: The type system is extensible - just add new InstrumentType variants.

**Q: Can strategies subscribe to specific types of events?**
A: Yes, pattern-based subscriptions allow filtering by asset type, venue, price changes, etc.

**Q: How does this integrate with our existing relay?**
A: Registry runs alongside relay, enriching messages with instrument metadata.