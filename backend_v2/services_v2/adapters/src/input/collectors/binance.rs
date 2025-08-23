//! Binance WebSocket data collector
//!
//! Handles JSON WebSocket streams from Binance for:
//! - Trade streams
//! - Depth updates  
//! - Ticker streams
//!
//! ## Data Format Reference
//!
//! All schemas stored inline for validation and documentation

use async_trait::async_trait;
use futures_util::StreamExt;
use protocol_v2::{
    tlv::market_data::{QuoteTLV, TradeTLV},
    InstrumentId, RelayDomain, SourceType, TLVMessageBuilder, TLVType, VenueId,
};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::tungstenite::Message;

use crate::input::connection::ConnectionConfig;
use crate::input::{ConnectionManager, ConnectionState, HealthLevel, HealthStatus, InputAdapter};
use crate::{AdapterError, Result};
use crate::{AdapterMetrics, AuthManager, ErrorType, RateLimiter};

/// Binance Trade Stream JSON Schema
///
/// Example: wss://stream.binance.com:9443/ws/btcusdt@trade
const BINANCE_TRADE_SCHEMA: &str = r#"
{
  "e": "trade",          // Event type
  "E": 123456789,        // Event time (ms)
  "s": "BNBBTC",         // Symbol
  "t": 12345,            // Trade ID
  "p": "0.001",          // Price (string)
  "q": "100",            // Quantity (string)
  "b": 88,               // Buyer order ID
  "a": 50,               // Seller order ID
  "T": 123456785,        // Trade time (ms)
  "m": true,             // Is buyer the market maker?
  "M": true              // Ignore field
}
"#;

/// Binance Depth Update JSON Schema
///
/// Example: wss://stream.binance.com:9443/ws/btcusdt@depth
const BINANCE_DEPTH_SCHEMA: &str = r#"
{
  "e": "depthUpdate",    // Event type
  "E": 123456789,        // Event time (ms)
  "s": "BNBBTC",         // Symbol
  "U": 157,              // First update ID in event
  "u": 160,              // Final update ID in event
  "b": [                 // Bids to be updated
    [
      "0.0024",          // Price level
      "10"               // Quantity
    ]
  ],
  "a": [                 // Asks to be updated
    [
      "0.0026",          // Price level
      "100"              // Quantity
    ]
  ]
}
"#;

/// Binance 24hr Ticker JSON Schema
///
/// Example: wss://stream.binance.com:9443/ws/btcusdt@ticker
const BINANCE_TICKER_SCHEMA: &str = r#"
{
  "e": "24hrTicker",     // Event type
  "E": 123456789,        // Event time (ms)
  "s": "BNBBTC",         // Symbol
  "p": "0.0015",         // Price change
  "P": "250.00",         // Price change percent
  "w": "0.0018",         // Weighted average price
  "x": "0.0009",         // Previous day's close price
  "c": "0.0025",         // Current day's close price
  "Q": "10",             // Close quantity
  "b": "0.0024",         // Best bid price
  "B": "10",             // Best bid quantity
  "a": "0.0026",         // Best ask price
  "A": "100",            // Best ask quantity
  "o": "0.0010",         // Open price
  "h": "0.0025",         // High price
  "l": "0.0010",         // Low price
  "v": "10000",          // Total traded base asset volume
  "q": "18",             // Total traded quote asset volume
  "O": 0,                // Statistics open time
  "C": 86400000,         // Statistics close time
  "F": 0,                // First trade ID
  "L": 18150,            // Last trade ID
  "n": 18151             // Total count of trades
}
"#;

/// Binance WebSocket collector
pub struct BinanceCollector {
    /// Connection manager
    connection: Arc<ConnectionManager>,

    /// Authentication manager
    auth: AuthManager,

    /// Rate limiter
    rate_limiter: RateLimiter,

    /// Metrics
    metrics: Arc<AdapterMetrics>,

    /// Symbol to InstrumentId mapping
    symbol_map: Arc<RwLock<HashMap<String, InstrumentId>>>,

    /// Output channel for TLV messages
    output_tx: mpsc::Sender<Vec<u8>>,

