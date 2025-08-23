# AlphaPulse State Management Library

A flexible, high-performance state management framework implementing the GEMINI-2 "Core + Libraries" architecture. This library provides both embedded (microsecond latency) and shared service deployment options for managing trading system state.

## Architecture Overview

The state library follows a three-tier design:

```
libs/state/
├── core/        # Generic traits and abstractions
├── market/      # Market data state (pools, order books)
├── execution/   # Order and execution state
└── portfolio/   # Position and risk state
```

### Core Principles

1. **Dual Deployment Model**: State can be embedded directly in services for microsecond latency, or run as a shared service accessed via IPC
2. **Domain Separation**: Each domain (market, execution, portfolio) has its own specialized implementation
3. **Event-Driven**: All state changes are driven by typed events
4. **Snapshot/Recovery**: Built-in support for state persistence and recovery

## Core Traits

### Stateful

The foundational trait for all state managers:

```rust
pub trait Stateful {
    type Event;
    type Error;
    
    fn apply_event(&mut self, event: Self::Event) -> Result<(), Self::Error>;
    fn snapshot(&self) -> Vec<u8>;
    fn restore(&mut self, snapshot: &[u8]) -> Result<(), Self::Error>;
}
```

### SequencedStateful

Extension for state managers that track event sequences:

```rust
pub trait SequencedStateful: Stateful {
    fn apply_sequenced(&mut self, seq: u64, event: Self::Event) -> Result<(), Self::Error>;
    fn last_sequence(&self) -> u64;
}
```

## Usage Patterns

### Embedded Mode (Microsecond Latency)

Strategy embeds state manager directly for fastest possible access:

```rust
use alphapulse_state_market::{PoolStateManager, PoolEvent};
use alphapulse_state_core::Stateful;

struct ArbitrageStrategy {
    pool_state: PoolStateManager,
}

impl ArbitrageStrategy {
    async fn process_event(&mut self, event: PoolEvent) {
        // Direct, zero-copy access to state
        self.pool_state.apply_event(event).unwrap();
        
        // Check for arbitrage opportunities
        let opportunities = self.pool_state.find_arbitrage_pairs();
        // ... execute trades
    }
}
```

### Service Mode (Shared State)

State manager runs as a service, multiple consumers access via IPC:

```rust
use alphapulse_state_market::PoolStateManager;

struct StateService {
    pool_state: PoolStateManager,
    ipc_server: UnixListener,
}

impl StateService {
    async fn run(&mut self) {
        // Process events from relay
        // Serve state queries via IPC
        // Provide snapshots for recovery
    }
}
```

## Market State (Currently Implemented)

The market state library provides comprehensive DEX pool state management:

### Features
- **Multi-Protocol Support**: Uniswap V2, V3, and other AMMs
- **Dynamic Pool Discovery**: Automatically tracks new pools as they appear
- **Arbitrage Detection**: Built-in cross-pool arbitrage opportunity detection
- **Token Indexing**: Fast lookup of all pools containing specific tokens

### Example Usage

```rust
use alphapulse_state_market::{PoolStateManager, PoolEvent};
use alphapulse_protocol::tlv::PoolSyncTLV;

let mut manager = PoolStateManager::new();

// Process a V2 sync event
let sync = PoolSyncTLV {
    pool_id: pool_id,
    reserve0: 1000_00000000, // 1000 tokens (8 decimals)
    reserve1: 2000_00000000,
    timestamp_ns: 1234567890,
    block_number: 100,
};

manager.apply_event(PoolEvent::Sync(sync))?;

// Find arbitrage opportunities
let opportunities = manager.find_arbitrage_pairs();
for opp in opportunities {
    println!("Arbitrage: {:.2}% spread between pools", opp.spread_pct);
}
```

## Execution State (Planned)

Future implementation for order and execution tracking:

- Order lifecycle management
- Fill aggregation
- Execution quality metrics
- Local order book maintenance

## Portfolio State (Planned)

Future implementation for position and risk management:

- Multi-venue position tracking
- Real-time P&L calculation
- Risk metrics (VaR, exposure limits)
- Capital allocation tracking

## Performance Characteristics

### Embedded Mode
- **Latency**: <1 microsecond for state queries
- **Throughput**: >1M events/second
- **Memory**: Direct access, zero-copy operations

### Service Mode
- **Latency**: ~10-50 microseconds (Unix socket IPC)
- **Throughput**: >100K events/second
- **Memory**: Shared across consumers

## Migration from Old Architecture

If you're migrating from the old `services_v2/pool_state`:

1. **Update imports**:
```rust
// Old
use pool_state::PoolStateManager;

// New
use alphapulse_state_market::PoolStateManager;
```

2. **Add state library dependency**:
```toml
[dependencies]
alphapulse-state-market = { path = "../../libs/state/market" }
```

3. **Use Stateful trait for generic operations**:
```rust
use alphapulse_state_core::Stateful;

fn snapshot_any_state<S: Stateful>(state: &S) -> Vec<u8> {
    state.snapshot()
}
```

## Design Decisions

### Why Separate Core from Implementations?

- **DRY Principle**: Complex state logic (sequences, snapshots) written once
- **Type Safety**: Each domain defines its own event and error types
- **Independence**: Teams can own their domain without affecting others
- **Performance**: No overhead from unused domains

### Why Both Embedded and Service Modes?

- **Embedded**: Critical strategies need microsecond latency
- **Service**: Dashboards and monitoring need shared consistent view
- **Flexibility**: Can start embedded, move to service as needed

### Why Event-Driven?

- **Audit Trail**: Every state change is traceable
- **Replay**: Can reconstruct state from event history
- **Testing**: Easy to test with specific event sequences
- **Distribution**: Events can be broadcast to multiple consumers

## Future Enhancements

1. **State Synchronization**: Multi-node state replication
2. **Time-Travel Queries**: Query historical state at any point
3. **Incremental Snapshots**: Delta compression for efficiency
4. **State Sharding**: Distribute state across nodes by instrument
5. **WebAssembly Support**: Run state logic in browser/edge

## Contributing

When adding new state domains:

1. Create new directory under `libs/state/`
2. Implement `Stateful` trait for your domain
3. Add domain-specific event types
4. Include comprehensive tests
5. Document usage patterns

## License

Part of the AlphaPulse trading system.