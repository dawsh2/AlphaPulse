use crate::unix_socket::UnixSocketWriter;
use alphapulse_protocol::*;
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info};

const ALPACA_WS_URL: &str = "wss://stream.data.alpaca.markets/v2/iex";

#[derive(Debug, Serialize)]
struct AlpacaAuth {
    action: String,
    key: String,
    secret: String,
}

#[derive(Debug, Serialize)]
struct AlpacaSubscribe {
    action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    trades: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quotes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bars: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct AlpacaMessage {
    #[serde(rename = "T")]
    msg_type: Option<String>,
    #[serde(rename = "S")]
    symbol: Option<String>,
    #[serde(rename = "p")]
    price: Option<f64>,
    #[serde(rename = "s")]
    size: Option<u64>,
    #[serde(rename = "t")]
    timestamp: Option<String>,
    #[serde(rename = "c")]
    conditions: Option<Vec<String>>,
    #[serde(rename = "i")]
    trade_id: Option<u64>,
    // Quote fields
    #[serde(rename = "bp")]
    bid_price: Option<f64>,
    #[serde(rename = "bs")]
    bid_size: Option<u64>,
    #[serde(rename = "ap")]
    ask_price: Option<f64>,
    #[serde(rename = "as")]
    ask_size: Option<u64>,
    // Bar fields
    #[serde(rename = "o")]
    open: Option<f64>,
    #[serde(rename = "h")]
    high: Option<f64>,
    #[serde(rename = "l")]
    low: Option<f64>,
    #[serde(rename = "cl")]
    close: Option<f64>,
    #[serde(rename = "v")]
    volume: Option<u64>,
    // Status message fields
    msg: Option<String>,
    code: Option<i32>,
}

pub struct AlpacaCollector {
    socket_writer: Arc<UnixSocketWriter>,
    symbol_cache: Arc<RwLock<std::collections::HashMap<String, u64>>>, // ticker -> hash
    api_key: String,
    api_secret: String,
}

impl AlpacaCollector {
    pub fn new(
        socket_writer: Arc<UnixSocketWriter>,
        _symbol_mapper: Arc<RwLock<std::collections::HashMap<String, u32>>>, // Keep signature for now
    ) -> Result<Self> {
        // Get API credentials from environment
        let api_key = std::env::var("ALPACA_API_KEY")
            .context("ALPACA_API_KEY environment variable not set")?;
        let api_secret = std::env::var("ALPACA_API_SECRET")
            .context("ALPACA_API_SECRET environment variable not set")?;
        
        Ok(Self {
            socket_writer,
            symbol_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            api_key,
            api_secret,
        })
    }

