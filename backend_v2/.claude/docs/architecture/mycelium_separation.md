# Mycelium Architecture: Three-Layer Separation

## Overview

Converting the network module into a standalone Mycelium framework with clean architectural boundaries while maintaining zero-cost abstractions through monomorphization.

## Three-Layer Architecture

```
┌─────────────────────────────────────┐
│      Torq Trading Framework         │  Layer 3: Domain
├─────────────────────────────────────┤
│       Mycelium Actor Runtime        │  Layer 2: Orchestration  
├─────────────────────────────────────┤
│     Mycelium Transport Layer        │  Layer 1: Wire Protocol
└─────────────────────────────────────┘
```

## Project Structure

### Option A: Mycelium as Monorepo (Recommended)
```
mycelium/
├── Cargo.toml                 # Workspace root
├── mycelium-transport/         # Layer 1: Transport abstractions
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs             # Transport trait definitions
│   │   ├── tcp.rs             # TCP implementation
│   │   ├── zmq.rs             # ZeroMQ implementation  
│   │   ├── unix.rs            # Unix socket implementation
│   │   └── shared_memory.rs   # Shared memory transport
│   └── tests/
├── mycelium-actors/            # Layer 2: Actor runtime
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs             # Actor system core
│   │   ├── runtime.rs         # Scheduling and lifecycle
│   │   ├── supervision.rs     # Supervision trees
│   │   ├── mailbox.rs         # Message queues
│   │   └── bundle.rs          # Actor bundling for performance
│   └── tests/
├── mycelium-derive/            # Procedural macros
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs             # #[derive(Actor)] macros
└── examples/
    ├── trading_platform.rs     # How Torq uses Mycelium
    ├── chat_server.rs          # Generic example
    └── benchmark.rs            # Performance testing

torq/
├── Cargo.toml
├── trading/                    # Layer 3: Trading domain
│   ├── src/
│   │   ├── order_manager.rs
│   │   ├── market_data.rs
│   │   └── strategy_engine.rs
└── services/
    └── src/
        └── main.rs             # Wires everything together
```

### Option B: Mycelium as Separate Crates
```
mycelium-transport/             # Standalone transport crate
├── Cargo.toml
└── src/

mycelium-actors/                # Depends on mycelium-transport
├── Cargo.toml
└── src/

torq/                           # Depends on both
├── Cargo.toml
└── src/
```

## Core Implementation

### Layer 1: Transport Abstraction (mycelium-transport)

```rust
// mycelium-transport/src/lib.rs

/// Zero-cost transport abstraction through monomorphization
pub trait Transport: Send + Sync + 'static {
    /// Associated type for connection handles
    type Connection: Send + Sync;
    
    /// Associated type for addressing
    type Address: Send + Sync + Clone;
    
    /// Error type for this transport
    type Error: std::error::Error + Send + Sync;
    
    /// Connect to an endpoint
    async fn connect(&self, addr: &Self::Address) -> Result<Self::Connection, Self::Error>;
    
    /// Send raw bytes
    async fn send(&self, conn: &Self::Connection, data: &[u8]) -> Result<(), Self::Error>;
    
    /// Receive raw bytes
    async fn recv(&self, conn: &Self::Connection) -> Result<Vec<u8>, Self::Error>;
    
    /// Fast path for local delivery (returns None if not local)
    fn local_deliver(&self, addr: &Self::Address, data: Arc<[u8]>) -> Option<()> {
        None // Default: no local optimization
    }
}

/// Concrete ZeroMQ transport
pub struct ZeroMQTransport {
    context: zmq::Context,
    // ...
}

impl Transport for ZeroMQTransport {
    type Connection = zmq::Socket;
    type Address = String;
    type Error = zmq::Error;
    
    async fn connect(&self, addr: &Self::Address) -> Result<Self::Connection, Self::Error> {
        // ZeroMQ connection logic
    }
    
    async fn send(&self, socket: &Self::Connection, data: &[u8]) -> Result<(), Self::Error> {
        socket.send(data, 0)
    }
    
    async fn recv(&self, socket: &Self::Connection) -> Result<Vec<u8>, Self::Error> {
        socket.recv_bytes(0)
    }
}

/// Shared memory transport for same-machine communication
pub struct SharedMemoryTransport {
    segments: Arc<Mutex<HashMap<usize, Arc<[u8]>>>>,
}

impl Transport for SharedMemoryTransport {
    type Connection = usize; // Segment ID
    type Address = usize;    // Process-local address
    type Error = std::io::Error;
    
    fn local_deliver(&self, addr: &Self::Address, data: Arc<[u8]>) -> Option<()> {
        // Ultra-fast local delivery
        self.segments.lock().insert(*addr, data);
        Some(())
    }
    
    // ... other methods
}
```

