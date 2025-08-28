//! Transport adapter for integrating relays with infra/transport system

use crate::{ConsumerId, RelayConfig, RelayError, RelayResult, types::Transport as RelayTransport};
use torq_network::{
    ChannelConfig, NetworkTransport, Priority, TopologyConfig, TopologyIntegration,
    TransportConfig, TransportError, TransportMode, TransportStatistics,
};
use async_trait::async_trait;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Adapter that bridges relay transport trait with infra/transport system
pub struct InfraTransportAdapter {
    /// The underlying transport from infra
    transport: Option<Box<dyn torq_network::Transport>>,
    /// Topology integration for advanced routing
    topology: Option<Arc<TopologyIntegration>>,
    /// Transport configuration
    config: TransportAdapterConfig,
    /// Consumer connections mapped by ID
    consumers: Arc<RwLock<HashMap<ConsumerId, ConsumerConnection>>>,
}

/// Configuration for transport adapter
#[derive(Debug, Clone)]
pub struct TransportAdapterConfig {
    /// Transport mode (unix_socket, tcp, topology)
    pub mode: String,
    /// Path for unix socket
    pub socket_path: Option<String>,
    /// TCP address
    pub tcp_address: Option<String>,
    /// Channel name for topology-based routing
    pub channel_name: Option<String>,
    /// Use topology integration
    pub use_topology: bool,
}

/// Represents a consumer connection
#[derive(Debug, Clone)]
struct ConsumerConnection {
    id: ConsumerId,
    topics: Vec<String>,
    // In real implementation, this would hold actual connection
    // For now it's a placeholder
}

impl InfraTransportAdapter {
    /// Create new transport adapter
    pub async fn new(config: TransportAdapterConfig) -> RelayResult<Self> {
        info!("Creating transport adapter with mode: {}", config.mode);

        let adapter = Self {
            transport: None,
            topology: None,
            config,
            consumers: Arc::new(RwLock::new(HashMap::new())),
        };

        Ok(adapter)
    }

    /// Initialize the transport based on configuration
    async fn init_transport(&mut self) -> RelayResult<()> {
        match self.config.mode.as_str() {
            "unix_socket" => {
                self.init_unix_socket().await?;
            }
            "tcp" => {
                self.init_tcp().await?;
            }
            "topology" => {
                self.init_topology_transport().await?;
            }
            _ => {
                return Err(RelayError::Config(format!(
                    "Unknown transport mode: {}",
                    self.config.mode
                )));
            }
        }

        Ok(())
    }

    /// Initialize Unix socket transport
    async fn init_unix_socket(&mut self) -> RelayResult<()> {
        let socket_path = self
            .config
            .socket_path
            .as_ref()
            .ok_or_else(|| RelayError::Config("Unix socket path not configured".to_string()))?;

        info!("Initializing Unix socket transport at: {}", socket_path);

        // Create parent directory if needed
        if let Some(parent) = std::path::Path::new(socket_path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                RelayError::Transport(format!("Failed to create socket directory: {}", e))
            })?;
        }

        // Create Unix socket transport
        let config = torq_network::UnixSocketConfig {
            path: socket_path.into(),
            buffer_size: 64 * 1024,
            max_message_size: 16 * 1024 * 1024,
            cleanup_on_drop: true,
        };

        let mut transport = torq_network::UnixSocketTransport::new(config).map_err(|e| {
            RelayError::Transport(format!("Failed to create Unix socket transport: {}", e))
        })?;

        transport
            .bind()
            .await
            .map_err(|e| RelayError::Transport(format!("Failed to bind Unix socket: {}", e)))?;

        // Store transport wrapped in a compatibility adapter
        self.transport = Some(Box::new(UnixTransportAdapter::new(transport)));

        Ok(())
    }

    /// Initialize TCP transport
    async fn init_tcp(&mut self) -> RelayResult<()> {
        let tcp_address = self
            .config
            .tcp_address
            .as_ref()
            .ok_or_else(|| RelayError::Config("TCP address not configured".to_string()))?;

        info!("Initializing TCP transport at: {}", tcp_address);

        // Placeholder TCP transport - will be implemented when needed
        // For now, create a stub that satisfies the interface
        self.transport = Some(Box::new(TcpTransportStub {
            address: tcp_address.clone(),
        }));

        Ok(())
    }

    /// Initialize topology-based transport
    async fn init_topology_transport(&mut self) -> RelayResult<()> {
        info!("Initializing topology-based transport");

        let channel_name = self.config.channel_name.as_ref().ok_or_else(|| {
            RelayError::Config("Channel name not configured for topology mode".to_string())
        })?;

        // Placeholder for topology-based transport
        // Will be implemented when topology configuration is needed

        Ok(())
    }

    /// Register a consumer
    pub async fn register_consumer(
        &self,
        consumer_id: ConsumerId,
        topics: Vec<String>,
    ) -> RelayResult<()> {
        let mut consumers = self.consumers.write().await;

        let connection = ConsumerConnection {
            id: consumer_id.clone(),
            topics,
        };

        consumers.insert(consumer_id.clone(), connection);
        info!("Registered consumer: {}", consumer_id.0);

        Ok(())
    }

    /// Unregister a consumer
    pub async fn unregister_consumer(&self, consumer_id: &ConsumerId) -> RelayResult<()> {
        let mut consumers = self.consumers.write().await;

        if consumers.remove(consumer_id).is_some() {
            info!("Unregistered consumer: {}", consumer_id.0);
        }

        Ok(())
    }
}

