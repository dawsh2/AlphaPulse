# StateManager Architectural Issue in Adapters

## Problem Statement

Adapters currently include StateManager, which violates clean architecture principles by mixing data collection concerns with state management responsibilities.

## Current Architecture (Problematic)

```
Adapter (stateful) = ConnectionManager + MessageParser + StateManager + InvalidationLogic
```

**Issues:**
1. **Tight coupling** - Adapters become stateful when they should be pure data transformers
2. **Responsibility creep** - Parsing logic mixed with state management
3. **Testing complexity** - Hard to unit test adapters with state dependencies
4. **Scaling issues** - State management belongs at coordination level, not individual adapters

## Proposed Clean Architecture

### Responsibility Distribution

| Current StateManager Function | Proper Module Assignment | Rationale |
|------------------------------|-------------------------|-----------|
| Track monitored instruments | **Relay Consumer Registry** | Publishers shouldn't track subscribers; message buses do |
| State invalidation on disconnect | **Consumer/Strategy Level** | Only consumers know what "invalid state" means |
| Sequence number gap detection | **Protocol V2 Relay** | Message ordering is core protocol concern |
| Connection cleanup | **Connection Manager (in adapter)** | Resource lifecycle belongs with resource owner |

### Clean Pattern Implementation

#### 1. Adapter (Stateless Data Transformer)
```rust
pub struct CoinbaseCollector {
    connection: Arc<ConnectionManager>,     // ✓ Resource management
    metrics: Arc<AdapterMetrics>,          // ✓ Monitoring
    output_tx: mpsc::Sender<TLVMessage>,   // ✓ Data output
    // ❌ NO StateManager
    // ❌ NO InvalidationLogic
    // ❌ NO SubscriptionTracking
}

impl CoinbaseCollector {
    // Pure functions: Raw Data → TLV Messages
    async fn process_message(&self, raw: &str) -> Result<Option<TLVMessage>>
    
    // Connection events only
    fn emit_connection_event(&self, event: ConnectionEvent)
}
```

#### 2. Relay (Message Bus + Sequence Management)
```rust
pub struct MarketDataRelay {
    consumer_registry: ConsumerRegistry,          // Who wants what data
    sequence_tracker: SequenceTracker,           // Gap detection
    invalidation_broadcaster: InvalidationBroadcaster, // Notify consumers
}

impl MarketDataRelay {
    // Receives: TLV messages from adapters
    // Manages: Sequence numbers, consumer subscriptions
    // Emits: StateInvalidation TLV when gaps detected
}
```

#### 3. Consumer/Strategy (Business Logic)
```rust
pub struct ArbitrageStrategy {
    state_manager: StrategyStateManager,    // Application-specific state
    invalidation_handler: InvalidationHandler, // How to handle stale data
}

impl ArbitrageStrategy {
    // Receives: StateInvalidation TLV from relay
    // Action: Clear price cache, mark opportunities stale, etc.
}
```

### Event Flow Example

```
1. Adapter: "Polygon WebSocket dropped" → emit ConnectionDropped event
2. Relay: Receives event → broadcasts StateInvalidationTLV to all consumers
3. Arbitrage Strategy: Receives invalidation → clears price cache for Polygon
4. Portfolio Service: Receives invalidation → marks Polygon positions as stale
5. Dashboard: Receives invalidation → shows "Polygon data stale" indicator
```

## Implementation Plan

### Phase 1: Remove StateManager from Adapters
- [ ] Remove StateManager field from all adapters
- [ ] Remove state tracking logic from adapter constructors
- [ ] Adapters become pure: Raw Data → TLV Messages

### Phase 2: Move Subscription Tracking to Relays
- [ ] Add ConsumerRegistry to relay infrastructure
- [ ] Implement subscription management at relay level
- [ ] Route messages based on consumer subscriptions

### Phase 3: Move State Invalidation to Consumers
- [ ] Remove invalidation logic from adapters
- [ ] Add StateInvalidationTLV to protocol
- [ ] Implement invalidation handlers in strategies/consumers

### Phase 4: Move Sequence Tracking to Relays
- [ ] Add SequenceTracker to relay domains
- [ ] Implement gap detection at relay level
- [ ] Generate recovery requests for missing sequences

## Benefits

1. **Cleaner separation of concerns** - Each module has single responsibility
2. **Easier testing** - Adapters become pure functions
3. **Better scalability** - State coordination happens at proper level
4. **Reduced coupling** - Adapters don't depend on application state logic
5. **Protocol compliance** - Follows established message bus patterns

## Breaking Changes

- Adapters no longer include StateManager
- State invalidation moves from adapter lifecycle to TLV message protocol
- Subscription management moves from adapter to relay level

## Files Affected

### Remove StateManager Usage
- `src/input/collectors/coinbase.rs`
- `src/input/collectors/binance.rs` 
- `src/input/collectors/kraken.rs`
- `src/input/collectors/polygon_dex.rs`

### Add State Management to Relays
- `relays/market_data_relay.rs`
- `relays/signal_relay.rs`
- `relays/execution_relay.rs`

### Add State Invalidation to Protocol
- `protocol_v2/src/tlv/types.rs` (StateInvalidationTLV)
- Consumer strategy implementations

## Conclusion

This refactoring aligns the system with clean architecture principles and makes adapters into simple, testable data transformers while moving state management to the appropriate architectural level.