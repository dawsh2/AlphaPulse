//! Unified transport module for AlphaPulse network layer
//!
//! This module provides all transport implementations including
//! TCP, UDP, Unix sockets, and hybrid transports for different
//! latency and reliability requirements.

// Re-export network module components as transport
pub use crate::network::{
    compression,
    connection,
    envelope,
    security,
    tcp,
    udp,
    unix,
};

// Re-export primary transport types
pub use crate::network::tcp::TcpTransport;
pub use crate::network::udp::UdpTransport;
pub use crate::network::unix::{UnixSocketTransport, UnixSocketConfig};

// Re-export envelope types
pub use crate::network::envelope::{
    NetworkEnvelope,
    MessageFlags,
    WireFormat,
};

// Re-export security types
pub use crate::network::security::{
    SecurityInfo,
    SecurityLayer,
};

// Re-export compression types  
pub use crate::network::compression::{
    CompressionType,
    CompressionEngine,
    CompressionInfo,
};

/// Hybrid transport that automatically selects between available transports
/// based on latency requirements and destination characteristics
pub struct HybridTransport {
    tcp: Option<TcpTransport>,
    udp: Option<UdpTransport>,
    unix: Option<UnixSocketTransport>,
}

impl HybridTransport {
    pub fn new() -> Self {
        Self {
            tcp: None,
            udp: None,
            unix: None,
        }
    }
    
    pub fn with_tcp(mut self) -> Self {
        self.tcp = Some(TcpTransport);
        self
    }
    
    pub fn with_udp(mut self) -> Self {
        self.udp = Some(UdpTransport);
        self
    }
    
    pub fn with_unix(mut self, config: UnixSocketConfig) -> Self {
        if let Ok(transport) = UnixSocketTransport::new(config) {
            self.unix = Some(transport);
        }
        self
    }
}