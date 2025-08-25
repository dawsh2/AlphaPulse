//! Configuration management for unified Polygon collector
//!
//! Supports both TOML-based configuration and environment variable fallbacks
//! for maximum flexibility in development and production deployments.

use anyhow::{Context, Result};
use protocol_v2::RelayDomain;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// WebSocket connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// Primary WebSocket endpoint
    pub url: String,

    /// Fallback endpoints (tried in order)
    pub fallback_urls: Vec<String>,

    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,

    /// Message timeout for heartbeat/keep-alive in milliseconds
    pub message_timeout_ms: u64,

    /// Base backoff delay for reconnection attempts
    pub base_backoff_ms: u64,

    /// Maximum backoff delay for reconnection attempts
    pub max_backoff_ms: u64,

    /// Maximum number of reconnection attempts before giving up
    pub max_reconnect_attempts: u32,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            url: "wss://polygon.drpc.org".to_string(),
            fallback_urls: vec![],
            connection_timeout_ms: 30000,
            message_timeout_ms: 60000,
            base_backoff_ms: 1000,
            max_backoff_ms: 30000,
            max_reconnect_attempts: 10,
        }
    }
}

/// Relay output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayConfig {
    /// Unix socket path for relay connection
    pub socket_path: String,

    /// Relay domain for message routing
    pub domain: String,

    /// Source identifier for this collector
    pub source_id: u32,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            socket_path: "/tmp/alphapulse/market_data.sock".to_string(),
            domain: "MarketData".to_string(),
            source_id: 3,
        }
    }
}

impl RelayConfig {
    /// Parse domain string to RelayDomain enum
    pub fn parse_domain(&self) -> Result<RelayDomain> {
        match self.domain.as_str() {
            "MarketData" => Ok(RelayDomain::MarketData),
            "Signal" => Ok(RelayDomain::Signal),
            "Execution" => Ok(RelayDomain::Execution),
            other => Err(anyhow::anyhow!("Invalid relay domain: {}", other)),
        }
    }
}

/// Runtime validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Duration of runtime TLV validation in seconds
    pub runtime_validation_seconds: u64,

    /// Enable verbose validation logging
    pub verbose_validation: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            runtime_validation_seconds: 0, // Disabled by default for testing
            verbose_validation: true,
        }
    }
}

/// Monitoring and health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,

    /// Statistics reporting interval in seconds
    pub stats_report_interval_seconds: u64,

    /// Maximum processing latency warning threshold in milliseconds
    pub max_processing_latency_ms: u64,

    /// Maximum memory usage warning threshold in MB
    pub max_memory_usage_mb: u64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            health_check_interval_seconds: 10,
            stats_report_interval_seconds: 60,
            max_processing_latency_ms: 35,
            max_memory_usage_mb: 50,
        }
    }
}

// DEX event signatures removed - now handled by libs/dex with generated signatures from ABI

// Contract addresses removed - should come from a proper registry service, not hardcoded config

/// Complete Polygon collector configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PolygonConfig {
    pub websocket: WebSocketConfig,
    pub relay: RelayConfig,
    pub validation: ValidationConfig,
    pub monitoring: MonitoringConfig,
}

