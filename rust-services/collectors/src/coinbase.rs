// Coinbase Pro WebSocket collector with L2 orderbook support
use alphapulse_common::{
    Result, Trade, CoinbaseTradeMessage, CoinbaseL2UpdateMessage,
    OrderBookUpdate, OrderBookLevel, MetricsCollector,
    OrderBookTracker, OrderBookSnapshot, OrderBookDelta,
    shared_memory::{OrderBookDeltaWriter, SharedOrderBookDelta, SharedMemoryWriter, SharedTrade},
    event_driven_shm::EventDrivenTradeWriter,
    shared_memory_registry::{SharedMemoryRegistry, FeedType, create_feed_metadata}
};
use crate::collector_trait::MarketDataCollector;
use std::collections::HashMap;
use std::path::PathBuf;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{info, warn, error, debug};
use url::Url;

pub struct CoinbaseCollector {
    symbols: Vec<String>,
    ws_url: String,
    healthy: Arc<AtomicBool>,
    metrics: Arc<MetricsCollector>,
    orderbook_tx: Option<mpsc::Sender<OrderBookUpdate>>,
    delta_tx: Option<mpsc::Sender<OrderBookDelta>>,
    orderbooks: Arc<tokio::sync::RwLock<HashMap<String, OrderBookUpdate>>>,
    orderbook_tracker: OrderBookTracker,
    delta_writer: Option<Arc<tokio::sync::Mutex<OrderBookDeltaWriter>>>,
    trade_writer: Option<Arc<tokio::sync::Mutex<EventDrivenTradeWriter>>>,
}

impl CoinbaseCollector {
    pub fn new(symbols: Vec<String>) -> Self {
        Self {
            symbols,
            ws_url: "wss://ws-feed.exchange.coinbase.com".to_string(),
            healthy: Arc::new(AtomicBool::new(false)),
            metrics: Arc::new(MetricsCollector::new()),
            orderbook_tx: None,
            delta_tx: None,
            orderbooks: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            orderbook_tracker: OrderBookTracker::new(50), // Track top 50 levels
            delta_writer: None,
            trade_writer: None,
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
        info!("🎯 Setting up shared memory writers for Coinbase");
        
        // Create shared memory writer for orderbook deltas
        let delta_path = "./shm/coinbase_orderbook_deltas";
        let delta_writer = OrderBookDeltaWriter::create(delta_path, 10000)?;
        self.delta_writer = Some(Arc::new(tokio::sync::Mutex::new(delta_writer)));
        
        // Create event-driven shared memory writer for trades
        let trade_path = "./shm/coinbase_trades";
        let trade_writer = EventDrivenTradeWriter::create(trade_path, 10000)?;
        self.trade_writer = Some(Arc::new(tokio::sync::Mutex::new(trade_writer)));
        
        // Register feeds with service discovery
        self.register_feeds(delta_path, trade_path)?;
        
        // Start heartbeat task for service discovery
        self.start_heartbeat_task();
        
        info!("✅ Coinbase shared memory writers and service registration complete");
        Ok(self)
    }
    
    fn register_feeds(&self, delta_path: &str, trade_path: &str) -> Result<()> {
        let mut registry = SharedMemoryRegistry::new()?;
        
        // Register orderbook delta feed
        let delta_metadata = create_feed_metadata(
            "coinbase_orderbook_deltas".to_string(),
            FeedType::OrderBookDeltas,
            PathBuf::from(delta_path),
            "coinbase".to_string(),
            None, // Multi-symbol feed
            10000,
        )?;
        registry.register_feed(delta_metadata)?;
        
        // Register trade feed
        let trade_metadata = create_feed_metadata(
            "coinbase_trades".to_string(),
            FeedType::Trades,
            PathBuf::from(trade_path),
            "coinbase".to_string(),
            None, // Multi-symbol feed
            10000,
        )?;
        registry.register_feed(trade_metadata)?;
        
        info!("📋 Registered Coinbase feeds with service discovery");
        Ok(())
    }
    
    fn start_heartbeat_task(&self) {
        use alphapulse_common::shared_memory_registry::update_feed_heartbeat;
        
        // Start heartbeat task for trade feed
        tokio::spawn(async {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                interval.tick().await;
                if let Err(e) = update_feed_heartbeat("coinbase_trades") {
                    tracing::warn!("Failed to update trades heartbeat: {}", e);
                }
            }
        });
        
        // Start heartbeat task for delta feed
        tokio::spawn(async {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                interval.tick().await;
                if let Err(e) = update_feed_heartbeat("coinbase_orderbook_deltas") {
                    tracing::warn!("Failed to update deltas heartbeat: {}", e);
                }
            }
        });
        
