//! AlphaPulse Declarative Topology System
//!
//! Separates logical service contracts (actors) from physical deployment (nodes)
//! to enable optimal transport selection and NUMA-aware placement.

pub mod actors;
pub mod config;
pub mod deployment;
pub mod error;
pub mod nodes;
pub mod resolution;
pub mod runtime;
pub mod transport;
pub mod validation;

// Re-export main types
pub use actors::{Actor, ActorPersistence, ActorState, ActorType};
pub use config::TopologyConfig;
pub use deployment::DeploymentEngine;
pub use error::{Result, TopologyError};
pub use nodes::{ActorPlacement, ChannelConfig, Node};
pub use resolution::TopologyResolver;
pub use transport::{CompressionType, NetworkProtocol, Transport};

/// Current version of the topology configuration format
pub const TOPOLOGY_VERSION: &str = "1.0.0";

/// Maximum number of actors per node (safety limit)
pub const MAX_ACTORS_PER_NODE: usize = 64;

/// Maximum number of CPU cores that can be assigned to a single actor
pub const MAX_CPU_CORES_PER_ACTOR: usize = 16;