impl PolygonConfig {
    /// Load configuration from TOML file
    pub fn from_toml_file(file_path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read config file: {}", file_path))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML config: {}", file_path))
    }

    /// Load configuration from TOML string
    pub fn from_toml_str(content: &str) -> Result<Self> {
        toml::from_str(content).with_context(|| "Failed to parse TOML configuration")
    }

    /// Load configuration with environment variable overrides
    pub fn from_toml_with_env_overrides(file_path: &str) -> Result<Self> {
        let mut config = if std::path::Path::new(file_path).exists() {
            Self::from_toml_file(file_path)?
        } else {
            Self::default()
        };

        // Apply environment variable overrides
        config.apply_env_overrides();

        Ok(config)
    }

    /// Apply environment variable overrides to configuration
    pub fn apply_env_overrides(&mut self) {
        use std::env;

        // WebSocket configuration overrides
        if let Ok(url) = env::var("POLYGON_WS_URL") {
            self.websocket.url = url;
        }

        if let Ok(timeout) = env::var("POLYGON_WS_TIMEOUT_MS") {
            if let Ok(timeout) = timeout.parse() {
                self.websocket.connection_timeout_ms = timeout;
            }
        }

        // Relay configuration overrides
        if let Ok(socket_path) = env::var("POLYGON_RELAY_SOCKET") {
            self.relay.socket_path = socket_path;
        }

        if let Ok(domain) = env::var("POLYGON_RELAY_DOMAIN") {
            self.relay.domain = domain;
        }

        if let Ok(source_id) = env::var("POLYGON_SOURCE_ID") {
            if let Ok(source_id) = source_id.parse() {
                self.relay.source_id = source_id;
            }
        }

        // Validation configuration overrides
        if let Ok(validation_seconds) = env::var("POLYGON_VALIDATION_SECONDS") {
            if let Ok(validation_seconds) = validation_seconds.parse() {
                self.validation.runtime_validation_seconds = validation_seconds;
            }
        }
    }

    /// Validate the complete configuration
    pub fn validate(&self) -> Result<()> {
        // Validate WebSocket URL
        if self.websocket.url.is_empty() {
            return Err(anyhow::anyhow!("WebSocket URL cannot be empty"));
        }

        if !self.websocket.url.starts_with("ws://") && !self.websocket.url.starts_with("wss://") {
            return Err(anyhow::anyhow!(
                "WebSocket URL must start with ws:// or wss://"
            ));
        }

        // Validate relay domain
        self.relay
            .parse_domain()
            .with_context(|| "Invalid relay domain configuration")?;

        // Validate socket path
        if self.relay.socket_path.is_empty() {
            return Err(anyhow::anyhow!("Relay socket path cannot be empty"));
        }

        // Validate timeouts
        if self.websocket.connection_timeout_ms == 0 {
            return Err(anyhow::anyhow!("Connection timeout must be greater than 0"));
        }

        if self.websocket.message_timeout_ms == 0 {
            return Err(anyhow::anyhow!("Message timeout must be greater than 0"));
        }

        // Event signatures now validated at runtime via libs/dex
        // Contract addresses now handled by registry service

        Ok(())
    }

    /// Convert WebSocket config to Duration values
    pub fn websocket_timeouts(&self) -> (Duration, Duration) {
        (
            Duration::from_millis(self.websocket.connection_timeout_ms),
            Duration::from_millis(self.websocket.message_timeout_ms),
        )
    }

    // Event signatures now provided by libs/dex::get_all_event_signatures()

    /// Save configuration to TOML file
    pub fn save_toml_file(&self, file_path: &str) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .with_context(|| "Failed to serialize configuration to TOML")?;

        std::fs::write(file_path, content)
            .with_context(|| format!("Failed to write config file: {}", file_path))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config_is_valid() {
        let config = PolygonConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_relay_domain_parsing() {
        let mut config = PolygonConfig::default();

        // Valid domains
        config.relay.domain = "MarketData".to_string();
        assert!(config.relay.parse_domain().is_ok());

        config.relay.domain = "Signal".to_string();
        assert!(config.relay.parse_domain().is_ok());

        config.relay.domain = "Execution".to_string();
        assert!(config.relay.parse_domain().is_ok());

        // Invalid domain
        config.relay.domain = "InvalidDomain".to_string();
        assert!(config.relay.parse_domain().is_err());
    }

    #[test]
    fn test_env_overrides() {
        // Set test environment variables
        env::set_var("POLYGON_WS_URL", "wss://test.polygon.com");
        env::set_var("POLYGON_RELAY_SOCKET", "/tmp/test.sock");
        env::set_var("POLYGON_SOURCE_ID", "99");

        let mut config = PolygonConfig::default();
        config.apply_env_overrides();

        assert_eq!(config.websocket.url, "wss://test.polygon.com");
        assert_eq!(config.relay.socket_path, "/tmp/test.sock");
        assert_eq!(config.relay.source_id, 99);

        // Clean up
        env::remove_var("POLYGON_WS_URL");
        env::remove_var("POLYGON_RELAY_SOCKET");
        env::remove_var("POLYGON_SOURCE_ID");
    }

    #[test]
    fn test_toml_roundtrip() {
        let config = PolygonConfig::default();

        // Serialize to TOML
        let toml_str = toml::to_string(&config).unwrap();

        // Deserialize back
        let deserialized: PolygonConfig = toml::from_str(&toml_str).unwrap();

        // Should be identical
        assert_eq!(config.websocket.url, deserialized.websocket.url);
        assert_eq!(config.relay.socket_path, deserialized.relay.socket_path);
        assert_eq!(
            config.validation.runtime_validation_seconds,
            deserialized.validation.runtime_validation_seconds
        );
    }

    #[test]
    fn test_invalid_config_validation() {
        let mut config = PolygonConfig::default();

        // Invalid WebSocket URL
        config.websocket.url = "http://invalid.com".to_string();
        assert!(config.validate().is_err());

        // Empty WebSocket URL
        config.websocket.url = "".to_string();
        assert!(config.validate().is_err());

        // Invalid relay domain
        config.websocket.url = "wss://valid.com".to_string();
        config.relay.domain = "InvalidDomain".to_string();
        assert!(config.validate().is_err());

        // Valid configuration (event signatures now handled by libs/dex)
        config.relay.domain = "MarketData".to_string();
        assert!(config.validate().is_ok());
    }
}
