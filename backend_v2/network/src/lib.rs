//! # High-Performance Network Transport System
//!
//! The `torq-network` crate provides a unified network transport system for AlphaPulse
//! that consolidates topology management and transport protocols into a single,
//! high-performance solution.
//!
//! ## Features
//!
//! - **Multiple Transport Modes**: Direct (TCP/UDP/QUIC), Message Queue, and Hybrid routing
//! - **Protocol V2 Integration**: TLV message validation with domain separation
//! - **Precision Handling**: DEX token and traditional exchange precision preservation
//! - **Topology Management**: Actor placement with NUMA awareness and resource optimization
//! - **Zero-Copy Operations**: High-performance serialization with zerocopy traits
//! - **Comprehensive Error Handling**: Context-preserving error types with retry logic
//!
//! ## Migration Guide
//!
//! This crate consolidates three previously separate crates into a unified solution:
//!
//! ### From `alphapulse-topology`
//! ```rust
//! // OLD (separate crate)
//! use alphapulse_topology::{TopologyConfig, TopologyResolver, Actor, ActorType};
//!
//! // NEW (unified crate)  
//! use torq_network::{TopologyConfig, TopologyResolver, Actor, ActorType};
//! ```
//!
//! ### From `alphapulse-transport`
//! ```rust
//! // OLD (separate crate)
//! use alphapulse_transport::{TransportConfig, TransportMode, ProtocolType};
//!
//! // NEW (unified crate)
//! use torq_network::{TransportConfig, TransportMode, ProtocolType};
//! ```
//!
//! ### From `alphapulse-network`
//! ```rust
//! // OLD (separate crate)
//! use alphapulse_network::{NetworkConfig, NetworkTransport};
//!
//! // NEW (unified crate - now called torq-network)
//! use torq_network::{NetworkConfig, NetworkTransport};
//! ```
//!
//! ### ChannelConfig Name Collision Resolution
//! The consolidation resolved a name collision between transport and topology `ChannelConfig` types:
//! ```rust
//! // Explicit type selection
//! use torq_network::{TransportChannelConfig, TopologyChannelConfig};
//!
//! // Default (backward compatible - uses transport version)
//! use torq_network::ChannelConfig; // Same as TransportChannelConfig
//! ```
//!
//! ### Cargo.toml Updates
//! ```toml
//! # OLD (separate crates)
//! [dependencies]
//! alphapulse-topology = { path = "../network/topology" }
//! alphapulse-transport = { path = "../network/transport" } 
//! alphapulse-network = { path = "../network" }
//!
//! # NEW (unified crate)
//! [dependencies]
//! torq-network = { path = "../network" }
//! ```
//!
//! ## Module Structure
//! ```
//! torq-network/
//! ├── topology/          # Actor placement and NUMA optimization (from alphapulse-topology)
//! │   ├── actors/        # Actor definitions and resource requirements
//! │   ├── nodes/         # Node configuration and capabilities  
//! │   ├── runtime/       # Actor runtime and lifecycle management
//! │   └── resolver/      # Actor-to-node placement resolution
//! ├── transport/         # Core transport abstractions (from alphapulse-transport)
//! │   ├── hybrid/        # Hybrid routing for different message types
//! │   ├── direct/        # Direct peer-to-peer networking
//! │   └── message_queue/ # Message queue backends (RabbitMQ, Kafka, Redis)
//! ├── network/           # Low-level network protocols (from alphapulse-network)
//! │   ├── tcp/           # TCP connection management
//! │   ├── udp/           # UDP high-frequency messaging
//! │   └── security/      # TLS encryption and authentication
//! ├── protocol_v2/       # Protocol V2 TLV message validation (new)
//! ├── precision/         # Financial precision handling (new)
//! └── error/            # Unified error types with context preservation
//! ```
//!
//! ## Key Design Decisions
//! - **Zero Breaking Changes**: All original APIs preserved with backward-compatible aliases
//! - **Feature Flags**: Optional dependencies for Protocol V2 integration and NUMA optimization
//! - **Performance First**: Maintains >1M msg/s throughput requirements
//! - **Error Context**: Improved error messages preserve original error context during conversion
//!
//! ## Protocol V2 Support
//!
//! Built-in support for AlphaPulse Protocol V2 TLV messages:
//! - 32-byte MessageHeader + variable TLV payload
//! - Domain separation: Market Data (1-19), Signals (20-39), Execution (40-79)
//! - Nanosecond timestamp precision
//! - Zero floating-point financial calculations
//!
//! Enable Protocol V2 features:
//! ```toml
//! [dependencies]
//! torq-network = { path = "../network", features = ["protocol-integration"] }
//! ```
//!
//! ## Performance Characteristics
//!
//! - **TCP Direct**: <5ms latency for inter-node communication
//! - **UDP Direct**: <1ms latency for trading signals
//! - **Shared Memory**: <35μs for same-node communication
//! - **Throughput**: >10,000 messages/second per connection
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use torq_network::{TransportConfig, NetworkTransport, TransportMode, ProtocolType, CompressionType};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create transport configuration
//! let config = TransportConfig::builder()
//!     .mode(TransportMode::Direct)
//!     .protocol(ProtocolType::Tcp)
//!     .compression(CompressionType::Lz4)
//!     .build()?;
//!
//! // Initialize transport
//! let mut transport = NetworkTransport::new(config).await?;
//! transport.start().await?;
//!
//! // Send message to remote actor
//! let message = b"market_data_update";
//! transport.send_to_actor("remote_node", "price_analyzer", message).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Performance
//!
//! - **TCP Direct**: <5ms latency for inter-node communication
//! - **UDP Direct**: <1ms latency for trading signals
//! - **Shared Memory**: <35μs for same-node communication
//! - **Throughput**: >10,000 messages/second per connection
//!
//! # Transport Selection
//!
//! Transport selection is automatic based on:
//! - Actor placement (same node vs different nodes)
//! - Channel criticality (latency vs reliability requirements)
//! - Network topology (same datacenter vs cross-region)
//! - Security requirements (encrypted vs plain)

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod error;
pub mod mycelium;
pub mod network;
pub mod performance;
pub mod precision;
pub mod protocol_v2;
pub mod time;
pub mod topology;
pub mod transport;

