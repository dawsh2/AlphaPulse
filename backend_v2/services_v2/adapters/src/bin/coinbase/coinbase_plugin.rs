//! # Coinbase Plugin Adapter - Plugin Architecture Proof of Concept
//!
//! ## Purpose
//! Demonstrates the plugin architecture for exchange adapters using the Coinbase
//! collector as the reference implementation. This plugin-based design enables
//! dynamic loading, consistent interfaces, and hot-swappable exchange integrations.

use alphapulse_adapters::{
    Adapter, AdapterHealth, CircuitState, ConnectionStatus, InstrumentType, Result,
};
use codec::{InstrumentId, TLVMessageBuilder, TLVType, VenueId};
use alphapulse_types::protocol::{RelayDomain, SourceType, TradeTLV, QuoteTLV};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use tungstenite::Message as WsMessage;

/// Coinbase plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinbasePluginConfig {
    /// WebSocket URL for Coinbase
    pub websocket_url: String,
    
    /// API credentials (optional for public data)
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    
    /// Instruments to subscribe to
    pub instruments: Vec<String>,
    
    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,
    
    /// Reconnect delay in milliseconds
    pub reconnect_delay_ms: u64,
    
    /// Rate limit (requests per second)
    pub rate_limit_rps: Option<u32>,
}

impl Default for CoinbasePluginConfig {
    fn default() -> Self {
        Self {
            websocket_url: "wss://ws-feed.exchange.coinbase.com".to_string(),
            api_key: None,
            api_secret: None,
            instruments: vec!["BTC-USD".to_string(), "ETH-USD".to_string()],
            connection_timeout_ms: 5000,
            reconnect_delay_ms: 1000,
            rate_limit_rps: Some(10),
        }
    }
}

/// Coinbase adapter plugin implementation
pub struct CoinbasePlugin {
    config: CoinbasePluginConfig,
    output_tx: mpsc::Sender<Vec<u8>>,
    health: Arc<RwLock<AdapterHealth>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    instruments: Arc<RwLock<Vec<String>>>,
}

impl CoinbasePlugin {
    /// Create a new Coinbase plugin adapter
    pub fn new(config: CoinbasePluginConfig, output_tx: mpsc::Sender<Vec<u8>>) -> Self {
        let health = Arc::new(RwLock::new(AdapterHealth {
            is_healthy: false,
            connection_status: ConnectionStatus::Disconnected,
            messages_processed: 0,
            error_count: 0,
            last_error: None,
            uptime_seconds: 0,
            latency_ms: None,
            circuit_breaker_state: CircuitState::Closed,
            rate_limit_remaining: config.rate_limit_rps,
            connection_timeout_ms: config.connection_timeout_ms,
            max_latency_ms: 35.0, // Hot path requirement
        }));

        Self {
            instruments: Arc::new(RwLock::new(config.instruments.clone())),
            config,
            output_tx,
            health,
            shutdown_tx: None,
        }
    }

    /// Process Coinbase WebSocket message
    async fn process_coinbase_message(&self, msg: &str) -> Result<Option<Vec<u8>>> {
        // Parse JSON message
        let json: serde_json::Value = serde_json::from_str(msg)
            .map_err(|e| alphapulse_adapters::AdapterError::ParseError(e.to_string()))?;

        // Extract message type
        let msg_type = json["type"].as_str().unwrap_or("");
        
        match msg_type {
            "ticker" => self.process_ticker(&json).await,
            "match" => self.process_trade(&json).await,
            "l2update" => self.process_l2_update(&json).await,
            "heartbeat" => {
                debug!("Heartbeat received");
                Ok(None)
            }
            _ => {
                debug!("Unknown message type: {}", msg_type);
                Ok(None)
            }
        }
    }

