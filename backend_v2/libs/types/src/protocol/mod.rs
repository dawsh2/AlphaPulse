//! Protocol layer modules for Torq system
//!
//! This module contains protocol-specific implementations including
//! TLV structures, message handling, and identifier systems.

// help module moved to codec/src/help.rs
// validation modules moved to codec/src/validation/
// recovery module moved to network/src/recovery/
pub mod constants;
pub mod identifiers;
pub mod message;
pub mod tlv;

// Re-export key types for convenience with explicit naming to avoid conflicts
pub use constants::{
    MESSAGE_MAGIC, PROTOCOL_VERSION, RelayDomain, SourceType, ProtocolError,
    MARKET_DATA_RELAY_PATH, SIGNAL_RELAY_PATH, EXECUTION_RELAY_PATH
};
pub use identifiers::*;
pub use message::*;

// Re-export TLV types selectively to avoid conflicts
pub use tlv::{
    // Buffer management moved to network/src/buffers.rs
    // Timestamp functions moved to network/src/time.rs
    pool_cache::{CachePoolType, PoolCacheJournalEntry},

    // Pool types with explicit naming
    pool_state::{DEXProtocol, PoolStateTracker, PoolType as TLVPoolType},
    // Buffer functions moved to network - import directly from network crate
    // Address handling
    AddressConversion,
    AddressExtraction,
    ArbitrageSignalTLV,

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

// Protocol types moved to codec - consumers should import directly from codec
// to avoid circular dependencies

// Re-export commonly needed types at protocol level
pub use tlv::types::TLVType;