        info!("🫀 Started heartbeat tasks for Coinbase feeds");
    }
    
    async fn connect_websocket(&self) -> Result<()> {
        let url = Url::parse(&self.ws_url)?;
        info!("Connecting to Coinbase WebSocket: {}", url);
        
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();
        
        // Subscribe to trade messages and L2 orderbook for all symbols
        let subscribe_msg = json!({
            "type": "subscribe",
            "product_ids": self.symbols,
            "channels": ["matches", "level2_batch"]
        });
        
        write.send(Message::Text(subscribe_msg.to_string())).await?;
        info!("Subscribed to Coinbase trades for symbols: {:?}", self.symbols);
        
        self.healthy.store(true, Ordering::Relaxed);
        self.metrics.record_websocket_connection_status("coinbase", true);
        
        Ok(())
    }
    
    async fn handle_message(&self, msg: Message, tx: &mpsc::Sender<Trade>) -> Result<()> {
        match msg {
            Message::Text(text) => {
                debug!("Received Coinbase message: {}", text);
                
                // Try parsing as trade message
                if let Ok(trade_msg) = serde_json::from_str::<CoinbaseTradeMessage>(&text) {
                    if trade_msg.r#type == Some("match".to_string()) {
                        let trade = Trade::from(trade_msg.clone());
                        
                        // Write trade to event-driven shared memory with instant notifications
                        if let Some(writer) = &self.trade_writer {
                            let shared_trade = self.convert_to_shared_trade(&trade);
                            let mut writer_guard = writer.lock().await;
                            if let Err(e) = writer_guard.write_trade_and_notify(&shared_trade) {
                                warn!("Failed to write trade to shared memory: {}", e);
                            } else {
                                debug!("🚀 Trade written to event-driven shared memory for {}", trade.symbol);
                            }
                        }
                        
                        // Send to processing channel
                        if let Err(e) = tx.send(trade).await {
                            warn!("Failed to send trade to channel: {}", e);
                            return Ok(()); // Don't crash on channel errors
                        }
                        
                        // Record metrics
                        self.metrics.record_trade_processed("coinbase", &trade_msg.product_id);
                        self.metrics.record_websocket_message("coinbase", "trade");
                    }
                } 
                // Try parsing as L2 update
                else if let Ok(l2_msg) = serde_json::from_str::<CoinbaseL2UpdateMessage>(&text) {
                    if l2_msg.r#type == Some("l2update".to_string()) && self.orderbook_tx.is_some() {
                        // Process L2 orderbook update
                        self.handle_l2_update(l2_msg).await?;
                        self.metrics.record_websocket_message("coinbase", "l2_update");
                    }
                }
                // Try parsing as snapshot (initial L2 state)
                else if text.contains("\"type\":\"snapshot\"") {
                    if let Ok(snapshot) = serde_json::from_str::<serde_json::Value>(&text) {
                        if self.orderbook_tx.is_some() {
                            self.handle_l2_snapshot(snapshot).await?;
                            self.metrics.record_websocket_message("coinbase", "l2_snapshot");
                        }
                    }
                }
                else {
                    // Handle subscription confirmations and other messages
                    if text.contains("\"type\":\"subscriptions\"") {
                        info!("Coinbase subscription confirmed");
                    }
                }
            }
            Message::Ping(data) => {
                debug!("Received Coinbase ping, sending pong");
                // WebSocket library handles pong automatically
            }
            Message::Pong(_) => {
                debug!("Received Coinbase pong");
            }
            Message::Close(_) => {
                warn!("Coinbase WebSocket closed");
                self.healthy.store(false, Ordering::Relaxed);
                self.metrics.record_websocket_connection_status("coinbase", false);
            }
            Message::Binary(_) => {
                warn!("Received unexpected binary message from Coinbase");
            }
            Message::Frame(_) => {
                // Frame messages can be ignored
                debug!("Received frame message from Coinbase");
            }
        }
        
        Ok(())
    }
}

