#![doc = include_str!("../../../../../.claude/docs/generated/protocol/tlv_type_system.md")]
//!
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
//!
//! ## Performance Profile
//!
//! - **Type Lookup**: O(1) enum-to-integer conversion via compiler optimization
//! - **Domain Mapping**: Compile-time constant folding for range checks
//! - **Size Validation**: Zero-cost for fixed-size types, single bounds check for bounded
//! - **Introspection Cost**: <1μs per type info query (development/debugging only)
//! - **Memory Overhead**: Static string tables + enum metadata (~2KB total)
//! - **Runtime Impact**: Zero - all metadata resolved at compile time where possible
//!
//! ## Type Organization Strategy
//!
//! ### Domain-Based Numeric Ranges
//! - **Market Data (1-19)**: High-frequency price/volume data → MarketDataRelay
//! - **Strategy Signals (20-39)**: Trading logic coordination → SignalRelay
//! - **Execution (40-59)**: Order lifecycle management → ExecutionRelay
//! - **Portfolio/Risk (60-79)**: Risk monitoring → SignalRelay (analytics)
//! - **Compliance/Audit (80-99)**: Regulatory tracking → SystemRelay
//! - **System (100-119)**: Infrastructure messaging → SystemRelay
//! - **Vendor (200-254)**: Custom/experimental types → ConfigurableRelay
//! - **Extended (255)**: Large payload marker → Any domain
//!
//! ### Size Constraint Strategy
//! - **Fixed**: Critical hot path types (Trade=40B, Economics=32B) - zero validation overhead
//! - **Bounded**: Pool events with variable addresses (60-200B) - single bounds check
//! - **Variable**: Order books, snapshots - dynamic allocation with careful usage
//!
//! ## Examples
//!
//! ### Basic Type Discovery
//! ```rust
//! use protocol_v2::tlv::TLVType;
//! use protocol_v2::RelayDomain;
//!
//! // Get comprehensive type information
//! let info = TLVType::Trade.type_info();
//! println!("Type {}: {} - {}", info.type_number, info.name, info.description);
//! println!("Routes to: {:?}, Size: {:?}", info.relay_domain, info.size_constraint);
//!
//! // Query types by relay domain for service logic
//! let market_types = TLVType::types_in_domain(RelayDomain::MarketData);
//! println!("Market data relay handles {} message types", market_types.len());
//!
//! // Development workflow
//! println!("Trade type implemented: {}", TLVType::Trade.is_implemented());
//! ```
//!
//! ### Documentation Generation
//! ```rust
//! // Auto-generate complete API documentation
//! let markdown = TLVType::generate_markdown_table();
//! std::fs::write("docs/message-types.md", markdown)?;
//! println!("Generated documentation for {} types", TLVType::all_implemented().len());
//! ```
//!
//! ### Runtime Message Handling
//! ```rust
//! use protocol_v2::tlv::{TLVType, TLVSizeConstraint};
//!
//! // Size validation during parsing
//! let tlv_type = TLVType::try_from(message_type)?;
//! match tlv_type.size_constraint() {
//!     TLVSizeConstraint::Fixed(expected) => {
//!         // Hot path: no validation needed for fixed types like Trade
//!         assert_eq!(payload.len(), expected);
//!     },
//!     TLVSizeConstraint::Bounded { min, max } => {
//!         // Bounded types: single validation for pool events
//!         if payload.len() < min || payload.len() > max {
//!             return Err(ParseError::InvalidSize);
//!         }
//!     },
//!     TLVSizeConstraint::Variable => {
//!         // Variable types: accept any size for order books
//!     }
//! }
//! ```
//!
//! ### Service Integration
//! ```rust
//! // Relay service automatically routes based on type number
//! let relay_domain = TLVType::PoolSwap.relay_domain();
//! match relay_domain {
//!     RelayDomain::MarketData => send_to_market_relay(message),
//!     RelayDomain::Signal => send_to_signal_relay(message),
//!     RelayDomain::Execution => send_to_execution_relay(message),
//!     RelayDomain::System => send_to_system_relay(message),
//! }
//! ```

use num_enum::TryFromPrimitive;
// use chrono::Utc; // Temporarily removed for validation test
use super::super::RelayDomain;

/// Complete metadata for TLV type introspection and development tools
///
/// Comprehensive type information enabling IDE integration, documentation generation,
/// and runtime service discovery. Each TLV type provides rich metadata including
/// routing domain, size constraints, implementation status, and usage examples.
///
/// # Fields
/// - **type_number**: Unique identifier (1-255) for protocol parsing
/// - **name**: Human-readable type name for development and debugging
/// - **description**: Detailed purpose and data content explanation
/// - **relay_domain**: Automatic routing destination for message distribution
/// - **size_constraint**: Validation rules for payload size checking
/// - **status**: Implementation maturity and availability
/// - **examples**: Practical usage patterns and integration examples
///
/// # Performance Impact
/// This struct is used exclusively for development, debugging, and documentation.
/// It has zero runtime cost in hot paths - only accessed for introspection.
#[derive(Debug, Clone)]
pub struct TLVTypeInfo {
    /// TLV type number (1-255) for wire protocol identification
    pub type_number: u8,
    /// Human-readable name for development tools and logging
    pub name: &'static str,
    /// Detailed description of message purpose and content structure
    pub description: &'static str,
    /// Relay domain for automatic message routing
    pub relay_domain: crate::RelayDomain,
    /// Size validation constraint for parsing safety
    pub size_constraint: TLVSizeConstraint,
    /// Current implementation and availability status
    pub status: TLVImplementationStatus,
    /// Real-world usage examples and integration patterns
    pub examples: Vec<&'static str>,
}

