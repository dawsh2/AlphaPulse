//! Kraken WebSocket data collector
//!
//! Handles JSON and array-based WebSocket streams from Kraken for:
//! - Trade streams
//! - Order book updates (L2)
//! - Ticker streams
//!
//! ## Data Format Reference
//!
//! Kraken uses array-based format for data messages and JSON for control messages

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use protocol_v2::{
    current_timestamp_ns, InstrumentId, QuoteTLV, RelayDomain, SourceType, TLVMessageBuilder,
    TLVType, TradeTLV, VenueId,
};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde_json::Value;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

use crate::input::{HealthStatus, InputAdapter};
use crate::AdapterMetrics;
use crate::{AdapterError, Result};

/// Kraken System Status JSON Schema
///
/// Connection initialization message
const KRAKEN_SYSTEM_STATUS_SCHEMA: &str = r#"
{
  "event": "systemStatus",
  "version": "1.9.6",
  "status": "online",
  "connectionID": 12345678901234567890
}
"#;

/// Kraken Subscription Status JSON Schema
///
/// Confirmation of subscription requests
const KRAKEN_SUBSCRIPTION_STATUS_SCHEMA: &str = r#"
{
  "channelID": 119930881,
  "channelName": "trade",
  "event": "subscriptionStatus",
  "pair": "XBT/USD",
  "status": "subscribed",
  "subscription": {
    "name": "trade"
  }
}
"#;

/// Kraken Heartbeat JSON Schema
///
/// Periodic keepalive message
const KRAKEN_HEARTBEAT_SCHEMA: &str = r#"
{
  "event": "heartbeat"
}
"#;

/// Kraken Trade Data Array Schema
///
/// Format: [channelID, [trade_data_array], "trade", "PAIR"]
/// Trade data: [price, volume, time, side, orderType, misc]
const KRAKEN_TRADE_ARRAY_SCHEMA: &str = r#"
[
  119930881,                           // Channel ID
  [
    [
      "113879.30000",                  // Price (string)
      "0.01317184",                    // Volume (string)  
      "1755750124.577095",             // Time (string, seconds.microseconds)
      "s",                             // Side: "b" = buy, "s" = sell
      "m",                             // Order type: "m" = market, "l" = limit
      ""                               // Miscellaneous info
    ]
  ],
  "trade",                             // Channel name
  "XBT/USD"                            // Trading pair
]
"#;

/// Kraken Order Book Array Schema (L2)
///
/// Format: [channelID, book_data, "book", "PAIR"]
/// Book data: {"bs": bids, "as": asks} or {"b": bids, "a": asks}
const KRAKEN_BOOK_ARRAY_SCHEMA: &str = r#"
[
  13959169,                            // Channel ID
  {
    "bs": [                            // Bids (best to worst)
      [
        "4287.73000",                  // Price (string)
        "0.10000000",                  // Volume (string)
        "1755750122.927411"            // Timestamp (string)
      ]
    ],
    "as": [                            // Asks (best to worst)
      [
        "4287.74000",                  // Price (string)
        "0.05000000",                  // Volume (string) 
        "1755750122.927411"            // Timestamp (string)
      ]
    ]
  },
  "book",                              // Channel name
  "ETH/USD"                            // Trading pair
]
"#;

/// Kraken Subscription Request JSON Schema
///
/// Used to subscribe to channels
const KRAKEN_SUBSCRIPTION_REQUEST_SCHEMA: &str = r#"
{
  "event": "subscribe",
  "pair": ["XBT/USD", "ETH/USD"],      // Trading pairs
  "subscription": {
    "name": "trade"                    // Channel: "trade", "book", "ticker"
  }
}
"#;

/// Kraken Order Book Subscription Request
///
/// Subscribe to L2 order book with depth
const KRAKEN_BOOK_SUBSCRIPTION_SCHEMA: &str = r#"
{
  "event": "subscribe", 
  "pair": ["XBT/USD"],
  "subscription": {
    "name": "book",
    "depth": 10                        // Order book depth (1, 5, 10, 25, 100, 500, 1000)
  }
}
"#;

/// Configuration for Kraken WebSocket collector
#[derive(Debug, Clone)]
pub struct KrakenConfig {
    pub websocket_url: String,
    pub trading_pairs: Vec<String>,
    pub venue_id: VenueId,
    pub reconnect_interval_ms: u64,
    pub max_reconnect_attempts: usize,
    pub heartbeat_timeout_ms: u64,
}

