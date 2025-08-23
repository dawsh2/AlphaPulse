//! # AlphaPulse Protocol V2 - TLV Universal Message Protocol
//!
//! High-performance message protocol with bijective IDs and zero-copy TLV format.
//! Achieves >1M msg/s throughput with deterministic routing and no lookup tables.
//!
//! ## API Surface
//!
//! The public API consists of types and functions exported through this module.
//! Key components:
//! - **Identifiers**: `InstrumentId`, `VenueId` - Bijective instrument identification
//! - **TLV Types**: `TradeTLV`, `QuoteTLV`, `PoolSwapTLV` - Message payload types
//! - **Message Building**: `TLVMessageBuilder` - Construct protocol messages
//! - **Parsing**: `parse_header()`, `parse_tlv_extensions()` - Parse received messages
//! - **Routing**: `RelayDomain`, `SourceType` - Message routing metadata
//!
//! ## Quick Start - Common Tasks
//!
//! ### Creating Instrument IDs (Most Common)
//! ```rust
//! use alphapulse_protocol_v2::{InstrumentId, VenueId};
//!
//! // Cryptocurrency coins - USE coin(venue, symbol)
//! let btc = InstrumentId::coin(VenueId::Ethereum, "BTC");
//! let eth = InstrumentId::coin(VenueId::Polygon, "ETH");
//!
//! // Stocks - USE stock(exchange, symbol)  
//! let apple = InstrumentId::stock(VenueId::NASDAQ, "AAPL");
//! let google = InstrumentId::stock(VenueId::NYSE, "GOOGL");
//!
//! // ERC-20 Tokens - USE specific token methods
//! let usdc = InstrumentId::ethereum_token("0xA0b86a33E6441C4F32B87D3c49de33AD3E2F1EFe")?;
//! let usdc_polygon = InstrumentId::polygon_token("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174")?;
//!
//! // From raw ID
//! let raw_id = InstrumentId::from_u64(12345);
//!
//! // Convert back to numeric
//! let numeric_id: u64 = btc.to_u64();
//!
//! // WRONG - These methods DON'T exist:
//! // InstrumentId::crypto("BTC", "USD");   // ❌ Use coin(venue, symbol)
//! // InstrumentId::currency("USD");        // ❌ Use coin(venue, symbol)  
//! // InstrumentId::stock("AAPL");          // ❌ Missing VenueId parameter
//! ```
//!
//! ### Working with TLV Messages
//! ```rust
//! use alphapulse_protocol_v2::{TLVMessageBuilder, TLVType, TradeTLV, RelayDomain, SourceType};
//!
//! // Create a trade message
//! let trade = TradeTLV::new(/* ... */);
//!
//! // Build TLV message with proper routing
//! let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
//!     .add_tlv(TLVType::Trade, &trade)
//!     .build();
//!
//! // Serialize for transport (zero-copy)
//! let bytes = message.as_bytes();  // ✅ Use as_bytes()
//! // NOT: message.to_bytes()        // ❌ Method doesn't exist
//!
//! // Parse received message
//! let header = alphapulse_protocol_v2::parse_header(&bytes)?;
//! let tlvs = alphapulse_protocol_v2::parse_tlv_extensions(&bytes[32..])?;
//! ```
//!
//! ### ⚠️ Critical: Packed Struct Safety
//!
//! TLV structs use `#[repr(C, packed)]` for memory efficiency. **Never access fields directly**:
//!
//! ```rust
//! let trade = TradeTLV::from_bytes(data)?;
//!
//! // ❌ WRONG - Creates unaligned reference (crashes on ARM/M1/M2!)
//! println!("Price: {}", trade.price);
//! assert_eq!(trade.price, expected);
//! process_trade(&trade.price);  // Passing reference = crash!
//!
//! // ✅ CORRECT - Always copy packed fields first
//! let price = trade.price;      // Copy to stack
//! let volume = trade.volume;    // Copy to stack  
//! let side = trade.side;        // Copy to stack
//!
//! println!("Price: {}", price);              // ✅ Safe
//! assert_eq!(price, expected);               // ✅ Safe
//! process_trade(price);                      // ✅ Safe (by value)
//!
//! // ✅ Also correct - method calls (they copy internally)
//! let venue = trade.venue()?;    // Methods are safe
//! ```
//!
//! **Why this matters**: Intel/AMD CPUs tolerate unaligned access (with performance penalty),
//! but ARM (M1/M2 Macs, mobile) will **segfault immediately**.
//!
//! ### TLV Type Discovery (Developer API)
//! ```rust
//! use alphapulse_protocol_v2::{TLVType, RelayDomain};
//!
//! // Get detailed type information
//! let info = TLVType::Trade.type_info();
//! println!("{}: {} - {}", info.name, info.size_constraint, info.description);
//!
//! // Find types by domain for routing
//! let market_types = TLVType::types_in_domain(RelayDomain::MarketData);
//! let execution_types = TLVType::types_in_domain(RelayDomain::Execution);
//!
//! // Generate documentation
//! let markdown = TLVType::generate_markdown_table();
//! ```
//!
//! ## Module Organization
//!
//! - [`tlv`] - **TLV message types and parsing** (TradeTLV, QuoteTLV, PoolSwapTLV, etc.)
//! - [`identifiers`] - **ID system** (InstrumentId, VenueId with bijective properties)
//! - [`message`] - **Message headers and core protocol**
//! - [`validation`] - **Bounds checking and integrity validation**
//! - [`recovery`] - **Message recovery and sequence synchronization**
//!
//! ## Performance Characteristics
//!
//! - **Message Construction**: >1M msg/s (measured: 1,097,624 msg/s)
//! - **Message Parsing**: >1.6M msg/s (measured: 1,643,779 msg/s)
//! - **InstrumentId Operations**: >19M ops/s (bijective conversion)
//! - **Memory Usage**: <50MB per service, zero-copy where possible
//! - **Latency**: <35μs hot path processing target
//!
//! ## Relay Routing
//!
//! Messages automatically route to appropriate relays based on TLV type:
//!
//! | Domain | Types | Relay | Purpose |
//! |--------|-------|-------|---------|
//! | [`RelayDomain::MarketData`] | 1-19 | MarketDataRelay | Price feeds, order books, DEX events |
//! | [`RelayDomain::Signal`] | 20-39 | SignalRelay | Trading signals, strategy coordination |
//! | [`RelayDomain::Execution`] | 40-59 | ExecutionRelay | Orders, fills, portfolio updates |
//! | [`RelayDomain::System`] | 100-119 | SystemRelay | Health, errors, service discovery |
//!
//! ## Common Patterns
//!
//! ### Safe Packed Struct Usage (Important!)
//! ```rust
//! use alphapulse_protocol_v2::TradeTLV;
//! use zerocopy::AsBytes;
//!
//! let trade_tlv = TradeTLV::new(/* ... */);
//!
//! // ⚠️ ALWAYS copy packed fields to stack first!
//! let price = trade_tlv.price;  // Copy to local variable
//! let volume = trade_tlv.volume;  // Safe to use now
//!
//! // Serialization (zero-copy via zerocopy crate)
//! let bytes = trade_tlv.as_bytes();
//! let recovered = TradeTLV::from_bytes(bytes)?;
//! ```
//!
//! ### Error Handling
//! ```rust
//! use alphapulse_protocol_v2::{Result, ProtocolError};
//!
//! fn process_message(data: &[u8]) -> Result<()> {
//!     let header = alphapulse_protocol_v2::parse_header(data)?;
//!     
//!     if header.magic != alphapulse_protocol_v2::MESSAGE_MAGIC {
//!         return Err(ProtocolError::ChecksumFailed);
//!     }
//!     
//!     // Process TLV payload...
//!     Ok(())
//! }
//! ```
//!
//! ## See Also
//!
//! **Documentation:**
//! - [Common Mistakes Guide](../docs/common_mistakes.md) - Wrong vs correct API usage patterns
//! - [TLV Type Reference](../docs/message-types-auto.md) - Complete type catalog with routing
//! - [Performance Guide](../docs/PERFORMANCE_ANALYSIS.md) - Optimization and benchmarking
//! - [Maintenance Guide](../docs/MAINTENANCE.md) - System health and TLV registry updates
//!
//! **Code Examples:**
//! - [Examples Directory](../examples/) - Runnable code samples for all major features
//! - [`examples/instrument_id_creation.rs`](../examples/instrument_id_creation.rs) - Complete InstrumentId API
//! - [`examples/tlv_message_building.rs`](../examples/tlv_message_building.rs) - TLV construction patterns
//!
//! **Interactive Help:**
//! - [`help::show_all_help()`] - Comprehensive API overview in console
//! - [`help::show_instrument_id_methods()`] - InstrumentId creation methods
//! - [`help::show_common_mistakes()`] - Quick mistake reference
//! - [`help::explore_tlv_type()`] - Detailed type information
//!
//! **Performance Analysis:**
//! - `cargo run --bin test_protocol --release` - Benchmark Protocol V2 performance
//! - `cargo bench --workspace` - Full performance regression testing
//! - [`help::show_performance_tips()`] - Optimization recommendations