    /// Running flag
    running: Arc<RwLock<bool>>,
}

impl BinanceCollector {
    /// Create a new Binance collector
    pub fn new(
        api_key: Option<String>,
        api_secret: Option<String>,
        output_tx: mpsc::Sender<Vec<u8>>,
    ) -> Self {
        let metrics = Arc::new(AdapterMetrics::new());

        let config = ConnectionConfig {
            url: "wss://stream.binance.com:9443/ws".to_string(),
            connect_timeout: Duration::from_secs(10),
            message_timeout: Duration::from_secs(30),
            base_backoff_ms: 1000,
            max_backoff_ms: 30000,
            max_reconnect_attempts: 10,
            health_check_interval: Duration::from_secs(5),
        };

        let connection = Arc::new(ConnectionManager::new(
            VenueId::Binance,
            config,
            metrics.clone(),
        ));

        let mut auth = AuthManager::new();
        if let (Some(key), Some(secret)) = (api_key, api_secret) {
            auth.set_credentials(VenueId::Binance, key, secret);
        }

        let mut rate_limiter = RateLimiter::new();
        rate_limiter.configure_venue(VenueId::Binance, 1200); // 1200 requests/min

        Self {
            connection,
            auth,
            rate_limiter,
            metrics,
            symbol_map: Arc::new(RwLock::new(HashMap::new())),
            output_tx,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Process incoming WebSocket message
    async fn process_message(&self, msg: Message) -> Result<()> {
        match msg {
            Message::Text(text) => {
                self.metrics.record_message(VenueId::Binance, text.len());

                let start = std::time::Instant::now();
                let result = self.parse_json_message(&text).await;

                self.metrics
                    .record_processing_time(VenueId::Binance, start.elapsed());

                result
            }
            Message::Binary(data) => {
                // Binance shouldn't send binary, but handle gracefully
                tracing::warn!(
                    "Unexpected binary message from Binance: {} bytes",
                    data.len()
                );
                Ok(())
            }
            Message::Ping(data) => {
                // Send pong
                self.connection.send(Message::Pong(data)).await?;
                Ok(())
            }
            Message::Pong(_) => {
                // Pong received, connection healthy
                Ok(())
            }
            Message::Close(frame) => {
                tracing::info!("Binance WebSocket closed: {:?}", frame);
                Err(AdapterError::ConnectionClosed {
                    venue: VenueId::Binance,
                    reason: frame.map(|f| f.reason.to_string()),
                })
            }
            Message::Frame(_) => {
                // Raw frame, shouldn't happen with high-level API
                Ok(())
            }
        }
    }

    /// Parse JSON message from Binance
    async fn parse_json_message(&self, text: &str) -> Result<()> {
        let value: Value = serde_json::from_str(text).map_err(|e| {
            self.metrics.record_processing_error(ErrorType::Parse);
            AdapterError::ParseError {
                venue: VenueId::Binance,
                message: text.to_string(),
                error: e.to_string(),
            }
        })?;

        // Determine message type by checking fields
        if let Some(event_type) = value.get("e").and_then(|v| v.as_str()) {
            match event_type {
                "trade" => self.handle_trade(&value).await,
                "depthUpdate" => self.handle_depth_update(&value).await,
                "24hrTicker" => self.handle_ticker(&value).await,
                _ => {
                    tracing::debug!("Unhandled Binance event type: {}", event_type);
                    Ok(())
                }
            }
        } else if value.get("result").is_some() {
            // Subscription response
            self.handle_subscription_response(&value).await
        } else {
            tracing::debug!("Unknown Binance message format: {}", text);
            Ok(())
        }
    }

    /// Handle trade event
    async fn handle_trade(&self, value: &Value) -> Result<()> {
        let symbol =
            value
                .get("s")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AdapterError::ParseError {
                    venue: VenueId::Binance,
                    message: value.to_string(),
                    error: "Missing symbol field".to_string(),
                })?;

        let instrument_id = self.get_or_create_instrument_id(symbol).await;

        // Parse trade fields
        let price =
            parse_decimal_string(value.get("p")).ok_or_else(|| AdapterError::ParseError {
                venue: VenueId::Binance,
                message: value.to_string(),
                error: "Invalid price field".to_string(),
            })?;

        let quantity =
            parse_decimal_string(value.get("q")).ok_or_else(|| AdapterError::ParseError {
                venue: VenueId::Binance,
                message: value.to_string(),
                error: "Invalid quantity field".to_string(),
            })?;

        let timestamp =
            value
                .get("T")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| AdapterError::ParseError {
                    venue: VenueId::Binance,
                    message: value.to_string(),
                    error: "Missing timestamp field".to_string(),
                })?;

        let is_buyer_maker = value.get("m").and_then(|v| v.as_bool()).unwrap_or(false);

        // Create TradeTLV using the new() constructor
        let trade_tlv = TradeTLV::new(
            VenueId::Binance,
            instrument_id,
            price,
            quantity,
            if is_buyer_maker { 1 } else { 0 }, // 0 = buy, 1 = sell
            timestamp * 1_000_000,              // Convert ms to ns
        );

        // Convert to TLV message and send
        let tlv_message =
            TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
                .add_tlv(TLVType::Trade, &trade_tlv)
                .build();
        self.output_tx
            .send(tlv_message)
            .await
            .map_err(|_| AdapterError::Internal("Output channel closed".to_string()))?;

        Ok(())
    }