impl Default for KrakenConfig {
    fn default() -> Self {
        Self {
            websocket_url: "wss://ws.kraken.com".to_string(),
            trading_pairs: vec!["XBT/USD".to_string(), "ETH/USD".to_string()],
            venue_id: VenueId::Kraken,
            reconnect_interval_ms: 5000,
            max_reconnect_attempts: 10,
            heartbeat_timeout_ms: 30000,
        }
    }
}

/// Enhanced error types for Kraken collector operations
#[derive(Debug, thiserror::Error)]
pub enum KrakenError {
    #[error("WebSocket connection failed: {0}")]
    WebSocketConnectionFailed(String),

    #[error("Subscription failed for pair {pair}: {reason}")]
    SubscriptionFailed { pair: String, reason: String },

    #[error("Invalid message format: {0}")]
    InvalidMessageFormat(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Connection timeout after {timeout_ms}ms")]
    ConnectionTimeout { timeout_ms: u64 },

    #[error("Heartbeat timeout - no response for {timeout_ms}ms")]
    HeartbeatTimeout { timeout_ms: u64 },

    #[error("Adapter error: {0}")]
    AdapterError(#[from] AdapterError),
}

impl From<KrakenError> for AdapterError {
    fn from(err: KrakenError) -> Self {
        match err {
            KrakenError::WebSocketConnectionFailed(msg) => AdapterError::ConnectionFailed {
                venue: VenueId::Kraken,
                reason: msg,
            },
            KrakenError::ConnectionTimeout { timeout_ms } => AdapterError::ConnectionTimeout {
                venue: VenueId::Kraken,
                timeout_ms,
            },
            KrakenError::InvalidMessageFormat(msg) => AdapterError::InvalidMessage(msg),
            KrakenError::SerializationError(msg) => AdapterError::Internal(msg),
            KrakenError::SubscriptionFailed { pair, reason } => AdapterError::Other(
                anyhow::anyhow!("Subscription failed for {}: {}", pair, reason),
            ),
            KrakenError::HeartbeatTimeout { timeout_ms } => AdapterError::ConnectionTimeout {
                venue: VenueId::Kraken,
                timeout_ms,
            },
            KrakenError::AdapterError(err) => err,
        }
    }
}

/// Production Kraken WebSocket collector with Protocol V2 integration
pub struct KrakenCollector {
    config: KrakenConfig,
    metrics: Arc<AdapterMetrics>,
    output_tx: mpsc::Sender<Vec<u8>>, // Protocol V2 binary messages
    running: Arc<RwLock<bool>>,
    subscribed_pairs: Arc<RwLock<Vec<String>>>,
    reconnect_attempts: Arc<RwLock<usize>>,
}

impl KrakenCollector {
    /// Create new Kraken collector with configuration
    pub fn new(config: KrakenConfig, output_tx: mpsc::Sender<Vec<u8>>) -> Self {
        let metrics = Arc::new(AdapterMetrics::new());

        Self {
            config,
            metrics,
            output_tx,
            running: Arc::new(RwLock::new(false)),
            subscribed_pairs: Arc::new(RwLock::new(Vec::new())),
            reconnect_attempts: Arc::new(RwLock::new(0)),
        }
    }

    /// Create Kraken collector with default configuration
    pub fn with_defaults(output_tx: mpsc::Sender<Vec<u8>>) -> Self {
        Self::new(KrakenConfig::default(), output_tx)
    }

    /// Get schema reference for documentation
    pub fn get_trade_schema() -> &'static str {
        KRAKEN_TRADE_ARRAY_SCHEMA
    }

    /// Get order book schema reference  
    pub fn get_book_schema() -> &'static str {
        KRAKEN_BOOK_ARRAY_SCHEMA
    }

