// Binance.US WebSocket collector for USDT pairs with L2 orderbook support
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

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceTradeMessage {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "t")]
    pub trade_id: u64,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "q")]
    pub quantity: String,
    #[serde(rename = "b")]
    pub buyer_order_id: u64,
    #[serde(rename = "a")]
    pub seller_order_id: u64,
    #[serde(rename = "T")]
    pub trade_time: u64,
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,
    #[serde(rename = "M")]
    pub ignore: bool,
}

impl From<BinanceTradeMessage> for Trade {
    fn from(msg: BinanceTradeMessage) -> Self {
        Trade {
            timestamp: msg.trade_time as f64 / 1000.0, // Convert from ms to seconds
            price: msg.price.parse().unwrap_or(0.0),
            volume: msg.quantity.parse().unwrap_or(0.0),
            side: Some(if msg.is_buyer_maker { "sell".to_string() } else { "buy".to_string() }),
            trade_id: Some(msg.trade_id.to_string()),
            symbol: msg.symbol.clone(),
            exchange: "binance_us".to_string(),
        }
    }
}

pub struct BinanceUSCollector {
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

impl BinanceUSCollector {
    pub fn new(symbols: Vec<String>) -> Self {
        // Convert symbols to Binance format (e.g., "BTC/USDT" -> "btcusdt")
        let binance_symbols: Vec<String> = symbols
            .iter()
            .map(|s| Self::convert_symbol_to_binance(s))
            .collect();
            
        Self {
            symbols: binance_symbols,
            ws_url: "wss://stream.binance.us:9443/ws".to_string(),
            healthy: Arc::new(AtomicBool::new(false)),
            metrics: Arc::new(MetricsCollector::new()),
            orderbook_tx: None,
            delta_tx: None,
            orderbooks: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            orderbook_tracker: OrderBookTracker::new(50), // Track top 50 levels
            delta_writer: None,
        }
    }
    
    fn convert_symbol_to_binance(symbol: &str) -> String {
        // Convert to Binance format: lowercase, no separators
        symbol.replace("/", "").replace("-", "").to_lowercase()
    }
    
    fn convert_symbol_from_binance(binance_symbol: &str) -> String {
        // Convert common Binance symbols back to standard format
        let upper = binance_symbol.to_uppercase();
        match upper.as_str() {
            "BTCUSDT" => "BTC/USDT".to_string(),
            "ETHUSDT" => "ETH/USDT".to_string(),
            s => {
                // Try to parse other USDT pairs
                if s.ends_with("USDT") {
                    let base = &s[..s.len()-4];
                    format!("{}/USDT", base)
                } else {
                    s.to_string()
                }
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
            "/tmp/alphapulse_shm/binance_orderbook_deltas", 
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
                debug!("Received Binance.US message: {}", text);
                
                // Try to parse as trade message
                if let Ok(trade_msg) = serde_json::from_str::<BinanceTradeMessage>(&text) {
                    if trade_msg.event_type == "trade" {
                        let mut trade = Trade::from(trade_msg.clone());
                        
                        // Convert symbol back to standard format
                        trade.symbol = Self::convert_symbol_from_binance(&trade_msg.symbol);
                        let symbol_for_metrics = trade.symbol.clone();
                        
                        // Send to processing channel
                        if let Err(e) = tx.send(trade).await {
                            warn!("Failed to send trade to channel: {}", e);
                            return Ok(()); // Don't crash on channel errors
                        }
                        
                        // Record metrics
                        self.metrics.record_trade_processed("binance_us", &symbol_for_metrics);
                        self.metrics.record_websocket_message("binance_us", "trade");
                    }
                }
                // Try parsing as orderbook depth message
                else if text.contains("\"e\":\"depthUpdate\"") || text.contains("@depth") {
                    if let Ok(depth_msg) = serde_json::from_str::<serde_json::Value>(&text) {
                        if self.orderbook_tx.is_some() || self.delta_tx.is_some() || self.delta_writer.is_some() {
                            self.handle_binance_orderbook(depth_msg).await?;
                            self.metrics.record_websocket_message("binance_us", "orderbook_update");
                        }
                    }
                }
                else {
                    // Handle other message types
                    if text.contains("\"result\":null") && text.contains("\"id\":") {
                        info!("Binance.US subscription confirmed");
                    }
                }
            }
            Message::Ping(data) => {
                debug!("Received Binance.US ping, sending pong");
                // WebSocket library handles pong automatically
            }
            Message::Pong(_) => {
                debug!("Received Binance.US pong");
            }
            Message::Close(_) => {
                warn!("Binance.US WebSocket closed");
                self.healthy.store(false, Ordering::Relaxed);
                self.metrics.record_websocket_connection_status("binance_us", false);
            }
            Message::Binary(_) => {
                warn!("Received unexpected binary message from Binance.US");
            }
            Message::Frame(_) => {
                debug!("Received frame message from Binance.US");
            }
        }
        
        Ok(())
    }
}

#[async_trait]
impl MarketDataCollector for BinanceUSCollector {
    async fn start(&self, tx: mpsc::Sender<Trade>) -> Result<()> {
        loop {
            match self.run_collector(&tx).await {
                Ok(_) => {
                    info!("Binance.US collector completed normally");
                    break;
                }
                Err(e) => {
                    error!("Binance.US collector error: {}", e);
                    self.healthy.store(false, Ordering::Relaxed);
                    self.metrics.record_websocket_connection_status("binance_us", false);
                    self.metrics.record_websocket_reconnection("binance_us");
                    
                    // Wait before reconnecting
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    info!("Attempting to reconnect to Binance.US...");
                }
            }
        }
        
        Ok(())
    }
    
