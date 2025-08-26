# Sprint 004: Mycelium Actor Runtime Implementation
*Sprint Duration: 2 weeks*
*Objective: Build location-transparent actor runtime with zero-cost in-process communication*

## Mission Statement
Transform AlphaPulse's relay-based architecture into an actor runtime that achieves **zero serialization** for same-process actors while maintaining Protocol V2's >1M msg/s performance. Focus on practical implementation over theoretical purity.

## Core Innovation
**Zero-Cost Bundling**: Actors pass `Arc<T>` when bundled in same process, automatically switching to TLV serialization only when crossing process boundaries. This eliminates ~90% of serialization overhead in typical deployments.

## Sprint Goals
1. Build practical actor runtime with typed channels for local communication
2. Implement ActorTransport abstraction hiding local vs remote complexity
3. Preserve existing TLV protocol for cross-process communication
4. Migrate one service pair as proof-of-concept
5. Achieve measurable performance improvement in bundled mode

## Task Breakdown

### 🔴 CORE RUNTIME (Week 1)

#### MYCEL-001: Actor Transport Abstraction
**Assignee**: TBD
**Priority**: CRITICAL
**Estimate**: 8 hours
**Dependencies**: None
**Files**: `libs/mycelium/src/transport.rs`

Build the adaptive transport layer:
```rust
pub struct ActorTransport {
    // Local: Zero-copy via channels
    local: Option<mpsc::Sender<Arc<dyn Any + Send + Sync>>>,
    // Remote: Existing TLV serialization
    remote: Option<UnixSocketSender>,
}

impl ActorTransport {
    async fn send<T: Message>(&self, msg: T) -> Result<()> {
        if let Some(local) = &self.local {
            // Fast path: Arc::clone() only
            local.send(Arc::new(msg) as Arc<dyn Any>).await?;
        } else if let Some(remote) = &self.remote {
            // Serialize only when necessary
            let tlv = msg.to_tlv()?;
            remote.send(tlv).await?;
        }
        Ok(())
    }
}
```

**TDD Requirements**:
```rust
#[test]
fn test_local_zero_serialization() {
    // Verify Arc is passed without serialization
}

#[test]
fn test_remote_tlv_serialization() {
    // Verify TLV used for remote
}

#[test]
fn test_transport_selection() {
    // Verify correct transport chosen
}
```

#### MYCEL-002: Message Type System
**Assignee**: TBD
**Priority**: CRITICAL
**Estimate**: 6 hours
**Dependencies**: MYCEL-001
**Files**: `libs/mycelium/src/message.rs`

Define message traits and enums for type safety:
```rust
// Avoid type proliferation with domain enums
pub enum MarketMessage {
    Swap(Arc<PoolSwapEvent>),
    Quote(Arc<QuoteUpdate>),
    OrderBook(Arc<OrderBookUpdate>),
}

pub trait Message: Send + Sync + 'static {
    fn to_tlv(&self) -> Result<Vec<u8>>;
    fn from_tlv(bytes: &[u8]) -> Result<Self>;
}
```

**Validation**:
- [ ] Type-safe local message passing
- [ ] Efficient enum dispatch
- [ ] TLV compatibility maintained

#### MYCEL-003: Actor System Core
**Assignee**: TBD
**Priority**: HIGH
**Estimate**: 12 hours
**Dependencies**: MYCEL-001, MYCEL-002
**Files**: `libs/mycelium/src/actor.rs`, `libs/mycelium/src/system.rs`

Build actor lifecycle management:
```rust
pub struct ActorSystem {
    actors: HashMap<ActorId, ActorHandle>,
    topology: TopologyConfig,  // Reuse existing
    bundles: HashMap<BundleId, ActorBundle>,
}

pub struct Actor<M: Message> {
    id: ActorId,
    mailbox: Mailbox<M>,
    transport: ActorTransport,
    state: ActorState,
}

pub trait ActorBehavior: Send + Sync {
    type Message: Message;
    async fn handle(&mut self, msg: Self::Message) -> Result<()>;
}
```

**TDD Requirements**:
```rust
#[tokio::test]
async fn test_actor_spawn_and_messaging() {
    let system = ActorSystem::new();
    let actor = system.spawn(MyActor::new()).await;
    actor.send(MyMessage { ... }).await?;
    // Verify message handled
}

#[test]
fn test_actor_lifecycle() {
    // Test spawn, stop, restart
}
```

### 🟡 BUNDLING & ROUTING (Week 1-2)

#### MYCEL-004: Bundle Configuration
**Assignee**: TBD
**Priority**: HIGH
**Estimate**: 6 hours
**Dependencies**: MYCEL-003
**Files**: `libs/mycelium/src/bundle.rs`

Implement actor bundling for zero-cost communication:
```rust
pub struct ActorBundle {
    name: String,
    actors: Vec<ActorId>,
    deployment: DeploymentMode,
}

pub enum DeploymentMode {
    SharedMemory {  // Zero serialization
        channels: HashMap<ActorId, mpsc::Sender<Arc<dyn Any>>>,
    },
    SameNode {      // Unix sockets
        sockets: HashMap<ActorId, UnixSocket>,
    },
    Distributed {   // TCP/QUIC
        connections: HashMap<ActorId, NetworkConnection>,
    },
}
```

