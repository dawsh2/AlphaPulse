//! Network Infrastructure
//! 
//! This crate provides the reorganized networking infrastructure with clear module boundaries
//! and consolidated functionality. The previous scattered implementations have been unified.

pub mod error;
pub mod message;

// New unified modules
pub mod transports;
pub mod routing;
pub mod actors;
pub mod discovery; 
pub mod protocol;

// Performance and monitoring modules
pub mod performance;
pub mod time;

// Re-export commonly used types
pub use error::{NetworkError, Result, TransportError};
pub use message::{NetworkMessage, ByteMessage, NetworkEnvelope, NetworkPriority, Priority};

// Re-export from new unified modules
pub use transports::{Transport, TransportFactory, TransportConfig, TransportType, TransportInfo};
pub use routing::{Router, RouterFactory, RoutingStrategy, RoutingDecision};
pub use actors::{ActorSystem, ActorSystemBuilder, ActorTransport};
pub use discovery::{ServiceDiscovery, ServiceDiscoveryFactory, ServiceLocation};
pub use protocol::{ProtocolProcessor, ProtocolConfig};

// Re-export time functions for external use
pub use time::{
    CachedClock, fast_timestamp_ns, current_timestamp_ns, precise_timestamp_ns,
    init_timestamp_system, parse_external_timestamp_safe, parse_external_unix_timestamp_safe,
    safe_duration_to_ns, safe_duration_to_ns_checked, safe_system_timestamp_ns, 
    safe_system_timestamp_ns_checked, timestamp_accuracy_info, timestamp_system_stats,
    TimestampError
};

// Constants for configuration
pub const DEFAULT_TCP_BUFFER_SIZE: usize = 64 * 1024; // 64KB
pub const DEFAULT_UDP_BUFFER_SIZE: usize = 64 * 1024; // 64KB
pub const DEFAULT_CONNECTION_POOL_SIZE: usize = 10;
pub const DEFAULT_CONNECTION_TIMEOUT_SECS: u64 = 30;
pub const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 60;