    async fn stop(&self) -> Result<()> {
        info!("Stopping Binance.US collector");
        self.healthy.store(false, Ordering::Relaxed);
        Ok(())
    }
    
    fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }
    
    fn exchange_name(&self) -> &str {
        "binance_us"
    }
    
    fn symbols(&self) -> &[String] {
        &self.symbols
    }
}

impl BinanceUSCollector {
    async fn run_collector(&self, tx: &mpsc::Sender<Trade>) -> Result<()> {
        let url = Url::parse(&self.ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();
        
        // Subscribe to individual trade streams for each symbol
        for (id, symbol) in self.symbols.iter().enumerate() {
            let subscribe_msg = json!({
                "method": "SUBSCRIBE",
                "params": [format!("{}@trade", symbol)],
                "id": id + 1
            });
            
            write.send(Message::Text(subscribe_msg.to_string())).await?;
            info!("Subscribed to Binance.US trades for symbol: {}", symbol);
        }
        
        // Subscribe to orderbook streams if we have orderbook handlers
        if self.orderbook_tx.is_some() || self.delta_tx.is_some() || self.delta_writer.is_some() {
            for (id, symbol) in self.symbols.iter().enumerate() {
                let book_subscribe_msg = json!({
                    "method": "SUBSCRIBE",
                    "params": [format!("{}@depth20@100ms", symbol)], // 20-level depth at 100ms updates
                    "id": id + 1000 // Offset IDs to avoid conflicts
                });
                
                write.send(Message::Text(book_subscribe_msg.to_string())).await?;
                info!("Subscribed to Binance.US orderbook for symbol: {}", symbol);
            }
        }
        
        self.healthy.store(true, Ordering::Relaxed);
        self.metrics.record_websocket_connection_status("binance_us", true);
        
        info!("Connected to Binance.US for symbols: {:?}", self.symbols);
        
        // Process incoming messages
        while let Some(msg_result) = read.next().await {
            match msg_result {
                Ok(msg) => {
                    if let Err(e) = self.handle_message(msg, tx).await {
                        error!("Error handling Binance.US message: {}", e);
                    }
                }
                Err(e) => {
                    error!("WebSocket error from Binance.US: {}", e);
                    return Err(e.into());
                }
            }
        }
        
        self.healthy.store(false, Ordering::Relaxed);
        self.metrics.record_websocket_connection_status("binance_us", false);
        
        Ok(())
    }
    
    async fn handle_binance_orderbook(&self, msg: serde_json::Value) -> Result<()> {
        // Parse Binance depth stream message format
        if let Some(stream) = msg.get("stream").and_then(|s| s.as_str()) {
            if let Some(data) = msg.get("data") {
                // Extract symbol from stream name (e.g., "btcusdt@depth20@100ms" -> "btcusdt")
                let symbol = stream.split('@').next().unwrap_or("");
                let standard_symbol = Self::convert_symbol_from_binance(symbol);
                
                let timestamp = chrono::Utc::now().timestamp() as f64;
                
                // Parse bids and asks
                let mut bids = Vec::new();
                let mut asks = Vec::new();
                
                if let Some(bid_array) = data.get("bids").and_then(|b| b.as_array()) {
                    for bid in bid_array.iter() {
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
                
                if let Some(ask_array) = data.get("asks").and_then(|a| a.as_array()) {
                    for ask in ask_array.iter() {
                        if let Some(ask_data) = ask.as_array() {
                            if ask_data.len() >= 2 {
                                let price = ask_data[0].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                                let size = ask_data[1].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                                if size > 0.0 { // Only include non-zero volumes
                                    asks.push([price, size]);
                                }
                            }
                        }
                    }
                }
                
                // Create OrderBookSnapshot for delta tracking
                let snapshot = OrderBookSnapshot {
                    symbol: standard_symbol.clone(),
                    exchange: "binance_us".to_string(),
                    version: chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default() as u64,
                    timestamp,
                    bids: bids.clone(),
                    asks: asks.clone(),
                };
                
                // Update OrderBookTracker with snapshot
                self.orderbook_tracker.update_snapshot(&standard_symbol, "binance_us", snapshot.clone()).await;
                
                // Compute delta if this is an update (not first snapshot)
                if let Some(delta) = self.orderbook_tracker.compute_delta(&snapshot, &standard_symbol).await {
                    // Send delta update via channel
                    if let Some(tx) = &self.delta_tx {
                        if let Err(e) = tx.send(delta.clone()).await {
                            warn!("Failed to send Binance.US orderbook delta: {}", e);
                        }
                    }
                    
                    // Write delta to shared memory for ultra-low latency access
                    if let Some(writer) = &self.delta_writer {
                        let shared_delta = self.convert_to_shared_delta(&delta);
                        let mut writer_guard = writer.lock().await;
                        if let Err(e) = writer_guard.write_delta(&shared_delta) {
                            warn!("Failed to write Binance.US delta to shared memory: {}", e);
                        } else {
                            info!(
                                "🚀 Binance.US delta written to shared memory for {}: {} bid changes, {} ask changes (vs {} full levels)", 
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
                    exchange: "binance_us".to_string(),
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
                        warn!("Failed to send Binance.US orderbook update: {}", e);
                    }
                }
                
                info!("Processed orderbook for Binance.US symbol: {}", standard_symbol);
            }
        }
        
        Ok(())
    }
}