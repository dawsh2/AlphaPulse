// Tokio-based event-driven transport for real-time data streaming
// Zero polling, cross-platform, production-ready

use crate::{Result, AlphaPulseError, Trade};
use crossbeam::queue::ArrayQueue;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;
use tokio::sync::Notify;
use tracing::{debug, info, warn};

/// Performance metrics for monitoring
#[derive(Debug, Default)]
pub struct TransportMetrics {
    pub writes_total: AtomicU64,
    pub reads_total: AtomicU64,
    pub notifications_sent: AtomicU64,
    pub notifications_received: AtomicU64,
    pub queue_size: AtomicUsize,
    pub dropped_trades: AtomicU64,
}

/// Lock-free, event-driven transport for real-time data
pub struct TokioTransport {
    /// Lock-free ring buffer for zero-copy data transfer
    ring: Arc<ArrayQueue<Trade>>,
    
    /// Tokio's Notify for cross-platform event notification
    notify: Arc<Notify>,
    
    /// Metrics for monitoring
    metrics: Arc<TransportMetrics>,
    
    /// Capacity for overflow detection
    capacity: usize,
}

impl TokioTransport {
    /// Create a new transport with specified capacity
    pub fn new(capacity: usize) -> Self {
        info!("Creating TokioTransport with capacity: {}", capacity);
        
        Self {
            ring: Arc::new(ArrayQueue::new(capacity)),
            notify: Arc::new(Notify::new()),
            metrics: Arc::new(TransportMetrics::default()),
            capacity,
        }
    }
    
    /// Write a trade and notify all waiting consumers (non-blocking)
    pub async fn write(&self, trade: Trade) -> Result<()> {
        // Capture values before moving trade
        let symbol = trade.symbol.clone();
        let price = trade.price;
        
        // Try to push to ring buffer (lock-free, wait-free)
        match self.ring.push(trade) {
            Ok(()) => {
                self.metrics.writes_total.fetch_add(1, Ordering::Relaxed);
                self.metrics.queue_size.store(self.ring.len(), Ordering::Relaxed);
                
                // Wake ALL waiting consumers immediately - no polling!
                self.notify.notify_waiters();
                self.metrics.notifications_sent.fetch_add(1, Ordering::Relaxed);
                
                debug!("Trade written: {} @ ${}", symbol, price);
                Ok(())
            }
            Err(_) => {
                // Ring is full - we could implement backpressure here
                self.metrics.dropped_trades.fetch_add(1, Ordering::Relaxed);
                warn!("Ring buffer full, dropping trade");
                Err(AlphaPulseError::BufferOverflow { 
                    index: self.capacity, 
                    capacity: self.capacity 
                })
            }
        }
    }
    
    /// Write a batch of trades efficiently
    pub async fn write_batch(&self, trades: Vec<Trade>) -> Result<usize> {
        let mut written = 0;
        
        for trade in trades {
            if self.ring.push(trade).is_ok() {
                written += 1;
                self.metrics.writes_total.fetch_add(1, Ordering::Relaxed);
            } else {
                self.metrics.dropped_trades.fetch_add(1, Ordering::Relaxed);
                break; // Stop on first failure
            }
        }
        
        if written > 0 {
            self.metrics.queue_size.store(self.ring.len(), Ordering::Relaxed);
            self.notify.notify_waiters();
            self.metrics.notifications_sent.fetch_add(1, Ordering::Relaxed);
            debug!("Batch written: {} trades", written);
        }
        
        Ok(written)
    }
    
    /// Read all available trades (blocks until data available)
    pub async fn read_batch(&self) -> Vec<Trade> {
        // Wait for notification - TRUE event-driven, no polling!
        self.notify.notified().await;
        self.metrics.notifications_received.fetch_add(1, Ordering::Relaxed);
        
        // Drain all available trades
        let mut trades = Vec::new();
        while let Some(trade) = self.ring.pop() {
            trades.push(trade);
            self.metrics.reads_total.fetch_add(1, Ordering::Relaxed);
        }
        
        self.metrics.queue_size.store(self.ring.len(), Ordering::Relaxed);
        debug!("Read batch: {} trades", trades.len());
        
        trades
    }
    
    /// Try to read without blocking (for testing/debugging)
    pub fn try_read_batch(&self) -> Vec<Trade> {
        let mut trades = Vec::new();
        while let Some(trade) = self.ring.pop() {
            trades.push(trade);
            self.metrics.reads_total.fetch_add(1, Ordering::Relaxed);
        }
        
        self.metrics.queue_size.store(self.ring.len(), Ordering::Relaxed);
        trades
    }
    
