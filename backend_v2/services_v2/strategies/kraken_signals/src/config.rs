//! Strategy configuration

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// Moving average periods for momentum calculation
    pub short_ma_period: usize,
    pub long_ma_period: usize,

    /// Minimum price movement threshold for signal generation (percentage)
    pub min_price_movement_pct: Decimal,

    /// Signal confidence threshold (0-100)
    pub min_confidence: u8,

    /// Maximum position size as percentage of typical volume
    pub max_position_pct: Decimal,

    /// Minimum time between signals for same instrument
    pub signal_cooldown: Duration,

    /// Target instruments to monitor
    pub target_instruments: Vec<String>,

    /// Signal relay connection path
    pub signal_relay_path: String,

    /// Market data relay connection path  
    pub market_data_relay_path: String,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            short_ma_period: 10,
            long_ma_period: 30,
            min_price_movement_pct: dec!(0.5), // 0.5% minimum movement
            min_confidence: 70,
            max_position_pct: dec!(5.0), // 5% of typical volume
            signal_cooldown: Duration::from_secs(300), // 5 minutes
            target_instruments: vec![
                "BTCUSD".to_string(),
                "ETHUSD".to_string(),
                "ADAUSD".to_string(),
                "DOTUSD".to_string(),
            ],
            signal_relay_path: "/tmp/torq/signals.sock".to_string(),
            market_data_relay_path: "/tmp/torq/market_data.sock".to_string(),
        }
    }
}
