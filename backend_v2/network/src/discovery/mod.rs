//! Service Discovery and Topology Management
//!
//! ## Consolidation Note
//!
//! This module was originally the `torq-topology` crate and has been
//! consolidated into the unified `torq-network` crate. All original APIs are
//! preserved for backward compatibility.
//!
//! **Migration**: Replace `use torq_topology::*` with `use network::topology::*`
//!
//! ## Purpose
//! Separates logical service contracts (actors) from physical deployment (nodes)
//! to enable optimal transport selection, NUMA-aware placement, and dynamic
//! service discovery across the Torq trading infrastructure.
//!
//! ## Architecture Role
//!
//! ```mermaid
//! graph TB
//!     Config[topology.toml] -->|Actor Definitions| ActorRegistry[Actor Registry]
//!     Config -->|Node Definitions| NodeRegistry[Node Registry]
//!     
//!     ActorRegistry --> Resolver[Topology Resolver]
//!     NodeRegistry --> Resolver
//!     
//!     Resolver -->|Service Discovery| Placement{Placement Engine}
//!     
//!     Placement -->|NUMA Node 0| Node0[Node 0: Hot Path Services]
//!     Placement -->|NUMA Node 1| Node1[Node 1: Analytics Services]
//!     Placement -->|Remote| RemoteNodes[Remote Nodes]
//!     
//!     subgraph "Node 0 - Ultra Low Latency"
//!         MarketDataRelay[market_data_relay]
//!         PolygonPublisher[polygon_publisher]
//!         FlashStrategy[flash_arbitrage]
//!     end
//!     
//!     subgraph "Node 1 - Compute Intensive"
//!         SignalRelay[signal_relay]
//!         Dashboard[dashboard_websocket]
//!         Analytics[trace_collector]
//!     end
//!     
//!     subgraph "Transport Selection"
//!         SameNode[Same Node: Unix Sockets <35μs]
//!         SameNUMA[Same NUMA: Shared Memory <100μs]
//!         CrossNUMA[Cross NUMA: TCP <5ms]
//!         Remote[Remote: TCP/QUIC <50ms]
//!     end
//!     
//!     classDef hotpath fill:#FF6B6B
//!     classDef analytics fill:#4ECDC4
//!     classDef transport fill:#FFE66D
//!     class Node0,MarketDataRelay,PolygonPublisher,FlashStrategy hotpath
//!     class Node1,SignalRelay,Dashboard,Analytics analytics
//!     class SameNode,SameNUMA,CrossNUMA,Remote transport
//! ```
//!
//! ## Service Discovery Framework
//!
//! **Actor-Based Abstraction**: Services define logical contracts (actors)
//! independent of physical deployment location. Topology resolver maps
//! actors to optimal nodes based on:
//!
//! - **Performance Requirements**: Latency vs throughput optimization
//! - **Resource Constraints**: CPU, memory, network bandwidth  
//! - **Affinity Rules**: Co-locate related services for efficiency
//! - **Fault Tolerance**: Distribute critical services across nodes
//!
//! ## Dynamic Transport Selection
//!
//! **Automatic Optimization**: Transport layer chooses fastest available method:
//!
//! 1. **Same Process**: Direct function calls (0μs)
//! 2. **Same Node**: Unix domain sockets (<35μs)
//! 3. **Same NUMA**: Shared memory IPC (<100μs)
//! 4. **Cross NUMA**: TCP with NUMA pinning (<5ms)
//! 5. **Remote Node**: TCP/QUIC with compression (<50ms)
//!
//! ## Configuration-Driven Deployment
//!
//! **Declarative Topology**: Single `topology.toml` defines entire system:
//!
//! ```toml
//! # High-performance actor on dedicated CPU cores
//! [actors.market_data_relay]
//! type = "relay"
//! domain = 1
//! cpu_affinity = [0, 1]
//! memory_limit = "512MB"
//! priority = "realtime"
//!
//! # Analytics actor with relaxed constraints  
//! [actors.trace_collector]
//! type = "analytics"
//! cpu_affinity = [4, 5, 6, 7]
//! memory_limit = "2GB"
//! priority = "normal"
//!
//! # Node placement rules
//! [nodes.trading_node]
//! numa_node = 0
//! actors = ["market_data_relay", "polygon_publisher", "flash_arbitrage"]
//!
//! [nodes.analytics_node]
//! numa_node = 1
//! actors = ["signal_relay", "dashboard_websocket", "trace_collector"]
//! ```
//!
//! ## Performance Profile
//!
//! - **Service Resolution**: <1μs actor-to-node lookup via hash table
//! - **Transport Selection**: <5μs optimal path calculation
//! - **Configuration Loading**: <10ms full topology parse and validation
//! - **Memory Usage**: <10MB for 1000+ actor definitions
//! - **Deployment Time**: <100ms to start all actors with proper placement
//!
//! ## Integration Points
//!
//! **Service Bootstrap**: Every service queries topology resolver for:
//! - **Own Placement**: Which node/CPU cores to bind to
//! - **Dependency Discovery**: How to connect to required services  
//! - **Transport Configuration**: Unix socket paths, TCP addresses, etc.
//! - **Resource Limits**: Memory, CPU, and network constraints
//!
//! **Runtime Reconfiguration**: Topology can be updated without restart:
//! - Add new actors to handle increased load
//! - Migrate services between nodes for rebalancing
//! - Update transport selection for changed network topology
//! - Modify resource constraints based on measured performance
//!
//! ## Critical for System Connectivity
//!
//! **Service Discovery**: Prevents hardcoded connection paths that break:
//! - Unix socket paths determined by actor placement rules
//! - Port assignments based on node configuration
//! - Transport selection optimized for actual deployment topology
//! - Service startup order determined by dependency graph
//!
//! **Fault Tolerance**: Enables graceful handling of node failures:
//! - Automatic failover to backup nodes for critical actors
//! - Load rebalancing when nodes become unavailable
//! - Circuit breaker integration for unhealthy transports
//! - Health check propagation across service dependency graph
//!
//! ## Troubleshooting Service Placement
//!
//! **Service discovery failures**:
//! - Check `topology.toml` has all required actor definitions
//! - Verify node placement rules don't create conflicts
//! - Ensure transport configuration matches actual network topology
//! - Monitor resolver logs for placement decision details
//!
//! **Performance issues**:
//! - Verify CPU affinity matches NUMA topology
//! - Check if actors are placed optimally for communication patterns
//! - Monitor transport selection for suboptimal routing
//! - Validate memory limits don't cause excessive swapping
//!
//! **Connection failures**:
//! - Ensure topology resolver provides correct connection endpoints
//! - Check if transport selection matches service capabilities
//! - Verify firewall rules allow selected transport protocols
//! - Monitor for race conditions in service startup ordering

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
pub use nodes::{ActorPlacement, ChannelConfig, Node, ServiceDiscoveryConfig};
pub use resolution::TopologyResolver;
pub use transport::{CompressionType, NetworkProtocol, Transport};

// Service discovery compatibility types
pub type ServiceDiscovery = TopologyResolver;
pub struct ServiceDiscoveryFactory;

impl ServiceDiscoveryFactory {
    /// Create a new service discovery instance
    pub fn create() -> ServiceDiscovery {
        let config = config::TopologyConfig {
            version: TOPOLOGY_VERSION.to_string(),
            actors: std::collections::HashMap::new(),
            nodes: std::collections::HashMap::new(),
            inter_node: None,
            metadata: config::ConfigMetadata::default(),
        };
        TopologyResolver::new(config)
    }
}

/// Service location information
#[derive(Debug, Clone)]
pub struct ServiceLocation {
    /// Node name
    pub node: String,
    /// Actor name
    pub actor: String,
    /// Service endpoint address
    pub endpoint: String,
}

/// Current version of the topology configuration format
pub const TOPOLOGY_VERSION: &str = "1.0.0";

/// Maximum number of actors per node (safety limit)
pub const MAX_ACTORS_PER_NODE: usize = 64;

/// Maximum number of CPU cores that can be assigned to a single actor
pub const MAX_CPU_CORES_PER_ACTOR: usize = 16;