### Layer 2: Actor Runtime (mycelium-actors)

```rust
// mycelium-actors/src/lib.rs

use mycelium_transport::Transport;

/// Actor system parameterized by transport
/// Monomorphization ensures zero-cost abstraction
pub struct ActorSystem<T: Transport> {
    transport: T,
    actors: Arc<DashMap<ActorId, ActorState>>,
    bundles: Arc<DashMap<BundleId, Bundle>>,
}

/// Key insight: Transport is a generic parameter, not trait object
/// This enables compile-time optimization
impl<T: Transport> ActorSystem<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            actors: Arc::new(DashMap::new()),
            bundles: Arc::new(DashMap::new()),
        }
    }
    
    /// Send message - compiler inlines based on transport type
    pub async fn send(&self, target: ActorId, msg: Message) -> Result<(), Error> {
        // Fast path: local actor
        if let Some(actor) = self.actors.get(&target) {
            // No serialization, just Arc::clone
            actor.mailbox.push(Arc::new(msg));
            return Ok(());
        }
        
        // Remote path: use transport
        let addr = self.resolve_address(target)?;
        let bytes = bincode::serialize(&msg)?;
        
        // Check for local optimization first
        if let Some(()) = self.transport.local_deliver(&addr, Arc::from(bytes.as_slice())) {
            return Ok(()); // <100ns for shared memory
        }
        
        // Network path
        let conn = self.transport.connect(&addr).await?;
        self.transport.send(&conn, &bytes).await?;
        Ok(())
    }
}

/// Actor trait that users implement
#[async_trait]
pub trait Actor: Send + Sync + 'static {
    type Message: Send + Sync + 'static;
    
    async fn handle(&mut self, msg: Self::Message, ctx: &mut Context);
}

/// Bundle multiple actors for cache locality
pub struct Bundle {
    actors: Vec<Box<dyn ActorHandle>>,
    shared_memory: Arc<[u8; 64 * 1024]>, // 64KB L1 cache friendly
}
```

### Layer 3: Trading Domain (torq)

```rust
// torq/trading/src/order_manager.rs

use mycelium_actors::{Actor, Context};

/// Pure domain logic - doesn't know about transport
pub struct OrderManager {
    orders: HashMap<OrderId, Order>,
    risk_limits: RiskLimits,
}

#[async_trait]
impl Actor for OrderManager {
    type Message = OrderCommand;
    
    async fn handle(&mut self, msg: OrderCommand, ctx: &mut Context) {
        match msg {
            OrderCommand::Submit(order) => {
                // Pure business logic
                if self.validate_risk(&order) {
                    self.orders.insert(order.id, order);
                    ctx.send("ExecutionEngine", ExecuteOrder(order)).await;
                }
            }
            // ...
        }
    }
}
```

### Wiring It Together (Zero-Cost Via Monomorphization)

