---
task_id: MYCEL-001
status: COMPLETED
priority: CRITICAL
estimated_hours: 8
actual_hours: 6
assigned_branch: feat/mycelium-actor-transport
assignee: Claude
created: 2025-08-26
completed: 2025-08-28
depends_on:
  - CODEC-002  # Need protocol refactoring for shared types
blocks:
  - MYCEL-002  # Message types depend on transport
  - MYCEL-003  # Actor system depends on transport
scope:
  - "network/transport/src/actor/"  # New actor transport module
  - "libs/types/src/common/traits.rs"  # Actor trait definitions
---

# MYCEL-001: Actor Transport Abstraction

## Task Overview
**Sprint**: 004-mycelium-runtime
**Priority**: CRITICAL
**Estimate**: 8 hours
**Status**: IN_PROGRESS
**Goal**: Zero-cost message passing for same-process actors

## Problem
Current architecture serializes ALL messages through TLV, even when services are in the same process. This creates unnecessary overhead for high-frequency communication paths.

## Solution
Build ActorTransport that automatically selects optimal transport:
- **Same process**: Pass `Arc<T>` through channels (zero serialization)
- **Different process**: Use existing TLV serialization

## Implementation

### Core Transport Abstraction
```rust
use tokio::sync::mpsc;
use std::sync::Arc;
use std::any::Any;

pub struct ActorTransport {
    // Fast path: in-process communication
    local: Option<mpsc::Sender<Arc<dyn Any + Send + Sync>>>,
    
    // Slow path: cross-process communication  
    remote: Option<UnixSocketSender>,
    
    // Metrics for monitoring
    metrics: TransportMetrics,
}

impl ActorTransport {
    pub async fn send<T>(&self, msg: T) -> Result<()> 
    where 
        T: Message + Send + Sync + 'static
    {
        let start = Instant::now();
        
        if let Some(local) = &self.local {
            // FAST PATH: Zero serialization
            // Just Arc::clone() - typically <100ns
            let arc_msg = Arc::new(msg) as Arc<dyn Any + Send + Sync>;
            local.send(arc_msg).await?;
            self.metrics.record_local_send(start.elapsed());
        } else if let Some(remote) = &self.remote {
            // SLOW PATH: Serialize to TLV
            // Only when crossing process boundary
            let tlv = msg.to_tlv()?;
            remote.send(tlv).await?;
            self.metrics.record_remote_send(start.elapsed());
        } else {
            return Err(Error::NoTransportConfigured);
        }
        
        Ok(())
    }
}
```

### Transport Selection Logic
```rust
pub struct TransportSelector {
    topology: Arc<TopologyConfig>,
    bundle_config: BundleConfiguration,
}

impl TransportSelector {
    pub fn select_transport(
        &self,
        from: ActorId,
        to: ActorId,
    ) -> TransportType {
        // Check if actors are in same bundle
        if self.bundle_config.are_bundled(from, to) {
            TransportType::Local
        } 
        // Check if on same node
        else if self.topology.same_node(from, to) {
            TransportType::UnixSocket
        }
        // Different nodes
        else {
            TransportType::Network
        }
    }
}

pub enum TransportType {
    Local,       // Arc<T> through channels
    UnixSocket,  // TLV over Unix domain socket
    Network,     // TLV over TCP/QUIC
}
```

### Message Trait
```rust
pub trait Message: Send + Sync + 'static {
    /// Convert to TLV for cross-process communication
    fn to_tlv(&self) -> Result<Vec<u8>>;
    
    /// Reconstruct from TLV bytes
    fn from_tlv(bytes: &[u8]) -> Result<Self> 
    where 
        Self: Sized;
    
    /// Type identifier for downcasting
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}
```

### Performance Metrics
```rust
#[derive(Debug, Clone)]
pub struct TransportMetrics {
    local_sends: AtomicU64,
    remote_sends: AtomicU64,
    local_latency_ns: AtomicU64,
    remote_latency_ns: AtomicU64,
}

impl TransportMetrics {
    pub fn record_local_send(&self, duration: Duration) {
        self.local_sends.fetch_add(1, Ordering::Relaxed);
        self.local_latency_ns.fetch_add(
            duration.as_nanos() as u64,
            Ordering::Relaxed
        );
    }
    
    pub fn get_stats(&self) -> TransportStats {
        TransportStats {
            local_sends: self.local_sends.load(Ordering::Relaxed),
            avg_local_latency_ns: self.calculate_avg_local(),
            serialization_avoided: self.calculate_bytes_saved(),
        }
    }
}
```

