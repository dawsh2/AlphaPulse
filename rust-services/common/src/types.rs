// Core trading data types - JSON-serializable and compatible with Python schemas
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub timestamp: f64,
    pub price: f64,
    pub volume: f64,
    pub side: Option<String>,
    pub trade_id: Option<String>,
    pub symbol: String,
    pub exchange: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OHLCVBar {
    pub timestamp: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookLevel {
    pub price: f64,
    pub size: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookSnapshot {
    pub symbol: String,
    pub exchange: String,
    pub timestamp: f64,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketStatistics {
    pub symbol: String,
    pub exchange: String,
    pub mean_price: f64,
    pub volatility: f64,
    pub volume_avg: f64,
    pub high_24h: f64,
    pub low_24h: f64,
    pub price_change_24h: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSummary {
    pub total_symbols: i32,
    pub total_exchanges: i32,
    pub total_records: i32,
    pub date_range: serde_json::Value,
    pub symbols_by_exchange: serde_json::Value,
    pub record_count_by_symbol: serde_json::Value,
}

// Exchange-specific message types

#[derive(Debug, Clone, Deserialize)]
pub struct CoinbaseTradeMessage {
    pub r#type: String,
    pub trade_id: i64,
    pub sequence: i64,
    pub maker_order_id: String,
    pub taker_order_id: String,
    pub time: String,
    pub product_id: String,
    pub size: String,
    pub price: String,
    pub side: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KrakenTradeMessage {
    pub channel: String,
    pub r#type: String,
    pub data: Vec<KrakenTradeData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KrakenTradeData {
    pub symbol: String,
    pub side: String,
    pub ord_type: String,
    pub qty: String,
    pub price: String,
    pub trade_id: i64,
    pub timestamp: String,
}

// Configuration types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorConfig {
    pub exchange: String,
    pub symbols: Vec<String>,
    pub redis_url: String,
    pub api_port: u16,
    pub buffer_size: usize,
    pub batch_timeout_ms: u64,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            exchange: "coinbase".to_string(),
            symbols: vec!["BTC-USD".to_string(), "ETH-USD".to_string()],
            redis_url: "redis://localhost:6379".to_string(),
            api_port: 3001,
            buffer_size: 1000,
            batch_timeout_ms: 100,
        }
    }
}

// L2 Orderbook specific types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookUpdate {
    pub symbol: String,
    pub exchange: String,
    pub timestamp: f64,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub sequence: Option<u64>,
    pub update_type: String, // "snapshot" or "update"
}

// Exchange-specific orderbook message types
#[derive(Debug, Clone, Deserialize)]
pub struct CoinbaseL2UpdateMessage {
    pub r#type: String,
    pub product_id: String,
    pub time: String,
    pub changes: Vec<Vec<String>>, // [side, price, size]
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceDepthUpdateMessage {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "U")]
    pub first_update_id: u64,
    #[serde(rename = "u")]
    pub final_update_id: u64,
    #[serde(rename = "b")]
    pub bids: Vec<Vec<String>>, // [price, quantity]
    #[serde(rename = "a")]
    pub asks: Vec<Vec<String>>, // [price, quantity]
}

// Helper functions for type conversions
impl From<CoinbaseTradeMessage> for Trade {
    fn from(msg: CoinbaseTradeMessage) -> Self {
        Trade {
            timestamp: DateTime::parse_from_rfc3339(&msg.time)
                .unwrap_or_else(|_| Utc::now().into())
                .timestamp() as f64,
            price: msg.price.parse().unwrap_or(0.0),
            volume: msg.size.parse().unwrap_or(0.0),
            side: Some(msg.side),
            trade_id: Some(msg.trade_id.to_string()),
            symbol: msg.product_id.clone(),
            exchange: "coinbase".to_string(),
        }
    }
}

impl From<KrakenTradeData> for Trade {
    fn from(data: KrakenTradeData) -> Self {
        Trade {
            timestamp: data.timestamp.parse().unwrap_or(0.0),
            price: data.price.parse().unwrap_or(0.0),
            volume: data.qty.parse().unwrap_or(0.0),
            side: Some(data.side),
            trade_id: Some(data.trade_id.to_string()),
            symbol: data.symbol.clone(),
            exchange: "kraken".to_string(),
        }
    }
}