```rust
// torq/services/src/main.rs

use mycelium_transport::{ZeroMQTransport, SharedMemoryTransport};
use mycelium_actors::ActorSystem;

// Compile-time decision on transport
#[cfg(feature = "zeromq")]
type TransportImpl = ZeroMQTransport;

#[cfg(feature = "shared-memory")]
type TransportImpl = SharedMemoryTransport;

#[tokio::main]
async fn main() {
    // Monomorphization happens here!
    // ActorSystem<ZeroMQTransport> is a completely different type
    // from ActorSystem<SharedMemoryTransport> at compile time
    let transport = TransportImpl::new();
    let system = ActorSystem::new(transport);
    
    // Spawn domain actors
    let order_mgr = OrderManager::new();
    system.spawn(order_mgr, "OrderManager").await;
    
    // The compiler generates specialized code for each transport
    // No vtables, no dynamic dispatch, just direct calls
}
```

## Monomorphization Deep Dive

### How It Works

```rust
// This generic function...
impl<T: Transport> ActorSystem<T> {
    pub async fn send(&self, target: ActorId, msg: Message) {
        self.transport.send(...).await
    }
}

// ...becomes TWO separate functions after monomorphization:

// Generated for ActorSystem<ZeroMQTransport>
pub async fn send_zeromq(&self, target: ActorId, msg: Message) {
    ZeroMQTransport::send(...).await  // Direct call, no indirection
}

// Generated for ActorSystem<SharedMemoryTransport>  
pub async fn send_sharedmem(&self, target: ActorId, msg: Message) {
    SharedMemoryTransport::send(...).await  // Direct call, inlined
}
```

### Performance Impact

```rust
// With trait objects (dynamic dispatch)
let transport: Box<dyn Transport> = Box::new(ZeroMQTransport);
transport.send(...).await  // Virtual call through vtable: ~3-5ns overhead

// With monomorphization (static dispatch)
let transport = ZeroMQTransport;
transport.send(...).await  // Direct call, often inlined: 0ns overhead
```

## Mycelium Repository Decision

### Recommendation: Single Mycelium Repository with Both Layers

**Why:**
1. **Cohesion**: Actor runtime and transport are designed together for performance
2. **Optimization**: Can optimize across boundaries (e.g., bundle placement affects transport)
3. **Versioning**: Keep compatible versions together
4. **Documentation**: Single place to understand the system
5. **Examples**: Show complete working systems

**Structure:**
```
github.com/torq/mycelium/
├── mycelium-transport/    # Can be used standalone
├── mycelium-actors/       # Depends on transport
├── mycelium/              # Re-exports both with presets
└── examples/              # Complete examples
```

**Usage in Torq:**
```toml
# Cargo.toml
[dependencies]
mycelium = { git = "https://github.com/torq/mycelium" }
# Or use individual crates:
# mycelium-transport = { git = "...", features = ["zeromq"] }
# mycelium-actors = { git = "..." }
```

## Migration Path

1. **Phase 1**: Extract network/ to mycelium repo as-is
2. **Phase 2**: Refactor into transport/actors separation  
3. **Phase 3**: Add generic parameters for monomorphization
4. **Phase 4**: Update Torq to use Mycelium
5. **Phase 5**: Add alternative transports (ZeroMQ, QUIC)

## Key Design Principles

1. **Zero-cost abstractions**: Monomorphization eliminates runtime overhead
2. **Local optimization**: Fast path for same-process communication
3. **Clean layers**: Each layer can be understood independently
4. **Flexible composition**: Mix and match transports and actor systems
5. **Performance first**: <100ns local, <35μs IPC, <5ms network

## Testing Strategy

```rust
// Test with different transports using generics
async fn test_actor_communication<T: Transport>(transport: T) {
    let system = ActorSystem::new(transport);
    // Test code works with ANY transport
}

#[tokio::test]
async fn test_with_mock() {
    test_actor_communication(MockTransport::new()).await;
}

#[tokio::test]
async fn test_with_zeromq() {
    test_actor_communication(ZeroMQTransport::new()).await;
}
```

This architecture gives you maximum flexibility with zero performance cost!