use thiserror::Error;

// Re-export core types and modules
pub mod help;
pub mod identifiers;
pub mod message;
pub mod recovery;
pub mod tlv;
pub mod validation;

pub use message::header::*;
// Re-export specific TLV types to avoid PoolType conflicts
pub use tlv::{
    parse_header,
    parse_tlv_extensions,
    // Export PoolType alias for backward compatibility from pool_state only
    pool_state::PoolType,
    // System TLVs
    system::{SystemHealthTLV, TraceContextTLV, TraceEvent, TraceEventType, TraceId},
    InvalidationReason,
    ParseError,
    ParseResult,
    PoolStateTLV,
    PoolSwapTLV,
    QuoteTLV,
    StateInvalidationTLV,
    TLVExtensionEnum,
    TLVMessageBuilder,
    TLVType,
    // Market data TLVs
    TradeTLV,
    VendorTLVType,
};

// Re-export specific identifier types to avoid PoolType conflicts
pub use identifiers::{
    // Export PoolType from pairing for pool structure types
    instrument::pairing::PoolType as PoolStructureType,
    AssetType,
    InstrumentId,
    VenueId,
};
pub use recovery::*;
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
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    num_enum::TryFromPrimitive,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum RelayDomain {
    MarketData = 1,
    Signal = 2,
    Execution = 3,
    System = 4,
}

/// Source types for message attribution
#[repr(u8)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    num_enum::TryFromPrimitive,
    serde::Serialize,
    serde::Deserialize,
)]
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
