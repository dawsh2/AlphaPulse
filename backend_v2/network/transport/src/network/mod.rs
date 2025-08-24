//! Network Transport Module
//!
//! Direct peer-to-peer network transport implementation supporting TCP, UDP,
//! and QUIC protocols for low-latency actor communication.

pub mod compression;
pub mod connection;
pub mod envelope;
pub mod security;
pub mod tcp;
pub mod udp;

// TODO: Implement QUIC module when needed
// #[cfg(feature = "quic")]
// pub mod quic;

// Re-export main types
pub use compression::{CompressionEngine, CompressionType};
pub use connection::{Connection, ConnectionManager, ConnectionPool, ConnectionStats};
pub use envelope::{NetworkEnvelope, WireFormat};
pub use security::{EncryptionType, SecurityLayer};
pub use tcp::{TcpConfig, TcpConnection, TcpTransport};
pub use udp::{UdpConfig, UdpTransport};

// #[cfg(feature = "quic")]
// pub use quic::{QuicTransport, QuicConfig};

use crate::{Result, TransportError};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

/// Network transport for inter-node communication
pub struct NetworkTransport {
    config: NetworkConfig,
    connection_manager: ConnectionManager,
    compression: CompressionEngine,
    security: SecurityLayer,
}

/// Network transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Node identifier
    pub node_id: String,
    /// Network protocol configuration
    pub protocol: NetworkProtocol,
    /// Compression settings
    pub compression: CompressionType,
    /// Encryption settings
    pub encryption: EncryptionType,
    /// Connection management
    pub connection: ConnectionConfig,
    /// Performance tuning
    pub performance: PerformanceConfig,
}

/// Network protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkProtocol {
    /// Protocol type
    pub protocol_type: ProtocolType,
    /// Listen address for incoming connections
    pub listen_addr: SocketAddr,
    /// Protocol-specific options
    pub options: ProtocolOptions,
}

/// Protocol type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProtocolType {
    /// TCP with connection pooling
    Tcp,
    /// UDP for ultra-low latency
    Udp,
    /// QUIC for modern encrypted transport
    #[cfg(feature = "quic")]
    Quic,
}

/// Protocol-specific options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolOptions {
    /// TCP options
    pub tcp: Option<TcpOptions>,
    /// UDP options
    pub udp: Option<UdpOptions>,
    /// QUIC options
    #[cfg(feature = "quic")]
    pub quic: Option<QuicOptions>,
}

/// TCP protocol options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpOptions {
    /// Disable Nagle's algorithm (TCP_NODELAY)
    pub nodelay: bool,
    /// Enable TCP keepalive
    pub keepalive: bool,
    /// TCP receive buffer size
    pub recv_buffer_size: Option<usize>,
    /// TCP send buffer size
    pub send_buffer_size: Option<usize>,
    /// Connection backlog for listen socket
    pub backlog: Option<u32>,
}

/// UDP protocol options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpOptions {
    /// UDP receive buffer size
    pub recv_buffer_size: Option<usize>,
    /// UDP send buffer size
    pub send_buffer_size: Option<usize>,
    /// Multicast configuration
    pub multicast: Option<MulticastConfig>,
}

/// QUIC protocol options
#[cfg(feature = "quic")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuicOptions {
    /// TLS certificate path
    pub cert_path: String,
    /// TLS private key path
    pub key_path: String,
    /// Maximum concurrent streams
    pub max_concurrent_streams: Option<u64>,
    /// Keep alive interval
    pub keep_alive_interval: Option<Duration>,
}

/// Multicast configuration for UDP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MulticastConfig {
    /// Multicast group address
    pub group_addr: SocketAddr,
    /// Network interface to use
    pub interface: Option<String>,
    /// Time-to-live for multicast packets
    pub ttl: Option<u32>,
}

/// Connection management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Maximum connections per remote node
    pub max_connections_per_node: usize,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Idle timeout before closing connection
    pub idle_timeout: Duration,
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Backoff strategy for reconnection
    pub backoff_strategy: BackoffStrategy,
}

/// Backoff strategy for connection retries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    /// Fixed delay between retries
    Fixed { delay: Duration },
    /// Linear backoff (delay increases linearly)
    Linear {
        initial_delay: Duration,
        increment: Duration,
        max_delay: Duration,
    },
    /// Exponential backoff (delay doubles each retry)
    Exponential {
        initial_delay: Duration,
        multiplier: f64,
        max_delay: Duration,
    },
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Message batching configuration
    pub batching: Option<BatchingConfig>,
    /// Send queue size per connection
    pub send_queue_size: usize,
    /// Receive queue size per connection
    pub recv_queue_size: usize,
    /// Worker thread count (None = auto-detect)
    pub worker_threads: Option<usize>,
    /// Enable zero-copy optimizations where possible
    pub zero_copy: bool,
}

