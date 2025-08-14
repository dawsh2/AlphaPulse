// Kraken WebSocket collector with L2 orderbook support
use alphapulse_common::{
    Result, Trade, KrakenTradeMessage, MetricsCollector,
    OrderBookUpdate, OrderBookLevel, OrderBookTracker, 
    OrderBookSnapshot, OrderBookDelta,
    shared_memory::{OrderBookDeltaWriter, SharedOrderBookDelta}
};
use crate::collector_trait::MarketDataCollector;
use std::collections::HashMap;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{info, warn, error, debug};
use url::Url;

pub struct KrakenCollector {
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

impl KrakenCollector {
    pub fn new(symbols: Vec<String>) -> Self {
        // Convert symbols to Kraken format (e.g., "BTC/USD" -> "XBT/USD")
        let kraken_symbols: Vec<String> = symbols
            .iter()
            .map(|s| Self::convert_symbol_to_kraken(s))
            .collect();
            
        Self {
            symbols: kraken_symbols,
            ws_url: "wss://ws.kraken.com/v2".to_string(),
            healthy: Arc::new(AtomicBool::new(false)),
            metrics: Arc::new(MetricsCollector::new()),
            orderbook_tx: None,
            delta_tx: None,
            orderbooks: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            orderbook_tracker: OrderBookTracker::new(50), // Track top 50 levels
            delta_writer: None,
        }
    }
    
    fn convert_symbol_to_kraken(symbol: &str) -> String {
        // Convert common symbols to Kraken format
        match symbol {
            "BTC-USD" | "BTC/USD" => "XBT/USD".to_string(),
            "ETH-USD" | "ETH/USD" => "ETH/USD".to_string(),
            s => s.replace("-", "/"),
        }
    }
    
    fn convert_symbol_from_kraken(kraken_symbol: &str) -> String {
        // Convert Kraken symbols back to standard format
        match kraken_symbol {
            "XBT/USD" => "BTC/USD".to_string(),
            s => s.to_string(),
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
            "/tmp/alphapulse_shm/kraken_orderbook_deltas", 
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
                debug!("Received Kraken message: {}", text);
                
                // Try to parse as trade message
                if let Ok(trade_msg) = serde_json::from_str::<KrakenTradeMessage>(&text) {
                    if trade_msg.channel_name == Some("trade".to_string()) {
                        if let Some(trades) = trade_msg.trades {
                            for trade_values in trades {
                                // Parse trade from Vec<serde_json::Value>
                                // Kraken trade format: [price, volume, timestamp, side, orderType, misc]
                                if trade_values.len() >= 4 {
                                    let price = trade_values[0].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                                    let volume = trade_values[1].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                                    let timestamp = trade_values[2].as_f64().unwrap_or(0.0);
                                    let side = trade_values[3].as_str().unwrap_or("unknown").to_string();
                                    
                                    let trade = Trade {
                                        timestamp,
                                        symbol: trade_msg.pair.clone().unwrap_or_default(),
                                        exchange: "kraken".to_string(),
                                        price,
                                        volume,
                                        side: Some(side),
                                        trade_id: None,
                                    };
                                    
                                    // Send to processing channel
                                    if let Err(e) = tx.send(trade).await {
                                        warn!("Failed to send trade to channel: {}", e);
                                        return Ok(()); // Don't crash on channel errors
                                    }
                                    
                                    // Record metrics
                                    self.metrics.record_trade_processed("kraken", &trade_msg.pair.clone().unwrap_or_default());
                                    self.metrics.record_websocket_message("kraken", "trade");
                                }
                            }
                        }
                    }
                }
                // Try parsing as L2 orderbook message
                else if text.contains("\"channel\":\"level2\"") || text.contains("\"channel_name\":\"level2\"") {
                    if let Ok(orderbook_msg) = serde_json::from_str::<serde_json::Value>(&text) {
                        if self.orderbook_tx.is_some() || self.delta_tx.is_some() || self.delta_writer.is_some() {
                            self.handle_kraken_orderbook(orderbook_msg).await?;
                            self.metrics.record_websocket_message("kraken", "orderbook_update");
                        }
                    }
                }
                else {
                    // Handle subscription confirmations and other messages
                    if text.contains("\"method\":\"subscribe\"") && text.contains("\"result\":\"success\"") {
                        info!("Kraken subscription confirmed");
                    } else if text.contains("\"method\":\"pong\"") {
                        debug!("Received Kraken pong");
                    } else if text.contains("\"event\":\"heartbeat\"") {
                        debug!("Received Kraken heartbeat");
                    }
                }
            }
            Message::Ping(_) => {
                debug!("Received Kraken ping, sending pong");
                // WebSocket library handles pong automatically
            }
            Message::Pong(_) => {
                debug!("Received Kraken pong");
            }
            Message::Close(_) => {
                warn!("Kraken WebSocket closed");
                self.healthy.store(false, Ordering::Relaxed);
                self.metrics.record_websocket_connection_status("kraken", false);
            }
            Message::Binary(_) => {
                warn!("Received unexpected binary message from Kraken");
            }
            Message::Frame(_) => {
                // Frame messages can be ignored
                debug!("Received frame message from Kraken");
            }
        }
        
