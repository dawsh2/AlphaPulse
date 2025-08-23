//! Error types for Kraken signals strategy

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StrategyError {
    #[error("Protocol error: {0}")]
    Protocol(#[from] protocol_v2::ProtocolError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Calculation error: {message}")]
    Calculation { message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Signal generation error: {message}")]
    SignalGeneration { message: String },

    #[error("Market data error: {message}")]
    MarketData { message: String },
}

pub type Result<T> = std::result::Result<T, StrategyError>;
