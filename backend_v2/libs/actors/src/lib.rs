//! Actor-Based Message Passing Infrastructure
//!
//! Zero-cost actor runtime for Torq with adaptive transport selection.
//! Provides Arc<T> message passing for same-process actors and TLV serialization
//! only when crossing process boundaries.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────┐    ┌──────────────────────┐
//! │    Actor System     │    │   Transport Layer    │
//! │                     │    │                      │
//! │  ┌───────────────┐  │    │ ┌──────────────────┐ │
//! │  │ ActorA        │  │    │ │  Local Channel   │ │
//! │  │ (bundled)     │──┼────┼─│  Arc<Message>    │ │
//! │  └───────────────┘  │    │ │  <100ns          │ │
//! │                     │    │ └──────────────────┘ │
//! │  ┌───────────────┐  │    │                      │
//! │  │ ActorB        │  │    │ ┌──────────────────┐ │
//! │  │ (remote)      │──┼────┼─│  Unix Socket     │ │
//! │  └───────────────┘  │    │ │  TLV Serialized  │ │
//! └─────────────────────┘    │ │  <35μs           │ │
//!                            │ └──────────────────┘ │
//!                            └──────────────────────┘
//! ```
//!
//! # Performance Targets
//!
//! - **Local (Arc<T>)**: <100ns per message
//! - **Unix Socket**: <35μs per message  
//! - **Network**: <5ms per message
//! - **Zero allocations** in steady state for bundled actors
//!
//! # Examples
//!
//! ```rust
//! use messaging_actors::{ActorSystem, ActorBehavior};
//!
//! // Define actor behavior
//! struct MyActor;
//! 
//! impl ActorBehavior for MyActor {
//!     type Message = String;
//!     
//!     async fn handle(&mut self, msg: Self::Message) {
//!         println!("Received: {}", msg);
//!     }
//! }
//!
//! // Create and run actor system
//! let system = ActorSystem::new("my_system");
//! let actor_ref = system.spawn(MyActor);
//! actor_ref.send("Hello, Actor!".to_string()).await;
//! ```

pub mod transport;
pub mod messages;
pub mod system;
pub mod bundle;
pub mod registry;
pub mod proof_of_concept;

pub use transport::{ActorTransport, TransportType, TransportMetrics};
pub use messages::{
    ActorMessage, ActorEnvelope, ByteActorMessage,
    TypedReceiver, MessageHandler,
    ActorMessageRegistry, ActorMessageStats,
};
pub use system::{ActorSystem, ActorBehavior, ActorRef, SupervisorDirective};
pub use bundle::{BundleConfiguration, DeploymentMode};
pub use registry::{ActorRegistry, ActorId};
pub use proof_of_concept::{ProofOfConceptMigration, MigrationStats};