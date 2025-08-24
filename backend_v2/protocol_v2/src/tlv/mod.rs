//! # TLV (Type-Length-Value) Protocol System - Core Module
//!
//! ## Purpose
//!
//! Implements the complete TLV message system for AlphaPulse Protocol V2, providing
//! zero-copy serialization, domain-based routing, and comprehensive type safety.
//! This module serves as the foundation for all inter-service communication within
//! the AlphaPulse trading system.
//!
//! ## Integration Points
//!
//! - **Message Construction**: TLVMessageBuilder for composing protocol messages
//! - **Parsing Pipeline**: Standard and extended TLV parsing with validation  
//! - **Relay Routing**: Automatic domain-based message routing to appropriate services
//! - **Serialization**: Zero-copy binary serialization via zerocopy crate
//! - **Type Registry**: Complete catalog of all supported message types with metadata
//!
//! ## Architecture Role
//!
//! ```text
//! Services → [TLV Builder] → Binary Messages → [TLV Parser] → Services
//!     ↑                            ↓                            ↓
//! Typed         Zero-Copy       Network        Type-Safe     Relay
//! Structs      Serialization   Transport      Parsing       Routing
//! ```
//!
//! ## Message Flow
//!
//! 1. **Construction**: Services create typed TLV structs (TradeTLV, PoolSwapTLV)
//! 2. **Building**: TLVMessageBuilder wraps TLV with header and routing metadata
//! 3. **Serialization**: Zero-copy conversion to binary format via AsBytes trait
//! 4. **Transport**: Binary messages sent over Unix sockets or network
//! 5. **Parsing**: parse_header() and parse_tlv_extensions() extract typed data
//! 6. **Routing**: Relay domain determines which service processes the message
//!
//! ## Performance Profile
//!
//! - **Construction Speed**: >1M messages/second (measured: 1,097,624 msg/s)
//! - **Parsing Speed**: >1.6M messages/second (measured: 1,643,779 msg/s)
//! - **Memory Usage**: Zero-copy operations minimize allocations
//! - **Latency**: <10μs overhead for fixed-size TLVs in hot paths
//! - **Throughput**: Designed for high-frequency trading workloads
//!
//! ## Type System Organization
//!
//! TLV types are organized into routing domains for efficient message distribution:
//!
//! | Domain | Type Range | Purpose | Examples |
//! |--------|------------|---------|----------|
//! | MarketData | 1-19 | Price feeds, order books | TradeTLV, PoolSwapTLV |
//! | Signal | 20-39 | Trading signals, coordination | SignalIdentity, Economics |
//! | Execution | 40-59 | Order management, fills | OrderRequest, ExecutionReport |
//! | System | 100-119 | Health, errors, discovery | Heartbeat, SystemHealth |
//!
//! ## Size Constraints and Performance
//!
//! The type system uses three size constraint categories for optimal performance:
//!
//! ### Fixed-Size TLVs (Highest Performance)
//! - **Size**: Exact byte count known at compile time
//! - **Performance**: Zero validation overhead, optimal for hot paths
//! - **Examples**: TradeTLV (37 bytes), Economics (32 bytes)
//! - **Use Case**: High-frequency market data, critical trading signals
//!
//! ### Bounded-Size TLVs (Good Performance)  
//! - **Size**: Minimum and maximum bounds with single validation check
//! - **Performance**: One bounds check per message
//! - **Examples**: PoolSwap (60-200 bytes), SignalIdentity (32-128 bytes)
//! - **Use Case**: Variable-content messages with reasonable limits
//!
//! ### Variable-Size TLVs (Flexible)
//! - **Size**: No upper limit, dynamic allocation required
//! - **Performance**: Memory allocation overhead
//! - **Examples**: OrderBook (unlimited levels), ComplexSignal
//! - **Use Case**: Batch processing, less time-sensitive operations
//!
//! ## Extended TLV Format
//!
//! For payloads exceeding 255 bytes, the extended format (Type 255) provides:
//! - **Length Field**: 16-bit length (up to 65,535 bytes)
//! - **Type Field**: 8-bit actual type embedded in extended header
//! - **Use Cases**: Large order books, batch messages, complex analytics
//!
//! ## Safety and Validation
//!
//! ### Packed Struct Safety (Critical)
//! ```rust
//! // ❌ DANGEROUS - Creates unaligned reference (crashes on ARM!)
//! println!("Price: {}", trade_tlv.price);
//!
//! // ✅ SAFE - Always copy packed fields first
//! let price = trade_tlv.price;  // Copy to stack
//! println!("Price: {}", price); // Safe to use
//! ```
//!
//! ### Checksum Validation
//! All TLV messages include checksums for data integrity verification
//!
//! ### Size Validation  
//! Parser enforces size constraints based on TLV type metadata
//!
//! ## Examples
//!
//! ### Basic Message Construction
//! ```rust
//! use alphapulse_protocol_v2::tlv::{TLVMessageBuilder, TLVType, TradeTLV};
//! use alphapulse_protocol_v2::{RelayDomain, SourceType};
//!
//! // Create trade data
//! let trade = TradeTLV::new(venue, instrument, price, volume, side, timestamp);
//!
//! // Build complete message with routing
//! let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
//!     .add_tlv(TLVType::Trade, &trade)
//!     .build();
//!
//! // Zero-copy serialization
//! let bytes = message.as_bytes();
//! ```
//!
//! ### Message Parsing
//! ```rust
//! use alphapulse_protocol_v2::tlv::{parse_header, parse_tlv_extensions};
//!
//! // Parse header with validation
//! let header = parse_header(&received_bytes)?;
//! if header.magic != MESSAGE_MAGIC {
//!     return Err(ProtocolError::ChecksumFailed);
//! }
//!
//! // Parse TLV payload
//! let tlvs = parse_tlv_extensions(&received_bytes[32..])?;
//! for tlv_ext in tlvs {
//!     match tlv_ext.header.tlv_type {
//!         1 => { /* Process trade */ }
//!         11 => { /* Process pool swap */ }
//!         _ => { /* Unknown type */ }
//!     }
//! }
//! ```
//!
//! ### Type Discovery and Metadata
//! ```rust
//! use alphapulse_protocol_v2::tlv::{TLVType, TLVSizeConstraint};
//!
//! // Get comprehensive type information
//! let trade_info = TLVType::Trade.type_info();
//! println!("Trade TLV: {} bytes, routes to {:?}",
//!          match trade_info.size_constraint {
//!              TLVSizeConstraint::Fixed(size) => size.to_string(),
//!              _ => "Variable".to_string()
//!          },
//!          trade_info.relay_domain);
//!
//! // Query types by routing domain
//! let market_types = TLVType::types_in_domain(RelayDomain::MarketData);
//! println!("Market data domain has {} message types", market_types.len());
//! ```
//!
//! ## Error Handling
//!
//! The TLV system provides comprehensive error handling with specific error types:
//!
//! ```rust
//! use alphapulse_protocol_v2::tlv::{ParseError, ParseResult};
//!
//! fn process_message(data: &[u8]) -> ParseResult<()> {
//!     // Size validation
//!     if data.len() < 32 {
//!         return Err(ParseError::MessageTooSmall {
//!             need: 32, got: data.len()
//!         });
//!     }
//!
//!     // Magic validation
//!     let header = parse_header(data)?;
//!     if header.magic != MESSAGE_MAGIC {
//!         return Err(ParseError::InvalidMagic {
//!             expected: MESSAGE_MAGIC,
//!             actual: header.magic
//!         });
//!     }
//!
//!     // Process TLV extensions with type-specific validation
//!     let extensions = parse_tlv_extensions(&data[32..])?;
//!     for ext in extensions {
//!         validate_tlv_size(&ext)?;
//!         route_to_handler(&ext)?;
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Development Tools
//!
//! ### Interactive Type Discovery
//! ```rust
//! // Comprehensive help system
//! alphapulse_protocol_v2::help::show_tlv_type_methods();
//! alphapulse_protocol_v2::help::explore_tlv_type(TLVType::PoolSwap);
//! ```
//!
//! ### Auto-Generated Documentation
//! ```rust
//! // Generate up-to-date markdown documentation
//! let markdown = TLVType::generate_markdown_table();
//! std::fs::write("docs/message-types.md", markdown)?;
//! ```
//!
//! ### Benchmarking and Validation
//! ```bash
//! # Performance testing
//! cargo run --bin test_protocol --release
//! cargo bench --package protocol_v2
//!
//! # Validation testing  
//! cargo test tlv --package protocol_v2
//! ```
//!
//! ## Service Integration Patterns
//!
//! ### Message Producer (Exchange Collector)
//! ```rust
//! impl ExchangeCollector {
//!     async fn send_trade(&self, trade: TradeTLV) -> Result<()> {
//!         let message = TLVMessageBuilder::new(
//!             RelayDomain::MarketData,
//!             SourceType::BinanceCollector
//!         )
//!         .add_tlv(TLVType::Trade, &trade)
//!         .build();
//!
//!         self.market_data_relay.send(message.as_bytes()).await
//!     }
//! }
//! ```
//!
//! ### Message Consumer (Trading Strategy)
//! ```rust
//! impl TradingStrategy {
//!     async fn handle_message(&mut self, data: &[u8]) -> Result<()> {
//!         let header = parse_header(data)?;
//!         let extensions = parse_tlv_extensions(&data[32..])?;
//!
//!         for ext in extensions {
//!             match ext.header.tlv_type {
//!                 1 => self.handle_trade(TradeTLV::from_bytes(&ext.payload)?).await?,
//!                 11 => self.handle_pool_swap(PoolSwapTLV::from_bytes(&ext.payload)?).await?,
//!                 _ => warn!("Unknown TLV type: {}", ext.header.tlv_type),
//!             }
//!         }
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ## Best Practices
//!
//! ### Performance Optimization
//! 1. **Use fixed-size TLVs** in hot paths where possible
//! 2. **Pre-allocate buffers** for message construction
//! 3. **Batch variable-size messages** to amortize allocation costs
//! 4. **Copy packed fields** before use to avoid alignment issues
//!
//! ### Memory Safety
//! 1. **Never take references** to packed struct fields directly
//! 2. **Always validate** message size before parsing
//! 3. **Check checksums** for data integrity
//! 4. **Handle unknown TLV types** gracefully
//!
//! ### System Integration
//! 1. **Use appropriate relay domains** for message routing
//! 2. **Include source attribution** for debugging and monitoring
//! 3. **Implement comprehensive error handling** with specific error types
//! 4. **Monitor message frequencies** and performance characteristics
//!
//! ## Module Structure
//!
//! - [`parser`] - Core TLV parsing logic for standard and extended formats
//! - [`builder`] - TLVMessageBuilder for constructing protocol messages
//! - [`types`] - Complete TLV type registry with metadata and discovery
//! - [`extended`] - Extended TLV format for large payloads (>255 bytes)
//! - [`market_data`] - Market data TLV structures (TradeTLV, PoolSwapTLV, etc.)
//! - [`system`] - System management TLVs (Heartbeat, Error, Health)
//! - [`pool_state`] - DEX pool state management and tracking
//! - [`type_safe`] - Type-safe wrappers and validation utilities