/// Implementation maturity and availability status for TLV types
///
/// Tracks the development lifecycle of each TLV type from allocation through
/// production deployment. Used by development tools to provide accurate
/// information about type availability and guide implementation decisions.
///
/// # Lifecycle Progression
/// 1. **Reserved**: Type number allocated, implementation pending
/// 2. **Implemented**: Full zero-copy serialization with comprehensive tests
/// 3. **Vendor**: Available for custom/experimental extensions
/// 4. **Extended**: Special marker type for large payload handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TLVImplementationStatus {
    /// Production-ready with zero-copy serialization and comprehensive test coverage
    ///
    /// Characteristics:
    /// - Full zerocopy::AsBytes and FromBytes trait implementation
    /// - Comprehensive unit and integration tests
    /// - Performance benchmarks and validation
    /// - Documentation and usage examples
    /// - Stable API surface (breaking changes coordinated)
    Implemented,

    /// Type number allocated but implementation not yet complete
    ///
    /// Characteristics:
    /// - Type number reserved in enum but no struct definition
    /// - Placeholder for future protocol extensions
    /// - Used for planning and avoiding number conflicts
    /// - Safe to reference but will cause compile errors if used
    Reserved,

    /// Available for vendor-specific or experimental functionality
    ///
    /// Characteristics:
    /// - Type numbers 200-254 available for custom use
    /// - No standard protocol definition - vendor responsibility
    /// - Useful for prototyping new message types
    /// - Can be promoted to standard types in future protocol versions
    Vendor,

    /// Special extended format marker for large payloads (type 255)
    ///
    /// Characteristics:
    /// - Not a message type itself but a format indicator
    /// - Enables >255 byte payloads with 5-byte header
    /// - Embeds actual type number within extended header
    /// - Automatically selected by TLVMessageBuilder for large payloads
    Extended,
}

/// TLV payload size validation constraints with performance characteristics
///
/// Defines validation rules for TLV payload sizes with direct impact on parsing
/// performance and memory allocation patterns. The constraint type determines
/// both validation overhead and optimal usage patterns in hot paths.
///
/// # Performance Impact by Constraint Type
///
/// ## Fixed
/// - **Validation Cost**: Zero (compile-time known size)
/// - **Memory Pattern**: Predictable allocation for pre-sized buffers
/// - **Cache Behavior**: Optimal - consistent memory access patterns
/// - **Use Case**: High-frequency trading data (Trade, Economics, Heartbeat)
///
/// ## Bounded
/// - **Validation Cost**: Single comparison (~1ns)
/// - **Memory Pattern**: Range-based allocation with worst-case planning
/// - **Cache Behavior**: Good - limited size variation reduces fragmentation
/// - **Use Case**: Pool events with variable-length addresses/identifiers
///
/// ## Variable
/// - **Validation Cost**: Zero (accepts any size)
/// - **Memory Pattern**: Dynamic allocation - potential fragmentation
/// - **Cache Behavior**: Unpredictable - large payloads may cause cache misses
/// - **Use Case**: Order books, snapshots, batch operations (non-hot path)
///
/// # Constraint Selection Guidelines
/// - Choose **Fixed** for hot path messages with known structure
/// - Choose **Bounded** for semi-structured data with reasonable limits
/// - Choose **Variable** only when flexibility is essential and performance is secondary
#[derive(Debug, Clone, PartialEq)]
pub enum TLVSizeConstraint {
    /// Exact byte count required - optimal for hot path performance
    ///
    /// Zero validation overhead as size is known at compile time.
    /// Enables pre-allocation and optimal cache behavior.
    Fixed(usize),

    /// Minimum and maximum byte counts - single bounds check validation
    ///
    /// Good balance between flexibility and performance. Single comparison
    /// validates payload falls within acceptable range.
    Bounded { min: usize, max: usize },

    /// No size restrictions - maximum flexibility with potential performance cost
    ///
    /// Accepts any payload size but may require dynamic allocation and
    /// cause cache misses for very large payloads. Use sparingly in hot paths.
    Variable,
}

/// Protocol V2 TLV message type enumeration with domain-based organization
///
/// Central registry of all message types in the Torq trading system, organized
/// by relay domain for automatic routing and performance optimization. Each type
/// number uniquely identifies message format and determines processing path through
/// the system architecture.
///
/// # Domain Organization Philosophy
///
/// Types are grouped by processing characteristics and routing destinations:
/// - **Hot Path Types (1-19)**: Market data requiring <35μs processing
/// - **Coordination Types (20-39)**: Strategy signals with medium latency tolerance
/// - **Execution Types (40-59)**: Order management with strict reliability requirements
/// - **Analytics Types (60-79)**: Risk/portfolio monitoring with batch processing
/// - **System Types (100-119)**: Infrastructure messaging with highest priority
/// - **Vendor Types (200-254)**: Custom extensions with configurable routing
///
/// # Type Number Allocation Strategy
///
/// **Numeric Ranges**:
/// - 1-19: MarketData → High frequency, performance critical
/// - 20-39: Signal → Strategy coordination, medium frequency
/// - 40-59: Execution → Order lifecycle, reliability critical
/// - 60-79: Signal → Portfolio/risk monitoring (routed as analytics)
/// - 80-99: System → Compliance/audit (routed as system)
/// - 100-119: System → Infrastructure, highest priority
/// - 200-254: Vendor → Custom/experimental, configurable routing
/// - 255: Extended → Large payload format marker
///
/// **Gap Management**: Reserved ranges (17-19, 33-39, 50-59, etc.) enable
/// future expansion without renumbering existing types.
///
/// # Performance Characteristics by Type
///
/// - **Fixed-Size Types**: Zero parsing overhead, optimal for hot paths
///   - Examples: Trade (40B), Economics (32B), Heartbeat (16B)
///   - Target: >1M msg/s processing for real-time feeds
///
/// - **Bounded Types**: Single bounds check, good for variable identifiers
///   - Examples: PoolSwap (60-200B), OrderRequest (32B fixed)
///   - Target: >100K msg/s for pool events and orders
///
/// - **Variable Types**: Dynamic allocation, use in batch processing
///   - Examples: OrderBook (unlimited), L2Snapshot (large)
///   - Target: Flexibility over raw speed, careful memory management
///
/// # Routing and Service Integration
///
/// ```rust
/// // Automatic routing based on type number ranges
/// let message_type = TLVType::PoolSwap;
/// match message_type.relay_domain() {
///     RelayDomain::MarketData => route_to_market_relay(message),
///     RelayDomain::Signal => route_to_signal_relay(message),
///     RelayDomain::Execution => route_to_execution_relay(message),
///     RelayDomain::System => route_to_system_relay(message),
/// }
///
/// // Service discovery and filtering
/// let execution_types = TLVType::types_in_domain(RelayDomain::Execution);
/// let execution_handler = ExecutionService::new(execution_types);
/// ```
///
/// # Development and Maintenance
///
/// The type system provides comprehensive introspection for:
/// - **IDE Integration**: Type information, usage examples, constraints
/// - **Documentation Generation**: Auto-updating API references
/// - **Testing Validation**: Ensure all types have proper test coverage
/// - **Performance Monitoring**: Track processing characteristics per type
/// - **Protocol Evolution**: Manage type additions without breaking existing code
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
pub enum TLVType {
    // ═══════════════════════════════════════════════════════════════════════
    // Market Data Domain (1-19) - Routes through MarketDataRelay
    // ═══════════════════════════════════════════════════════════════════════
    /// **Trade execution event** - Individual trade with price, volume, side, timestamp
    ///
    /// Fixed 40 bytes: venue_id(2) + instrument_id(16) + price(8) + volume(8) + side(1) + timestamp(8) + padding(3)
    ///
    /// Used for: Real-time price feeds, volume analysis, trade history
    Trade = 1,

