// Redis writer for L2 orderbook data
use alphapulse_common::{Result, OrderBookUpdate, OrderBookLevel, MetricsCollector};
use redis::{aio::MultiplexedConnection, AsyncCommands};
use serde_json::json;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tracing::{info, warn, error, debug};

pub struct OrderBookWriter {
    connection: Arc<RwLock<Option<MultiplexedConnection>>>,
    redis_url: String,
    buffer: Arc<RwLock<VecDeque<OrderBookUpdate>>>,
    buffer_size: usize,
    batch_timeout: Duration,
    metrics: Arc<MetricsCollector>,
}

impl OrderBookWriter {
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
    
    pub async fn start(&self, mut rx: mpsc::Receiver<OrderBookUpdate>) -> Result<()> {
        info!("Starting L2 OrderBook writer");
        
        // Connect to Redis
        self.connect().await?;
        
        // Start batch flushing task
        let writer = self.clone();
        let flush_task = tokio::spawn(async move {
            writer.batch_flush_task().await;
        });
        
        // Process incoming orderbook updates
        while let Some(update) = rx.recv().await {
            if let Err(e) = self.add_update(update).await {
                error!("Failed to add orderbook update to buffer: {}", e);
            }
        }
        
        // Shutdown: flush remaining updates
        info!("Shutting down OrderBook writer, flushing remaining updates");
        self.flush_buffer().await?;
        
        flush_task.abort();
        Ok(())
    }
    
    async fn connect(&self) -> Result<()> {
        let client = redis::Client::open(self.redis_url.as_str())?;
        let connection = client.get_multiplexed_async_connection().await?;
        
        // Test connection
        let mut conn = connection.clone();
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;
        
        *self.connection.write().await = Some(connection);
        info!("Connected to Redis for OrderBook data at {}", self.redis_url);
        
        Ok(())
    }
    
    async fn add_update(&self, update: OrderBookUpdate) -> Result<()> {
        let mut buffer = self.buffer.write().await;
        
        if buffer.len() >= self.buffer_size {
            self.metrics.record_buffer_overflow("orderbook_buffer");
            buffer.pop_front();
            warn!("OrderBook buffer overflow, dropping oldest update");
        }
        
        buffer.push_back(update);
        self.metrics.record_buffer_size(buffer.len(), "orderbook_buffer");
        
        // Flush immediately if buffer is full
        if buffer.len() >= self.buffer_size {
            drop(buffer);
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
                    error!("OrderBook batch flush failed: {}", e);
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
        
        let updates: Vec<OrderBookUpdate> = buffer.drain(..).collect();
        drop(buffer);
        
        let batch_size = updates.len();
        debug!("Flushing {} orderbook updates to Redis", batch_size);
        
        // Group updates by exchange and symbol
        let mut grouped: std::collections::HashMap<String, Vec<&OrderBookUpdate>> = 
            std::collections::HashMap::new();
            
        for update in &updates {
            let key = format!("orderbook:{}:{}", update.exchange, update.symbol);
            grouped.entry(key).or_insert_with(Vec::new).push(update);
        }
        
        // Write to Redis
        let connection_guard = self.connection.read().await;
        if let Some(conn) = connection_guard.as_ref() {
            let mut conn = conn.clone();
            
            for (key, updates) in grouped {
                // Store latest orderbook snapshot
                if let Some(latest) = updates.last() {
                    let data = json!({
                        "timestamp": latest.timestamp,
                        "symbol": latest.symbol,
                        "exchange": latest.exchange,
                        "bids": &latest.bids, // ALL levels
                        "asks": &latest.asks, // ALL levels
                        "sequence": latest.sequence,
                        "update_type": latest.update_type,
                    });
                    
                    // Store current snapshot
                    let _: () = conn.set(&key, data.to_string()).await?;
                    
                    // Also store in time series for history
                    let ts_key = format!("{}:{}", key, latest.timestamp);
                    let _: () = conn.set_ex(&ts_key, data.to_string(), 3600).await?; // Expire after 1 hour
                    
                    self.metrics.record_redis_operation("orderbook_set", true);
                }
            }
        } else {
            return Err(alphapulse_common::AlphaPulseError::RedisError(
                redis::RedisError::from((redis::ErrorKind::IoError, "No Redis connection"))
            ));
        }
        
        let latency = start_time.elapsed().as_millis() as f64;
        self.metrics.record_redis_latency(latency, "orderbook_flush");
        self.metrics.record_batch_size(batch_size, "orderbook");
        
        info!("Flushed {} orderbook updates to Redis in {:.2}ms", batch_size, latency);
        Ok(())
    }
}

impl Clone for OrderBookWriter {
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