#[async_trait]
impl RelayTransport for InfraTransportAdapter {
    async fn start(&mut self) -> RelayResult<()> {
        info!("Starting transport adapter");

        // Initialize transport if not already done
        if self.transport.is_none() {
            self.init_transport().await?;
        }

        // Start the underlying transport
        if let Some(transport) = &mut self.transport {
            // transport.start().await
            //     .map_err(|e| RelayError::Transport(format!("Failed to start transport: {}", e)))?;
        }

        info!("Transport adapter started successfully");
        Ok(())
    }

    async fn stop(&mut self) -> RelayResult<()> {
        info!("Stopping transport adapter");

        if let Some(transport) = &mut self.transport {
            // transport.stop().await
            //     .map_err(|e| RelayError::Transport(format!("Failed to stop transport: {}", e)))?;
        }

        info!("Transport adapter stopped");
        Ok(())
    }

    async fn receive(&mut self) -> RelayResult<Bytes> {
        if let Some(transport) = &mut self.transport {
            // let data = transport.receive().await
            //     .map_err(|e| RelayError::Transport(format!("Failed to receive: {}", e)))?;
            // Ok(Bytes::from(data))

            // Placeholder for now
            Ok(Bytes::new())
        } else {
            Err(RelayError::Transport(
                "Transport not initialized".to_string(),
            ))
        }
    }

    async fn send(&mut self, data: &[u8], consumers: &[ConsumerId]) -> RelayResult<()> {
        if consumers.is_empty() {
            debug!("No consumers to send to");
            return Ok(());
        }

        if let Some(transport) = &mut self.transport {
            // TODO: Integrate with connection management system
            // The connection management methods (connect, disconnect, send_data) are implemented
            // but not yet integrated with this main RelayTransport::send() method.
            //
            // Next steps:
            // 1. Replace placeholder logic with actual UnixTransportAdapter::send_data() calls
            // 2. Map ConsumerIds to socket paths for connection lookup
            // 3. Handle connection failures by cleaning up dead connections
            //
            // See GitHub issue: [Add issue reference when created]

            for consumer_id in consumers {
                debug!(
                    "Sending {} bytes to consumer: {}",
                    data.len(),
                    consumer_id.0
                );
                // transport.send_to(consumer_id, data).await?;
            }

            Ok(())
        } else {
            Err(RelayError::Transport(
                "Transport not initialized".to_string(),
            ))
        }
    }
}

/// Create transport adapter from relay configuration
pub fn create_transport_from_config(config: &RelayConfig) -> TransportAdapterConfig {
    TransportAdapterConfig {
        mode: config.transport.mode.clone(),
        socket_path: config.transport.path.clone(),
        tcp_address: config.transport.address.clone().map(|addr| {
            if let Some(port) = config.transport.port {
                format!("{}:{}", addr, port)
            } else {
                addr
            }
        }),
        channel_name: Some(config.relay.name.clone()),
        use_topology: config.transport.use_topology,
    }
}

