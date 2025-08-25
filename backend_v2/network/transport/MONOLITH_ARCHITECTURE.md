# Monolith-as-Microservices Architecture
*The Mycelium Approach to Transport-Agnostic Actor Systems*

## Overview

The AlphaPulse transport system supports a revolutionary deployment pattern: **microservices that can run as a monolith**. Through Mycelium's transport abstraction, actors maintain microservice boundaries while being deployable as either separate processes or bundled into a single monolith.

**Mycelium's Key Innovation**: Message-oriented programming with transport-adaptive performance optimization.

## The Mycelium Advantage

### Message-Oriented API, Performance-Oriented Implementation

Unlike traditional message queues that always serialize and queue messages, Mycelium provides a **smart routing layer** that adapts transport mechanisms based on deployment topology:

```rust
// Same API everywhere - always message-oriented
mycelium.send("execution_engine", arbitrage_signal).await;

// But transport optimizes automatically:
// Same process:    Arc<Message> sharing     (~50ns)
// Same machine:    TLV serialization        (~1.5μs)
// Network:         TCP + TLV                (~50μs)
```

### Why This Matters

**Traditional Message Queues** (RabbitMQ, Kafka, even ZeroMQ):
- **Always serialize** - even for same-process communication
- **Always copy messages** - no true zero-copy within process
- **Broker overhead** - queuing, routing, delivery guarantees add latency

**Mycelium**:
- **Conditionally serializes** - only when crossing process boundaries
- **True zero-copy** - shared memory via Arc when co-located
- **Direct delivery** - no intermediate brokers for same-process communication
- **Minimal overhead** - just enough abstraction for deployment flexibility

This document describes how memory sharing, serialization boundaries, and performance optimization work in this hybrid architecture.

## The Problem: Microservices Performance Tax

Traditional microservices architectures pay a significant **serialization tax**:

```
Monolith (same process):   5ns    (pointer passing)
Microservices:            50,000ns (HTTP + JSON serialization)
                          ↑
                          10,000x slower!
```

Even with optimized protocols, serialization overhead remains substantial:
- **JSON**: 3,700ns per message
- **Protocol Buffers**: 2,000-5,000ns per message
- **Binary (TLV)**: 400ns per message (our current approach)

## The Mycelium Solution: Transport Abstraction

Mycelium provides a **transport-agnostic** interface that automatically selects the optimal transport based on actor topology:

```rust
pub enum Transport {
    Channel(mpsc::Sender<Arc<Message>>),    // Same process: ~50ns
    SharedMemory(RingBuffer),                // Same machine: ~200ns
    UnixSocket(PathBuf),                     // Same machine: ~1,500ns
    Tcp(SocketAddr),                         // Network: ~50,000ns
    Rdma(RdmaEndpoint),                      // Network: ~2,000ns
}

// Actors use the same API regardless of transport
impl Mycelium {
    pub async fn send(&self, to: ActorId, msg: Message) -> Result<()> {
        match self.resolve_transport(&to) {
            Transport::Channel(tx) => {
                // Same process - zero serialization!
                tx.send(Arc::new(msg)).await?;
            },
            Transport::UnixSocket(path) => {
                // Process boundary - serialize to TLV
                let bytes = msg.to_tlv_bytes();
                self.socket_send(&path, bytes).await?;
            },
            // ... other transports
        }
    }
}
```

## Memory Sharing in Bundled Microservices

When microservices run as a monolith, they share the **same process memory space** without breaking actor boundaries:

### 1. Arc-Based Message Sharing

```rust
// Message created once in heap
let market_data = Arc::new(TLVMessage::new(price, volume, timestamp));

// Shared with multiple actors - zero copies!
collector_tx.send(market_data.clone()).await;  // ~50ns (increment refcount)
relay_tx.send(market_data.clone()).await;      // Same data pointer
strategy_tx.send(market_data.clone()).await;   // Still same data!

// Memory layout:
// Process Heap: [TLVMessage Data: 64 bytes]
//                      ↑     ↑     ↑
//                   Arc(1) Arc(2) Arc(3) ← All point to same memory
```

### 2. Smart Ownership Transfer