        Ok(())
    }
}

#[async_trait]
impl MarketDataCollector for KrakenCollector {
    async fn start(&self, tx: mpsc::Sender<Trade>) -> Result<()> {
        loop {
            match self.run_collector(&tx).await {
                Ok(_) => {
                    info!("Kraken collector completed normally");
                    break;
                }
                Err(e) => {
                    error!("Kraken collector error: {}", e);
                    self.healthy.store(false, Ordering::Relaxed);
                    self.metrics.record_websocket_connection_status("kraken", false);
                    self.metrics.record_websocket_reconnection("kraken");
                    
                    // Wait before reconnecting
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    info!("Attempting to reconnect to Kraken...");
                }
            }
        }
        
        Ok(())
    }
    
    async fn stop(&self) -> Result<()> {
        info!("Stopping Kraken collector");
        self.healthy.store(false, Ordering::Relaxed);
        Ok(())
    }
    
    fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }
    
    fn exchange_name(&self) -> &str {
        "kraken"
    }
    
    fn symbols(&self) -> &[String] {
        &self.symbols
    }
}

impl KrakenCollector {
    async fn run_collector(&self, tx: &mpsc::Sender<Trade>) -> Result<()> {
        let url = Url::parse(&self.ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();
        
        // Subscribe to trade messages for all symbols
        let trade_subscribe_msg = json!({
            "method": "subscribe",
            "params": {
                "channel": "trade",
                "symbol": self.symbols,
                "snapshot": false
            }
        });
        
        write.send(Message::Text(trade_subscribe_msg.to_string())).await?;
        
        // Subscribe to orderbook (level2) if we have orderbook handlers
        if self.orderbook_tx.is_some() || self.delta_tx.is_some() || self.delta_writer.is_some() {
            let book_subscribe_msg = json!({
                "method": "subscribe", 
                "params": {
                    "channel": "level2",
                    "symbol": self.symbols,
                    "depth": 100,
                    "snapshot": true
                }
            });
            
            write.send(Message::Text(book_subscribe_msg.to_string())).await?;
            info!("Subscribed to Kraken orderbooks for symbols: {:?}", self.symbols);
        }
        
        info!("Connected and subscribed to Kraken for symbols: {:?}", self.symbols);
        
        self.healthy.store(true, Ordering::Relaxed);
        self.metrics.record_websocket_connection_status("kraken", true);
        
        // Store write half in Arc<Mutex> for sharing
        let write_shared = Arc::new(tokio::sync::Mutex::new(write));
        let write_heartbeat = write_shared.clone();
        
        // Send periodic heartbeat to keep connection alive
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                let ping_msg = json!({
                    "method": "ping"
                });
                
                let mut write_guard = write_heartbeat.lock().await;
                if let Err(e) = write_guard.send(Message::Text(ping_msg.to_string())).await {
                    error!("Failed to send Kraken heartbeat: {}", e);
                    break;
                }
                drop(write_guard);
            }
        });
        
        // Process incoming messages
        while let Some(msg_result) = read.next().await {
            match msg_result {
                Ok(msg) => {
                    if let Err(e) = self.handle_message(msg, tx).await {
                        error!("Error handling Kraken message: {}", e);
                    }
                }
                Err(e) => {
                    error!("WebSocket error from Kraken: {}", e);
                    return Err(e.into());
                }
            }
        }
        
        self.healthy.store(false, Ordering::Relaxed);
        self.metrics.record_websocket_connection_status("kraken", false);
        
        Ok(())
    }
    
    async fn handle_kraken_orderbook(&self, msg: serde_json::Value) -> Result<()> {
        // Parse Kraken L2 orderbook message format
        if let Some(data) = msg.get("data") {
            if let Some(data_array) = data.as_array() {
                for item in data_array {
                    if let Some(symbol) = item.get("symbol").and_then(|s| s.as_str()) {
                        let timestamp = chrono::Utc::now().timestamp() as f64;
                        
                        // Parse bids and asks
                        let mut bids = Vec::new();
                        let mut asks = Vec::new();
                        
                        if let Some(bid_array) = item.get("bids").and_then(|b| b.as_array()) {
                            for bid in bid_array.iter() {
                                if let Some(bid_data) = bid.as_array() {
                                    if bid_data.len() >= 2 {
                                        let price = bid_data[0].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                                        let size = bid_data[1].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                                        bids.push([price, size]);
                                    }
                                }
                            }
                        }
                        
                        if let Some(ask_array) = item.get("asks").and_then(|a| a.as_array()) {
                            for ask in ask_array.iter() {
                                if let Some(ask_data) = ask.as_array() {
                                    if ask_data.len() >= 2 {
                                        let price = ask_data[0].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                                        let size = ask_data[1].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                                        asks.push([price, size]);
                                    }
                                }
                            }
                        }
                        
                        // Create OrderBookSnapshot for delta tracking
                        let snapshot = OrderBookSnapshot {
                            symbol: Self::convert_symbol_from_kraken(symbol),
                            exchange: "kraken".to_string(),
                            version: chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default() as u64,
                            timestamp,
                            bids: bids.clone(),
                            asks: asks.clone(),
                        };
                        
                        // Update OrderBookTracker with snapshot
                        self.orderbook_tracker.update_snapshot(&snapshot.symbol, "kraken", snapshot.clone()).await;
                        
                        // Compute delta if this is an update (not first snapshot)
                        if let Some(delta) = self.orderbook_tracker.compute_delta(&snapshot, &snapshot.symbol).await {
                            // Send delta update via channel
                            if let Some(tx) = &self.delta_tx {
                                if let Err(e) = tx.send(delta.clone()).await {
                                    warn!("Failed to send Kraken orderbook delta: {}", e);
                                }
                            }
                            
                            // Write delta to shared memory for ultra-low latency access
                            if let Some(writer) = &self.delta_writer {
                                let shared_delta = self.convert_to_shared_delta(&delta);
                                let mut writer_guard = writer.lock().await;
                                if let Err(e) = writer_guard.write_delta(&shared_delta) {
                                    warn!("Failed to write Kraken delta to shared memory: {}", e);
                                } else {
                                    info!(
                                        "🚀 Kraken delta written to shared memory for {}: {} bid changes, {} ask changes (vs {} full levels)", 
                                        snapshot.symbol,
                                        delta.bid_changes.len(),
                                        delta.ask_changes.len(),
                                        bids.len() + asks.len()
                                    );
                                }
                            }
                        }
                        
                        // Create legacy OrderBookUpdate for backward compatibility
                        let orderbook = OrderBookUpdate {
                            symbol: snapshot.symbol.clone(),
                            exchange: "kraken".to_string(),
                            timestamp,
                            bids,
                            asks,
                            sequence: None,
                            update_type: Some("snapshot".to_string()),
                        };
                        
                        // Store in local cache
                        let mut orderbooks = self.orderbooks.write().await;
                        orderbooks.insert(snapshot.symbol.clone(), orderbook.clone());
                        
                        // Send to channel if available (backward compatibility)
                        if let Some(tx) = &self.orderbook_tx {
                            if let Err(e) = tx.send(orderbook).await {
                                warn!("Failed to send Kraken orderbook update: {}", e);
                            }
                        }
                        
                        info!("Processed L2 orderbook for Kraken symbol: {}", snapshot.symbol);
                    }
                }
            }
        }
        
        Ok(())
    }
}