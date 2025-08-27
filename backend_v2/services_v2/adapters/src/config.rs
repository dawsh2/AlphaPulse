//! Configuration module for adapters
//!
//! Provides environment-based configuration for all adapter services

use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

/// Base configuration shared by all adapters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseAdapterConfig {
    /// Adapter name/identifier
    pub name: String,
    
    /// Whether this adapter is enabled
    pub enabled: bool,
    
    /// Maximum number of retry attempts
    pub max_retries: u32,
    
    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,
    
    /// Initial reconnection delay in milliseconds
    pub reconnect_delay_ms: u64,
    
    /// Maximum reconnection delay in milliseconds
    pub max_reconnect_delay_ms: u64,
}

impl Default for BaseAdapterConfig {
    fn default() -> Self {
        Self {
            name: "adapter".to_string(),
            enabled: true,
            max_retries: 5,
            connection_timeout_ms: 10000,
            reconnect_delay_ms: 1000,
            max_reconnect_delay_ms: 60000,
        }
    }
}

/// Configuration for Polygon DEX collector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolygonDexConfig {
    /// WebSocket endpoint URL (can include API key)
    pub websocket_url: String,

    /// Fallback RPC endpoints for queries
    pub rpc_endpoints: Vec<String>,

    /// Maximum events per second to process
    pub max_events_per_second: u32,

    /// Circuit breaker error threshold (errors per minute)
    pub circuit_breaker_threshold: u32,

    /// Circuit breaker recovery time
    pub circuit_breaker_recovery_secs: u64,

    /// Pool cache size limit
    pub pool_cache_max_size: usize,

    /// Enable metrics collection
    pub enable_metrics: bool,
}

impl PolygonDexConfig {
    /// Load configuration from environment variables with defaults
    pub fn from_env() -> Self {
        Self {
            websocket_url: env::var("POLYGON_WS_URL")
                .unwrap_or_else(|_| "wss://polygon-bor-rpc.publicnode.com".to_string()),

            rpc_endpoints: env::var("POLYGON_RPC_ENDPOINTS")
                .map(|s| s.split(',').map(String::from).collect())
                .unwrap_or_else(|_| {
                    vec![
                        "https://polygon-rpc.com".to_string(),
                        "https://rpc-mainnet.matic.network".to_string(),
                    ]
                }),

            max_events_per_second: env::var("POLYGON_MAX_EVENTS_PER_SECOND")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1000),

            circuit_breaker_threshold: env::var("POLYGON_CIRCUIT_BREAKER_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),

            circuit_breaker_recovery_secs: env::var("POLYGON_CIRCUIT_BREAKER_RECOVERY_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),

            pool_cache_max_size: env::var("POLYGON_POOL_CACHE_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10000),

            enable_metrics: env::var("ENABLE_METRICS")
                .map(|s| s.to_lowercase() == "true")
                .unwrap_or(true),
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.websocket_url.is_empty() {
            return Err("WebSocket URL cannot be empty".to_string());
        }

        if !self.websocket_url.starts_with("ws://") && !self.websocket_url.starts_with("wss://") {
            return Err("WebSocket URL must start with ws:// or wss://".to_string());
        }

        if self.max_events_per_second == 0 {
            return Err("Max events per second must be greater than 0".to_string());
        }

        if self.circuit_breaker_threshold == 0 {
            return Err("Circuit breaker threshold must be greater than 0".to_string());
        }

        Ok(())
    }
}

/// Global adapter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    /// Polygon DEX specific configuration
    pub polygon_dex: PolygonDexConfig,

    /// Global metrics configuration
    pub metrics_port: u16,

    /// Global log level
    pub log_level: String,
}

impl AdapterConfig {
    /// Load complete configuration from environment
    pub fn from_env() -> Self {
        Self {
            polygon_dex: PolygonDexConfig::from_env(),

            metrics_port: env::var("METRICS_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(9090),

            log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        }
    }

    /// Validate all configurations
    pub fn validate(&self) -> Result<(), String> {
        self.polygon_dex.validate()?;

        if self.metrics_port == 0 {
            return Err("Metrics port must be greater than 0".to_string());
        }

        Ok(())
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per second
    pub max_per_second: u32,

    /// Burst size
    pub burst_size: u32,

    /// Recovery time after rate limit hit
    pub recovery_time: Duration,
}

impl RateLimitConfig {
    /// Create from environment or defaults
    pub fn from_env(prefix: &str) -> Self {
        let max_per_second_key = format!("{}_RATE_LIMIT_PER_SECOND", prefix);
        let burst_size_key = format!("{}_RATE_LIMIT_BURST", prefix);
        let recovery_time_key = format!("{}_RATE_LIMIT_RECOVERY_MS", prefix);

        Self {
            max_per_second: env::var(&max_per_second_key)
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),

            burst_size: env::var(&burst_size_key)
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),

            recovery_time: Duration::from_millis(
                env::var(&recovery_time_key)
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1000),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polygon_config_from_env() {
        // Set test environment variables
        env::set_var("POLYGON_WS_URL", "wss://test.polygon.com");
        env::set_var("POLYGON_MAX_EVENTS_PER_SECOND", "500");

        let config = PolygonDexConfig::from_env();
        assert_eq!(config.websocket_url, "wss://test.polygon.com");
        assert_eq!(config.max_events_per_second, 500);

        // Clean up
        env::remove_var("POLYGON_WS_URL");
        env::remove_var("POLYGON_MAX_EVENTS_PER_SECOND");
    }

    #[test]
    fn test_config_validation() {
        let mut config = PolygonDexConfig::from_env();

        // Valid config should pass
        assert!(config.validate().is_ok());

        // Invalid WebSocket URL
        config.websocket_url = "http://invalid.com".to_string();
        assert!(config.validate().is_err());

        // Empty WebSocket URL
        config.websocket_url = "".to_string();
        assert!(config.validate().is_err());

        // Invalid max events
        config.websocket_url = "wss://valid.com".to_string();
        config.max_events_per_second = 0;
        assert!(config.validate().is_err());
    }
}
