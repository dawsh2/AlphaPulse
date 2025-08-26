---
task_id: MYCEL-002
status: TODO
priority: CRITICAL
estimated_hours: 6
assigned_branch: feat/mycelium-message-types
assignee: TBD
created: 2025-08-26
completed: null
depends_on:
  - MYCEL-001  # Need transport layer first
blocks:
  - MYCEL-003  # Actor system depends on message types
scope:
  - "network/transport/src/actor/messages.rs"  # Message type definitions
  - "libs/types/src/common/messages.rs"  # Shared message types
---

# MYCEL-002: Message Type System

## Task Overview
**Sprint**: 004-mycelium-runtime
**Priority**: CRITICAL
**Estimate**: 6 hours
**Status**: TODO
**Dependencies**: MYCEL-001
**Goal**: Type-safe message passing without proliferation

## Problem
Need to support multiple message types per actor without:
- Creating hundreds of typed channels
- Losing type safety
- Adding significant overhead

## Solution
Use domain-specific message enums that group related messages while maintaining type safety and enabling efficient dispatch.

## Implementation

### Domain Message Enums
```rust
// Group related messages by domain to prevent proliferation
#[derive(Debug, Clone)]
pub enum MarketMessage {
    Swap(Arc<PoolSwapEvent>),
    Quote(Arc<QuoteUpdate>),
    OrderBook(Arc<OrderBookUpdate>),
    VolumeSnapshot(Arc<VolumeData>),
}

#[derive(Debug, Clone)]
pub enum SignalMessage {
    Arbitrage(Arc<ArbitrageSignal>),
    Momentum(Arc<MomentumSignal>),
    Liquidation(Arc<LiquidationSignal>),
}

#[derive(Debug, Clone)]
pub enum ExecutionMessage {
    SubmitOrder(Arc<OrderRequest>),
    CancelOrder(Arc<CancelRequest>),
    ExecutionReport(Arc<ExecutionResult>),
}
```

### Message Trait
```rust
use std::any::{Any, TypeId};

pub trait Message: Send + Sync + 'static {
    /// Convert to TLV for cross-process communication
    fn to_tlv(&self) -> Result<Vec<u8>>;
    
    /// Type ID for runtime checking
    fn message_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
    
    /// Convert to Any for local passing
    fn as_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self as Arc<dyn Any + Send + Sync>
    }
}

// Macro to reduce boilerplate
macro_rules! impl_message {
    ($type:ty, $tlv_type:expr) => {
        impl Message for $type {
            fn to_tlv(&self) -> Result<Vec<u8>> {
                let mut builder = TLVMessageBuilder::new(
                    RelayDomain::MarketData,
                    SourceType::Internal
                );
                builder.add_tlv($tlv_type, self);
                Ok(builder.build())
            }
        }
    };
}

impl_message!(PoolSwapEvent, TLVType::PoolSwap);
impl_message!(QuoteUpdate, TLVType::Quote);
impl_message!(OrderBookUpdate, TLVType::OrderBook);
```

### Type-Safe Downcasting
```rust
pub struct TypedReceiver<M: Message> {
    rx: mpsc::Receiver<Arc<dyn Any + Send + Sync>>,
    _phantom: PhantomData<M>,
}

impl<M: Message> TypedReceiver<M> {
    pub async fn recv(&mut self) -> Option<Arc<M>> {
        while let Some(any_msg) = self.rx.recv().await {
            // Try to downcast to expected type
            if let Ok(typed) = any_msg.downcast::<M>() {
                return Some(typed);
            } else {
                // Log unexpected message type
                warn!("Received unexpected message type");
            }
        }
        None
    }
}
```

### Efficient Dispatch
```rust
pub trait MessageHandler: Send + Sync {
    type Message: Message;
    
    async fn handle(&mut self, msg: Self::Message) -> Result<()>;
}

// Example actor handling multiple message types
pub struct MarketDataProcessor {
    state: MarketState,
}

impl MessageHandler for MarketDataProcessor {
    type Message = MarketMessage;
    
    async fn handle(&mut self, msg: MarketMessage) -> Result<()> {
        // Efficient enum dispatch
        match msg {
            MarketMessage::Swap(event) => self.handle_swap(event).await,
            MarketMessage::Quote(quote) => self.handle_quote(quote).await,
            MarketMessage::OrderBook(book) => self.handle_orderbook(book).await,
            MarketMessage::VolumeSnapshot(vol) => self.handle_volume(vol).await,
        }
    }
}
```