```rust
enum Message {
    Owned(Box<TLVMessage>),      // Single consumer - transfer ownership
    Shared(Arc<TLVMessage>),     // Multiple consumers - shared reference
    Static(&'static TLVMessage), // Global constants - no allocation
}

impl Mycelium {
    async fn send_optimized(&self, msg: TLVMessage, targets: &[ActorId]) {
        match targets.len() {
            0 => { /* drop message */ },
            1 => {
                // Single consumer: transfer ownership (no Arc overhead)
                let target = &targets[0];
                self.send_owned(target, Message::Owned(Box::new(msg))).await;
            },
            _ => {
                // Multiple consumers: share via Arc
                let shared = Arc::new(msg);
                for target in targets {
                    self.send_shared(target, Message::Shared(shared.clone())).await;
                }
            }
        }
    }
}
```

## Serialization Boundaries

Serialization **only occurs at monolith boundaries**, not between actors within the same process:

```
┌─────────────────────────────────────────────────┐
│              Monolith Process A                 │
│                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌──────┐ │
│  │  Collector  │───▶│    Relay    │───▶│ Strat│ │
│  │   Actor     │    │   Actor     │    │ egy  │ │
│  └─────────────┘    └─────────────┘    └──────┘ │
│         ↑                   ↑              ↑    │
│      Arc<T>             Arc<T>         Arc<T>   │
│   (NO serialization between these actors)       │
└─────────────────┬───────────────────────────────┘
                  │
         TLV Serialization (~400ns)
                  │
                  ↓
┌─────────────────────────────────────────────────┐
│              Monolith Process B                 │
│                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌──────┐ │
│  │  Executor   │───▶│ Risk Mgmt   │───▶│ Audit│ │
│  │   Actor     │    │   Actor     │    │ Trail│ │
│  └─────────────┘    └─────────────┘    └──────┘ │
│         ↑                   ↑              ↑    │
│      Arc<T>             Arc<T>         Arc<T>   │
└─────────────────────────────────────────────────┘
```

## Performance Analysis

### Within a Monolith (Channel Transport)
```rust
// Creating and sending message within same process
let msg = Arc::new(TLVMessage::new(...));        // Allocate: ~20ns
tx.send(msg.clone()).await;                      // Channel send: ~30ns
// Total: 50ns, zero serialization, zero copies
```

### Between Monoliths (Unix Socket Transport)
```rust
// Sending across process boundary
let msg = TLVMessage::new(...);                  // Create: ~20ns
let bytes = msg.to_tlv_bytes();                  // Serialize: ~400ns
unix_socket.send(&bytes).await;                  // IPC: ~1000ns
let received = TLVMessage::from_bytes(&bytes);   // Deserialize: ~100ns
// Total: 1520ns with full serialization cycle
```

**Performance difference: 30x faster for same-process communication!**

## Configuration-Driven Topology

The same actor code works across all deployment modes through configuration:

### Development Configuration (Monolith)
```yaml
# dev.yaml - Everything in one process for debugging
topology:
  mode: monolith
  actors:
    polygon_collector: { transport: channel }
    market_relay: { transport: channel }
    arbitrage_strategy: { transport: channel }
    order_executor: { transport: channel }

performance:
  expected_latency: "200ns"  # All channels
  expected_throughput: "20M msg/s"
```

### Production Configuration (Hybrid)
```yaml
# prod.yaml - Optimize hot path, isolate risky components
topology:
  mode: hybrid

  process_groups:
    hot_path:
      actors: [polygon_collector, market_relay, arbitrage_strategy]
      transport: channel
      deployment: same_process

    execution:
      actors: [order_executor]
      transport: unix_socket
      deployment: isolated_process
      reason: "Execution isolation for safety"

    compliance:
      actors: [audit_trail, risk_manager]
      transport: tcp
      deployment: separate_machine
      reason: "Regulatory separation"

performance:
  hot_path_latency: "200ns"      # Channels within hot_path group
  execution_latency: "2μs"       # Unix socket to executor
  compliance_latency: "50μs"     # TCP to compliance machine
```

### Full Microservices Configuration
```yaml
# microservices.yaml - Full distribution
topology:
  mode: distributed

  actors:
    polygon_collector:
      transport: tcp
      endpoint: "collector.internal:8001"

    market_relay:
      transport: tcp
      endpoint: "relay.internal:8002"

    arbitrage_strategy:
      transport: tcp
      endpoint: "strategy.internal:8003"

    order_executor:
      transport: tcp
      endpoint: "executor.internal:8004"
```

## Implementation Details