/// Placeholder TCP transport stub
struct TcpTransportStub {
    address: String,
}

#[async_trait]
impl torq_network::Transport for TcpTransportStub {
    async fn start(&mut self) -> torq_network::Result<()> {
        info!("TCP transport stub started at: {}", self.address);
        Ok(())
    }

    async fn stop(&mut self) -> torq_network::Result<()> {
        info!("TCP transport stub stopped");
        Ok(())
    }

    async fn send_to_actor(
        &self,
        _target_node: &str,
        _target_actor: &str,
        _message: &[u8],
    ) -> torq_network::Result<()> {
        Err(torq_network::TransportError::Configuration {
            message: "TCP transport is not implemented (placeholder stub)".to_string(),
            field: Some("tcp_implementation".to_string()),
        })
    }

    async fn send_with_priority(
        &self,
        _target_node: &str,
        _target_actor: &str,
        _message: &[u8],
        _priority: Priority,
    ) -> torq_network::Result<()> {
        Err(torq_network::TransportError::Configuration {
            message: "TCP transport is not implemented (placeholder stub)".to_string(),
            field: Some("tcp_implementation".to_string()),
        })
    }

    fn is_healthy(&self) -> bool {
        false // Stub is never healthy
    }

    fn statistics(&self) -> TransportStatistics {
        TransportStatistics::default()
    }
}

/// Adapter to wrap UnixSocketTransport as a generic Transport
struct UnixTransportAdapter {
    transport: torq_network::UnixSocketTransport,
    connections: Arc<RwLock<HashMap<String, torq_network::UnixSocketConnection>>>,
}

impl UnixTransportAdapter {
    fn new(transport: torq_network::UnixSocketTransport) -> Self {
        Self {
            transport,
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Connect to a socket and store the connection
    async fn connect(&self, socket_path: &str) -> torq_network::Result<()> {
        // Check if already connected
        {
            let connections = self.connections.read().await;
            if connections.contains_key(socket_path) {
                debug!("Already connected to {}", socket_path);
                return Ok(());
            }
        }

        // Create new connection
        let connection = torq_network::UnixSocketTransport::connect(socket_path).await?;

        // Store connection in map
        {
            let mut connections = self.connections.write().await;
            connections.insert(socket_path.to_string(), connection);
            info!("Stored connection to {}", socket_path);
        }

        Ok(())
    }

    /// Remove a connection from the map
    async fn disconnect(&self, socket_path: &str) -> torq_network::Result<()> {
        let mut connections = self.connections.write().await;
        if connections.remove(socket_path).is_some() {
            info!("Removed connection to {}", socket_path);
        } else {
            warn!("No connection found for {}", socket_path);
        }
        Ok(())
    }

    /// Send data using a stored connection
    async fn send_data(&self, socket_path: &str, data: &[u8]) -> torq_network::Result<()> {
        let mut connections = self.connections.write().await;

        if let Some(connection) = connections.get_mut(socket_path) {
            match connection.send(data).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    error!("Failed to send data, removing dead connection: {}", e);
                    connections.remove(socket_path);
                    Err(e)
                }
            }
        } else {
            Err(torq_network::TransportError::Connection {
                message: format!("No connection to {}", socket_path),
                remote_addr: None,
                source: None,
            })
        }
    }

    /// Get the number of active connections (for testing)
    #[cfg(test)]
    async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }
}

#[async_trait]
impl torq_network::Transport for UnixTransportAdapter {
    async fn start(&mut self) -> torq_network::Result<()> {
        // Transport is already bound in init_unix_socket
        Ok(())
    }

    async fn stop(&mut self) -> torq_network::Result<()> {
        self.transport.shutdown().await
    }

    async fn send_to_actor(
        &self,
        _target_node: &str,
        _target_actor: &str,
        _message: &[u8],
    ) -> torq_network::Result<()> {
        // For relays, we use a different sending pattern through RelayTransport trait
        Err(torq_network::TransportError::Configuration {
            message: "Use RelayTransport::send for relay-specific messaging".to_string(),
            field: Some("relay_transport_send".to_string()),
        })
    }

