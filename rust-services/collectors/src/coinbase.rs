// Coinbase Pro WebSocket collector with L2 orderbook support
use alphapulse_common::{
    Result, Trade, CoinbaseTradeMessage, CoinbaseL2UpdateMessage,
    OrderBookUpdate, OrderBookLevel, MetricsCollector
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

pub struct CoinbaseCollector {
    symbols: Vec<String>,
    ws_url: String,
    healthy: Arc<AtomicBool>,
    metrics: Arc<MetricsCollector>,
    orderbook_tx: Option<mpsc::Sender<OrderBookUpdate>>,
    orderbooks: Arc<tokio::sync::RwLock<HashMap<String, OrderBookUpdate>>>,
}

impl CoinbaseCollector {
    pub fn new(symbols: Vec<String>) -> Self {
        Self {
            symbols,
            ws_url: "wss://ws-feed.exchange.coinbase.com".to_string(),
            healthy: Arc::new(AtomicBool::new(false)),
            metrics: Arc::new(MetricsCollector::new()),
            orderbook_tx: None,
            orderbooks: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
    
    pub fn with_orderbook_sender(mut self, tx: mpsc::Sender<OrderBookUpdate>) -> Self {
        self.orderbook_tx = Some(tx);
        self
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
                    if trade_msg.r#type == "match" {
                        let trade = Trade::from(trade_msg.clone());
                        
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
                    if l2_msg.r#type == "l2update" && self.orderbook_tx.is_some() {
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
        
        let orderbook = OrderBookUpdate {
            symbol: product_id.to_string(),
            exchange: "coinbase".to_string(),
            timestamp,
            bids,
            asks,
            sequence: None,
            update_type: "snapshot".to_string(),
        };
        
        // Store in local cache
        let mut orderbooks = self.orderbooks.write().await;
        orderbooks.insert(product_id.to_string(), orderbook.clone());
        
        // Send to channel if available
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
            // Apply changes to orderbook
            for change in &update.changes {
                if change.len() >= 3 {
                    let side = &change[0];
                    let price: f64 = change[1].parse().unwrap_or(0.0);
                    let size: f64 = change[2].parse().unwrap_or(0.0);
                    
                    if side == "buy" {
                        // Update bids
                        if size == 0.0 {
                            // Remove level
                            orderbook.bids.retain(|level| level.price != price);
                        } else {
                            // Update or add level
                            if let Some(level) = orderbook.bids.iter_mut().find(|l| l.price == price) {
                                level.size = size;
                            } else {
                                orderbook.bids.push(OrderBookLevel { price, size });
                                orderbook.bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
                                // Keep ALL levels - no truncation
                            }
                        }
                    } else if side == "sell" {
                        // Update asks
                        if size == 0.0 {
                            // Remove level
                            orderbook.asks.retain(|level| level.price != price);
                        } else {
                            // Update or add level
                            if let Some(level) = orderbook.asks.iter_mut().find(|l| l.price == price) {
                                level.size = size;
                            } else {
                                orderbook.asks.push(OrderBookLevel { price, size });
                                orderbook.asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
                                // Keep ALL levels - no truncation
                            }
                        }
                    }
                }
            }
            
            orderbook.timestamp = timestamp;
            orderbook.update_type = "update".to_string();
            
            // Send updated orderbook
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