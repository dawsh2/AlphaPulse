# MVP-002: Actor Core Trait Design

## Task Overview
**Sprint**: 005-mycelium-mvp
**Priority**: CRITICAL
**Estimate**: 8 hours
**Status**: TODO
**Goal**: Design the foundational Actor trait with proper Rust generics and lifetime management

## Problem
The Actor trait is the foundation of the entire system. It must be:
- Object-safe (usable as `Box<dyn Actor>`)
- Efficient (no unnecessary allocations)
- Flexible (support different message types)
- Lifecycle-aware (start, stop, error handling)

## Critical Design Decisions

### Why Associated Types over Generics?
```rust
// GOOD: Associated type
trait Actor {
    type Message;
}

// BAD: Generic parameter
trait Actor<M> {
    // Makes trait objects harder: Box<dyn Actor<???>>
}
```

### Object Safety Requirements
The trait must be object-safe to allow `Box<dyn Actor>` in the runtime. This means:
- No generic methods
- No methods returning Self
- No static methods

## Implementation

### Core Actor Trait
```rust
// libs/mycelium/src/actor.rs

use std::any::Any;
use async_trait::async_trait;

/// Core actor behavior trait
#[async_trait]
pub trait Actor: Send + Sync + 'static {
    /// Message type this actor handles
    type Message: Send + Sync + 'static;
    
    /// Handle incoming message
    async fn handle(&mut self, msg: Self::Message) -> Result<(), ActorError>;
    
    /// Called when actor starts
    async fn on_start(&mut self) -> Result<(), ActorError> {
        Ok(())
    }
    
    /// Called before actor stops
    async fn on_stop(&mut self) -> Result<(), ActorError> {
        Ok(())
    }
    
    /// Handle errors with supervision directive
    async fn on_error(&mut self, error: ActorError) -> SupervisionDirective {
        SupervisionDirective::Stop
    }
    
    /// Get actor name for debugging
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// Errors that can occur in actor processing
#[derive(Debug, thiserror::Error)]
pub enum ActorError {
    #[error("Message handling failed: {0}")]
    HandleError(String),
    
    #[error("Actor initialization failed: {0}")]
    StartupError(String),
    
    #[error("Actor shutdown failed: {0}")]
    ShutdownError(String),
    
    #[error("Mailbox full")]
    MailboxFull,
    
    #[error("Actor stopped")]
    ActorStopped,
    
    #[error("Custom error: {0}")]
    Custom(Box<dyn std::error::Error + Send + Sync>),
}

/// Supervision directives for error handling
#[derive(Debug, Clone, Copy)]
pub enum SupervisionDirective {
    /// Continue processing messages
    Resume,
    
    /// Restart the actor (clear state)
    Restart,
    
    /// Stop the actor permanently
    Stop,
    
    /// Escalate to parent supervisor (Phase 2)
    Escalate,
}
```

### Type-Erased Actor for Runtime
```rust
/// Type-erased actor for the runtime
#[async_trait]
pub trait DynActor: Send + Sync + 'static {
    /// Handle type-erased message
    async fn handle_any(&mut self, msg: Arc<dyn Any + Send + Sync>) -> Result<(), ActorError>;
    
    /// Lifecycle methods
    async fn start(&mut self) -> Result<(), ActorError>;
    async fn stop(&mut self) -> Result<(), ActorError>;
    async fn supervise(&mut self, error: ActorError) -> SupervisionDirective;
    
    /// Actor metadata
    fn id(&self) -> &ActorId;
    fn name(&self) -> &str;
}

/// Wrapper to convert Actor to DynActor
pub struct ActorWrapper<A: Actor> {
    actor: A,
    id: ActorId,
}

#[async_trait]
impl<A: Actor> DynActor for ActorWrapper<A> {
    async fn handle_any(&mut self, msg: Arc<dyn Any + Send + Sync>) -> Result<(), ActorError> {
        // Try to downcast to expected message type
        if let Ok(typed_msg) = msg.downcast::<A::Message>() {
            // Extract from Arc and handle
            match Arc::try_unwrap(typed_msg) {
                Ok(msg) => self.actor.handle(msg).await,
                Err(arc) => {
                    // Arc has multiple owners, clone the message
                    self.actor.handle((*arc).clone()).await
                }
            }
        } else {
            // Message type mismatch - log and continue
            warn!("Actor {} received unexpected message type", self.id);
            Ok(())
        }
    }
    
    async fn start(&mut self) -> Result<(), ActorError> {
        self.actor.on_start().await
    }
    
    async fn stop(&mut self) -> Result<(), ActorError> {
        self.actor.on_stop().await
    }
    
    async fn supervise(&mut self, error: ActorError) -> SupervisionDirective {
        self.actor.on_error(error).await
    }
    
    fn id(&self) -> &ActorId {
        &self.id
    }
    
    fn name(&self) -> &str {
        self.actor.name()
    }
}
```

