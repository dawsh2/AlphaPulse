//! Unified Transport Layer
//!
//! This module provides a unified transport abstraction for the network layer,
//! consolidating previously scattered transport implementations into a coherent system.

use crate::{Result, TransportError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;

pub mod tcp;
pub mod udp;
pub mod unix;

// Re-export transport types
pub use tcp::{TcpNetworkConfig, TcpNetworkTransport, TcpConnectionStats};
pub use udp::{UdpConfig, UdpTransport};
pub use unix::{UnixSocketConfig, UnixSocketTransport, UnixSocketConnection};

/// Unified Transport trait for all transport implementations
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send message over the transport
    async fn send(&self, message: &[u8]) -> Result<()>;
    
    /// Check if transport is healthy
    fn is_healthy(&self) -> bool;
    
    /// Get transport-specific information
    fn transport_info(&self) -> TransportInfo;
}

/// Transport type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportType {
    /// TCP network transport
    Tcp,
    /// UDP network transport  
    Udp,
    /// Unix domain socket transport
    Unix,
}

/// Transport configuration enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportConfig {
    /// TCP configuration
    Tcp(TcpNetworkConfig),
    /// UDP configuration  
    Udp(UdpConfig),
    /// Unix socket configuration
    Unix(UnixSocketConfig),
}

/// Transport information for monitoring
#[derive(Debug, Clone)]
pub struct TransportInfo {
    pub transport_type: TransportType,
    pub local_address: Option<String>,
    pub remote_address: Option<String>,
    pub connection_count: usize,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// Transport factory for creating transport instances
pub struct TransportFactory;

impl TransportFactory {
    /// Create transport from configuration
    pub async fn create_transport(config: TransportConfig) -> Result<Box<dyn Transport>> {
        match config {
            TransportConfig::Tcp(tcp_config) => {
                let transport = TcpNetworkTransport::from_config(tcp_config);
                Ok(Box::new(transport))
            }
            TransportConfig::Udp(udp_config) => {
                let transport = UdpTransport::new(udp_config)?;
                Ok(Box::new(transport))
            }
            TransportConfig::Unix(unix_config) => {
                let transport = UnixSocketTransport::new(unix_config)?;
                Ok(Box::new(transport))
            }
        }
    }
    
    /// Create TCP transport for client connections
    pub fn create_tcp_client(remote_address: SocketAddr) -> Box<dyn Transport> {
        Box::new(TcpNetworkTransport::new_client(remote_address))
    }
    
    /// Create TCP transport for server connections  
    pub fn create_tcp_server(bind_address: SocketAddr) -> Box<dyn Transport> {
        Box::new(TcpNetworkTransport::new_server(bind_address))
    }
    
    /// Create Unix socket transport
    pub fn create_unix_socket(path: PathBuf) -> Result<Box<dyn Transport>> {
        let config = UnixSocketConfig {
            path,
            ..Default::default()
        };
        let transport = UnixSocketTransport::new(config)?;
        Ok(Box::new(transport))
    }
}

/// Transport implementation for the unified trait - TCP
#[async_trait]
impl Transport for TcpNetworkTransport {
    async fn send(&self, message: &[u8]) -> Result<()> {
        use crate::mycelium::transport::NetworkTransport;
        NetworkTransport::send(self, message).await
    }
    
    fn is_healthy(&self) -> bool {
        use crate::mycelium::transport::NetworkTransport;
        NetworkTransport::is_healthy(self)
    }
    
    fn transport_info(&self) -> TransportInfo {
        let stats = futures::executor::block_on(self.get_stats());
        TransportInfo {
            transport_type: TransportType::Tcp,
            local_address: None, // Would need to track this
            remote_address: stats.as_ref().map(|s| s.peer_addr.to_string()),
            connection_count: if stats.is_some() { 1 } else { 0 },
            bytes_sent: stats.as_ref().map(|s| s.bytes_sent).unwrap_or(0),
            bytes_received: stats.as_ref().map(|s| s.bytes_received).unwrap_or(0),
        }
    }
}

/// Transport implementation for the unified trait - UDP
#[async_trait]
impl Transport for UdpTransport {
    async fn send(&self, _message: &[u8]) -> Result<()> {
        // UDP implementation would go here
        Err(TransportError::configuration(
            "UDP transport not yet implemented",
            Some("transport_type")
        ))
    }
    
    fn is_healthy(&self) -> bool {
        // UDP health check would go here
        true
    }
    
    fn transport_info(&self) -> TransportInfo {
        TransportInfo {
            transport_type: TransportType::Udp,
            local_address: None,
            remote_address: None, 
            connection_count: 0,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }
}

/// Transport implementation for the unified trait - Unix Socket
#[async_trait]
impl Transport for UnixSocketTransport {
    async fn send(&self, _message: &[u8]) -> Result<()> {
        // Unix socket send would need refactoring to work with the transport interface
        Err(TransportError::configuration(
            "Unix socket transport interface needs refactoring",
            Some("transport_interface")
        ))
    }
    
    fn is_healthy(&self) -> bool {
        // Unix socket health check would go here
        true
    }
    
    fn transport_info(&self) -> TransportInfo {
        TransportInfo {
            transport_type: TransportType::Unix,
            local_address: None,
            remote_address: None,
            connection_count: 0,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }
}