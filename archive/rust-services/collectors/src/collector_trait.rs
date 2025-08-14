// Common trait for all market data collectors
use alphapulse_common::{Result, Trade, OrderBookUpdate, CollectorConfig};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;

#[async_trait]
pub trait MarketDataCollector: Send + Sync {
    /// Start collecting data and send trades to the channel
    async fn start(&self, tx: mpsc::Sender<Trade>) -> Result<()>;
    
    /// Stop the collector gracefully
    async fn stop(&self) -> Result<()>;
    
    /// Get collector health status
    fn is_healthy(&self) -> bool;
    
    /// Get exchange name
    fn exchange_name(&self) -> &str;
    
    /// Get subscribed symbols
    fn symbols(&self) -> &[String];
}

pub enum MarketDataMessage {
    Trade(Trade),
    OrderBookUpdate(OrderBookUpdate),
}

pub struct CollectorManager {
    collectors: Vec<Arc<dyn MarketDataCollector>>,
    trade_tx: mpsc::Sender<Trade>,
    trade_rx: mpsc::Receiver<Trade>,
    orderbook_tx: mpsc::Sender<OrderBookUpdate>,
    orderbook_rx: mpsc::Receiver<OrderBookUpdate>,
}

impl CollectorManager {
    pub fn new(buffer_size: usize) -> Self {
        let (trade_tx, trade_rx) = mpsc::channel(buffer_size);
        let (orderbook_tx, orderbook_rx) = mpsc::channel(buffer_size);
        
        Self {
            collectors: Vec::new(),
            trade_tx,
            trade_rx,
            orderbook_tx,
            orderbook_rx,
        }
    }
    
    pub fn add_collector(&mut self, collector: Arc<dyn MarketDataCollector>) {
        self.collectors.push(collector);
    }
    
    pub async fn start_all(&self) -> Result<()> {
        tracing::info!("Starting {} collectors", self.collectors.len());
        
        for collector in &self.collectors {
            let tx = self.trade_tx.clone();
            let collector = collector.clone();
            
            tokio::spawn(async move {
                if let Err(e) = collector.start(tx).await {
                    tracing::error!("Collector {} failed: {}", collector.exchange_name(), e);
                }
            });
        }
        
        Ok(())
    }
    
    pub fn get_trade_receiver(&mut self) -> mpsc::Receiver<Trade> {
        // Don't replace the sender! Just return the receiver.
        // The sender is already being used by collectors.
        let (_, new_rx) = mpsc::channel(1000);
        std::mem::replace(&mut self.trade_rx, new_rx)
    }
    
    pub fn get_orderbook_receiver(&mut self) -> mpsc::Receiver<OrderBookUpdate> {
        let (_, new_rx) = mpsc::channel(1000);
        std::mem::replace(&mut self.orderbook_rx, new_rx)
    }
    
    pub fn get_orderbook_sender(&self) -> mpsc::Sender<OrderBookUpdate> {
        self.orderbook_tx.clone()
    }
    
    pub fn health_status(&self) -> Vec<(String, bool)> {
        self.collectors
            .iter()
            .map(|c| (c.exchange_name().to_string(), c.is_healthy()))
            .collect()
    }
}