//! AlphaPulse Trading Strategies
//!
//! This crate contains all trading strategy implementations for the AlphaPulse system.
//! Strategies consume market data and generate trading signals for execution.

pub mod flash_arbitrage;
pub mod kraken_signals;

// Re-export commonly used types
pub use flash_arbitrage::{
    ArbitrageCalculator,
    ArbitrageConfig,
    ArbitrageDetector,
    SignalOutput,
};

/// Common strategy traits and types
pub mod common {
    use alphapulse_types::protocol::tlv::SignalIdentityTLV;
    use rust_decimal::Decimal;
    
    /// Result type for strategy operations
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
    
    /// Common configuration for all strategies
    #[derive(Debug, Clone)]
    pub struct StrategyConfig {
        pub name: String,
        pub enabled: bool,
        pub min_profit_threshold: Decimal,
        pub max_position_size: Decimal,
    }
    
    /// Trait for all trading strategies
    pub trait TradingStrategy: Send + Sync {
        fn name(&self) -> &str;
        fn process_market_data(&mut self, data: &[u8]) -> Result<Option<SignalIdentityTLV>>;
        fn is_enabled(&self) -> bool;
    }
}