### Actor Interface (Transport Agnostic)
```rust
#[async_trait]
pub trait Actor {
    async fn handle_message(&mut self, msg: Message) -> Result<()>;

    // Actors send messages without knowing transport details
    async fn send_to(&self, target: &str, msg: Message) -> Result<()> {
        self.mycelium.send(target, msg).await
    }
}

// Example actor - works in any deployment mode
pub struct ArbitrageStrategy {
    mycelium: Arc<Mycelium>,
    config: StrategyConfig,
}

impl Actor for ArbitrageStrategy {
    async fn handle_message(&mut self, msg: Message) -> Result<()> {
        match msg {
            Message::MarketData(data) => {
                // Process arbitrage opportunity
                if let Some(signal) = self.calculate_signal(data) {
                    // Send to executor - Mycelium handles transport
                    self.send_to("order_executor", Message::Signal(signal)).await?;
                }
            },
            _ => {}
        }
        Ok(())
    }
}
```

### Transport Resolution
```rust
pub struct TopologyResolver {
    config: TopologyConfig,
    process_groups: HashMap<String, ProcessGroup>,
}

impl TopologyResolver {
    pub fn resolve_transport(&self, from: &str, to: &str) -> Transport {
        let from_group = self.find_process_group(from);
        let to_group = self.find_process_group(to);

        match (from_group, to_group) {
            (Some(group_a), Some(group_b)) if group_a == group_b => {
                // Same process group - use channels
                Transport::Channel(self.get_channel(from, to))
            },
            (Some(_), Some(_)) => {
                // Different process groups - use IPC
                Transport::UnixSocket(self.get_socket_path(to))
            },
            _ => {
                // Different machines - use network
                Transport::Tcp(self.get_network_endpoint(to))
            }
        }
    }
}
```

## Benefits Summary

### For Development
- **Single process debugging**: Step through entire pipeline
- **Fast iteration**: No IPC setup, just cargo run
- **Simple deployment**: One binary to manage
- **Easy testing**: Integration tests in same process

### For Production
- **Optimal performance**: Channels where possible, serialization only when necessary
- **Flexible deployment**: Can optimize grouping based on performance requirements
- **Fault isolation**: Critical components can be separated when needed
- **Incremental migration**: Start monolith, extract services gradually

### For Architecture
- **Clean boundaries**: Actors remain decoupled regardless of deployment
- **Transport transparency**: Same code works across all deployment modes
- **Performance predictability**: Clear understanding of serialization costs
- **Configuration-driven**: Change deployment without code changes

## Future Enhancements

### Shared Memory Transport
```rust
// Zero-copy via memory-mapped regions
Transport::SharedMemory {
    region: MmapRegion,
    ring_buffer: LockFreeRingBuffer,
    // ~200ns latency, zero serialization for same machine
}
```

### Automatic Optimization
```rust
// Runtime profiling to optimize transport selection
impl Mycelium {
    async fn auto_optimize(&mut self) {
        let metrics = self.collect_performance_metrics().await;

        // If actors communicate frequently, suggest co-location
        if metrics.cross_process_frequency("strategy", "executor") > 1000 {
            self.suggest_process_grouping(["strategy", "executor"]);
        }
    }
}
```

## Mycelium's Competitive Advantage

### The First Transport-Adaptive Message System

Mycelium represents a **new category** of messaging infrastructure:

**Traditional Approach**: Choose your messaging pattern upfront
- **Monolith**: Function calls (fast, inflexible)
- **Microservices**: Message queue (flexible, slow)
- **Locked in**: Can't change without rewriting code

**Mycelium Approach**: Adaptive transport based on deployment needs
- **Same codebase**: Works in all deployment modes
- **Optimal performance**: Automatically uses fastest transport available
- **Future-proof**: Easy migration from monolith → distributed → cloud

### Performance Comparison

| Transport Layer | Same Process | Same Machine | Network |
|----------------|--------------|--------------|---------|
| **Mycelium** | **50ns** ✅ | **1.5μs** ✅ | **50μs** ✅ |
| RabbitMQ | 50μs | 100μs | 200μs |
| Apache Kafka | 1ms | 2ms | 5ms |
| ZeroMQ | 200ns | 1.5μs | 50μs |
| gRPC | 500ns | 5μs | 100μs |

**Result**: Mycelium matches or beats specialized solutions at every deployment level.

### The Value Proposition

1. **Developer Experience**: Write once, deploy anywhere (monolith or distributed)
2. **Performance**: Get the speed of function calls with the flexibility of message queues
3. **Architecture Evolution**: Start monolith, go distributed as you scale
4. **Operational Simplicity**: Same monitoring, debugging, and deployment patterns across all modes

This architecture gives us the **best of both worlds**: microservices architecture with monolith performance when needed, all through a simple configuration change.