### Message Registry (Optional)
```rust
// For debugging and monitoring
pub struct MessageRegistry {
    types: HashMap<TypeId, &'static str>,
    counts: HashMap<TypeId, AtomicU64>,
}

impl MessageRegistry {
    pub fn register<M: Message>(&mut self, name: &'static str) {
        let type_id = TypeId::of::<M>();
        self.types.insert(type_id, name);
        self.counts.insert(type_id, AtomicU64::new(0));
    }
    
    pub fn record_message<M: Message>(&self) {
        if let Some(counter) = self.counts.get(&TypeId::of::<M>()) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    pub fn get_stats(&self) -> MessageStats {
        MessageStats {
            message_counts: self.counts.iter().map(|(id, count)| {
                let name = self.types.get(id).unwrap_or(&"Unknown");
                (name.to_string(), count.load(Ordering::Relaxed))
            }).collect(),
        }
    }
}
```

## TDD Test Cases

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_enum_size() {
        // Ensure enums are reasonably sized (Arc keeps them small)
        assert_eq!(std::mem::size_of::<MarketMessage>(), 16); // 8 bytes Arc + 8 bytes discriminant
        assert_eq!(std::mem::size_of::<SignalMessage>(), 16);
        assert_eq!(std::mem::size_of::<ExecutionMessage>(), 16);
    }

    #[test]
    fn test_message_to_tlv() {
        let swap = PoolSwapEvent {
            pool: [0u8; 20],
            amount_in: 1000,
            amount_out: 2000,
            timestamp_ns: 123456789,
        };
        
        let tlv = swap.to_tlv().unwrap();
        assert!(!tlv.is_empty());
        
        // Verify TLV structure
        let header = &tlv[0..32];
        assert_eq!(&header[0..4], &[0xDE, 0xAD, 0xBE, 0xEF]); // Magic
    }

    #[tokio::test]
    async fn test_typed_receiver() {
        let (tx, rx) = mpsc::channel(10);
        let mut typed_rx = TypedReceiver::<PoolSwapEvent> {
            rx,
            _phantom: PhantomData,
        };
        
        // Send correct type
        let swap = Arc::new(PoolSwapEvent { /* ... */ });
        tx.send(swap.clone() as Arc<dyn Any + Send + Sync>).await.unwrap();
        
        // Receive and verify
        let received = typed_rx.recv().await.unwrap();
        assert_eq!(*received, *swap);
    }

    #[test]
    fn test_enum_dispatch_performance() {
        let msg = MarketMessage::Swap(Arc::new(PoolSwapEvent { /* ... */ }));
        
        // Enum dispatch should be a simple jump table
        let start = Instant::now();
        for _ in 0..1_000_000 {
            match &msg {
                MarketMessage::Swap(_) => {},
                MarketMessage::Quote(_) => {},
                MarketMessage::OrderBook(_) => {},
                MarketMessage::VolumeSnapshot(_) => {},
            }
        }
        let elapsed = start.elapsed();
        
        // Should be <1ns per dispatch
        assert!(elapsed.as_nanos() / 1_000_000 < 10);
    }

    #[test]
    fn test_message_registry() {
        let mut registry = MessageRegistry::new();
        registry.register::<PoolSwapEvent>("PoolSwap");
        registry.register::<QuoteUpdate>("Quote");
        
        // Record some messages
        registry.record_message::<PoolSwapEvent>();
        registry.record_message::<PoolSwapEvent>();
        registry.record_message::<QuoteUpdate>();
        
        let stats = registry.get_stats();
        assert_eq!(stats.message_counts["PoolSwap"], 2);
        assert_eq!(stats.message_counts["Quote"], 1);
    }

    #[test]
    fn test_arc_sharing() {
        // Verify Arc enables zero-copy sharing
        let event = Arc::new(PoolSwapEvent { /* ... */ });
        let msg1 = MarketMessage::Swap(Arc::clone(&event));
        let msg2 = MarketMessage::Swap(Arc::clone(&event));
        
        // Both point to same allocation
        assert_eq!(Arc::strong_count(&event), 3);
    }
}
```

## Design Rationale

### Why Domain Enums?
- **Prevents proliferation**: One channel per domain vs one per message type
- **Cache locality**: Related messages processed together
- **Type safety**: Compile-time checking within domains
- **Efficient dispatch**: Simple jump table, no virtual calls

### Why Arc<T> in Enums?
- **Fixed size**: All enum variants same size (pointer + discriminant)
- **Zero-copy**: Multiple actors can share same message
- **Cheap cloning**: Just atomic reference count increment

### Trade-offs Accepted
- **Runtime downcast**: Required for `dyn Any`, but hidden in framework
- **Domain coupling**: Messages in same domain know about each other
- **Enum size**: 16 bytes per message (acceptable for performance gain)

## Validation Checklist
- [ ] Domain enums defined for all message categories
- [ ] Message trait implemented for all types
- [ ] TLV conversion working
- [ ] Type-safe receiving via TypedReceiver
- [ ] Efficient enum dispatch (<10ns)
- [ ] Arc sharing verified
- [ ] Registry for monitoring

## Definition of Done
- All message types organized into domain enums
- Message trait providing TLV conversion
- Type-safe receiver implementation
- Performance benchmarks passing
- Zero-copy sharing via Arc verified
- Tests demonstrate type safety