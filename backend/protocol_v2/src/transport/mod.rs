//! Mixed Transport Mode Implementation
//! 
//! Supports both Unix domain sockets and future message bus transports

pub mod unix_socket;
pub mod message_bus;

pub use unix_socket::*;
pub use message_bus::*;

use crate::ProtocolError;
use serde::{Deserialize, Serialize};

/// Transport configuration for mixed-mode deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    pub mode: TransportMode,
    pub market_data: EndpointConfig,
    pub signals: EndpointConfig,
    pub execution: EndpointConfig,
}

/// Transport mode selection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportMode {
    UnixSocket,    // Pure Unix socket mode
    MessageBus,    // Pure message bus mode  
    Mixed,         // Mixed mode (per-endpoint configuration)
}

/// Configuration for individual endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    pub transport_type: TransportType,
    pub path: Option<String>,           // For Unix sockets
    pub channel_capacity: Option<usize>, // For message bus
    pub buffer_size: Option<usize>,     // Buffer size
    pub enable_recovery: bool,          // Enable recovery protocol
    pub checksum_validation: bool,      // Enable checksum validation
}

/// Transport type for individual endpoints
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportType {
    UnixSocket,
    MessageBus,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            mode: TransportMode::UnixSocket,
            market_data: EndpointConfig {
                transport_type: TransportType::UnixSocket,
                path: Some("/tmp/alphapulse/market_data.sock".to_string()),
                channel_capacity: None,
                buffer_size: Some(1024 * 1024), // 1MB buffer
                enable_recovery: true,
                checksum_validation: false, // Market data prioritizes speed
            },
            signals: EndpointConfig {
                transport_type: TransportType::UnixSocket,
                path: Some("/tmp/alphapulse/signals.sock".to_string()),
                channel_capacity: None,
                buffer_size: Some(512 * 1024), // 512KB buffer
                enable_recovery: true,
                checksum_validation: true, // Signals need reliability
            },
            execution: EndpointConfig {
                transport_type: TransportType::UnixSocket,
                path: Some("/tmp/alphapulse/execution.sock".to_string()),
                channel_capacity: None,
                buffer_size: Some(256 * 1024), // 256KB buffer  
                enable_recovery: true,
                checksum_validation: true, // Execution always validates
            },
        }
    }
}

impl TransportConfig {
    /// Load configuration from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self, ProtocolError> {
        serde_yaml::from_str(yaml)
            .map_err(|e| ProtocolError::Transport(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid YAML config: {}", e)
            )))
    }
    
    /// Save configuration to YAML string
    pub fn to_yaml(&self) -> Result<String, ProtocolError> {
        serde_yaml::to_string(self)
            .map_err(|e| ProtocolError::Transport(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("YAML serialization failed: {}", e)
            )))
    }
    
    /// Get endpoint configuration by relay domain
    pub fn get_endpoint(&self, domain: crate::RelayDomain) -> &EndpointConfig {
        match domain {
            crate::RelayDomain::MarketData => &self.market_data,
            crate::RelayDomain::Signal => &self.signals,
            crate::RelayDomain::Execution => &self.execution,
        }
    }
    
    /// Check if mixed mode is enabled
    pub fn is_mixed_mode(&self) -> bool {
        matches!(self.mode, TransportMode::Mixed)
    }
    
    /// Validate configuration consistency
    pub fn validate(&self) -> Result<(), ProtocolError> {
        match self.mode {
            TransportMode::UnixSocket => {
                // All endpoints should use Unix sockets
                if self.market_data.transport_type != TransportType::UnixSocket ||
                   self.signals.transport_type != TransportType::UnixSocket ||
                   self.execution.transport_type != TransportType::UnixSocket {
                    return Err(ProtocolError::Transport(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "All endpoints must use unix_socket transport in UnixSocket mode"
                    )));
                }
            }
            TransportMode::MessageBus => {
                // All endpoints should use message bus
                if self.market_data.transport_type != TransportType::MessageBus ||
                   self.signals.transport_type != TransportType::MessageBus ||
                   self.execution.transport_type != TransportType::MessageBus {
                    return Err(ProtocolError::Transport(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "All endpoints must use message_bus transport in MessageBus mode"
                    )));
                }
            }
            TransportMode::Mixed => {
                // Mixed mode allows different transports per endpoint
                // No additional validation needed
            }
        }
        
        // Validate individual endpoint configurations
        self.validate_endpoint("market_data", &self.market_data)?;
        self.validate_endpoint("signals", &self.signals)?;
        self.validate_endpoint("execution", &self.execution)?;
        
        Ok(())
    }
    
    fn validate_endpoint(&self, name: &str, config: &EndpointConfig) -> Result<(), ProtocolError> {
        match config.transport_type {
            TransportType::UnixSocket => {
                if config.path.is_none() {
                    return Err(ProtocolError::Transport(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("Unix socket path required for endpoint: {}", name)
                    )));
                }
            }
            TransportType::MessageBus => {
                if config.channel_capacity.is_none() {
                    return Err(ProtocolError::Transport(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("Channel capacity required for message bus endpoint: {}", name)
                    )));
                }
            }
        }
        
        Ok(())
    }
}

/// Transport factory for creating connections
pub struct TransportFactory {
    config: TransportConfig,
}

impl TransportFactory {
    /// Create a new transport factory with configuration
    pub fn new(config: TransportConfig) -> Result<Self, ProtocolError> {
        config.validate()?;
        Ok(Self { config })
    }
    