    /// **Bid/Ask quote update** - Current best bid/offer with sizes
    ///
    /// Fixed 52 bytes: venue_id + instrument_id + bid_price + bid_size + ask_price + ask_size + timestamp
    ///
    /// Used for: Spread analysis, quote-driven trading, market making
    Quote = 2,

    /// **Order book level data** - Multiple price levels with quantities
    ///
    /// Variable size: Unlimited levels for deep market analysis
    ///
    /// Used for: Full market depth, algorithmic trading, liquidity analysis
    OrderBook = 3,
    InstrumentMeta = 4,
    L2Snapshot = 5,
    L2Delta = 6,
    L2Reset = 7,
    PriceUpdate = 8,
    VolumeUpdate = 9,
    PoolLiquidity = 10,
    PoolSwap = 11,                // Swap event with V3 state updates
    PoolMint = 12,                // Liquidity add event
    PoolBurn = 13,                // Liquidity remove event
    PoolTick = 14,                // Tick crossing event (V3)
    PoolState = 15,               // Pool state snapshot (full state)
    PoolSync = 16,                // V2 Sync event (complete reserves)
    QuoteUpdate = 17,             // Quote update events (GAP-001 implementation)
    GasPrice = 18,                // Gas price updates from WebSocket stream (Market Data domain)
    StateInvalidationReason = 19, // State invalidation reasons (GAP-001 implementation)

    // Strategy Signal Domain (20-39) - Routes through SignalRelay
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
    ArbitrageSignal = 32, // Real arbitrage opportunity signal
    // Reserved 33-39 for future strategy signal types

    // Execution Domain (40-59) - Routes through ExecutionRelay
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
    // Reserved 50-59 for future execution types

    // Portfolio-Risk Domain (60-79) - Routes through SignalRelay (monitoring/analytics)
    RiskDecision = 60,         // Risk approval/rejection decisions
    PositionUpdate = 61,       // Portfolio position changes
    RiskAlert = 62,            // Risk threshold breaches
    CollateralUpdate = 63,     // Margin/collateral changes
    ExposureReport = 64,       // Net exposure by asset/venue
    PortfolioEvent = 65,       // P&L, allocation changes
    RiskThreshold = 66,        // Stop-loss, position limits
    FlashLoanResult = 67,      // Self-contained strategy results
    PostTradeAnalytics = 68,   // Execution analysis
    PositionQuery = 69,        // Position requests
    RiskMetrics = 70,          // Risk calculations
    CircuitBreaker = 71,       // Emergency controls
    StrategyRegistration = 72, // Strategy declarations
    // Reserved 73-79 for future portfolio/risk types

    // Compliance-Audit Domain (80-99) - Routes through SystemRelay
    ComplianceCheck = 80,       // Regulatory validation
    AuditTrail = 81,            // Trade audit records
    RegulatoryReport = 82,      // Compliance reporting
    TradeBreakdown = 83,        // Detailed trade analysis
    SettlementInstruction = 84, // Cross-venue settlement
    ClearingReport = 85,        // Trade clearing status
    MarginCall = 86,            // Margin requirements
    CustodyEvent = 87,          // Asset custody transfers
    VenueConnectivity = 88,     // Venue latency/health status
    CrossVenueSpread = 89,      // Real-time spread monitoring
    ArbitrageExecution = 90,    // Cross-venue execution coordination
    // Reserved 91-99 for future compliance/audit types

    // System Domain (100-119) - Routes through SystemRelay
    Heartbeat = 100,         // Keep existing
    Snapshot = 101,          // Keep existing
    Error = 102,             // Keep existing
    ConfigUpdate = 103,      // Keep existing
    ServiceDiscovery = 104,  // Keep original - enhanced later
    ResourceUsage = 105,     // Replace MetricsReport
    StateInvalidation = 106, // Keep original - state invalidation events
    SystemHealth = 107,      // New - overall system health
    TraceContext = 108,      // New - distributed tracing
    // Reserved 109 for future system types

    // Recovery Domain (110-119)
    RecoveryRequest = 110,
    RecoveryResponse = 111,
    SequenceSync = 112,
    // Reserved 113-119 for future recovery types

    // Extended TLV marker (255)
    ExtendedTLV = 255,
}

/// Deprecated TLV types that should not be used in new code
///
/// These types are maintained for documentation and error handling purposes.
/// The message converter can provide meaningful deprecation messages when
/// encountering these types instead of using magic numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeprecatedTLVType {
    /// DemoDeFiArbitrageTLV - Type 255 was abused for demo purposes
    ///
    /// This type violated protocol specifications by using the ExtendedTLV
    /// marker for application data. Replaced by ArbitrageSignalTLV (type 32).
    DemoDeFiArbitrage = 255,
}

impl DeprecatedTLVType {
    /// Check if a type number represents a deprecated TLV type
    pub fn is_deprecated(type_number: u8) -> bool {
        matches!(type_number, 255) // Add more deprecated types here as needed
    }