pub mod address;
pub mod builder;
pub mod demo_defi;
pub mod dynamic_payload;
pub mod extended;
pub mod market_data;
pub mod parser;
pub mod pool_cache;
pub mod pool_state;
pub mod relay_parser;
pub mod system;
pub mod type_safe;
pub mod types;
pub mod zero_copy_builder;
pub mod zero_copy_tests;

pub use address::{AddressConversion, AddressExtraction, PaddedAddress};
pub use builder::*;
pub use dynamic_payload::{
    DynamicPayload, FixedStr, FixedVec, PayloadError, MAX_INSTRUMENTS, MAX_POOL_TOKENS,
};
pub use extended::*;
pub use market_data::*;
pub use parser::*;
pub use relay_parser::*;
pub use types::*;
pub use zero_copy_builder::{TLVRef, ZeroCopyTLVMessageBuilder};
// Export pool_state PoolType explicitly (it's a type alias for DEXProtocol)
pub use demo_defi::*;
pub use pool_state::{DEXProtocol, PoolStateTLV, PoolStateTracker, PoolType};
// Export pool_cache with renamed type to avoid conflicts
pub use pool_cache::{CachePoolType, PoolCacheJournalEntry, PoolInfoTLV};
// Export system/tracing TLVs for observability
pub use system::{SystemHealthTLV, TraceContextTLV, TraceEvent, TraceEventType};

