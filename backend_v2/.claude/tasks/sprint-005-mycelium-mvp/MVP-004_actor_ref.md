# MVP-004: ActorRef Implementation

## Task Overview
**Sprint**: 005-mycelium-mvp
**Priority**: HIGH
**Estimate**: 6 hours
**Status**: TODO
**Dependencies**: MVP-002, MVP-003
**Goal**: Implement location-transparent actor reference that hides transport complexity

## Problem
ActorRef must provide a uniform interface for sending messages regardless of whether the target actor is:
- In the same process (use Arc<T> through channels)
- In a different process (use serialization - Phase 2)
- On a different node (use network - Phase 3)

For MVP, we only need local support but the design must accommodate future transports.

## Implementation

### ActorRef Core
```rust
// libs/mycelium/src/actor_ref.rs

use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use crate::message::ActorMessage;
use crate::actor::{ActorError, ActorId};

/// Reference to an actor that can receive messages of type M
#[derive(Clone)]
pub struct ActorRef<M> {
    /// Unique actor identifier
    id: ActorId,
    
    /// Transport for sending messages
    transport: Transport,
    
    /// Phantom type for message type safety
    _phantom: PhantomData<M>,
}

impl<M> ActorRef<M> 
where 
    M: Send + Sync + 'static
{
    /// Create a new local actor reference
    pub(crate) fn local(id: ActorId, sender: mpsc::Sender<ActorMessage>) -> Self {
        Self {
            id,
            transport: Transport::Local(sender),
            _phantom: PhantomData,
        }
    }
    
    /// Send a message to this actor (fire-and-forget)
    pub async fn send(&self, msg: M) -> Result<(), ActorError> {
        let message = ActorMessage::local(msg);
        
        match &self.transport {
            Transport::Local(sender) => {
                sender.send(message).await
                    .map_err(|_| ActorError::ActorStopped)?;
            }
            Transport::Remote(_endpoint) => {
                // Phase 2: Serialize and send over IPC
                return Err(ActorError::Custom(
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Remote transport not implemented in MVP"
                    ))
                ));
            }
        }
        
        Ok(())
    }
    
    /// Send a message and wait for response (request-response pattern)
    pub async fn ask<R>(&self, msg: M) -> Result<R, ActorError> 
    where
        R: Send + 'static
    {
        // For MVP, we'll use a simple oneshot channel approach
        // This requires the message type to support embedding a response channel
        todo!("Implement ask pattern in next iteration")
    }
    
    /// Try to send without waiting (non-blocking)
    pub fn try_send(&self, msg: M) -> Result<(), ActorError> {
        let message = ActorMessage::local(msg);
        
        match &self.transport {
            Transport::Local(sender) => {
                sender.try_send(message)
                    .map_err(|e| match e {
                        mpsc::error::TrySendError::Full(_) => ActorError::MailboxFull,
                        mpsc::error::TrySendError::Closed(_) => ActorError::ActorStopped,
                    })?;
            }
            Transport::Remote(_) => {
                return Err(ActorError::Custom(
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Remote transport not implemented in MVP"
                    ))
                ));
            }
        }
        
        Ok(())
    }
    
    /// Get the actor's unique identifier
    pub fn id(&self) -> &ActorId {
        &self.id
    }
    
    /// Check if the referenced actor is local to this process
    pub fn is_local(&self) -> bool {
        matches!(self.transport, Transport::Local(_))
    }
}

/// Transport mechanism for message delivery
#[derive(Clone)]
enum Transport {
    /// Local in-process transport using channels
    Local(mpsc::Sender<ActorMessage>),
    
    /// Remote transport (Phase 2+)
    Remote(RemoteEndpoint),
}

/// Placeholder for remote endpoint (Phase 2)
#[derive(Clone)]
struct RemoteEndpoint {
    #[allow(dead_code)]
    address: String,
}
```

### Typed ActorRef Extensions
```rust
/// Extension trait for ActorRef with specific message types
pub trait ActorRefExt<M> {
    /// Send message with priority (Phase 2)
    async fn send_with_priority(&self, msg: M, priority: Priority) -> Result<(), ActorError>;
    
    /// Send with timeout
    async fn send_timeout(&self, msg: M, timeout: Duration) -> Result<(), ActorError>;
    
    /// Batch send multiple messages
    async fn send_batch(&self, messages: Vec<M>) -> Result<(), ActorError>;
}

impl<M> ActorRefExt<M> for ActorRef<M>
where
    M: Send + Sync + 'static
{
    async fn send_with_priority(&self, msg: M, _priority: Priority) -> Result<(), ActorError> {
        // For MVP, ignore priority and just send
        self.send(msg).await
    }
    
    async fn send_timeout(&self, msg: M, timeout: Duration) -> Result<(), ActorError> {
        tokio::time::timeout(timeout, self.send(msg))
            .await
            .map_err(|_| ActorError::Custom(
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Send timeout"
                ))
            ))?
    }
    
    async fn send_batch(&self, messages: Vec<M>) -> Result<(), ActorError> {
        for msg in messages {
            self.send(msg).await?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Priority {
    Low,
    Normal,
    High,
}
```

