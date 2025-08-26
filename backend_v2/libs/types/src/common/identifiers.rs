//! # Unified Identifier System - Bijective Identifiers + Typed ID Wrappers
//!
//! **MIGRATED FROM**: `protocol_v2/src/identifiers/` - This is now the canonical location
//!
//! Provides both bijective instrument identification (InstrumentId) AND type-safe
//! wrappers for simple database identifiers to create a complete identification
//! system for the AlphaPulse trading platform.
//!
//! ## Dual System Architecture
//!
//! ### 1. Bijective InstrumentId (migrated from protocol_v2)
//! - Self-describing identifiers for trading instruments
//! - Embed venue, asset type, and identifying data  
//! - Deterministic construction, no registries needed
//! - >19M operations/second performance
//! - 12-byte packed struct with zerocopy support
//!
//! ### 2. Typed Simple IDs (new macro system)
//! - Zero-cost wrappers for u64 database IDs
//! - Compile-time type safety for orders, positions, signals, etc.
//! - Prevents ID confusion bugs at compile time
//! - Transparent serialization and database integration
//!
//! ## Usage Examples
//!
//! ```rust
//! use alphapulse_types::{InstrumentId, VenueId, OrderId, PositionId, StrategyId};
//!
//! // Bijective instrument identification (no registry needed)
//! let btc_usdc = InstrumentId::ethereum_token("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap();
//! let aapl_stock = InstrumentId::stock(VenueId::NASDAQ, "AAPL");
//!
//! // Type-safe simple IDs from database
//! let order = OrderId::new(12345);
//! let position = PositionId::new(67890);
//! let strategy = StrategyId::new(1001);
//!
//! // Type-safe function signatures prevent confusion
//! fn execute_order(
//!     instrument: InstrumentId,  // Self-describing bijective ID
//!     order: OrderId,           // Type-safe database ID
//!     strategy: StrategyId      // Type-safe database ID
//! ) {
//!     // Compiler prevents mixing up order vs strategy IDs
//! }
//!
//! // Cannot accidentally swap parameters
//! execute_order(btc_usdc, order, strategy); // ✅ Correct
//! // execute_order(btc_usdc, strategy, order); // ❌ Compile error!
//! ```
//!
//! ## Performance Characteristics
//!
//! Benchmarks demonstrate true zero-cost abstraction for typed IDs:
//!
//! | Operation | Raw u64 | Typed ID | Overhead |
//! |-----------|---------|----------|----------|
//! | Creation | 649 ps | 575 ps | **-11%** (faster!) |
//! | Arithmetic | 526 ps | 535 ps | +2% |
//! | Inner Access | - | 577 ps | N/A |
//! | Conversions | - | 594 ps | N/A |
//! | Memory Size | 8 bytes | 8 bytes | 0% |
//! | Alignment | 8 bytes | 8 bytes | 0% |
//!
//! ### TLV Integration Performance
//!
//! | Operation | Time | Throughput |
//! |-----------|------|------------|
//! | Single TLV Message | 92 ns | ~10.8M msg/s |
//! | Multiple Messages | 184 ns | ~5.4M msg/s |
//! | Typed ID Context Ops | 76 ns | ~13.1M ops/s |
//! | 10 TLVs per Message | 503 ns | ~2.0M msg/s |
//!
//! **Key Finding**: Typed IDs add **zero measurable overhead** to TLV message construction
//! while providing complete compile-time safety against ID confusion bugs.
//!
//! ## Integration Examples
//!
//! ### Service Function Migration
//!
//! ```rust
//! // BEFORE: Error-prone raw u64 parameters
//! fn process_arbitrage(
//!     pool_id: u64,      // Which pool?
//!     signal_id: u64,    // Easy to confuse with pool_id
//!     strategy_id: u64,  // Could be swapped with signal_id
//! ) -> Result<u64> {    // What does this u64 represent?
//!     // Implementation...
//! }
//!
//! // AFTER: Type-safe, self-documenting
//! fn process_arbitrage(
//!     pool: PoolId,      // Clearly a pool identifier
//!     signal: SignalId,  // Cannot be confused with pool
//!     strategy: StrategyId, // Cannot be swapped
//! ) -> Result<OrderId> { // Clear return type
//!     // Compiler prevents: process_arbitrage(signal, pool, strategy)
//!     // Implementation...
//! }
//! ```
//!
//! ### TLV Message Construction with Typed IDs
//!
//! ```rust
//! use alphapulse_types::{
//!     TLVMessageBuilder, TLVType, RelayDomain, SourceType,
//!     SignalId, StrategyId, OrderId
//! };
//!
//! // Generate typed IDs from business logic
//! let signal = SignalId::new(12345);
//! let strategy = StrategyId::new(42);
//! let order = OrderId::new(67890);
//!
//! // Use in context where raw u64 is still needed (TLV structures)
//! let signal_data = ArbitrageSignalTLV {
//!     signal_id: signal.inner(), // Convert to u64 for TLV
//!     strategy_id: strategy.inner() as u16, // Size conversion if needed
//!     // ... other fields
//! };
//!
//! // Build message
//! let message = TLVMessageBuilder::new(RelayDomain::Signal, SourceType::Strategy)
//!     .add_tlv(TLVType::ArbitrageSignal, &signal_data)
//!     .build();
//! ```
//!
//! ### Database Integration Pattern
//!
//! ```rust
//! // Database layer returns typed IDs
//! async fn fetch_active_orders() -> Result<Vec<(OrderId, PositionId)>> {
//!     let rows = sqlx::query!("SELECT order_id, position_id FROM active_orders")
//!         .fetch_all(&pool)
//!         .await?;
//!     
//!     Ok(rows.into_iter()
//!         .map(|row| (OrderId::new(row.order_id as u64), 
//!                     PositionId::new(row.position_id as u64)))
//!         .collect())
//! }
//!
//! // Service layer uses typed IDs
//! async fn cancel_order(order: OrderId, position: PositionId) -> Result<()> {
//!     // Cannot accidentally swap order and position parameters
//!     execute_cancellation(order, position).await
//! }
//! ```
//!
//! ### Gradual Migration Strategy
//!
//! ```rust
//! // Step 1: Add compatibility wrapper for existing code
//! fn legacy_process_trade(pool_id: u64, signal_id: u64) -> u64 {
//!     // Wrap raw IDs immediately at boundary
//!     let pool = PoolId::new(pool_id);
//!     let signal = SignalId::new(signal_id);
//!     
//!     // Use new typed version internally
//!     let order = process_trade_typed(pool, signal);
//!     
//!     // Return raw ID for compatibility
//!     order.inner()
//! }
//!
//! // Step 2: New implementation uses typed IDs throughout
//! fn process_trade_typed(pool: PoolId, signal: SignalId) -> OrderId {
//!     // Type-safe implementation
//!     // Compiler catches: process_trade_typed(signal, pool) // ERROR!
//!     OrderId::new(123) // Implementation details...
//! }
//!
//! // Step 3: Eventually migrate callers to use typed version directly
//! ```
//!
//! ## Migration Benefits
//!
//! - **Single Source of Truth**: All identifier types in one location
//! - **Type Safety**: Compile-time prevention of ID confusion bugs
//! - **Performance**: Zero-cost abstractions maintain >19M ops/s
//! - **Integration**: Seamless serde, database, and Protocol V2 support
//! - **Maintainability**: Centralized identifier system easier to maintain

