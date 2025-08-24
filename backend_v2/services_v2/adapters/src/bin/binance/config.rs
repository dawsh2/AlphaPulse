//! Configuration for Binance unified collector

use anyhow::{Context, Result};
use protocol_v2::RelayDomain;
use serde::{Deserialize, Serialize};
use std::fs;

/// Main Binance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceConfig {
    pub websocket: WebSocketConfig,
    pub relay: RelayConfig,
    pub symbols: Vec<String>,
    pub validation: ValidationConfig,
    pub monitoring: MonitoringConfig,
}

/// WebSocket connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub url: String,
    pub stream_url: String,
    pub connection_timeout_ms: u64,
    pub message_timeout_ms: u64,
    pub max_reconnect_attempts: u32,
    pub base_backoff_ms: u64,
    pub max_backoff_ms: u64,
}

/// Relay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayConfig {
    pub domain: String,
    pub socket_path: String,
}

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub runtime_validation_seconds: u64,
    pub verbose_validation: bool,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub stats_interval_seconds: u64,
    pub max_processing_latency_ms: u64,
}

impl Default for BinanceConfig {
    fn default() -> Self {
        Self {
            websocket: WebSocketConfig {
                url: "wss://stream.binance.com:9443".to_string(),
                stream_url: "/stream".to_string(),
                connection_timeout_ms: 30000,
                message_timeout_ms: 120000,
                max_reconnect_attempts: 10,
                base_backoff_ms: 1000,
                max_backoff_ms: 60000,
            },
            relay: RelayConfig {
                domain: "market_data".to_string(),
                socket_path: "/tmp/alphapulse/market_data.sock".to_string(),
            },
            symbols: vec![
                "btcusdt".to_string(),
                "ethusdt".to_string(),
                "bnbusdt".to_string(),
                "solusdt".to_string(),
                "adausdt".to_string(),
            ],
            validation: ValidationConfig {
                runtime_validation_seconds: 60,
                verbose_validation: false,
            },
            monitoring: MonitoringConfig {
                stats_interval_seconds: 30,
                max_processing_latency_ms: 100,
            },
        }
    }
}

impl BinanceConfig {
    /// Load configuration from TOML file with environment variable overrides
    pub fn from_toml_with_env_overrides(path: &str) -> Result<Self> {
        // Load base config from file
        let config_str =
            fs::read_to_string(path).context(format!("Failed to read config file: {}", path))?;

        let mut config: BinanceConfig =
            toml::from_str(&config_str).context("Failed to parse TOML configuration")?;

        // Apply environment variable overrides
        if let Ok(url) = std::env::var("BINANCE_WS_URL") {
            config.websocket.url = url;
        }

        if let Ok(path) = std::env::var("RELAY_SOCKET_PATH") {
            config.relay.socket_path = path;
        }

        if let Ok(symbols) = std::env::var("BINANCE_SYMBOLS") {
            config.symbols = symbols.split(',').map(|s| s.trim().to_string()).collect();
        }

        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.symbols.is_empty() {
            return Err(anyhow::anyhow!("No symbols configured"));
        }

        if !self.websocket.url.starts_with("wss://") && !self.websocket.url.starts_with("ws://") {
            return Err(anyhow::anyhow!("Invalid WebSocket URL scheme"));
        }

        Ok(())
    }

    /// Parse relay domain from configuration
    pub fn parse_relay_domain(&self) -> Result<RelayDomain> {
        match self.relay.domain.as_str() {
            "market_data" => Ok(RelayDomain::MarketData),
            "signal" => Ok(RelayDomain::Signal),
            "execution" => Ok(RelayDomain::Execution),
            _ => Err(anyhow::anyhow!(
                "Invalid relay domain: {}",
                self.relay.domain
            )),
        }
    }
}

impl RelayConfig {
    pub fn parse_domain(&self) -> Result<RelayDomain> {
        match self.domain.as_str() {
            "market_data" => Ok(RelayDomain::MarketData),
            "signal" => Ok(RelayDomain::Signal),
            "execution" => Ok(RelayDomain::Execution),
            _ => Err(anyhow::anyhow!("Invalid relay domain: {}", self.domain)),
        }
    }
}
