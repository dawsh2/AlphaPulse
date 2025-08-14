// Dedicated thread for shared memory reading to avoid SIGBUS in async context
use alphapulse_common::{
    shared_memory::{SharedMemoryReader, OrderBookDeltaReader, SharedTrade, SharedOrderBookDelta},
    types::Trade,
};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tracing::{info, debug, warn};

pub enum ReaderCommand {
    Stop,
}

pub enum ReaderData {
    Trade(Trade),
    Delta(OrderBookDelta),
}

// Dedicated thread for reading trades from shared memory
pub fn spawn_trade_reader_thread(path: &str, reader_id: usize) -> mpsc::Receiver<ReaderData> {
    let (tx, rx) = mpsc::channel();
    let path = path.to_string();
    
    thread::spawn(move || {
        info!("📊 Starting dedicated trade reader thread");
        
        // Create reader in the thread context (not async)
        let mut reader = match SharedMemoryReader::open(&path, reader_id) {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to open trade reader: {:?}", e);
                return;
            }
        };
        
        loop {
            // Read trades directly in thread context
            let trades = reader.read_trades();
            
            if !trades.is_empty() {
                for shared_trade in trades {
                    let trade = convert_shared_trade_to_trade(&shared_trade);
                    if tx.send(ReaderData::Trade(trade)).is_err() {
                        // Channel closed, exit thread
                        info!("Trade reader channel closed, exiting thread");
                        return;
                    }
                }
            } else {
                // No data, sleep briefly
                thread::sleep(Duration::from_micros(100));
            }
        }
    });
    
    rx
}

// Dedicated thread for reading orderbook deltas from shared memory
pub fn spawn_delta_reader_thread(path: &str, reader_id: usize, exchange: &str) -> mpsc::Receiver<ReaderData> {
    let (tx, rx) = mpsc::channel();
    let path = path.to_string();
    let exchange = exchange.to_string();
    
    thread::spawn(move || {
        info!("📊 Starting dedicated delta reader thread for {}", exchange);
        
        // Create reader in the thread context (not async)
        let mut reader = match OrderBookDeltaReader::open(&path, reader_id) {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to open delta reader for {}: {:?}", exchange, e);
                return;
            }
        };
        
        loop {
            // Read deltas directly in thread context
            let deltas = reader.read_deltas();
            
            if !deltas.is_empty() {
                for shared_delta in deltas {
                    let delta = convert_shared_delta_to_delta(&shared_delta, &exchange);
                    if tx.send(ReaderData::Delta(delta)).is_err() {
                        // Channel closed, exit thread
                        info!("{} delta reader channel closed, exiting thread", exchange);
                        return;
                    }
                }
            } else {
                // No data, sleep briefly
                thread::sleep(Duration::from_micros(100));
            }
        }
    });
    
    rx
}

fn convert_shared_trade_to_trade(shared: &SharedTrade) -> Trade {
    Trade {
        timestamp: shared.timestamp_ns,
        symbol: shared.symbol_str(),
        exchange: shared.exchange_str(),
        price: shared.price,
        volume: shared.volume,
        side: if shared.side == 0 { "buy".to_string() } else { "sell".to_string() },
        trade_id: String::from_utf8_lossy(&shared.trade_id)
            .trim_end_matches('\0')
            .to_string(),
    }
}

fn convert_shared_delta_to_delta(shared: &SharedOrderBookDelta, exchange: &str) -> OrderBookDelta {
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