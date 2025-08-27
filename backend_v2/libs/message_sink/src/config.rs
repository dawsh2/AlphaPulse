//! Configuration support for MessageSink factory
//!
//! Provides TOML-based configuration for Stage 1 sink creation, with support for:
//! - Service endpoint definitions
//! - Sink type specifications (relay, direct, composite)
//! - Lazy connection configuration
//! - Composite pattern definitions (fanout, round-robin, failover)

use crate::{LazyConfig, MessageDomain};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Top-level services configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServicesConfig {
    /// Map of service name to service configuration
    pub services: HashMap<String, ServiceConfig>,
}

/// Configuration for a single service sink
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ServiceConfig {
    /// Type of sink to create
    #[serde(rename = "type")]
    pub sink_type: SinkType,
    
    /// Connection endpoint (for relay and direct sinks)
    pub endpoint: Option<String>,
    
    /// Pattern for composite sinks
    pub pattern: Option<CompositePattern>,
    
    /// Target services for composite sinks
    pub targets: Option<Vec<String>>,
    
    /// Buffer size for the sink
    pub buffer_size: Option<usize>,
    
    /// Maximum retry attempts
    pub max_retries: Option<u32>,
    
    /// Retry delay in milliseconds
    pub retry_delay_ms: Option<u64>,
    
    /// Connection timeout in seconds
    pub connect_timeout_secs: Option<u64>,
    
    /// Custom lazy configuration
    pub lazy: Option<LazyConfigToml>,
    
    /// TLV message domain for Protocol V2 validation (optional)
    pub domain: Option<MessageDomain>,
    
    /// Precision context for financial calculations
    pub precision_context: Option<PrecisionContext>,
    
    /// Additional metadata
    pub metadata: Option<HashMap<String, String>>,
}

/// Supported sink types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SinkType {
    /// Unix socket connection to relay service
    Relay,
    /// Direct TCP/WebSocket connection
    Direct,
    /// Composite pattern (fanout, round-robin, failover)
    Composite,
}

impl SinkType {
    /// Get human-readable name for error messages
    pub fn name(self) -> &'static str {
        match self {
            SinkType::Relay => "Relay",
            SinkType::Direct => "Direct", 
            SinkType::Composite => "Composite",
        }
    }
}

/// Patterns for composite sinks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositePattern {
    /// Send to all targets simultaneously
    Fanout,
    /// Rotate between targets
    RoundRobin,
    /// Primary with fallback targets
    Failover,
}

/// Precision handling context for financial calculations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrecisionContext {
    /// DEX token operations - preserve native precision (18 decimals WETH, 6 USDC, etc.)
    DexToken,
    /// Traditional exchange operations - use 8-decimal fixed-point for USD prices
    TraditionalExchange,
    /// Mixed operations - handle both DEX and traditional with automatic detection
    Mixed,
}

/// Lazy configuration in TOML format
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct LazyConfigToml {
    /// Maximum connection retry attempts
    pub max_retries: Option<u32>,
    
    /// Initial retry delay in milliseconds
    pub retry_delay_ms: Option<u64>,
    
    /// Exponential backoff multiplier
    pub backoff_multiplier: Option<f64>,
    
    /// Maximum retry delay in seconds
    pub max_retry_delay_secs: Option<u64>,
    
    /// Enable automatic reconnection on connection loss
    pub auto_reconnect: Option<bool>,
    
    /// Connection timeout in seconds
    pub connect_timeout_secs: Option<u64>,
    
    /// Wait timeout for other threads' connections in seconds
    pub wait_timeout_secs: Option<u64>,
}

