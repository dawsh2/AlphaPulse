use crate::unix_socket::UnixSocketWriter;
use alphapulse_protocol::*;
use alphapulse_protocol::conversion::{parse_price_to_fixed_point, parse_volume_to_fixed_point, parse_trade_side};
use alphapulse_protocol::validation::{validate_trade_data, detect_corruption_patterns};
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use metrics::{counter, histogram};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message, tungstenite::http::Request};
use tracing::{debug, error, info, warn};

const KRAKEN_WS_URL: &str = "wss://ws.kraken.com";
const RECONNECT_DELAY: Duration = Duration::from_secs(5);
const PING_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Serialize)]
struct KrakenSubscribe {
    event: String,
    pair: Vec<String>,
    subscription: KrakenSubscription,
}

#[derive(Debug, Serialize)]
struct KrakenSubscription {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum KrakenMessage {
    Event(KrakenEvent),
    Trade(Value),
    OrderBook(Value),
}

#[derive(Debug, Deserialize)]
struct KrakenEvent {
    event: String,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    errorMessage: Option<String>,
}

pub struct KrakenCollector {
    socket_writer: Arc<UnixSocketWriter>,
    symbol_cache: Arc<RwLock<std::collections::HashMap<String, u64>>>, // pair -> hash
}

impl KrakenCollector {
    pub fn new(
        socket_writer: Arc<UnixSocketWriter>,
        _symbol_mapper: Arc<RwLock<std::collections::HashMap<String, u32>>>, // Keep signature for now
    ) -> Self {
        Self {
            socket_writer,
            symbol_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn connect_and_stream(&self) -> Result<()> {
        info!("Connecting to Kraken WebSocket at {}", KRAKEN_WS_URL);

        let (ws_stream, _) = connect_async(KRAKEN_WS_URL).await
            .map_err(|e| anyhow::anyhow!("Kraken WebSocket connection failed: {}", e))?;

        info!("Connected to Kraken WebSocket");

        let (mut write, mut read) = ws_stream.split();

        let subscribe_msg = KrakenSubscribe {
            event: "subscribe".to_string(),
            pair: vec!["XBT/USD".to_string(), "ETH/USD".to_string()],
            subscription: KrakenSubscription {
                name: "trade".to_string(),
            },
        };

        let msg = serde_json::to_string(&subscribe_msg)?;
        write.send(Message::Text(msg)).await?;
        info!("Subscribed to Kraken trade feed");

        let subscribe_ob = KrakenSubscribe {
            event: "subscribe".to_string(),
            pair: vec!["XBT/USD".to_string(), "ETH/USD".to_string()],
            subscription: KrakenSubscription {
                name: "book".to_string(),
            },
        };

        let msg = serde_json::to_string(&subscribe_ob)?;
        write.send(Message::Text(msg)).await?;
        info!("Subscribed to Kraken orderbook feed");

        let mut ping_interval = tokio::time::interval(PING_INTERVAL);
        let mut last_message = Instant::now();

        loop {
            tokio::select! {
                msg = timeout(Duration::from_secs(60), read.next()) => {
                    match msg {
                        Ok(Some(Ok(Message::Text(text)))) => {
                            last_message = Instant::now();
                            self.handle_message(&text).await;
                        }
                        Ok(Some(Ok(Message::Ping(data)))) => {
                            write.send(Message::Pong(data)).await?;
                        }
                        Ok(Some(Ok(Message::Close(_)))) => {
                            info!("WebSocket closed by server");
                            break;
                        }
                        Ok(Some(Err(e))) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                        Ok(None) => {
                            info!("WebSocket stream ended");
                            break;
                        }
                        Err(_) => {
                            warn!("No message received in 60 seconds");
                            if last_message.elapsed() > Duration::from_secs(120) {
                                error!("Connection appears dead, reconnecting");
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                
                _ = ping_interval.tick() => {
                    write.send(Message::Ping(vec![])).await?;
                }
            }
        }

        Ok(())
    }

    async fn handle_message(&self, text: &str) {
        let start = Instant::now();
        
        match serde_json::from_str::<Value>(text) {
            Ok(Value::Array(arr)) if arr.len() >= 3 => {
                let channel_name = arr.last()
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
                if channel_name.starts_with("trade") {
                    self.handle_trade(&arr).await;
                } else if channel_name.starts_with("book") {
                    self.handle_orderbook(&arr).await;
                }
                
                let latency_us = start.elapsed().as_micros() as f64;
                histogram!("kraken.parse_latency_us").record(latency_us);
            }
            Ok(Value::Object(obj)) => {
                if let Some(event) = obj.get("event").and_then(|v| v.as_str()) {
                    match event {
                        "systemStatus" => info!("Kraken system status: {:?}", obj),
                        "subscriptionStatus" => info!("Subscription status: {:?}", obj),
                        "heartbeat" => debug!("Heartbeat received"),
                        _ => debug!("Event: {}", event),
                    }
                }
            }
            _ => {
                debug!("Unhandled message: {}", text);
            }
        }
    }

    async fn handle_trade(&self, arr: &[Value]) {
        if arr.len() < 4 {
            return;
        }

        let pair = arr[3].as_str().unwrap_or("");
        let symbol_hash = self.get_or_create_symbol_hash(pair);

        if let Some(Value::Array(trades)) = arr.get(1) {
            for trade_data in trades {
                if let Some(trade) = trade_data.as_array() {
                    if trade.len() >= 6 {
                        // Use precision-preserving conversion
                        let price_str = trade[0].as_str().unwrap_or("0");
                        let volume_str = trade[1].as_str().unwrap_or("0");
                        let side_str = trade[3].as_str().unwrap_or("?");
                        
                        match (
                            parse_price_to_fixed_point(price_str),
                            parse_volume_to_fixed_point(volume_str),
                            parse_trade_side(side_str)
                        ) {
                            (Ok(price_fixed), Ok(volume_fixed), Ok(trade_side)) => {
                                let timestamp = trade[2].as_f64().unwrap_or(0.0);
                                let timestamp_ns = (timestamp * 1e9) as u64;
                                
                                // Validate the trade data before processing
                                if let Err(validation_error) = validate_trade_data(
                                    pair, 
                                    price_fixed, 
                                    volume_fixed, 
                                    timestamp_ns, 
                                    "kraken"
                                ) {
                                    error!("Trade validation failed for {}: {}", pair, validation_error);
                                    continue;
                                }
                                
                                // Check for potential data corruption
                                let warnings = detect_corruption_patterns(pair, price_fixed, volume_fixed);
                                if !warnings.is_empty() {
                                    warn!("Data corruption warnings for {}: {:?}", pair, warnings);
                                }
                                
                                let trade_msg = TradeMessage::new(
                                    timestamp_ns,
                                    price_fixed as u64,
                                    volume_fixed as u64,
                                    symbol_hash,
                                    trade_side,
                                );
                                
                                if let Err(e) = self.socket_writer.write_trade(&trade_msg) {
                                    error!("Failed to write trade: {}", e);
                                } else {
                                    counter!("kraken.trades_processed").increment(1);
                                    // Use conversion module for display to show exact precision
                                    let display_price = alphapulse_protocol::conversion::fixed_point_to_f64(price_fixed);
                                    let display_volume = alphapulse_protocol::conversion::fixed_point_to_f64(volume_fixed);
                                    debug!("Kraken trade: {} ${:.8} ({:.8} {:?})", pair, display_price, display_volume, trade_side);
                                }
                            }
                            _ => {
                                error!("Failed to parse Kraken trade data: price='{}', volume='{}', side='{}'", 
                                       price_str, volume_str, side_str);
                            }
                        }
                    }
                }
            }
        }
    }

    async fn handle_orderbook(&self, arr: &[Value]) {
        if arr.len() < 4 {
            return;
        }

        let pair = arr[3].as_str().unwrap_or("");
        let symbol_hash = self.get_or_create_symbol_hash(pair);

        let timestamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let mut bids = Vec::new();
        let mut asks = Vec::new();

        if let Some(Value::Object(book_data)) = arr.get(1) {
            if let Some(Value::Array(bid_arr)) = book_data.get("bs") {
                for bid in bid_arr.iter().take(10) {
                    if let Some(level) = bid.as_array() {
                        if level.len() >= 2 {
                            let price_str = level[0].as_str().unwrap_or("0");
                            let volume_str = level[1].as_str().unwrap_or("0");
                            
                            if let (Ok(price_fixed), Ok(volume_fixed)) = (
                                parse_price_to_fixed_point(price_str),
                                parse_volume_to_fixed_point(volume_str)
                            ) {
                                bids.push(PriceLevel::new(
                                    price_fixed as u64,
                                    volume_fixed as u64,
                                ));
                            }
                        }
                    }
                }
            }

            if let Some(Value::Array(ask_arr)) = book_data.get("as") {
                for ask in ask_arr.iter().take(10) {
                    if let Some(level) = ask.as_array() {
                        if level.len() >= 2 {
                            let price_str = level[0].as_str().unwrap_or("0");
                            let volume_str = level[1].as_str().unwrap_or("0");
                            
                            if let (Ok(price_fixed), Ok(volume_fixed)) = (
                                parse_price_to_fixed_point(price_str),
                                parse_volume_to_fixed_point(volume_str)
                            ) {
                                asks.push(PriceLevel::new(
                                    price_fixed as u64,
                                    volume_fixed as u64,
                                ));
                            }
                        }
                    }
                }
            }
        }

        if !bids.is_empty() || !asks.is_empty() {
            let orderbook = OrderBookMessage {
                timestamp_ns,
                symbol_hash,
                bids,
                asks,
            };

            if let Err(e) = self.socket_writer.write_orderbook(&orderbook) {
                error!("Failed to write orderbook: {}", e);
            } else {
                counter!("kraken.orderbooks_processed").increment(1);
            }
        }
    }

    fn get_or_create_symbol_hash(&self, pair: &str) -> u64 {
        let mut cache = self.symbol_cache.write();
        
        if let Some(&hash) = cache.get(pair) {
            return hash;
        }
        
        // Parse Kraken format: XBT/USD -> BTC-USD, ETH/USD -> ETH-USD
        let normalized = pair.replace("XBT", "BTC");
        let parts: Vec<&str> = normalized.split('/').collect();
        
        let descriptor = if parts.len() == 2 {
            SymbolDescriptor::spot("kraken", parts[0], parts[1])
        } else {
            // Fallback for unknown formats
            SymbolDescriptor::spot("kraken", pair, "USD")
        };
        
        let hash = descriptor.hash();
        cache.insert(pair.to_string(), hash);
        
        // Send symbol mapping message
        let mapping = SymbolMappingMessage::new(&descriptor);
        if let Err(e) = self.socket_writer.write_symbol_mapping(&mapping) {
            error!("Failed to send symbol mapping: {}", e);
        } else {
            info!("Sent symbol mapping: {} -> {}", pair, hash);
        }
        
        hash
    }
}