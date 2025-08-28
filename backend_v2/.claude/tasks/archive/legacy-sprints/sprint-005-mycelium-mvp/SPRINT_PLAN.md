# Sprint 005: Mycelium Phase 1 - Minimum Viable Product
*Sprint Duration: 3-4 weeks*
*Objective: Build MVP actor runtime for bundled monolith with zero-cost local communication*

## Mission Statement
Implement Phase 1 of Mycelium: Core abstractions and LocalTransport only. Achieve zero-cost communication for the hot path by refactoring MarketDataProcessor and SignalGenerator into actors. Defer IPC and distribution to later phases.

## Scope Definition
**IN SCOPE (Phase 1 MVP):**
- Core actor abstractions (Actor trait, ActorRef, ActorMessage)
- LocalTransport using tokio::mpsc channels
- Basic ActorSystem for single-process deployment
- Migration of 2 critical services as proof
- Shared message types in libs/types

**OUT OF SCOPE (Future Phases):**
- UnixSocketTransport (Phase 2)
- TcpTransport (Phase 3)
- Full supervision trees (Phase 2+)
- Dynamic code reloading (Phase 3)
- TOML configuration (Phase 2)
- Procedural macros (Optional enhancement)

## Architecture Components

### Component 1: Core Abstractions (Medium Complexity)
Foundation requiring careful Rust generics and trait design.

### Component 2: LocalTransport Only (Low Complexity)
Just in-memory channels for Phase 1.

### Component 3: Basic Runtime (Medium Complexity)
Simplified runtime without supervision for MVP.

### Component 4: Manual Configuration (Low Complexity)
Hardcoded configuration for MVP, TOML comes in Phase 2.

## Task Breakdown

### 🔴 Week 1: Core Abstractions & Types

#### MVP-001: Shared Message Types Migration
**Priority**: CRITICAL
**Estimate**: 6 hours
**Status**: TODO
**Files**: `libs/types/src/messages.rs`

Move protocol message definitions to shared library:
```rust
// libs/types/src/messages.rs
// Shared message types used across services

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolSwapEvent {
    pub pool: [u8; 20],
    pub amount0_in: i64,
    pub amount1_out: i64,
    pub timestamp_ns: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageSignal {
    pub opportunity_id: u64,
    pub profit_usd: f64,
    pub pools: Vec<[u8; 20]>,
    pub timestamp_ns: u64,
}

// Domain message enums
pub enum MarketMessage {
    Swap(Arc<PoolSwapEvent>),
    Quote(Arc<QuoteUpdate>),
    OrderBook(Arc<OrderBookSnapshot>),
}

pub enum SignalMessage {
    Arbitrage(Arc<ArbitrageSignal>),
    Momentum(Arc<MomentumSignal>),
}
```

**Validation**:
- [ ] All shared types moved to libs/types
- [ ] No circular dependencies
- [ ] Serialization traits implemented
- [ ] Tests for each message type

#### MVP-002: Actor Core Trait Design
**Priority**: CRITICAL
**Estimate**: 8 hours
**Status**: TODO
**Files**: `libs/mycelium/src/actor.rs`

Design the foundational Actor trait:
```rust
#[async_trait]
pub trait Actor: Send + Sync + 'static {
    type Message: Send + Sync + 'static;
    
    async fn handle(&mut self, msg: Self::Message) -> Result<()>;
    
    async fn on_start(&mut self) -> Result<()> {
        Ok(())
    }
    
    async fn on_stop(&mut self) -> Result<()> {
        Ok(())
    }
}
```

**TDD Requirements**:
```rust
#[test]
fn test_actor_trait_object_safety() {
    // Verify trait can be used as trait object
}

#[test]
fn test_message_type_constraints() {
    // Verify Send + Sync requirements
}
```

#### MVP-003: ActorMessage Wrapper
**Priority**: CRITICAL
**Estimate**: 4 hours
**Status**: TODO
**Files**: `libs/mycelium/src/message.rs`

Create the wrapper for local vs serialized:
```rust
pub enum ActorMessage {
    Local(Arc<dyn Any + Send + Sync>),  // Zero-copy for same-process
    Serialized(Vec<u8>),                 // For future IPC support
}

impl ActorMessage {
    pub fn local<T: Send + Sync + 'static>(msg: T) -> Self {
        ActorMessage::Local(Arc::new(msg))
    }
    
    pub fn downcast<T: 'static>(&self) -> Option<Arc<T>> {
        match self {
            ActorMessage::Local(any) => {
                any.clone().downcast::<T>().ok()
            }
            _ => None,
        }
    }
}
```

#### MVP-004: ActorRef Implementation
**Priority**: HIGH
**Estimate**: 6 hours
**Status**: TODO
**Dependencies**: MVP-002, MVP-003
**Files**: `libs/mycelium/src/actor_ref.rs`

