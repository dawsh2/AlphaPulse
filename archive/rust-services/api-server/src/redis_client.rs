// Redis client for reading from streams
use alphapulse_common::{Result, Trade};
use redis::{aio::MultiplexedConnection, AsyncCommands};
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

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
        // Build the stream key: trades:exchange:symbol
        let stream_key = format!("trades:{}:{}", exchange, symbol);
        
        let mut conn = self.connection.clone();
        
        // Read from Redis Stream using XREVRANGE (newest first)
        // Format: XREVRANGE stream_key + - COUNT limit
        let entries: Vec<(String, HashMap<String, String>)> = redis::cmd("XREVRANGE")
            .arg(&stream_key)
            .arg("+")  // Start from newest
            .arg("-")  // To oldest
            .arg("COUNT")
            .arg(limit)
            .query_async(&mut conn)
            .await
            .unwrap_or_else(|e| {
                debug!("Failed to read from stream {}: {}", stream_key, e);
                Vec::new()
            });
        
        // Convert stream entries to Trade objects
        let mut trades = Vec::new();
        for (_id, fields) in entries {
            if let (Some(timestamp), Some(price), Some(volume)) = 
                (fields.get("timestamp"), fields.get("price"), fields.get("volume")) {
                
                let trade = Trade {
                    timestamp: timestamp.parse().unwrap_or(0.0),
                    price: price.parse().unwrap_or(0.0),
                    volume: volume.parse().unwrap_or(0.0),
                    side: fields.get("side").cloned(),
                    trade_id: fields.get("trade_id").cloned(),
                    symbol: fields.get("symbol").cloned().unwrap_or_else(|| symbol.to_string()),
                    exchange: fields.get("exchange").cloned().unwrap_or_else(|| exchange.to_string()),
                };
                trades.push(trade);
            }
        }
        
        // If no trades in stream, try the latest key as fallback
        if trades.is_empty() {
            let latest_key = format!("latest:{}:{}", exchange, symbol);
            if let Ok(Some(trade_json)) = conn.get::<_, Option<String>>(&latest_key).await {
                if let Ok(trade) = serde_json::from_str::<Trade>(&trade_json) {
                    trades.push(trade);
                }
            }
        }
        
        debug!("Retrieved {} trades for {}:{} from stream", trades.len(), exchange, symbol);
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
        let stream_key = format!("trades:{}:{}", exchange, symbol);
        let mut conn = self.connection.clone();
        
        // Convert timestamps to Redis Stream IDs (milliseconds-sequence)
        // If not provided, use full range
        let start_id = start_timestamp
            .map(|ts| format!("{}-0", (ts * 1000.0) as i64))
            .unwrap_or_else(|| "-".to_string());
        let end_id = end_timestamp
            .map(|ts| format!("{}-9999", (ts * 1000.0) as i64))
            .unwrap_or_else(|| "+".to_string());
        
        // Use XRANGE for chronological order with time range
        let mut cmd = redis::cmd("XRANGE");
        cmd.arg(&stream_key)
           .arg(&start_id)
           .arg(&end_id);
        
        if let Some(count) = limit {
            cmd.arg("COUNT").arg(count);
        }
        
        let entries: Vec<(String, HashMap<String, String>)> = cmd
            .query_async(&mut conn)
            .await
            .unwrap_or_else(|e| {
                debug!("Failed to read range from stream {}: {}", stream_key, e);
                Vec::new()
            });
        
        // Convert stream entries to Trade objects
        let mut trades = Vec::new();
        for (_id, fields) in entries {
            if let (Some(timestamp), Some(price), Some(volume)) = 
                (fields.get("timestamp"), fields.get("price"), fields.get("volume")) {
                
                let trade = Trade {
                    timestamp: timestamp.parse().unwrap_or(0.0),
                    price: price.parse().unwrap_or(0.0),
                    volume: volume.parse().unwrap_or(0.0),
                    side: fields.get("side").cloned(),
                    trade_id: fields.get("trade_id").cloned(),
                    symbol: fields.get("symbol").cloned().unwrap_or_else(|| symbol.to_string()),
                    exchange: fields.get("exchange").cloned().unwrap_or_else(|| exchange.to_string()),
                };
                trades.push(trade);
            }
        }
        
        debug!("Retrieved {} trades in range for {}:{}", trades.len(), exchange, symbol);
        Ok(trades)
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