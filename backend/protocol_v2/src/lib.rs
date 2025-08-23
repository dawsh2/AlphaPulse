//! AlphaPulse Protocol V2 - TLV Universal Message Protocol
//! 
//! This crate implements a high-performance message protocol using bijective (reversible) IDs 
//! and universal TLV (Type-Length-Value) message format. All services communicate using 
//! structured binary messages with deterministic IDs that require no mapping tables.

use thiserror::Error;

// Re-export core types and modules
pub mod header;
pub mod tlv;
pub mod instrument_id;
pub mod recovery;
pub mod transport;
pub mod validation;
pub mod relay;

pub use header::*;
pub use tlv::*;
pub use instrument_id::*;
pub use recovery::*;
pub use transport::*;
pub use validation::*;

/// Protocol magic number for message identification
pub const MESSAGE_MAGIC: u32 = 0xDEADBEEF;

/// Protocol version
pub const PROTOCOL_VERSION: u8 = 1;

/// Standard Unix socket paths for relays
pub const MARKET_DATA_RELAY_PATH: &str = "/tmp/alphapulse/market_data.sock";
pub const SIGNAL_RELAY_PATH: &str = "/tmp/alphapulse/signals.sock";
pub const EXECUTION_RELAY_PATH: &str = "/tmp/alphapulse/execution.sock";

/// Protocol errors
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),
    
    #[error("Unknown TLV type: {0}")]
    UnknownTLV(u8),
    
    #[error("Invalid instrument ID")]
    InvalidInstrument,
    
    #[error("Checksum validation failed")]
    ChecksumFailed,
    
    #[error("Message too large: {size} bytes")]
    MessageTooLarge { size: usize },
    
    #[error("Invalid relay domain: {0}")]
    InvalidRelayDomain(u8),
    
    #[error("Recovery error: {0}")]
    Recovery(String),
    
    #[error("Transport error: {0}")]
    Transport(#[from] std::io::Error),
}

/// Result type for protocol operations
pub type Result<T> = std::result::Result<T, ProtocolError>;

/// Relay domains for message routing
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, num_enum::TryFromPrimitive, serde::Serialize, serde::Deserialize)]
pub enum RelayDomain {
    MarketData = 1,
    Signal = 2,
    Execution = 3,
}

/// Source types for message attribution
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, num_enum::TryFromPrimitive, serde::Serialize, serde::Deserialize)]
pub enum SourceType {
    // Exchange collectors (1-19)
    BinanceCollector = 1,
    KrakenCollector = 2,
    CoinbaseCollector = 3,
    PolygonCollector = 4,
    
    // Strategy services (20-39)
    ArbitrageStrategy = 20,
    MarketMaker = 21,
    TrendFollower = 22,
    
    // Execution services (40-59)
    PortfolioManager = 40,
    RiskManager = 41,
    ExecutionEngine = 42,
    
    // System services (60-79)
    Dashboard = 60,
    MetricsCollector = 61,
    
    // Relays themselves (80-99)
    MarketDataRelay = 80,
    SignalRelay = 81,
    ExecutionRelay = 82,
}