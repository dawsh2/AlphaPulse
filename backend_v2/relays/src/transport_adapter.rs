//! Transport adapter for integrating relays with infra/transport system

use crate::{ConsumerId, RelayError, RelayResult, Transport as RelayTransport};
use alphapulse_topology::TopologyConfig;
use alphapulse_transport::{
    ChannelConfig, NetworkTransport, TopologyIntegration, TransportConfig, TransportError,
    TransportMode,
};
use async_trait::async_trait;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

/// Adapter that bridges relay transport trait with infra/transport system
pub struct InfraTransportAdapter {
    /// The underlying transport from infra
    transport: Option<Box<dyn alphapulse_transport::Transport>>,
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

        // TODO: Create actual Unix socket transport from infra/transport
        // For now, we're using a placeholder
        // let transport = UnixSocketTransport::new(socket_path)?;
        // self.transport = Some(Box::new(transport));

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

        // TODO: Create actual TCP transport from infra/transport
        // let config = NetworkConfig::builder()
        //     .protocol(ProtocolType::Tcp)
        //     .address(tcp_address)
        //     .build()?;
        // let transport = NetworkTransport::new(config).await?;
        // self.transport = Some(Box::new(transport));

        Ok(())
    }

    /// Initialize topology-based transport
    async fn init_topology_transport(&mut self) -> RelayResult<()> {
        info!("Initializing topology-based transport");

        let channel_name = self.config.channel_name.as_ref().ok_or_else(|| {
            RelayError::Config("Channel name not configured for topology mode".to_string())
        })?;

        // TODO: Load topology configuration and create transport
        // let topology_config = TopologyConfig::from_file("topology.yaml")?;
        // let transport_config = TransportConfig::from_file("transport.yaml")?;
        //
        // let topology = TopologyIntegration::new(topology_config, transport_config).await?;
        // self.topology = Some(Arc::new(topology));
        //
        // let transport = topology.create_transport_for_channel(channel_name).await?;
        // self.transport = Some(transport);

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
            // In real implementation, would send to specific consumers
            // For now, this is a placeholder

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
pub fn create_transport_from_config(config: &crate::RelayConfig) -> TransportAdapterConfig {
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
}