### Stateful Actor Example
```rust
/// Example implementation for testing
pub struct CounterActor {
    count: u64,
    max_count: u64,
}

#[derive(Debug, Clone)]
pub enum CounterMessage {
    Increment(u64),
    GetCount(tokio::sync::oneshot::Sender<u64>),
    Reset,
}

#[async_trait]
impl Actor for CounterActor {
    type Message = CounterMessage;
    
    async fn handle(&mut self, msg: CounterMessage) -> Result<(), ActorError> {
        match msg {
            CounterMessage::Increment(n) => {
                self.count += n;
                if self.count > self.max_count {
                    return Err(ActorError::HandleError(
                        format!("Count {} exceeds max {}", self.count, self.max_count)
                    ));
                }
                Ok(())
            }
            CounterMessage::GetCount(sender) => {
                let _ = sender.send(self.count);
                Ok(())
            }
            CounterMessage::Reset => {
                self.count = 0;
                Ok(())
            }
        }
    }
    
    async fn on_start(&mut self) -> Result<(), ActorError> {
        info!("CounterActor starting with max_count: {}", self.max_count);
        Ok(())
    }
    
    async fn on_error(&mut self, error: ActorError) -> SupervisionDirective {
        match error {
            ActorError::HandleError(_) => {
                // Reset on overflow
                self.count = 0;
                SupervisionDirective::Resume
            }
            _ => SupervisionDirective::Stop
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
    fn test_actor_trait_object_safety() {
        // Verify trait can be used as trait object
        fn accepts_dyn_actor(_: Box<dyn DynActor>) {}
        
        let actor = ActorWrapper {
            actor: CounterActor { count: 0, max_count: 100 },
            id: ActorId::new(),
        };
        accepts_dyn_actor(Box::new(actor));
    }

    #[tokio::test]
    async fn test_actor_lifecycle() {
        let mut actor = CounterActor { count: 0, max_count: 100 };
        
        // Start
        assert!(actor.on_start().await.is_ok());
        
        // Handle messages
        assert!(actor.handle(CounterMessage::Increment(5)).await.is_ok());
        assert_eq!(actor.count, 5);
        
        // Stop
        assert!(actor.on_stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_error_handling() {
        let mut actor = CounterActor { count: 0, max_count: 10 };
        
        // Trigger error
        let result = actor.handle(CounterMessage::Increment(20)).await;
        assert!(result.is_err());
        
        // Check supervision directive
        let directive = actor.on_error(result.unwrap_err()).await;
        assert_eq!(directive, SupervisionDirective::Resume);
        assert_eq!(actor.count, 0); // Should be reset
    }

    #[tokio::test]
    async fn test_type_erased_handling() {
        let mut wrapper = ActorWrapper {
            actor: CounterActor { count: 0, max_count: 100 },
            id: ActorId::new(),
        };
        
        // Create type-erased message
        let msg = Arc::new(CounterMessage::Increment(5)) as Arc<dyn Any + Send + Sync>;
        
        // Handle through wrapper
        assert!(wrapper.handle_any(msg).await.is_ok());
        assert_eq!(wrapper.actor.count, 5);
    }

    #[test]
    fn test_message_constraints() {
        // Verify Send + Sync requirements
        fn assert_send_sync<T: Send + Sync>() {}
        
        assert_send_sync::<CounterMessage>();
        assert_send_sync::<CounterActor>();
    }

    #[test]
    fn test_actor_name() {
        let actor = CounterActor { count: 0, max_count: 100 };
        assert!(actor.name().contains("CounterActor"));
    }
}
```

## Design Rationale

### Why async_trait?
- Required for async methods in traits (until async fn in traits stabilizes)
- Small runtime overhead acceptable for actor model

### Why ActorError enum?
- Structured error handling
- Clear supervision decisions based on error type
- Extensible for future error cases

### Why DynActor wrapper?
- Allows runtime to store heterogeneous actors
- Type erasure happens once at spawn time
- Preserves type safety within each actor

## Validation Checklist
- [ ] Actor trait is object-safe
- [ ] DynActor wrapper works correctly
- [ ] Lifecycle methods called in order
- [ ] Error handling with supervision directives
- [ ] Message type constraints enforced
- [ ] No unnecessary allocations in hot path
- [ ] Thread safety guaranteed

## Definition of Done
- Actor trait defined with all lifecycle methods
- DynActor wrapper for type erasure
- Error types and supervision directives
- Complete test coverage
- Example implementation (CounterActor)
- Documentation for trait usage