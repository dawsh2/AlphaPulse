// Dedicated thread pool for shared memory reading
// This is the CORRECT architecture for memory-mapped I/O with async servers

use alphapulse_common::{
    shared_memory::{SharedMemoryReader, OrderBookDeltaReader, SharedTrade, SharedOrderBookDelta},
    types::{Trade, OrderBookUpdate},
    Result, AlphaPulseError,
};
use tokio::sync::mpsc;
use std::thread;
use std::time::Duration;
use tracing::{info, debug, warn, error};

// Message types that readers send to async context
#[derive(Debug, Clone)]
pub enum MarketDataMessage {
    Trade(Trade),
    OrderBookDelta(OrderBookDelta),
    Stats(ReaderStats),
}

#[derive(Debug, Clone)]
pub struct OrderBookDelta {
    pub timestamp: u64,
    pub symbol: String,
    pub exchange: String,
    pub version: u64,
    pub prev_version: u64,
    pub changes: Vec<PriceChange>,
}

#[derive(Debug, Clone)]
pub struct PriceChange {
    pub price: f64,
    pub volume: f64,
    pub side: String,
    pub action: String,
}

#[derive(Debug, Clone)]
pub struct ReaderStats {
    pub reader_type: String,
    pub messages_read: u64,
    pub lag: u64,
    pub read_latency_us: f64,
}

/// Spawns a dedicated OS thread for reading trades from shared memory
pub fn spawn_trade_reader(
    path: &str,
    reader_id: usize,
    buffer_size: usize,
) -> mpsc::Receiver<MarketDataMessage> {
    let (tx, rx) = mpsc::channel(buffer_size);
    let path = path.to_string();
    
    thread::Builder::new()
        .name(format!("shm-trade-reader-{}", reader_id))
        .spawn(move || {
            info!("🚀 Starting dedicated trade reader thread (id={})", reader_id);
            
            // Create reader in thread context (not async)
            let mut reader = match SharedMemoryReader::open(&path, reader_id) {
                Ok(r) => r,
                Err(e) => {
                    error!("Failed to open trade reader: {:?}", e);
                    return;
                }
            };
            
            let mut messages_read = 0u64;
            let mut last_stats_time = std::time::Instant::now();
            
            loop {
                let start = std::time::Instant::now();
                
                // Read trades from shared memory (blocking operation)
                let trades = reader.read_trades();
                
                if !trades.is_empty() {
                    let read_latency_us = start.elapsed().as_nanos() as f64 / 1000.0;
                    
                    for shared_trade in trades {
                        let trade = convert_shared_trade(&shared_trade);
                        
                        // Send to async context via channel
                        if tx.blocking_send(MarketDataMessage::Trade(trade)).is_err() {
                            info!("Trade reader channel closed, exiting thread");
                            return;
                        }
                        messages_read += 1;
                    }
                    
                    debug!("Read {} trades in {:.1}μs", trades.len(), read_latency_us);
                }
                
                // Send stats periodically
                if last_stats_time.elapsed() > Duration::from_secs(10) {
                    let stats = ReaderStats {
                        reader_type: "trades".to_string(),
                        messages_read,
                        lag: reader.get_lag(),
                        read_latency_us: start.elapsed().as_nanos() as f64 / 1000.0,
                    };
                    let _ = tx.blocking_send(MarketDataMessage::Stats(stats));
                    last_stats_time = std::time::Instant::now();
                }
                
                // Small sleep if no data to prevent CPU spinning
                if trades.is_empty() {
                    thread::sleep(Duration::from_micros(100));
                }
            }
        })
        .expect("Failed to spawn trade reader thread");
    
    rx
}

