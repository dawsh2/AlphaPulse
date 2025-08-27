//! # TLV Type System - Protocol V2 Message Type Registry
//!
//! ## Purpose
//!
//! Comprehensive type registry and introspection system for Protocol V2 TLV messages.
//! Provides domain-based organization (1-19 MarketData, 20-39 Signal, 40-59 Execution, 100-119 System)
//! with automatic routing, size validation, and rich developer API for discovery and documentation
//! generation. The type system enforces protocol integrity while enabling rapid development
//! through runtime introspection and comprehensive metadata.
//!
//! ## Integration Points
//!
//! - **Message Construction**: TLVMessageBuilder uses type metadata for format selection
//! - **Parsing Validation**: Parser validates payload sizes against type constraints
//! - **Relay Routing**: Automatic domain-based routing to appropriate relay services
//! - **Documentation**: Auto-generation of API references and message type tables
//! - **Development Tools**: IDE integration through rich type introspection API
//! - **Service Discovery**: Runtime enumeration of available message types
//!
//! ## Architecture Role
//!
//! ```text
//! Developer Tools → [TLV Type Registry] → Protocol Implementation
//!       ↑                ↓                        ↓
//!   IDE Help        Type Metadata           Message Routing
//!   Code Gen        Size Validation         Service Discovery
//!   Docs Gen        Domain Mapping          Format Selection
//! ```
//!
//! The type registry serves as the central source of truth for all Protocol V2 message
//! types, enabling both compile-time safety and runtime discoverability.

use num_enum::TryFromPrimitive;

/// Size constraint validation for TLV message payloads
///
/// Enables efficient payload validation during parsing with minimal overhead.
/// Fixed sizes have zero validation cost, bounded sizes require single bounds check,
/// and variable sizes accept any payload length for flexible message types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TLVSizeConstraint {
    /// Fixed size payload (most efficient - zero validation overhead)
    ///
    /// Used for hot-path message types like Trade, Quote where size is always identical.
    /// Parser can skip size validation entirely since struct size is compile-time known.
    Fixed(usize),

    /// Variable size within bounds (single bounds check)
    ///
    /// Used for pool events where base structure is fixed but addresses/IDs vary.
    /// Enables efficient validation with single comparison: min <= size <= max.
    Bounded { min: usize, max: usize },

    /// Variable size with no constraints (accept any size)
    ///
    /// Used for order books, snapshots, and other truly dynamic message types.
    /// Parser accepts any payload size and delegates validation to struct parsing.
    Variable,
}

/// Official TLV type registry for AlphaPulse Protocol V2
///
/// Complete enumeration of all supported message types with domain-based organization
/// enabling automatic relay routing and type-safe message construction.
///
/// ## Domain Organization
/// - **Market Data (1-19)**: High-frequency price/volume → MarketDataRelay
/// - **Strategy Signals (20-39)**: Trading coordination → SignalRelay
/// - **Execution (40-59)**: Order management → ExecutionRelay
/// - **Portfolio/Risk (60-79)**: Risk monitoring → SignalRelay
/// - **System (100-119)**: Infrastructure → SystemRelay
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
pub enum TLVType {
    // ═══════════════════════════════════════════════════════════════════════
    // Market Data Domain (1-19) - Routes through MarketDataRelay
    // ═══════════════════════════════════════════════════════════════════════
    /// Individual trade execution with price, volume, side, timestamp (40 bytes)
    Trade = 1,

    /// Bid/ask quote update with current best prices and sizes (52 bytes)
    Quote = 2,

    /// Order book level data - multiple price levels with quantities (variable)
    OrderBook = 3,

    // Additional market data types
    InstrumentMeta = 4,
    L2Snapshot = 5,
    L2Delta = 6,
    L2Reset = 7,
    PriceUpdate = 8,
    VolumeUpdate = 9,
    PoolLiquidity = 10,
    PoolSwap = 11,
    PoolMint = 12,
    PoolBurn = 13,
    PoolTick = 14,
    PoolState = 15,
    PoolSync = 16,
    GasPrice = 18,

    // ═══════════════════════════════════════════════════════════════════════
    // Strategy Signal Domain (20-39) - Routes through SignalRelay
    // ═══════════════════════════════════════════════════════════════════════
    /// Strategy identification with signal ID and confidence (16 bytes)
    SignalIdentity = 20,

    AssetCorrelation = 21,
    Economics = 22,
    ExecutionAddresses = 23,
    VenueMetadata = 24,
    StateReference = 25,
    ExecutionControl = 26,
    PoolAddresses = 27,
    MEVBundle = 28,
    TertiaryVenue = 29,
    RiskParameters = 30,
    PerformanceMetrics = 31,
    ArbitrageSignal = 32,

    // ═══════════════════════════════════════════════════════════════════════
    // Execution Domain (40-59) - Routes through ExecutionRelay
    // ═══════════════════════════════════════════════════════════════════════
    OrderRequest = 40,
    OrderStatus = 41,
    Fill = 42,
    OrderCancel = 43,
    OrderModify = 44,
    ExecutionReport = 45,
    Portfolio = 46,
    Position = 47,
    Balance = 48,
    TradeConfirmation = 49,

    // ═══════════════════════════════════════════════════════════════════════
    // System Domain (100-119) - Routes through SystemRelay
    // ═══════════════════════════════════════════════════════════════════════
    Heartbeat = 100,
    Snapshot = 101,
    Error = 102,
    ConfigUpdate = 103,
    ServiceDiscovery = 104,
    ResourceUsage = 105,
    StateInvalidation = 106,
    SystemHealth = 107,
    TraceContext = 108,