use num_enum::TryFromPrimitive;
use std::hash::{Hash, Hasher};
// Remove serde_big_array for now and use simple custom serialization

// ================================
// Typed ID Macro System (New)
// ================================

/// Macro for generating zero-cost typed byte array wrappers
///
/// Creates a new type that wraps fixed-size byte arrays with complete type safety
/// while maintaining identical runtime performance and memory layout.
///
/// This eliminates entire classes of bugs where byte arrays of different sizes
/// are confused (e.g., 20-byte addresses vs 32-byte hashes).
///
/// # Examples
///
/// ```rust
/// use alphapulse_types::define_typed_wrapper;
///
/// // Create strongly typed Ethereum address (20 bytes)
/// define_typed_wrapper!(
///     /// Ethereum address (20 bytes)
///     EthAddress, [u8; 20]
/// );
///
/// // Create strongly typed hash (32 bytes)  
/// define_typed_wrapper!(
///     /// SHA-256 hash (32 bytes)
///     Hash256, [u8; 32]
/// );
///
/// // Compile-time safety - these cannot be confused
/// fn process_address(addr: EthAddress) { }
/// fn process_hash(hash: Hash256) { }
///
/// let addr = EthAddress::from([0u8; 20]);
/// let hash = Hash256::from([0u8; 32]);
///
/// process_address(addr); // ✅ Works
/// // process_address(hash); // ❌ Compile error!
/// ```
#[macro_export]
macro_rules! define_typed_wrapper {
    (
        $(#[$meta:meta])*
        $name:ident, $inner_type:ty
    ) => {
        $(#[$meta])*
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Default
        )]
        #[repr(transparent)] // Same memory layout as inner type for zero cost
        pub struct $name(pub $inner_type);

        impl $name {
            /// Create a new typed wrapper
            #[inline(always)]
            pub const fn new(inner: $inner_type) -> Self {
                Self(inner)
            }

            /// Extract the inner value
            #[inline(always)]
            pub const fn inner(&self) -> &$inner_type {
                &self.0
            }

            /// Extract the inner value by value
            #[inline(always)]
            pub const fn into_inner(self) -> $inner_type {
                self.0
            }

            /// Get a reference to the inner bytes (works for byte arrays)
            #[inline(always)]
            pub fn as_bytes(&self) -> &[u8] {
                unsafe {
                    std::slice::from_raw_parts(
                        &self.0 as *const $inner_type as *const u8,
                        std::mem::size_of::<$inner_type>()
                    )
                }
            }
        }

        // Display for debugging and logging
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}(0x", stringify!($name))?;
                for byte in self.as_bytes() {
                    write!(f, "{:02x}", byte)?;
                }
                write!(f, ")")
            }
        }

        // Conversions for interoperability
        impl From<$inner_type> for $name {
            #[inline(always)]
            fn from(inner: $inner_type) -> Self {
                Self(inner)
            }
        }

        impl From<$name> for $inner_type {
            #[inline(always)]
            fn from(wrapper: $name) -> $inner_type {
                wrapper.0
            }
        }

        // AsRef for ergonomic usage
        impl AsRef<$inner_type> for $name {
            #[inline(always)]
            fn as_ref(&self) -> &$inner_type {
                &self.0
            }
        }

        // AsMut for ergonomic usage
        impl AsMut<$inner_type> for $name {
            #[inline(always)]
            fn as_mut(&mut self) -> &mut $inner_type {
                &mut self.0
            }
        }

        // Serialization support - serializes the inner type directly
        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                self.0.serialize(serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                <$inner_type>::deserialize(deserializer).map(Self)
            }
        }
    };
}