/// Spawns a dedicated OS thread for reading orderbook deltas from shared memory
pub fn spawn_delta_reader(
    path: &str,
    reader_id: usize,
    exchange: &str,
    buffer_size: usize,
) -> mpsc::Receiver<MarketDataMessage> {
    let (tx, rx) = mpsc::channel(buffer_size);
    let path = path.to_string();
    let exchange_str = exchange.to_string();
    let thread_name = format!("shm-delta-reader-{}-{}", exchange, reader_id);
    
    thread::Builder::new()
        .name(thread_name.clone())
        .spawn(move || {
            info!("🚀 Starting dedicated {} delta reader thread (id={})", exchange_str, reader_id);
            
            // Create reader in thread context (not async)
            let mut reader = match OrderBookDeltaReader::open(&path, reader_id) {
                Ok(r) => r,
                Err(e) => {
                    error!("Failed to open {} delta reader: {:?}", exchange_str, e);
                    return;
                }
            };
            
            let mut messages_read = 0u64;
            let mut last_stats_time = std::time::Instant::now();
            
            loop {
                let start = std::time::Instant::now();
                
                // Read deltas from shared memory (blocking operation)
                let deltas = reader.read_deltas();
                
                if !deltas.is_empty() {
                    let read_latency_us = start.elapsed().as_nanos() as f64 / 1000.0;
                    
                    for shared_delta in deltas {
                        let delta = convert_shared_delta(&shared_delta, &exchange_str);
                        
                        // Send to async context via channel
                        if tx.blocking_send(MarketDataMessage::OrderBookDelta(delta)).is_err() {
                            info!("{} delta reader channel closed, exiting thread", exchange_str);
                            return;
                        }
                        messages_read += 1;
                    }
                    
                    debug!("Read {} {} deltas in {:.1}μs", deltas.len(), exchange_str, read_latency_us);
                }
                
                // Send stats periodically
                if last_stats_time.elapsed() > Duration::from_secs(10) {
                    let stats = ReaderStats {
                        reader_type: format!("{}_deltas", exchange_str),
                        messages_read,
                        lag: reader.get_lag(),
                        read_latency_us: start.elapsed().as_nanos() as f64 / 1000.0,
                    };
                    let _ = tx.blocking_send(MarketDataMessage::Stats(stats));
                    last_stats_time = std::time::Instant::now();
                }
                
                // Small sleep if no data to prevent CPU spinning
                if deltas.is_empty() {
                    thread::sleep(Duration::from_micros(100));
                }
            }
        })
        .expect("Failed to spawn delta reader thread");
    
    rx
}

fn convert_shared_trade(shared: &SharedTrade) -> Trade {
    Trade {
        timestamp: shared.timestamp_ns as f64 / 1_000_000_000.0, // Convert to seconds
        symbol: shared.symbol_str(),
        exchange: shared.exchange_str(),
        price: shared.price,
        volume: shared.volume,
        side: Some(if shared.side == 0 { "buy".to_string() } else { "sell".to_string() }),
        trade_id: Some(
            String::from_utf8_lossy(&shared.trade_id)
                .trim_end_matches('\0')
                .to_string()
        ),
    }
}

fn convert_shared_delta(shared: &SharedOrderBookDelta, exchange: &str) -> OrderBookDelta {
    let mut changes = Vec::new();
    
    for i in 0..shared.change_count as usize {
        let change = &shared.changes[i];
        let is_ask = (change.side_and_action & 0x80) != 0;
        let action_code = change.side_and_action & 0x7F;
        
        let action = match action_code {
            0 => "add",
            1 => "update",
            2 => "remove",
            _ => "unknown",
        };
        
        changes.push(PriceChange {
            price: change.price as f64,
            volume: change.volume as f64,
            side: if is_ask { "ask".to_string() } else { "bid".to_string() },
            action: action.to_string(),
        });
    }
    
    OrderBookDelta {
        timestamp: shared.timestamp_ns,
        symbol: shared.symbol_str(),
        exchange: exchange.to_string(),
        version: shared.version,
        prev_version: shared.prev_version,
        changes,
    }
}

/// Manager for all shared memory readers
pub struct SharedMemoryReaderPool {
    receivers: Vec<mpsc::Receiver<MarketDataMessage>>,
}

impl SharedMemoryReaderPool {
    pub fn new() -> Self {
        let mut receivers = Vec::new();
        
        // Spawn trade reader
        receivers.push(spawn_trade_reader(
            "/tmp/alphapulse_shm/trades",
            0,
            10000, // Buffer size
        ));
        
        // Spawn orderbook delta readers for each exchange
        receivers.push(spawn_delta_reader(
            "/tmp/alphapulse_shm/orderbook_deltas",
            1,
            "coinbase",
            10000,
        ));
        
        receivers.push(spawn_delta_reader(
            "/tmp/alphapulse_shm/kraken_orderbook_deltas",
            2,
            "kraken",
            10000,
        ));
        
        receivers.push(spawn_delta_reader(
            "/tmp/alphapulse_shm/binance_orderbook_deltas",
            3,
            "binance",
            10000,
        ));
        
        info!("✅ Shared memory reader pool initialized with {} readers", receivers.len());
        
        Self { receivers }
    }
    
    /// Consolidates all receivers into a single stream
    pub async fn into_stream(self) -> mpsc::Receiver<MarketDataMessage> {
        let (tx, rx) = mpsc::channel(50000);
        
        // Spawn tasks to forward from each reader
        for mut receiver in self.receivers {
            let tx = tx.clone();
            tokio::spawn(async move {
                while let Some(msg) = receiver.recv().await {
                    if tx.send(msg).await.is_err() {
                        break;
                    }
                }
            });
        }
        
        rx
    }
}