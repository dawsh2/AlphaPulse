// Configuration management
use serde::{Deserialize, Serialize};
use std::fs;
use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub redis_url: String,
    pub api_port: u16,
    pub exchanges: Vec<ExchangeConfig>,
    pub buffer_size: usize,
    pub batch_timeout_ms: u64,
    pub data: DataConfig,
    pub server: ServerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConfig {
    pub base_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub websocket_update_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    pub name: String,
    pub enabled: bool,
    pub symbols: Vec<String>,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = std::env::var("CONFIG_PATH")
            .unwrap_or_else(|_| "config.toml".to_string());
        
        let contents = fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&contents)
            .map_err(|e| crate::AlphaPulseError::ConfigError(e.to_string()))?;
        
        Ok(config)
    }
    
    pub fn from_env() -> Self {
        Self {
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            api_port: std::env::var("API_PORT")
                .unwrap_or_else(|_| "3001".to_string())
                .parse()
                .unwrap_or(3001),
            exchanges: vec![],
            buffer_size: std::env::var("BUFFER_SIZE")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .unwrap_or(1000),
            batch_timeout_ms: std::env::var("BATCH_TIMEOUT_MS")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .unwrap_or(100),
            data: DataConfig {
                base_dir: std::env::var("DATA_BASE_DIR")
                    .unwrap_or_else(|_| "./market_data/parquet".to_string()),
            },
            server: ServerConfig {
                websocket_update_interval_ms: std::env::var("WEBSOCKET_UPDATE_INTERVAL_MS")
                    .unwrap_or_else(|_| "1000".to_string())
                    .parse()
                    .unwrap_or(1000),
            },
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            redis_url: "redis://localhost:6379".to_string(),
            api_port: 3001,
            exchanges: vec![],
            buffer_size: 1000,
            batch_timeout_ms: 100,
            data: DataConfig {
                base_dir: "./market_data/parquet".to_string(),
            },
            server: ServerConfig {
                websocket_update_interval_ms: 1000,
            },
        }
    }
}

// Symbol converter for different exchange formats
pub struct SymbolConverter;

impl SymbolConverter {
    // Convert from standard format (BTC/USD) to exchange-specific format
    pub fn to_coinbase(symbol: &str) -> String {
        symbol.replace("/", "-")
    }
    
    pub fn to_kraken(symbol: &str) -> String {
        symbol.to_string()  // Kraken uses BTC/USD format
    }
    
    pub fn to_binance(symbol: &str) -> String {
        symbol.replace("/", "")  // Binance uses BTCUSDT format
    }
    
    // Convert from exchange-specific format to standard format
    pub fn from_coinbase(symbol: &str) -> String {
        symbol.replace("-", "/")
    }
    
    pub fn from_kraken(symbol: &str) -> String {
        symbol.to_string()
    }
    
    pub fn from_binance(symbol: &str) -> String {
        // Simple heuristic: insert / before last 3-4 characters
        if symbol.ends_with("USDT") {
            format!("{}/USDT", &symbol[..symbol.len() - 4])
        } else if symbol.ends_with("USD") {
            format!("{}/USD", &symbol[..symbol.len() - 3])
        } else {
            symbol.to_string()
        }
    }
}