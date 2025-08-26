//! # AlphaPulse Unified Types Library
//!
//! Unified type system for AlphaPulse Protocol V2 TLV messages and common types.
//!
//! ## Design Philosophy
//!
//! - **Unified Type System**: Single library for all AlphaPulse type definitions
//! - **No Precision Loss**: All financial values stored as scaled integers
//! - **Protocol V2 Integration**: Complete TLV message format support with >1M msg/s performance
//! - **Type Safety**: Distinct types prevent mixing incompatible scales or domains
//! - **Zero-Copy Operations**: zerocopy-enabled structs for high-performance parsing
//! - **Clear Boundaries**: Explicit conversion points between floating-point and fixed-point
//!
//! ## Quick Start
//!
//! ### Protocol V2 TLV Messages
//! ```rust
//! use alphapulse_types::{TLVMessageBuilder, TLVType, TradeTLV, RelayDomain, SourceType};
//!
//! // Create a trade message
//! let trade = TradeTLV::new(/* ... */);
//!
//! // Build TLV message with proper routing
//! let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
//!     .add_tlv(TLVType::Trade, &trade)
//!     .build();
//!
//! // Zero-copy serialization
//! let bytes = message.as_bytes();
//! ```
//!
//! ### Instrument Identification
//! ```rust
//! use alphapulse_types::{InstrumentId, VenueId};
//!
//! // Cryptocurrency coins
//! let btc = InstrumentId::coin(VenueId::Ethereum, "BTC");
//! let eth = InstrumentId::coin(VenueId::Polygon, "ETH");
//!
//! // ERC-20 Tokens
//! let usdc = InstrumentId::ethereum_token("0xA0b86a33E6441C4F32B87D3c49de33AD3E2F1EFe")?;
//! ```
//!
//! ### Fixed-Point Financial Calculations
//! ```rust
//! use alphapulse_types::{UsdFixedPoint8, PercentageFixedPoint4};
//!
//! // Parse from decimal strings (primary method)
//! let price = UsdFixedPoint8::from_decimal_str("42.12345678").unwrap();
//! let spread = PercentageFixedPoint4::from_decimal_str("0.25").unwrap();
//!
//! // Checked arithmetic for critical calculations
//! let fee = UsdFixedPoint8::ONE_CENT;
//! if let Some(total) = price.checked_add(fee) {
//!     println!("Total: {}", total);
//! }
//! ```
//!
//! ## Integration Points
//!
//! This unified library serves the entire AlphaPulse system:
//! - **Protocol V2**: TLV message construction, parsing, and routing (>1M msg/s)
//! - **Strategy Services**: Arbitrage detection, profit calculations, signal generation
//! - **Portfolio Management**: Position tracking, risk calculations, PnL computation
//! - **Market Data**: Price feeds, order book updates, DEX event processing
//! - **Execution Services**: Order management, trade execution, settlement
//! - **Dashboard Services**: Real-time display, historical analysis, monitoring
//!
//! ## Performance Characteristics
//!
//! - **Message Construction**: >1M msg/s (measured: 1,097,624 msg/s)
//! - **Message Parsing**: >1.6M msg/s (measured: 1,643,779 msg/s)  
//! - **InstrumentId Operations**: >19M ops/s (bijective conversion)
//! - **Zero-Copy Parsing**: Direct memory access with zerocopy traits
//! - **Memory Usage**: Minimal allocations, optimized for hot path operations

#[cfg(feature = "common")]
pub mod common;

#[cfg(feature = "protocol")]
pub mod protocol;

// Re-export common types for convenience
#[cfg(feature = "common")]
pub use common::errors::FixedPointError;
#[cfg(feature = "common")]
pub use common::fixed_point::{PercentageFixedPoint4, UsdFixedPoint8};

// Re-export common identifier types
#[cfg(feature = "common")]
pub use common::identifiers::{
    // Typed ID system
    OrderId, PositionId, StrategyId, SignalId, OpportunityId,
    TradeId, PortfolioId, SessionId, ActorId, RelayId, SequenceId,
    PoolId, PoolPairId, SimpleInstrumentId, ChainId, SimpleVenueId,
    // Typed byte array wrappers
    EthAddress, TxHash, BlockHash, Hash256, PoolAddress, TokenAddress,
    EthSignature, PublicKey, PrivateKey,
};

// Re-export protocol types for backward compatibility
#[cfg(feature = "protocol")]
pub use protocol::*;

// Re-export protocol identifiers for primary API
#[cfg(feature = "protocol")]
pub use protocol::identifiers::{AssetType, InstrumentId, VenueId};

// Re-export core protocol types that are commonly used in imports
#[cfg(feature = "protocol")]
pub use protocol::{RelayDomain, SourceType, ProtocolError, Result, TLVType};
#[cfg(feature = "protocol")]
pub use protocol::message::header::MessageHeader;

// Re-export protocol constants
#[cfg(feature = "protocol")]
pub use protocol::{MESSAGE_MAGIC, PROTOCOL_VERSION, MARKET_DATA_RELAY_PATH, SIGNAL_RELAY_PATH, EXECUTION_RELAY_PATH};