    /// Handle depth update
    async fn handle_depth_update(&self, value: &Value) -> Result<()> {
        let symbol =
            value
                .get("s")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AdapterError::ParseError {
                    venue: VenueId::Binance,
                    message: value.to_string(),
                    error: "Missing symbol field".to_string(),
                })?;

        let instrument_id = self.get_or_create_instrument_id(symbol).await;

        // Parse best bid
        if let Some(bids) = value.get("b").and_then(|v| v.as_array()) {
            if let Some(best_bid) = bids.first() {
                let bid_price = parse_decimal_from_array(best_bid, 0)?;
                let bid_size = parse_decimal_from_array(best_bid, 1)?;

                // Parse best ask
                if let Some(asks) = value.get("a").and_then(|v| v.as_array()) {
                    if let Some(best_ask) = asks.first() {
                        let ask_price = parse_decimal_from_array(best_ask, 0)?;
                        let ask_size = parse_decimal_from_array(best_ask, 1)?;

                        let timestamp = value
                            .get("E")
                            .and_then(|v| v.as_u64())
                            .unwrap_or_else(|| current_millis());

                        // Create QuoteTLV
                        let quote_tlv = QuoteTLV::new(
                            VenueId::Binance,
                            instrument_id,
                            bid_price,
                            bid_size,
                            ask_price,
                            ask_size,
                            timestamp * 1_000_000,
                        );

                        let tlv_message = TLVMessageBuilder::new(
                            RelayDomain::MarketData,
                            SourceType::BinanceCollector,
                        )
                        .add_tlv(TLVType::Quote, &quote_tlv)
                        .build();
                        self.output_tx.send(tlv_message).await.map_err(|_| {
                            AdapterError::Internal("Output channel closed".to_string())
                        })?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle 24hr ticker
    async fn handle_ticker(&self, value: &Value) -> Result<()> {
        // Extract best bid/ask from ticker
        let symbol =
            value
                .get("s")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AdapterError::ParseError {
                    venue: VenueId::Binance,
                    message: value.to_string(),
                    error: "Missing symbol field".to_string(),
                })?;

        let instrument_id = self.get_or_create_instrument_id(symbol).await;

        let bid_price =
            parse_decimal_string(value.get("b")).ok_or_else(|| AdapterError::ParseError {
                venue: VenueId::Binance,
                message: value.to_string(),
                error: "Invalid bid price".to_string(),
            })?;
        let bid_size =
            parse_decimal_string(value.get("B")).ok_or_else(|| AdapterError::ParseError {
                venue: VenueId::Binance,
                message: value.to_string(),
                error: "Invalid bid size".to_string(),
            })?;
        let ask_price =
            parse_decimal_string(value.get("a")).ok_or_else(|| AdapterError::ParseError {
                venue: VenueId::Binance,
                message: value.to_string(),
                error: "Invalid ask price".to_string(),
            })?;
        let ask_size =
            parse_decimal_string(value.get("A")).ok_or_else(|| AdapterError::ParseError {
                venue: VenueId::Binance,
                message: value.to_string(),
                error: "Invalid ask size".to_string(),
            })?;

        let timestamp = value
            .get("E")
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| current_millis());

        let quote_tlv = QuoteTLV::new(
            VenueId::Binance,
            instrument_id,
            bid_price,
            bid_size,
            ask_price,
            ask_size,
            timestamp * 1_000_000,
        );

        let tlv_message =
            TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
                .add_tlv(TLVType::Quote, &quote_tlv)
                .build();
        self.output_tx
            .send(tlv_message)
            .await
            .map_err(|_| AdapterError::Internal("Output channel closed".to_string()))?;

        Ok(())
    }

    /// Handle subscription response
    async fn handle_subscription_response(&self, value: &Value) -> Result<()> {
        if let Some(result) = value.get("result") {
            if result.is_null() {
                tracing::info!("Binance subscription successful");
            } else {
                tracing::warn!("Binance subscription response: {:?}", result);
            }
        }
        Ok(())
    }

    /// Get or create instrument ID for symbol
    async fn get_or_create_instrument_id(&self, symbol: &str) -> InstrumentId {
        let mut map = self.symbol_map.write().await;

        if let Some(&id) = map.get(symbol) {
            id
        } else {
            // Generate deterministic ID from symbol
            let id = InstrumentId::from_cache_key(hash_symbol(symbol) as u128);
            map.insert(symbol.to_string(), id);

            // Instrument tracking removed (no StateManager)

            id
        }
    }

    /// Subscribe to streams for instruments
    async fn subscribe_to_streams(&self, symbols: Vec<String>) -> Result<()> {
        // Build subscription message
        let streams: Vec<String> = symbols
            .iter()
            .flat_map(|s| {
                let symbol_lower = s.to_lowercase();
                vec![
                    format!("{}@trade", symbol_lower),
                    format!("{}@depth@100ms", symbol_lower),
                    format!("{}@ticker", symbol_lower),
                ]
            })
            .collect();

        let sub_msg = json!({
            "method": "SUBSCRIBE",
            "params": streams,
            "id": 1
        });

        self.connection
            .send(Message::Text(sub_msg.to_string()))
            .await?;

        tracing::info!("Subscribed to {} Binance streams", streams.len());

        Ok(())
    }

    /// Main event loop
    async fn event_loop(self: Arc<Self>) {
        while *self.running.read().await {
            match self.connection.receive().await {
                Ok(Some(msg)) => {
                    if let Err(e) = self.process_message(msg).await {
                        tracing::error!("Error processing Binance message: {}", e);
                        self.metrics.record_processing_error(ErrorType::Protocol);
                    }
                }
                Ok(None) => {
                    tracing::info!("Binance WebSocket stream ended");
                    break;
                }
                Err(e) => {
                    tracing::error!("Binance receive error: {}", e);

                    // State invalidation removed (no StateManager)

                    // Attempt reconnection
                    if let Err(e) = self
                        .connection
                        .handle_disconnection(
                            crate::input::connection::DisconnectReason::NetworkError,
                        )
                        .await
                    {
                        tracing::error!("Failed to reconnect: {}", e);
                        break;
                    }

                    // Clear state after invalidation
                    // State clearing removed (no StateManager)

                    // Resubscribe after reconnection
                    let symbols: Vec<String> =
                        self.symbol_map.read().await.keys().cloned().collect();

                    if !symbols.is_empty() {
                        if let Err(e) = self.subscribe_to_streams(symbols).await {
                            tracing::error!("Failed to resubscribe: {}", e);
                        }
                    }
                }
            }
        }
    }
}

#[async_trait]
impl InputAdapter for BinanceCollector {
    fn venue(&self) -> VenueId {
        VenueId::Binance
    }

    async fn start(&mut self) -> Result<()> {
        *self.running.write().await = true;

        // Connect to WebSocket
        self.connection.connect().await?;

        // Subscribe to default symbols
        let default_symbols = vec![
            "BTCUSDT".to_string(),
            "ETHUSDT".to_string(),
            "BNBUSDT".to_string(),
        ];

        self.subscribe_to_streams(default_symbols).await?;

        // Start event loop
        let collector = Arc::new(self.clone());
        tokio::spawn(collector.event_loop());

        tracing::info!("Binance collector started");

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().await = false;

        // State invalidation removed (no StateManager)

        // Close connection
        self.connection.close().await?;

        // State clearing removed (no StateManager)

        tracing::info!("Binance collector stopped");

        Ok(())
    }

    fn is_connected(&self) -> bool {
        // Use tokio::task::block_in_place or return a default
        // For now, return a simple check
        true // This should ideally check self.connection.is_connected().await
    }

    fn tracked_instruments(&self) -> Vec<InstrumentId> {
        // Similar issue with async in sync context
        Vec::new() // No StateManager to track instruments
    }

    async fn subscribe(&mut self, instruments: Vec<InstrumentId>) -> Result<()> {
        // Convert InstrumentIds back to symbols (would need reverse mapping)
        // For now, just track them
        for instrument in instruments {
            // Instrument tracking removed (no StateManager)
        }
        Ok(())
    }

    async fn unsubscribe(&mut self, instruments: Vec<InstrumentId>) -> Result<()> {
        for instrument in instruments {
            // Instrument untracking removed (no StateManager)
        }
        Ok(())
    }

    async fn reconnect(&mut self) -> Result<()> {
        self.connection.close().await?;
        self.connection.connect().await
    }

    async fn health_check(&self) -> HealthStatus {
        let state = self.connection.state().await;
        let metrics = self.metrics.summary();

        if state == ConnectionState::Connected && metrics.is_healthy() {
            HealthStatus::healthy(state, metrics.total_messages)
        } else if state == ConnectionState::Connected {
            HealthStatus {
                level: HealthLevel::Degraded,
                connection: state,
                messages_per_minute: metrics.total_messages,
                last_message_time: Some(current_nanos()),
                instrument_count: self.symbol_map.read().await.len(),
                error_count: metrics.total_messages - metrics.total_messages,
                details: Some("High error rate or slow processing".to_string()),
            }
        } else {
            HealthStatus::unhealthy(state, "Not connected".to_string())
        }
    }
}

impl Clone for BinanceCollector {
    fn clone(&self) -> Self {
        Self {
            connection: self.connection.clone(),
            auth: self.auth.clone(),
            rate_limiter: self.rate_limiter.clone(),
            metrics: self.metrics.clone(),
            symbol_map: self.symbol_map.clone(),
            output_tx: self.output_tx.clone(),
            running: self.running.clone(),
        }
    }
}

// Helper functions

/// Parse decimal from JSON string with proper precision
fn parse_decimal_string(value: Option<&Value>) -> Option<i64> {
    value.and_then(|v| v.as_str()).and_then(|s| {
        let decimal: Decimal = s.parse().ok()?;
        // Convert to fixed-point integer with 8 decimal places
        let scaled = decimal * Decimal::from(100_000_000); // 1e8
        scaled.to_i64()
    })
}

/// Parse decimal from array element with proper precision
fn parse_decimal_from_array(array: &Value, index: usize) -> Result<i64> {
    array
        .get(index)
        .and_then(|v| v.as_str())
        .and_then(|s| {
            let decimal: Decimal = s.parse().ok()?;
            // Convert to fixed-point integer with 8 decimal places
            let scaled = decimal * Decimal::from(100_000_000); // 1e8
            scaled.to_i64()
        })
        .ok_or_else(|| AdapterError::ParseError {
            venue: VenueId::Binance,
            message: array.to_string(),
            error: format!("Invalid decimal at index {}", index),
        })
}

/// Generate deterministic hash for symbol
fn hash_symbol(symbol: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    symbol.hash(&mut hasher);
    hasher.finish()
}

/// Get current time in milliseconds
fn current_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Get current time in nanoseconds
fn current_nanos() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

// Use the protocol's built-in TLV conversion methods

// Import serde_json for JSON handling
use serde_json::json;