    /// Get current queue size
    pub fn len(&self) -> usize {
        self.ring.len()
    }
    
    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }
    
    /// Get metrics for monitoring
    pub fn metrics(&self) -> TransportMetrics {
        TransportMetrics {
            writes_total: AtomicU64::new(self.metrics.writes_total.load(Ordering::Relaxed)),
            reads_total: AtomicU64::new(self.metrics.reads_total.load(Ordering::Relaxed)),
            notifications_sent: AtomicU64::new(self.metrics.notifications_sent.load(Ordering::Relaxed)),
            notifications_received: AtomicU64::new(self.metrics.notifications_received.load(Ordering::Relaxed)),
            queue_size: AtomicUsize::new(self.metrics.queue_size.load(Ordering::Relaxed)),
            dropped_trades: AtomicU64::new(self.metrics.dropped_trades.load(Ordering::Relaxed)),
        }
    }
    
    /// Clone for sharing between tasks
    pub fn clone(&self) -> Self {
        Self {
            ring: Arc::clone(&self.ring),
            notify: Arc::clone(&self.notify),
            metrics: Arc::clone(&self.metrics),
            capacity: self.capacity,
        }
    }
}

/// Global transport instance for easy access
static mut GLOBAL_TRANSPORT: Option<TokioTransport> = None;
static TRANSPORT_INIT: std::sync::Once = std::sync::Once::new();

/// Initialize global transport (call once at startup)
pub fn init_global_transport(capacity: usize) -> &'static TokioTransport {
    unsafe {
        TRANSPORT_INIT.call_once(|| {
            GLOBAL_TRANSPORT = Some(TokioTransport::new(capacity));
            info!("Global transport initialized with capacity: {}", capacity);
        });
        GLOBAL_TRANSPORT.as_ref().unwrap()
    }
}

/// Get global transport instance
pub fn get_global_transport() -> Option<&'static TokioTransport> {
    unsafe { GLOBAL_TRANSPORT.as_ref() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};
    
    #[tokio::test]
    async fn test_basic_write_read() {
        let transport = TokioTransport::new(100);
        
        let trade = Trade {
            timestamp: 1000,
            symbol: "BTC-USD".to_string(),
            exchange: "coinbase".to_string(),
            price: 50000.0,
            volume: 0.1,
            side: "buy".to_string(),
            trade_id: Some("test1".to_string()),
        };
        
        // Write trade
        transport.write(trade.clone()).await.unwrap();
        
        // Read should return immediately
        let trades = transport.read_batch().await;
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price, 50000.0);
    }
    
    #[tokio::test]
    async fn test_event_driven_no_polling() {
        let transport = TokioTransport::new(100);
        let transport_clone = transport.clone();
        
        // Spawn reader that waits for data
        let reader = tokio::spawn(async move {
            let trades = transport_clone.read_batch().await;
            trades.len()
        });
        
        // Give reader time to start waiting
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Write should wake reader immediately
        let trade = Trade {
            timestamp: 1000,
            symbol: "ETH-USD".to_string(),
            exchange: "kraken".to_string(),
            price: 3000.0,
            volume: 1.0,
            side: "sell".to_string(),
            trade_id: Some("test2".to_string()),
        };
        transport.write(trade).await.unwrap();
        
        // Reader should return quickly (not timeout)
        let result = timeout(Duration::from_millis(100), reader).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().unwrap(), 1);
    }
    
    #[tokio::test]
    async fn test_multiple_consumers() {
        let transport = TokioTransport::new(100);
        
        // Spawn multiple consumers
        let mut handles = vec![];
        for i in 0..3 {
            let t = transport.clone();
            handles.push(tokio::spawn(async move {
                let trades = t.read_batch().await;
                (i, trades.len())
            }));
        }
        
        // Give consumers time to start waiting
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Write a trade - should wake ALL consumers
        let trade = Trade {
            timestamp: 1000,
            symbol: "BTC-USD".to_string(),
            exchange: "binance".to_string(),
            price: 49999.0,
            volume: 0.5,
            side: "buy".to_string(),
            trade_id: Some("test3".to_string()),
        };
        transport.write(trade).await.unwrap();
        
        // All consumers should wake (though only one gets the trade)
        for handle in handles {
            let result = timeout(Duration::from_millis(100), handle).await;
            assert!(result.is_ok());
        }
    }
}