impl LazyConfigToml {
    /// Convert TOML configuration to LazyConfig
    pub fn to_lazy_config(&self) -> LazyConfig {
        LazyConfig {
            max_retries: self.max_retries.unwrap_or(3),
            retry_delay: Duration::from_millis(self.retry_delay_ms.unwrap_or(100)),
            backoff_multiplier: self.backoff_multiplier.unwrap_or(2.0),
            max_retry_delay: Duration::from_secs(self.max_retry_delay_secs.unwrap_or(30)),
            auto_reconnect: self.auto_reconnect.unwrap_or(true),
            connect_timeout: Duration::from_secs(self.connect_timeout_secs.unwrap_or(5)),
            wait_timeout: Duration::from_secs(self.wait_timeout_secs.unwrap_or(10)),
        }
    }
}

impl Default for LazyConfigToml {
    fn default() -> Self {
        Self {
            max_retries: Some(3),
            retry_delay_ms: Some(100),
            backoff_multiplier: Some(2.0),
            max_retry_delay_secs: Some(30),
            auto_reconnect: Some(true),
            connect_timeout_secs: Some(5),
            wait_timeout_secs: Some(10),
        }
    }
}

impl ServiceConfig {
    /// Get the lazy configuration, using defaults if not specified
    pub fn lazy_config(&self) -> LazyConfig {
        self.lazy
            .as_ref()
            .map(|l| l.to_lazy_config())
            .unwrap_or_else(LazyConfig::default)
    }
    
    /// Validate the service configuration
    pub fn validate(&self) -> Result<(), String> {
        match self.sink_type {
            SinkType::Relay | SinkType::Direct => {
                if self.endpoint.is_none() {
                    return Err(format!("{:?} sink type requires endpoint", self.sink_type));
                }
            }
            SinkType::Composite => {
                if self.pattern.is_none() {
                    return Err("Composite sink type requires pattern".to_string());
                }
                if self.targets.is_none() || self.targets.as_ref().unwrap().is_empty() {
                    return Err("Composite sink type requires targets".to_string());
                }
            }
        }
        
        // Validate endpoint format if present
        if let Some(endpoint) = &self.endpoint {
            self.validate_endpoint(endpoint)?;
        }
        
        // Validate buffer size
        if let Some(buffer_size) = self.buffer_size {
            if buffer_size == 0 {
                return Err("Buffer size must be greater than 0".to_string());
            }
        }
        
        // Validate retry configuration
        if let Some(max_retries) = self.max_retries {
            if max_retries > 10 {
                return Err("Max retries should not exceed 10".to_string());
            }
        }
        
        Ok(())
    }
    
    /// Validate endpoint format
    fn validate_endpoint(&self, endpoint: &str) -> Result<(), String> {
        if endpoint.starts_with("unix://") {
            if endpoint.len() <= 7 {
                return Err("Unix socket path cannot be empty".to_string());
            }
        } else if endpoint.starts_with("tcp://") {
            if endpoint.len() <= 6 {
                return Err("TCP endpoint cannot be empty".to_string());
            }
            // Basic validation - should contain host:port
            let addr_part = &endpoint[6..];
            if !addr_part.contains(':') {
                return Err("TCP endpoint must include port (host:port)".to_string());
            }
        } else if endpoint.starts_with("ws://") || endpoint.starts_with("wss://") {
            if endpoint.len() <= 5 {
                return Err("WebSocket endpoint cannot be empty".to_string());
            }
        } else {
            return Err(format!(
                "Unsupported endpoint type. Supported: unix://, tcp://, ws://, wss://. Got: {}",
                endpoint
            ));
        }
        
        Ok(())
    }
}

