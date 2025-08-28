//! Network Transport Abstraction
//!
//! Defines transport protocols, connection management, and failure handling
//! for inter-node communication in the topology system.

use super::error::{Result, TopologyError};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

/// Transport configuration for actor communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum Transport {
    /// Shared memory for same-node communication
    SharedMemory {
        channel_name: String,
        numa_optimized: bool,
        buffer_size: usize,
        huge_pages: bool,
    },
    /// Network transport for inter-node communication
    Network {
        protocol: NetworkProtocol,
        compression: CompressionType,
        routing: NetworkRoute,
    },
}

/// Network protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkProtocol {
    pub protocol_type: ProtocolType,
    pub addressing: AddressResolution,
    pub connection: ConnectionConfig,
    pub reliability: ReliabilityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolType {
    Tcp,
    Udp,
    Rdma,       // For high-performance clusters
    InfiniBand, // For HPC environments
}

/// Address resolution and service discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressResolution {
    pub method: ResolutionMethod,
    pub timeout: Duration,
    pub cache_ttl: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolutionMethod {
    /// Static IP:port configuration
    Static { addresses: Vec<SocketAddr> },
    /// DNS-based resolution
    Dns { hostname: String, port: u16 },
    /// Service discovery (Consul, etcd, etc.)
    ServiceDiscovery {
        service_name: String,
        discovery_url: String,
    },
}

/// Connection management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub pool_size: usize,
    pub connect_timeout: Duration,
    pub idle_timeout: Duration,
    pub keepalive_interval: Duration,
    pub max_reconnect_attempts: usize,
    pub backoff_strategy: BackoffStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Linear { increment: Duration },
    Exponential { base: Duration, max: Duration },
    Fibonacci { base: Duration, max: Duration },
}

/// Reliability and failure handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReliabilityConfig {
    pub failure_detection: FailureDetection,
    pub recovery_strategy: RecoveryStrategy,
    pub circuit_breaker: CircuitBreakerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureDetection {
    pub heartbeat_interval: Duration,
    pub failure_threshold: usize,
    pub recovery_threshold: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    Retry { max_attempts: usize },
    Failover { backup_addresses: Vec<SocketAddr> },
    CircuitBreaker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: usize,
    pub success_threshold: usize,
    pub timeout: Duration,
    pub half_open_max_calls: usize,
}

/// Data compression options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Lz4,
    Zstd { level: i32 },
    Snappy,
}

/// Network routing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRoute {
    pub source_node: String,
    pub target_node: String,
    pub route_path: Vec<String>, // Intermediate hops for complex topologies
    pub bandwidth_mbps: Option<u64>,
    pub latency_ms: Option<f64>,
}

impl Default for NetworkProtocol {
    fn default() -> Self {
        Self {
            protocol_type: ProtocolType::Tcp,
            addressing: AddressResolution {
                method: ResolutionMethod::Dns {
                    hostname: "localhost".to_string(),
                    port: 8080,
                },
                timeout: Duration::from_secs(5),
                cache_ttl: Duration::from_secs(300),
            },
            connection: ConnectionConfig {
                pool_size: 10,
                connect_timeout: Duration::from_secs(5),
                idle_timeout: Duration::from_secs(60),
                keepalive_interval: Duration::from_secs(30),
                max_reconnect_attempts: 3,
                backoff_strategy: BackoffStrategy::Exponential {
                    base: Duration::from_millis(100),
                    max: Duration::from_secs(10),
                },
            },
            reliability: ReliabilityConfig {
                failure_detection: FailureDetection {
                    heartbeat_interval: Duration::from_secs(10),
                    failure_threshold: 3,
                    recovery_threshold: 2,
                },
                recovery_strategy: RecoveryStrategy::Retry { max_attempts: 3 },
                circuit_breaker: CircuitBreakerConfig {
                    failure_threshold: 5,
                    success_threshold: 2,
                    timeout: Duration::from_secs(60),
                    half_open_max_calls: 1,
                },
            },
        }
    }
}

