// Kraken WebSocket collector
use alphapulse_common::{Result, Trade, KrakenTradeMessage, MetricsCollector};
use crate::collector_trait::MarketDataCollector;
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
    
    async fn handle_message(&self, msg: Message, tx: &mpsc::Sender<Trade>) -> Result<()> {
        match msg {
            Message::Text(text) => {
                debug!("Received Kraken message: {}", text);
                
                // Try to parse as trade message
                if let Ok(trade_msg) = serde_json::from_str::<KrakenTradeMessage>(&text) {
                    if trade_msg.channel == "trade" && trade_msg.r#type == "update" {
                        for trade_data in trade_msg.data {
                            let mut trade = Trade::from(trade_data.clone());
                            
                            // Convert symbol back to standard format
                            let standard_symbol = Self::convert_symbol_from_kraken(&trade_data.symbol);
                            
                            // Send to processing channel
                            if let Err(e) = tx.send(trade).await {
                                warn!("Failed to send trade to channel: {}", e);
                                return Ok(()); // Don't crash on channel errors
                            }
                            
                            // Record metrics
                            self.metrics.record_trade_processed("kraken", &standard_symbol);
                            self.metrics.record_websocket_message("kraken", "trade");
                        }
                    }
                } else {
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
        let subscribe_msg = json!({
            "method": "subscribe",
            "params": {
                "channel": "trade",
                "symbol": self.symbols,
                "snapshot": false
            }
        });
        
        write.send(Message::Text(subscribe_msg.to_string())).await?;
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
}