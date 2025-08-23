# Adapter Architecture - Separation of Concerns

## Core Principle: Adapters are Stateless

Adapters are **pure data transformers** that convert external data formats into Protocol V2 TLV messages. They do NOT manage state, make trading decisions, or implement business logic.

## The Three-Layer Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    EXTERNAL SOURCES                      │
│           (Exchanges, WebSockets, RPC, APIs)            │
└──────────────────────┬──────────────────────────────────┘
                       │ Raw Data (JSON, Binary, etc.)
                       ▼
┌─────────────────────────────────────────────────────────┐
│                 LAYER 1: ADAPTERS                        │
│                  (This Directory)                        │
│                                                          │
│  Responsibility: Data Format Conversion                  │
│  - Parse external formats (JSON, protobuf, etc.)        │
│  - Validate structural integrity                         │
│  - Convert to TLV binary messages                       │
│  - Forward to relays                                    │
│                                                          │
│  What they DON'T do:                                    │
│  ❌ NO StateManager                                     │
│  ❌ NO business logic                                   │
│  ❌ NO trading decisions                                │
│  ❌ NO data aggregation                                 │
│  ❌ NO historical storage                               │
└──────────────────────┬──────────────────────────────────┘
                       │ TLV Messages (Binary Protocol V2)
                       ▼
┌─────────────────────────────────────────────────────────┐
│                  LAYER 2: RELAYS                         │
│              (Domain-Specific Routing)                   │
│                                                          │
│  Responsibility: Message Distribution & Sequencing       │
│  - Route messages to appropriate consumers              │
│  - Maintain sequence numbers                            │
│  - Detect gaps and request recovery                     │
│  - Manage consumer subscriptions                        │
│                                                          │
│  Relay Types:                                           │
│  - MarketDataRelay (TLV types 1-19)                    │
│  - SignalRelay (TLV types 20-39)                       │
│  - ExecutionRelay (TLV types 40-79)                    │
└──────────────────────┬──────────────────────────────────┘
                       │ Routed TLV Messages
                       ▼
┌─────────────────────────────────────────────────────────┐
│                 LAYER 3: CONSUMERS                       │
│            (Strategies, Portfolio, etc.)                 │
│                                                          │
│  Responsibility: Business Logic & State Management       │
│  - StateManager lives HERE                              │
│  - Maintain order books                                 │
│  - Track positions                                      │
│  - Make trading decisions                               │
│  - Generate signals                                     │
│  - Risk management                                      │
└─────────────────────────────────────────────────────────┘
```

## Why StateManager is NOT in Adapters

### 1. Single Responsibility Principle
- **Adapters**: Convert data formats (JSON → TLV)
- **StateManager**: Manages trading state and history
- Mixing these violates SRP

### 2. Scalability
- Multiple adapters can feed the same StateManager
- StateManager can aggregate data from many sources
- Adapters can be restarted without losing state

### 3. Testing
- Adapters: Test format conversion with fixtures
- StateManager: Test business logic with scenarios
- Separation enables focused, fast tests

### 4. Performance
- Adapters: Optimize for parsing speed
- StateManager: Optimize for query performance
- Different optimization goals

### 5. Reusability
- Same adapter code for backtesting and live trading
- StateManager implementation varies by use case

## Correct Architecture Examples

### ✅ CORRECT: Stateless Adapter
```rust
pub struct CoinbaseAdapter {
    // Connection and output only
    connection: Arc<ConnectionManager>,
    output_tx: Sender<TLVMessage>,
    metrics: Arc<AdapterMetrics>,
    
    // Small caches for performance (NOT business state)
    symbol_map: HashMap<String, InstrumentId>,  // OK: lookup cache
}

impl CoinbaseAdapter {
    async fn process_trade(&self, json: &str) -> Result<()> {
        // 1. Parse JSON
        let trade: Trade = serde_json::from_str(json)?;
        
        // 2. Convert to TLV
        let tlv = TradeTLV::try_from(trade)?;
        
        // 3. Send downstream
        self.output_tx.send(tlv.to_tlv_message()).await?;
        
        // That's it! No state updates, no business logic
        Ok(())
    }
}
```

### ❌ WRONG: Stateful Adapter
```rust
pub struct BadAdapter {
    // WRONG: State management in adapter
    state: Arc<StateManager>,          // ❌ NO!
    order_book: OrderBook,             // ❌ NO!
    positions: HashMap<String, Position>, // ❌ NO!
    price_history: VecDeque<Price>,   // ❌ NO!
    
