# AlphaPulse Collector Development Guide

## Overview

This guide provides standardized patterns and best practices for developing market data collectors in the AlphaPulse ultra-low latency trading system. All collectors must implement consistent interfaces, error handling, and performance optimizations to maintain the system's sub-10μs latency requirements.

## 🏗️ Architecture Principles

### Core Requirements
1. **Ultra-Low Latency**: Sub-10μs shared memory operations
2. **Delta Compression**: 99.975% bandwidth reduction through orderbook deltas
3. **Memory Safety**: Comprehensive bounds checking and error handling
4. **Exchange Agnostic**: Consistent interface across all exchanges
5. **Production Ready**: Robust error handling and reconnection logic

### Data Flow
```
Exchange WebSocket → Collector → OrderBookTracker → Delta Compression → Shared Memory → WebSocket Server → Clients
```

## 📋 Collector Interface

### Required Trait Implementation

All collectors must implement the `MarketDataCollector` trait:

```rust
#[async_trait]
pub trait MarketDataCollector {
    async fn start(&self, tx: mpsc::Sender<Trade>) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    fn is_healthy(&self) -> bool;
    fn exchange_name(&self) -> &str;
    fn symbols(&self) -> &[String];
}
```

### Required Imports

```rust
use alphapulse_common::{
    Result, Trade, MetricsCollector,
    OrderBookUpdate, OrderBookTracker, 
    OrderBookSnapshot, OrderBookDelta,
    shared_memory::{OrderBookDeltaWriter, SharedOrderBookDelta}
};
use crate::collector_trait::MarketDataCollector;
use std::collections::HashMap;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{info, warn, error, debug};
use url::Url;
```

## 🏛️ Collector Structure Template

### Basic Structure

```rust
pub struct {Exchange}Collector {
    // Required fields
    symbols: Vec<String>,
    ws_url: String,
    healthy: Arc<AtomicBool>,
    metrics: Arc<MetricsCollector>,
    
    // OrderBook support fields
    orderbook_tx: Option<mpsc::Sender<OrderBookUpdate>>,
    delta_tx: Option<mpsc::Sender<OrderBookDelta>>,
    orderbooks: Arc<tokio::sync::RwLock<HashMap<String, OrderBookUpdate>>>,
    orderbook_tracker: OrderBookTracker,
    delta_writer: Option<Arc<tokio::sync::Mutex<OrderBookDeltaWriter>>>,
}
```

### Constructor Pattern

```rust
impl {Exchange}Collector {
    pub fn new(symbols: Vec<String>) -> Self {
        // Convert symbols to exchange format
        let exchange_symbols: Vec<String> = symbols
            .iter()
            .map(|s| Self::convert_symbol_to_{exchange}(s))
            .collect();
            
        Self {
            symbols: exchange_symbols,
            ws_url: "{EXCHANGE_WEBSOCKET_URL}".to_string(),
            healthy: Arc::new(AtomicBool::new(false)),
            metrics: Arc::new(MetricsCollector::new()),
            orderbook_tx: None,
            delta_tx: None,
            orderbooks: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            orderbook_tracker: OrderBookTracker::new(50), // Track top 50 levels
            delta_writer: None,
        }
    }
}
```

### Required Builder Methods

```rust
pub fn with_orderbook_sender(mut self, tx: mpsc::Sender<OrderBookUpdate>) -> Self {
    self.orderbook_tx = Some(tx);
    self
}

pub fn with_delta_sender(mut self, tx: mpsc::Sender<OrderBookDelta>) -> Self {
    self.delta_tx = Some(tx);
    self
}

pub fn with_shared_memory_writer(mut self) -> Result<Self> {
    let writer = OrderBookDeltaWriter::create(
        "/tmp/alphapulse_shm/{exchange}_orderbook_deltas", 
        10000 // 10k capacity
    )?;
    self.delta_writer = Some(Arc::new(tokio::sync::Mutex::new(writer)));
    Ok(self)
}
```

