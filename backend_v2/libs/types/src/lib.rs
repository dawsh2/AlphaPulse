//! # Torq Unified Types Library
//!
//! Unified type system for Torq Protocol V2 TLV messages and common types.
//!
//! ## Design Philosophy
//!
//! - **Unified Type System**: Single library for all Torq type definitions
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
//! use torq_types::{TradeTLV, RelayDomain, SourceType, TLVType};
//!
//! // Create a trade message
//! let trade = TradeTLV::new(/* ... */);
//!
//! // For message building, import codec separately in services:
//! // use codec::TLVMessageBuilder;
//! // let message = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
//! //     .add_tlv(TLVType::Trade, &trade)
//! //     .build();
//! ```
//!
//! ### Instrument Identification
//! ```rust
//! use torq_types::{InstrumentId, VenueId};
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
//! use torq_types::{UsdFixedPoint8, PercentageFixedPoint4};
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
//! This unified library serves the entire Torq system:
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

// Precision module for financial data validation
pub mod precision;

// Message types for domain-specific communication
#[cfg(feature = "messages")]
pub mod messages;

// Re-export common types for convenience
#[cfg(feature = "common")]
pub use common::errors::{FixedPointError, ValidationError};
#[cfg(feature = "common")]
pub use common::fixed_point::{PercentageFixedPoint4, UsdFixedPoint8};

// Re-export common identifier types
#[cfg(feature = "common")]
pub use common::identifiers::{
    ActorId,
    BlockHash,
    ChainId,
    // Typed byte array wrappers
    EthAddress,
    EthSignature,
    Hash256,
    OpportunityId,
    // Typed ID system
    OrderId,
    PoolAddress,
    PoolId,
    PoolPairId,
    PortfolioId,
    PositionId,
    PrivateKey,
    PublicKey,
    RelayId,
    SequenceId,
    SessionId,
    SignalId,
    SimpleInstrumentId,
    SimpleVenueId,
    StrategyId,
    TokenAddress,
    TradeId,
    TxHash,
};

// Re-export protocol types for backward compatibility
#[cfg(feature = "protocol")]
pub use protocol::*;

// Re-export protocol identifiers for primary API
#[cfg(feature = "protocol")]
pub use protocol::identifiers::{AssetType, InstrumentId, VenueId};

// Re-export core protocol types that are commonly used in imports
#[cfg(feature = "protocol")]
pub use protocol::message::header::MessageHeader;
#[cfg(feature = "protocol")]
pub use protocol::{ProtocolError, RelayDomain, SourceType, TLVType};

// Define Result type alias
pub type Result<T> = std::result::Result<T, anyhow::Error>;

// Re-export protocol constants
#[cfg(feature = "protocol")]
pub use protocol::{
    EXECUTION_RELAY_PATH, MARKET_DATA_RELAY_PATH, MESSAGE_MAGIC, PROTOCOL_VERSION,
    SIGNAL_RELAY_PATH,
};

// Re-export common message types
pub use messages::{
    MarketMessage, SignalMessage, ExecutionMessage,
    PoolSwapEvent, QuoteUpdate, OrderBookUpdate, VolumeData,
    ArbitrageSignal, MomentumSignal, LiquidationSignal, RiskAlert,
    OrderRequest, CancelRequest, ExecutionResult, PositionUpdate,
    Message, MessageHandler, TypedReceiver, MessageRegistry, MessageStats,
};