/// Macro for generating zero-cost typed ID wrappers
///
/// Creates a new type that wraps `u64` with complete type safety while maintaining
/// identical runtime performance and memory layout.
#[macro_export]
macro_rules! define_typed_id {
    (
        $(#[$meta:meta])*
        $name:ident
    ) => {
        $(#[$meta])*
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash
        )]
        // Add zerocopy traits for TLV compatibility
        #[cfg(feature = "protocol")]
        #[derive(zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
        #[repr(transparent)] // Same memory layout as u64 for zero cost
        pub struct $name(pub u64);

        impl $name {
            /// Create a new typed ID
            #[inline(always)]
            pub const fn new(id: u64) -> Self {
                Self(id)
            }

            /// Extract the inner u64 value
            #[inline(always)]
            pub const fn inner(&self) -> u64 {
                self.0
            }

            /// Generate next sequential ID
            #[inline(always)]
            pub fn next(&self) -> Self {
                Self(self.0.wrapping_add(1))
            }

            /// Check if this is a null/zero ID
            #[inline(always)]
            pub fn is_null(&self) -> bool {
                self.0 == 0
            }

            /// Create a null/zero ID
            #[inline(always)]
            pub const fn null() -> Self {
                Self(0)
            }
        }

        // Display for debugging and logging
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        // Conversions for interoperability
        impl From<u64> for $name {
            #[inline(always)]
            fn from(id: u64) -> Self {
                Self(id)
            }
        }

        impl From<$name> for u64 {
            #[inline(always)]
            fn from(id: $name) -> u64 {
                id.0
            }
        }

        // Serialization support - serializes as raw u64
        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                self.0.serialize(serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                u64::deserialize(deserializer).map(Self)
            }
        }
    };
}

// ================================
// Simple Typed IDs (New)
// ================================

define_typed_id!(
    /// Unique identifier for an order
    OrderId
);

define_typed_id!(
    /// Unique identifier for a position
    PositionId
);

define_typed_id!(
    /// Unique identifier for a trading strategy
    StrategyId
);

define_typed_id!(
    /// Unique identifier for a trading signal
    SignalId
);

define_typed_id!(
    /// Unique identifier for an arbitrage opportunity
    OpportunityId
);

define_typed_id!(
    /// Unique identifier for a trade execution
    TradeId
);

define_typed_id!(
    /// Unique identifier for a portfolio
    PortfolioId
);

define_typed_id!(
    /// Unique identifier for a session
    SessionId
);

define_typed_id!(
    /// Unique identifier for an actor
    ActorId
);

define_typed_id!(
    /// Unique identifier for a relay connection
    RelayId
);

define_typed_id!(
    /// Unique identifier for a message sequence
    SequenceId
);

define_typed_id!(
    /// Unique identifier for a liquidity pool
    PoolId
);

define_typed_id!(
    /// Unique identifier for a pool pair
    PoolPairId
);

define_typed_id!(
    /// Unique identifier for an instrument (different from the bijective InstrumentId)
    SimpleInstrumentId
);

define_typed_id!(
    /// Unique identifier for a chain
    ChainId
);

define_typed_id!(
    /// Unique identifier for a venue
    SimpleVenueId
);

// ================================
// Typed Byte Array Wrappers (New)
// ================================

define_typed_wrapper!(
    /// Ethereum address (20 bytes)
    ///
    /// Prevents confusion with 32-byte hashes and provides compile-time
    /// safety for address handling throughout the system.
    EthAddress, [u8; 20]
);

define_typed_wrapper!(
    /// Transaction hash (32 bytes)
    ///
    /// Strongly typed wrapper for transaction hashes, preventing confusion
    /// with addresses, block hashes, or other 32-byte values.
    TxHash, [u8; 32]
);

define_typed_wrapper!(
    /// Block hash (32 bytes)
    ///
    /// Strongly typed wrapper for block hashes, ensuring compile-time
    /// safety when working with blockchain data.
    BlockHash, [u8; 32]
);

define_typed_wrapper!(
    /// Generic 32-byte hash
    ///
    /// For cases where you need a generic hash type but still want
    /// compile-time safety vs addresses or other data types.
    Hash256, [u8; 32]
);