## 🔄 Symbol Conversion

### Required Methods

Every collector must implement symbol conversion:

```rust
fn convert_symbol_to_{exchange}(symbol: &str) -> String {
    // Convert standard format to exchange format
    match symbol {
        "BTC-USD" | "BTC/USD" => "{EXCHANGE_BTC_FORMAT}".to_string(),
        "ETH-USD" | "ETH/USD" => "{EXCHANGE_ETH_FORMAT}".to_string(),
        s => {
            // Generic conversion logic
            s.replace("-", "/") // or exchange-specific logic
        }
    }
}

fn convert_symbol_from_{exchange}(exchange_symbol: &str) -> String {
    // Convert exchange format back to standard format
    match exchange_symbol {
        "{EXCHANGE_BTC_FORMAT}" => "BTC/USD".to_string(),
        "{EXCHANGE_ETH_FORMAT}" => "ETH/USD".to_string(),
        s => s.to_string(),
    }
}
```

## 📡 WebSocket Implementation

### Subscription Pattern

```rust
async fn run_collector(&self, tx: &mpsc::Sender<Trade>) -> Result<()> {
    let url = Url::parse(&self.ws_url)?;
    let (ws_stream, _) = connect_async(url).await?;
    let (mut write, mut read) = ws_stream.split();
    
    // Subscribe to trade messages
    let trade_subscribe_msg = json!({
        // Exchange-specific trade subscription format
    });
    write.send(Message::Text(trade_subscribe_msg.to_string())).await?;
    
    // Subscribe to orderbook if handlers exist
    if self.orderbook_tx.is_some() || self.delta_tx.is_some() || self.delta_writer.is_some() {
        let book_subscribe_msg = json!({
            // Exchange-specific orderbook subscription format
        });
        write.send(Message::Text(book_subscribe_msg.to_string())).await?;
        info!("Subscribed to {exchange} orderbooks for symbols: {:?}", self.symbols);
    }
    
    self.healthy.store(true, Ordering::Relaxed);
    self.metrics.record_websocket_connection_status("{exchange}", true);
    
    // Process incoming messages
    while let Some(msg_result) = read.next().await {
        match msg_result {
            Ok(msg) => {
                if let Err(e) = self.handle_message(msg, tx).await {
                    error!("Error handling {exchange} message: {}", e);
                }
            }
            Err(e) => {
                error!("WebSocket error from {exchange}: {}", e);
                return Err(e.into());
            }
        }
    }
    
    self.healthy.store(false, Ordering::Relaxed);
    self.metrics.record_websocket_connection_status("{exchange}", false);
    
    Ok(())
}
```

## 📊 Message Handling

### Trade Message Pattern