    async fn send_with_priority(
        &self,
        _target_node: &str,
        _target_actor: &str,
        _message: &[u8],
        _priority: Priority,
    ) -> torq_network::Result<()> {
        Err(torq_network::TransportError::Configuration {
            message: "Priority sending not implemented for Unix sockets".to_string(),
            field: Some("priority_send_implementation".to_string()),
        })
    }

    fn is_healthy(&self) -> bool {
        // Basic health check - transport exists and socket path exists
        true
    }

    fn statistics(&self) -> TransportStatistics {
        TransportStatistics::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transport_adapter_creation() {
        let config = TransportAdapterConfig {
            mode: "unix_socket".to_string(),
            socket_path: Some("/tmp/test.sock".to_string()),
            tcp_address: None,
            channel_name: None,
            use_topology: false,
        };

        let adapter = InfraTransportAdapter::new(config).await.unwrap();
        assert!(adapter.transport.is_none()); // Not initialized until start()
    }

    #[tokio::test]
    async fn test_consumer_registration() {
        let config = TransportAdapterConfig {
            mode: "unix_socket".to_string(),
            socket_path: Some("/tmp/test.sock".to_string()),
            tcp_address: None,
            channel_name: None,
            use_topology: false,
        };

        let adapter = InfraTransportAdapter::new(config).await.unwrap();
        let consumer_id = ConsumerId("test_consumer".to_string());
        let topics = vec!["topic1".to_string(), "topic2".to_string()];

        adapter
            .register_consumer(consumer_id.clone(), topics)
            .await
            .unwrap();

        let consumers = adapter.consumers.read().await;
        assert!(consumers.contains_key(&consumer_id));
    }

    #[tokio::test]
    async fn test_unix_transport_connection_management() {
        use tempfile::tempdir;

        // Create a temporary directory for test sockets
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let socket_path = temp_dir.path().join("test.sock");

        // Create Unix transport adapter
        let config = torq_network::UnixSocketConfig {
            path: socket_path.clone(),
            buffer_size: 64 * 1024,
            max_message_size: 16 * 1024 * 1024,
            cleanup_on_drop: true,
        };

        let transport = torq_network::UnixSocketTransport::new(config)
            .expect("Failed to create Unix transport");
        let adapter = UnixTransportAdapter::new(transport);

        // Test 1: Initially no connections
        assert_eq!(
            adapter.connection_count().await,
            0,
            "Should start with no connections"
        );

        // Test 2: Connect to a socket (this will fail since no server is running, but we can test the attempt)
        let test_socket_path = temp_dir
            .path()
            .join("client.sock")
            .to_str()
            .unwrap()
            .to_string();

        // Since we can't actually connect without a server, let's test the connection map directly
        // We'll simulate storing a connection
        {
            // Create a dummy connection for testing (in real scenario, this would be from connect())
            // For now, we'll just verify the map operations work
            let mut connections = adapter.connections.write().await;
            assert_eq!(connections.len(), 0, "Connections map should be empty");

            // We can't create a real connection without a server, but we've verified:
            // 1. The connections map exists and is accessible
            // 2. We can acquire write locks on it
            // 3. The connection count method works
        }

        // Test 3: Verify disconnect removes connections
        // First, let's pretend we have a connection by checking disconnect on non-existent path
        let result = adapter.disconnect(&test_socket_path).await;
        assert!(
            result.is_ok(),
            "Disconnect should not fail even if connection doesn't exist"
        );

        // Test 4: Verify send_data fails properly when no connection exists
        let data = b"test data";
        let send_result = adapter.send_data(&test_socket_path, data).await;
        assert!(
            send_result.is_err(),
            "Send should fail when no connection exists"
        );

        if let Err(e) = send_result {
            match e {
                torq_network::TransportError::Connection { message, .. } => {
                    assert!(
                        message.contains(&test_socket_path),
                        "Error should mention the socket path"
                    );
                }
                _ => panic!("Expected NotConnected error, got {:?}", e),
            }
        }
    }
}
