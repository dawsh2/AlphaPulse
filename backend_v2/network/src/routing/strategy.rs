//! Routing Strategy Definitions
//!
//! Defines the routing strategies and decisions consolidated from various modules

use serde::{Deserialize, Serialize};

/// Routing strategy enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingStrategy {
    /// Always use local transport if possible
    LocalFirst,
    /// Always use network transport
    NetworkOnly,
    /// Hybrid approach based on latency and reliability requirements
    Hybrid,
    /// Optimize for lowest latency
    LatencyOptimized,
    /// Topology-aware routing considering NUMA, CPU affinity, etc.
    TopologyAware,
    /// Load-balanced routing across available transports
    LoadBalanced,
}

/// Routing decision result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingDecision {
    /// Use in-process communication (Arc<T> via channels)
    Local {
        channel_name: String,
        buffer_size: Option<usize>,
    },
    /// Use Unix domain socket
    UnixSocket {
        socket_path: String,
        connection_pool: bool,
    },
    /// Use TCP transport
    Tcp {
        address: String,
        port: u16,
        use_tls: bool,
    },
    /// Use UDP transport
    Udp {
        address: String,
        port: u16,
        reliable: bool,
    },
    /// Use message queue (if available)
    MessageQueue {
        queue_name: String,
        exchange: Option<String>,
        routing_key: Option<String>,
    },
    /// Error - no viable route found
    Unavailable {
        reason: String,
    },
}

impl RoutingDecision {
    /// Check if this decision represents a successful route
    pub fn is_success(&self) -> bool {
        !matches!(self, RoutingDecision::Unavailable { .. })
    }
    
    /// Get the transport type for this decision
    pub fn transport_type(&self) -> Option<&'static str> {
        match self {
            RoutingDecision::Local { .. } => Some("local"),
            RoutingDecision::UnixSocket { .. } => Some("unix_socket"),
            RoutingDecision::Tcp { .. } => Some("tcp"),
            RoutingDecision::Udp { .. } => Some("udp"),
            RoutingDecision::MessageQueue { .. } => Some("message_queue"),
            RoutingDecision::Unavailable { .. } => None,
        }
    }
    
    /// Check if this decision uses a local transport
    pub fn is_local(&self) -> bool {
        matches!(self, RoutingDecision::Local { .. })
    }
    
    /// Check if this decision uses a network transport
    pub fn is_network(&self) -> bool {
        matches!(self, RoutingDecision::Tcp { .. } | RoutingDecision::Udp { .. })
    }
}