    /// Get deprecation message for a type number
    pub fn deprecation_message(type_number: u8) -> Option<&'static str> {
        match type_number {
            255 => {
                Some("Type 255 (DemoDeFiArbitrageTLV) removed - use type 32 (ArbitrageSignalTLV)")
            }
            _ => None,
        }
    }
}

impl TLVType {
    // ═══════════════════════════════════════════════════════════════════════
    // Developer API - Rich introspection for development and documentation
    // ═══════════════════════════════════════════════════════════════════════

    /// Get complete type information for development and documentation
    ///
    /// Returns rich metadata including size constraints, routing domain, implementation
    /// status, and usage examples. Perfect for IDE tooltips and auto-generated docs.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use protocol_v2::tlv::TLVType;
    ///
    /// let info = TLVType::Trade.type_info();
    /// println!("Type {}: {}", info.type_number, info.name);
    /// println!("  Size: {:?}", info.size_constraint);
    /// println!("  Routes to: {:?}", info.relay_domain);
    /// println!("  Status: {:?}", info.status);
    /// ```
    pub fn type_info(&self) -> TLVTypeInfo {
        TLVTypeInfo {
            type_number: *self as u8,
            name: self.name(),
            description: self.description(),
            relay_domain: self.relay_domain(),
            size_constraint: self.size_constraint(),
            status: self.implementation_status(),
            examples: self.usage_examples(),
        }
    }

