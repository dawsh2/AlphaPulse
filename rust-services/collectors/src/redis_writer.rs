// Redis Streams writer for trade data
use alphapulse_common::{Result, Trade, MetricsCollector};
use redis::{aio::MultiplexedConnection, AsyncCommands};
use serde_json::json;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tracing::{info, warn, error, debug};

pub struct RedisStreamsWriter {
    connection: Arc<RwLock<Option<MultiplexedConnection>>>,
    redis_url: String,
    buffer: Arc<RwLock<VecDeque<Trade>>>,
    buffer_size: usize,
    batch_timeout: Duration,
    metrics: Arc<MetricsCollector>,
}

impl RedisStreamsWriter {
    pub fn new(redis_url: String, buffer_size: usize, batch_timeout_ms: u64) -> Self {
        Self {
            connection: Arc::new(RwLock::new(None)),
            redis_url,
            buffer: Arc::new(RwLock::new(VecDeque::with_capacity(buffer_size))),
            buffer_size,
            batch_timeout: Duration::from_millis(batch_timeout_ms),
            metrics: Arc::new(MetricsCollector::new()),
        }
    }
    
    pub async fn start(&self, mut rx: mpsc::Receiver<Trade>) -> Result<()> {
        info!("Starting Redis Streams writer");
        
        // Connect to Redis
        self.connect().await?;
        
        // Start batch flushing task
        let writer = self.clone();
        let flush_task = tokio::spawn(async move {
            writer.batch_flush_task().await;
        });
        
        // Process incoming trades
        while let Some(trade) = rx.recv().await {
            if let Err(e) = self.add_trade(trade).await {
                error!("Failed to add trade to buffer: {}", e);
            }
        }
        
        // Shutdown: flush remaining trades
        info!("Shutting down Redis writer, flushing remaining trades");
        self.flush_buffer().await?;
        
        flush_task.abort();
        Ok(())
    }
    
    async fn connect(&self) -> Result<()> {
        let client = redis::Client::open(self.redis_url.as_str())?;
        let connection = client.get_multiplexed_async_connection().await?;
        
        // Test connection with a simple operation
        let mut conn = connection.clone();
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;
        
        *self.connection.write().await = Some(connection);
        info!("Connected to Redis at {}", self.redis_url);
        
        Ok(())
    }
    
    async fn add_trade(&self, trade: Trade) -> Result<()> {
        let mut buffer = self.buffer.write().await;
        
        if buffer.len() >= self.buffer_size {
            // Buffer is full, record overflow and drop oldest trade
            self.metrics.record_buffer_overflow("trade_buffer");
            buffer.pop_front();
            warn!("Trade buffer overflow, dropping oldest trade");
        }
        
        buffer.push_back(trade);
        self.metrics.record_buffer_size(buffer.len(), "trade_buffer");
        
        // Flush immediately if buffer is full
        if buffer.len() >= self.buffer_size {
            drop(buffer); // Release lock
            self.flush_buffer().await?;
        }
        
        Ok(())
    }
    
    async fn batch_flush_task(&self) {
        let mut interval = interval(self.batch_timeout);
        
        loop {
            interval.tick().await;
            
            let buffer_size = {
                let buffer = self.buffer.read().await;
                buffer.len()
            };
            
            if buffer_size > 0 {
                if let Err(e) = self.flush_buffer().await {
                    error!("Batch flush failed: {}", e);
                }
            }
        }
    }
    
    async fn flush_buffer(&self) -> Result<()> {
        let start_time = Instant::now();
        let mut buffer = self.buffer.write().await;
        
        if buffer.is_empty() {
            return Ok(());
        }
        
        let trades: Vec<Trade> = buffer.drain(..).collect();
        drop(buffer); // Release lock early
        
        let batch_size = trades.len();
        debug!("Flushing {} trades to Redis", batch_size);
        
        // Group trades by exchange and symbol for efficient streaming
        let mut streams: std::collections::HashMap<String, Vec<&Trade>> = 
            std::collections::HashMap::new();
            
        for trade in &trades {
            // Use the exchange and symbol from the Trade struct
            let exchange = &trade.exchange;
            let symbol = &trade.symbol;
                
            // Create stream key: trades:{exchange}:{symbol}
            let stream_key = format!("trades:{}:{}", exchange, symbol);
            
            streams.entry(stream_key).or_insert_with(Vec::new).push(trade);
        }
        
        // Write to Redis Streams
        let connection_guard = self.connection.read().await;
        if let Some(conn) = connection_guard.as_ref() {
            let mut conn = conn.clone();
            
            for (stream_key, stream_trades) in streams {
                match self.write_trades_to_stream(&mut conn, &stream_key, stream_trades).await {
                    Ok(count) => {
                        self.metrics.record_redis_operation("xadd", true);
                        debug!("Wrote {} trades to stream {}", count, stream_key);
                    }
                    Err(e) => {
                        self.metrics.record_redis_operation("xadd", false);
                        error!("Failed to write to stream {}: {}", stream_key, e);
                        return Err(e);
                    }
                }
            }
        } else {
            return Err(alphapulse_common::AlphaPulseError::RedisError(
                redis::RedisError::from((redis::ErrorKind::IoError, "No Redis connection"))
            ));
        }
        
        let latency = start_time.elapsed().as_millis() as f64;
        self.metrics.record_redis_latency(latency, "batch_flush");
        self.metrics.record_batch_size(batch_size, "redis");
        
        info!("Flushed {} trades to Redis in {:.2}ms", batch_size, latency);
        Ok(())
    }
    
    async fn write_trades_to_stream(
        &self,
        conn: &mut MultiplexedConnection,
        stream_key: &str,
        trades: Vec<&Trade>
    ) -> Result<usize> {
        let mut count = 0;
        
        for trade in trades {
            let trade_data = json!({
                "timestamp": trade.timestamp,
                "price": trade.price,
                "volume": trade.volume,
                "side": trade.side,
                "trade_id": trade.trade_id,
                "symbol": trade.symbol,
                "exchange": trade.exchange
            });
            
            // Convert to Redis stream fields
            let fields = vec![
                ("data", trade_data.to_string()),
                ("ingested_at", chrono::Utc::now().timestamp().to_string()),
            ];
            
            // For now, store as simple key-value until streams are fully implemented
            let key = format!("trade:{}:{}", stream_key, trade.timestamp);
            let _: () = conn.set(&key, trade_data.to_string()).await?;
            count += 1;
        }
        
        Ok(count)
    }
}

impl Clone for RedisStreamsWriter {
    fn clone(&self) -> Self {
        Self {
            connection: self.connection.clone(),
            redis_url: self.redis_url.clone(),
            buffer: self.buffer.clone(),
            buffer_size: self.buffer_size,
            batch_timeout: self.batch_timeout,
            metrics: self.metrics.clone(),
        }
    }
}