pub mod hybrid;
pub mod topology_integration;

// Re-export main types
pub use error::{Result, TransportError};

// Re-export network transport types
pub use network::{
    CompressionEngine, CompressionType, Connection, ConnectionConfig, ConnectionPool,
    EncryptionType, NetworkConfig, NetworkEnvelope, NetworkProtocol, NetworkTransport,
    PerformanceConfig, ProtocolOptions, ProtocolType, SecurityLayer, TcpOptions, UdpOptions,
    UnixSocketConfig, UnixSocketConnection, UnixSocketTransport,
};

// Re-export hybrid transport types
pub use hybrid::{
    ChannelConfig as TransportChannelConfig, HybridTransport, TransportBridge, TransportConfig, TransportMode,
    TransportRouter,
};

// Re-export topology types
pub use topology::{
    Actor, ActorPersistence, ActorState, ActorType, ActorPlacement,
    ChannelConfig as TopologyChannelConfig, DeploymentEngine, Node, TopologyConfig,
    TopologyError, TopologyResolver, TOPOLOGY_VERSION, MAX_ACTORS_PER_NODE, MAX_CPU_CORES_PER_ACTOR,
};

// Re-export topology integration
pub use topology_integration::{TopologyIntegration, TopologyTransportResolver, TransportFactory};

// Convenience re-exports with clear naming to avoid confusion
pub use TransportChannelConfig as ChannelConfig; // Default to transport version for backward compatibility

// Re-export Protocol V2 validation
pub use protocol_v2::{
    ProtocolV2Validator, ValidationResult, TLVValidation, TLVTypeRange,
    validate_timestamp_precision, validate_no_float_in_price,
};

// Re-export precision handling
pub use precision::{
    TokenAmount, ExchangePrice, TokenSymbol, PrecisionConverter,
    validate_precision, validate_no_floating_point, create_precision_registry,
};

// Re-export time system
pub use time::{
    fast_timestamp_ns as current_timestamp_ns, fast_timestamp_ns, init_timestamp_system,
    precise_timestamp_ns, safe_duration_to_ns, safe_duration_to_ns_checked,
    safe_system_timestamp_ns, safe_system_timestamp_ns_checked, timestamp_accuracy_info,
    timestamp_system_stats, TimestampError,
};

