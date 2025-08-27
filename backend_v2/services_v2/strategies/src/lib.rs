//! AlphaPulse Trading Strategies
//!
//! This crate contains all trading strategy implementations for the AlphaPulse system.
//! Strategies consume market data and generate trading signals for execution.

// Module declarations
pub mod common;

#[cfg(feature = "flash-arbitrage")]
pub mod flash_arbitrage;

#[cfg(feature = "kraken-signals")]
pub mod kraken_signals;

#[cfg(feature = "flash-arbitrage")]
// Re-export flash_arbitrage public API
pub use flash_arbitrage::{
    OpportunityDetector as ArbitrageDetector,
    RelayConsumer as FlashArbitrageConsumer,
    SignalOutput,
    DetectorConfig,
    FlashArbitrageConfig as ArbitrageConfig,
    StrategyConfig as FlashStrategyConfig,
    StrategyEngine as FlashStrategyEngine,
    Executor,
    GasPriceFetcher,
};

#[cfg(feature = "kraken-signals")]
// Re-export kraken_signals public API  
pub use kraken_signals::{
    KrakenSignalStrategy,
    StrategyConfig as KrakenStrategyConfig,
    StrategyError as KrakenStrategyError,
    SignalType,
    TradingSignal,
};

/// Common strategy traits and types
pub mod common {
    use alphapulse_types::protocol::tlv::ArbitrageSignalTLV;
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
        fn process_market_data(&mut self, data: &[u8]) -> Result<Option<ArbitrageSignalTLV>>;
        fn is_enabled(&self) -> bool;
    }
}