```rust
async fn handle_message(&self, msg: Message, tx: &mpsc::Sender<Trade>) -> Result<()> {
    match msg {
        Message::Text(text) => {
            debug!("Received {exchange} message: {}", text);
            
            // Try parsing as trade message
            if let Ok(trade_msg) = serde_json::from_str::<{Exchange}TradeMessage>(&text) {
                if trade_msg.{type_field} == "{TRADE_TYPE}" {
                    let trade = Trade::from(trade_msg.clone());
                    
                    // Send to processing channel
                    if let Err(e) = tx.send(trade).await {
                        warn!("Failed to send trade to channel: {}", e);
                        return Ok(()); // Don't crash on channel errors
                    }
                    
                    // Record metrics
                    self.metrics.record_trade_processed("{exchange}", &trade_msg.{symbol_field});
                    self.metrics.record_websocket_message("{exchange}", "trade");
                }
            }
            // Try parsing as orderbook message
            else if text.contains("{ORDERBOOK_IDENTIFIER}") {
                if let Ok(orderbook_msg) = serde_json::from_str::<serde_json::Value>(&text) {
                    if self.orderbook_tx.is_some() || self.delta_tx.is_some() || self.delta_writer.is_some() {
                        self.handle_{exchange}_orderbook(orderbook_msg).await?;
                        self.metrics.record_websocket_message("{exchange}", "orderbook_update");
                    }
                }
            }
            else {
                // Handle subscription confirmations and other messages
                if text.contains("{SUBSCRIPTION_CONFIRMATION}") {
                    info!("{Exchange} subscription confirmed");
                }
            }
        }
        Message::Ping(_) => {
            debug!("Received {exchange} ping, sending pong");
            // WebSocket library handles pong automatically
        }
        Message::Pong(_) => {
            debug!("Received {exchange} pong");
        }
        Message::Close(_) => {
            warn!("{Exchange} WebSocket closed");
            self.healthy.store(false, Ordering::Relaxed);
            self.metrics.record_websocket_connection_status("{exchange}", false);
        }
        Message::Binary(_) => {
            warn!("Received unexpected binary message from {exchange}");
        }
        Message::Frame(_) => {
            debug!("Received frame message from {exchange}");
        }
    }
    
    Ok(())
}
```

## 📖 OrderBook Processing

### Required OrderBook Handler

```rust
async fn handle_{exchange}_orderbook(&self, msg: serde_json::Value) -> Result<()> {
    // Parse exchange-specific orderbook format
    if let Some(symbol_data) = msg.get("{SYMBOL_FIELD}").and_then(|s| s.as_str()) {
        let timestamp = chrono::Utc::now().timestamp() as f64;
        
        // Parse bids and asks according to exchange format
        let mut bids = Vec::new();
        let mut asks = Vec::new();
        
        // Exchange-specific parsing logic
        if let Some(bid_array) = msg.get("{BIDS_FIELD}").and_then(|b| b.as_array()) {
            for bid in bid_array.iter() {
                // Parse bid according to exchange format
                // Add to bids vector as [price, size]
            }
        }
        
        if let Some(ask_array) = msg.get("{ASKS_FIELD}").and_then(|a| a.as_array()) {
            for ask in ask_array.iter() {
                // Parse ask according to exchange format
                // Add to asks vector as [price, size]
            }
        }
        
        // Convert to standard symbol format
        let standard_symbol = Self::convert_symbol_from_{exchange}(symbol_data);
        
        // Create OrderBookSnapshot for delta tracking
        let snapshot = OrderBookSnapshot {
            symbol: standard_symbol.clone(),
            exchange: "{exchange}".to_string(),
            version: chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default() as u64,
            timestamp,
            bids: bids.clone(),
            asks: asks.clone(),
        };
        
        // Update OrderBookTracker with snapshot
        self.orderbook_tracker.update_snapshot(&standard_symbol, "{exchange}", snapshot.clone()).await;
        
        // Compute delta if this is an update (not first snapshot)
        if let Some(delta) = self.orderbook_tracker.compute_delta(&snapshot, &standard_symbol).await {
            // Send delta update via channel
            if let Some(tx) = &self.delta_tx {
                if let Err(e) = tx.send(delta.clone()).await {
                    warn!("Failed to send {exchange} orderbook delta: {}", e);
                }
            }
            
            // Write delta to shared memory for ultra-low latency access
            if let Some(writer) = &self.delta_writer {
                let shared_delta = self.convert_to_shared_delta(&delta);
                let mut writer_guard = writer.lock().await;
                if let Err(e) = writer_guard.write_delta(&shared_delta) {
                    warn!("Failed to write {exchange} delta to shared memory: {}", e);
                } else {
                    info!(
                        "🚀 {Exchange} delta written to shared memory for {}: {} bid changes, {} ask changes (vs {} full levels)", 
                        standard_symbol,
                        delta.bid_changes.len(),
                        delta.ask_changes.len(),
                        bids.len() + asks.len()
                    );
                }
            }
        }
        
        // Create legacy OrderBookUpdate for backward compatibility
        let orderbook = OrderBookUpdate {
            symbol: standard_symbol.clone(),
            exchange: "{exchange}".to_string(),
            timestamp,
            bids,
            asks,
            sequence: None,
            update_type: Some("snapshot".to_string()),
        };
        
        // Store in local cache
        let mut orderbooks = self.orderbooks.write().await;
        orderbooks.insert(standard_symbol.clone(), orderbook.clone());
        
        // Send to channel if available (backward compatibility)
        if let Some(tx) = &self.orderbook_tx {
            if let Err(e) = tx.send(orderbook).await {
                warn!("Failed to send {exchange} orderbook update: {}", e);
            }
        }
        
        info!("Processed orderbook for {exchange} symbol: {}", standard_symbol);
    }
    
    Ok(())
}
```