/// Message batching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchingConfig {
    /// Maximum messages to batch together
    pub max_batch_size: usize,
    /// Maximum time to wait for batch to fill
    pub max_batch_delay: Duration,
    /// Enable adaptive batching based on load
    pub adaptive: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            protocol: NetworkProtocol {
                protocol_type: ProtocolType::Tcp,
                listen_addr: "0.0.0.0:8080".parse().unwrap(),
                options: ProtocolOptions {
                    tcp: Some(TcpOptions {
                        nodelay: true,
                        keepalive: true,
                        recv_buffer_size: Some(crate::DEFAULT_TCP_BUFFER_SIZE),
                        send_buffer_size: Some(crate::DEFAULT_TCP_BUFFER_SIZE),
                        backlog: Some(1024),
                    }),
                    udp: None,
                    #[cfg(feature = "quic")]
                    quic: None,
                },
            },
            compression: CompressionType::None,
            encryption: EncryptionType::None,
            connection: ConnectionConfig {
                max_connections_per_node: crate::DEFAULT_CONNECTION_POOL_SIZE,
                connect_timeout: Duration::from_secs(crate::DEFAULT_CONNECTION_TIMEOUT_SECS),
                idle_timeout: Duration::from_secs(300), // 5 minutes
                heartbeat_interval: Duration::from_secs(crate::DEFAULT_HEARTBEAT_INTERVAL_SECS),
                max_reconnect_attempts: 3,
                backoff_strategy: BackoffStrategy::Exponential {
                    initial_delay: Duration::from_millis(100),
                    multiplier: 2.0,
                    max_delay: Duration::from_secs(30),
                },
            },
            performance: PerformanceConfig {
                batching: None, // Disabled by default for low latency
                send_queue_size: 1000,
                recv_queue_size: 1000,
                worker_threads: None, // Auto-detect
                zero_copy: true,
            },
        }
    }
}

impl NetworkConfig {
    /// Create configuration optimized for ultra-low latency
    pub fn ultra_low_latency() -> Self {
        let mut config = Self {
            protocol: NetworkProtocol {
                protocol_type: ProtocolType::Udp,
                listen_addr: "0.0.0.0:8080".parse().unwrap(),
                options: ProtocolOptions {
                    tcp: None,
                    udp: Some(UdpOptions {
                        recv_buffer_size: Some(crate::DEFAULT_UDP_BUFFER_SIZE),
                        send_buffer_size: Some(crate::DEFAULT_UDP_BUFFER_SIZE),
                        multicast: None,
                    }),
                    #[cfg(feature = "quic")]
                    quic: None,
                },
            },
            compression: CompressionType::None,
            encryption: EncryptionType::None,
            ..Default::default()
        };

        // Optimize for latency
        config.performance.batching = None;
        config.performance.send_queue_size = 100; // Smaller queues
        config.performance.recv_queue_size = 100;
        config.connection.connect_timeout = Duration::from_secs(1);
        config.connection.heartbeat_interval = Duration::from_millis(100);

        config
    }

    /// Create configuration optimized for high throughput
    pub fn high_throughput() -> Self {
        let mut config = Self::default();

        // Enable compression for bandwidth efficiency
        config.compression = CompressionType::Lz4;

        // Enable batching for throughput
        config.performance.batching = Some(BatchingConfig {
            max_batch_size: 100,
            max_batch_delay: Duration::from_millis(1),
            adaptive: true,
        });

        // Larger buffers and queues
        if let Some(ref mut tcp_opts) = config.protocol.options.tcp {
            tcp_opts.recv_buffer_size = Some(256 * 1024); // 256KB
            tcp_opts.send_buffer_size = Some(256 * 1024);
        }
        config.performance.send_queue_size = 10000;
        config.performance.recv_queue_size = 10000;

        config
    }