    /// Get human-readable name of this TLV type
    ///
    /// Returns the enum variant name (e.g., "Trade", "PoolSwap", "OrderRequest")
    pub fn name(&self) -> &'static str {
        match self {
            TLVType::Trade => "Trade",
            TLVType::Quote => "Quote",
            TLVType::OrderBook => "OrderBook",
            TLVType::InstrumentMeta => "InstrumentMeta",
            TLVType::L2Snapshot => "L2Snapshot",
            TLVType::L2Delta => "L2Delta",
            TLVType::L2Reset => "L2Reset",
            TLVType::PriceUpdate => "PriceUpdate",
            TLVType::VolumeUpdate => "VolumeUpdate",
            TLVType::PoolLiquidity => "PoolLiquidity",
            TLVType::PoolSwap => "PoolSwap",
            TLVType::PoolMint => "PoolMint",
            TLVType::PoolBurn => "PoolBurn",
            TLVType::PoolTick => "PoolTick",
            TLVType::PoolState => "PoolState",
            TLVType::PoolSync => "PoolSync",
            TLVType::QuoteUpdate => "QuoteUpdate",
            TLVType::GasPrice => "GasPrice",
            TLVType::StateInvalidationReason => "StateInvalidationReason",
            TLVType::SignalIdentity => "SignalIdentity",
            TLVType::AssetCorrelation => "AssetCorrelation",
            TLVType::Economics => "Economics",
            TLVType::ExecutionAddresses => "ExecutionAddresses",
            TLVType::VenueMetadata => "VenueMetadata",
            TLVType::StateReference => "StateReference",
            TLVType::ExecutionControl => "ExecutionControl",
            TLVType::PoolAddresses => "PoolAddresses",
            TLVType::MEVBundle => "MEVBundle",
            TLVType::TertiaryVenue => "TertiaryVenue",
            TLVType::RiskParameters => "RiskParameters",
            TLVType::PerformanceMetrics => "PerformanceMetrics",
            TLVType::ArbitrageSignal => "ArbitrageSignal",
            TLVType::OrderRequest => "OrderRequest",
            TLVType::OrderStatus => "OrderStatus",
            TLVType::Fill => "Fill",
            TLVType::OrderCancel => "OrderCancel",
            TLVType::OrderModify => "OrderModify",
            TLVType::ExecutionReport => "ExecutionReport",
            TLVType::Portfolio => "Portfolio",
            TLVType::Position => "Position",
            TLVType::Balance => "Balance",
            TLVType::TradeConfirmation => "TradeConfirmation",
            TLVType::Heartbeat => "Heartbeat",
            TLVType::Snapshot => "Snapshot",
            TLVType::Error => "Error",
            TLVType::ConfigUpdate => "ConfigUpdate",
            TLVType::ServiceDiscovery => "ServiceDiscovery",
            TLVType::RecoveryRequest => "RecoveryRequest",
            TLVType::RecoveryResponse => "RecoveryResponse",
            TLVType::SequenceSync => "SequenceSync",
            TLVType::ExtendedTLV => "ExtendedTLV",
            // Add more as needed - this is just a sample of key types
            _ => "Unknown",
        }
    }

    /// Get brief description of this TLV type's purpose
    pub fn description(&self) -> &'static str {
        match self {
            TLVType::Trade => "Individual trade execution with price, volume, side, timestamp",
            TLVType::Quote => "Bid/ask quote update with current best prices and sizes",
            TLVType::OrderBook => "Multiple price levels with quantities for market depth",
            TLVType::PoolSwap => "DEX swap event with V3 state updates and reserves",
            TLVType::QuoteUpdate => "Best bid/ask update with sizes for order book maintenance",
            TLVType::GasPrice => "Ethereum gas price updates for transaction cost optimization",
            TLVType::StateInvalidationReason => {
                "Reason for state invalidation (disconnection, rate limit, etc.)"
            }
            TLVType::SignalIdentity => "Strategy identification with signal ID and confidence",
            TLVType::Economics => "Profit estimates and capital requirements for execution",
            TLVType::ArbitrageSignal => {
                "Real arbitrage opportunity with pool addresses and profit metrics"
            }
            TLVType::OrderRequest => "Order placement request with type, quantity, limits",
            TLVType::Fill => "Execution confirmation with actual price, quantity, fees",
            TLVType::Heartbeat => "Service health check with timestamp and status",
            _ => "TLV message type - see documentation for details",
        }
    }

    /// Get implementation status of this TLV type
    pub fn implementation_status(&self) -> TLVImplementationStatus {
        if self.is_reserved() {
            TLVImplementationStatus::Reserved
        } else if *self == TLVType::ExtendedTLV {
            TLVImplementationStatus::Extended
        } else if VendorTLVType::is_vendor_type(*self as u8) {
            TLVImplementationStatus::Vendor
        } else {
            TLVImplementationStatus::Implemented
        }
    }

    /// Get usage examples for this TLV type
    pub fn usage_examples(&self) -> Vec<&'static str> {
        match self {
            TLVType::Trade => vec![
                "Real-time price feeds from exchanges",
                "Trade history analysis and reporting",
                "Volume-weighted average price calculations",
            ],
            TLVType::PoolSwap => vec![
                "DEX arbitrage opportunity detection",
                "Pool state tracking for AMM strategies",
                "Cross-pool liquidity analysis",
            ],
            TLVType::SignalIdentity => vec![
                "Strategy signal routing and attribution",
                "Multi-strategy portfolio coordination",
                "Signal confidence scoring and filtering",
            ],
            _ => vec!["See Protocol V2 documentation for usage patterns"],
        }
    }

    /// Check if this TLV type is fully implemented
    pub fn is_implemented(&self) -> bool {
        matches!(
            self.implementation_status(),
            TLVImplementationStatus::Implemented
        )
    }

    /// Get all TLV types in a specific relay domain
    ///
    /// Useful for routing logic and domain-specific processing
    ///
    /// # Examples
    ///
    /// ```rust
    /// use protocol_v2::tlv::TLVType;
    /// use protocol_v2::RelayDomain;
    ///
    /// let market_types = TLVType::types_in_domain(RelayDomain::MarketData);
    /// println!("Market data domain has {} types", market_types.len());
    ///
    /// for tlv_type in market_types {
    ///     println!("  Type {}: {}", tlv_type as u8, tlv_type.name());
    /// }
    /// ```
    pub fn types_in_domain(domain: crate::RelayDomain) -> Vec<TLVType> {
        Self::all_implemented()
            .into_iter()
            .filter(|t| t.relay_domain() == domain)
            .collect()
    }

    /// Get all implemented TLV types (excludes reserved ranges)
    pub fn all_implemented() -> Vec<TLVType> {
        vec![
            // Market Data Domain (1-19)
            TLVType::Trade,
            TLVType::Quote,
            TLVType::OrderBook,
            TLVType::InstrumentMeta,
            TLVType::L2Snapshot,
            TLVType::L2Delta,
            TLVType::L2Reset,
            TLVType::PriceUpdate,
            TLVType::VolumeUpdate,
            TLVType::PoolLiquidity,
            TLVType::PoolSwap,
            TLVType::PoolMint,
            TLVType::PoolBurn,
            TLVType::PoolTick,
            TLVType::PoolState,
            TLVType::PoolSync,
            TLVType::QuoteUpdate,
            TLVType::GasPrice,
            TLVType::StateInvalidationReason,
            // Strategy Signal Domain (20-39)
            TLVType::SignalIdentity,
            TLVType::AssetCorrelation,
            TLVType::Economics,
            TLVType::ExecutionAddresses,
            TLVType::VenueMetadata,
            TLVType::StateReference,
            TLVType::ExecutionControl,
            TLVType::PoolAddresses,
            TLVType::MEVBundle,
            TLVType::TertiaryVenue,
            TLVType::RiskParameters,
            TLVType::PerformanceMetrics,
            // Execution Domain (40-59)
            TLVType::OrderRequest,
            TLVType::OrderStatus,
            TLVType::Fill,
            TLVType::OrderCancel,
            TLVType::OrderModify,
            TLVType::ExecutionReport,
            TLVType::Portfolio,
            TLVType::Position,
            TLVType::Balance,
            TLVType::TradeConfirmation,
            // System Domain (100-119)
            TLVType::Heartbeat,
            TLVType::Snapshot,
            TLVType::Error,
            TLVType::ConfigUpdate,
            TLVType::ServiceDiscovery,
            TLVType::RecoveryRequest,
            TLVType::RecoveryResponse,
            TLVType::SequenceSync,
            // Extended
            TLVType::ExtendedTLV,
        ]
    }

    /// Generate markdown table for auto-updating documentation
    ///
    /// Creates the complete message-types.md content with all TLV types organized
    /// by domain. This ensures documentation stays in sync with code changes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use protocol_v2::tlv::TLVType;
    ///
    /// let markdown = TLVType::generate_markdown_table();
    /// std::fs::write("docs/message-types.md", markdown).expect("Write failed");
    /// println!("Updated message-types.md with {} types", TLVType::all_implemented().len());
    /// ```
    pub fn generate_markdown_table() -> String {
        let mut output = String::new();

        // Header
        output.push_str("# Torq Protocol V2 - Message Types Reference\n\n");
        output.push_str("**⚠️ This file is auto-generated from `protocol_v2/src/tlv/types.rs` - DO NOT EDIT MANUALLY**\n\n");
        output.push_str("This document provides a comprehensive index of all TLV message types defined in the Torq Protocol V2.\n\n");

        // Overview stats
        let all_types = Self::all_implemented();
        let market_types = Self::types_in_domain(RelayDomain::MarketData);
        let signal_types = Self::types_in_domain(RelayDomain::Signal);
        let execution_types = Self::types_in_domain(RelayDomain::Execution);
        let system_types = Self::types_in_domain(RelayDomain::System);

        output.push_str("## Overview\n\n");
        output.push_str(&format!(
            "- **Total Types**: {} implemented\n",
            all_types.len()
        ));
        output.push_str(&format!(
            "- **Market Data**: {} types (1-19)\n",
            market_types.len()
        ));
        output.push_str(&format!(
            "- **Strategy Signals**: {} types (20-39)\n",
            signal_types.len()
        ));
        output.push_str(&format!(
            "- **Execution**: {} types (40-59)\n",
            execution_types.len()
        ));
        output.push_str(&format!(
            "- **System**: {} types (100-119)\n",
            system_types.len()
        ));
        output.push('\n');

        // Generate domain tables
        Self::generate_domain_table(
            &mut output,
            "Market Data Domain (Types 1-19)",
            "Routes through MarketDataRelay",
            &market_types,
        );

        Self::generate_domain_table(
            &mut output,
            "Strategy Signal Domain (Types 20-39)",
            "Routes through SignalRelay",
            &signal_types,
        );

        Self::generate_domain_table(
            &mut output,
            "Execution Domain (Types 40-59)",
            "Routes through ExecutionRelay",
            &execution_types,
        );

        Self::generate_domain_table(
            &mut output,
            "System Domain (Types 100-119)",
            "Routes through SystemRelay",
            &system_types,
        );

        // Usage examples
        output.push_str("## Usage Examples\n\n");
        output.push_str("### Querying Types by Domain\n");
        output.push_str("```rust\n");
        output.push_str("use protocol_v2::tlv::TLVType;\n");
        output.push_str("use protocol_v2::RelayDomain;\n\n");
        output.push_str("// Get all market data types\n");
        output.push_str("let market_types = TLVType::types_in_domain(RelayDomain::MarketData);\n");
        output.push_str("for tlv_type in market_types {\n");
        output.push_str("    let info = tlv_type.type_info();\n");
        output.push_str("    println!(\"{}: {}\", info.name, info.description);\n");
        output.push_str("}\n");
        output.push_str("```\n\n");

        output.push_str("### Type Information API\n");
        output.push_str("```rust\n");
        output.push_str("let trade_info = TLVType::Trade.type_info();\n");
        output.push_str("println!(\"Type {}: {} bytes\", trade_info.type_number, \n");
        output.push_str("         match trade_info.size_constraint {\n");
        output.push_str("             TLVSizeConstraint::Fixed(size) => size.to_string(),\n");
        output.push_str("             _ => \"Variable\".to_string()\n");
        output.push_str("         });\n");
        output.push_str("```\n\n");

        // Footer
        output.push_str("---\n");
        output.push_str("*Generated automatically from code*\n");

        output
    }

    /// Generate markdown table for a specific domain
    fn generate_domain_table(output: &mut String, title: &str, routing: &str, types: &[TLVType]) {
        output.push_str(&format!("## {}\n", title));
        output.push_str(&format!("*{}*\n\n", routing));

        // Table header
        output.push_str("| Type | Name | Description | Size | Status |\n");
        output.push_str("|------|------|-------------|------|---------|\n");

        // Table rows
        for tlv_type in types {
            let info = tlv_type.type_info();
            let size_str = match info.size_constraint {
                TLVSizeConstraint::Fixed(size) => format!("{} bytes", size),
                TLVSizeConstraint::Bounded { min, max } => format!("{}-{} bytes", min, max),
                TLVSizeConstraint::Variable => "Variable".to_string(),
            };

            let status_str = match info.status {
                TLVImplementationStatus::Implemented => "Implemented",
                TLVImplementationStatus::Reserved => "Reserved",
                TLVImplementationStatus::Vendor => "Vendor",
                TLVImplementationStatus::Extended => "Extended",
            };

            output.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                info.type_number, info.name, info.description, size_str, status_str
            ));
        }

        output.push('\n');
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Core Protocol Methods (existing)
    // ═══════════════════════════════════════════════════════════════════════

    /// Get the relay domain for this TLV type
    pub fn relay_domain(&self) -> crate::RelayDomain {
        match *self as u8 {
            1..=19 => crate::RelayDomain::MarketData, // Market data events
            20..=39 => crate::RelayDomain::Signal,    // Strategy signals
            40..=59 => crate::RelayDomain::Execution, // Order execution
            60..=79 => crate::RelayDomain::Signal, // Portfolio/Risk → Signal (monitoring/analytics)
            80..=99 => crate::RelayDomain::System, // Compliance → System
            100..=119 => crate::RelayDomain::System, // System/Recovery
            _ => crate::RelayDomain::MarketData,   // Default fallback
        }
    }

    /// Check if this is a standard TLV type (not extended)
    pub fn is_standard(&self) -> bool {
        *self != TLVType::ExtendedTLV
    }

    /// Check if this TLV type is reserved/undefined
    pub fn is_reserved(&self) -> bool {
        match *self as u8 {
            // Market Data Domain (1-19): Reserved 17-19
            17..=19 => true,
            // Strategy Signal Domain (20-39): Reserved 33-39
            33..=39 => true,
            // Execution Domain (40-59): Reserved 50-59
            50..=59 => true,
            // Portfolio-Risk Domain (60-79): Reserved 73-79
            73..=79 => true,
            // Compliance-Audit Domain (80-99): Reserved 91-99
            91..=99 => true,
            // System Domain (100-119): Reserved 109, 113-119
            109 | 113..=119 => true,
            // Unallocated ranges: 120-199
            120..=199 => true,
            // Vendor range: 200-254 (not reserved, available for use)
            // Extended TLV: 255 (not reserved)
            _ => false,
        }
    }

    /// Get size constraint for TLV validation
    pub fn size_constraint(&self) -> TLVSizeConstraint {
        match self {
            // Fixed-size TLVs (using zerocopy structs)
            TLVType::Trade => TLVSizeConstraint::Fixed(40),
            TLVType::Quote => TLVSizeConstraint::Fixed(52),
            TLVType::SignalIdentity => TLVSizeConstraint::Fixed(16),
            TLVType::AssetCorrelation => TLVSizeConstraint::Fixed(24),
            TLVType::Economics => TLVSizeConstraint::Fixed(32),
            TLVType::ExecutionAddresses => TLVSizeConstraint::Fixed(84),
            TLVType::VenueMetadata => TLVSizeConstraint::Fixed(12),
            TLVType::StateReference => TLVSizeConstraint::Fixed(24),
            TLVType::ExecutionControl => TLVSizeConstraint::Fixed(16),
            TLVType::PoolAddresses => TLVSizeConstraint::Fixed(44),
            TLVType::MEVBundle => TLVSizeConstraint::Fixed(40),
            TLVType::TertiaryVenue => TLVSizeConstraint::Fixed(24),
            TLVType::ArbitrageSignal => TLVSizeConstraint::Fixed(168),
            TLVType::OrderRequest => TLVSizeConstraint::Fixed(32),
            TLVType::OrderStatus => TLVSizeConstraint::Fixed(24),
            TLVType::Fill => TLVSizeConstraint::Fixed(32),
            TLVType::OrderCancel => TLVSizeConstraint::Fixed(16),
            TLVType::OrderModify => TLVSizeConstraint::Fixed(24),
            TLVType::ExecutionReport => TLVSizeConstraint::Fixed(48),
            TLVType::Heartbeat => TLVSizeConstraint::Fixed(16),
            TLVType::RecoveryRequest => TLVSizeConstraint::Fixed(24),

            // Pool TLVs - bounded size due to variable-length PoolInstrumentId
            TLVType::PoolSwap => TLVSizeConstraint::Bounded { min: 60, max: 200 }, // Base: ~60, Pool ID can vary
            TLVType::PoolMint => TLVSizeConstraint::Bounded { min: 50, max: 180 }, // Updated for decimal fields
            TLVType::PoolBurn => TLVSizeConstraint::Bounded { min: 50, max: 180 }, // Updated for decimal fields
            TLVType::PoolSync => TLVSizeConstraint::Bounded { min: 40, max: 150 }, // Updated for decimal fields
            TLVType::PoolState => TLVSizeConstraint::Bounded { min: 60, max: 200 }, // Pool state snapshot
            TLVType::PoolTick => TLVSizeConstraint::Bounded { min: 30, max: 120 },  // Tick crossing
            TLVType::PoolLiquidity => TLVSizeConstraint::Bounded { min: 20, max: 300 }, // Variable reserves count

            // Truly variable-size TLVs
            TLVType::OrderBook => TLVSizeConstraint::Variable, // Unlimited order levels
            TLVType::InstrumentMeta => TLVSizeConstraint::Variable, // Variable metadata
            TLVType::L2Snapshot => TLVSizeConstraint::Variable, // Full order book snapshot
            TLVType::L2Delta => TLVSizeConstraint::Variable,   // Variable delta updates
            TLVType::L2Reset => TLVSizeConstraint::Variable,   // Variable reset data
            TLVType::PriceUpdate => TLVSizeConstraint::Variable, // Variable price data
            TLVType::VolumeUpdate => TLVSizeConstraint::Variable, // Variable volume data
            TLVType::QuoteUpdate => TLVSizeConstraint::Fixed(52), // QuoteTLV size (52 bytes verified)
            TLVType::GasPrice => TLVSizeConstraint::Fixed(32), // Gas price updates (32 bytes as verified in gas_price.rs)
            TLVType::StateInvalidationReason => TLVSizeConstraint::Fixed(1), // Single byte enum

            // System TLVs (100-119)
            TLVType::Snapshot => TLVSizeConstraint::Bounded { min: 32, max: 1024 },
            TLVType::Error => TLVSizeConstraint::Bounded { min: 16, max: 512 },
            TLVType::ConfigUpdate => TLVSizeConstraint::Bounded { min: 20, max: 2048 },
            TLVType::ServiceDiscovery => TLVSizeConstraint::Bounded { min: 24, max: 512 },
            TLVType::ResourceUsage => TLVSizeConstraint::Fixed(64),
            TLVType::StateInvalidation => TLVSizeConstraint::Bounded { min: 16, max: 512 },
            TLVType::SystemHealth => TLVSizeConstraint::Fixed(48),
            TLVType::TraceContext => TLVSizeConstraint::Bounded { min: 32, max: 256 },
            TLVType::RecoveryResponse => TLVSizeConstraint::Bounded { min: 20, max: 1024 },
            TLVType::SequenceSync => TLVSizeConstraint::Bounded { min: 16, max: 256 },

            // Portfolio and risk types (existing)
            TLVType::Portfolio => TLVSizeConstraint::Bounded { min: 32, max: 2048 },
            TLVType::Position => TLVSizeConstraint::Bounded { min: 24, max: 512 },
            TLVType::Balance => TLVSizeConstraint::Bounded { min: 16, max: 256 },
            TLVType::TradeConfirmation => TLVSizeConstraint::Bounded { min: 32, max: 256 },
            TLVType::RiskParameters => TLVSizeConstraint::Bounded { min: 24, max: 512 },
            TLVType::PerformanceMetrics => TLVSizeConstraint::Bounded { min: 32, max: 1024 },

            // Portfolio-Risk Domain (60-79) - Signal routing
            TLVType::RiskDecision => TLVSizeConstraint::Fixed(32),
            TLVType::PositionUpdate => TLVSizeConstraint::Bounded { min: 40, max: 512 },
            TLVType::RiskAlert => TLVSizeConstraint::Bounded { min: 24, max: 256 },
            TLVType::CollateralUpdate => TLVSizeConstraint::Fixed(48),
            TLVType::ExposureReport => TLVSizeConstraint::Bounded { min: 32, max: 1024 },
            TLVType::PortfolioEvent => TLVSizeConstraint::Bounded { min: 32, max: 512 },
            TLVType::RiskThreshold => TLVSizeConstraint::Fixed(40),
            TLVType::FlashLoanResult => TLVSizeConstraint::Bounded { min: 48, max: 256 },
            TLVType::PostTradeAnalytics => TLVSizeConstraint::Bounded { min: 64, max: 512 },
            TLVType::PositionQuery => TLVSizeConstraint::Fixed(32),
            TLVType::RiskMetrics => TLVSizeConstraint::Bounded { min: 48, max: 1024 },
            TLVType::CircuitBreaker => TLVSizeConstraint::Fixed(24),
            TLVType::StrategyRegistration => TLVSizeConstraint::Bounded { min: 32, max: 256 },

            // Compliance-Audit Domain (80-99) - System routing
            TLVType::ComplianceCheck => TLVSizeConstraint::Bounded { min: 32, max: 512 },
            TLVType::AuditTrail => TLVSizeConstraint::Bounded { min: 48, max: 2048 },
            TLVType::RegulatoryReport => TLVSizeConstraint::Bounded { min: 64, max: 4096 },
            TLVType::TradeBreakdown => TLVSizeConstraint::Bounded { min: 48, max: 1024 },
            TLVType::SettlementInstruction => TLVSizeConstraint::Bounded { min: 40, max: 512 },
            TLVType::ClearingReport => TLVSizeConstraint::Bounded { min: 32, max: 512 },
            TLVType::MarginCall => TLVSizeConstraint::Fixed(48),
            TLVType::CustodyEvent => TLVSizeConstraint::Bounded { min: 40, max: 256 },
            TLVType::VenueConnectivity => TLVSizeConstraint::Fixed(32),
            TLVType::CrossVenueSpread => TLVSizeConstraint::Fixed(40),
            TLVType::ArbitrageExecution => TLVSizeConstraint::Bounded { min: 48, max: 256 },

            // Extended TLV
            TLVType::ExtendedTLV => TLVSizeConstraint::Variable, // No limit for extended format
        }
    }

    /// Get expected payload size for fixed-size TLVs (backward compatibility)
    pub fn expected_payload_size(&self) -> Option<usize> {
        match self.size_constraint() {
            TLVSizeConstraint::Fixed(size) => Some(size),
            _ => None,
        }
    }
}