### Required Delta Conversion

```rust
fn convert_to_shared_delta(&self, delta: &OrderBookDelta) -> SharedOrderBookDelta {
    let timestamp_ns = (delta.timestamp * 1_000_000_000.0) as u64;
    let mut shared_delta = SharedOrderBookDelta::new(
        timestamp_ns,
        &delta.symbol,
        &delta.exchange,
        delta.version,
        delta.prev_version
    );
    
    // Add bid changes
    for change in &delta.bid_changes {
        if !shared_delta.add_change(change.price, change.volume, false, 0) {
            warn!("Delta buffer full, some bid changes dropped");
            break;
        }
    }
    
    // Add ask changes
    for change in &delta.ask_changes {
        if !shared_delta.add_change(change.price, change.volume, true, 0) {
            warn!("Delta buffer full, some ask changes dropped");
            break;
        }
    }
    
    shared_delta
}
```

## 🔄 Trait Implementation

### MarketDataCollector Implementation

```rust
#[async_trait]
impl MarketDataCollector for {Exchange}Collector {
    async fn start(&self, tx: mpsc::Sender<Trade>) -> Result<()> {
        loop {
            match self.run_collector(&tx).await {
                Ok(_) => {
                    info!("{Exchange} collector completed normally");
                    break;
                }
                Err(e) => {
                    error!("{Exchange} collector error: {}", e);
                    self.healthy.store(false, Ordering::Relaxed);
                    self.metrics.record_websocket_connection_status("{exchange}", false);
                    self.metrics.record_websocket_reconnection("{exchange}");
                    
                    // Wait before reconnecting
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    info!("Attempting to reconnect to {exchange}...");
                }
            }
        }
        
        Ok(())
    }
    
    async fn stop(&self) -> Result<()> {
        info!("Stopping {exchange} collector");
        self.healthy.store(false, Ordering::Relaxed);
        Ok(())
    }
    
    fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }
    
    fn exchange_name(&self) -> &str {
        "{exchange}"
    }
    
    fn symbols(&self) -> &[String] {
        &self.symbols
    }
}
```

## 📝 Data Structures

### Exchange-Specific Trade Message

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct {Exchange}TradeMessage {
    // Map exchange fields to standard names
    #[serde(rename = "{EXCHANGE_TYPE_FIELD}")]
    pub message_type: String,
    
    #[serde(rename = "{EXCHANGE_SYMBOL_FIELD}")]
    pub symbol: String,
    
    #[serde(rename = "{EXCHANGE_PRICE_FIELD}")]
    pub price: String, // Usually string in JSON
    
    #[serde(rename = "{EXCHANGE_VOLUME_FIELD}")]
    pub volume: String,
    
    #[serde(rename = "{EXCHANGE_TIMESTAMP_FIELD}")]
    pub timestamp: f64,
    
    #[serde(rename = "{EXCHANGE_SIDE_FIELD}")]
    pub side: String,
    
    // Add other exchange-specific fields as needed
}