    /// Process ticker message into QuoteTLV
    async fn process_ticker(&self, json: &serde_json::Value) -> Result<Option<Vec<u8>>> {
        let product_id = json["product_id"].as_str()
            .ok_or_else(|| alphapulse_adapters::AdapterError::ParseError("Missing product_id".to_string()))?;
        
        // Create instrument ID
        let instrument_id = InstrumentId::coin(VenueId::Coinbase, product_id)?;
        
        // Extract price and size
        let best_bid = json["best_bid"].as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let best_ask = json["best_ask"].as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let bid_size = json["best_bid_size"].as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let ask_size = json["best_ask_size"].as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        // Convert to fixed-point (8 decimals for USD)
        let bid_price = (best_bid * 100_000_000.0) as i64;
        let ask_price = (best_ask * 100_000_000.0) as i64;
        let bid_quantity = (bid_size * 100_000_000.0) as i64;
        let ask_quantity = (ask_size * 100_000_000.0) as i64;

        // Create QuoteTLV
        let quote_tlv = QuoteTLV {
            instrument_id: instrument_id.to_u64(),
            bid_price,
            bid_quantity,
            ask_price,
            ask_quantity,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        };

        // Build Protocol V2 message
        let mut builder = TLVMessageBuilder::new(
            RelayDomain::MarketData as u8,
            SourceType::Coinbase as u8,
        );
        builder.add_tlv(TLVType::Quote, quote_tlv.as_bytes());
        
        Ok(Some(builder.build()))
    }

    /// Process trade/match message into TradeTLV
    async fn process_trade(&self, json: &serde_json::Value) -> Result<Option<Vec<u8>>> {
        let product_id = json["product_id"].as_str()
            .ok_or_else(|| alphapulse_adapters::AdapterError::ParseError("Missing product_id".to_string()))?;
        
        // Create instrument ID
        let instrument_id = InstrumentId::coin(VenueId::Coinbase, product_id)?;
        
        // Extract trade data
        let price = json["price"].as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let size = json["size"].as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let side = json["side"].as_str().unwrap_or("unknown");

        // Convert to fixed-point
        let price_fixed = (price * 100_000_000.0) as i64;
        let quantity = (size * 100_000_000.0) as i64;

        // Create TradeTLV
        let trade_tlv = TradeTLV {
            instrument_id: instrument_id.to_u64(),
            price: price_fixed,
            quantity,
            is_buy: side == "buy",
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        };

        // Build Protocol V2 message
        let mut builder = TLVMessageBuilder::new(
            RelayDomain::MarketData as u8,
            SourceType::Coinbase as u8,
        );
        builder.add_tlv(TLVType::Trade, trade_tlv.as_bytes());
        
        // Update health metrics
        let mut health = self.health.write().await;
        health.messages_processed += 1;
        
        Ok(Some(builder.build()))
    }

    /// Process L2 update (order book update)
    async fn process_l2_update(&self, json: &serde_json::Value) -> Result<Option<Vec<u8>>> {
        // For now, we'll skip L2 updates as they require more complex handling
        debug!("L2 update received but not processed");
        Ok(None)
    }
}

#[async_trait]
impl Adapter for CoinbasePlugin {
    type Config = CoinbasePluginConfig;

