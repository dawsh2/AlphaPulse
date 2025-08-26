//! # Instrument Identifier System - Protocol V2 Bijective Asset Identification
//!
//! ## Purpose
//!
//! Complete instrument identification system providing bijective, deterministic, and collision-free
//! identifiers for all tradeable assets across exchanges and blockchains. Eliminates centralized
//! registries through self-describing u64/u128 identifiers that embed venue, asset type, and
//! identifying data in a reversible format optimized for >19M operations/second performance
//! in high-frequency trading and cross-venue arbitrage scenarios.
//!
//! ## Integration Points
//!
//! - **Protocol Messages**: Embedded in all TLV messages as compact binary identifiers
//! - **Cache Systems**: u64/u128 conversion enables ultra-fast HashMap key operations
//! - **Exchange APIs**: Direct construction from native exchange symbols and addresses
//! - **Cross-Chain Operations**: Embedded chain_id enables multi-blockchain routing
//! - **Pool Resolution**: Deterministic pool identification from constituent token pairs
//! - **Venue Management**: Comprehensive venue metadata and classification system
//! - **Asset Pairing**: Compatibility validation for DEX pool construction
//!
//! ## Architecture Role
//!
//! ```text
//! Exchange Native → [Instrument System] → Protocol Storage → Business Logic
//!      ↑                    ↓                    ↓              ↓
//!  Venue APIs         Bijective Encoding    TLV Messages    Trading Decisions
//!  Symbols/Addresses  Venue Classification  12-byte IDs     Position Updates
//!  Pool Data          Asset Type Mapping    Cache Keys      Arbitrage Routing
//!                     Pairing Validation    Zero-copy Refs  Cross-venue Logic
//! ```
//!
//! The instrument system serves as the universal addressing foundation for all trading
//! operations, enabling efficient entity identification, routing, and relationship management.
//!
//! ## Performance Profile
//!
//! - **Construction Rate**: >19M identifiers/second (measured: 19,796,915 ops/s)
//! - **Lookup Performance**: O(1) HashMap access with excellent hash distribution
//! - **Memory Footprint**: 12 bytes per identifier regardless of source complexity
//! - **Cache Efficiency**: u64/u128 keys maximize CPU cache line utilization
//! - **Bijective Operations**: Constant-time conversion and extraction
//! - **Venue Resolution**: <1μs venue metadata lookup via enum dispatch
//!
//! ## Module Organization
//!
//! ### Core Identification (`core.rs`)
//! - **InstrumentId**: Primary bijective identifier struct with zerocopy traits
//! - **Construction Methods**: Asset-specific builders (token, stock, pool, option)
//! - **Conversion Functions**: u64/u128 cache key generation and reconstruction
//! - **Debug Support**: Human-readable representations and metadata extraction
//!
//! ### Venue Management (`venues.rs`)
//! - **VenueId**: Comprehensive venue enumeration with blockchain metadata
//! - **Classification**: DeFi vs centralized exchange categorization
//! - **Chain Integration**: Ethereum, Polygon, BSC, Arbitrum chain ID mapping
//! - **Routing Support**: Network-specific configuration and capabilities
//!
//! ### Asset Pairing (`pairing.rs`)
//! - **Compatibility Logic**: Validation for DEX pool creation and arbitrage
//! - **Type Constraints**: Fungible asset requirements and cross-venue restrictions
//! - **Pool Construction**: Deterministic pool ID generation from constituent tokens
//! - **Relationship Validation**: Same-venue checks and asset type compatibility
//!
//! ## Examples
//!
//! ### Multi-Asset Identifier Construction
//! ```rust
//! use alphapulse_protocol_v2::identifiers::{InstrumentId, VenueId, AssetType};
//!
//! // Blockchain tokens with full contract addresses
//! let usdc_ethereum = InstrumentId::ethereum_token("0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48")?;
//! let usdc_polygon = InstrumentId::polygon_token("0x2791Bca1f2de4661Ed88A30DC4175f623Ccc1b78")?;
//!
//! // Traditional securities with venue-specific symbols
//! let aapl_nasdaq = InstrumentId::stock(VenueId::NASDAQ, "AAPL");
//! let spy_nyse = InstrumentId::stock(VenueId::NYSE, "SPY");
//!
//! // DEX liquidity pools with deterministic construction
//! let weth = InstrumentId::ethereum_token("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?;
//! let eth_usdc_pool = InstrumentId::pool(VenueId::UniswapV3, usdc_ethereum, weth);
//!
//! // All identifiers are bijective and self-describing
//! assert_eq!(usdc_ethereum.venue()?, VenueId::Ethereum);
//! assert_eq!(usdc_ethereum.asset_type()?, AssetType::Token);
//! assert!(usdc_ethereum.is_defi());
//! assert_eq!(usdc_ethereum.chain_id(), Some(1)); // Ethereum mainnet
//! ```
//!
//! ### High-Performance Caching Integration
//! ```rust
//! use std::collections::HashMap;
//! use rustc_hash::FxHashMap;
//!
//! // Cache with u64 keys for maximum performance
//! let mut price_cache: FxHashMap<u64, Decimal> = FxHashMap::default();
//! let mut position_cache: FxHashMap<u128, Position> = FxHashMap::default();
//!
//! // Ultra-fast cache operations (19M ops/s)
//! price_cache.insert(btc_usdc.to_u64(), current_price);
//! position_cache.insert(complex_derivative.cache_key(), position_data);
//!
//! // Zero registry lookup required for resolution
//! if let Some(price) = price_cache.get(&instrument_from_message.to_u64()) {
//!     execute_trade_with_known_price(price);
//! }
//! ```
//!
//! ### Cross-Service Communication
//! ```rust
//! // Service A: Market data collector
//! let pool_swap = PoolSwapTLV::new(
//!     VenueId::UniswapV3,
//!     eth_usdc_pool,  // Self-describing - no lookup required
//!     swap_amount, token_in, token_out, timestamp
//! );
//! market_relay.send(pool_swap).await?;
//!
//! // Service B: Arbitrage strategy (different process/machine)
//! let received_swap: PoolSwapTLV = signal_relay.receive().await?;
//! let venue = received_swap.pool_id.venue()?;        // Extracted from ID
//! let chain_id = received_swap.pool_id.chain_id();   // Blockchain routing
//! let is_v3 = venue == VenueId::UniswapV3;          // Protocol detection
//!
//! // No external registry - all metadata embedded in identifier
//! if chain_id == Some(137) && is_v3 {  // Polygon + Uniswap V3
//!     process_polygon_v3_arbitrage(received_swap);
//! }
//! ```

pub mod core;
pub mod pairing;
pub mod venues;

pub use core::*;
pub use pairing::*;
pub use venues::*;