impl Transport {
    /// Create optimized transport for same-node communication
    pub fn shared_memory(channel_name: String, numa_node: Option<u8>) -> Self {
        Self::SharedMemory {
            channel_name,
            numa_optimized: numa_node.is_some(),
            buffer_size: 64 * 1024 * 1024,   // 64MB default
            huge_pages: numa_node.is_some(), // Use huge pages for NUMA
        }
    }

    /// Create network transport with defaults
    pub fn network(source_node: String, target_node: String) -> Self {
        Self::Network {
            protocol: NetworkProtocol::default(),
            compression: CompressionType::Lz4, // Good balance of speed/compression
            routing: NetworkRoute {
                source_node,
                target_node,
                route_path: vec![],
                bandwidth_mbps: None,
                latency_ms: None,
            },
        }
    }

    /// Create high-performance transport for low-latency requirements
    pub fn high_performance(source_node: String, target_node: String) -> Self {
        let mut protocol = NetworkProtocol {
            protocol_type: ProtocolType::Rdma, // Use RDMA if available
            ..Default::default()
        };
        protocol.connection.pool_size = 1; // Single dedicated connection
        protocol.connection.connect_timeout = Duration::from_millis(100);

        Self::Network {
            protocol,
            compression: CompressionType::None, // No compression for speed
            routing: NetworkRoute {
                source_node,
                target_node,
                route_path: vec![],
                bandwidth_mbps: Some(10_000), // 10Gbps
                latency_ms: Some(0.1),        // Sub-millisecond target
            },
        }
    }

    /// Validate transport configuration
    pub fn validate(&self) -> Result<()> {
        match self {
            Transport::SharedMemory { buffer_size, .. } => {
                if *buffer_size == 0 {
                    return Err(TopologyError::Config {
                        message: "Shared memory buffer size cannot be zero".to_string(),
                    });
                }
                if *buffer_size > 16 * 1024 * 1024 * 1024 {
                    return Err(TopologyError::Config {
                        message: "Shared memory buffer size too large (>16GB)".to_string(),
                    });
                }
                Ok(())
            }
            Transport::Network {
                protocol, routing, ..
            } => {
                // Validate connection pool size
                if protocol.connection.pool_size == 0 {
                    return Err(TopologyError::NetworkConfig {
                        message: "Connection pool size cannot be zero".to_string(),
                    });
                }

                // Validate timeouts are reasonable
                if protocol.connection.connect_timeout.as_millis() > 30_000 {
                    return Err(TopologyError::NetworkConfig {
                        message: "Connect timeout too large (>30s)".to_string(),
                    });
                }

                // Validate source != target
                if routing.source_node == routing.target_node {
                    return Err(TopologyError::NetworkConfig {
                        message: "Source and target nodes cannot be the same".to_string(),
                    });
                }

                Ok(())
            }
        }
    }

    /// Get expected latency characteristics
    pub fn expected_latency(&self) -> Duration {
        match self {
            Transport::SharedMemory { numa_optimized, .. } => {
                if *numa_optimized {
                    Duration::from_nanos(100) // ~100ns for NUMA-optimized
                } else {
                    Duration::from_micros(1) // ~1μs for cross-NUMA
                }
            }
            Transport::Network { routing, .. } => {
                routing
                    .latency_ms
                    .map(|ms| Duration::from_millis(ms as u64))
                    .unwrap_or(Duration::from_millis(1)) // Default 1ms
            }
        }
    }

    /// Get expected bandwidth
    pub fn expected_bandwidth(&self) -> u64 {
        match self {
            Transport::SharedMemory { .. } => {
                100_000 // ~100GB/s for shared memory
            }
            Transport::Network { routing, .. } => {
                routing.bandwidth_mbps.unwrap_or(1000) // Default 1Gbps
            }
        }
    }
}
