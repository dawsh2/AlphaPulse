//! Mycelium Actor Runtime
//!
//! Zero-cost actor runtime for AlphaPulse with adaptive transport selection.
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

pub mod transport;
pub mod messages;
pub mod system;
pub mod bundle;
pub mod registry;

pub use transport::{ActorTransport, TransportType, TransportMetrics};
pub use messages::{Message, MarketMessage, SignalMessage, ExecutionMessage, MessageRegistry};
pub use system::{ActorSystem, ActorBehavior, ActorRef, SupervisorDirective};
pub use bundle::{BundleConfiguration, DeploymentMode};
pub use registry::{ActorRegistry, ActorId};