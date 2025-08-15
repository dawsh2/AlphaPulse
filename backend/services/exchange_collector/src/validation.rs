use alphapulse_protocol::*;
use tracing::{warn, error};

/// Minimal validation following NautilusTrader philosophy:
/// - Only reject structurally invalid data
/// - Preserve all market anomalies (they might be opportunities)
/// - Track data quality metrics without filtering
pub struct DataValidator {
    symbol: String,
    last_timestamp: Option<u64>,
    last_sequence: Option<u32>,
    gaps_detected: u64,
    out_of_order_count: u64,
}

impl DataValidator {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            last_timestamp: None,
            last_sequence: None,
            gaps_detected: 0,
            out_of_order_count: 0,
        }
    }

    /// Validate orderbook structural integrity only
    pub fn validate_orderbook(&mut self, orderbook: &OrderBookMessage) -> ValidationResult {
        let mut errors = Vec::new();
        
        // Check timestamp monotonicity
        if let Some(last_ts) = self.last_timestamp {
            if orderbook.timestamp_ns < last_ts {
                self.out_of_order_count += 1;
                warn!("Out-of-order orderbook for {}: ts {} < last {}", 
                    self.symbol, orderbook.timestamp_ns, last_ts);
                // Don't reject - exchanges can send corrections
            }
        }
        self.last_timestamp = Some(orderbook.timestamp_ns);
        
        let bid_count = orderbook.bids.len();
        let ask_count = orderbook.asks.len();

        // Only check for structurally invalid data
        for i in 0..bid_count {
            let bid = &orderbook.bids[i];
            let price = bid.price_f64();
            let volume = bid.volume_f64();
            
            if !price.is_finite() || price <= 0.0 {
                errors.push(format!("Invalid bid price: {}", price));
            }
            if !volume.is_finite() || volume < 0.0 {
                errors.push(format!("Invalid bid volume: {}", volume));
            }
        }
        
        for i in 0..ask_count {
            let ask = &orderbook.asks[i];
            let price = ask.price_f64();
            let volume = ask.volume_f64();
            
            if !price.is_finite() || price <= 0.0 {
                errors.push(format!("Invalid ask price: {}", price));
            }
            if !volume.is_finite() || volume < 0.0 {
                errors.push(format!("Invalid ask volume: {}", volume));
            }
        }

        // Log crossed books for debugging, but don't reject
        // This is likely our bug, not invalid market data
        if bid_count > 0 && ask_count > 0 {
            let best_bid = orderbook.bids[0].price_f64();
            let best_ask = orderbook.asks[0].price_f64();
            
            if best_bid >= best_ask {
                error!("DATA BUG: Crossed book in {} - bid {} >= ask {} at ts {}", 
                    self.symbol, best_bid, best_ask, orderbook.timestamp_ns);
                // Still process it - helps us debug the issue
            }
        }
        
        if !errors.is_empty() {
            ValidationResult::Invalid(errors)
        } else {
            ValidationResult::Valid
        }
    }
    
    /// Validate trade structural integrity
    pub fn validate_trade(&mut self, trade: &TradeMessage) -> ValidationResult {
        let mut errors = Vec::new();
        
        // Check timestamp monotonicity
        let ts = trade.timestamp_ns();
        if let Some(last_ts) = self.last_timestamp {
            if ts < last_ts {
                self.out_of_order_count += 1;
                warn!("Out-of-order trade for {}: ts {} < last {}", 
                    self.symbol, ts, last_ts);
                // Don't reject - valid during corrections
            }
        }
        self.last_timestamp = Some(ts);
        
        let price = trade.price_f64();
        let volume = trade.volume_f64();
        
        // Only structural validation
        if !price.is_finite() || price <= 0.0 {
            errors.push(format!("Invalid trade price: {}", price));
        }
        
        if !volume.is_finite() || volume < 0.0 {
            errors.push(format!("Invalid trade volume: {}", volume));
        }
        
        // Don't filter "unusual" prices - flash crashes are real!
        // Don't filter "unusual" volumes - block trades exist!
        
        if !errors.is_empty() {
            ValidationResult::Invalid(errors)
        } else {
            ValidationResult::Valid
        }
    }
    
    /// Validate L2 delta structural integrity
    pub fn validate_l2_delta(&mut self, delta: &L2DeltaMessage) -> ValidationResult {
        let mut errors = Vec::new();
        
        // Track sequence gaps (but don't reject)
        if let Some(last_seq) = self.last_sequence {
            let delta_seq = delta.sequence as u32; // Convert u64 to u32 for comparison
            if delta_seq > last_seq + 1 {
                let gap = delta_seq - last_seq - 1;
                self.gaps_detected += gap as u64;
                warn!("Sequence gap in {}: {} messages missed (seq {} -> {})", 
                    self.symbol, gap, last_seq, delta_seq);
                // Don't reject - we can still use the data
            } else if delta_seq <= last_seq {
                warn!("Duplicate/out-of-order sequence for {}: {} (last: {})",
                    self.symbol, delta_seq, last_seq);
                // Still process - might be a replay/correction
            }
            self.last_sequence = Some(delta_seq);
        } else {
            self.last_sequence = Some(delta.sequence as u32);
        }
        
        // Structural validation only
        for update in &delta.updates {
            let price = f64::from_le_bytes(update.price);
            let volume = f64::from_le_bytes(update.volume);
            
            if !price.is_finite() || price <= 0.0 {
                errors.push(format!("Invalid update price: {}", price));
            }
            
            // Volume=0 means remove level, negative is invalid
            if !volume.is_finite() || volume < 0.0 {
                errors.push(format!("Invalid update volume: {}", volume));
            }
        }
        
        if !errors.is_empty() {
            ValidationResult::Invalid(errors)
        } else {
            ValidationResult::Valid
        }
    }
    
    /// Get data quality metrics (for monitoring, not filtering)
    pub fn metrics(&self) -> DataQualityMetrics {
        DataQualityMetrics {
            symbol: self.symbol.clone(),
            gaps_detected: self.gaps_detected,
            out_of_order_count: self.out_of_order_count,
        }
    }
}

pub enum ValidationResult {
    Valid,
    Invalid(Vec<String>),
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid)
    }
}

/// Track data quality without filtering
#[derive(Debug, Clone)]
pub struct DataQualityMetrics {
    pub symbol: String,
    pub gaps_detected: u64,
    pub out_of_order_count: u64,
}