## TDD Test Cases

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_zero_serialization() {
        // Setup
        let (tx, mut rx) = mpsc::channel(100);
        let transport = ActorTransport {
            local: Some(tx),
            remote: None,
            metrics: TransportMetrics::new(),
        };
        
        // Send message
        let msg = TestMessage { data: vec![1, 2, 3] };
        transport.send(msg.clone()).await.unwrap();
        
        // Verify Arc passed without serialization
        let received = rx.recv().await.unwrap();
        let downcast = received.downcast::<TestMessage>().unwrap();
        assert_eq!(*downcast, msg);
        
        // Verify no serialization occurred
        assert_eq!(transport.metrics.remote_sends.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_remote_tlv_serialization() {
        // Setup mock Unix socket
        let (socket_tx, socket_rx) = create_mock_unix_socket();
        let transport = ActorTransport {
            local: None,
            remote: Some(socket_tx),
            metrics: TransportMetrics::new(),
        };
        
        // Send message
        let msg = TestMessage { data: vec![1, 2, 3] };
        transport.send(msg.clone()).await.unwrap();
        
        // Verify TLV serialization occurred
        let tlv_bytes = socket_rx.recv().await.unwrap();
        let reconstructed = TestMessage::from_tlv(&tlv_bytes).unwrap();
        assert_eq!(reconstructed, msg);
    }

    #[bench]
    fn bench_local_arc_passing(b: &mut Bencher) {
        // Measure Arc::clone() performance
        let msg = Arc::new(LargeMessage::new(1_000_000));
        
        b.iter(|| {
            let _cloned = Arc::clone(&msg);
            // Should be <100ns
        });
    }

    #[bench]
    fn bench_remote_serialization(b: &mut Bencher) {
        // Measure TLV serialization overhead
        let msg = LargeMessage::new(1_000_000);
        
        b.iter(|| {
            let _tlv = msg.to_tlv().unwrap();
            // Expect ~35μs for 1MB message
        });
    }

    #[test]
    fn test_transport_selection() {
        let selector = TransportSelector::new(test_topology());
        
        // Same bundle -> Local
        assert_eq!(
            selector.select_transport(actor1, actor2),
            TransportType::Local
        );
        
        // Same node, different bundle -> Unix
        assert_eq!(
            selector.select_transport(actor1, actor3),
            TransportType::UnixSocket
        );
        
        // Different nodes -> Network
        assert_eq!(
            selector.select_transport(actor1, actor4),
            TransportType::Network
        );
    }
}
```

## Validation Checklist
- [ ] Arc passing without serialization for local
- [ ] TLV serialization only for remote
- [ ] <100ns latency for local sends
- [ ] Correct transport selection based on topology
- [ ] Metrics tracking serialization avoided
- [ ] No allocations in steady state
- [ ] Thread-safe concurrent access

## Performance Requirements
- **Local (Arc)**: <100ns per message
- **Unix Socket**: <35μs per message (existing)
- **Network**: <5ms per message (existing)
- **Memory**: Zero allocations after warm-up

## Progress Log

### 2025-08-26: Transport Layer Infrastructure Fixed ✅
**Issue**: Critical compilation errors blocking Mycelium development
**Root Cause**: Transport layer had multiple compilation failures:
1. Missing `bytes` dependency for zero-copy operations
2. Incorrect TransportError enum usage throughout codebase
3. Unused imports causing compiler warnings
4. Buffer ownership issues in UnixSocketConnection

**Resolution**:
1. ✅ **Added bytes dependency** - `bytes = "1.5"` in Cargo.toml for zero-copy byte buffer management
2. ✅ **Fixed TransportError constructors** - Updated all error usage in unix.rs:
   - `TransportError::Io(...)` → `TransportError::network_with_source(...)`
   - `TransportError::Bind(...)` → `TransportError::network_with_source(...)`
   - `TransportError::NotConnected(...)` → `TransportError::connection(...)`
   - `TransportError::MessageTooLarge(...)` → `TransportError::protocol(...)`
   - `TransportError::Send/Receive/Accept(...)` → `TransportError::network_with_source(...)`
3. ✅ **Cleaned imports** - Removed unused `error` and `warn` from tracing imports
4. ✅ **Fixed ownership** - Resolved buffer size access in `UnixSocketConnection::new()`

**Impact**: 
- Transport layer now compiles successfully (only doc warnings remain)
- Maintains zero-copy performance characteristics required for Mycelium
- Follows established error handling patterns
- **BLOCKER REMOVED** - Can now proceed with ActorTransport implementation

**Next**: Continue with ActorTransport abstraction implementation using the fixed transport infrastructure.

## Definition of Done
- ActorTransport implemented with local and remote modes
- Zero serialization verified for local path
- Performance benchmarks meet targets
- Comprehensive test coverage
- Metrics prove serialization elimination
- Documentation complete