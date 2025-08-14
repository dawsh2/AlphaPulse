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
use tracing::{error, info};

const COINBASE_WS_URL: &str = "wss://ws-feed.exchange.coinbase.com";

#[derive(Debug, Serialize)]
struct CoinbaseSubscribe {
    r#type: String,
    product_ids: Vec<String>,
    channels: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CoinbaseMessage {
    r#type: String,
    #[serde(default)]
    product_id: Option<String>,
    #[serde(default)]
    price: Option<String>,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    side: Option<String>,
    #[serde(default)]
    time: Option<String>,
    #[serde(default)]
    trade_id: Option<u64>,
}

pub struct CoinbaseCollector {
    socket_writer: Arc<UnixSocketWriter>,
    symbol_mapper: Arc<RwLock<SymbolMapper>>,
}

impl CoinbaseCollector {
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
        info!("Connecting to Coinbase WebSocket at {}", COINBASE_WS_URL);

        let (ws_stream, _) = connect_async(COINBASE_WS_URL).await
            .map_err(|e| anyhow::anyhow!("Coinbase WebSocket connection failed: {}", e))?;

        info!("Connected to Coinbase WebSocket");

        let (mut write, mut read) = ws_stream.split();

        // Subscribe to trades
        let subscribe_msg = CoinbaseSubscribe {
            r#type: "subscribe".to_string(),
            product_ids: vec!["BTC-USD".to_string(), "ETH-USD".to_string()],
            channels: vec!["matches".to_string()],
        };

        let msg = serde_json::to_string(&subscribe_msg)?;
        write.send(Message::Text(msg)).await?;
        info!("Subscribed to Coinbase trade feed");

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    self.handle_message(&text).await;
                }
                Ok(Message::Close(_)) => {
                    info!("Coinbase WebSocket closed");
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
        match serde_json::from_str::<CoinbaseMessage>(text) {
            Ok(msg) => {
                if msg.r#type == "match" {
                    self.handle_trade(msg).await;
                }
            }
            Err(e) => {
                error!("Failed to parse Coinbase message: {} - {}", e, text);
            }
        }
    }

    async fn handle_trade(&self, trade: CoinbaseMessage) {
        if let (Some(product_id), Some(price_str), Some(size_str), Some(side)) = 
            (trade.product_id, trade.price, trade.size, trade.side) {
            
            if let (Ok(price), Ok(volume)) = (price_str.parse::<f64>(), size_str.parse::<f64>()) {
                let timestamp_ns = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;

                // Get symbol and exchange IDs from mappers
                let symbol_id = {
                    let mapper = self.symbol_mapper.read();
                    mapper.get_id(&product_id).unwrap_or_else(|| {
                        drop(mapper);
                        let mut mapper = self.symbol_mapper.write();
                        mapper.add_symbol(product_id.clone())
                    })
                };

                let exchange_id = ExchangeId::Coinbase as u16;

                // Convert prices to fixed-point (8 decimal places)
                let price_fixed = (price * 1e8) as u64;
                let volume_fixed = (volume * 1e8) as u64;

                let trade_message = TradeMessage::new(
                    timestamp_ns,
                    price_fixed,
                    volume_fixed,
                    symbol_id,
                    exchange_id,
                    if side == "buy" { TradeSide::Buy } else { TradeSide::Sell },
                );

                if let Err(e) = self.socket_writer.write_trade(&trade_message) {
                    error!("Failed to send trade: {}", e);
                } else {
                    info!("Sent {} trade: ${:.2} ({} {})", product_id, price, volume, side);
                }
            }
        }
    }
}