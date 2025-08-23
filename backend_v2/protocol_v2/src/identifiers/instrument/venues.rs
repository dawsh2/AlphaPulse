//! # Venue and Asset Type Registry - Protocol V2 Classification System
//!
//! ## Purpose
//!
//! Comprehensive registry of all supported trading venues and asset types for the bijective
//! identifier system. Provides hierarchical classification, blockchain metadata, and routing
//! information enabling cross-venue arbitrage, multi-chain operations, and venue-specific
//! optimizations. The registry maps venue enums to blockchain networks, exchange capabilities,
//! and asset type constraints for unified trading system operations.
//!
//! ## Integration Points
//!
//! - **InstrumentId Construction**: Venue and asset type embedding in bijective identifiers
//! - **Routing Logic**: Chain ID extraction for blockchain-specific message routing
//! - **Exchange Adapters**: Venue-specific configuration and capability detection
//! - **Pool Validation**: DEX protocol identification and liquidity pool support checking
//! - **Precision Handling**: Asset type-specific decimal precision for accurate calculations
//! - **Cross-Chain Arbitrage**: Multi-blockchain venue mapping for opportunity detection
//! - **Trading Strategies**: Venue classification for protocol-specific optimization
//!
//! ## Architecture Role
//!
//! ```text
//! Exchange APIs → [Venue Registry] → InstrumentId → Protocol Messages → Trading Logic
//!       ↑              ↓                 ↓               ↓                   ↓
//!   Native Data    Venue Classification  Asset Type      TLV Messages      Strategy Routing
//!   Symbols/Pools  Chain ID Mapping      Precision       12-byte IDs       Venue Optimization
//!   Protocol APIs  DeFi vs CEX          Fungibility      Binary Transport   Cross-chain Arb
//! ```
//!
//! The venue registry provides the taxonomic foundation for all trading operations,
//! enabling protocol-aware routing and venue-specific optimizations.
//!
//! ## Performance Profile
//!
//! - **Classification Speed**: <1μs venue property lookup via enum match
//! - **Chain ID Resolution**: O(1) blockchain mapping with zero allocation
//! - **Memory Efficiency**: 2-byte venue IDs, 1-byte asset types in identifiers
//! - **Cache Optimization**: enum-based dispatch maximizes branch prediction
//! - **Precision Lookup**: Asset type decimal mapping for calculation accuracy
//! - **Registration**: Compile-time venue definition prevents runtime errors
//!
//! ## Venue Hierarchy
//!
//! ### Traditional Finance (1-199)
//! - **Stock Exchanges (1-99)**: NYSE, NASDAQ, LSE, TSE, HKEX
//! - **Crypto CEX (100-199)**: Binance, Kraken, Coinbase, Huobi, OKEx
//! - **Derivatives (700-799)**: Deribit, BybitDerivatives, OpynProtocol
//! - **Commodities (800-899)**: COMEX, CME, ICE, ForexCom
//!
//! ### Blockchain Networks (200-299)
//! - **Layer 1**: Ethereum(1), Polygon(137), BSC(56), Arbitrum(42161)
//! - **Chain ID Mapping**: Direct EVM chain ID embedded for routing
//! - **Cross-Chain Support**: Multi-blockchain asset identification
//!
//! ### DeFi Protocols (300-699)
//! - **Ethereum DeFi (300-399)**: Uniswap, SushiSwap, Curve, Balancer, Aave
//! - **Polygon DeFi (400-499)**: QuickSwap, SushiSwapPolygon, CurvePolygon
//! - **BSC DeFi (500-599)**: PancakeSwap, VenusProtocol
//! - **Arbitrum DeFi (600-699)**: UniswapV3Arbitrum, SushiSwapArbitrum
//!
//! ## Asset Type Classification
//!
//! ### Traditional Assets (1-49)
//! - **Securities**: Stock, Bond, ETF with 2-4 decimal precision
//! - **Commodities**: Physical assets with exchange-specific handling
//! - **Currencies**: Fiat currencies with 4-decimal precision
//!
//! ### Cryptocurrency (50-99)
//! - **Native Tokens**: Token (18 decimals), Coin (8 decimals)
//! - **Specialized**: StableCoin (6 decimals), WrappedToken (18 decimals)
//! - **Non-Fungible**: NFT (unique identifiers)
//!
//! ### DeFi Instruments (100-149)
//! - **Liquidity**: Pool, LPToken with 18-decimal precision
//! - **Yield Bearing**: YieldToken for lending protocols
//! - **Governance**: GovernanceToken for DAO operations
//!
//! ### Derivatives (150-199)
//! - **Traditional**: Option, Future, Swap, Forward, CDS
//! - **Structured**: StructuredNote, ConvertibleBond
//!
//! ## Examples
//!
//! ### Venue Classification and Routing
//! ```rust
//! use alphapulse_protocol_v2::identifiers::{VenueId, AssetType};
//!
//! // Venue property checking for routing decisions
//! let venue = VenueId::UniswapV3;
//! assert!(venue.is_defi());
//! assert!(venue.supports_pools());
//! assert_eq!(venue.chain_id(), Some(1)); // Ethereum mainnet
//! assert_eq!(venue.blockchain(), Some(VenueId::Ethereum));
//!
//! // Cross-chain arbitrage routing
//! let polygon_quickswap = VenueId::QuickSwap;
//! assert_eq!(polygon_quickswap.chain_id(), Some(137)); // Polygon
//!
//! // Routing decision based on venue properties
//! if venue.is_defi() && venue.chain_id() == Some(137) {
//!     route_to_polygon_handler(message);
//! }
//! ```
//!
//! ### Asset Type Precision Handling
//! ```rust
//! // Precision-aware calculations based on asset type
//! let usdc_type = AssetType::StableCoin;
//! let weth_type = AssetType::WrappedToken;
//!
//! assert_eq!(usdc_type.typical_decimals(), 6);  // USDC: 6 decimals
//! assert_eq!(weth_type.typical_decimals(), 18); // WETH: 18 decimals
//!
//! // Use for accurate price calculations
//! let usdc_amount = 1000 * 10u64.pow(usdc_type.typical_decimals() as u32);
//! let weth_amount = 1 * 10u64.pow(weth_type.typical_decimals() as u32);
//! ```
//!
//! ### Multi-Chain DEX Protocol Discovery
//! ```rust
//! // Find all Uniswap V3 deployments across chains
//! let eth_uniswap = VenueId::UniswapV3;
//! let arb_uniswap = VenueId::UniswapV3Arbitrum;
//!
//! assert_eq!(eth_uniswap.chain_id(), Some(1));     // Ethereum
//! assert_eq!(arb_uniswap.chain_id(), Some(42161)); // Arbitrum
//!
//! // Both support the same pool interface
//! assert!(eth_uniswap.supports_pools());
//! assert!(arb_uniswap.supports_pools());
//!
//! // Route to appropriate chain-specific handlers
//! match venue.chain_id() {
//!     Some(1) => process_ethereum_pools(pool_data),
//!     Some(42161) => process_arbitrum_pools(pool_data),
//!     _ => log::warn!("Unsupported chain for pool processing"),
//! }
//! ```
//!
//! ### Asset Type Compatibility Validation
//! ```rust
//! // Validate assets can be paired in DEX pools
//! let token_type = AssetType::Token;
//! let stable_type = AssetType::StableCoin;
//! let nft_type = AssetType::NFT;
//!
//! assert!(token_type.is_fungible());      // Can be pooled
//! assert!(stable_type.is_fungible());     // Can be pooled
//! assert!(!nft_type.is_fungible());       // Cannot be pooled
//!
//! // DeFi protocol compatibility
//! assert!(token_type.is_blockchain_native());
//! assert!(!AssetType::Stock.is_blockchain_native());
//!
//! // Derivative handling
//! if asset_type.is_derivative() {
//!     apply_derivative_specific_logic(instrument);
//! }
//! ```