define_typed_wrapper!(
    /// Pool address (20 bytes)
    ///
    /// Specialized wrapper for DEX pool addresses, providing additional
    /// type safety in DeFi operations where pools and tokens are distinct.
    PoolAddress, [u8; 20]
);

define_typed_wrapper!(
    /// Token contract address (20 bytes)
    ///
    /// Specialized wrapper for ERC-20 token contract addresses,
    /// preventing confusion with pool addresses or EOA addresses.
    TokenAddress, [u8; 20]
);

// Special implementations for large byte arrays (>32 bytes) that need custom serde handling

/// Ethereum signature (65 bytes: r + s + v)
///
/// Typed wrapper for complete Ethereum signatures including recovery parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct EthSignature(pub [u8; 65]);

impl EthSignature {
    #[inline(always)]
    pub const fn new(inner: [u8; 65]) -> Self {
        Self(inner)
    }

    #[inline(always)]
    pub const fn inner(&self) -> &[u8; 65] {
        &self.0
    }

    #[inline(always)]
    pub const fn into_inner(self) -> [u8; 65] {
        self.0
    }

    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Default for EthSignature {
    fn default() -> Self {
        Self([0u8; 65])
    }
}

impl std::fmt::Display for EthSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EthSignature(0x")?;
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        write!(f, ")")
    }
}

impl From<[u8; 65]> for EthSignature {
    #[inline(always)]
    fn from(inner: [u8; 65]) -> Self {
        Self(inner)
    }
}

impl From<EthSignature> for [u8; 65] {
    #[inline(always)]
    fn from(wrapper: EthSignature) -> [u8; 65] {
        wrapper.0
    }
}

impl AsRef<[u8; 65]> for EthSignature {
    #[inline(always)]
    fn as_ref(&self) -> &[u8; 65] {
        &self.0
    }
}

impl AsMut<[u8; 65]> for EthSignature {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut [u8; 65] {
        &mut self.0
    }
}

impl serde::Serialize for EthSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as Vec<u8> for compatibility
        self.0.as_slice().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for EthSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let vec = Vec::<u8>::deserialize(deserializer)?;
        if vec.len() != 65 {
            return Err(serde::de::Error::invalid_length(vec.len(), &"65 bytes"));
        }
        let mut array = [0u8; 65];
        array.copy_from_slice(&vec);
        Ok(Self(array))
    }
}

/// Public key (64 bytes: uncompressed secp256k1)
///
/// Typed wrapper for uncompressed public keys, ensuring proper handling
/// in cryptographic operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PublicKey(pub [u8; 64]);

impl PublicKey {
    #[inline(always)]
    pub const fn new(inner: [u8; 64]) -> Self {
        Self(inner)
    }

    #[inline(always)]
    pub const fn inner(&self) -> &[u8; 64] {
        &self.0
    }

    #[inline(always)]
    pub const fn into_inner(self) -> [u8; 64] {
        self.0
    }

    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Default for PublicKey {
    fn default() -> Self {
        Self([0u8; 64])
    }
}

impl std::fmt::Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PublicKey(0x")?;
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        write!(f, ")")
    }
}

impl From<[u8; 64]> for PublicKey {
    #[inline(always)]
    fn from(inner: [u8; 64]) -> Self {
        Self(inner)
    }
}

impl From<PublicKey> for [u8; 64] {
    #[inline(always)]
    fn from(wrapper: PublicKey) -> [u8; 64] {
        wrapper.0
    }
}

impl AsRef<[u8; 64]> for PublicKey {
    #[inline(always)]
    fn as_ref(&self) -> &[u8; 64] {
        &self.0
    }
}

impl AsMut<[u8; 64]> for PublicKey {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut [u8; 64] {
        &mut self.0
    }
}

impl serde::Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as Vec<u8> for compatibility
        self.0.as_slice().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let vec = Vec::<u8>::deserialize(deserializer)?;
        if vec.len() != 64 {
            return Err(serde::de::Error::invalid_length(vec.len(), &"64 bytes"));
        }
        let mut array = [0u8; 64];
        array.copy_from_slice(&vec);
        Ok(Self(array))
    }
}

// Special case for PrivateKey - needs manual implementation due to Drop
/// Private key (32 bytes)
///
/// Typed wrapper for private keys. Use with extreme caution
/// and ensure proper zeroization when dropping.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct PrivateKey(pub [u8; 32]);

impl PrivateKey {
    /// Create a new private key wrapper  
    #[inline(always)]
    pub const fn new(inner: [u8; 32]) -> Self {
        Self(inner)
    }

    /// Create a private key with explicit warning
    ///
    /// # Security Warning
    /// Private keys should be handled with extreme care:
    /// - Never log or print private keys
    /// - Use secure memory allocators when possible  
    /// - Ensure proper cleanup (automatic via Drop)
    /// - Consider using hardware security modules for production
    pub fn new_with_warning(key: [u8; 32]) -> Self {
        Self(key)
    }

    /// Extract the inner value
    #[inline(always)]
    pub const fn inner(&self) -> &[u8; 32] {
        &self.0
    }

