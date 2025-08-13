// Centralized configuration management
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use anyhow::Result;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub redis: RedisConfig,
    pub data: DataConfig,
    pub collectors: CollectorsConfig,
    pub orderbook: OrderBookConfig,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub api_port: u16,
    pub websocket_update_interval_ms: u64,
    pub health_check_interval_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedisConfig {
    pub url: String,
    pub buffer_size: usize,
    pub batch_timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataConfig {
    pub base_dir: PathBuf,
    pub parquet_dir: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CollectorsConfig {
    pub reconnect_delay_secs: u64,
    pub max_reconnect_attempts: u32,
    pub exponential_backoff: bool,
    pub backoff_multiplier: f64,
    pub max_backoff_secs: u64,
    pub symbols: Vec<SymbolMapping>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SymbolMapping {
    pub internal: String,
    pub coinbase: String,
    pub kraken: String,
    pub binance: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OrderBookConfig {
    pub max_depth: usize,
    pub delta_updates_enabled: bool,
    pub snapshot_interval_secs: u64,
    pub compression_enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitoringConfig {
    pub prometheus_enabled: bool,
    pub prometheus_port: u16,
    pub jaeger_enabled: bool,
    pub jaeger_endpoint: String,
}

impl Config {
    /// Load configuration from file and environment variables
    pub fn load() -> Result<Self> {
        // Try to load from config file first
        let config_path = std::env::var("ALPHAPULSE_CONFIG")
            .unwrap_or_else(|_| "config.toml".to_string());
        
        let mut config = if Path::new(&config_path).exists() {
            let contents = std::fs::read_to_string(&config_path)?;
            toml::from_str(&contents)?
        } else {
            Self::default()
        };
        
        // Override with environment variables
        config.override_from_env();
        
        Ok(config)
    }
    
    fn override_from_env(&mut self) {
        // Server config
        if let Ok(port) = std::env::var("API_PORT") {
            if let Ok(p) = port.parse() {
                self.server.api_port = p;
            }
        }
        
        // Redis config
        if let Ok(url) = std::env::var("REDIS_URL") {
            self.redis.url = url;
        }
        
        // Data config
        if let Ok(dir) = std::env::var("DATA_BASE_DIR") {
            self.data.base_dir = PathBuf::from(dir);
        }
        
        // Add more environment variable overrides as needed
    }
    
    /// Get parquet directory path
    pub fn parquet_path(&self) -> PathBuf {
        self.data.base_dir.join(&self.data.parquet_dir)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            server: ServerConfig {
                api_port: 3001,
                websocket_update_interval_ms: 500,
                health_check_interval_secs: 30,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                buffer_size: 1000,
                batch_timeout_ms: 100,
            },
            data: DataConfig {
                base_dir: PathBuf::from("./data"),
                parquet_dir: "parquet".to_string(),
            },
            collectors: CollectorsConfig {
                reconnect_delay_secs: 5,
                max_reconnect_attempts: 10,
                exponential_backoff: true,
                backoff_multiplier: 2.0,
                max_backoff_secs: 300,
                symbols: vec![],
            },
            orderbook: OrderBookConfig {
                max_depth: 1000,
                delta_updates_enabled: true,
                snapshot_interval_secs: 30,
                compression_enabled: false,
            },
            monitoring: MonitoringConfig {
                prometheus_enabled: true,
                prometheus_port: 9090,
                jaeger_enabled: false,
                jaeger_endpoint: "http://localhost:14268/api/traces".to_string(),
            },
        }
    }
}

// Symbol conversion utilities
pub struct SymbolConverter {
    mappings: HashMap<String, SymbolMapping>,
}

impl SymbolConverter {
    pub fn new(symbols: Vec<SymbolMapping>) -> Self {
        let mut mappings = HashMap::new();
        for symbol in symbols {
            mappings.insert(symbol.internal.clone(), symbol);
        }
        Self { mappings }
    }
    
    pub fn to_exchange(&self, internal: &str, exchange: &str) -> String {
        if let Some(mapping) = self.mappings.get(internal) {
            match exchange.to_lowercase().as_str() {
                "coinbase" => mapping.coinbase.clone(),
                "kraken" => mapping.kraken.clone(),
                "binance" | "binance_us" => mapping.binance.clone(),
                _ => internal.to_string(),
            }
        } else {
            internal.to_string()
        }
    }
    
    pub fn from_exchange(&self, symbol: &str, exchange: &str) -> String {
        for (internal, mapping) in &self.mappings {
            let exchange_symbol = match exchange.to_lowercase().as_str() {
                "coinbase" => &mapping.coinbase,
                "kraken" => &mapping.kraken,
                "binance" | "binance_us" => &mapping.binance,
                _ => return symbol.to_string(),
            };
            
            if exchange_symbol == symbol {
                return internal.clone();
            }
        }
        symbol.to_string()
    }
}