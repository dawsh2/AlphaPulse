// Redis client for reading from streams
use alphapulse_common::{Result, Trade};
use redis::{aio::MultiplexedConnection, AsyncCommands};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, warn};

pub struct RedisClient {
    connection: MultiplexedConnection,
}

impl RedisClient {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let connection = client.get_multiplexed_async_connection().await?;
        
        Ok(Self { connection })
    }
    
    pub fn get_connection(&self) -> MultiplexedConnection {
        self.connection.clone()
    }
    
    pub async fn get_recent_trades(
        &self, 
        symbol: &str, 
        exchange: &str, 
        limit: usize
    ) -> Result<Vec<Trade>> {
        // Convert symbol format for Redis key (BTC/USD -> BTC-USD for Coinbase)
        let redis_symbol = if exchange == "coinbase" {
            symbol.replace("/", "-")
        } else {
            symbol.to_string()
        };
        
        let pattern = format!("trade:trades:{}:{}:*", exchange, redis_symbol);
        
        let mut conn = self.connection.clone();
        let keys: Vec<String> = conn
            .keys(&pattern)
            .await
            .unwrap_or_else(|_| Vec::new());
        
        // Sort keys by timestamp (newest first) and limit
        let mut timestamp_keys: Vec<(f64, String)> = keys
            .into_iter()
            .filter_map(|key| {
                // Extract timestamp from key: trade:trades:exchange:symbol:timestamp
                let parts: Vec<&str> = key.split(':').collect();
                if parts.len() == 5 {
                    parts[4].parse::<f64>().ok().map(|ts| (ts, key))
                } else {
                    None
                }
            })
            .collect();
        
        // Sort by timestamp descending (newest first)
        timestamp_keys.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        
        // Get trade data for the most recent trades
        let mut trades = Vec::new();
        for (_, key) in timestamp_keys.into_iter().take(limit) {
            if let Ok(Some(trade_json)) = conn.get::<String, Option<String>>(key).await {
                if let Ok(trade) = serde_json::from_str::<Trade>(&trade_json) {
                    trades.push(trade);
                }
            }
        }
        
        debug!("Retrieved {} trades for {}:{}", trades.len(), exchange, symbol);
        Ok(trades)
    }
    
    pub async fn get_trades_in_range(
        &self,
        symbol: &str,
        exchange: &str,
        start_timestamp: Option<f64>,
        end_timestamp: Option<f64>,
        limit: Option<usize>,
    ) -> Result<Vec<Trade>> {
        // For now, use the same logic as get_recent_trades with filtering
        let trades = self.get_recent_trades(symbol, exchange, limit.unwrap_or(1000)).await?;
        
        let filtered_trades: Vec<Trade> = trades
            .into_iter()
            .filter(|trade| {
                let ts = trade.timestamp;
                let after_start = start_timestamp.map_or(true, |start| ts >= start);
                let before_end = end_timestamp.map_or(true, |end| ts <= end);
                after_start && before_end
            })
            .collect();
        
        debug!("Retrieved {} trades in range for {}:{}", filtered_trades.len(), exchange, symbol);
        Ok(filtered_trades)
    }
    
    pub async fn get_stream_info(&self, symbol: &str, exchange: &str) -> Result<HashMap<String, Value>> {
        let key = format!("info:{}:{}", exchange, symbol);
        
        let mut conn = self.connection.clone();
        let info_str: Option<String> = conn.get(&key).await.unwrap_or(None);
        
        let info = if let Some(data) = info_str {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            HashMap::new()
        };
        
        Ok(info)
    }
    
    pub async fn get_available_symbols(&self, exchange: &str) -> Result<Vec<String>> {
        let pattern = format!("recent_trades:{}:*", exchange);
        
        let mut conn = self.connection.clone();
        let keys: Vec<String> = conn
            .keys(&pattern)
            .await
            .unwrap_or_else(|_| Vec::new());
        
        // Extract symbols from keys
        let symbols: Vec<String> = keys
            .into_iter()
            .filter_map(|key| {
                let parts: Vec<&str> = key.split(':').collect();
                if parts.len() == 3 && parts[0] == "recent_trades" && parts[1] == exchange {
                    Some(parts[2].to_string())
                } else {
                    None
                }
            })
            .collect();
        
        Ok(symbols)
    }
}