#[async_trait]
impl MarketDataCollector for CoinbaseCollector {
    async fn start(&self, tx: mpsc::Sender<Trade>) -> Result<()> {
        loop {
            match self.run_collector(&tx).await {
                Ok(_) => {
                    info!("Coinbase collector completed normally");
                    break;
                }
                Err(e) => {
                    error!("Coinbase collector error: {}", e);
                    self.healthy.store(false, Ordering::Relaxed);
                    self.metrics.record_websocket_connection_status("coinbase", false);
                    self.metrics.record_websocket_reconnection("coinbase");
                    
                    // Wait before reconnecting
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    info!("Attempting to reconnect to Coinbase...");
                }
            }
        }
        
        Ok(())
    }
    
    async fn stop(&self) -> Result<()> {
        info!("Stopping Coinbase collector");
        self.healthy.store(false, Ordering::Relaxed);
        Ok(())
    }
    
    fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }
    
    fn exchange_name(&self) -> &str {
        "coinbase"
    }
    
    fn symbols(&self) -> &[String] {
        &self.symbols
    }
}

impl CoinbaseCollector {
    fn convert_to_shared_trade(&self, trade: &Trade) -> SharedTrade {
        let timestamp_ns = (trade.timestamp * 1_000_000_000.0) as u64;
        
        // Determine side
        let side = trade.side.as_ref().map(|s| s == "buy").unwrap_or(false);
        
        // Get trade_id or empty string
        let trade_id = trade.trade_id.as_ref().map(|s| s.as_str()).unwrap_or("");
        
        SharedTrade::new(
            timestamp_ns,
            &trade.symbol,
            &trade.exchange,
            trade.price,
            trade.volume,
            side,
            trade_id
        )
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
    
    async fn handle_l2_snapshot(&self, snapshot: serde_json::Value) -> Result<()> {
        let product_id = snapshot["product_id"].as_str().unwrap_or("");
        let timestamp = chrono::Utc::now().timestamp() as f64;
        
        // Parse bids and asks from snapshot
        let mut bids = Vec::new();
        let mut asks = Vec::new();
        
        if let Some(bid_array) = snapshot["bids"].as_array() {
            for bid in bid_array.iter() { // ALL levels
                if let Some(bid_data) = bid.as_array() {
                    if bid_data.len() >= 2 {
                        let price = bid_data[0].as_str().unwrap_or("0").parse().unwrap_or(0.0);
                        let size = bid_data[1].as_str().unwrap_or("0").parse().unwrap_or(0.0);
                        bids.push(OrderBookLevel { price, size });
                    }
                }
            }
        }
        
        if let Some(ask_array) = snapshot["asks"].as_array() {
            for ask in ask_array.iter() { // ALL levels
                if let Some(ask_data) = ask.as_array() {
                    if ask_data.len() >= 2 {
                        let price = ask_data[0].as_str().unwrap_or("0").parse().unwrap_or(0.0);
                        let size = ask_data[1].as_str().unwrap_or("0").parse().unwrap_or(0.0);
                        asks.push(OrderBookLevel { price, size });
                    }
                }
            }
        }
        
        // Create OrderBookSnapshot for delta tracking
        let snapshot = OrderBookSnapshot {
            symbol: product_id.to_string(),
            exchange: "coinbase".to_string(),
            version: chrono::Utc::now().timestamp_nanos() as u64,
            timestamp,
            bids: bids.into_iter().map(|l| [l.price, l.size]).collect(),
            asks: asks.into_iter().map(|l| [l.price, l.size]).collect(),
        };
        
        // Update OrderBookTracker with snapshot
        self.orderbook_tracker.update_snapshot(product_id, "coinbase", snapshot.clone()).await;
        
        // Create legacy OrderBookUpdate for backward compatibility
        let orderbook = OrderBookUpdate {
            symbol: product_id.to_string(),
            exchange: "coinbase".to_string(),
            timestamp,
            bids: snapshot.bids.clone(),
            asks: snapshot.asks.clone(),
            sequence: None,
            update_type: Some("snapshot".to_string()),
        };
        
        // Store in local cache
        let mut orderbooks = self.orderbooks.write().await;
        orderbooks.insert(product_id.to_string(), orderbook.clone());
        
        // Send to channel if available (backward compatibility)
        if let Some(tx) = &self.orderbook_tx {
            if let Err(e) = tx.send(orderbook).await {
                warn!("Failed to send orderbook snapshot: {}", e);
            }
        }
        
        info!("Received L2 snapshot for {}", product_id);
        Ok(())
    }
    
    async fn handle_l2_update(&self, update: CoinbaseL2UpdateMessage) -> Result<()> {
        let timestamp = chrono::Utc::now().timestamp() as f64;
        
        // Get current orderbook from cache
        let mut orderbooks = self.orderbooks.write().await;
        if let Some(orderbook) = orderbooks.get_mut(&update.product_id) {
            // Apply changes to orderbook (legacy compatibility)
            for change in &update.changes {
                if change.len() >= 3 {
                    let side = &change[0];
                    let price: f64 = change[1].parse().unwrap_or(0.0);
                    let size: f64 = change[2].parse().unwrap_or(0.0);
                    
                    if side == "buy" {
                        // Update bids
                        if size == 0.0 {
                            orderbook.bids.retain(|level| level[0] != price);
                        } else {
                            if let Some(level) = orderbook.bids.iter_mut().find(|l| l[0] == price) {
                                level[1] = size;
                            } else {
                                orderbook.bids.push([price, size]);
                                orderbook.bids.sort_by(|a, b| b[0].partial_cmp(&a[0]).unwrap());
                            }
                        }
                    } else if side == "sell" {
                        // Update asks
                        if size == 0.0 {
                            orderbook.asks.retain(|level| level[0] != price);
                        } else {
                            if let Some(level) = orderbook.asks.iter_mut().find(|l| l[0] == price) {
                                level[1] = size;
                            } else {
                                orderbook.asks.push([price, size]);
                                orderbook.asks.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());
                            }
                        }
                    }
                }
            }
            
            // Create updated snapshot for delta computation
            let new_snapshot = OrderBookSnapshot {
                symbol: update.product_id.clone(),
                exchange: "coinbase".to_string(),
                version: chrono::Utc::now().timestamp_nanos() as u64,
                timestamp,
                bids: orderbook.bids.clone(),
                asks: orderbook.asks.clone(),
            };
            
            // Compute delta against previous snapshot
            if let Some(delta) = self.orderbook_tracker.compute_delta(&new_snapshot, &update.product_id).await {
                // Send delta update via channel (90% bandwidth reduction!)
                if let Some(tx) = &self.delta_tx {
                    if let Err(e) = tx.send(delta.clone()).await {
                        warn!("Failed to send orderbook delta: {}", e);
                    }
                }
                
                // Write delta to shared memory for ultra-low latency access
                if let Some(writer) = &self.delta_writer {
                    let shared_delta = self.convert_to_shared_delta(&delta);
                    let mut writer_guard = writer.lock().await;
                    if let Err(e) = writer_guard.write_delta(&shared_delta) {
                        warn!("Failed to write delta to shared memory: {}", e);
                    } else {
                        info!(
                            "🚀 Delta written to shared memory for {}: {} bid changes, {} ask changes (vs {} full levels)", 
                            update.product_id,
                            delta.bid_changes.len(),
                            delta.ask_changes.len(),
                            orderbook.bids.len() + orderbook.asks.len()
                        );
                    }
                }
            }
            
            // Update the tracker with new snapshot
            self.orderbook_tracker.update_snapshot(&update.product_id, "coinbase", new_snapshot).await;
            
            orderbook.timestamp = timestamp;
            orderbook.update_type = Some("update".to_string());
            
            // Send full orderbook for backward compatibility
            if let Some(tx) = &self.orderbook_tx {
                if let Err(e) = tx.send(orderbook.clone()).await {
                    warn!("Failed to send orderbook update: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    async fn run_collector(&self, tx: &mpsc::Sender<Trade>) -> Result<()> {
        let url = Url::parse(&self.ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();
        
        // Subscribe to trade messages and L2 orderbook
        let subscribe_msg = json!({
            "type": "subscribe",
            "product_ids": self.symbols,
            "channels": ["matches", "level2_batch"]
        });
        
        write.send(Message::Text(subscribe_msg.to_string())).await?;
        info!("Connected and subscribed to Coinbase for symbols: {:?}", self.symbols);
        
        self.healthy.store(true, Ordering::Relaxed);
        self.metrics.record_websocket_connection_status("coinbase", true);
        
        // Process incoming messages
        while let Some(msg_result) = read.next().await {
            match msg_result {
                Ok(msg) => {
                    if let Err(e) = self.handle_message(msg, tx).await {
                        error!("Error handling Coinbase message: {}", e);
                    }
                }
                Err(e) => {
                    error!("WebSocket error from Coinbase: {}", e);
                    return Err(e.into());
                }
            }
        }
        
        self.healthy.store(false, Ordering::Relaxed);
        self.metrics.record_websocket_connection_status("coinbase", false);
        
        Ok(())
    }
}