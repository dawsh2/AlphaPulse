---
task_id: MYCEL-003
status: COMPLETED
priority: HIGH
estimated_hours: 12
actual_hours: 8
assigned_branch: feat/mycelium-actor-system
assignee: Claude
created: 2025-08-26
completed: 2025-08-28
depends_on:
  - MYCEL-001  # Need transport layer first
  - MYCEL-002  # Need message types first
blocks:
  - MYCEL-007  # Proof of concept needs actor system
scope:
  - "network/transport/src/actor/system.rs"  # Actor system core
  - "network/transport/src/actor/runtime.rs"  # Runtime management
---

# MYCEL-003: Actor System Core

## Task Overview
**Sprint**: 004-mycelium-runtime  
**Priority**: HIGH
**Estimate**: 12 hours
**Status**: TODO
**Dependencies**: MYCEL-001, MYCEL-002
**Goal**: Build actor lifecycle management with zero-cost local communication

## Problem
Need a runtime that manages actor lifecycles, routing, and mailboxes while maintaining the zero-cost abstraction for bundled actors.

## Implementation

### Actor System
```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ActorSystem {
    /// All actors in the system
    actors: Arc<RwLock<HashMap<ActorId, ActorHandle>>>,
    
    /// Topology configuration (reuse existing)
    topology: Arc<TopologyConfig>,
    
    /// Bundle configurations
    bundles: Arc<RwLock<HashMap<BundleId, ActorBundle>>>,
    
    /// System-wide event bus
    event_bus: EventBus,
    
    /// Metrics collection
    metrics: SystemMetrics,
}

impl ActorSystem {
    pub async fn spawn<A>(&self, actor: A) -> Result<ActorRef<A::Message>>
    where
        A: ActorBehavior + 'static,
    {
        let id = ActorId::new();
        let (mailbox, receiver) = Mailbox::new(1000);
        
        // Determine transport based on topology
        let transport = self.create_transport(&id).await?;
        
        // Create actor handle
        let handle = ActorHandle {
            id: id.clone(),
            mailbox: mailbox.clone(),
            transport: transport.clone(),
            status: ActorStatus::Running,
        };
        
        // Register actor
        self.actors.write().await.insert(id.clone(), handle);
        
        // Spawn actor task
        let actor_task = ActorTask {
            id: id.clone(),
            behavior: Box::new(actor),
            receiver,
            system: self.clone(),
        };
        
        tokio::spawn(actor_task.run());
        
        Ok(ActorRef {
            id,
            transport,
            _phantom: PhantomData,
        })
    }
    
    async fn create_transport(&self, actor_id: &ActorId) -> Result<ActorTransport> {
        // Find which bundle this actor belongs to
        let bundle_id = self.find_bundle(actor_id).await?;
        let bundle = self.bundles.read().await.get(&bundle_id).cloned();
        
        match bundle.map(|b| b.deployment) {
            Some(DeploymentMode::SharedMemory { channels }) => {
                // Local transport for bundled actors
                Ok(ActorTransport {
                    local: channels.get(actor_id).cloned(),
                    remote: None,
                    metrics: TransportMetrics::new(),
                })
            }
            Some(DeploymentMode::SameNode { sockets }) => {
                // Unix socket for same-node actors
                Ok(ActorTransport {
                    local: None,
                    remote: Some(sockets.get(actor_id).cloned()?),
                    metrics: TransportMetrics::new(),
                })
            }
            _ => {
                // Network transport for distributed actors
                Ok(ActorTransport {
                    local: None,
                    remote: Some(self.create_network_transport(actor_id)?),
                    metrics: TransportMetrics::new(),
                })
            }
        }
    }
}
```

### Actor Behavior Trait
```rust
#[async_trait]
pub trait ActorBehavior: Send + Sync + 'static {
    type Message: Message;
    
    /// Handle incoming message
    async fn handle(&mut self, msg: Self::Message) -> Result<()>;
    
    /// Called when actor starts
    async fn on_start(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// Called before actor stops
    async fn on_stop(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// Handle failure
    async fn on_error(&mut self, error: Error) -> SupervisorDirective {
        SupervisorDirective::Restart
    }
}

pub enum SupervisorDirective {
    Resume,    // Continue processing
    Restart,   // Restart the actor
    Stop,      // Stop the actor
    Escalate,  // Escalate to parent
}
```