    /// Create a producer connection for a relay domain
    pub async fn create_producer(&self, domain: crate::RelayDomain) -> Result<Box<dyn MessageProducer>, ProtocolError> {
        let endpoint = self.config.get_endpoint(domain);
        
        match endpoint.transport_type {
            TransportType::UnixSocket => {
                let path = endpoint.path.as_ref()
                    .ok_or_else(|| ProtocolError::Transport(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Unix socket path not configured"
                    )))?;
                
                let producer = UnixSocketProducer::new(path.clone()).await?;
                Ok(Box::new(producer))
            }
            TransportType::MessageBus => {
                let capacity = endpoint.channel_capacity
                    .ok_or_else(|| ProtocolError::Transport(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Message bus channel capacity not configured"
                    )))?;
                
                let producer = MessageBusProducer::new(domain, capacity).await?;
                Ok(Box::new(producer))
            }
        }
    }
    
    /// Create a consumer connection for a relay domain
    pub async fn create_consumer(&self, domain: crate::RelayDomain) -> Result<Box<dyn MessageConsumer>, ProtocolError> {
        let endpoint = self.config.get_endpoint(domain);
        
        match endpoint.transport_type {
            TransportType::UnixSocket => {
                let path = endpoint.path.as_ref()
                    .ok_or_else(|| ProtocolError::Transport(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Unix socket path not configured"
                    )))?;
                
                let consumer = UnixSocketConsumer::new(path.clone()).await?;
                Ok(Box::new(consumer))
            }
            TransportType::MessageBus => {
                let consumer = MessageBusConsumer::new(domain).await?;
                Ok(Box::new(consumer))
            }
        }
    }
    
    /// Get the transport configuration
    pub fn config(&self) -> &TransportConfig {
        &self.config
    }
}

/// Trait for message producers (relays sending messages)
#[async_trait::async_trait]
pub trait MessageProducer: Send + Sync {
    async fn send(&mut self, message: &[u8]) -> Result<(), ProtocolError>;
    async fn flush(&mut self) -> Result<(), ProtocolError>;
    fn is_connected(&self) -> bool;
}

/// Trait for message consumers (services receiving messages)
#[async_trait::async_trait]
pub trait MessageConsumer: Send + Sync {
    async fn receive(&mut self) -> Result<Vec<u8>, ProtocolError>;
    async fn receive_timeout(&mut self, timeout_ms: u64) -> Result<Option<Vec<u8>>, ProtocolError>;
    fn is_connected(&self) -> bool;
}

// Add missing dependency
#[cfg(not(test))]
use async_trait;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = TransportConfig::default();
        assert_eq!(config.mode, TransportMode::UnixSocket);
        assert_eq!(config.market_data.transport_type, TransportType::UnixSocket);
        assert_eq!(config.signals.transport_type, TransportType::UnixSocket);
        assert_eq!(config.execution.transport_type, TransportType::UnixSocket);
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_mixed_mode_config() {
        let yaml_config = r#"
mode: mixed
market_data:
  transport_type: unix_socket
  path: "/tmp/alphapulse/market_data.sock"
  buffer_size: 1048576
  enable_recovery: true
  checksum_validation: false
signals:
  transport_type: message_bus
  channel_capacity: 100000
  enable_recovery: true
  checksum_validation: true
execution:
  transport_type: unix_socket
  path: "/tmp/alphapulse/execution.sock"
  buffer_size: 262144
  enable_recovery: true
  checksum_validation: true
"#;
        
        let config = TransportConfig::from_yaml(yaml_config).unwrap();
        assert_eq!(config.mode, TransportMode::Mixed);
        assert!(config.is_mixed_mode());
        assert_eq!(config.market_data.transport_type, TransportType::UnixSocket);
        assert_eq!(config.signals.transport_type, TransportType::MessageBus);
        assert_eq!(config.execution.transport_type, TransportType::UnixSocket);
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_config_validation_errors() {
        // Test missing path for Unix socket
        let mut config = TransportConfig::default();
        config.market_data.path = None;
        assert!(config.validate().is_err());
        
        // Test missing channel capacity for message bus
        config = TransportConfig::default();
        config.mode = TransportMode::MessageBus;
        config.market_data.transport_type = TransportType::MessageBus;
        config.signals.transport_type = TransportType::MessageBus;
        config.execution.transport_type = TransportType::MessageBus;
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_config_serialization() {
        let config = TransportConfig::default();
        let yaml = config.to_yaml().unwrap();
        let parsed = TransportConfig::from_yaml(&yaml).unwrap();
        
        assert_eq!(config.mode, parsed.mode);
        assert_eq!(config.market_data.transport_type, parsed.market_data.transport_type);
        assert_eq!(config.market_data.path, parsed.market_data.path);
    }
    
    #[test]
    fn test_endpoint_selection() {
        let config = TransportConfig::default();
        
        let market_data_endpoint = config.get_endpoint(crate::RelayDomain::MarketData);
        assert_eq!(market_data_endpoint.transport_type, TransportType::UnixSocket);
        assert_eq!(market_data_endpoint.path.as_ref().unwrap(), "/tmp/alphapulse/market_data.sock");
        
        let signals_endpoint = config.get_endpoint(crate::RelayDomain::Signal);
        assert_eq!(signals_endpoint.transport_type, TransportType::UnixSocket);
        assert_eq!(signals_endpoint.path.as_ref().unwrap(), "/tmp/alphapulse/signals.sock");
        
        let execution_endpoint = config.get_endpoint(crate::RelayDomain::Execution);
        assert_eq!(execution_endpoint.transport_type, TransportType::UnixSocket);
        assert_eq!(execution_endpoint.path.as_ref().unwrap(), "/tmp/alphapulse/execution.sock");
    }
}