use thiserror::Error;

/// TLV parsing errors with detailed context
///
/// Provides comprehensive error information for debugging and monitoring.
/// Each error variant includes specific context about what went wrong and
/// what was expected, enabling precise error handling and diagnostics.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Message too small: need {need} bytes, got {got}")]
    MessageTooSmall { need: usize, got: usize },

    #[error("Invalid magic number: expected {expected:#x}, got {actual:#x}")]
    InvalidMagic { expected: u32, actual: u32 },

    #[error("Checksum mismatch: expected {expected:#x}, calculated {calculated:#x}")]
    ChecksumMismatch { expected: u32, calculated: u32 },

    #[error("Truncated TLV at offset {offset}")]
    TruncatedTLV { offset: usize },

    #[error("Unknown TLV type: {0}")]
    UnknownTLVType(u8),

    #[error("Unknown source type: {0}")]
    UnknownSource(u8),

    #[error("Invalid extended TLV format")]
    InvalidExtendedTLV,

    #[error("TLV payload too large: {size} bytes")]
    PayloadTooLarge { size: usize },
}

/// Simple TLV Header for basic parsing (types 1-254)
///
/// Used for standard TLV messages with payloads up to 255 bytes.
/// This is the most common format for performance-critical messages.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SimpleTLVHeader {
    /// TLV type number (1-254, 255 reserved for extended format)
    pub tlv_type: u8,
    /// Payload length in bytes (0-255)
    pub tlv_length: u8,
}