    async fn process_trade(&self, trade: Trade) -> Result<()> {
        // WRONG: Business logic in adapter
        self.state.update_price(trade.symbol, trade.price); // ❌
        self.check_arbitrage_opportunity();                 // ❌
        self.update_position_pnl();                        // ❌
    }
}
```

### ✅ CORRECT: StateManager in Consumer
```rust
// In services_v2/strategies/arbitrage_strategy.rs
pub struct ArbitrageStrategy {
    // StateManager belongs in strategies/consumers
    state: Arc<StateManager>,  // ✅ Correct location
    
    // Consumes TLV messages from relays
    relay_consumer: RelayConsumer,
}

impl ArbitrageStrategy {
    async fn handle_trade(&mut self, tlv: TradeTLV) -> Result<()> {
        // Business logic belongs here
        self.state.update_price(tlv.instrument_id, tlv.price);
        
        if let Some(opportunity) = self.detect_arbitrage() {
            self.execute_trade(opportunity).await?;
        }
        
        Ok(())
    }
}
```

## Common Confusions Clarified

### Q: Where do I store the order book?
**A:** In the consumer/strategy layer, NOT in adapters. Adapters just forward order book updates as TLV messages.

### Q: How do I track which symbols are subscribed?
**A:** The relay layer tracks subscriptions. Adapters just forward all data they receive.

### Q: Where does reconnection logic go?
**A:** In ConnectionManager (used by adapters). But subscription restoration is handled by relays.

### Q: Can adapters filter messages?
**A:** Minimal filtering only (e.g., subscribed symbols). Complex filtering belongs in consumers.

### Q: Where do I implement rate limiting?
**A:** 
- **Outbound to exchange**: In adapter (RateLimiter)
- **Inbound processing**: In consumer (backpressure)

### Q: How do I aggregate data from multiple exchanges?
**A:** In the consumer layer. Each adapter sends to relays independently. Consumers aggregate.

## Implementation Checklist

When implementing a new adapter, verify:

### ✅ Adapter SHOULD Have:
- [ ] Connection management (ConnectionManager)
- [ ] Message parsing (JSON/Binary → Structs)
- [ ] Format conversion (Structs → TLV)
- [ ] Output channel (Sender<TLVMessage>)
- [ ] Metrics collection
- [ ] Basic validation (structural integrity)
- [ ] Error handling and logging

### ❌ Adapter should NOT Have:
- [ ] StateManager
- [ ] Order books
- [ ] Position tracking
- [ ] Price history
- [ ] Trading logic
- [ ] Arbitrage detection
- [ ] Risk calculations
- [ ] PnL tracking
- [ ] Signal generation

## File Organization

```
adapters/
├── src/
│   ├── input/
│   │   └── collectors/         # Data collectors (adapters)
│   │       ├── coinbase.rs     # ✅ Stateless transformer
│   │       └── binance.rs      # ✅ Stateless transformer
│   └── common/
│       ├── connection.rs       # ✅ Connection utilities
│       └── metrics.rs          # ✅ Metrics collection
│
├── ❌ NO state/ directory      # State belongs in consumers
├── ❌ NO strategies/ directory # Strategies are consumers
└── ❌ NO trading/ directory    # Trading logic is elsewhere
```

## Migration Guide

If you have existing code with StateManager in adapters:

### Step 1: Identify State Management Code
Look for:
- StateManager usage
- Order book updates
- Position tracking
- Any accumulated data

### Step 2: Move State to Consumer
1. Create a new consumer in `services_v2/strategies/`
2. Move all state management code there
3. Subscribe to relay for TLV messages

### Step 3: Simplify Adapter
1. Remove StateManager
2. Keep only parsing and conversion
3. Forward all messages to output channel

### Step 4: Test Separately
1. Test adapter with fixture data → TLV conversion
2. Test consumer with TLV messages → business logic
3. Integration test the full pipeline

## Summary

**Remember**: Adapters are dumb pipes that transform data formats. All intelligence belongs in the consumer layer.

If you find yourself adding complex logic to an adapter, STOP and ask:
- Am I transforming data formats? → OK in adapter
- Am I making decisions based on data? → Move to consumer
- Am I maintaining state across messages? → Move to consumer
- Am I implementing business rules? → Move to consumer

When in doubt: Keep adapters simple, put logic in consumers.