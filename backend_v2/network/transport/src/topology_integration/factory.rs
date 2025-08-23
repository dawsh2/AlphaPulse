//! Transport Factory
//!
//! Creates appropriate transport instances based on topology configuration.

use crate::{Transport, TransportError, Result, Priority, TransportStatistics};
use super::resolver::{TopologyResolver, NodeAddress};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info};

/// Factory for creating transport instances
pub struct TransportFactory {
    resolver: Arc<TopologyResolver>,
}

impl TransportFactory {
    /// Create new transport factory
    pub fn new(resolver: Arc<TopologyResolver>) -> Self {
        Self { resolver }
    }
    
    /// Update configuration
    pub async fn update_config(&self, config: crate::hybrid::TransportConfig) -> Result<()> {
        // Placeholder implementation
        Ok(())
    }
    
    /// Create transport for a specific node
    pub async fn create_transport(&self, node_id: &str) -> Result<Box<dyn Transport>> {
        // Resolve node address
        let node_address = self.resolver.resolve_node(node_id).await?;
        
        // Decide which transport to create based on address
        if let Some(unix_socket) = node_address.unix_socket {
            debug!("Creating Unix socket transport for node {}", node_id);
            Ok(Box::new(UnixSocketTransport::new(unix_socket)))
        } else {
            debug!("Creating TCP transport for node {}", node_id);
            Ok(Box::new(TcpTransport::new(node_address.primary)))
        }
    }
}

/// Simple Unix socket transport (placeholder)
struct UnixSocketTransport {
    path: String,
    is_connected: bool,
}

impl UnixSocketTransport {
    fn new(path: String) -> Self {
        Self {
            path,
            is_connected: false,
        }
    }
}

#[async_trait]
impl Transport for UnixSocketTransport {
    async fn start(&mut self) -> Result<()> {
        info!("Starting Unix socket transport on {}", self.path);
        self.is_connected = true;
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<()> {
        info!("Stopping Unix socket transport");
        self.is_connected = false;
        Ok(())
    }
    
    async fn send_to_actor(
        &self,
        _target_node: &str,
        _target_actor: &str,
        _message: &[u8],
    ) -> Result<()> {
        if !self.is_connected {
            return Err(TransportError::transport(
                "Unix socket not connected",
                Some("send_to_actor")
            ));
        }
        // Placeholder implementation
        Ok(())
    }
    
    async fn send_with_priority(
        &self,
        target_node: &str,
        target_actor: &str,
        message: &[u8],
        _priority: Priority,
    ) -> Result<()> {
        self.send_to_actor(target_node, target_actor, message).await
    }
    
    fn is_healthy(&self) -> bool {
        self.is_connected
    }
    
    fn statistics(&self) -> TransportStatistics {
        TransportStatistics::default()
    }
}

/// Simple TCP transport (placeholder)
struct TcpTransport {
    address: String,
    is_connected: bool,
}

impl TcpTransport {
    fn new(address: String) -> Self {
        Self {
            address,
            is_connected: false,
        }
    }
}

#[async_trait]
impl Transport for TcpTransport {
    async fn start(&mut self) -> Result<()> {
        info!("Starting TCP transport to {}", self.address);
        self.is_connected = true;
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<()> {
        info!("Stopping TCP transport");
        self.is_connected = false;
        Ok(())
    }
    
    async fn send_to_actor(
        &self,
        _target_node: &str,
        _target_actor: &str,
        _message: &[u8],
    ) -> Result<()> {
        if !self.is_connected {
            return Err(TransportError::transport(
                "TCP transport not connected",
                Some("send_to_actor")
            ));
        }
        // Placeholder implementation
        Ok(())
    }
    
    async fn send_with_priority(
        &self,
        target_node: &str,
        target_actor: &str,
        message: &[u8],
        _priority: Priority,
    ) -> Result<()> {
        self.send_to_actor(target_node, target_actor, message).await
    }
    
    fn is_healthy(&self) -> bool {
        self.is_connected
    }
    
    fn statistics(&self) -> TransportStatistics {
        TransportStatistics::default()
    }
}

/// Export additional types needed by topology integration
pub use super::resolver::{TopologyResolver as TopologyTransportResolver};

/// Transport resolution result
#[derive(Debug, Clone)]
pub struct TransportResolution {
    pub node_id: String,
    pub transport_type: String,
    pub address: String,
}

/// Criteria for transport selection
#[derive(Debug, Clone)]
pub struct TransportCriteria {
    pub priority: Priority,
    pub reliability_required: bool,
    pub latency_sensitive: bool,
}