/// Extended TLV Header for type 255 (large payloads)
///
/// Used for messages requiring more than 255 bytes of payload data.
/// The extended format embeds the actual TLV type and uses a 16-bit length field.
/// Packed to achieve exactly 5 bytes
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ExtendedTLVHeader {
    /// Marker byte (always 255) indicating extended format
    pub marker: u8,
    /// Reserved byte (always 0) for future use
    pub reserved: u8,
    /// Actual TLV type embedded in extended header
    pub tlv_type: u8,
    /// Payload length as 16-bit value (up to 65,535 bytes)
    pub tlv_length: u16,
}

/// A parsed simple TLV extension with payload
///
/// Represents a successfully parsed standard TLV message with header
/// and payload ready for type-specific processing.
#[derive(Debug, Clone)]
pub struct SimpleTLVExtension {
    pub header: SimpleTLVHeader,
    pub payload: Vec<u8>,
}

/// An extended TLV extension with larger payload
///
/// Represents a successfully parsed extended TLV message that exceeded
/// the 255-byte limit of the standard format.
#[derive(Debug, Clone)]
pub struct ExtendedTLVExtension {
    pub header: ExtendedTLVHeader,
    pub payload: Vec<u8>,
}

/// Result type for TLV parsing operations
pub type ParseResult<T> = std::result::Result<T, ParseError>;

// Legacy TLV Message Format removed - use Protocol V2 MessageHeader + TLV extensions
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tlv_header_size() {
        assert_eq!(std::mem::size_of::<SimpleTLVHeader>(), 2);
        assert_eq!(std::mem::size_of::<ExtendedTLVHeader>(), 5);
    }

    #[test]
    fn test_parse_error_display() {
        let error = ParseError::MessageTooSmall { need: 32, got: 16 };
        let error_str = format!("{}", error);
        assert!(error_str.contains("need 32 bytes"));
        assert!(error_str.contains("got 16"));
    }
}
