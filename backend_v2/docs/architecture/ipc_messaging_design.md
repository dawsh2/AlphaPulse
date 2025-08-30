# IPC and Messaging Architecture Discussion

## Current Architecture: Multi-Process on Single Machine

### What We Have Now

Our current deployment model runs multiple **separate processes** on a single machine, communicating via Unix domain sockets:

```bash
# Each service is a separate OS process with isolated memory
torq-polygon-adapter      # Process 1: Collects Polygon DEX data
torq-arbitrage-strategy   # Process 2: Detects arbitrage opportunities  
torq-dashboard-websocket  # Process 3: Serves dashboard clients
torq-market-data-relay    # Process 4: Routes market data messages
```

**Key Point**: Even though everything runs on one machine, these are **separate processes**, not threads. Each has its own memory space and cannot directly share memory via Arc.

### Why This Architecture?

1. **Fault Isolation**: If polygon_adapter crashes, arbitrage_strategy keeps running
2. **Resource Control**: Each process can have different CPU/memory limits
3. **Development Velocity**: Teams can develop/deploy services independently
4. **Debugging**: Easier to trace issues to specific services
5. **Future Flexibility**: Can move services to different machines without code changes

## The Abstraction Layers We Have

### 1. network/ - Transport Abstraction
Located at `backend_v2/network/`, this provides:
- TCP and Unix socket implementations
- Message envelope/framing
- Compression and security layers
- Routing logic

### 2. services_v2/messaging/ - Domain Relay System
Located at `backend_v2/services_v2/messaging/`, this provides:
- Domain-specific relays (MarketDataRelay, SignalRelay, ExecutionRelay)
- TLV message routing based on type ranges
- Consumer registration and management
- Circuit breaker patterns

### 3. libs/codec/ - Protocol Layer
The TLV codec for ultra-fast message construction/parsing:
- Zero-copy serialization for hot path
- >1M messages/second performance
- Binary protocol with 32-byte headers

## The Two Serialization Paths

### Path 1: Hot Path (TLV + Zerocopy)
For ultra-high-frequency market data and signals:

```rust
// In polygon_adapter process
let trade_tlv = TradeTLV { 
    price: 4500000000000,  // $45,000 as fixed-point
    volume: 1000000000,     // 1 ETH in wei
    // ...
};
let bytes = trade_tlv.as_bytes();  // Zero-copy view
unix_socket.send(bytes)?;          // Direct send to relay

// In arbitrage_strategy process (different memory space!)
let bytes = unix_socket.recv()?;
let trade = TradeTLV::ref_from(bytes)?;  // Zero-copy parse
```

**Why Zerocopy Here**: 
- Processes millions of messages per second
- Every microsecond matters for arbitrage detection
- Fixed, simple message structures (prices, volumes, trades)

### Path 2: Control/Config Path (Serde)
For complex state, configuration, and less frequent messages:

```rust
// Complex nested structures with Arc
#[derive(Serialize, Deserialize)]
struct StrategyConfig {
    pools: Arc<Vec<PoolConfig>>,      // Shared within process
    parameters: Arc<RiskParameters>,   // Shared between threads
    // ...
}

// Must serialize to cross process boundary
let config_json = serde_json::to_string(&config)?;
unix_socket.send(config_json.as_bytes())?;
```

**Why Serde Here**:
- Complex, nested structures
- Flexibility more important than speed
- Configuration happens rarely (not millions/second)
- Need to handle Arc<T> for thread-safety within each process

## The Arc<T> Question

### Where Arc is Used
1. **Within each process** for thread-safety:
   ```rust
   // Inside polygon_adapter process
   let shared_state = Arc::new(ConnectionState::new());
   let state1 = Arc::clone(&shared_state);
   tokio::spawn(async move { handle_websocket(state1) });
   let state2 = Arc::clone(&shared_state);
   tokio::spawn(async move { process_messages(state2) });
   ```

2. **In message definitions** for efficient cloning:
   ```rust
   pub struct PoolUpdateMessage {
       pub pool_data: Arc<PoolData>,  // Avoid cloning large data
   }
   ```

### The Serialization Problem
When these Arc-containing messages cross process boundaries, we need to serialize them:

**Option 1: serde_with** (Current approach)
```rust
#[derive(Serialize, Deserialize)]
struct Message {
    #[serde(with = "serde_with::arc")]
    data: Arc<PoolData>,
}
```

**Option 2: Custom serialize/deserialize**
```rust
impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.data.as_ref().serialize(serializer)  // Serialize inner data
    }
}
```

## Future: Modular Monolith Option

### What Would Change
To support running as a single process (true modular monolith):

```rust
// All services in one process, different threads
fn main() {
    let shared_cache = Arc::new(RwLock::new(GlobalCache::new()));
    
    // Services as threads, not processes
    thread::spawn(|| polygon_adapter::run(Arc::clone(&shared_cache)));
    thread::spawn(|| arbitrage_strategy::run(Arc::clone(&shared_cache)));
    thread::spawn(|| dashboard::run(Arc::clone(&shared_cache)));
}
```

Benefits:
- Direct memory sharing (no serialization for internal communication)
- Lower latency (no IPC overhead)
- Simpler deployment (one binary)