impl ServicesConfig {
    /// Create from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self, String> {
        toml::from_str(toml_str)
            .map_err(|e| format!("Failed to parse TOML: {}", e))
    }
    
    /// Create from file path
    pub fn from_file(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        Self::from_toml(&content)
    }
    
    /// Convert to TOML string
    pub fn to_toml(&self) -> Result<String, String> {
        toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize to TOML: {}", e))
    }
    
    /// Validate all service configurations
    pub fn validate(&self) -> Result<(), String> {
        for (service_name, config) in &self.services {
            config.validate()
                .map_err(|e| format!("Service '{}': {}", service_name, e))?;
        }
        
        // Validate composite sink targets exist
        for (service_name, config) in &self.services {
            if let Some(targets) = &config.targets {
                for target in targets {
                    if !self.services.contains_key(target) {
                        return Err(format!(
                            "Service '{}' references unknown target '{}'",
                            service_name, target
                        ));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Get service configuration by name
    pub fn get_service(&self, name: &str) -> Option<&ServiceConfig> {
        self.services.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_config() {
        let toml = r#"
            [services.test_service]
            type = "relay"
            endpoint = "unix:///tmp/test.sock"
            buffer_size = 1000
        "#;
        
        let config = ServicesConfig::from_toml(toml).unwrap();
        let service = config.get_service("test_service").unwrap();
        
        assert_eq!(service.sink_type, SinkType::Relay);
        assert_eq!(service.endpoint, Some("unix:///tmp/test.sock".to_string()));
        assert_eq!(service.buffer_size, Some(1000));
    }
    
    #[test]
    fn test_parse_composite_config() {
        let toml = r#"
            [services.target1]
            type = "direct"
            endpoint = "tcp://localhost:8001"
            
            [services.target2]
            type = "direct"
            endpoint = "tcp://localhost:8002"
            
            [services.fanout_service]
            type = "composite"
            pattern = "fanout"
            targets = ["target1", "target2"]
        "#;
        
        let config = ServicesConfig::from_toml(toml).unwrap();
        config.validate().unwrap();
        
        let service = config.get_service("fanout_service").unwrap();
        assert_eq!(service.sink_type, SinkType::Composite);
        assert_eq!(service.pattern, Some(CompositePattern::Fanout));
        assert_eq!(service.targets, Some(vec!["target1".to_string(), "target2".to_string()]));
    }
    
    #[test]
    fn test_lazy_config_conversion() {
        let lazy_toml = LazyConfigToml {
            max_retries: Some(5),
            retry_delay_ms: Some(200),
            backoff_multiplier: Some(1.5),
            max_retry_delay_secs: Some(60),
            auto_reconnect: Some(false),
            connect_timeout_secs: Some(10),
            wait_timeout_secs: Some(20),
        };
        
        let lazy_config = lazy_toml.to_lazy_config();
        
        assert_eq!(lazy_config.max_retries, 5);
        assert_eq!(lazy_config.retry_delay, Duration::from_millis(200));
        assert_eq!(lazy_config.backoff_multiplier, 1.5);
        assert_eq!(lazy_config.max_retry_delay, Duration::from_secs(60));
        assert_eq!(lazy_config.auto_reconnect, false);
        assert_eq!(lazy_config.connect_timeout, Duration::from_secs(10));
        assert_eq!(lazy_config.wait_timeout, Duration::from_secs(20));
    }
    
    #[test]
    fn test_validation_errors() {
        let toml = r#"
            [services.invalid_service]
            type = "relay"
            # Missing endpoint
        "#;
        
        let config = ServicesConfig::from_toml(toml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires endpoint"));
    }
    
    #[test]
    fn test_endpoint_validation() {
        let mut config = ServiceConfig {
            sink_type: SinkType::Direct,
            endpoint: Some("invalid://test".to_string()),
            pattern: None,
            targets: None,
            buffer_size: None,
            max_retries: None,
            retry_delay_ms: None,
            connect_timeout_secs: None,
            lazy: None,
            domain: None,
            precision_context: None,
            metadata: None,
        };
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported endpoint type"));
        
        // Valid endpoint
        config.endpoint = Some("tcp://localhost:8080".to_string());
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_composite_target_validation() {
        let toml = r#"
            [services.composite_service]
            type = "composite"
            pattern = "fanout"
            targets = ["nonexistent_service"]
        "#;
        
        let config = ServicesConfig::from_toml(toml).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown target"));
    }
}