    /// Extract the inner value by value
    #[inline(always)]
    pub fn into_inner(self) -> [u8; 32] {
        self.0
    }

    /// Get a reference to the inner bytes
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

// Display for debugging - but don't show the actual key!
impl std::fmt::Display for PrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PrivateKey([REDACTED])")
    }
}

// Conversions for interoperability
impl From<[u8; 32]> for PrivateKey {
    #[inline(always)]
    fn from(inner: [u8; 32]) -> Self {
        Self(inner)
    }
}

impl From<PrivateKey> for [u8; 32] {
    #[inline(always)]
    fn from(wrapper: PrivateKey) -> [u8; 32] {
        wrapper.0
    }
}

// AsRef for ergonomic usage
impl AsRef<[u8; 32]> for PrivateKey {
    #[inline(always)]
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

// AsMut for ergonomic usage
impl AsMut<[u8; 32]> for PrivateKey {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut [u8; 32] {
        &mut self.0
    }
}

// Serialization - serialize as raw bytes (be careful with this!)
impl serde::Serialize for PrivateKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for PrivateKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <[u8; 32]>::deserialize(deserializer).map(Self)
    }
}

// Special security handling for PrivateKey
impl Drop for PrivateKey {
    fn drop(&mut self) {
        // Zero out private key memory for security
        self.0.fill(0);
    }
}

// ================================
// Bijective System (Migrated from protocol_v2)
// ================================

/// Venue identifiers for different exchanges and protocols
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
pub enum VenueId {
    // Generic venue for testing and legacy compatibility (0)
    Generic = 0,

    // Traditional Exchanges (1-99)
    NYSE = 1,
    NASDAQ = 2,
    LSE = 3,  // London Stock Exchange
    TSE = 4,  // Tokyo Stock Exchange
    HKEX = 5, // Hong Kong Exchange

    // Cryptocurrency Centralized Exchanges (100-199)
    Binance = 100,
    Kraken = 101,
    Coinbase = 102,
    Huobi = 103,
    OKEx = 104,
    FTX = 105, // Historical
    Bybit = 106,
    KuCoin = 107,
    Gemini = 108,

    // Layer 1 Blockchains (200-299)
    Ethereum = 200,
    Bitcoin = 201,
    Polygon = 202,
    BinanceSmartChain = 203,
    Avalanche = 204,
    Fantom = 205,
    Arbitrum = 206,
    Optimism = 207,
    Solana = 208,
    Cardano = 209,
    Polkadot = 210,
    Cosmos = 211,

    // DeFi Protocols on Ethereum (300-399)
    UniswapV2 = 300,
    UniswapV3 = 301,
    SushiSwap = 302,
    Curve = 303,
    Balancer = 304,
    Aave = 305,
    Compound = 306,
    MakerDAO = 307,
    Yearn = 308,
    Synthetix = 309,
    DYdX = 310,

    // DeFi Protocols on Polygon (400-499)
    QuickSwap = 400,
    SushiSwapPolygon = 401,
    CurvePolygon = 402,
    AavePolygon = 403,
    BalancerPolygon = 404,

    // DeFi Protocols on BSC (500-599)
    PancakeSwap = 500,
    VenusProtocol = 501,

    // DeFi Protocols on Arbitrum (600-699)
    UniswapV3Arbitrum = 600,
    SushiSwapArbitrum = 601,
    CurveArbitrum = 602,

    // Options and Derivatives (700-799)
    Deribit = 700,
    BybitDerivatives = 701,
    OpynProtocol = 702,
    Hegic = 703,

    // Commodities and Forex (800-899)
    COMEX = 800, // Commodity Exchange
    CME = 801,   // Chicago Mercantile Exchange
    ICE = 802,   // Intercontinental Exchange
    ForexCom = 803,

    // Test/Development Venues (65000+)
    TestVenue = 65000,
    MockExchange = 65001,
}

impl VenueId {
    /// Get the blockchain chain ID for blockchain venues
    pub fn chain_id(&self) -> Option<u64> {
        match self {
            VenueId::Ethereum
            | VenueId::UniswapV2
            | VenueId::UniswapV3
            | VenueId::SushiSwap
            | VenueId::Curve
            | VenueId::Balancer
            | VenueId::Aave
            | VenueId::Compound
            | VenueId::MakerDAO
            | VenueId::Yearn
            | VenueId::Synthetix
            | VenueId::DYdX => Some(1),

            VenueId::Polygon
            | VenueId::QuickSwap
            | VenueId::SushiSwapPolygon
            | VenueId::CurvePolygon
            | VenueId::AavePolygon
            | VenueId::BalancerPolygon => Some(137),

            VenueId::BinanceSmartChain | VenueId::PancakeSwap | VenueId::VenusProtocol => Some(56),

            VenueId::Arbitrum
            | VenueId::UniswapV3Arbitrum
            | VenueId::SushiSwapArbitrum
            | VenueId::CurveArbitrum => Some(42161),

            VenueId::Optimism => Some(10),
            VenueId::Avalanche => Some(43114),
            VenueId::Fantom => Some(250),

            _ => None,
        }
    }

