// Core types shared across AlphaPulse Rust services
use serde::{Deserialize, Serialize};
use chrono;

// Trade data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub timestamp: f64,  // Unix timestamp with fractional seconds
    pub symbol: String,
    pub exchange: String,
    pub price: f64,
    pub volume: f64,
    pub side: Option<String>,  // "buy" or "sell"
    pub trade_id: Option<String>,
}

// OrderBook update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookUpdate {
    pub timestamp: f64,
    pub symbol: String,
    pub exchange: String,
    pub bids: Vec<[f64; 2]>,  // [price, size]
    pub asks: Vec<[f64; 2]>,
    pub sequence: Option<u64>,
    pub update_type: Option<String>,  // "snapshot" or "l2update"
}

impl OrderBookUpdate {
    // Helper to convert from OrderBookLevel vec to [f64; 2] vec
    pub fn from_levels(
        timestamp: f64,
        symbol: String,
        exchange: String,
        bid_levels: Vec<OrderBookLevel>,
        ask_levels: Vec<OrderBookLevel>,
        sequence: Option<u64>,
    ) -> Self {
        Self {
            timestamp,
            symbol,
            exchange,
            bids: bid_levels.into_iter().map(|l| [l.price, l.size]).collect(),
            asks: ask_levels.into_iter().map(|l| [l.price, l.size]).collect(),
            sequence,
            update_type: None,
        }
    }
}

// Configuration for collectors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorConfig {
    pub exchange: String,
    pub symbols: Vec<String>,
    pub redis_url: String,
    pub api_port: u16,
    pub buffer_size: usize,
    pub batch_timeout_ms: u64,
}

// Kraken-specific message types
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum KrakenMessage {
    Trade(KrakenTradeMessage),
    OrderBook(KrakenOrderBookMessage),
    Subscription(KrakenSubscriptionMessage),
    Heartbeat(KrakenHeartbeat),
    SystemStatus(KrakenSystemStatus),
}

#[derive(Debug, Deserialize)]
pub struct KrakenTradeMessage {
    #[serde(rename = "channelID")]
    pub channel_id: Option<i64>,
    #[serde(rename = "channelName")]
    pub channel_name: Option<String>,
    pub pair: Option<String>,
    #[serde(rename = "trades")]
    pub trades: Option<Vec<Vec<serde_json::Value>>>,
}

#[derive(Debug, Deserialize)]
pub struct KrakenOrderBookMessage {
    #[serde(rename = "channelID")]
    pub channel_id: Option<i64>,
    pub pair: Option<String>,
    pub bids: Option<Vec<Vec<String>>>,
    pub asks: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Deserialize)]
pub struct KrakenSubscriptionMessage {
    pub event: String,
    pub status: Option<String>,
    pub pair: Option<String>,
    pub subscription: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct KrakenHeartbeat {
    pub event: String,
}

#[derive(Debug, Deserialize)]
pub struct KrakenSystemStatus {
    pub event: String,
    pub status: String,
    pub version: Option<String>,
}

// Coinbase message types
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum CoinbaseMessage {
    #[serde(rename = "ticker")]
    Ticker(CoinbaseTickerMessage),
    #[serde(rename = "match")]
    Match(CoinbaseMatchMessage),
    #[serde(rename = "snapshot")]
    Snapshot(CoinbaseSnapshotMessage),
    #[serde(rename = "l2update")]
    L2Update(CoinbaseL2UpdateMessage),
    #[serde(rename = "subscriptions")]
    Subscriptions(CoinbaseSubscriptionsMessage),
    #[serde(rename = "heartbeat")]
    Heartbeat(CoinbaseHeartbeatMessage),
    #[serde(rename = "error")]
    Error(CoinbaseErrorMessage),
}

#[derive(Debug, Deserialize)]
pub struct CoinbaseTickerMessage {
    pub product_id: String,
    pub price: String,
    pub time: String,
    pub best_bid: String,
    pub best_ask: String,
}

#[derive(Debug, Deserialize)]
pub struct CoinbaseMatchMessage {
    pub product_id: String,
    pub trade_id: u64,
    pub price: String,
    pub size: String,
    pub side: String,
    pub time: String,
}

#[derive(Debug, Deserialize)]
pub struct CoinbaseSnapshotMessage {
    pub product_id: String,
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

#[derive(Debug, Deserialize)]
pub struct CoinbaseL2UpdateMessage {
    pub product_id: String,
    pub time: String,
    pub changes: Vec<[String; 3]>,  // ["buy"/"sell", price, size]
    #[serde(rename = "type")]
    pub r#type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CoinbaseSubscriptionsMessage {
    pub channels: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CoinbaseHeartbeatMessage {
    pub sequence: u64,
    pub last_trade_id: u64,
    pub product_id: String,
    pub time: String,
}

#[derive(Debug, Deserialize)]
pub struct CoinbaseErrorMessage {
    pub message: String,
    pub reason: Option<String>,
}

// Additional types needed by collectors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookLevel {
    pub price: f64,
    pub size: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSummary {
    pub total_trades: u64,
    pub total_orderbooks: u64,
    pub start_time: f64,
    pub end_time: f64,
    pub total_symbols: Option<u64>,
    pub total_exchanges: Option<u64>,
    pub total_records: Option<u64>,
    pub date_range: Option<String>,
}

// Coinbase trade message for parsing
#[derive(Debug, Clone, Deserialize)]
pub struct CoinbaseTradeMessage {
    pub product_id: String,
    pub trade_id: u64,
    pub price: String,
    pub size: String,
    pub side: String,
    pub time: String,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
}

impl From<CoinbaseTradeMessage> for Trade {
    fn from(msg: CoinbaseTradeMessage) -> Self {
        Trade {
            timestamp: chrono::DateTime::parse_from_rfc3339(&msg.time)
                .map(|dt| dt.timestamp() as f64 + dt.timestamp_subsec_nanos() as f64 / 1e9)
                .unwrap_or(0.0),
            symbol: msg.product_id.replace("-", "/"),  // Convert BTC-USD to BTC/USD
            exchange: "coinbase".to_string(),
            price: msg.price.parse().unwrap_or(0.0),
            volume: msg.size.parse().unwrap_or(0.0),
            side: Some(msg.side),
            trade_id: Some(msg.trade_id.to_string()),
        }
    }
}