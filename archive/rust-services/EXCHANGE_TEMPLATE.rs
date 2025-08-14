// {EXCHANGE_NAME} WebSocket collector with L2 orderbook support
// 
// TEMPLATE INSTRUCTIONS:
// 1. Replace {EXCHANGE_NAME} with actual exchange name (e.g., "Bitfinex")
// 2. Replace {exchange} with lowercase exchange name (e.g., "bitfinex")
// 3. Replace {EXCHANGE_WEBSOCKET_URL} with actual WebSocket URL
// 4. Update symbol conversion logic for exchange format
// 5. Implement exchange-specific message parsing
// 6. Test with live data and validate compression ratios

use alphapulse_common::{
    Result, Trade, MetricsCollector,
    OrderBookUpdate, OrderBookLevel, OrderBookTracker,
    OrderBookSnapshot, OrderBookDelta,
    shared_memory::{OrderBookDeltaWriter, SharedOrderBookDelta}
};
use crate::collector_trait::MarketDataCollector;
use std::collections::HashMap;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{info, warn, error, debug};
use url::Url;

// TODO: Define exchange-specific trade message structure
#[derive(Debug, Clone, Deserialize)]
pub struct {EXCHANGE_NAME}TradeMessage {
    // TODO: Map exchange fields using serde rename
    #[serde(rename = "TYPE_FIELD_NAME")]
    pub message_type: String,
    
    #[serde(rename = "SYMBOL_FIELD_NAME")]
    pub symbol: String,
    
    #[serde(rename = "PRICE_FIELD_NAME")]
    pub price: String, // Usually string in JSON
    
    #[serde(rename = "VOLUME_FIELD_NAME")]
    pub volume: String,
    
    #[serde(rename = "TIMESTAMP_FIELD_NAME")]
    pub timestamp: f64, // Or u64, depending on exchange format
    
    #[serde(rename = "SIDE_FIELD_NAME")]
    pub side: String,
    
    // TODO: Add other exchange-specific fields
    // #[serde(rename = "TRADE_ID_FIELD")]
    // pub trade_id: Option<String>,
}

impl From<{EXCHANGE_NAME}TradeMessage> for Trade {
    fn from(msg: {EXCHANGE_NAME}TradeMessage) -> Self {
        Trade {
            // TODO: Convert timestamp to seconds (Unix epoch)
            timestamp: msg.timestamp, // / 1000.0 if milliseconds
            
            // Convert to standard symbol format
            symbol: {EXCHANGE_NAME}Collector::convert_symbol_from_{exchange}(&msg.symbol),
            
            exchange: "{exchange}".to_string(),
            
            // Parse price and volume from strings
            price: msg.price.parse().unwrap_or(0.0),
            volume: msg.volume.parse().unwrap_or(0.0),
            
            // TODO: Convert side format if needed
            side: Some(msg.side), // Or map to "buy"/"sell"
            
            // TODO: Extract trade ID if available
            trade_id: None, // Some(msg.trade_id)
        }
    }
}

pub struct {EXCHANGE_NAME}Collector {
    symbols: Vec<String>,
    ws_url: String,
    healthy: Arc<AtomicBool>,
    metrics: Arc<MetricsCollector>,
    orderbook_tx: Option<mpsc::Sender<OrderBookUpdate>>,
    delta_tx: Option<mpsc::Sender<OrderBookDelta>>,
    orderbooks: Arc<tokio::sync::RwLock<HashMap<String, OrderBookUpdate>>>,
    orderbook_tracker: OrderBookTracker,
    delta_writer: Option<Arc<tokio::sync::Mutex<OrderBookDeltaWriter>>>,
}

impl {EXCHANGE_NAME}Collector {
    pub fn new(symbols: Vec<String>) -> Self {
        // Convert symbols to exchange format
        let exchange_symbols: Vec<String> = symbols
            .iter()
            .map(|s| Self::convert_symbol_to_{exchange}(s))
            .collect();
            
        Self {
            symbols: exchange_symbols,
            ws_url: "{EXCHANGE_WEBSOCKET_URL}".to_string(), // TODO: Set actual URL
            healthy: Arc<new(AtomicBool::new(false)),
            metrics: Arc::new(MetricsCollector::new()),
            orderbook_tx: None,
            delta_tx: None,
            orderbooks: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            orderbook_tracker: OrderBookTracker::new(50), // Track top 50 levels
            delta_writer: None,
        }
    }
    
