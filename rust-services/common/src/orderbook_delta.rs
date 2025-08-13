// Orderbook delta tracking and compression
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookSnapshot {
    pub symbol: String,
    pub exchange: String,
    pub version: u64,
    pub timestamp: f64,
    pub bids: Vec<[f64; 2]>, // [price, volume]
    pub asks: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookDelta {
    pub symbol: String,
    pub exchange: String,
    pub version: u64,
    pub prev_version: u64,
    pub timestamp: f64,
    pub bid_changes: Vec<PriceLevel>,
    pub ask_changes: Vec<PriceLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: f64,
    pub volume: f64,  // 0 means remove this level
    pub action: DeltaAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeltaAction {
    Add,
    Update,
    Remove,
}

pub struct OrderBookTracker {
    snapshots: Arc<RwLock<HashMap<String, OrderBookSnapshot>>>,
    version_counter: Arc<RwLock<HashMap<String, u64>>>,
    max_depth: usize,
}

impl OrderBookTracker {
    pub fn new(max_depth: usize) -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            version_counter: Arc::new(RwLock::new(HashMap::new())),
            max_depth,
        }
    }
    
    pub async fn update_snapshot(&self, symbol: &str, exchange: &str, snapshot: OrderBookSnapshot) {
        let key = format!("{}:{}", exchange, symbol);
        let mut snapshots = self.snapshots.write().await;
        let mut versions = self.version_counter.write().await;
        
        let version = versions.entry(key.clone()).or_insert(0);
        *version += 1;
        
        snapshots.insert(key, snapshot);
    }
    
    pub async fn compute_delta(&self, new_book: &OrderBookSnapshot, symbol: &str) -> Option<OrderBookDelta> {
        let key = format!("{}:{}", new_book.exchange, symbol);
        let snapshots = self.snapshots.read().await;
        
        if let Some(old_book) = snapshots.get(&key) {
            let mut bid_changes = Vec::new();
            let mut ask_changes = Vec::new();
            
            // Create HashMap for O(1) lookups - optimization for O(n²) to O(n)
            let old_bids_map: HashMap<u64, f64> = old_book.bids.iter()
                .map(|level| ((level[0] * 100000.0) as u64, level[1]))
                .collect();
            
            let old_asks_map: HashMap<u64, f64> = old_book.asks.iter()
                .map(|level| ((level[0] * 100000.0) as u64, level[1]))
                .collect();
            
            let new_bids_map: HashMap<u64, f64> = new_book.bids.iter()
                .map(|level| ((level[0] * 100000.0) as u64, level[1]))
                .collect();
            
            let new_asks_map: HashMap<u64, f64> = new_book.asks.iter()
                .map(|level| ((level[0] * 100000.0) as u64, level[1]))
                .collect();
            
            // Compare bids efficiently
            for new_bid in &new_book.bids {
                let price_key = (new_bid[0] * 100000.0) as u64;
                let new_volume = new_bid[1];
                
                match old_bids_map.get(&price_key) {
                    Some(&old_volume) if old_volume != new_volume => {
                        // Price exists but volume changed - Update
                        bid_changes.push(PriceLevel {
                            price: new_bid[0],
                            volume: new_volume,
                            action: DeltaAction::Update,
                        });
                    }
                    None => {
                        // New price level - Add
                        bid_changes.push(PriceLevel {
                            price: new_bid[0],
                            volume: new_volume,
                            action: DeltaAction::Add,
                        });
                    }
                    _ => {} // No change
                }
            }
            
            // Find removed bids
            for old_bid in &old_book.bids {
                let price_key = (old_bid[0] * 100000.0) as u64;
                if !new_bids_map.contains_key(&price_key) {
                    bid_changes.push(PriceLevel {
                        price: old_bid[0],
                        volume: 0.0,
                        action: DeltaAction::Remove,
                    });
                }
            }
            
            // Similar optimized logic for asks
            for new_ask in &new_book.asks {
                let price_key = (new_ask[0] * 100000.0) as u64;
                let new_volume = new_ask[1];
                
                match old_asks_map.get(&price_key) {
                    Some(&old_volume) if old_volume != new_volume => {
                        // Price exists but volume changed - Update
                        ask_changes.push(PriceLevel {
                            price: new_ask[0],
                            volume: new_volume,
                            action: DeltaAction::Update,
                        });
                    }
                    None => {
                        // New price level - Add
                        ask_changes.push(PriceLevel {
                            price: new_ask[0],
                            volume: new_volume,
                            action: DeltaAction::Add,
                        });
                    }
                    _ => {} // No change
                }
            }
            
            // Find removed asks
            for old_ask in &old_book.asks {
                let price_key = (old_ask[0] * 100000.0) as u64;
                if !new_asks_map.contains_key(&price_key) {
                    ask_changes.push(PriceLevel {
                        price: old_ask[0],
                        volume: 0.0,
                        action: DeltaAction::Remove,
                    });
                }
            }
            
            if !bid_changes.is_empty() || !ask_changes.is_empty() {
                return Some(OrderBookDelta {
                    symbol: symbol.to_string(),
                    exchange: new_book.exchange.clone(),
                    version: new_book.version,
                    prev_version: old_book.version,
                    timestamp: new_book.timestamp,
                    bid_changes,
                    ask_changes,
                });
            }
        }
        
        None
    }
}