    /// Get subscription schema reference
    pub fn get_subscription_schema() -> &'static str {
        KRAKEN_SUBSCRIPTION_REQUEST_SCHEMA
    }

    /// Parse Kraken trade array and convert to Protocol V2 TLV message
    fn parse_trade_message(&self, trade_data: &Value) -> Result<Vec<u8>> {
        // Expected format: [channelID, [[trade_data]], "trade", "PAIR"]
        let array = trade_data.as_array().ok_or_else(|| {
            KrakenError::InvalidMessageFormat("Expected array format".to_string())
        })?;

        if array.len() < 4 {
            return Err(KrakenError::InvalidMessageFormat(
                "Insufficient array elements".to_string(),
            )
            .into());
        }

        let trades_array = array[1].as_array().ok_or_else(|| {
            KrakenError::InvalidMessageFormat("Expected trades array".to_string())
        })?;

        let pair = array[3]
            .as_str()
            .ok_or_else(|| KrakenError::InvalidMessageFormat("Expected pair string".to_string()))?;

        // Convert pair to InstrumentId using proper symbol mapping
        let instrument_id = match pair {
            "XBT/USD" => InstrumentId::stock(self.config.venue_id, "BTCUSD"),
            "ETH/USD" => InstrumentId::stock(self.config.venue_id, "ETHUSD"),
            _ => {
                return Err(KrakenError::InvalidMessageFormat(format!(
                    "Unsupported pair: {}",
                    pair
                ))
                .into())
            }
        };

        // Process each trade in the array
        if let Some(trade) = trades_array.first() {
            let trade_array = trade.as_array().ok_or_else(|| {
                KrakenError::InvalidMessageFormat("Expected trade array".to_string())
            })?;

            if trade_array.len() < 6 {
                return Err(KrakenError::InvalidMessageFormat(
                    "Insufficient trade fields".to_string(),
                )
                .into());
            }

            // Parse trade fields: [price, volume, time, side, orderType, misc]
            let price_str = trade_array[0]
                .as_str()
                .ok_or_else(|| KrakenError::InvalidMessageFormat("Invalid price".to_string()))?;
            let volume_str = trade_array[1]
                .as_str()
                .ok_or_else(|| KrakenError::InvalidMessageFormat("Invalid volume".to_string()))?;
            let time_str = trade_array[2]
                .as_str()
                .ok_or_else(|| KrakenError::InvalidMessageFormat("Invalid time".to_string()))?;
            let side_str = trade_array[3]
                .as_str()
                .ok_or_else(|| KrakenError::InvalidMessageFormat("Invalid side".to_string()))?;

            // Convert to fixed-point format (8 decimal places)
            let price = Decimal::from_str_exact(price_str)
                .map_err(|e| {
                    KrakenError::InvalidMessageFormat(format!("Price parse error: {}", e))
                })?
                .to_f64()
                .ok_or_else(|| {
                    KrakenError::InvalidMessageFormat("Price conversion failed".to_string())
                })?;
            let price_fixed = (price * 100_000_000.0) as i64;

            let volume = Decimal::from_str_exact(volume_str)
                .map_err(|e| {
                    KrakenError::InvalidMessageFormat(format!("Volume parse error: {}", e))
                })?
                .to_f64()
                .ok_or_else(|| {
                    KrakenError::InvalidMessageFormat("Volume conversion failed".to_string())
                })?;
            let volume_fixed = (volume * 100_000_000.0) as i64;

            // Convert time to nanoseconds
            let time_seconds = time_str.parse::<f64>().map_err(|e| {
                KrakenError::InvalidMessageFormat(format!("Time parse error: {}", e))
            })?;
            let timestamp_ns = (time_seconds * 1_000_000_000.0) as u64;

            // Convert side: "b" = buy (0), "s" = sell (1)
            let side = match side_str {
                "b" => 0,
                "s" => 1,
                _ => {
                    return Err(KrakenError::InvalidMessageFormat(format!(
                        "Invalid side: {}",
                        side_str
                    ))
                    .into())
                }
            };

            // Create TradeTLV
            let trade_tlv = TradeTLV::new(
                self.config.venue_id,
                instrument_id,
                price_fixed,
                volume_fixed,
                side,
                timestamp_ns,
            );

            // Build Protocol V2 binary message
            let message =
                TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::KrakenCollector)
                    .add_tlv(TLVType::Trade, &trade_tlv)
                    .build();

            Ok(message)
        } else {
            Err(KrakenError::InvalidMessageFormat("No trades in array".to_string()).into())
        }
    }

    /// Parse Kraken order book message and convert to Protocol V2 TLV
    fn parse_book_message(&self, book_data: &Value) -> Result<Vec<u8>> {
        // Expected format: [channelID, book_data, "book", "PAIR"]
        let array = book_data.as_array().ok_or_else(|| {
            KrakenError::InvalidMessageFormat("Expected array format".to_string())
        })?;

        if array.len() < 4 {
            return Err(KrakenError::InvalidMessageFormat(
                "Insufficient array elements".to_string(),
            )
            .into());
        }

        let book = &array[1];
        let pair = array[3]
            .as_str()
            .ok_or_else(|| KrakenError::InvalidMessageFormat("Expected pair string".to_string()))?;

        // Convert pair to InstrumentId using proper symbol mapping
        let instrument_id = match pair {
            "XBT/USD" => InstrumentId::stock(self.config.venue_id, "BTCUSD"),
            "ETH/USD" => InstrumentId::stock(self.config.venue_id, "ETHUSD"),
            _ => {
                return Err(KrakenError::InvalidMessageFormat(format!(
                    "Unsupported pair: {}",
                    pair
                ))
                .into())
            }
        };

        // Parse best bid and ask for QuoteTLV
        let bids = book.get("bs").or_else(|| book.get("b"));
        let asks = book.get("as").or_else(|| book.get("a"));

        if let (Some(bids_array), Some(asks_array)) = (bids, asks) {
            let bids = bids_array.as_array().ok_or_else(|| {
                KrakenError::InvalidMessageFormat("Invalid bids format".to_string())
            })?;
            let asks = asks_array.as_array().ok_or_else(|| {
                KrakenError::InvalidMessageFormat("Invalid asks format".to_string())
            })?;

            if let (Some(best_bid), Some(best_ask)) = (bids.first(), asks.first()) {
                let bid_array = best_bid.as_array().ok_or_else(|| {
                    KrakenError::InvalidMessageFormat("Invalid bid array".to_string())
                })?;
                let ask_array = best_ask.as_array().ok_or_else(|| {
                    KrakenError::InvalidMessageFormat("Invalid ask array".to_string())
                })?;

                if bid_array.len() < 2 || ask_array.len() < 2 {
                    return Err(KrakenError::InvalidMessageFormat(
                        "Insufficient bid/ask data".to_string(),
                    )
                    .into());
                }

                // Parse bid and ask prices/sizes
                let bid_price_str = bid_array[0].as_str().ok_or_else(|| {
                    KrakenError::InvalidMessageFormat("Invalid bid price".to_string())
                })?;
                let bid_size_str = bid_array[1].as_str().ok_or_else(|| {
                    KrakenError::InvalidMessageFormat("Invalid bid size".to_string())
                })?;
                let ask_price_str = ask_array[0].as_str().ok_or_else(|| {
                    KrakenError::InvalidMessageFormat("Invalid ask price".to_string())
                })?;
                let ask_size_str = ask_array[1].as_str().ok_or_else(|| {
                    KrakenError::InvalidMessageFormat("Invalid ask size".to_string())
                })?;

                // Convert to fixed-point
                let bid_price = (Decimal::from_str_exact(bid_price_str)
                    .map_err(|e| {
                        KrakenError::InvalidMessageFormat(format!("Bid price error: {}", e))
                    })?
                    .to_f64()
                    .unwrap_or(0.0)
                    * 100_000_000.0) as i64;
                let bid_size = (Decimal::from_str_exact(bid_size_str)
                    .map_err(|e| {
                        KrakenError::InvalidMessageFormat(format!("Bid size error: {}", e))
                    })?
                    .to_f64()
                    .unwrap_or(0.0)
                    * 100_000_000.0) as i64;
                let ask_price = (Decimal::from_str_exact(ask_price_str)
                    .map_err(|e| {
                        KrakenError::InvalidMessageFormat(format!("Ask price error: {}", e))
                    })?
                    .to_f64()
                    .unwrap_or(0.0)
                    * 100_000_000.0) as i64;
                let ask_size = (Decimal::from_str_exact(ask_size_str)
                    .map_err(|e| {
                        KrakenError::InvalidMessageFormat(format!("Ask size error: {}", e))
                    })?
                    .to_f64()
                    .unwrap_or(0.0)
                    * 100_000_000.0) as i64;

                // Create QuoteTLV for order book update
                let quote_tlv = QuoteTLV::new(
                    self.config.venue_id,
                    instrument_id,
                    bid_price,
                    bid_size,
                    ask_price,
                    ask_size,
                    current_timestamp_ns(),
                );

                // Build Protocol V2 binary message
                let message =
                    TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::KrakenCollector)
                        .add_tlv(TLVType::Quote, &quote_tlv)
                        .build();

                Ok(message)
            } else {
                Err(KrakenError::InvalidMessageFormat("No bid/ask data".to_string()).into())
            }
        } else {
            Err(KrakenError::InvalidMessageFormat("Missing bid/ask arrays".to_string()).into())
        }
    }

    /// Create subscription message for trading pairs
    fn create_subscription_message(&self, pairs: &[String], channel: &str) -> String {
        serde_json::json!({
            "event": "subscribe",
            "pair": pairs,
            "subscription": {
                "name": channel
            }
        })
        .to_string()
    }
}

