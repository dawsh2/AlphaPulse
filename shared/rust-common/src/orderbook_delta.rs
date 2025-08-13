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
    
    pub async fn update_orderbook(
        &self,
        symbol: &str,
        exchange: &str,
        bids: Vec<[f64; 2]>,
        asks: Vec<[f64; 2]>,
        timestamp: f64,
    ) -> (Option<OrderBookSnapshot>, Option<OrderBookDelta>) {
        let key = format!("{}:{}", exchange, symbol);
        
        // Get or create version counter
        let mut versions = self.version_counter.write().await;
        let version = versions.entry(key.clone()).or_insert(0);
        *version += 1;
        let current_version = *version;
        drop(versions);
        
        // Limit depth
        let limited_bids: Vec<[f64; 2]> = bids.into_iter()
            .take(self.max_depth)
            .collect();
        let limited_asks: Vec<[f64; 2]> = asks.into_iter()
            .take(self.max_depth)
            .collect();
        
        // Get previous snapshot
        let mut snapshots = self.snapshots.write().await;
        let prev_snapshot = snapshots.get(&key).cloned();
        
        // Create new snapshot
        let new_snapshot = OrderBookSnapshot {
            symbol: symbol.to_string(),
            exchange: exchange.to_string(),
            version: current_version,
            timestamp,
            bids: limited_bids.clone(),
            asks: limited_asks.clone(),
        };
        
        // Calculate delta if we have a previous snapshot
        let delta = if let Some(prev) = &prev_snapshot {
            Some(calculate_delta(prev, &new_snapshot))
        } else {
            None
        };
        
        // Store new snapshot
        snapshots.insert(key, new_snapshot.clone());
        
        // Return snapshot only if it's the first one, otherwise return delta
        if prev_snapshot.is_none() {
            (Some(new_snapshot), None)
        } else {
            (None, delta)
        }
    }
    
    pub async fn get_snapshot(&self, symbol: &str, exchange: &str) -> Option<OrderBookSnapshot> {
        let key = format!("{}:{}", exchange, symbol);
        let snapshots = self.snapshots.read().await;
        snapshots.get(&key).cloned()
    }
}

fn calculate_delta(prev: &OrderBookSnapshot, new: &OrderBookSnapshot) -> OrderBookDelta {
    let bid_changes = calculate_level_changes(&prev.bids, &new.bids);
    let ask_changes = calculate_level_changes(&prev.asks, &new.asks);
    
    OrderBookDelta {
        symbol: new.symbol.clone(),
        exchange: new.exchange.clone(),
        version: new.version,
        prev_version: prev.version,
        timestamp: new.timestamp,
        bid_changes,
        ask_changes,
    }
}

fn calculate_level_changes(old: &[[f64; 2]], new: &[[f64; 2]]) -> Vec<PriceLevel> {
    let mut changes = Vec::new();
    
    // Create maps for easier comparison
    let mut old_map: HashMap<i64, f64> = HashMap::new();
    for [price, volume] in old {
        // Use integer representation of price for exact comparison
        let price_key = (price * 100000.0) as i64;
        old_map.insert(price_key, *volume);
    }
    
    let mut new_map: HashMap<i64, f64> = HashMap::new();
    for [price, volume] in new {
        let price_key = (price * 100000.0) as i64;
        new_map.insert(price_key, *volume);
    }
    
    // Find changes
    for (price_key, new_volume) in &new_map {
        let price = (*price_key as f64) / 100000.0;
        
        if let Some(old_volume) = old_map.get(price_key) {
            if (new_volume - old_volume).abs() > 0.00000001 {
                // Volume changed
                changes.push(PriceLevel {
                    price,
                    volume: *new_volume,
                    action: DeltaAction::Update,
                });
            }
        } else {
            // New price level
            changes.push(PriceLevel {
                price,
                volume: *new_volume,
                action: DeltaAction::Add,
            });
        }
    }
    
    // Find removed levels
    for (price_key, _) in &old_map {
        if !new_map.contains_key(price_key) {
            let price = (*price_key as f64) / 100000.0;
            changes.push(PriceLevel {
                price,
                volume: 0.0,
                action: DeltaAction::Remove,
            });
        }
    }
    
    changes
}