    fn convert_symbol_to_{exchange}(symbol: &str) -> String {
        // TODO: Implement exchange-specific symbol conversion
        // Examples:
        // - Binance: "BTC/USDT" -> "btcusdt" (lowercase, no separator)
        // - Kraken: "BTC/USD" -> "XBT/USD" (special BTC mapping)
        // - Coinbase: "BTC/USD" -> "BTC-USD" (dash separator)
        
        match symbol {
            "BTC-USD" | "BTC/USD" => "TODO_EXCHANGE_BTC_FORMAT".to_string(),
            "ETH-USD" | "ETH/USD" => "TODO_EXCHANGE_ETH_FORMAT".to_string(),
            s => {
                // Generic conversion logic
                s.replace("/", "SEPARATOR").to_lowercase() // Adjust as needed
            }
        }
    }
    
    fn convert_symbol_from_{exchange}(exchange_symbol: &str) -> String {
        // TODO: Convert exchange format back to standard format
        match exchange_symbol {
            "TODO_EXCHANGE_BTC_FORMAT" => "BTC/USD".to_string(),
            "TODO_EXCHANGE_ETH_FORMAT" => "ETH/USD".to_string(),
            s => {
                // TODO: Generic reverse conversion
                s.to_uppercase().replace("SEPARATOR", "/")
            }
        }
    }
    
    pub fn with_orderbook_sender(mut self, tx: mpsc::Sender<OrderBookUpdate>) -> Self {
        self.orderbook_tx = Some(tx);
        self
    }
    
    pub fn with_delta_sender(mut self, tx: mpsc::Sender<OrderBookDelta>) -> Self {
        self.delta_tx = Some(tx);
        self
    }
    
    pub fn with_shared_memory_writer(mut self) -> Result<Self> {
        // Create shared memory writer for orderbook deltas
        let writer = OrderBookDeltaWriter::create(
            "/tmp/alphapulse_shm/{exchange}_orderbook_deltas", 
            10000 // 10k capacity
        )?;
        self.delta_writer = Some(Arc::new(tokio::sync::Mutex::new(writer)));
        Ok(self)
    }
    
    fn convert_to_shared_delta(&self, delta: &OrderBookDelta) -> SharedOrderBookDelta {
        let timestamp_ns = (delta.timestamp * 1_000_000_000.0) as u64;
        let mut shared_delta = SharedOrderBookDelta::new(
            timestamp_ns,
            &delta.symbol,
            &delta.exchange,
            delta.version,
            delta.prev_version
        );
        
        // Add bid changes
        for change in &delta.bid_changes {
            if !shared_delta.add_change(change.price, change.volume, false, 0) {
                warn!("Delta buffer full, some bid changes dropped");
                break;
            }
        }
        
        // Add ask changes
        for change in &delta.ask_changes {
            if !shared_delta.add_change(change.price, change.volume, true, 0) {
                warn!("Delta buffer full, some ask changes dropped");
                break;
            }
        }
        
        shared_delta
    }
    
    async fn handle_message(&self, msg: Message, tx: &mpsc::Sender<Trade>) -> Result<()> {
        match msg {
            Message::Text(text) => {
                debug!("Received {exchange} message: {}", text);
                
                // Try parsing as trade message
                if let Ok(trade_msg) = serde_json::from_str::<{EXCHANGE_NAME}TradeMessage>(&text) {
                    // TODO: Check message type field to identify trades
                    if trade_msg.message_type == "TRADE_MESSAGE_TYPE" {
                        let trade = Trade::from(trade_msg.clone());
                        
                        // Send to processing channel
                        if let Err(e) = tx.send(trade).await {
                            warn!("Failed to send trade to channel: {}", e);
                            return Ok(()); // Don't crash on channel errors
                        }
                        
                        // Record metrics
                        self.metrics.record_trade_processed("{exchange}", &trade_msg.symbol);
                        self.metrics.record_websocket_message("{exchange}", "trade");
                    }
                }
                // Try parsing as orderbook message
                else if text.contains("ORDERBOOK_MESSAGE_IDENTIFIER") {
                    if let Ok(orderbook_msg) = serde_json::from_str::<serde_json::Value>(&text) {
                        if self.orderbook_tx.is_some() || self.delta_tx.is_some() || self.delta_writer.is_some() {
                            self.handle_{exchange}_orderbook(orderbook_msg).await?;
                            self.metrics.record_websocket_message("{exchange}", "orderbook_update");
                        }
                    }
                }
                else {
                    // Handle subscription confirmations and other messages
                    if text.contains("SUBSCRIPTION_SUCCESS_INDICATOR") {
                        info!("{EXCHANGE_NAME} subscription confirmed");
                    } else if text.contains("HEARTBEAT_INDICATOR") {
                        debug!("Received {exchange} heartbeat");
                    }
                }
            }
            Message::Ping(_) => {
                debug!("Received {exchange} ping, sending pong");
                // WebSocket library handles pong automatically
            }
            Message::Pong(_) => {
                debug!("Received {exchange} pong");
            }
            Message::Close(_) => {
                warn!("{EXCHANGE_NAME} WebSocket closed");
                self.healthy.store(false, Ordering::Relaxed);
                self.metrics.record_websocket_connection_status("{exchange}", false);
            }
            Message::Binary(_) => {
                warn!("Received unexpected binary message from {exchange}");
            }
            Message::Frame(_) => {
                debug!("Received frame message from {exchange}");
            }
        }
        
        Ok(())
    }
    