// #[cfg(feature = "monitoring")]
// pub use monitoring::{
//     TransportMetrics, HealthMonitor, CircuitBreaker, TransportTracing,
// };

/// Transport system version
pub const TRANSPORT_VERSION: &str = "0.1.0";

/// Maximum message size for network transport (16MB)
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// Default connection pool size per remote node
pub const DEFAULT_CONNECTION_POOL_SIZE: usize = 4;

/// Default TCP buffer size (64KB)
pub const DEFAULT_TCP_BUFFER_SIZE: usize = 64 * 1024;

/// Default UDP buffer size (8KB - fits in single ethernet frame)
pub const DEFAULT_UDP_BUFFER_SIZE: usize = 8 * 1024;

/// Default heartbeat interval (5 seconds)
pub const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 5;

/// Default connection timeout (10 seconds)
pub const DEFAULT_CONNECTION_TIMEOUT_SECS: u64 = 10;

/// Transport criticality levels for automatic selection
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Criticality {
    /// Ultra-low latency required (<1ms) - trading signals
    UltraLowLatency,
    /// Low latency required (<5ms) - market data
    LowLatency,
    /// Standard latency acceptable (<50ms) - general communication
    Standard,
    /// High latency acceptable (>50ms) - audit, compliance
    HighLatency,
}

/// Transport reliability requirements
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Reliability {
    /// Best effort delivery - may lose messages
    BestEffort,
    /// At-least-once delivery - may duplicate messages
    AtLeastOnce,
    /// Exactly-once delivery - guaranteed delivery without duplication
    ExactlyOnce,
    /// Guaranteed delivery with persistence
    GuaranteedDelivery,
}

impl Reliability {
    /// Check if this reliability level requires guaranteed delivery
    pub fn requires_guaranteed_delivery(&self) -> bool {
        matches!(
            self,
            Reliability::ExactlyOnce | Reliability::GuaranteedDelivery
        )
    }
}

/// Message priority levels
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Priority {
    /// Background priority - process when resources available
    Background = 0,
    /// Normal priority - standard processing
    Normal = 1,
    /// High priority - expedited processing
    High = 2,
    /// Critical priority - immediate processing
    Critical = 3,
}

// Transport trait is defined below with full functionality at line 273

/// Transport endpoint configuration
#[derive(Debug, Clone)]
pub struct EndpointConfig {
    /// Transport mode to use
    pub mode: TransportMode,
    /// Protocol for direct transport
    pub protocol: Option<ProtocolType>,
    /// Compression configuration
    pub compression: CompressionType,
    /// Encryption configuration
    pub encryption: EncryptionType,
    /// Message priority
    pub priority: Priority,
    /// Criticality level
    pub criticality: Criticality,
    /// Reliability requirements
    pub reliability: Reliability,
    /// Maximum message size
    pub max_message_size: usize,
    /// Connection timeout
    pub connection_timeout_secs: u64,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            mode: TransportMode::Auto,
            protocol: None, // Auto-select based on requirements
            compression: CompressionType::None,
            encryption: EncryptionType::None,
            priority: Priority::Normal,
            criticality: Criticality::Standard,
            reliability: Reliability::BestEffort,
            max_message_size: MAX_MESSAGE_SIZE,
            connection_timeout_secs: DEFAULT_CONNECTION_TIMEOUT_SECS,
        }
    }
}

impl EndpointConfig {
    /// Create configuration for ultra-low latency trading signals
    pub fn ultra_low_latency() -> Self {
        Self {
            mode: TransportMode::Direct,
            protocol: Some(ProtocolType::Udp),
            compression: CompressionType::None,
            encryption: EncryptionType::None,
            priority: Priority::Critical,
            criticality: Criticality::UltraLowLatency,
            reliability: Reliability::BestEffort,
            max_message_size: DEFAULT_UDP_BUFFER_SIZE,
            connection_timeout_secs: 1,
        }
    }

    /// Create configuration for high-throughput market data
    pub fn high_throughput() -> Self {
        Self {
            mode: TransportMode::Direct,
            protocol: Some(ProtocolType::Tcp),
            compression: CompressionType::Lz4,
            encryption: EncryptionType::None,
            priority: Priority::High,
            criticality: Criticality::LowLatency,
            reliability: Reliability::AtLeastOnce,
            max_message_size: MAX_MESSAGE_SIZE,
            connection_timeout_secs: 5,
        }
    }