#[async_trait]
impl InputAdapter for KrakenCollector {
    fn venue(&self) -> VenueId {
        self.config.venue_id
    }

    async fn start(&mut self) -> Result<()> {
        *self.running.write().await = true;
        *self.reconnect_attempts.write().await = 0;

        tracing::info!("Starting Kraken collector for {:?}", self.config.venue_id);

        // Start WebSocket connection task
        let config = self.config.clone();
        let output_tx = self.output_tx.clone();
        let running = self.running.clone();
        let reconnect_attempts = self.reconnect_attempts.clone();
        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            Self::websocket_task(config, output_tx, running, reconnect_attempts, metrics).await;
        });

        tracing::info!("Kraken collector started successfully");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().await = false;
        tracing::info!("Kraken collector stopped");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        // Connection status tracking removed (no StateManager)
        false
    }

    fn tracked_instruments(&self) -> Vec<InstrumentId> {
        // Convert trading pairs to InstrumentIds
        self.config
            .trading_pairs
            .iter()
            .filter_map(|pair| match pair.as_str() {
                "XBT/USD" => Some(InstrumentId::stock(self.config.venue_id, "BTCUSD")),
                "ETH/USD" => Some(InstrumentId::stock(self.config.venue_id, "ETHUSD")),
                _ => None,
            })
            .collect()
    }

    async fn subscribe(&mut self, _instruments: Vec<InstrumentId>) -> Result<()> {
        // For now, we subscribe to all configured pairs on start
        // This could be enhanced to support dynamic subscription
        tracing::info!(
            "Kraken subscription management not yet implemented for dynamic instruments"
        );
        Ok(())
    }

    async fn unsubscribe(&mut self, _instruments: Vec<InstrumentId>) -> Result<()> {
        tracing::info!("Kraken unsubscription not yet implemented");
        Ok(())
    }

    async fn reconnect(&mut self) -> Result<()> {
        tracing::info!("Triggering Kraken reconnection");
        // The WebSocket task handles reconnection automatically
        // This could trigger a faster reconnect
        Ok(())
    }

    async fn health_check(&self) -> HealthStatus {
        if self.is_connected() {
            HealthStatus::healthy(crate::input::ConnectionState::Connected, 0)
        } else {
            let attempts = *self.reconnect_attempts.read().await;
            HealthStatus::unhealthy(
                crate::input::ConnectionState::Disconnected,
                format!("WebSocket disconnected (reconnect attempts: {})", attempts),
            )
        }
    }
}