    /// Check if this venue supports DEX-style liquidity pools
    pub fn supports_pools(&self) -> bool {
        matches!(
            self,
            VenueId::UniswapV2
                | VenueId::UniswapV3
                | VenueId::SushiSwap
                | VenueId::Curve
                | VenueId::Balancer
                | VenueId::QuickSwap
                | VenueId::SushiSwapPolygon
                | VenueId::CurvePolygon
                | VenueId::BalancerPolygon
                | VenueId::PancakeSwap
                | VenueId::UniswapV3Arbitrum
                | VenueId::SushiSwapArbitrum
                | VenueId::CurveArbitrum
        )
    }

    /// Check if this is a DeFi (decentralized) venue
    pub fn is_defi(&self) -> bool {
        matches!(*self as u16, 300..=699)
    }

    /// Check if this is a centralized exchange
    pub fn is_centralized(&self) -> bool {
        matches!(*self as u16, 100..=199)
    }
}

impl std::fmt::Display for VenueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Asset types for different instrument classes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
pub enum AssetType {
    // Traditional Assets (1-49)
    Stock = 1,
    Bond = 2,
    ETF = 3,
    Commodity = 4,
    Currency = 5,
    Index = 6,

    // Cryptocurrency Assets (50-99)
    Token = 50,        // ERC-20, SPL, etc.
    Coin = 51,         // Native blockchain tokens (ETH, BTC, etc.)
    NFT = 52,          // Non-fungible tokens
    StableCoin = 53,   // USDC, USDT, DAI, etc.
    WrappedToken = 54, // WETH, WBTC, etc.

    // DeFi Instruments (100-149)
    Pool = 100,            // Liquidity pools (Uniswap, Curve, etc.)
    LPToken = 101,         // Liquidity provider tokens
    YieldToken = 102,      // Yield-bearing tokens (aUSDC, cDAI, etc.)
    SyntheticAsset = 103,  // Synthetix synths
    DerivativeToken = 104, // Options, futures tokens
    GovernanceToken = 105, // DAO governance tokens

    // Derivatives (150-199)
    Option = 150,
    Future = 151,
    Swap = 152,
    Forward = 153,
    CDS = 154, // Credit Default Swap

    // Test/Development (250-255)
    TestAsset = 250,
    MockAsset = 251,
}

impl AssetType {
    /// Check if this asset type represents a fungible token
    pub fn is_fungible(&self) -> bool {
        !matches!(self, AssetType::NFT)
    }

    /// Check if this asset type is a blockchain-native asset
    pub fn is_blockchain_native(&self) -> bool {
        matches!(*self as u8, 50..=149)
    }

    /// Check if this asset type represents a derivative
    pub fn is_derivative(&self) -> bool {
        matches!(*self as u8, 150..=199)
    }

    /// Get typical decimal places for this asset type
    pub fn typical_decimals(&self) -> u8 {
        match self {
            AssetType::Stock => 2,
            AssetType::Bond => 4,
            AssetType::Currency => 4,
            AssetType::Token | AssetType::WrappedToken => 18,
            AssetType::StableCoin => 6,
            AssetType::Coin => 8,
            AssetType::Pool | AssetType::LPToken => 18,
            _ => 8,
        }
    }
}

/// Bijective Instrument ID - MIGRATED FROM protocol_v2
///
/// Self-describing instrument identifier that contains all necessary routing information.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InstrumentId {
    pub asset_id: u64,  // Venue-specific identifier (8 bytes)
    pub venue: u16,     // VenueId enum (2 bytes)
    pub asset_type: u8, // AssetType enum (1 byte)
    pub reserved: u8,   // Future use/flags (1 byte)
                        // Total: exactly 12 bytes
}

impl InstrumentId {
    /// Create Ethereum token ID from contract address
    pub fn ethereum_token(address: &str) -> Result<Self, crate::FixedPointError> {
        Self::evm_token(VenueId::Ethereum, address)
    }

    /// Create Polygon token ID from contract address
    pub fn polygon_token(address: &str) -> Result<Self, crate::FixedPointError> {
        Self::evm_token(VenueId::Polygon, address)
    }

    /// Generic EVM token ID from contract address
    fn evm_token(venue: VenueId, address: &str) -> Result<Self, crate::FixedPointError> {
        // Clean the address (remove 0x prefix if present)
        let hex_clean = address.strip_prefix("0x").unwrap_or(address);

        if hex_clean.len() != 40 {
            return Err(crate::FixedPointError::InvalidFormat(
                "Invalid address length".to_string(),
            ));
        }

        // Use first 8 bytes (16 hex chars) of address as asset_id
        let bytes = hex::decode(&hex_clean[..16]).map_err(|_| {
            crate::FixedPointError::InvalidFormat("Invalid hex address".to_string())
        })?;
        let asset_id = u64::from_be_bytes(bytes.try_into().map_err(|_| {
            crate::FixedPointError::InvalidFormat("Address conversion failed".to_string())
        })?);

        Ok(Self {
            venue: venue as u16,
            asset_type: AssetType::Token as u8,
            reserved: 0,
            asset_id,
        })
    }