    pub async fn connect_and_stream(&self) -> Result<()> {
        info!("Connecting to Alpaca WebSocket at {}", ALPACA_WS_URL);

        let (ws_stream, _) = connect_async(ALPACA_WS_URL).await
            .map_err(|e| anyhow::anyhow!("Alpaca WebSocket connection failed: {}", e))?;

        info!("Connected to Alpaca WebSocket");

        let (mut write, mut read) = ws_stream.split();

        // First, authenticate
        let auth_msg = AlpacaAuth {
            action: "auth".to_string(),
            key: self.api_key.clone(),
            secret: self.api_secret.clone(),
        };

        let msg = serde_json::to_string(&auth_msg)?;
        write.send(Message::Text(msg)).await?;
        info!("Sent authentication to Alpaca");

        // Wait for auth response
        let mut authenticated = false;
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!("Alpaca response: {}", text);
                    if let Ok(parsed) = serde_json::from_str::<Vec<AlpacaMessage>>(&text) {
                        for message in parsed {
                            if let Some(msg_type) = &message.msg_type {
                                if msg_type == "success" {
                                    if let Some(msg_text) = &message.msg {
                                        if msg_text.contains("authenticated") {
                                            info!("Successfully authenticated with Alpaca");
                                            authenticated = true;
                                            break;
                                        }
                                    }
                                } else if msg_type == "error" {
                                    error!("Alpaca error: {:?}", message.msg);
                                }
                            }
                        }
                    }
                    if authenticated {
                        break;
                    }
                }
                _ => {}
            }
        }

        if !authenticated {
            return Err(anyhow::anyhow!("Failed to authenticate with Alpaca"));
        }

        // Subscribe to popular stocks and ETFs
        let subscribe_msg = AlpacaSubscribe {
            action: "subscribe".to_string(),
            trades: Some(vec![
                "AAPL".to_string(),
                "MSFT".to_string(),
                "GOOGL".to_string(),
                "AMZN".to_string(),
                "TSLA".to_string(),
                "SPY".to_string(),
                "QQQ".to_string(),
                "NVDA".to_string(),
                "META".to_string(),
                "AMD".to_string(),
            ]),
            quotes: Some(vec![
                "AAPL".to_string(),
                "MSFT".to_string(),
                "GOOGL".to_string(),
                "AMZN".to_string(),
                "TSLA".to_string(),
                "SPY".to_string(),
                "QQQ".to_string(),
                "NVDA".to_string(),
                "META".to_string(),
                "AMD".to_string(),
            ]),
            bars: None, // We'll focus on trades and quotes for now
        };

        let msg = serde_json::to_string(&subscribe_msg)?;
        write.send(Message::Text(msg)).await?;
        info!("Subscribed to Alpaca stock feeds");

        // Main message processing loop
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    self.handle_message(&text).await;
                }
                Ok(Message::Close(_)) => {
                    info!("Alpaca WebSocket closed");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_message(&self, text: &str) {
        // Alpaca sends messages as an array
        match serde_json::from_str::<Vec<AlpacaMessage>>(text) {
            Ok(messages) => {
                for msg in messages {
                    if let Some(msg_type) = &msg.msg_type {
                        match msg_type.as_str() {
                            "t" => self.handle_trade(msg).await,
                            "q" => self.handle_quote(msg).await,
                            "b" => self.handle_bar(msg).await,
                            "subscription" => {
                                info!("Alpaca subscription confirmed");
                            }
                            "error" => {
                                if let Some(msg_text) = &msg.msg {
                                    error!("Alpaca error: {}", msg_text);
                                }
                            }
                            _ => {
                                debug!("Unhandled Alpaca message type: {}", msg_type);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to parse Alpaca message: {} - {}", e, text);
            }
        }
    }

    async fn handle_trade(&self, trade: AlpacaMessage) {
        if let (Some(symbol), Some(price), Some(size)) = 
            (trade.symbol, trade.price, trade.size) {
            
            let timestamp_ns = if let Some(ts_str) = trade.timestamp {
                // Parse RFC3339 timestamp
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&ts_str) {
                    dt.timestamp_nanos_opt().unwrap_or_else(|| {
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_nanos() as i64
                    }) as u64
                } else {
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64
                }
            } else {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64
            };

            // Get or create symbol hash
            let symbol_hash = self.get_or_create_symbol_hash(&symbol);

            // Convert prices to fixed-point (8 decimal places)
            let price_fixed = (price * 1e8) as u64;
            let volume_fixed = (size as f64 * 1e8) as u64;

            let trade_message = TradeMessage::new(
                timestamp_ns,
                price_fixed,
                volume_fixed,
                symbol_hash,
                TradeSide::Buy, // Alpaca doesn't provide side info in trades
            );

            if let Err(e) = self.socket_writer.write_trade(&trade_message) {
                error!("Failed to send trade: {}", e);
            } else {
                debug!("Sent {} trade: ${:.2} ({})", symbol, price, size);
            }
        }
    }

    async fn handle_quote(&self, quote: AlpacaMessage) {
        if let (Some(symbol), Some(bid_price), Some(bid_size), Some(ask_price), Some(ask_size)) = 
            (quote.symbol, quote.bid_price, quote.bid_size, quote.ask_price, quote.ask_size) {
            
            let symbol_hash = self.get_or_create_symbol_hash(&symbol);

            let timestamp_ns = if let Some(ts_str) = quote.timestamp {
                // Parse RFC3339 timestamp
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&ts_str) {
                    dt.timestamp_nanos_opt().unwrap_or_else(|| {
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_nanos() as i64
                    }) as u64
                } else {
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64
                }
            } else {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64
            };

            // Note: Alpaca provides quotes (top-of-book), not full L2 order book data
            // We'll log the quote but NOT send L2 delta messages since Alpaca doesn't have L2 data
            debug!("Received {} quote: bid ${:.2} ({}), ask ${:.2} ({})", 
                symbol, bid_price, bid_size, ask_price, ask_size);
            
            // TODO: In the future, we could send this as a special "quote" message type
            // if we want to track top-of-book quotes separately from L2 data
        }
    }

    async fn handle_bar(&self, bar: AlpacaMessage) {
        // For now, we'll just log bars but not send them through the pipeline
        if let (Some(symbol), Some(open), Some(high), Some(low), Some(close), Some(volume)) = 
            (bar.symbol, bar.open, bar.high, bar.low, bar.close, bar.volume) {
            debug!("Received {} bar: O={:.2} H={:.2} L={:.2} C={:.2} V={}", 
                symbol, open, high, low, close, volume);
        }
    }
    
    fn get_or_create_symbol_hash(&self, ticker: &str) -> u64 {
        let mut cache = self.symbol_cache.write();
        
        if let Some(&hash) = cache.get(ticker) {
            return hash;
        }
        
        // Create stock symbol descriptor
        let descriptor = SymbolDescriptor::stock("alpaca", ticker);
        let hash = descriptor.hash();
        cache.insert(ticker.to_string(), hash);
        
        // Send symbol mapping message
        let mapping = SymbolMappingMessage::new(&descriptor);
        if let Err(e) = self.socket_writer.write_symbol_mapping(&mapping) {
            error!("Failed to send symbol mapping: {}", e);
        } else {
            info!("Sent symbol mapping: {} -> {}", ticker, hash);
        }
        
        hash
    }
}