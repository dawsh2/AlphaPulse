use crate::unix_socket::UnixSocketWriter;
use alphapulse_protocol::*;
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use metrics::{counter, histogram};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};
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
    symbol_mapper: Arc<RwLock<SymbolMapper>>,
}

impl KrakenCollector {
    pub fn new(
        socket_writer: Arc<UnixSocketWriter>,
        symbol_mapper: Arc<RwLock<SymbolMapper>>,
    ) -> Self {
        Self {
            socket_writer,
            symbol_mapper,
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
        let symbol = self.normalize_symbol(pair);
        
        let symbol_id = {
            let mut mapper = self.symbol_mapper.write();
            mapper.add_symbol(symbol.clone())
        };

        if let Some(Value::Array(trades)) = arr.get(1) {
            for trade_data in trades {
                if let Some(trade) = trade_data.as_array() {
                    if trade.len() >= 6 {
                        let price = trade[0].as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        let volume = trade[1].as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        let timestamp = trade[2].as_f64().unwrap_or(0.0);
                        let side = trade[3].as_str().unwrap_or("?");
                        
                        let trade_side = match side {
                            "b" => TradeSide::Buy,
                            "s" => TradeSide::Sell,
                            _ => TradeSide::Unknown,
                        };
                        
                        let timestamp_ns = (timestamp * 1e9) as u64;
                        let price_fixed = (price * 1e8) as u64;
                        let volume_fixed = (volume * 1e8) as u64;
                        
                        let trade_msg = TradeMessage::new(
                            timestamp_ns,
                            price_fixed,
                            volume_fixed,
                            symbol_id,
                            ExchangeId::Kraken as u16,
                            trade_side,
                        );
                        
                        if let Err(e) = self.socket_writer.write_trade(&trade_msg) {
                            error!("Failed to write trade: {}", e);
                        } else {
                            counter!("kraken.trades_processed").increment(1);
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
        let symbol = self.normalize_symbol(pair);
        
        let symbol_id = {
            let mut mapper = self.symbol_mapper.write();
            mapper.add_symbol(symbol.clone())
        };

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
                            let price = level[0].as_str()
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);
                            let volume = level[1].as_str()
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);
                            
                            bids.push(PriceLevel::new(
                                (price * 1e8) as u64,
                                (volume * 1e8) as u64,
                            ));
                        }
                    }
                }
            }

            if let Some(Value::Array(ask_arr)) = book_data.get("as") {
                for ask in ask_arr.iter().take(10) {
                    if let Some(level) = ask.as_array() {
                        if level.len() >= 2 {
                            let price = level[0].as_str()
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);
                            let volume = level[1].as_str()
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);
                            
                            asks.push(PriceLevel::new(
                                (price * 1e8) as u64,
                                (volume * 1e8) as u64,
                            ));
                        }
                    }
                }
            }
        }

        if !bids.is_empty() || !asks.is_empty() {
            let orderbook = OrderBookMessage {
                timestamp_ns,
                symbol_id,
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

    fn normalize_symbol(&self, pair: &str) -> String {
        pair.replace("XBT", "BTC")
            .replace('/', "")
            .chars()
            .collect::<String>()
            .replace("USD", "/USD")
            .replace("USDT", "/USDT")
    }
}