**Configuration**:
```toml
[[bundles]]
name = "trading_core"
actors = ["market_processor", "signal_generator", "arbitrage_detector"]
mode = "shared_memory"  # Force Arc<T> passing

[[bundles]]
name = "dashboard"
actors = ["websocket_server", "metric_collector"]
mode = "same_node"  # Isolate but keep local
```

#### MYCEL-005: Actor Discovery & Routing
**Assignee**: TBD
**Priority**: MEDIUM
**Estimate**: 8 hours
**Dependencies**: MYCEL-004
**Files**: `libs/mycelium/src/discovery.rs`

Location-transparent actor references:
```rust
pub struct ActorRef<M: Message> {
    id: ActorId,
    transport: ActorTransport,
    _phantom: PhantomData<M>,
}

impl<M: Message> ActorRef<M> {
    pub async fn send(&self, msg: M) -> Result<()> {
        // Transport handles local vs remote
        self.transport.send(msg).await
    }
}

pub struct ActorRegistry {
    local: HashMap<ActorId, LocalHandle>,
    remote: HashMap<ActorId, RemoteEndpoint>,
}
```

### 🟢 MIGRATION & VALIDATION (Week 2)

#### MYCEL-006: Service Migration Wrapper
**Assignee**: TBD
**Priority**: HIGH
**Estimate**: 8 hours
**Dependencies**: MYCEL-001 through MYCEL-005
**Files**: `libs/mycelium/src/migration.rs`

Wrap existing services as actors:
```rust
// Adapter for existing relay-based services
pub struct RelayActorAdapter {
    service: Box<dyn RelayConsumer>,
    actor_transport: ActorTransport,
}

impl ActorBehavior for RelayActorAdapter {
    type Message = TLVMessage;
    
    async fn handle(&mut self, msg: TLVMessage) -> Result<()> {
        // Forward to existing service
        self.service.process_tlv(msg).await
    }
}
```

#### MYCEL-007: Proof-of-Concept Migration
**Assignee**: TBD
**Priority**: CRITICAL
**Estimate**: 12 hours
**Dependencies**: MYCEL-006
**Files**: Various service files

Migrate market_data → signal_generator pair:
- [ ] Wrap MarketDataProcessor as actor
- [ ] Wrap SignalGenerator as actor
- [ ] Bundle them with shared memory transport
- [ ] Measure performance improvement
- [ ] Document migration process

**Performance Targets**:
```rust
#[bench]
fn bench_bundled_vs_relay() {
    // Bundled: <100ns per message (Arc::clone)
    // Relay: ~35μs per message (serialization)
    // Expected: 350x improvement for bundled path
}
```

#### MYCEL-008: Performance Validation
**Assignee**: TBD
**Priority**: CRITICAL
**Estimate**: 6 hours
**Dependencies**: MYCEL-007
**Files**: `libs/mycelium/benches/`

Comprehensive benchmarks:
- [ ] Arc::clone message passing (<100ns)
- [ ] Zero allocations in steady state
- [ ] No regression in TLV performance
- [ ] Memory usage comparison
- [ ] Latency distribution analysis

### ⚪ FUTURE ENHANCEMENTS (Not This Sprint)

Document but don't implement:
- Supervision trees and failure handling
- Hot code reloading
- Automatic bundling based on communication patterns
- Distributed actor spawning
- Persistent actor state

## Definition of Done
- [ ] ActorTransport working with both local and remote modes
- [ ] Zero serialization verified for bundled actors
- [ ] One service pair successfully migrated
- [ ] Performance improvement measured and documented
- [ ] No regression in Protocol V2 metrics
- [ ] Tests demonstrate location transparency

## Success Metrics
- **Local messages**: <100ns via Arc::clone()
- **Remote messages**: <35μs via Unix socket (existing)
- **Zero allocations**: In bundled steady state
- **50% latency reduction**: For migrated service pair
- **Backward compatible**: With existing relay infrastructure

## Technical Notes

### Why This Design Works
1. **Honest about trade-offs**: Uses Arc<dyn Any> for flexibility
2. **Practical over pure**: Accepts small channel overhead for safety
3. **Incremental migration**: Services work during transition
4. **Performance focused**: Zero-cost where it matters most

### Key Decisions
- **Use channels**: 50-100ns overhead acceptable for thread safety
- **Domain enums**: Prevent type proliferation (MarketMessage, SignalMessage)
- **Arc<dyn Any>**: Runtime downcast hidden in actor runtime
- **Keep TLV**: For cross-process communication (already optimized)

### What We're NOT Doing (Yet)
- Complex supervision hierarchies
- Dynamic code reloading
- Auto-bundling based on patterns
- Distributed consensus
- These can wait - local Unix socket deployment is priority

## Risk Mitigation
- Start with non-critical service pair
- Maintain relay fallback during transition
- Automated performance regression tests
- Clear rollback procedure if issues arise

## Implementation Priority
Focus on **practical benefits** over theoretical purity:
1. Get zero-cost local messaging working
2. Prove performance improvement
3. Document migration path
4. Extend gradually to other services

**Remember**: The goal is 50-70% latency reduction for tightly coupled services, not academic perfection.