    async fn handle_{exchange}_orderbook(&self, msg: serde_json::Value) -> Result<()> {
        // TODO: Parse exchange-specific orderbook message format
        // Each exchange has different JSON structure for orderbook data
        
        if let Some(symbol_data) = msg.get("SYMBOL_FIELD").and_then(|s| s.as_str()) {
            let timestamp = chrono::Utc::now().timestamp() as f64;
            
            // Parse bids and asks
            let mut bids = Vec::new();
            let mut asks = Vec::new();
            
            // TODO: Parse bids according to exchange format
            if let Some(bid_array) = msg.get("BIDS_FIELD").and_then(|b| b.as_array()) {
                for bid in bid_array.iter() {
                    // TODO: Exchange-specific bid parsing
                    // Common patterns:
                    // - Array format: ["price", "size"]
                    // - Object format: {"price": "x", "size": "y"}
                    
                    if let Some(bid_data) = bid.as_array() {
                        if bid_data.len() >= 2 {
                            let price = bid_data[0].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                            let size = bid_data[1].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                            if size > 0.0 { // Only include non-zero volumes
                                bids.push([price, size]);
                            }
                        }
                    }
                }
            }
            
            // TODO: Parse asks according to exchange format
            if let Some(ask_array) = msg.get("ASKS_FIELD").and_then(|a| a.as_array()) {
                for ask in ask_array.iter() {
                    // TODO: Same parsing logic as bids
                    if let Some(ask_data) = ask.as_array() {
                        if ask_data.len() >= 2 {
                            let price = ask_data[0].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                            let size = ask_data[1].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                            if size > 0.0 {
                                asks.push([price, size]);
                            }
                        }
                    }
                }
            }
            
            // Convert to standard symbol format
            let standard_symbol = Self::convert_symbol_from_{exchange}(symbol_data);
            
            // Create OrderBookSnapshot for delta tracking
            let snapshot = OrderBookSnapshot {
                symbol: standard_symbol.clone(),
                exchange: "{exchange}".to_string(),
                version: chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default() as u64,
                timestamp,
                bids: bids.clone(),
                asks: asks.clone(),
            };
            
            // Update OrderBookTracker with snapshot
            self.orderbook_tracker.update_snapshot(&standard_symbol, "{exchange}", snapshot.clone()).await;
            
            // Compute delta if this is an update (not first snapshot)
            if let Some(delta) = self.orderbook_tracker.compute_delta(&snapshot, &standard_symbol).await {
                // Send delta update via channel
                if let Some(tx) = &self.delta_tx {
                    if let Err(e) = tx.send(delta.clone()).await {
                        warn!("Failed to send {exchange} orderbook delta: {}", e);
                    }
                }
                
                // Write delta to shared memory for ultra-low latency access
                if let Some(writer) = &self.delta_writer {
                    let shared_delta = self.convert_to_shared_delta(&delta);
                    let mut writer_guard = writer.lock().await;
                    if let Err(e) = writer_guard.write_delta(&shared_delta) {
                        warn!("Failed to write {exchange} delta to shared memory: {}", e);
                    } else {
                        info!(
                            "🚀 {EXCHANGE_NAME} delta written to shared memory for {}: {} bid changes, {} ask changes (vs {} full levels)", 
                            standard_symbol,
                            delta.bid_changes.len(),
                            delta.ask_changes.len(),
                            bids.len() + asks.len()
                        );
                    }
                }
            }
            
            // Create legacy OrderBookUpdate for backward compatibility
            let orderbook = OrderBookUpdate {
                symbol: standard_symbol.clone(),
                exchange: "{exchange}".to_string(),
                timestamp,
                bids,
                asks,
                sequence: None,
                update_type: Some("snapshot".to_string()),
            };
            
            // Store in local cache
            let mut orderbooks = self.orderbooks.write().await;
            orderbooks.insert(standard_symbol.clone(), orderbook.clone());
            
            // Send to channel if available (backward compatibility)
            if let Some(tx) = &self.orderbook_tx {
                if let Err(e) = tx.send(orderbook).await {
                    warn!("Failed to send {exchange} orderbook update: {}", e);
                }
            }
            
            info!("Processed orderbook for {exchange} symbol: {}", standard_symbol);
        }
        
        Ok(())
    }
}