Location-transparent actor reference:
```rust
pub struct ActorRef<M> {
    id: ActorId,
    sender: mpsc::Sender<ActorMessage>,
    _phantom: PhantomData<M>,
}

impl<M: Send + Sync + 'static> ActorRef<M> {
    pub async fn send(&self, msg: M) -> Result<()> {
        let wrapped = ActorMessage::local(msg);
        self.sender.send(wrapped).await
            .map_err(|_| Error::ActorStopped)
    }
}
```

### 🟡 Week 2: LocalTransport & Basic Runtime

#### MVP-005: LocalTransport Implementation
**Priority**: HIGH
**Estimate**: 4 hours
**Status**: TODO
**Dependencies**: MVP-003
**Files**: `libs/mycelium/src/transport/local.rs`

In-memory transport using channels:
```rust
pub struct LocalTransport {
    sender: mpsc::Sender<ActorMessage>,
    receiver: mpsc::Receiver<ActorMessage>,
}

impl Transport for LocalTransport {
    async fn send(&self, msg: ActorMessage) -> Result<()> {
        self.sender.send(msg).await
            .map_err(|_| Error::TransportClosed)
    }
    
    async fn recv(&mut self) -> Option<ActorMessage> {
        self.receiver.recv().await
    }
}
```

**Performance Target**: <100ns for Arc::clone() and send

#### MVP-006: Basic ActorSystem
**Priority**: HIGH
**Estimate**: 12 hours
**Status**: TODO
**Dependencies**: MVP-001 through MVP-005
**Files**: `libs/mycelium/src/system.rs`

Simplified runtime for single process:
```rust
pub struct ActorSystem {
    actors: Arc<RwLock<HashMap<ActorId, ActorHandle>>>,
    registry: Arc<ActorRegistry>,
}

impl ActorSystem {
    pub async fn spawn<A: Actor>(&self, actor: A) -> ActorRef<A::Message> {
        let id = ActorId::new();
        let (tx, rx) = mpsc::channel(1000);
        
        // Create actor task
        let task = ActorTask {
            actor: Box::new(actor),
            receiver: rx,
            id: id.clone(),
        };
        
        // Spawn task
        tokio::spawn(task.run());
        
        // Return reference
        ActorRef {
            id,
            sender: tx,
            _phantom: PhantomData,
        }
    }
}
```

#### MVP-007: Message Dispatch Loop
**Priority**: HIGH
**Estimate**: 8 hours
**Status**: TODO
**Dependencies**: MVP-006
**Files**: `libs/mycelium/src/runtime.rs`

Core message processing loop:
```rust
struct ActorTask<A: Actor> {
    actor: A,
    receiver: mpsc::Receiver<ActorMessage>,
    id: ActorId,
}

impl<A: Actor> ActorTask<A> {
    async fn run(mut self) {
        // Lifecycle: Start
        if let Err(e) = self.actor.on_start().await {
            error!("Actor {} failed to start: {}", self.id, e);
            return;
        }
        
        // Message loop
        while let Some(msg) = self.receiver.recv().await {
            if let Some(typed) = msg.downcast::<A::Message>() {
                if let Err(e) = self.actor.handle(*typed).await {
                    error!("Actor {} handle error: {}", self.id, e);
                    // Simple error handling for MVP
                    break;
                }
            }
        }
        
        // Lifecycle: Stop
        let _ = self.actor.on_stop().await;
    }
}
```

#### MVP-008: Basic Mailbox
**Priority**: MEDIUM
**Estimate**: 6 hours
**Status**: TODO
**Dependencies**: MVP-007
**Files**: `libs/mycelium/src/mailbox.rs`

Simple mailbox without priorities for MVP:
```rust
pub struct Mailbox {
    sender: mpsc::Sender<ActorMessage>,
    capacity: usize,
    dropped: AtomicU64,
}

impl Mailbox {
    pub fn new(capacity: usize) -> (Self, mpsc::Receiver<ActorMessage>) {
        let (tx, rx) = mpsc::channel(capacity);
        (
            Self {
                sender: tx,
                capacity,
                dropped: AtomicU64::new(0),
            },
            rx
        )
    }
    
    pub async fn send(&self, msg: ActorMessage) -> Result<()> {
        self.sender.send(msg).await
            .map_err(|_| {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                Error::MailboxFull
            })
    }
}
```

### 🟢 Week 3-4: Service Migration & Validation

#### MVP-009: MarketDataProcessor Actor
**Priority**: CRITICAL
**Estimate**: 8 hours
**Status**: TODO
**Dependencies**: MVP-001 through MVP-008
**Files**: `services_v2/actors/market_data_actor.rs`