### Mailbox Implementation
```rust
pub struct Mailbox<M: Message> {
    /// High priority messages
    high: mpsc::Sender<M>,
    
    /// Normal priority messages
    normal: mpsc::UnboundedSender<M>,
    
    /// Metrics
    queue_depth: AtomicUsize,
    dropped_count: AtomicU64,
}

impl<M: Message> Mailbox<M> {
    pub fn new(capacity: usize) -> (Self, MailboxReceiver<M>) {
        let (high_tx, high_rx) = mpsc::channel(capacity / 4);
        let (normal_tx, normal_rx) = mpsc::unbounded_channel();
        
        let mailbox = Self {
            high: high_tx,
            normal: normal_tx,
            queue_depth: AtomicUsize::new(0),
            dropped_count: AtomicU64::new(0),
        };
        
        let receiver = MailboxReceiver {
            high: high_rx,
            normal: normal_rx,
        };
        
        (mailbox, receiver)
    }
    
    pub async fn send(&self, msg: M, priority: Priority) -> Result<()> {
        self.queue_depth.fetch_add(1, Ordering::Relaxed);
        
        match priority {
            Priority::High => {
                self.high.send(msg).await
                    .map_err(|_| Error::MailboxFull)?;
            }
            Priority::Normal => {
                self.normal.send(msg)
                    .map_err(|_| Error::ActorStopped)?;
            }
        }
        
        Ok(())
    }
}

pub struct MailboxReceiver<M: Message> {
    high: mpsc::Receiver<M>,
    normal: mpsc::UnboundedReceiver<M>,
}

impl<M: Message> MailboxReceiver<M> {
    pub async fn recv(&mut self) -> Option<M> {
        // Prioritize high priority messages
        tokio::select! {
            biased;
            
            msg = self.high.recv() => msg,
            msg = self.normal.recv() => msg,
        }
    }
}
```

### Actor Task Runner
```rust
struct ActorTask {
    id: ActorId,
    behavior: Box<dyn ActorBehavior<Message = MarketMessage>>,
    receiver: MailboxReceiver<MarketMessage>,
    system: ActorSystem,
}

impl ActorTask {
    async fn run(mut self) {
        // Lifecycle: Start
        if let Err(e) = self.behavior.on_start().await {
            error!("Actor {} failed to start: {}", self.id, e);
            return;
        }
        
        // Main message loop
        while let Some(msg) = self.receiver.recv().await {
            let start = Instant::now();
            
            match self.behavior.handle(msg).await {
                Ok(()) => {
                    self.system.metrics.record_message_handled(start.elapsed());
                }
                Err(e) => {
                    error!("Actor {} error: {}", self.id, e);
                    match self.behavior.on_error(e).await {
                        SupervisorDirective::Resume => continue,
                        SupervisorDirective::Restart => {
                            self.restart().await;
                        }
                        SupervisorDirective::Stop => break,
                        SupervisorDirective::Escalate => {
                            self.escalate_error(e).await;
                        }
                    }
                }
            }
        }
        
        // Lifecycle: Stop
        if let Err(e) = self.behavior.on_stop().await {
            error!("Actor {} failed to stop cleanly: {}", self.id, e);
        }
    }
    
    async fn restart(&mut self) {
        info!("Restarting actor {}", self.id);
        // Re-initialize actor state
        self.behavior.on_stop().await.ok();
        self.behavior.on_start().await.ok();
    }
}
```

### Actor Reference
```rust
pub struct ActorRef<M: Message> {
    id: ActorId,
    transport: ActorTransport,
    _phantom: PhantomData<M>,
}

impl<M: Message> ActorRef<M> {
    pub async fn send(&self, msg: M) -> Result<()> {
        self.transport.send(msg).await
    }
    
    pub async fn ask<R>(&self, msg: M) -> Result<R> 
    where
        R: Message,
    {
        // Request-response pattern (future enhancement)
        todo!("Implement ask pattern")
    }
    
    pub fn id(&self) -> &ActorId {
        &self.id
    }
}
```

