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
            url: "wss://polygon-bor-rpc.publicnode.com".to_string(),
            fallback_urls: vec![
                "wss://polygon-mainnet.g.alchemy.com/v2/demo".to_string(),
                "wss://ws-polygon-mainnet.chainstacklabs.com".to_string(),
            ],
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
            runtime_validation_seconds: 10,
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

/// DEX event signatures configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexEventsConfig {
    /// Swap event signature (TLV 11)
    pub swap_signature: String,
    
    /// Mint event signature (TLV 12)
    pub mint_signature: String,
    
    /// Burn event signature (TLV 13)
    pub burn_signature: String,
    
    /// Tick crossing signature (TLV 14)
    pub tick_signature: String,
    
    /// V2 Sync event signature (TLV 16)
    pub sync_signature: String,
    
    /// Transfer event signature (TLV 10)
    pub transfer_signature: String,
    
    /// Approval event signature (supplementary)
    pub approval_signature: String,
    
    /// V3 pool creation signature (TLV 15)
    pub v3_pool_created_signature: String,
    
    /// V2 pair creation signature (TLV 15)
    pub v2_pair_created_signature: String,
}

impl Default for DexEventsConfig {
    fn default() -> Self {
        Self {
            swap_signature: "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822".to_string(),
            mint_signature: "0x7a53080ba414158be7ec69b987b5fb7d07dee101fe85488f0853ae16239d0bde".to_string(),
            burn_signature: "0x0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c".to_string(),
            tick_signature: "0x3067048beee31b25b2f1681f88dac838c8bba36af25bfb2b7cf7473a5847e35f".to_string(),
            sync_signature: "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1".to_string(),
            transfer_signature: "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef".to_string(),
            approval_signature: "0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925".to_string(),
            v3_pool_created_signature: "0x783cca1c0412dd0d695e784568c96da2e9c22ff989357a2e8b1d9b2b4e6b7118".to_string(),
            v2_pair_created_signature: "0x0d3648bd0f6ba80134a33ba9275ac585d9d315f0ad8355cddefde31afa28d0e9".to_string(),
        }
    }
}

/// Contract addresses configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractsConfig {
    /// Uniswap V3 Factory address
    pub uniswap_v3_factory: String,
    
    /// QuickSwap V2 Factory address
    pub quickswap_v2_factory: String,
    
    /// SushiSwap Factory address
    pub sushiswap_factory: String,
    
    /// Uniswap V3 Router address
    pub uniswap_v3_router: String,
    
    /// QuickSwap Router address
    pub quickswap_router: String,
    
    /// SushiSwap Router address
    pub sushiswap_router: String,
}

impl Default for ContractsConfig {
    fn default() -> Self {
        Self {
            uniswap_v3_factory: "0x1F98431c8aD98523631AE4a59f267346ea31F984".to_string(),
            quickswap_v2_factory: "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32".to_string(),
            sushiswap_factory: "0xc35DADB65012eC5796536bD9864eD8773aBc74C4".to_string(),
            uniswap_v3_router: "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string(),
            quickswap_router: "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff".to_string(),
            sushiswap_router: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506".to_string(),
        }
    }
}

/// Complete Polygon collector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolygonConfig {
    pub websocket: WebSocketConfig,
    pub relay: RelayConfig,
    pub validation: ValidationConfig,
    pub monitoring: MonitoringConfig,
    pub dex_events: DexEventsConfig,
    pub contracts: ContractsConfig,
}

impl Default for PolygonConfig {
    fn default() -> Self {
        Self {
            websocket: WebSocketConfig::default(),
            relay: RelayConfig::default(),
            validation: ValidationConfig::default(),
            monitoring: MonitoringConfig::default(),
            dex_events: DexEventsConfig::default(),
            contracts: ContractsConfig::default(),
        }
    }
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
        toml::from_str(content)
            .with_context(|| "Failed to parse TOML configuration")
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
            return Err(anyhow::anyhow!("WebSocket URL must start with ws:// or wss://"));
        }
        
        // Validate relay domain
        self.relay.parse_domain()
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
        
        // Validate event signatures
        for (name, signature) in [
            ("swap", &self.dex_events.swap_signature),
            ("mint", &self.dex_events.mint_signature),
            ("burn", &self.dex_events.burn_signature),
            ("tick", &self.dex_events.tick_signature),
            ("sync", &self.dex_events.sync_signature),
            ("transfer", &self.dex_events.transfer_signature),
            ("approval", &self.dex_events.approval_signature),
            ("v3_pool_created", &self.dex_events.v3_pool_created_signature),
            ("v2_pair_created", &self.dex_events.v2_pair_created_signature),
        ] {
            if !signature.starts_with("0x") || signature.len() != 66 {
                return Err(anyhow::anyhow!(
                    "Invalid {} event signature: must be 66-char hex string starting with 0x", 
                    name
                ));
            }
        }
        
        // Validate contract addresses
        for (name, address) in [
            ("uniswap_v3_factory", &self.contracts.uniswap_v3_factory),
            ("quickswap_v2_factory", &self.contracts.quickswap_v2_factory),
            ("sushiswap_factory", &self.contracts.sushiswap_factory),
        ] {
            if !address.starts_with("0x") || address.len() != 42 {
                return Err(anyhow::anyhow!(
                    "Invalid {} contract address: must be 42-char hex string starting with 0x", 
                    name
                ));
            }
        }
        
        Ok(())
    }
    
    /// Convert WebSocket config to Duration values
    pub fn websocket_timeouts(&self) -> (Duration, Duration) {
        (
            Duration::from_millis(self.websocket.connection_timeout_ms),
            Duration::from_millis(self.websocket.message_timeout_ms),
        )
    }
    
    /// Get all event signatures as a vector for subscription
    pub fn all_event_signatures(&self) -> Vec<&str> {
        vec![
            &self.dex_events.swap_signature,
            &self.dex_events.mint_signature,
            &self.dex_events.burn_signature,
            &self.dex_events.tick_signature,
            &self.dex_events.sync_signature,
            &self.dex_events.transfer_signature,
            &self.dex_events.approval_signature,
            &self.dex_events.v3_pool_created_signature,
            &self.dex_events.v2_pair_created_signature,
        ]
    }
    
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
        assert_eq!(config.validation.runtime_validation_seconds, deserialized.validation.runtime_validation_seconds);
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
        
        // Invalid event signature
        config.relay.domain = "MarketData".to_string();
        config.dex_events.swap_signature = "invalid_signature".to_string();
        assert!(config.validate().is_err());
    }
}