### ActorRef for Request-Response
```rust
/// Special message wrapper for request-response pattern
pub trait Request {
    type Response: Send + 'static;
}

/// Wrapper that includes response channel
pub struct RequestEnvelope<R: Request> {
    pub request: R,
    pub reply_to: oneshot::Sender<R::Response>,
}

impl<M> ActorRef<M>
where
    M: Send + Sync + 'static
{
    /// Send request and await response
    pub async fn request<R>(&self, request: R) -> Result<R::Response, ActorError>
    where
        R: Request + Send + 'static,
        M: From<RequestEnvelope<R>>,
    {
        let (tx, rx) = oneshot::channel();
        
        let envelope = RequestEnvelope {
            request,
            reply_to: tx,
        };
        
        self.send(envelope.into()).await?;
        
        rx.await.map_err(|_| ActorError::Custom(
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Response channel closed"
            ))
        ))
    }
}
```

### Usage Example
```rust
// Example: Market data actor sending to signal generator
pub struct MarketDataActor {
    signal_ref: ActorRef<SignalMessage>,
}

impl MarketDataActor {
    async fn process_swap(&mut self, event: PoolSwapEvent) -> Result<(), ActorError> {
        // Detect arbitrage opportunity
        if let Some(signal) = self.detect_arbitrage(&event) {
            // Zero-cost send to local actor!
            self.signal_ref.send(
                SignalMessage::Arbitrage(Arc::new(signal))
            ).await?;
        }
        Ok(())
    }
    
    async fn process_batch(&mut self, events: Vec<PoolSwapEvent>) -> Result<(), ActorError> {
        let signals: Vec<SignalMessage> = events
            .iter()
            .filter_map(|e| self.detect_arbitrage(e))
            .map(|s| SignalMessage::Arbitrage(Arc::new(s)))
            .collect();
        
        // Batch send for efficiency
        self.signal_ref.send_batch(signals).await?;
        Ok(())
    }
}
```

## TDD Test Cases

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_local_actor_ref_send() {
        let (tx, mut rx) = mpsc::channel(10);
        let actor_ref = ActorRef::<String>::local(ActorId::new(), tx);
        
        // Send message
        actor_ref.send("Hello".to_string()).await.unwrap();
        
        // Verify received
        let msg = rx.recv().await.unwrap();
        let text = msg.downcast::<String>().unwrap();
        assert_eq!(*text, "Hello");
    }

    #[tokio::test]
    async fn test_try_send_non_blocking() {
        let (tx, mut rx) = mpsc::channel(1);
        let actor_ref = ActorRef::<u32>::local(ActorId::new(), tx);
        
        // First send succeeds
        assert!(actor_ref.try_send(42).is_ok());
        
        // Second send fails (mailbox full)
        assert!(matches!(
            actor_ref.try_send(43),
            Err(ActorError::MailboxFull)
        ));
        
        // Consume message
        let _ = rx.recv().await;
        
        // Now send succeeds
        assert!(actor_ref.try_send(43).is_ok());
    }

    #[tokio::test]
    async fn test_send_timeout() {
        let (tx, _rx) = mpsc::channel(1);
        let actor_ref = ActorRef::<u32>::local(ActorId::new(), tx);
        
        // Fill mailbox
        actor_ref.send(1).await.unwrap();
        
        // Send with timeout should fail
        let result = actor_ref.send_timeout(2, Duration::from_millis(10)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_batch_send() {
        let (tx, mut rx) = mpsc::channel(10);
        let actor_ref = ActorRef::<u32>::local(ActorId::new(), tx);
        
        // Batch send
        let messages = vec![1, 2, 3, 4, 5];
        actor_ref.send_batch(messages.clone()).await.unwrap();
        
        // Verify all received
        for expected in messages {
            let msg = rx.recv().await.unwrap();
            let value = msg.downcast::<u32>().unwrap();
            assert_eq!(*value, expected);
        }
    }

    #[test]
    fn test_actor_ref_clone() {
        let (tx, _rx) = mpsc::channel(10);
        let actor_ref = ActorRef::<String>::local(ActorId::new(), tx);
        
        // ActorRef should be cheaply cloneable
        let cloned = actor_ref.clone();
        assert_eq!(actor_ref.id(), cloned.id());
    }

    #[test]
    fn test_is_local() {
        let (tx, _rx) = mpsc::channel(10);
        let actor_ref = ActorRef::<String>::local(ActorId::new(), tx);
        
        assert!(actor_ref.is_local());
    }

    #[tokio::test]
    async fn test_stopped_actor() {
        let (tx, rx) = mpsc::channel::<ActorMessage>(10);
        let actor_ref = ActorRef::<String>::local(ActorId::new(), tx);
        
        // Drop receiver to simulate stopped actor
        drop(rx);
        
        // Send should fail with ActorStopped
        let result = actor_ref.send("test".to_string()).await;
        assert!(matches!(result, Err(ActorError::ActorStopped)));
    }
}
```

## Design Rationale

### Why PhantomData?
- Provides compile-time type safety for messages
- Zero runtime cost
- Prevents sending wrong message types

### Why Transport enum?
- Prepares for future remote transports
- Single ActorRef API regardless of location
- Easy to extend without breaking changes

### Why not implement Sink trait?
- Sink requires Item type at trait level
- Would make ActorRef less flexible
- Our API is simpler and more focused

## Validation Checklist
- [ ] Type-safe message sending
- [ ] Zero-cost for local actors (just Arc + channel)
- [ ] Non-blocking try_send option
- [ ] Batch sending support
- [ ] Timeout support
- [ ] Cheap cloning of ActorRef
- [ ] Proper error handling for stopped actors
- [ ] Extensible for future transports

## Definition of Done
- ActorRef implemented with local transport
- Type safety via PhantomData
- Extension methods (timeout, batch, etc.)
- Comprehensive test coverage
- Ready for remote transport extension
- Documentation and examples