/// Vendor/Private TLV type range (200-254)
/// These are available for custom extensions and experimental features
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
pub enum VendorTLVType {
    // Pool cache persistence
    PoolInfo = 200,        // Individual pool record
    PoolCacheHeader = 201, // Cache file header

    // Other vendor extensions
    ProprietaryData = 202,
    CustomMetrics = 203,
    ExperimentalSignal = 204,
    // Reserved 205-254 for other vendors
}

impl VendorTLVType {
    /// Convert to standard TLV type value
    pub fn as_tlv_type(&self) -> u8 {
        *self as u8
    }

    /// Check if a TLV type is in the vendor range
    pub fn is_vendor_type(tlv_type: u8) -> bool {
        (200..=254).contains(&tlv_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tlv_domain_mapping() {
        // Market Data Domain (1-19)
        assert_eq!(
            TLVType::Trade.relay_domain(),
            crate::RelayDomain::MarketData
        );
        assert_eq!(
            TLVType::PoolSwap.relay_domain(),
            crate::RelayDomain::MarketData
        );

        // Strategy Signal Domain (20-39)
        assert_eq!(
            TLVType::SignalIdentity.relay_domain(),
            crate::RelayDomain::Signal
        );
        assert_eq!(
            TLVType::MEVBundle.relay_domain(),
            crate::RelayDomain::Signal
        );

        // Execution Domain (40-59)
        assert_eq!(
            TLVType::OrderRequest.relay_domain(),
            crate::RelayDomain::Execution
        );
        assert_eq!(TLVType::Fill.relay_domain(), crate::RelayDomain::Execution);

        // Portfolio-Risk Domain (60-79) → Signal (monitoring/analytics)
        assert_eq!(
            TLVType::RiskDecision.relay_domain(),
            crate::RelayDomain::Signal
        );
        assert_eq!(
            TLVType::PositionUpdate.relay_domain(),
            crate::RelayDomain::Signal
        );

        // Compliance-Audit Domain (80-99) → System
        assert_eq!(
            TLVType::ComplianceCheck.relay_domain(),
            crate::RelayDomain::System
        );
        assert_eq!(
            TLVType::AuditTrail.relay_domain(),
            crate::RelayDomain::System
        );

        // System Domain (100-119)
        assert_eq!(
            TLVType::Heartbeat.relay_domain(),
            crate::RelayDomain::System
        );
        assert_eq!(
            TLVType::SystemHealth.relay_domain(),
            crate::RelayDomain::System
        );
    }

    #[test]
    fn test_reserved_types() {
        assert!(TLVType::Trade.is_reserved() == false);
        // Note: We can't easily test reserved types since they're not defined as enum variants
        // This would need to be tested with raw u8 values
    }

    #[test]
    fn test_expected_sizes() {
        assert_eq!(TLVType::Trade.expected_payload_size(), Some(40));
        assert_eq!(TLVType::Economics.expected_payload_size(), Some(32));
        assert_eq!(TLVType::OrderBook.expected_payload_size(), None); // Variable size
    }

    #[test]
    fn test_size_constraints() {
        // Fixed-size TLVs
        assert_eq!(
            TLVType::Trade.size_constraint(),
            TLVSizeConstraint::Fixed(40)
        );
        assert_eq!(
            TLVType::Quote.size_constraint(),
            TLVSizeConstraint::Fixed(52)
        );

        // Bounded-size TLVs (pool events with variable-length PoolInstrumentId)
        assert_eq!(
            TLVType::PoolSwap.size_constraint(),
            TLVSizeConstraint::Bounded { min: 60, max: 200 }
        );
        assert_eq!(
            TLVType::PoolSync.size_constraint(),
            TLVSizeConstraint::Bounded { min: 40, max: 150 }
        );

        // Variable-size TLVs
        assert_eq!(
            TLVType::OrderBook.size_constraint(),
            TLVSizeConstraint::Variable
        );
        assert_eq!(
            TLVType::L2Snapshot.size_constraint(),
            TLVSizeConstraint::Variable
        );

        // New Portfolio-Risk TLVs
        assert_eq!(
            TLVType::RiskDecision.size_constraint(),
            TLVSizeConstraint::Fixed(32)
        );
        assert_eq!(
            TLVType::RiskAlert.size_constraint(),
            TLVSizeConstraint::Bounded { min: 24, max: 256 }
        );

        // New Compliance TLVs
        assert_eq!(
            TLVType::ComplianceCheck.size_constraint(),
            TLVSizeConstraint::Bounded { min: 32, max: 512 }
        );
        assert_eq!(
            TLVType::MarginCall.size_constraint(),
            TLVSizeConstraint::Fixed(48)
        );
    }

    #[test]
    fn test_vendor_types() {
        assert!(VendorTLVType::is_vendor_type(200));
        assert!(VendorTLVType::is_vendor_type(254));
        assert!(!VendorTLVType::is_vendor_type(199));
        assert!(!VendorTLVType::is_vendor_type(255));

        assert_eq!(VendorTLVType::CustomMetrics.as_tlv_type(), 203);
    }
}
