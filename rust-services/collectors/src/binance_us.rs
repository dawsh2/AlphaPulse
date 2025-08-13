// Binance.US WebSocket collector for USDT pairs
use alphapulse_common::{Result, Trade, MetricsCollector};
use crate::collector_trait::MarketDataCollector;
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
                } else {
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
}