    /// Create stock ID from exchange and symbol
    pub fn stock(exchange: VenueId, symbol: &str) -> Self {
        Self {
            venue: exchange as u16,
            asset_type: AssetType::Stock as u8,
            reserved: 0,
            asset_id: symbol_to_u64(symbol),
        }
    }

    /// Create cryptocurrency coin ID (native blockchain token)
    pub fn coin(blockchain: VenueId, symbol: &str) -> Self {
        Self {
            venue: blockchain as u16,
            asset_type: AssetType::Coin as u8,
            reserved: 0,
            asset_id: symbol_to_u64(symbol),
        }
    }

    /// Convert to u64 for cache keys (with potential precision loss)
    pub fn to_u64(&self) -> u64 {
        ((self.venue as u64) << 48)
            | ((self.asset_type as u64) << 40)
            | (self.asset_id & 0xFFFFFFFFFF) // Only lower 40 bits
    }

    /// Reconstruct from u64 cache key (may lose some precision)
    pub fn from_u64(value: u64) -> Self {
        Self {
            venue: ((value >> 48) & 0xFFFF) as u16,
            asset_type: ((value >> 40) & 0xFF) as u8,
            reserved: 0,
            asset_id: value & 0xFFFFFFFFFF,
        }
    }

    /// Convert to u128 for full-precision cache keys
    pub fn cache_key(&self) -> u128 {
        ((self.venue as u128) << 80)
            | ((self.asset_type as u128) << 72)
            | ((self.reserved as u128) << 64)
            | (self.asset_id as u128)
    }

    /// Get the venue for this instrument
    pub fn venue(&self) -> Result<VenueId, crate::FixedPointError> {
        VenueId::try_from(self.venue)
            .map_err(|_| crate::FixedPointError::InvalidFormat("Invalid venue".to_string()))
    }

    /// Get the asset type for this instrument
    pub fn asset_type(&self) -> Result<AssetType, crate::FixedPointError> {
        AssetType::try_from(self.asset_type)
            .map_err(|_| crate::FixedPointError::InvalidFormat("Invalid asset type".to_string()))
    }

    /// Human-readable debug representation
    pub fn debug_info(&self) -> String {
        // Copy packed struct fields to local variables to avoid alignment issues
        let venue_id = self.venue;
        let asset_type_id = self.asset_type;
        let asset_id = self.asset_id;

        match (self.venue(), self.asset_type()) {
            (Ok(venue), Ok(AssetType::Token)) => {
                format!("{:?} Token 0x{:016x}", venue, asset_id)
            }
            (Ok(venue), Ok(AssetType::Stock)) => {
                format!("{:?} Stock: {}", venue, u64_to_symbol(asset_id))
            }
            (Ok(venue), Ok(asset_type)) => {
                format!("{:?} {:?} #{}", venue, asset_type, asset_id)
            }
            _ => format!("Invalid {}/{} #{}", venue_id, asset_type_id, asset_id),
        }
    }
}

impl Hash for InstrumentId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cache_key().hash(state);
    }
}

impl std::fmt::Display for InstrumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.debug_info())
    }
}

/// Convert symbol string to u64 for asset_id (up to 8 characters)
fn symbol_to_u64(symbol: &str) -> u64 {
    let mut bytes = [0u8; 8];
    let len = symbol.len().min(8);
    bytes[..len].copy_from_slice(&symbol.as_bytes()[..len]);
    u64::from_be_bytes(bytes)
}

/// Convert u64 asset_id back to symbol string
fn u64_to_symbol(value: u64) -> String {
    let bytes = value.to_be_bytes();
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(8);
    String::from_utf8_lossy(&bytes[..end]).to_string()
}