Drawbacks:
- Loss of fault isolation
- Harder to scale individual components
- More complex resource management

### The Abstraction Need

To support BOTH deployment modes, we'd need the messaging layer to abstract over:
1. **In-memory channels** (for monolith mode)
2. **Unix sockets** (for multi-process on same machine)
3. **TCP sockets** (for distributed across machines)

This is partially what `network/` and `services_v2/messaging/` provide, but they're currently focused on the IPC case.

## Key Decisions Needed

1. **Deployment Strategy**: Will we always be multi-process, or do we need monolith flexibility?

2. **Message Types**: Which messages need zerocopy speed vs serde flexibility?
   - Hot path (millions/sec): TLV + zerocopy
   - Control path (rare): Serde for complex types

3. **Arc Serialization**: For messages with Arc<T>:
   - Use serde_with for simplicity?
   - Custom implementations for control?
   - Redesign to avoid Arc in messages?

4. **Abstraction Level**: Should network/ fully abstract deployment mode?
   - Current: Assumes IPC/network communication
   - Future: Could abstract over in-memory too?

## Performance Implications

### Current Multi-Process overhead:
- Unix socket syscall: ~1-2 μs
- TLV serialization: ~25 ns (with zerocopy)
- Total message latency: ~2-3 μs

### Potential Monolith Performance:
- Arc clone: ~5 ns
- Channel send: ~50 ns  
- Total message latency: ~100 ns (20-30x faster)

### Trade-off:
- Is 2 μs latency acceptable for fault isolation benefits?
- Or do we need <100 ns latency for competitive advantage?

## Recommendation

1. **Keep multi-process architecture** for production (fault isolation crucial for 24/7 trading)

2. **Use zerocopy for TLV hot path** (market data, trades, simple signals)

3. **Use serde for control path** (configuration, complex state, monitoring)

4. **Design messages to minimize Arc usage** in wire protocol:
   - Use Arc within services for thread-safety
   - Serialize to plain data for IPC
   - Reconstruct Arc on receiving side if needed

5. **Consider monolith mode** only for:
   - Development/testing environments
   - Ultra-low-latency specialized deployments
   - Specific strategies requiring <1 μs response

## Questions for Discussion

1. What's our latency budget for different message types?
2. How important is fault isolation vs raw speed?
3. Should we optimize for current multi-process or future flexibility?
4. Can we redesign messages to avoid Arc in the wire protocol entirely?
5. Is the complexity of supporting both modes worth the flexibility?

## Current Status

- **network/**: Provides transport abstraction (TCP, Unix sockets)
- **messaging/**: Provides domain relays and routing
- **codec/**: Provides TLV + zerocopy for hot path
- **Arc serialization**: 24 types need Serialize/Deserialize implementation

The architecture is sound for a multi-process system. The main question is whether we need to support other deployment modes and how to handle Arc<T> serialization efficiently.

## Addendum: A Reusable Messaging Library ("Project Mycelium")

Based on further discussion, the long-term vision is to refactor the existing networking code into a single, cohesive, and reusable high-performance messaging library. This library will be named **Mycelium** and will live in `libs/mycelium`.

### Core Design: One Crate, Internal Modules

To provide the best developer experience, `Mycelium` will be a single library that exposes a simple, high-level API for messaging patterns. The complexity of the underlying transport will be an internal implementation detail.

1.  **Public API (The Patterns)**: The library's public API will provide easy-to-use messaging patterns inspired by ZeroMQ, such as `Publisher`, `Subscriber`, `Requester`, and `Replier`. Application developers will interact directly with these.

2.  **Internal Transport Module**: Internally, the crate will contain a `transport` module. This module will be responsible for the low-level work of moving bytes and will be built on these principles:
    *   **Pluggable**: A core `Transport` trait will abstract over the different ways of moving data.
    *   **Zero-Copy Aware**: The trait's API will use `bytes::Bytes` to ensure high performance and avoid unnecessary memory allocations, integrating seamlessly with the application's zero-copy TLV codec.
    *   **Implementations**: It will provide initial implementations for `TcpTransport` and `InProcessTransport` (using Tokio MPSC channels) to allow for both distributed and "modular monolith" deployments.

This design provides the best of both worlds: a simple and intuitive public API, with a clean, modular, and high-performance implementation.

```rust
// Proposed structure for libs/mycelium/src/lib.rs

// Internal transport module
mod transport;

// Public patterns module
pub mod patterns;

// Re-export for ease of use
pub use patterns::{Publisher, Subscriber};
```

### Path Forward

The path to this new architecture is a focused refactor:

1.  **Create `libs/mycelium`**: Initialize the new, unified library crate.
2.  **Build the Transport Module**: Create the internal `transport` module. Define the `Transport` trait (using `bytes::Bytes`) and migrate the generic code from `network/` to create the `TcpTransport`. Implement the `InProcessTransport`.
3.  **Build the Public API**: Implement the high-level messaging patterns (`Publisher`, etc.) in a public `patterns` module that uses the internal transport module.
4.  **Refactor Application Services**: Modify all services to remove dependencies on `network/` and `services_v2/messaging`, replacing them with a single dependency on the new `mycelium` crate. They will be updated to use the new high-level patterns for all communication.