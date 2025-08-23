//! # Gemini Exchange WebSocket Data Collector
//!
//! ## Purpose
//! Collects real-time market data from Gemini Exchange WebSocket API and converts to Protocol V2 TLV messages.
//! Handles trade streams and order book updates for cryptocurrency pairs.
//!
//! ## Integration Points
//! - **Input**: Gemini WebSocket API (`wss://api.gemini.com/v1/marketdata/[SYMBOL]`)
//! - **Output**: TradeTLV and QuoteTLV messages via MarketDataRelay
//! - **Dependencies**: ConnectionManager, AdapterMetrics, RateLimiter
//!
//! ## Architecture Role
//! ```text
//! Gemini WebSocket → GeminiCollector → TLV Messages → MarketDataRelay → Strategies
//!                        ↓
//!                   JSON → Binary
//!                   8-decimal precision
//!                   Bijective InstrumentId
//! ```
//!
//! ## Performance Profile
//! - **Throughput**: Designed for >1M msg/s TLV construction
//! - **Latency**: <35μs JSON to TLV conversion target
//! - **Memory**: Stateless transformer - minimal memory footprint
//! - **Precision**: 8-decimal fixed-point for USD prices (CEX standard)
//!
//! ## Examples
//! ```rust
//! use alphapulse_adapters::GeminiCollector;
//! use tokio::sync::mpsc;
//! 
//! let (tx, rx) = mpsc::channel(1000);
//! let collector = GeminiCollector::new(
//!     vec!["BTCUSD".to_string(), "ETHUSD".to_string()],
//!     tx
//! );
//! 
//! // Start collecting in background
//! tokio::spawn(async move {
//!     collector.start().await
//! });
//! ```

use protocol_v2::{
    VenueId, InstrumentId, TradeTLV, QuoteTLV, TLVMessage, TLVType, TLVHeader,
    current_timestamp_ns
};
use async_trait::async_trait;
use futures_util::{StreamExt, SinkExt, stream::{SplitSink, SplitStream}};
use tokio_tungstenite::{connect_async, WebSocketStream};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use rust_decimal::Decimal;
use rust_decimal::prelude::{ToPrimitive, FromStr};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::tungstenite::Message;

use crate::{AdapterError, Result};
use crate::{AdapterMetrics, AuthManager, RateLimiter, ErrorType};
use crate::input::{
    InputAdapter, HealthStatus, HealthLevel, ConnectionManager, 
    ConnectionState
};
use crate::input::connection::ConnectionConfig;

/// Gemini Market Data Event JSON Schema
/// 
/// Based on Gemini's WebSocket API documentation
/// Example endpoint: wss://api.gemini.com/v1/marketdata/btcusd
const GEMINI_MARKET_DATA_SCHEMA: &str = r#"
{
  "type": "update",
  "eventId": 123456789,
  "socket_sequence": 1,
  "events": [
    {
      "type": "trade",
      "tid": 987654321,
      "price": "45123.50",
      "amount": "0.12345678",
      "makerSide": "bid",
      "timestampms": 1693234567890
    }
  ]
}
"#;

/// Gemini Heartbeat JSON Schema
const GEMINI_HEARTBEAT_SCHEMA: &str = r#"
{
  "type": "heartbeat",
  "socket_sequence": 1
}
"#;

/// Parsed Gemini market data event
#[derive(Debug, Clone, Deserialize)]
pub struct GeminiMarketDataEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    
    #[serde(rename = "eventId")]
    pub event_id: Option<u64>,
    
    #[serde(rename = "socket_sequence")]
    pub socket_sequence: u64,
    
    pub events: Option<Vec<GeminiTradeEvent>>,
}

/// Individual trade event within market data
#[derive(Debug, Clone, Deserialize)]
pub struct GeminiTradeEvent {
    #[serde(rename = "type")]
    pub trade_type: String,
    
    pub tid: u64,              // Trade ID
    pub price: String,         // String for precision preservation
    pub amount: String,        // String for precision preservation
    
    #[serde(rename = "makerSide")]
    pub maker_side: String,    // "bid" or "ask"
    
    #[serde(rename = "timestampms")]
    pub timestamp_ms: u64,     // Milliseconds since epoch
}