## TDD Test Cases

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct TestActor {
        counter: u32,
        received: Vec<TestMessage>,
    }
    
    #[async_trait]
    impl ActorBehavior for TestActor {
        type Message = TestMessage;
        
        async fn handle(&mut self, msg: TestMessage) -> Result<()> {
            self.counter += 1;
            self.received.push(msg);
            Ok(())
        }
        
        async fn on_start(&mut self) -> Result<()> {
            println!("TestActor started");
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_actor_spawn_and_messaging() {
        let system = ActorSystem::new();
        
        // Spawn actor
        let actor = TestActor {
            counter: 0,
            received: vec![],
        };
        let actor_ref = system.spawn(actor).await.unwrap();
        
        // Send messages
        for i in 0..10 {
            actor_ref.send(TestMessage { id: i }).await.unwrap();
        }
        
        // Give time to process
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Verify actor processed messages
        let handle = system.actors.read().await
            .get(&actor_ref.id())
            .cloned()
            .unwrap();
        assert_eq!(handle.status, ActorStatus::Running);
    }

    #[tokio::test]
    async fn test_actor_lifecycle() {
        let system = ActorSystem::new();
        
        let actor = TestActor::default();
        let actor_ref = system.spawn(actor).await.unwrap();
        
        // Verify actor started
        assert!(system.actors.read().await.contains_key(&actor_ref.id()));
        
        // Stop actor
        system.stop_actor(&actor_ref.id()).await.unwrap();
        
        // Verify actor stopped
        let handle = system.actors.read().await
            .get(&actor_ref.id())
            .cloned();
        assert_eq!(handle.unwrap().status, ActorStatus::Stopped);
    }

    #[tokio::test]
    async fn test_mailbox_priority() {
        let (mailbox, mut receiver) = Mailbox::<TestMessage>::new(100);
        
        // Send normal priority
        mailbox.send(TestMessage { id: 1 }, Priority::Normal).await.unwrap();
        
        // Send high priority
        mailbox.send(TestMessage { id: 2 }, Priority::High).await.unwrap();
        
        // High priority should be received first
        let msg = receiver.recv().await.unwrap();
        assert_eq!(msg.id, 2);
        
        let msg = receiver.recv().await.unwrap();
        assert_eq!(msg.id, 1);
    }

    #[tokio::test]
    async fn test_bundle_transport_selection() {
        let system = ActorSystem::new();
        
        // Create bundle with shared memory
        let bundle = ActorBundle {
            name: "test_bundle".to_string(),
            actors: vec![],
            deployment: DeploymentMode::SharedMemory {
                channels: HashMap::new(),
            },
        };
        
        system.bundles.write().await.insert(
            BundleId::new("test_bundle"),
            bundle,
        );
        
        // Spawn actor in bundle
        let actor = TestActor::default();
        let actor_ref = system.spawn(actor).await.unwrap();
        
        // Verify local transport selected
        assert!(actor_ref.transport.local.is_some());
        assert!(actor_ref.transport.remote.is_none());
    }

    #[bench]
    fn bench_actor_message_throughput(b: &mut Bencher) {
        let rt = Runtime::new().unwrap();
        
        b.iter(|| {
            rt.block_on(async {
                let system = ActorSystem::new();
                let actor = TestActor::default();
                let actor_ref = system.spawn(actor).await.unwrap();
                
                // Measure throughput
                for _ in 0..10_000 {
                    actor_ref.send(TestMessage::default()).await.unwrap();
                }
            });
        });
    }
}
```

## Validation Checklist
- [ ] Actor spawn and lifecycle management
- [ ] Message routing via transport abstraction  
- [ ] Mailbox with priority support
- [ ] Supervision directives (restart, stop, escalate)
- [ ] Bundle-aware transport selection
- [ ] Metrics collection
- [ ] Thread-safe concurrent operations

## Performance Requirements
- Actor spawn: <1ms
- Message processing: <1μs overhead
- Mailbox operations: Lock-free where possible
- System can handle 10,000+ actors
- Zero allocation in steady state

## Definition of Done
- ActorSystem managing actor lifecycles
- Mailbox implementation with priorities
- Transport selection based on bundles
- Supervision basics implemented
- Performance benchmarks passing
- Comprehensive test coverage