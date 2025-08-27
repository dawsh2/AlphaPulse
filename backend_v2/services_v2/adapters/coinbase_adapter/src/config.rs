//! Configuration management for Coinbase adapter

use alphapulse_adapter_service::BaseAdapterConfig;
use serde::{Deserialize, Serialize};

/// Configuration specific to Coinbase adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinbaseAdapterConfig {
    /// Base adapter configuration
    #[serde(flatten)]
    pub base: BaseAdapterConfig,

    /// Coinbase WebSocket URL
    pub websocket_url: String,

    /// Products to subscribe to (e.g., "BTC-USD", "ETH-USD")
    pub products: Vec<String>,

    /// Coinbase channels to subscribe to
    pub channels: Vec<String>,

    /// Message timeout for WebSocket operations
    pub message_timeout_ms: u64,
}

impl Default for CoinbaseAdapterConfig {
    fn default() -> Self {
        Self {
            base: BaseAdapterConfig {
                name: "coinbase".to_string(),
                enabled: true,
                max_retries: 3,
                connection_timeout_ms: 5000,
                reconnect_delay_ms: 1000,
                max_reconnect_delay_ms: 60000,
            },
            websocket_url: "wss://ws-feed.exchange.coinbase.com".to_string(),
            products: vec!["BTC-USD".to_string(), "ETH-USD".to_string()],
            channels: vec!["ticker".to_string(), "matches".to_string()],
            message_timeout_ms: 30000,
        }
    }
}