    async fn start(&self) -> Result<()> {
        info!("Starting Coinbase plugin adapter");
        
        // Update health status
        {
            let mut health = self.health.write().await;
            health.connection_status = ConnectionStatus::Connecting;
        }

        // Create WebSocket connection
        let url = &self.config.websocket_url;
        let (ws_stream, _) = tokio_tungstenite::connect_async(url).await
            .map_err(|e| alphapulse_adapters::AdapterError::ConnectionError(e.to_string()))?;

        info!("Connected to Coinbase WebSocket");

        // Update health
        {
            let mut health = self.health.write().await;
            health.is_healthy = true;
            health.connection_status = ConnectionStatus::Connected;
        }

        // Subscribe to instruments
        let instruments = self.instruments.read().await.clone();
        let subscribe_msg = serde_json::json!({
            "type": "subscribe",
            "product_ids": instruments,
            "channels": ["ticker", "matches", "level2"]
        });

        // Send subscription
        let (mut write, mut read) = ws_stream.split();
        write.send(WsMessage::Text(subscribe_msg.to_string())).await
            .map_err(|e| alphapulse_adapters::AdapterError::ConnectionError(e.to_string()))?;

        // Process messages
        while let Some(msg_result) = read.next().await {
            match msg_result {
                Ok(WsMessage::Text(text)) => {
                    match self.process_coinbase_message(&text).await {
                        Ok(Some(tlv_message)) => {
                            // Send to output
                            if let Err(e) = self.output_tx.send(tlv_message).await {
                                error!("Failed to send message to output: {}", e);
                            }
                        }
                        Ok(None) => {
                            // No message to send
                        }
                        Err(e) => {
                            warn!("Error processing message: {}", e);
                            let mut health = self.health.write().await;
                            health.error_count += 1;
                            health.last_error = Some(e.to_string());
                        }
                    }
                }
                Ok(WsMessage::Close(_)) => {
                    info!("WebSocket closed");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    let mut health = self.health.write().await;
                    health.is_healthy = false;
                    health.connection_status = ConnectionStatus::Disconnected;
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        info!("Stopping Coinbase plugin adapter");
        
        // Send shutdown signal if available
        if let Some(ref tx) = self.shutdown_tx {
            let _ = tx.send(()).await;
        }

        // Update health
        let mut health = self.health.write().await;
        health.is_healthy = false;
        health.connection_status = ConnectionStatus::Disconnected;

        Ok(())
    }

    async fn health_check(&self) -> AdapterHealth {
        self.health.read().await.clone()
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn identifier(&self) -> &str {
        "coinbase_plugin"
    }

    fn supported_instruments(&self) -> Vec<InstrumentType> {
        vec![
            InstrumentType::Spot,
            InstrumentType::Perpetual,
        ]
    }

    async fn configure_instruments(&mut self, instruments: Vec<String>) -> Result<()> {
        *self.instruments.write().await = instruments;
        Ok(())
    }

    async fn process_message(&self, raw_data: &[u8], output_buffer: &mut [u8]) -> Result<Option<usize>> {
        // Convert raw data to string
        let msg = std::str::from_utf8(raw_data)
            .map_err(|e| alphapulse_adapters::AdapterError::ParseError(e.to_string()))?;

        // Process the message
        match self.process_coinbase_message(msg).await? {
            Some(tlv_message) => {
                // Copy to output buffer
                if tlv_message.len() > output_buffer.len() {
                    return Err(alphapulse_adapters::AdapterError::BufferTooSmall);
                }
                output_buffer[..tlv_message.len()].copy_from_slice(&tlv_message);
                Ok(Some(tlv_message.len()))
            }
            None => Ok(None),
        }
    }
}

/// Plugin factory for creating Coinbase adapters
pub struct CoinbasePluginFactory;

impl CoinbasePluginFactory {
    /// Create a new Coinbase plugin instance
    pub fn create(
        config: CoinbasePluginConfig,
        output_tx: mpsc::Sender<Vec<u8>>,
    ) -> Box<dyn Adapter<Config = CoinbasePluginConfig>> {
        Box::new(CoinbasePlugin::new(config, output_tx))
    }

    /// Create from configuration file
    pub async fn from_config_file(
        path: &str,
        output_tx: mpsc::Sender<Vec<u8>>,
    ) -> Result<Box<dyn Adapter<Config = CoinbasePluginConfig>>> {
        let config_str = tokio::fs::read_to_string(path).await
            .map_err(|e| alphapulse_adapters::AdapterError::ConfigError(e.to_string()))?;
        
        let config: CoinbasePluginConfig = toml::from_str(&config_str)
            .map_err(|e| alphapulse_adapters::AdapterError::ConfigError(e.to_string()))?;
        
        Ok(Self::create(config, output_tx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coinbase_plugin_creation() {
        let (tx, _rx) = mpsc::channel(100);
        let config = CoinbasePluginConfig::default();
        let plugin = CoinbasePlugin::new(config.clone(), tx);
        
        assert_eq!(plugin.identifier(), "coinbase_plugin");
        assert_eq!(plugin.config().websocket_url, "wss://ws-feed.exchange.coinbase.com");
    }

    #[tokio::test]
    async fn test_health_check() {
        let (tx, _rx) = mpsc::channel(100);
        let config = CoinbasePluginConfig::default();
        let plugin = CoinbasePlugin::new(config, tx);
        
        let health = plugin.health_check().await;
        assert!(!health.is_healthy);
        assert_eq!(health.connection_status, ConnectionStatus::Disconnected);
        assert_eq!(health.messages_processed, 0);
    }

    #[tokio::test]
    async fn test_instrument_configuration() {
        let (tx, _rx) = mpsc::channel(100);
        let config = CoinbasePluginConfig::default();
        let mut plugin = CoinbasePlugin::new(config, tx);
        
        let new_instruments = vec!["SOL-USD".to_string(), "AVAX-USD".to_string()];
        plugin.configure_instruments(new_instruments.clone()).await.unwrap();
        
        let instruments = plugin.instruments.read().await;
        assert_eq!(*instruments, new_instruments);
    }
}