    /// Create configuration for reliable audit/compliance data
    pub fn guaranteed_delivery() -> Self {
        Self {
            mode: TransportMode::MessageQueue,
            protocol: None, // MQ handles protocol
            compression: CompressionType::Zstd,
            encryption: EncryptionType::Tls,
            priority: Priority::Normal,
            criticality: Criticality::HighLatency,
            reliability: Reliability::GuaranteedDelivery,
            max_message_size: MAX_MESSAGE_SIZE,
            connection_timeout_secs: DEFAULT_CONNECTION_TIMEOUT_SECS,
        }
    }
}

/// Trait for transport implementations
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    /// Start the transport system
    async fn start(&mut self) -> Result<()>;

    /// Stop the transport system
    async fn stop(&mut self) -> Result<()>;

    /// Send message to a specific actor on a remote node
    async fn send_to_actor(
        &self,
        target_node: &str,
        target_actor: &str,
        message: &[u8],
    ) -> Result<()>;

    /// Send message with priority
    async fn send_with_priority(
        &self,
        target_node: &str,
        target_actor: &str,
        message: &[u8],
        priority: Priority,
    ) -> Result<()>;

    /// Check if transport is healthy
    fn is_healthy(&self) -> bool;

    /// Get transport statistics
    fn statistics(&self) -> TransportStatistics;
}

/// Transport performance statistics
#[derive(Debug, Clone, Default)]
pub struct TransportStatistics {
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received  
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Connection errors
    pub connection_errors: u64,
    /// Average latency in microseconds
    pub avg_latency_us: f64,
    /// Messages per second (recent)
    pub messages_per_second: f64,
    /// Active connections
    pub active_connections: u32,
}

/// Current nanosecond timestamp
///
/// **Deprecated**: Use `fast_timestamp_ns()` from the time module instead.
/// This function is kept for backward compatibility and delegates to the
/// authoritative timestamp implementation.
#[inline]
pub fn current_nanos() -> u64 {
    time::fast_timestamp_ns()
}

/// Generate unique message ID
#[inline]
pub fn generate_message_id() -> u64 {
    uuid::Uuid::new_v4().as_u128() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_criticality_ordering() {
        assert!(Criticality::UltraLowLatency < Criticality::LowLatency);
        assert!(Criticality::LowLatency < Criticality::Standard);
        assert!(Criticality::Standard < Criticality::HighLatency);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Background < Priority::Normal);
        assert!(Priority::Normal < Priority::High);
        assert!(Priority::High < Priority::Critical);
    }

    #[test]
    fn test_endpoint_config_presets() {
        let ultra_low = EndpointConfig::ultra_low_latency();
        assert_eq!(ultra_low.mode, TransportMode::Direct);
        assert_eq!(ultra_low.protocol, Some(ProtocolType::Udp));
        assert_eq!(ultra_low.criticality, Criticality::UltraLowLatency);

        let high_throughput = EndpointConfig::high_throughput();
        assert_eq!(high_throughput.protocol, Some(ProtocolType::Tcp));
        assert_eq!(high_throughput.compression, CompressionType::Lz4);

        let guaranteed = EndpointConfig::guaranteed_delivery();
        assert_eq!(guaranteed.mode, TransportMode::MessageQueue);
        assert_eq!(guaranteed.reliability, Reliability::GuaranteedDelivery);
    }

    #[test]
    fn test_constants() {
        assert_eq!(TRANSPORT_VERSION, "0.1.0");
        assert_eq!(MAX_MESSAGE_SIZE, 16 * 1024 * 1024);
        assert_eq!(DEFAULT_CONNECTION_POOL_SIZE, 4);
    }

    #[test]
    fn test_statistics_default() {
        let stats = TransportStatistics::default();
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.avg_latency_us, 0.0);
    }

    #[test]
    fn test_utility_functions() {
        let now = current_nanos();
        assert!(now > 0);

        let id1 = generate_message_id();
        let id2 = generate_message_id();
        assert_ne!(id1, id2);
    }
}