    // Recovery Domain (110-119)
    RecoveryRequest = 110,
    RecoveryResponse = 111,
    SequenceSync = 112,

    // Extended TLV marker
    ExtendedTLV = 255,
}

impl TLVType {
    /// Get human-readable name for this TLV type
    pub fn name(&self) -> &'static str {
        match *self {
            TLVType::Trade => "Trade",
            TLVType::Quote => "Quote",
            TLVType::OrderBook => "OrderBook",
            TLVType::SignalIdentity => "SignalIdentity",
            TLVType::Economics => "Economics",
            TLVType::ArbitrageSignal => "ArbitrageSignal",
            TLVType::OrderRequest => "OrderRequest",
            TLVType::Fill => "Fill",
            TLVType::Heartbeat => "Heartbeat",
            TLVType::ExtendedTLV => "ExtendedTLV",
            // Add more as needed
            _ => "Unknown",
        }
    }

    /// Get size constraint for payload validation
    pub fn size_constraint(&self) -> TLVSizeConstraint {
        match *self {
            // Fixed size types (hot path - zero validation overhead)
            TLVType::Trade => TLVSizeConstraint::Fixed(40),
            TLVType::Quote => TLVSizeConstraint::Fixed(52),
            TLVType::SignalIdentity => TLVSizeConstraint::Fixed(16),
            TLVType::Economics => TLVSizeConstraint::Fixed(32),
            TLVType::Heartbeat => TLVSizeConstraint::Fixed(16),
            TLVType::GasPrice => TLVSizeConstraint::Fixed(32),

            // Bounded size types (single bounds check)
            TLVType::PoolSwap => TLVSizeConstraint::Bounded { min: 60, max: 200 },
            TLVType::PoolMint => TLVSizeConstraint::Bounded { min: 50, max: 180 },
            TLVType::PoolBurn => TLVSizeConstraint::Bounded { min: 50, max: 180 },
            TLVType::ArbitrageSignal => TLVSizeConstraint::Fixed(168),

            // Variable size types (no constraint)
            TLVType::OrderBook => TLVSizeConstraint::Variable,
            TLVType::InstrumentMeta => TLVSizeConstraint::Variable,
            TLVType::L2Snapshot => TLVSizeConstraint::Variable,

            // Default for unspecified types
            _ => TLVSizeConstraint::Variable,
        }
    }

    /// Check if this TLV type is implemented
    pub fn is_implemented(&self) -> bool {
        // For now, mark core types as implemented
        match *self {
            TLVType::Trade
            | TLVType::Quote
            | TLVType::SignalIdentity
            | TLVType::Economics
            | TLVType::Heartbeat
            | TLVType::GasPrice => true,
            TLVType::ExtendedTLV => false, // Special marker type
            _ => false,                    // Most types are still reserved
        }
    }

    /// Get expected payload size for fixed-size TLV types
    /// 
    /// Returns Some(size) for fixed-size types, None for variable/bounded types.
    /// Used by parser for strict size validation on hot-path message types.
    pub fn expected_payload_size(&self) -> Option<usize> {
        match self.size_constraint() {
            TLVSizeConstraint::Fixed(size) => Some(size),
            _ => None, // Variable and bounded types don't have fixed expected sizes
        }
    }

    /// Get all implemented TLV types
    pub fn all_implemented() -> Vec<TLVType> {
        vec![
            TLVType::Trade,
            TLVType::Quote,
            TLVType::SignalIdentity,
            TLVType::Economics,
            TLVType::Heartbeat,
            TLVType::GasPrice,
        ]
    }
}

/// Registry for TLV type metadata and introspection
pub struct TlvTypeRegistry;

impl TlvTypeRegistry {
    /// Get all available TLV types
    pub fn all_types() -> Vec<TLVType> {
        TLVType::all_implemented()
    }

    /// Validate payload size for given TLV type
    pub fn validate_size(tlv_type: TLVType, payload_size: usize) -> bool {
        match tlv_type.size_constraint() {
            TLVSizeConstraint::Fixed(expected) => payload_size == expected,
            TLVSizeConstraint::Bounded { min, max } => payload_size >= min && payload_size <= max,
            TLVSizeConstraint::Variable => true, // Accept any size
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tlv_type_basic_functionality() {
        let trade_type = TLVType::Trade;
        assert_eq!(trade_type.name(), "Trade");
        assert_eq!(trade_type as u8, 1);
        assert!(trade_type.is_implemented());

        match trade_type.size_constraint() {
            TLVSizeConstraint::Fixed(40) => (), // Expected
            _ => panic!("Trade should be fixed 40 bytes"),
        }
    }

    #[test]
    fn test_size_validation() {
        // Fixed size validation
        assert!(TlvTypeRegistry::validate_size(TLVType::Trade, 40));
        assert!(!TlvTypeRegistry::validate_size(TLVType::Trade, 39));
        assert!(!TlvTypeRegistry::validate_size(TLVType::Trade, 41));

        // Variable size validation (always passes)
        assert!(TlvTypeRegistry::validate_size(TLVType::OrderBook, 100));
        assert!(TlvTypeRegistry::validate_size(TLVType::OrderBook, 1000));
        assert!(TlvTypeRegistry::validate_size(TLVType::OrderBook, 10));
    }

    #[test]
    fn test_try_from_primitive() {
        // Test conversion from u8 to TLVType
        assert_eq!(TLVType::try_from(1u8).unwrap(), TLVType::Trade);
        assert_eq!(TLVType::try_from(2u8).unwrap(), TLVType::Quote);
        assert_eq!(TLVType::try_from(100u8).unwrap(), TLVType::Heartbeat);

        // Test invalid type number
        assert!(TLVType::try_from(99u8).is_err());
    }
}
