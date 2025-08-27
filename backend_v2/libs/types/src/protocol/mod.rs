//! Protocol layer modules for AlphaPulse system
//!
//! This module contains protocol-specific implementations including
//! TLV structures, message handling, and identifier systems.

pub mod help;
pub mod identifiers;
pub mod message;
pub mod recovery;
pub mod tlv;
pub mod validation;

// Re-export key types for convenience with explicit naming to avoid conflicts
pub use identifiers::*;
pub use message::*;
pub use recovery::*;
pub use validation::*;

// Re-export TLV types selectively to avoid conflicts
pub use tlv::{
    // Core TLV functionality (only include existing types)
    TradeTLV, QuoteTLV, ArbitrageSignalTLV, PoolStateTLV, PoolInfoTLV,
    
    // State management types
    InvalidationReason,
    
    // System and observability types
    SystemHealthTLV, TraceEvent, TraceEventType, StateInvalidationTLV,
    
    // Market data TLV types
    PoolSwapTLV,
    
    // TLV size constants (only include existing ones)
    ARBITRAGE_SIGNAL_TLV_SIZE,
    
    // Utility functions (avoiding conflicts)
    fast_timestamp_ns, init_timestamp_system,
    build_message_direct, TrueZeroCopyBuilder,
    
    // Pool types with explicit naming
    pool_state::{DEXProtocol, PoolStateTracker, PoolType as TLVPoolType},
    pool_cache::{CachePoolType, PoolCacheJournalEntry},
    
    // Buffer management
    build_with_size_hint, with_hot_path_buffer, with_signal_buffer,
    with_validation_buffer, BufferError,
    
    // Address handling
    AddressConversion, AddressExtraction, PaddedAddress,
    
    // Dynamic payload support
    DynamicPayload, FixedStr, FixedVec, PayloadError,
    MAX_INSTRUMENTS, MAX_ORDER_LEVELS, MAX_POOL_TOKENS
};

// Protocol-level error type
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Parse error: {0}")]
    Parse(#[from] tlv::ParseError),

    #[error("Unknown TLV type: {0}")]
    UnknownTLV(u8),

    #[error("Invalid instrument: {0}")]
    InvalidInstrument(String),

    #[error("Checksum validation failed")]
    ChecksumFailed,

    #[error(
        "Bounds check failed: offset {offset} + length {length} exceeds buffer size {buffer_size}"
    )]
    BoundsCheckFailed {
        offset: usize,
        length: usize,
        buffer_size: usize,
    },

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, num_enum::TryFromPrimitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RelayDomain {
    MarketData = 1,
    Signal = 2,
    Execution = 3,
    System = 4,
}

/// Source types for message attribution
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, num_enum::TryFromPrimitive)]
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

// Re-export commonly needed types at protocol level
pub use tlv::types::TLVType;

// Domain constants
pub const MESSAGE_MAGIC: u32 = 0xDEADBEEF;
pub const PROTOCOL_VERSION: u8 = 1;

/// Standard Unix socket paths for relays
pub const MARKET_DATA_RELAY_PATH: &str = "/tmp/alphapulse/market_data.sock";
pub const SIGNAL_RELAY_PATH: &str = "/tmp/alphapulse/signals.sock";
pub const EXECUTION_RELAY_PATH: &str = "/tmp/alphapulse/execution.sock";
