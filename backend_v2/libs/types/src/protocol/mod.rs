//! Protocol layer modules for Torq system
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
    // Buffer management (build_message_direct now in codec)
    build_with_size_hint,
    // Utility functions (avoiding conflicts)
    fast_timestamp_ns,
    init_timestamp_system,
    pool_cache::{CachePoolType, PoolCacheJournalEntry},

    // Pool types with explicit naming
    pool_state::{DEXProtocol, PoolStateTracker, PoolType as TLVPoolType},
    with_hot_path_buffer,
    with_signal_buffer,
    with_validation_buffer,
    // Address handling
    AddressConversion,
    AddressExtraction,
    ArbitrageSignalTLV,
    BufferError,

    // Dynamic payload support
    DynamicPayload,
    FixedStr,
    FixedVec,
    // State management types
    InvalidationReason,

    PaddedAddress,

    PayloadError,
    PoolInfoTLV,

    PoolStateTLV,
    // Market data TLV types
    PoolSwapTLV,

    QuoteTLV,
    StateInvalidationTLV,

    // System and observability types
    SystemHealthTLV,
    TraceEvent,
    TraceEventType,
    // Core TLV functionality (only include existing types)
    TradeTLV,
    // TrueZeroCopyBuilder moved to codec/src/builder.rs

    // TLV size constants (only include existing ones)
    ARBITRAGE_SIGNAL_TLV_SIZE,

    MAX_INSTRUMENTS,
    MAX_ORDER_LEVELS,
    MAX_POOL_TOKENS,
};

// TODO: Re-export protocol types from codec to maintain compatibility
// These need proper codec dependency setup in Cargo.toml
// pub use codec::{RelayDomain, SourceType, ProtocolError, MESSAGE_MAGIC, PROTOCOL_VERSION};
// pub use codec::{MARKET_DATA_RELAY_PATH, SIGNAL_RELAY_PATH, EXECUTION_RELAY_PATH};

/// Result type for protocol operations  
// TODO: Enable when codec dependency is properly set up
// pub type Result<T> = std::result::Result<T, ProtocolError>;

// Re-export commonly needed types at protocol level
pub use tlv::types::TLVType;