#[async_trait]
impl MarketDataCollector for {EXCHANGE_NAME}Collector {
    async fn start(&self, tx: mpsc::Sender<Trade>) -> Result<()> {
        loop {
            match self.run_collector(&tx).await {
                Ok(_) => {
                    info!("{EXCHANGE_NAME} collector completed normally");
                    break;
                }
                Err(e) => {
                    error!("{EXCHANGE_NAME} collector error: {}", e);
                    self.healthy.store(false, Ordering::Relaxed);
                    self.metrics.record_websocket_connection_status("{exchange}", false);
                    self.metrics.record_websocket_reconnection("{exchange}");
                    
                    // Wait before reconnecting
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    info!("Attempting to reconnect to {exchange}...");
                }
            }
        }
        
        Ok(())
    }
    
    async fn stop(&self) -> Result<()> {
        info!("Stopping {exchange} collector");
        self.healthy.store(false, Ordering::Relaxed);
        Ok(())
    }
    
    fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }
    
    fn exchange_name(&self) -> &str {
        "{exchange}"
    }
    
    fn symbols(&self) -> &[String] {
        &self.symbols
    }
}

impl {EXCHANGE_NAME}Collector {
    async fn run_collector(&self, tx: &mpsc::Sender<Trade>) -> Result<()> {
        let url = Url::parse(&self.ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();
        
        // TODO: Subscribe to trade messages for all symbols
        // Each exchange has different subscription format
        for (id, symbol) in self.symbols.iter().enumerate() {
            let trade_subscribe_msg = json!({
                // TODO: Exchange-specific trade subscription format
                "method": "SUBSCRIBE",
                "params": [format!("{}@trade", symbol)], // Adjust format
                "id": id + 1
            });
            
            write.send(Message::Text(trade_subscribe_msg.to_string())).await?;
            info!("Subscribed to {exchange} trades for symbol: {}", symbol);
        }
        
        // Subscribe to orderbook streams if we have orderbook handlers
        if self.orderbook_tx.is_some() || self.delta_tx.is_some() || self.delta_writer.is_some() {
            for (id, symbol) in self.symbols.iter().enumerate() {
                let book_subscribe_msg = json!({
                    // TODO: Exchange-specific orderbook subscription format
                    "method": "SUBSCRIBE",
                    "params": [format!("{}@depth20@100ms", symbol)], // Adjust format
                    "id": id + 1000 // Offset IDs to avoid conflicts
                });
                
                write.send(Message::Text(book_subscribe_msg.to_string())).await?;
                info!("Subscribed to {exchange} orderbook for symbol: {}", symbol);
            }
        }
        
        self.healthy.store(true, Ordering::Relaxed);
        self.metrics.record_websocket_connection_status("{exchange}", true);
        
        info!("Connected to {EXCHANGE_NAME} for symbols: {:?}", self.symbols);
        
        // Process incoming messages
        while let Some(msg_result) = read.next().await {
            match msg_result {
                Ok(msg) => {
                    if let Err(e) = self.handle_message(msg, tx).await {
                        error!("Error handling {exchange} message: {}", e);
                    }
                }
                Err(e) => {
                    error!("WebSocket error from {exchange}: {}", e);
                    return Err(e.into());
                }
            }
        }
        
        self.healthy.store(false, Ordering::Relaxed);
        self.metrics.record_websocket_connection_status("{exchange}", false);
        
        Ok(())
    }
}

// TODO: Add unit tests
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_symbol_conversion() {
        assert_eq!(
            {EXCHANGE_NAME}Collector::convert_symbol_to_{exchange}("BTC-USD"),
            "TODO_EXPECTED_FORMAT"
        );
        assert_eq!(
            {EXCHANGE_NAME}Collector::convert_symbol_from_{exchange}("TODO_EXCHANGE_FORMAT"),
            "BTC/USD"
        );
    }
    
    #[tokio::test]
    async fn test_collector_creation() {
        let collector = {EXCHANGE_NAME}Collector::new(vec!["BTC-USD".to_string()]);
        assert_eq!(collector.exchange_name(), "{exchange}");
        assert!(!collector.is_healthy()); // Should start unhealthy
    }
}