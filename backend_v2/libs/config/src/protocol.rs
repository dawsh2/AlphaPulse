//! Protocol V2 constants and configuration
//!
//! This module contains constants related to the Torq Protocol V2
//! TLV messaging system, including magic numbers, domain ranges, and
//! performance targets.

/// Protocol V2 standard magic number for message identification
pub const MAGIC_NUMBER: u32 = 0xDEADBEEF;

/// Message header size (32 bytes)
pub const MESSAGE_HEADER_SIZE: usize = 32;

/// TLV domain ranges for type separation
pub mod tlv {
    use std::ops::Range;
    
    /// Market Data TLV types (1-19)
    pub const MARKET_DATA_RANGE: Range<u8> = 1..20;
    
    /// Signal TLV types (20-39) 
    pub const SIGNAL_RANGE: Range<u8> = 20..40;
    
    /// Execution TLV types (40-79)
    pub const EXECUTION_RANGE: Range<u8> = 40..80;
    
    /// System/Internal TLV types (80-99)
    pub const SYSTEM_RANGE: Range<u8> = 80..100;
}

/// Performance targets for Protocol V2
pub mod performance {
    /// Target message construction rate (messages per second)
    pub const TARGET_CONSTRUCTION_RATE: u64 = 1_000_000;
    
    /// Target message parsing rate (messages per second)
    pub const TARGET_PARSING_RATE: u64 = 1_600_000;
    
    /// Target hot path latency (microseconds)
    pub const TARGET_HOT_PATH_LATENCY_US: u64 = 35;
    
    /// Target InstrumentID operation rate (operations per second)
    pub const TARGET_INSTRUMENT_ID_OPS: u64 = 19_000_000;
    
    /// Maximum memory usage per service (bytes)
    pub const MAX_MEMORY_USAGE_BYTES: u64 = 50 * 1024 * 1024; // 50MB
}