Refactor MarketDataProcessor as actor:
```rust
use libs::types::messages::{MarketMessage, SignalMessage};

pub struct MarketDataActor {
    state: MarketState,
    signal_ref: ActorRef<SignalMessage>,
}

#[async_trait]
impl Actor for MarketDataActor {
    type Message = MarketMessage;
    
    async fn handle(&mut self, msg: MarketMessage) -> Result<()> {
        match msg {
            MarketMessage::Swap(event) => {
                // Process swap
                self.state.update_pool(&event);
                
                // Check for signals
                if let Some(signal) = self.detect_arbitrage(&event) {
                    // Zero-cost send to signal actor!
                    self.signal_ref.send(
                        SignalMessage::Arbitrage(Arc::new(signal))
                    ).await?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
```

#### MVP-010: SignalGenerator Actor
**Priority**: CRITICAL
**Estimate**: 8 hours
**Status**: TODO
**Dependencies**: MVP-009
**Files**: `services_v2/actors/signal_generator_actor.rs`

Refactor SignalGenerator as actor:
```rust
pub struct SignalGeneratorActor {
    state: SignalState,
    config: SignalConfig,
}

#[async_trait]
impl Actor for SignalGeneratorActor {
    type Message = SignalMessage;
    
    async fn handle(&mut self, msg: SignalMessage) -> Result<()> {
        match msg {
            SignalMessage::Arbitrage(signal) => {
                // Validate signal
                if self.validate_signal(&signal) {
                    // Process for execution
                    self.prepare_execution(&signal).await?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
```

#### MVP-011: Integration Testing
**Priority**: HIGH
**Estimate**: 12 hours
**Status**: TODO
**Dependencies**: MVP-009, MVP-010
**Files**: `tests/mycelium_integration.rs`

End-to-end testing of actor system:
```rust
#[tokio::test]
async fn test_market_to_signal_zero_cost() {
    let system = ActorSystem::new();
    
    // Spawn signal actor
    let signal_actor = SignalGeneratorActor::new();
    let signal_ref = system.spawn(signal_actor).await?;
    
    // Spawn market actor with signal reference
    let market_actor = MarketDataActor::new(signal_ref.clone());
    let market_ref = system.spawn(market_actor).await?;
    
    // Send swap event
    let swap = PoolSwapEvent { /* ... */ };
    let start = Instant::now();
    market_ref.send(MarketMessage::Swap(Arc::new(swap))).await?;
    let elapsed = start.elapsed();
    
    // Verify zero-cost communication
    assert!(elapsed.as_nanos() < 1000); // <1μs including processing
}
```

#### MVP-012: Performance Benchmarking
**Priority**: CRITICAL
**Estimate**: 6 hours
**Status**: TODO
**Dependencies**: MVP-011
**Files**: `benches/mycelium_performance.rs`

Comprehensive performance validation:
```rust
#[bench]
fn bench_local_message_passing(b: &mut Bencher) {
    // Measure Arc::clone() + channel send
    // Target: <100ns
}

#[bench]
fn bench_actor_throughput(b: &mut Bencher) {
    // Messages per second
    // Target: >1M msg/s
}

#[bench]
fn bench_memory_overhead(b: &mut Bencher) {
    // Memory per actor
    // Target: <1KB base overhead
}
```

## Definition of Done

### Phase 1 MVP Complete When:
- [ ] Core actor abstractions implemented and tested
- [ ] LocalTransport working with Arc passing
- [ ] Basic ActorSystem can spawn and manage actors
- [ ] MarketDataProcessor refactored as actor
- [ ] SignalGenerator refactored as actor
- [ ] Zero-cost communication verified (<100ns)
- [ ] Integration tests passing
- [ ] Performance benchmarks meet targets
- [ ] No regression in existing functionality

## Success Metrics
- **Message Latency**: <100ns for local actor communication
- **Throughput**: >1M messages/second
- **Memory**: <1KB overhead per actor
- **Performance Gain**: 50%+ reduction in hot path latency
- **Code Simplicity**: Reduced coupling between services

## Risk Mitigation
1. **Incremental Migration**: Keep relay system running in parallel
2. **Performance Gates**: Automated benchmarks prevent regression
3. **Rollback Plan**: Can revert to relay with configuration change
4. **Limited Scope**: Only 2 services in Phase 1, proven before expanding

## Next Phases Preview

### Phase 2 (4-6 weeks): Production-Ready IPC
- UnixSocketTransport implementation
- Robust supervision and restart strategies
- TOML configuration for bundles
- Full relay infrastructure replacement

### Phase 3 (4+ weeks): Full Distribution
- TcpTransport for network communication
- Complete supervision hierarchies
- Advanced backpressure strategies
- Dynamic code reloading

## Notes
This is Phase 1 MVP only - we're building the minimum to prove value. The focus is on getting zero-cost local communication working for the hot path. All other features are explicitly deferred to keep scope manageable.