impl KrakenCollector {
    /// Main WebSocket connection and message handling task
    async fn websocket_task(
        config: KrakenConfig,
        output_tx: mpsc::Sender<Vec<u8>>,
        running: Arc<RwLock<bool>>,
        reconnect_attempts: Arc<RwLock<usize>>,
        metrics: Arc<AdapterMetrics>,
    ) {
        while *running.read().await {
            let attempts = *reconnect_attempts.read().await;

            if attempts >= config.max_reconnect_attempts {
                tracing::error!(
                    "Max reconnection attempts ({}) reached, stopping",
                    config.max_reconnect_attempts
                );
                break;
            }

            match Self::connect_and_run(&config, &output_tx, &running, &metrics).await {
                Ok(_) => {
                    tracing::info!("WebSocket connection closed normally");
                    *reconnect_attempts.write().await = 0;
                }
                Err(e) => {
                    *reconnect_attempts.write().await += 1;
                    let current_attempts = *reconnect_attempts.read().await;

                    tracing::error!("WebSocket error (attempt {}): {}", current_attempts, e);

                    if current_attempts < config.max_reconnect_attempts {
                        let delay = std::time::Duration::from_millis(
                            config.reconnect_interval_ms * (1 << current_attempts.min(5)), // Exponential backoff, max 32x
                        );
                        tracing::info!("Reconnecting in {:?}...", delay);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        tracing::info!("WebSocket task terminated");
    }

    /// Connect to Kraken WebSocket and handle messages
    async fn connect_and_run(
        config: &KrakenConfig,
        output_tx: &mpsc::Sender<Vec<u8>>,
        running: &Arc<RwLock<bool>>,
        metrics: &Arc<AdapterMetrics>,
    ) -> std::result::Result<(), KrakenError> {
        tracing::info!("Connecting to Kraken WebSocket: {}", config.websocket_url);

        let url = Url::parse(&config.websocket_url)
            .map_err(|e| KrakenError::WebSocketConnectionFailed(format!("Invalid URL: {}", e)))?;

        let (ws_stream, _) = connect_async(url.as_str()).await.map_err(|e| {
            KrakenError::WebSocketConnectionFailed(format!("Connection failed: {}", e))
        })?;

        tracing::info!("WebSocket connected successfully");

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Subscribe to trading pairs
        for channel in &["trade", "book"] {
            let subscription = serde_json::json!({
                "event": "subscribe",
                "pair": config.trading_pairs,
                "subscription": {
                    "name": channel
                }
            });

            let sub_message = Message::Text(subscription.to_string());
            ws_sender
                .send(sub_message)
                .await
                .map_err(|e| KrakenError::SubscriptionFailed {
                    pair: format!("{:?}", config.trading_pairs),
                    reason: e.to_string(),
                })?;

            tracing::info!(
                "Subscribed to {} for pairs: {:?}",
                channel,
                config.trading_pairs
            );
        }

        // Message handling loop
        let mut last_heartbeat = std::time::Instant::now();

        while *running.read().await {
            tokio::select! {
                message = ws_receiver.next() => {
                    match message {
                        Some(Ok(msg)) => {
                            if let Err(e) = Self::handle_message(msg, output_tx, config, metrics).await {
                                tracing::error!("Message handling error: {}", e);
                                metrics.increment_errors();
                            } else {
                                last_heartbeat = std::time::Instant::now();
                            }
                        }
                        Some(Err(e)) => {
                            return Err(KrakenError::WebSocketConnectionFailed(format!("WebSocket error: {}", e)).into());
                        }
                        None => {
                            tracing::warn!("WebSocket stream ended");
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(config.heartbeat_timeout_ms)) => {
                    if last_heartbeat.elapsed().as_millis() > config.heartbeat_timeout_ms as u128 {
                        return Err(KrakenError::HeartbeatTimeout {
                            timeout_ms: config.heartbeat_timeout_ms
                        }.into());
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle individual WebSocket messages
    async fn handle_message(
        message: Message,
        output_tx: &mpsc::Sender<Vec<u8>>,
        config: &KrakenConfig,
        metrics: &Arc<AdapterMetrics>,
    ) -> std::result::Result<(), KrakenError> {
        match message {
            Message::Text(text) => {
                let value: Value = serde_json::from_str(&text).map_err(|e| {
                    KrakenError::InvalidMessageFormat(format!("JSON parse error: {}", e))
                })?;

                // Handle different message types
                if let Some(event) = value.get("event") {
                    match event.as_str() {
                        Some("systemStatus") => {
                            tracing::debug!("Received system status: {}", text);
                        }
                        Some("subscriptionStatus") => {
                            tracing::info!("Subscription status: {}", text);
                        }
                        Some("heartbeat") => {
                            tracing::debug!("Received heartbeat");
                        }
                        _ => {
                            tracing::debug!("Unknown event type: {}", text);
                        }
                    }
                } else if value.is_array() {
                    // Handle data messages (trades, book updates)
                    let array = value.as_array().unwrap();
                    if array.len() >= 4 {
                        if let Some(channel) = array[2].as_str() {
                            match channel {
                                "trade" => {
                                    if let Ok(trade_message) =
                                        Self::parse_trade_message_static(&value, config.venue_id)
                                    {
                                        if let Err(e) = output_tx.send(trade_message).await {
                                            tracing::error!("Failed to send trade message: {}", e);
                                        } else {
                                            metrics.increment_messages_sent();
                                        }
                                    }
                                }
                                "book" => {
                                    if let Ok(book_message) =
                                        Self::parse_book_message_static(&value, config.venue_id)
                                    {
                                        if let Err(e) = output_tx.send(book_message).await {
                                            tracing::error!("Failed to send book message: {}", e);
                                        } else {
                                            metrics.increment_messages_sent();
                                        }
                                    }
                                }
                                _ => {
                                    tracing::debug!("Unknown channel: {}", channel);
                                }
                            }
                        }
                    }
                }
            }
            Message::Binary(_data) => {
                tracing::debug!("Received binary message (not supported by Kraken)");
            }
            Message::Ping(data) => {
                tracing::debug!("Received ping, sending pong");
                // WebSocket library handles pong automatically
            }
            Message::Pong(_data) => {
                tracing::debug!("Received pong");
            }
            Message::Close(_) => {
                tracing::info!("Received close message");
            }
            Message::Frame(_) => {
                tracing::debug!("Received raw frame");
            }
        }

        Ok(())
    }

    /// Static version of parse_trade_message for use in static context
    fn parse_trade_message_static(trade_data: &Value, venue_id: VenueId) -> Result<Vec<u8>> {
        // Same parsing logic as instance method but static
        let array = trade_data.as_array().ok_or_else(|| {
            KrakenError::InvalidMessageFormat("Expected array format".to_string())
        })?;

        if array.len() < 4 {
            return Err(KrakenError::InvalidMessageFormat(
                "Insufficient array elements".to_string(),
            )
            .into());
        }

        let trades_array = array[1].as_array().ok_or_else(|| {
            KrakenError::InvalidMessageFormat("Expected trades array".to_string())
        })?;

        let pair = array[3]
            .as_str()
            .ok_or_else(|| KrakenError::InvalidMessageFormat("Expected pair string".to_string()))?;

        // Convert pair to InstrumentId using proper symbol mapping
        let instrument_id = match pair {
            "XBT/USD" => InstrumentId::stock(venue_id, "BTCUSD"),
            "ETH/USD" => InstrumentId::stock(venue_id, "ETHUSD"),
            _ => {
                return Err(KrakenError::InvalidMessageFormat(format!(
                    "Unsupported pair: {}",
                    pair
                ))
                .into())
            }
        };

        if let Some(trade) = trades_array.first() {
            let trade_array = trade.as_array().ok_or_else(|| {
                KrakenError::InvalidMessageFormat("Expected trade array".to_string())
            })?;

            if trade_array.len() < 6 {
                return Err(KrakenError::InvalidMessageFormat(
                    "Insufficient trade fields".to_string(),
                )
                .into());
            }

            // Parse trade fields
            let price_str = trade_array[0]
                .as_str()
                .ok_or_else(|| KrakenError::InvalidMessageFormat("Invalid price".to_string()))?;
            let volume_str = trade_array[1]
                .as_str()
                .ok_or_else(|| KrakenError::InvalidMessageFormat("Invalid volume".to_string()))?;
            let time_str = trade_array[2]
                .as_str()
                .ok_or_else(|| KrakenError::InvalidMessageFormat("Invalid time".to_string()))?;
            let side_str = trade_array[3]
                .as_str()
                .ok_or_else(|| KrakenError::InvalidMessageFormat("Invalid side".to_string()))?;

            // Convert to fixed-point
            let price = Decimal::from_str_exact(price_str)
                .map_err(|e| {
                    KrakenError::InvalidMessageFormat(format!("Price parse error: {}", e))
                })?
                .to_f64()
                .ok_or_else(|| {
                    KrakenError::InvalidMessageFormat("Price conversion failed".to_string())
                })?;
            let price_fixed = (price * 100_000_000.0) as i64;

            let volume = Decimal::from_str_exact(volume_str)
                .map_err(|e| {
                    KrakenError::InvalidMessageFormat(format!("Volume parse error: {}", e))
                })?
                .to_f64()
                .ok_or_else(|| {
                    KrakenError::InvalidMessageFormat("Volume conversion failed".to_string())
                })?;
            let volume_fixed = (volume * 100_000_000.0) as i64;

            let time_seconds = time_str.parse::<f64>().map_err(|e| {
                KrakenError::InvalidMessageFormat(format!("Time parse error: {}", e))
            })?;
            let timestamp_ns = (time_seconds * 1_000_000_000.0) as u64;

            let side = match side_str {
                "b" => 0,
                "s" => 1,
                _ => {
                    return Err(KrakenError::InvalidMessageFormat(format!(
                        "Invalid side: {}",
                        side_str
                    ))
                    .into())
                }
            };

            // Create TradeTLV and build message
            let trade_tlv = TradeTLV::new(
                venue_id,
                instrument_id,
                price_fixed,
                volume_fixed,
                side,
                timestamp_ns,
            );

            let message =
                TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::KrakenCollector)
                    .add_tlv(TLVType::Trade, &trade_tlv)
                    .build();

            Ok(message)
        } else {
            Err(KrakenError::InvalidMessageFormat("No trades in array".to_string()).into())
        }
    }

    /// Static version of parse_book_message for use in static context
    fn parse_book_message_static(book_data: &Value, venue_id: VenueId) -> Result<Vec<u8>> {
        // Expected format: [channelID, book_data, "book", "PAIR"]
        let array = book_data.as_array().ok_or_else(|| {
            KrakenError::InvalidMessageFormat("Expected array format".to_string())
        })?;

        if array.len() < 4 {
            return Err(KrakenError::InvalidMessageFormat(
                "Insufficient array elements".to_string(),
            )
            .into());
        }

        let book = &array[1];
        let pair = array[3]
            .as_str()
            .ok_or_else(|| KrakenError::InvalidMessageFormat("Expected pair string".to_string()))?;

        // Convert pair to InstrumentId using proper symbol mapping
        let instrument_id = match pair {
            "XBT/USD" => InstrumentId::stock(venue_id, "BTCUSD"),
            "ETH/USD" => InstrumentId::stock(venue_id, "ETHUSD"),
            _ => {
                return Err(KrakenError::InvalidMessageFormat(format!(
                    "Unsupported pair: {}",
                    pair
                ))
                .into())
            }
        };

        // Parse best bid and ask for QuoteTLV
        let bids = book.get("bs").or_else(|| book.get("b"));
        let asks = book.get("as").or_else(|| book.get("a"));

        if let (Some(bids_array), Some(asks_array)) = (bids, asks) {
            let bids = bids_array.as_array().ok_or_else(|| {
                KrakenError::InvalidMessageFormat("Invalid bids format".to_string())
            })?;
            let asks = asks_array.as_array().ok_or_else(|| {
                KrakenError::InvalidMessageFormat("Invalid asks format".to_string())
            })?;

            if let (Some(best_bid), Some(best_ask)) = (bids.first(), asks.first()) {
                let bid_array = best_bid.as_array().ok_or_else(|| {
                    KrakenError::InvalidMessageFormat("Invalid bid array".to_string())
                })?;
                let ask_array = best_ask.as_array().ok_or_else(|| {
                    KrakenError::InvalidMessageFormat("Invalid ask array".to_string())
                })?;

                if bid_array.len() < 2 || ask_array.len() < 2 {
                    return Err(KrakenError::InvalidMessageFormat(
                        "Insufficient bid/ask data".to_string(),
                    )
                    .into());
                }

                // Parse bid and ask prices/sizes
                let bid_price_str = bid_array[0].as_str().ok_or_else(|| {
                    KrakenError::InvalidMessageFormat("Invalid bid price".to_string())
                })?;
                let bid_size_str = bid_array[1].as_str().ok_or_else(|| {
                    KrakenError::InvalidMessageFormat("Invalid bid size".to_string())
                })?;
                let ask_price_str = ask_array[0].as_str().ok_or_else(|| {
                    KrakenError::InvalidMessageFormat("Invalid ask price".to_string())
                })?;
                let ask_size_str = ask_array[1].as_str().ok_or_else(|| {
                    KrakenError::InvalidMessageFormat("Invalid ask size".to_string())
                })?;

                // Convert to fixed-point
                let bid_price = (Decimal::from_str_exact(bid_price_str)
                    .map_err(|e| {
                        KrakenError::InvalidMessageFormat(format!("Bid price error: {}", e))
                    })?
                    .to_f64()
                    .unwrap_or(0.0)
                    * 100_000_000.0) as i64;
                let bid_size = (Decimal::from_str_exact(bid_size_str)
                    .map_err(|e| {
                        KrakenError::InvalidMessageFormat(format!("Bid size error: {}", e))
                    })?
                    .to_f64()
                    .unwrap_or(0.0)
                    * 100_000_000.0) as i64;
                let ask_price = (Decimal::from_str_exact(ask_price_str)
                    .map_err(|e| {
                        KrakenError::InvalidMessageFormat(format!("Ask price error: {}", e))
                    })?
                    .to_f64()
                    .unwrap_or(0.0)
                    * 100_000_000.0) as i64;
                let ask_size = (Decimal::from_str_exact(ask_size_str)
                    .map_err(|e| {
                        KrakenError::InvalidMessageFormat(format!("Ask size error: {}", e))
                    })?
                    .to_f64()
                    .unwrap_or(0.0)
                    * 100_000_000.0) as i64;

                // Create QuoteTLV for order book update
                let quote_tlv = QuoteTLV::new(
                    venue_id,
                    instrument_id,
                    bid_price,
                    bid_size,
                    ask_price,
                    ask_size,
                    current_timestamp_ns(),
                );

                // Build Protocol V2 binary message
                let message =
                    TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::KrakenCollector)
                        .add_tlv(TLVType::Quote, &quote_tlv)
                        .build();

                Ok(message)
            } else {
                Err(KrakenError::InvalidMessageFormat("No bid/ask data".to_string()).into())
            }
        } else {
            Err(KrakenError::InvalidMessageFormat("Missing bid/ask arrays".to_string()).into())
        }
    }
}