    /// Create secure configuration with encryption
    pub fn secure() -> Self {
        let mut config = Self::default();
        config.encryption = EncryptionType::Tls;
        config
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate protocol-specific options
        match self.protocol.protocol_type {
            ProtocolType::Tcp => {
                if self.protocol.options.tcp.is_none() {
                    return Err(TransportError::configuration(
                        "TCP options required for TCP protocol",
                        Some("protocol.options.tcp"),
                    ));
                }
            }
            ProtocolType::Udp => {
                if self.protocol.options.udp.is_none() {
                    return Err(TransportError::configuration(
                        "UDP options required for UDP protocol",
                        Some("protocol.options.udp"),
                    ));
                }
            }
            #[cfg(feature = "quic")]
            ProtocolType::Quic => {
                if self.protocol.options.quic.is_none() {
                    return Err(TransportError::configuration(
                        "QUIC options required for QUIC protocol",
                        Some("protocol.options.quic"),
                    ));
                }
            }
        }

        // Validate timeouts
        if self.connection.connect_timeout.as_secs() == 0 {
            return Err(TransportError::configuration(
                "Connection timeout must be greater than 0",
                Some("connection.connect_timeout"),
            ));
        }

        if self.connection.idle_timeout < self.connection.heartbeat_interval {
            return Err(TransportError::configuration(
                "Idle timeout must be greater than heartbeat interval",
                Some("connection.idle_timeout"),
            ));
        }

        // Validate performance settings
        if self.performance.send_queue_size == 0 || self.performance.recv_queue_size == 0 {
            return Err(TransportError::configuration(
                "Queue sizes must be greater than 0",
                Some("performance"),
            ));
        }

        Ok(())
    }
}

impl NetworkTransport {
    /// Create new network transport with configuration
    pub async fn new(config: NetworkConfig) -> Result<Self> {
        config.validate()?;

        let compression = CompressionEngine::new(config.compression);
        let security = SecurityLayer::new(config.encryption.clone()).await?;
        let connection_manager = ConnectionManager::new(config.connection.clone());

        Ok(Self {
            config,
            connection_manager,
            compression,
            security,
        })
    }

    /// Start the network transport
    pub async fn start(&mut self) -> Result<()> {
        tracing::info!(
            node_id = %self.config.node_id,
            protocol = ?self.config.protocol.protocol_type,
            listen_addr = %self.config.protocol.listen_addr,
            "Starting network transport"
        );

        match self.config.protocol.protocol_type {
            ProtocolType::Tcp => {
                self.start_tcp_server().await?;
            }
            ProtocolType::Udp => {
                self.start_udp_server().await?;
            }
            #[cfg(feature = "quic")]
            ProtocolType::Quic => {
                self.start_quic_server().await?;
            }
        }

        Ok(())
    }

    /// Send message to remote node and actor
    pub async fn send_to_actor(
        &self,
        target_node: &str,
        target_actor: &str,
        message: &[u8],
    ) -> Result<()> {
        // Get or create connection to target node
        let connection = self
            .connection_manager
            .get_or_create_connection(target_node)
            .await?;

        // Compress message if configured
        let compressed_payload = self.compression.compress(message)?;

        // Encrypt if configured
        let encrypted_payload = self.security.encrypt(&compressed_payload).await?;

        // Create network envelope
        let envelope = NetworkEnvelope::new(
            self.config.node_id.clone(),
            target_node.to_string(),
            target_actor.to_string(),
            encrypted_payload,
            self.config.compression,
            self.config.encryption.clone(),
        );

        // Send envelope through connection
        let envelope_bytes = bincode::serialize(&envelope)
            .map_err(|e| TransportError::network(format!("Failed to serialize envelope: {}", e)))?;
        connection.send(&envelope_bytes).await?;

        Ok(())
    }

    /// Get transport configuration
    pub fn config(&self) -> &NetworkConfig {
        &self.config
    }

    // Private helper methods for starting protocol-specific servers
    async fn start_tcp_server(&mut self) -> Result<()> {
        // Implementation will be in tcp.rs
        todo!("TCP server implementation")
    }

    async fn start_udp_server(&mut self) -> Result<()> {
        // Implementation will be in udp.rs
        todo!("UDP server implementation")
    }

    #[cfg(feature = "quic")]
    async fn start_quic_server(&mut self) -> Result<()> {
        // Implementation will be in quic.rs
        todo!("QUIC server implementation")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NetworkConfig::default();
        assert_eq!(config.protocol.protocol_type, ProtocolType::Tcp);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_ultra_low_latency_config() {
        let config = NetworkConfig::ultra_low_latency();
        assert_eq!(config.protocol.protocol_type, ProtocolType::Udp);
        assert_eq!(config.compression, CompressionType::None);
        assert!(config.performance.batching.is_none());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_high_throughput_config() {
        let config = NetworkConfig::high_throughput();
        assert_eq!(config.compression, CompressionType::Lz4);
        assert!(config.performance.batching.is_some());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation() {
        let mut config = NetworkConfig::default();

        // Test invalid timeout
        config.connection.connect_timeout = Duration::from_secs(0);
        assert!(config.validate().is_err());

        // Test invalid queue size
        config.connection.connect_timeout = Duration::from_secs(10);
        config.performance.send_queue_size = 0;
        assert!(config.validate().is_err());
    }
}
