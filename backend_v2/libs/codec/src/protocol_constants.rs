//! Protocol-level constants for Torq Protocol V2
//!
//! This module contains immutable protocol constants that are part of the
//! wire format specification. These values MUST remain consistent across
//! all implementations for protocol compatibility.

use std::fmt;

/// Protocol magic number for message headers
///
/// This magic number (0xDEADBEEF) MUST be the first 4 bytes of every 
/// Protocol V2 message header for validation.
pub const MESSAGE_MAGIC: u32 = 0xDEADBEEF;

/// Current protocol version
///
/// Version 1 is the stable Protocol V2 implementation supporting:
/// - 32-byte MessageHeader with TLV payload
/// - Bijective InstrumentId system  
/// - Domain-based relay routing
/// - Zero-copy message parsing
pub const PROTOCOL_VERSION: u8 = 1;

/// Maximum message size (1MB)
///
/// Prevents memory exhaustion from malformed messages.
/// This is a protocol-level constraint that all implementations must enforce.
pub const MAX_MESSAGE_SIZE: usize = 1_048_576;

/// Maximum TLV payload size per extension (64KB)
///
/// Individual TLV extensions are limited to prevent unbounded allocations.
pub const MAX_TLV_PAYLOAD_SIZE: usize = 65_536;

/// Chain and protocol identifier for DEX operations
///
/// Replaces VenueId for DEX identification, combining chain_id with
/// the specific DEX protocol. This eliminates redundancy since pool
/// addresses already determine the protocol and chain context is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChainProtocol {
    pub chain_id: u32,
    pub protocol: DEXProtocol,
}

impl ChainProtocol {
    /// Create new ChainProtocol identifier
    pub fn new(chain_id: u32, protocol: DEXProtocol) -> Self {
        Self { chain_id, protocol }
    }
    
    /// Get router address for execution
    pub fn router_address(&self) -> Option<[u8; 20]> {
        self.protocol.router_address(self.chain_id)
    }
    
    /// Get factory address for pool discovery
    pub fn factory_address(&self) -> Option<[u8; 20]> {
        self.protocol.factory_address(self.chain_id)
    }
}

impl fmt::Display for ChainProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.protocol, self.chain_id)
    }
}

/// DEX Protocol types with enhanced functionality
///
/// Single source of truth for DEX protocol properties including
/// router addresses, factory addresses, and AMM math variants.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DEXProtocol {
    UniswapV2 = 0,
    UniswapV3 = 1,
    SushiswapV2 = 2,
    QuickswapV2 = 3,
    QuickswapV3 = 4,
    CurveStableSwap = 5,
    BalancerV2 = 6,
    PancakeSwapV2 = 7,
}

impl DEXProtocol {
    /// Get router address for this protocol on given chain
    pub fn router_address(&self, chain_id: u32) -> Option<[u8; 20]> {
        match (self, chain_id) {
            // Ethereum mainnet (chain_id = 1)
            (DEXProtocol::UniswapV2, 1) => Some(hex_literal::hex!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D")),
            (DEXProtocol::UniswapV3, 1) => Some(hex_literal::hex!("E592427A0AEce92De3Edee1F18E0157C05861564")),
            (DEXProtocol::SushiswapV2, 1) => Some(hex_literal::hex!("d9e1cE17f2641f24aE83637ab66a2cca9C378B9F")),
            
            // Polygon (chain_id = 137)
            (DEXProtocol::UniswapV3, 137) => Some(hex_literal::hex!("E592427A0AEce92De3Edee1F18E0157C05861564")),
            (DEXProtocol::QuickswapV2, 137) => Some(hex_literal::hex!("a5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff")),
            (DEXProtocol::SushiswapV2, 137) => Some(hex_literal::hex!("1b02dA8Cb0d097eB8D57A175b88c7D8b47997506")),
            
            // BSC (chain_id = 56)
            (DEXProtocol::PancakeSwapV2, 56) => Some(hex_literal::hex!("10ED43C718714eb63d5aA57B78B54704E256024E")),
            
            _ => None,
        }
    }
    
    /// Get factory address for pool discovery
    pub fn factory_address(&self, chain_id: u32) -> Option<[u8; 20]> {
        match (self, chain_id) {
            // Ethereum mainnet
            (DEXProtocol::UniswapV2, 1) => Some(hex_literal::hex!("5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f")),
            (DEXProtocol::UniswapV3, 1) => Some(hex_literal::hex!("1F98431c8aD98523631AE4a59f267346ea31F984")),
            (DEXProtocol::SushiswapV2, 1) => Some(hex_literal::hex!("C0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac")),
            
            // Polygon
            (DEXProtocol::UniswapV3, 137) => Some(hex_literal::hex!("1F98431c8aD98523631AE4a59f267346ea31F984")),
            (DEXProtocol::QuickswapV2, 137) => Some(hex_literal::hex!("5757371414417b8C6CAad45bAeF941aBc7d3Ab32")),
            (DEXProtocol::SushiswapV2, 137) => Some(hex_literal::hex!("c35DADB65012eC5796536bD9864eD8773aBc74C4")),
            
            // BSC
            (DEXProtocol::PancakeSwapV2, 56) => Some(hex_literal::hex!("cA143Ce32Fe78f1f7019d7d551a6402fC5350c73")),
            
            _ => None,
        }
    }
    
    /// AMM math variant for this protocol
    pub fn math_variant(&self) -> AMMVariant {
        match self {
            DEXProtocol::UniswapV2 | DEXProtocol::SushiswapV2 | 
            DEXProtocol::QuickswapV2 | DEXProtocol::PancakeSwapV2 => {
                AMMVariant::ConstantProduct
            }
            DEXProtocol::UniswapV3 | DEXProtocol::QuickswapV3 => {
                AMMVariant::ConcentratedLiquidity
            }
            DEXProtocol::CurveStableSwap => AMMVariant::StableSwap,
            DEXProtocol::BalancerV2 => AMMVariant::WeightedPool,
        }
    }
    
    /// Default fee tier for protocol (in basis points)
    pub fn default_fee(&self) -> u32 {
        match self {
            DEXProtocol::UniswapV2 | DEXProtocol::SushiswapV2 | 
            DEXProtocol::QuickswapV2 | DEXProtocol::PancakeSwapV2 => 30, // 0.3%
            DEXProtocol::UniswapV3 | DEXProtocol::QuickswapV3 => 30, // Variable, but 0.3% is common
            DEXProtocol::CurveStableSwap => 4, // 0.04%
            DEXProtocol::BalancerV2 => 10, // 0.1%
        }
    }
}

impl fmt::Display for DEXProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DEXProtocol::UniswapV2 => write!(f, "UniswapV2"),
            DEXProtocol::UniswapV3 => write!(f, "UniswapV3"),
            DEXProtocol::SushiswapV2 => write!(f, "SushiswapV2"),
            DEXProtocol::QuickswapV2 => write!(f, "QuickswapV2"),
            DEXProtocol::QuickswapV3 => write!(f, "QuickswapV3"),
            DEXProtocol::CurveStableSwap => write!(f, "Curve"),
            DEXProtocol::BalancerV2 => write!(f, "BalancerV2"),
            DEXProtocol::PancakeSwapV2 => write!(f, "PancakeSwapV2"),
        }
    }
}

/// AMM math variants for different protocol types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AMMVariant {
    ConstantProduct,      // x * y = k (UniswapV2 style)
    ConcentratedLiquidity, // UniswapV3 style with ticks
    StableSwap,           // Curve's StableSwap invariant
    WeightedPool,         // Balancer's weighted pools
}