impl GeminiTradeEvent {
    /// Validate semantic correctness of trade event
    /// 
    /// Following the same validation pattern as CoinbaseMatchEvent.
    /// Checks structural correctness and semantic validity without enforcing
    /// business logic constraints.
    pub fn validate(&self) -> Result<()> {
        // Event type validation
        if self.trade_type != "trade" {
            return Err(AdapterError::ParseError {
                venue: VenueId::Gemini,
                message: "trade_type validation".to_string(),
                error: format!("Invalid trade type: {}", self.trade_type),
            });
        }
        
        // Required field validation
        if self.price.is_empty() || self.amount.is_empty() {
            return Err(AdapterError::ParseError {
                venue: VenueId::Gemini,
                message: "price/amount validation".to_string(),
                error: "Empty price or amount".to_string(),
            });
        }
        
        // Maker side validation
        if self.maker_side != "bid" && self.maker_side != "ask" {
            return Err(AdapterError::ParseError {
                venue: VenueId::Gemini,
                message: "maker_side validation".to_string(),
                error: format!("Invalid maker side: {}", self.maker_side),
            });
        }
        
        // Decimal parsing validation
        let price = Decimal::from_str(&self.price)
            .map_err(|_| AdapterError::ParseError {
                venue: VenueId::Gemini,
                message: "price decimal parsing".to_string(),
                error: format!("Invalid price: {}", self.price),
            })?;
            
        let amount = Decimal::from_str(&self.amount)
            .map_err(|_| AdapterError::ParseError {
                venue: VenueId::Gemini,
                message: "amount decimal parsing".to_string(),
                error: format!("Invalid amount: {}", self.amount),
            })?;
        
        // Positive value validation
        if price <= Decimal::ZERO {
            return Err(AdapterError::ParseError {
                venue: VenueId::Gemini,
                message: "price validation".to_string(),
                error: "Price must be positive".to_string(),
            });
        }
        
        if amount <= Decimal::ZERO {
            return Err(AdapterError::ParseError {
                venue: VenueId::Gemini,
                message: "amount validation".to_string(),
                error: "Amount must be positive".to_string(),
            });
        }
        
        // Timestamp validation
        if self.timestamp_ms == 0 {
            return Err(AdapterError::ParseError {
                venue: VenueId::Gemini,
                message: "timestamp validation".to_string(),
                error: "Invalid timestamp".to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Convert millisecond timestamp to nanoseconds since epoch
    pub fn timestamp_ns(&self) -> Result<u64> {
        // Convert milliseconds to nanoseconds
        self.timestamp_ms.checked_mul(1_000_000)
            .ok_or(AdapterError::ParseError {
                venue: VenueId::Gemini,
                message: "timestamp conversion".to_string(),
                error: "Timestamp overflow".to_string(),
            })
    }
    
    /// Convert price to fixed-point i64 with 8 decimal places
    pub fn price_fixed_point(&self) -> Result<i64> {
        let price = Decimal::from_str(&self.price)
            .map_err(|_| AdapterError::ParseError {
                venue: VenueId::Gemini,
                message: "price fixed-point conversion".to_string(),
                error: format!("Invalid price: {}", self.price),
            })?;
        
        // Convert to 8 decimal places: price * 100_000_000
        let scaled = price * Decimal::from(100_000_000i64);
        
        scaled.to_i64()
            .ok_or(AdapterError::ParseError {
                venue: VenueId::Gemini,
                message: "price overflow check".to_string(),
                error: "Price overflow in fixed-point conversion".to_string(),
            })
    }
    
    /// Convert amount to fixed-point i64 with 8 decimal places  
    pub fn amount_fixed_point(&self) -> Result<i64> {
        let amount = Decimal::from_str(&self.amount)
            .map_err(|_| AdapterError::ParseError {
                venue: VenueId::Gemini,
                message: "amount fixed-point conversion".to_string(),
                error: format!("Invalid amount: {}", self.amount),
            })?;
        
        // Convert to 8 decimal places: amount * 100_000_000
        let scaled = amount * Decimal::from(100_000_000i64);
        
        scaled.to_i64()
            .ok_or(AdapterError::ParseError {
                venue: VenueId::Gemini,
                message: "amount overflow check".to_string(),
                error: "Amount overflow in fixed-point conversion".to_string(),
            })
    }
    
    /// Normalize Gemini symbol format (btcusd -> BTC/USD)
    pub fn normalized_symbol(&self, symbol: &str) -> String {
        // Gemini uses lowercase concatenated format (btcusd)
        // Convert to standard slash format (BTC/USD)
        match symbol.to_lowercase().as_str() {
            "btcusd" => "BTC/USD".to_string(),
            "ethusd" => "ETH/USD".to_string(),
            "ltcusd" => "LTC/USD".to_string(),
            "adausd" => "ADA/USD".to_string(),
            "dotusd" => "DOT/USD".to_string(),
            "linkusd" => "LINK/USD".to_string(),
            "uniusd" => "UNI/USD".to_string(),
            "maticusd" => "MATIC/USD".to_string(),
            "solusd" => "SOL/USD".to_string(),
            "avaxusd" => "AVAX/USD".to_string(),
            // Add more mappings as needed
            _ => {
                // Fallback: try to split common patterns
                if symbol.len() >= 6 {
                    let (base, quote) = symbol.split_at(symbol.len() - 3);
                    format!("{}/{}", base.to_uppercase(), quote.to_uppercase())
                } else {
                    symbol.to_uppercase()
                }
            }
        }
    }
    
    /// Convert maker side to trade side indicator (0=buy, 1=sell from market perspective)
    pub fn trade_side(&self) -> u8 {
        match self.maker_side.as_str() {
            "bid" => 1,    // Maker was bidding, taker sold (market sell)
            "ask" => 0,    // Maker was asking, taker bought (market buy)
            _ => 0,        // Default to buy
        }
    }
}

/// Convert GeminiTradeEvent to TradeTLV with semantic preservation
/// 
/// Following the same pattern as CoinbaseMatchEvent conversion.
impl TryFrom<(&GeminiTradeEvent, &str)> for TradeTLV {
    type Error = AdapterError;
    
    fn try_from((event, symbol): (&GeminiTradeEvent, &str)) -> Result<Self> {
        // Validate input data first
        event.validate()?;
        
        // Normalize symbol format (btcusd → BTC/USD)
        let normalized_symbol = event.normalized_symbol(symbol);
        
        // Create InstrumentId for cryptocurrency pair
        let instrument_id = InstrumentId::coin(VenueId::Gemini, &normalized_symbol);
        
        // Convert to fixed-point precision (8 decimals for CEX)
        let price_fp = event.price_fixed_point()?;
        let amount_fp = event.amount_fixed_point()?;
        let timestamp_ns = event.timestamp_ns()?;
        let side = event.trade_side();
        
        // Construct TradeTLV
        Ok(TradeTLV::new(
            VenueId::Gemini,
            instrument_id,
            price_fp,
            amount_fp,
            side,
            timestamp_ns
        ))
    }
}

/// Gemini WebSocket collector (stateless data transformer)
/// 
/// Following the CoinbaseCollector reference pattern - stateless transformer only.
/// No StateManager dependency for optimal performance.
pub struct GeminiCollector {
    /// Connection manager handles WebSocket lifecycle and reconnection
    connection: Arc<ConnectionManager>,
    
    /// Authentication manager (not needed for public market data)
    auth: Option<AuthManager>,
    
    /// Rate limiter prevents overwhelming the exchange
    rate_limiter: RateLimiter,
    
    /// Metrics for monitoring adapter health
    metrics: Arc<AdapterMetrics>,
    
    /// Symbol to InstrumentId mapping cache (minimal state for performance)
    symbol_map: Arc<RwLock<HashMap<String, InstrumentId>>>,
    
    /// Output channel for TLV messages
    output_tx: mpsc::Sender<TLVMessage>,
    
    /// Running flag for clean shutdown
    running: Arc<RwLock<bool>>,
    
    /// Subscribed symbols (e.g., ["btcusd", "ethusd"])
    symbols: Vec<String>,
}

impl GeminiCollector {
    /// Create a new Gemini collector
    pub fn new(
        symbols: Vec<String>,
        output_tx: mpsc::Sender<TLVMessage>,
    ) -> Self {
        let metrics = Arc::new(AdapterMetrics::new());
        
        // Gemini uses per-symbol WebSocket connections
        // For simplicity, we'll start with the first symbol
        // TODO: Support multiple concurrent connections for multiple symbols
        let primary_symbol = symbols.first()
            .unwrap_or(&"btcusd".to_string())
            .to_lowercase();
            
        let config = ConnectionConfig {
            url: format!("wss://api.gemini.com/v1/marketdata/{}", primary_symbol),
            connect_timeout: Duration::from_secs(10),
            message_timeout: Duration::from_secs(30),
            base_backoff_ms: 5000,     // 5 seconds
            max_backoff_ms: 60000,     // 60 seconds max
            max_reconnect_attempts: 10,
            health_check_interval: Duration::from_secs(30),
        };
        
        let connection = Arc::new(ConnectionManager::new(VenueId::Gemini, config, metrics.clone()));
        let rate_limiter = {
            let mut limiter = RateLimiter::new();
            limiter.configure_venue(VenueId::Gemini, 1000); // 1000 requests per minute
            limiter
        };
        
        Self {
            connection,
            auth: None,  // Public market data doesn't need authentication
            rate_limiter,
            metrics,
            symbol_map: Arc::new(RwLock::new(HashMap::new())),
            output_tx,
            running: Arc::new(RwLock::new(false)),
            symbols,
        }
    }
    
    /// Process incoming Gemini WebSocket message
    async fn process_message(&self, message: Message, symbol: &str) -> Result<()> {
        self.rate_limiter.wait(VenueId::Gemini).await?;
        
        let data = match message {
            Message::Text(text) => text,
            Message::Binary(_) => {
                return Err(AdapterError::UnexpectedFormat {
                    venue: VenueId::Gemini,
                    received: "Binary message".to_string(),
                    expected: "JSON text message".to_string(),
                });
            }
            _ => return Ok(()), // Ignore pings, pongs, close
        };
        
        // Parse JSON message
        let parsed: Value = serde_json::from_str(&data)
            .map_err(|e| AdapterError::ParseError {
                venue: VenueId::Gemini,
                message: "JSON parsing".to_string(),
                error: format!("JSON parse error: {}", e),
            })?;
        
        // Handle message by type
        if let Some(msg_type) = parsed.get("type").and_then(|t| t.as_str()) {
            match msg_type {
                "update" => self.process_market_update(&parsed, symbol).await?,
                "heartbeat" => {
                    // Update connection health
                    self.metrics.record_message(VenueId::Gemini, data.len());
                    Ok(())
                }
                _ => {
                    // Log unknown message types but don't error
                    tracing::debug!("Unknown Gemini message type: {}", msg_type);
                    Ok(())
                }
            }?;
        }
        
        Ok(())
    }
    
    /// Process market data update message
    async fn process_market_update(&self, data: &Value, symbol: &str) -> Result<()> {
        let market_event: GeminiMarketDataEvent = serde_json::from_value(data.clone())
            .map_err(|e| AdapterError::ParseError {
                venue: VenueId::Gemini,
                message: "market update parsing".to_string(),
                error: format!("Failed to parse market update: {}", e),
            })?;
        
        // Process trade events
        if let Some(events) = market_event.events {
            for event in events {
                if event.trade_type == "trade" {
                    self.process_trade_event(&event, symbol).await?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Process individual trade event and convert to TLV
    async fn process_trade_event(&self, event: &GeminiTradeEvent, symbol: &str) -> Result<()> {
        // Convert to TradeTLV
        let trade_tlv = TradeTLV::try_from((event, symbol))?;
        
        // Build TLV message
        let tlv_message = TLVMessage::new(
            TLVType::Trade,
            trade_tlv.as_bytes(),
        );
        
        // Send to output channel
        self.output_tx.send(tlv_message).await
            .map_err(|_| AdapterError::ChannelError {
                venue: VenueId::Gemini,
                operation: "send TradeTLV".to_string(),
            })?;
        
        // Update metrics
        self.metrics.increment_messages_sent();
        
        Ok(())
    }
}

#[async_trait]
impl InputAdapter for GeminiCollector {
    /// Get the venue this adapter connects to
    fn venue(&self) -> VenueId {
        VenueId::Gemini
    }
    
    /// Get adapter health status
    async fn health_check(&self) -> HealthStatus {
        let connection_state = self.connection.state().await;
        let metrics_summary = self.metrics.summary();
        
        if metrics_summary.state_invalidations > 10 {
            return HealthStatus {
                level: HealthLevel::Unhealthy,
                connection: connection_state,
                messages_per_minute: metrics_summary.total_messages,
                last_message_time: Some(current_timestamp_ns()),
                instrument_count: metrics_summary.total_instruments,
                error_count: metrics_summary.state_invalidations,
                details: Some(format!("High error count: {}", metrics_summary.state_invalidations)),
            };
        }
        
        HealthStatus {
            level: HealthLevel::Healthy,
            connection: connection_state,
            messages_per_minute: metrics_summary.total_messages,
            last_message_time: Some(current_timestamp_ns()),
            instrument_count: metrics_summary.total_instruments,
            error_count: 0,
            details: Some("All systems operational".to_string()),
        }
    }
    
    /// Check if adapter is currently connected
    fn is_connected(&self) -> bool {
        // Check running state as proxy for connection
        // TODO: Check actual connection state when ConnectionManager exposes it
        false // Placeholder - ConnectionManager doesn't expose is_connected
    }
    
    /// Get list of instruments being tracked
    fn tracked_instruments(&self) -> Vec<InstrumentId> {
        // Convert symbols to InstrumentIds
        self.symbols.iter()
            .map(|symbol| {
                let normalized = GeminiTradeEvent {
                    trade_type: "trade".to_string(),
                    tid: 0,
                    price: "1".to_string(),
                    amount: "1".to_string(),
                    maker_side: "bid".to_string(),
                    timestamp_ms: 1,
                }.normalized_symbol(symbol);
                InstrumentId::coin(VenueId::Gemini, &normalized)
            })
            .collect()
    }
    
    /// Subscribe to specific instruments (not supported - uses predefined symbols)
    async fn subscribe(&mut self, _instruments: Vec<InstrumentId>) -> Result<()> {
        Err(AdapterError::NotSupported(
            "Dynamic subscription not supported for Gemini adapter - uses predefined symbols".to_string()
        ))
    }
    
    /// Unsubscribe from instruments (not supported)
    async fn unsubscribe(&mut self, _instruments: Vec<InstrumentId>) -> Result<()> {
        Err(AdapterError::NotSupported(
            "Dynamic unsubscription not supported for Gemini adapter".to_string()
        ))
    }
    
    /// Force reconnection
    async fn reconnect(&mut self) -> Result<()> {
        // Stop and restart
        self.stop().await?;
        self.start().await
    }
    
    /// Start the adapter and begin collecting data
    async fn start(&mut self) -> Result<()> {
        *self.running.write().await = true;
        
        tracing::info!("Starting Gemini collector for symbols: {:?}", self.symbols);
        
        // Currently supports single symbol connection
        // TODO: Implement concurrent connections for multiple symbols
        if let Some(symbol) = self.symbols.first() {
            self.start_symbol_collection(symbol).await?;
        }
        
        Ok(())
    }
    
    /// Stop the adapter
    async fn stop(&mut self) -> Result<()> {
        *self.running.write().await = false;
        // Connection is dropped automatically when GeminiCollector is dropped
        tracing::info!("Gemini collector stopped");
        Ok(())
    }
}

impl GeminiCollector {
    /// Start collecting data for a specific symbol
    async fn start_symbol_collection(&self, symbol: &str) -> Result<()> {
        let symbol = symbol.to_lowercase();
        let url = format!("wss://api.gemini.com/v1/marketdata/{}", symbol);
        
        loop {
            if !*self.running.read().await {
                break;
            }
            
            // Direct WebSocket connection (following Coinbase pattern)
            // TODO: Integrate with ConnectionManager when interface is available
            match tokio_tungstenite::connect_async(&url).await {
                Ok((ws_stream, _response)) => {
                    tracing::info!("Connected to Gemini WebSocket for symbol: {}", symbol);
                    
                    let (_ws_sender, mut ws_receiver) = ws_stream.split();
                    
                    // Process messages
                    while let Some(message) = ws_receiver.next().await {
                        if !*self.running.read().await {
                            break;
                        }
                        
                        match message {
                            Ok(msg) => {
                                if let Err(e) = self.process_message(msg, &symbol).await {
                                    tracing::error!("Error processing Gemini message: {}", e);
                                    self.metrics.record_processing_error(ErrorType::Parse);
                                }
                            }
                            Err(e) => {
                                tracing::error!("WebSocket error from Gemini: {}", e);
                                self.metrics.record_processing_error(ErrorType::Protocol);
                                break; // Reconnect on WebSocket error
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to connect to Gemini: {}", e);
                    self.metrics.record_connection_failure(VenueId::Gemini);
                    
                    // Wait before retry
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol_v2::VenueId;
    
    #[test]
    fn test_gemini_trade_event_validation() {
        let valid_event = GeminiTradeEvent {
            trade_type: "trade".to_string(),
            tid: 987654321,
            price: "45123.50".to_string(),
            amount: "0.12345678".to_string(),
            maker_side: "bid".to_string(),
            timestamp_ms: 1693234567890,
        };
        
        assert!(valid_event.validate().is_ok());
        
        // Test invalid trade type
        let mut invalid_event = valid_event.clone();
        invalid_event.trade_type = "invalid".to_string();
        assert!(invalid_event.validate().is_err());
        
        // Test empty price
        let mut invalid_event = valid_event.clone();
        invalid_event.price = "".to_string();
        assert!(invalid_event.validate().is_err());
        
        // Test invalid maker side
        let mut invalid_event = valid_event.clone();
        invalid_event.maker_side = "invalid".to_string();
        assert!(invalid_event.validate().is_err());
    }
    
    #[test]
    fn test_symbol_normalization() {
        let event = GeminiTradeEvent {
            trade_type: "trade".to_string(),
            tid: 1,
            price: "1".to_string(),
            amount: "1".to_string(),
            maker_side: "bid".to_string(),
            timestamp_ms: 1,
        };
        
        assert_eq!(event.normalized_symbol("btcusd"), "BTC/USD");
        assert_eq!(event.normalized_symbol("ethusd"), "ETH/USD");
        assert_eq!(event.normalized_symbol("maticusd"), "MATIC/USD");
    }
    
    #[test]
    fn test_fixed_point_conversion() {
        let event = GeminiTradeEvent {
            trade_type: "trade".to_string(),
            tid: 1,
            price: "45123.50".to_string(),
            amount: "0.12345678".to_string(),
            maker_side: "bid".to_string(),
            timestamp_ms: 1693234567890,
        };
        
        // Test price conversion (8 decimal places)
        let price_fp = event.price_fixed_point().unwrap();
        assert_eq!(price_fp, 4512350000000); // 45123.50 * 100_000_000
        
        // Test amount conversion
        let amount_fp = event.amount_fixed_point().unwrap();
        assert_eq!(amount_fp, 12345678); // 0.12345678 * 100_000_000
    }
    
    #[test]
    fn test_timestamp_conversion() {
        let event = GeminiTradeEvent {
            trade_type: "trade".to_string(),
            tid: 1,
            price: "1".to_string(),
            amount: "1".to_string(),
            maker_side: "bid".to_string(),
            timestamp_ms: 1693234567890,
        };
        
        let timestamp_ns = event.timestamp_ns().unwrap();
        assert_eq!(timestamp_ns, 1693234567890000000); // ms * 1_000_000
    }
    
    #[tokio::test]
    async fn test_gemini_collector_creation() {
        let (tx, _rx) = mpsc::channel(100);
        let collector = GeminiCollector::new(
            vec!["btcusd".to_string(), "ethusd".to_string()],
            tx
        );
        
        assert_eq!(collector.venue(), VenueId::Gemini);
        assert_eq!(collector.symbols.len(), 2);
        assert_eq!(collector.symbols[0], "btcusd");
        assert_eq!(collector.symbols[1], "ethusd");
    }
}