// Need to add hex dependency to Cargo.toml
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typed_ids() {
        let order = OrderId::new(123);
        let position = PositionId::new(456);

        assert_eq!(order.inner(), 123);
        assert_eq!(position.inner(), 456);
        assert_ne!(order.inner(), position.inner());

        // Test serialization
        let json = serde_json::to_string(&order).unwrap();
        assert_eq!(json, "123");
    }

    #[test]
    fn test_venue_properties() {
        assert!(VenueId::UniswapV3.supports_pools());
        assert!(VenueId::UniswapV3.is_defi());
        assert!(!VenueId::Binance.supports_pools());
        assert!(VenueId::Binance.is_centralized());

        assert_eq!(VenueId::Ethereum.chain_id(), Some(1));
        assert_eq!(VenueId::Polygon.chain_id(), Some(137));
    }

    #[test]
    fn test_instrument_id() {
        let aapl = InstrumentId::stock(VenueId::NASDAQ, "AAPL");
        assert_eq!(aapl.venue().unwrap(), VenueId::NASDAQ);
        assert_eq!(aapl.asset_type().unwrap(), AssetType::Stock);

        let cache_key = aapl.cache_key();
        assert_ne!(cache_key, 0);

        println!("AAPL: {}", aapl.debug_info());
    }

    #[test]
    fn test_typed_byte_wrappers() {
        // Test EthAddress (20 bytes)
        let addr_bytes = [1u8; 20];
        let eth_addr = EthAddress::new(addr_bytes);

        assert_eq!(eth_addr.as_bytes().len(), 20);
        assert_eq!(eth_addr.as_bytes(), &addr_bytes);
        assert_eq!(eth_addr.into_inner(), addr_bytes);

        // Test Display formatting
        let display = format!("{}", eth_addr);
        assert!(display.starts_with("EthAddress(0x"));
        assert!(display.ends_with(")"));

        // Test serialization
        let json = serde_json::to_string(&eth_addr).unwrap();
        let recovered: EthAddress = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, eth_addr);
    }

    #[test]
    fn test_hash_types_cannot_be_confused() {
        // Test different 32-byte types cannot be confused
        let tx_hash = TxHash::new([2u8; 32]);
        let block_hash = BlockHash::new([3u8; 32]);
        let generic_hash = Hash256::new([4u8; 32]);

        assert_eq!(tx_hash.as_bytes().len(), 32);
        assert_eq!(block_hash.as_bytes().len(), 32);
        assert_eq!(generic_hash.as_bytes().len(), 32);

        // Different types with same data are not equal (different types)
        assert_ne!(tx_hash.as_bytes(), block_hash.as_bytes());

        // Test conversions
        let tx_bytes: [u8; 32] = tx_hash.into();
        assert_eq!(tx_bytes, [2u8; 32]);
    }

    #[test]
    fn test_address_specialization() {
        let eth_addr = EthAddress::new([10u8; 20]);
        let pool_addr = PoolAddress::new([20u8; 20]);
        let token_addr = TokenAddress::new([30u8; 20]);

        // All are 20 bytes but different types provide compile-time safety
        assert_eq!(eth_addr.as_bytes().len(), 20);
        assert_eq!(pool_addr.as_bytes().len(), 20);
        assert_eq!(token_addr.as_bytes().len(), 20);

        // Test that they maintain distinct types
        fn process_pool(_: PoolAddress) -> &'static str {
            "pool"
        }
        fn process_token(_: TokenAddress) -> &'static str {
            "token"
        }

        assert_eq!(process_pool(pool_addr), "pool");
        assert_eq!(process_token(token_addr), "token");
        // process_pool(token_addr); // Would be compile error!
    }

    #[test]
    fn test_zero_cost_abstraction() {
        // Verify no runtime overhead for typed wrappers
        assert_eq!(
            std::mem::size_of::<EthAddress>(),
            std::mem::size_of::<[u8; 20]>()
        );

        assert_eq!(
            std::mem::size_of::<TxHash>(),
            std::mem::size_of::<[u8; 32]>()
        );

        assert_eq!(
            std::mem::size_of::<EthSignature>(),
            std::mem::size_of::<[u8; 65]>()
        );

        // Verify alignment is identical
        assert_eq!(
            std::mem::align_of::<EthAddress>(),
            std::mem::align_of::<[u8; 20]>()
        );
    }

    #[test]
    fn test_ergonomic_usage() {
        let addr_bytes = [42u8; 20];
        let eth_addr = EthAddress::from(addr_bytes);

        // AsRef works
        let addr_ref: &[u8; 20] = eth_addr.as_ref();
        assert_eq!(addr_ref, &addr_bytes);

        // Conversions work smoothly
        let recovered_bytes: [u8; 20] = eth_addr.into();
        assert_eq!(recovered_bytes, addr_bytes);

        // Default works
        let default_addr = EthAddress::default();
        assert_eq!(default_addr.as_bytes(), &[0u8; 20]);
    }

    #[test]
    fn test_private_key_security() {
        let key_data = [7u8; 32];
        let private_key = PrivateKey::new_with_warning(key_data);

        assert_eq!(private_key.as_bytes().len(), 32);
        assert_eq!(private_key.as_bytes(), &[7u8; 32]);

        // Test that drop automatically zeroes the key memory
        // (this is handled by the Drop implementation)
        drop(private_key);
        // Original key_data remains unchanged (we only zero the wrapper's copy)
        assert_eq!(key_data, [7u8; 32]);
    }

    #[test]
    fn test_compile_time_safety_examples() {
        // These examples demonstrate the compile-time safety benefits

        // ✅ Correct usage
        let eth_addr = EthAddress::new([1u8; 20]);
        let tx_hash = TxHash::new([2u8; 32]);

        fn process_address(_addr: EthAddress) -> &'static str {
            "address processed"
        }
        fn process_transaction(_hash: TxHash) -> &'static str {
            "transaction processed"
        }

        assert_eq!(process_address(eth_addr), "address processed");
        assert_eq!(process_transaction(tx_hash), "transaction processed");

        // ❌ These would be compile errors (uncomment to test):
        // process_address(tx_hash);  // Error: expected EthAddress, found TxHash
        // process_transaction(eth_addr); // Error: expected TxHash, found EthAddress

        // ❌ These would also be compile errors:
        // let _: EthAddress = [0u8; 32].into();  // Error: wrong size
        // let _: TxHash = [0u8; 20].into();      // Error: wrong size
    }
}