impl From<{Exchange}TradeMessage> for Trade {
    fn from(msg: {Exchange}TradeMessage) -> Self {
        Trade {
            timestamp: msg.timestamp, // Convert if needed
            symbol: {Exchange}Collector::convert_symbol_from_{exchange}(&msg.symbol),
            exchange: "{exchange}".to_string(),
            price: msg.price.parse().unwrap_or(0.0),
            volume: msg.volume.parse().unwrap_or(0.0),
            side: Some(msg.side), // Convert if needed
            trade_id: None, // Add if available
        }
    }
}
```

## 🚀 Performance Requirements

### Memory Management
- Use `Arc<AtomicBool>` for health status
- Use `Arc<tokio::sync::RwLock<HashMap>>` for orderbook cache
- Minimize allocations in hot paths
- Fixed-size SharedOrderBookDelta for zero-copy operations

### Error Handling
- Never panic in production code
- Use `Result<()>` return types
- Log errors with appropriate levels (warn/error)
- Graceful degradation on channel send failures

### Metrics
- Record WebSocket connection status
- Track message processing rates
- Monitor shared memory latency
- Report delta compression ratios

## 📋 Integration Checklist

### Before Implementation
- [ ] Study exchange WebSocket API documentation
- [ ] Identify trade and orderbook message formats
- [ ] Determine subscription mechanisms
- [ ] Plan symbol conversion logic

### During Implementation
- [ ] Implement all required trait methods
- [ ] Add comprehensive error handling
- [ ] Include proper logging statements
- [ ] Test symbol conversion both directions
- [ ] Verify orderbook parsing accuracy

### After Implementation
- [ ] Test with live exchange data
- [ ] Verify delta compression works
- [ ] Confirm shared memory integration
- [ ] Validate WebSocket reconnection logic
- [ ] Measure performance metrics

### WebSocket Server Integration
- [ ] Add delta reader to `websocket-server/src/main.rs`
- [ ] Use unique reader ID (increment from existing)
- [ ] Update shared memory path pattern
- [ ] Add exchange-specific error handling

## 🔧 Testing Guidelines

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_symbol_conversion() {
        assert_eq!(
            {Exchange}Collector::convert_symbol_to_{exchange}("BTC-USD"),
            "{EXPECTED_EXCHANGE_FORMAT}"
        );
        assert_eq!(
            {Exchange}Collector::convert_symbol_from_{exchange}("{EXCHANGE_FORMAT}"),
            "BTC/USD"
        );
    }
    
    #[tokio::test]
    async fn test_orderbook_processing() {
        let collector = {Exchange}Collector::new(vec!["BTC-USD".to_string()]);
        // Test orderbook message parsing
    }
}
```

### Integration Tests
- Test live WebSocket connection
- Verify trade data accuracy
- Validate orderbook reconstruction
- Confirm delta compression ratios

## 📖 Example Implementations

Reference implementations:
- **Coinbase**: `collectors/src/coinbase.rs` (Full featured)
- **Kraken**: `collectors/src/kraken.rs` (Recently completed)
- **Binance.US**: `collectors/src/binance_us.rs` (Recently completed)

## 🎯 Performance Targets

- **Latency**: <10μs shared memory operations
- **Compression**: >99% bandwidth reduction through deltas
- **Throughput**: Handle 10k+ messages/second per exchange
- **Memory**: <100MB RAM usage per collector
- **Reliability**: 99.9% uptime with automatic reconnection

## 📚 Additional Resources

- [WebSocket Protocol RFC 6455](https://tools.ietf.org/html/rfc6455)
- [Exchange API Documentation Links]
- [AlphaPulse Architecture Overview](./rust-migration.md)
- [Shared Memory Implementation](./common/src/shared_memory.rs)
- [OrderBook Delta Compression](./common/src/orderbook_delta.rs)

---

Following this guide ensures all collectors maintain the ultra-low latency performance and consistency required for production trading systems.