use num_enum::TryFromPrimitive;

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

impl std::fmt::Display for VenueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl VenueId {
    /// Get the blockchain/network for DeFi venues
    pub fn blockchain(&self) -> Option<VenueId> {
        match self {
            VenueId::UniswapV2
            | VenueId::UniswapV3
            | VenueId::SushiSwap
            | VenueId::Curve
            | VenueId::Balancer
            | VenueId::Aave
            | VenueId::Compound
            | VenueId::MakerDAO
            | VenueId::Yearn
            | VenueId::Synthetix
            | VenueId::DYdX => Some(VenueId::Ethereum),

            VenueId::QuickSwap
            | VenueId::SushiSwapPolygon
            | VenueId::CurvePolygon
            | VenueId::AavePolygon
            | VenueId::BalancerPolygon => Some(VenueId::Polygon),

            VenueId::PancakeSwap | VenueId::VenusProtocol => Some(VenueId::BinanceSmartChain),

            VenueId::UniswapV3Arbitrum | VenueId::SushiSwapArbitrum | VenueId::CurveArbitrum => {
                Some(VenueId::Arbitrum)
            }

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

    /// Check if this venue is a centralized exchange  
    pub fn is_centralized(&self) -> bool {
        matches!(*self as u16, 1..=99 | 100..=199 | 700..=799 | 800..=899)
    }

    /// Check if this venue is a DeFi protocol or blockchain
    pub fn is_defi(&self) -> bool {
        matches!(*self as u16, 200..=699)
    }

    /// Get the chain ID for blockchain venues (for EVM chains)
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

    // Structured Products (200-249)
    StructuredNote = 200,
    ConvertibleBond = 201,

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

    /// Check if this asset type is DeFi-related
    pub fn is_defi(&self) -> bool {
        matches!(*self as u8, 100..=149)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_venue_blockchain_mapping() {
        assert_eq!(VenueId::UniswapV3.blockchain(), Some(VenueId::Ethereum));
        assert_eq!(VenueId::QuickSwap.blockchain(), Some(VenueId::Polygon));
        assert_eq!(
            VenueId::PancakeSwap.blockchain(),
            Some(VenueId::BinanceSmartChain)
        );
        assert_eq!(VenueId::Binance.blockchain(), None); // CEX
    }

    #[test]
    fn test_venue_properties() {
        assert!(VenueId::UniswapV3.supports_pools());
        assert!(!VenueId::Binance.supports_pools());

        assert!(VenueId::Binance.is_centralized());
        assert!(!VenueId::UniswapV3.is_centralized());

        assert!(VenueId::UniswapV3.is_defi());
        assert!(!VenueId::NYSE.is_defi());
    }

    #[test]
    fn test_chain_ids() {
        assert_eq!(VenueId::Ethereum.chain_id(), Some(1));
        assert_eq!(VenueId::Polygon.chain_id(), Some(137));
        assert_eq!(VenueId::BinanceSmartChain.chain_id(), Some(56));
        assert_eq!(VenueId::Arbitrum.chain_id(), Some(42161));
        assert_eq!(VenueId::NYSE.chain_id(), None); // Traditional exchange
    }

    #[test]
    fn test_asset_type_properties() {
        assert!(AssetType::Token.is_fungible());
        assert!(!AssetType::NFT.is_fungible());

        assert!(AssetType::Token.is_blockchain_native());
        assert!(!AssetType::Stock.is_blockchain_native());

        assert!(AssetType::Option.is_derivative());
        assert!(!AssetType::Stock.is_derivative());

        assert!(AssetType::Pool.is_defi());
        assert!(!AssetType::Stock.is_defi());
    }

    #[test]
    fn test_typical_decimals() {
        assert_eq!(AssetType::Stock.typical_decimals(), 2);
        assert_eq!(AssetType::Token.typical_decimals(), 18);
        assert_eq!(AssetType::StableCoin.typical_decimals(), 6);
        assert_eq!(AssetType::Coin.typical_decimals(), 8);
    }
}
