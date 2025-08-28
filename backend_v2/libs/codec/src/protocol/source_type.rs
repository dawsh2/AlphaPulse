//! Source types for message attribution in Torq system

use num_enum::TryFromPrimitive;

/// Source types for message attribution
/// 
/// Each service that produces messages has a unique source type
/// for tracking message origin and enabling per-source sequence numbers.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SourceType {
    // Exchange collectors (1-19)
    BinanceCollector = 1,
    KrakenCollector = 2,
    CoinbaseCollector = 3,
    PolygonCollector = 4,
    GeminiCollector = 5,

    // Strategy services (20-39)
    ArbitrageStrategy = 20,
    MarketMaker = 21,
    TrendFollower = 22,
    KrakenSignalStrategy = 23,

    // Execution services (40-59)
    PortfolioManager = 40,
    RiskManager = 41,
    ExecutionEngine = 42,

    // System services (60-79)
    Dashboard = 60,
    MetricsCollector = 61,
    StateManager = 62,

    // Relays themselves (80-99)
    MarketDataRelay = 80,
    SignalRelay = 81,
    ExecutionRelay = 82,
}

impl SourceType {
    /// Check if this source is an exchange collector
    pub fn is_collector(&self) -> bool {
        matches!(*self as u8, 1..=19)
    }
    
    /// Check if this source is a strategy service
    pub fn is_strategy(&self) -> bool {
        matches!(*self as u8, 20..=39)
    }
    
    /// Check if this source is an execution service
    pub fn is_execution(&self) -> bool {
        matches!(*self as u8, 40..=59)
    }
    
    /// Check if this source is a system service
    pub fn is_system(&self) -> bool {
        matches!(*self as u8, 60..=79)
    }
    
    /// Check if this source is a relay
    pub fn is_relay(&self) -> bool {
        matches!(*self as u8, 80..=99)
    }
}