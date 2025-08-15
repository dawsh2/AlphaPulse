use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod uniswap_v2;
pub mod quickswap;
pub mod sushiswap;
// pub mod uniswap_v3;
// pub mod curve;

/// Common trait for all DEX pool types
#[async_trait]
pub trait DexPool: Send + Sync {
    /// Get the DEX name (quickswap, sushiswap, etc.)
    fn dex_name(&self) -> &str;
    
    /// Get the pool address
    fn address(&self) -> &str;
    
    /// Query token addresses from the pool contract
    async fn get_tokens(&self) -> Result<(String, String)>;
    
    /// Parse a swap event from this pool type
    fn parse_swap_event(&self, data: &str) -> Result<SwapEvent>;
    
    /// Calculate price from a swap event
    fn calculate_price(&self, swap: &SwapEvent) -> Price;
    
    /// Get the pool type identifier
    fn pool_type(&self) -> PoolType;
    
    /// Format a trading pair symbol according to this DEX's convention
    /// e.g., "WETH/USDC" or "WETH-USDC" depending on DEX preference
    fn format_symbol(&self, token0: &str, token1: &str) -> String {
        // Default format uses / separator (can be overridden by specific implementations)
        format!("{}:{}/{}", self.dex_name(), token0, token1)
    }
}

/// Types of DEX pools we support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolType {
    UniswapV2,
    UniswapV3,
    Curve,
    Balancer,
}

/// Parsed swap event data
#[derive(Debug, Clone)]
pub struct SwapEvent {
    pub pool_address: String,
    pub tx_hash: String,
    pub block_number: u64,
    pub amount0_in: f64,
    pub amount1_in: f64,
    pub amount0_out: f64,
    pub amount1_out: f64,
    pub to_address: String,
    pub from_address: String,
}

/// Calculated price from a swap
#[derive(Debug, Clone)]
pub struct Price {
    pub token0_symbol: String,
    pub token1_symbol: String,
    pub price: f64,  // Price of token0 in terms of token1
    pub volume: f64,  // Volume in USD equivalent
    pub timestamp: u64,
}

/// Standard price and volume calculation for UniswapV2-style swaps
/// Returns (price_of_token0_in_token1_terms, volume_in_token1_terms)
pub fn calculate_standard_price_and_volume(swap: &SwapEvent) -> (f64, f64) {
    if swap.amount0_in > 0.0 && swap.amount1_out > 0.0 {
        // Swapping token0 -> token1
        // Price = how much token1 we get per token0
        let price = swap.amount1_out / swap.amount0_in;
        // Volume = amount of token0 being traded, valued in token1 terms
        let volume = swap.amount0_in * price;
        (price, volume)
    } else if swap.amount1_in > 0.0 && swap.amount0_out > 0.0 {
        // Swapping token1 -> token0
        // Price = how much token1 we need per token0 (inverse of swap ratio)
        let price = swap.amount1_in / swap.amount0_out;
        // Volume = amount of token0 being traded, valued in token1 terms  
        let volume = swap.amount0_out * price;
        (price, volume)
    } else {
        // Complex multi-directional swap or invalid data
        (0.0, 0.0)
    }
}

/// Event signatures for different pool types - these are public Ethereum event hashes, not secrets
// nosec: Public Ethereum event signatures
pub const UNISWAP_V2_SWAP_SIGNATURE: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
pub const UNISWAP_V3_SWAP_SIGNATURE: &str = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";
pub const CURVE_TOKEN_EXCHANGE_SIGNATURE: &str = "0x8b3e96f2b889fa771c53c981b40daf005f63f637f1869f707052d15a3dd97140";

/// Pool type based on event signature
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventBasedPoolType {
    UniswapV2Style,  // Includes QuickSwap, SushiSwap, etc.
    UniswapV3Style,
    CurveStyle,
}

/// Factory for creating pools based on event signatures instead of contract inspection
pub struct PoolFactory {
    rpc_url: String,
}

impl PoolFactory {
    pub fn new(rpc_url: String) -> Self {
        Self { rpc_url }
    }
    
    /// Classify pool type based on event signature
    pub fn classify_by_event_signature(&self, event_signature: &str) -> Option<EventBasedPoolType> {
        match event_signature {
            UNISWAP_V2_SWAP_SIGNATURE => Some(EventBasedPoolType::UniswapV2Style),
            UNISWAP_V3_SWAP_SIGNATURE => Some(EventBasedPoolType::UniswapV3Style),
            CURVE_TOKEN_EXCHANGE_SIGNATURE => Some(EventBasedPoolType::CurveStyle),
            _ => None,
        }
    }
    
    /// Create pool based on event signature (no DEX identification needed)
    pub async fn create_pool_by_signature(&self, address: &str, pool_type: EventBasedPoolType) -> Result<Box<dyn DexPool>> {
        match pool_type {
            EventBasedPoolType::UniswapV2Style => {
                // Use generic UniswapV2 implementation for all V2-style pools
                // DEX name becomes optional metadata
                Ok(Box::new(uniswap_v2::UniswapV2Pool::new(
                    address.to_string(),
                    "uniswapv2-style".to_string(), // Generic name
                    self.rpc_url.clone(),
                )))
            }
            EventBasedPoolType::UniswapV3Style => {
                // Future: implement V3 pool
                Err(anyhow::anyhow!("UniswapV3 pools not yet implemented"))
            }
            EventBasedPoolType::CurveStyle => {
                // Future: implement Curve pool
                Err(anyhow::anyhow!("Curve pools not yet implemented"))
            }
        }
    }
    
    /// Detect pool type by inspecting the contract
    pub async fn detect_pool_type(&self, address: &str) -> Result<PoolType> {
        // Try V2 first (most common)
        if self.is_uniswap_v2(address).await? {
            return Ok(PoolType::UniswapV2);
        }
        
        // Try V3 (has different interface)
        if self.is_uniswap_v3(address).await? {
            return Ok(PoolType::UniswapV3);
        }
        
        // Try Curve (has different methods)
        if self.is_curve(address).await? {
            return Ok(PoolType::Curve);
        }
        
        // Default to V2 if unknown
        Ok(PoolType::UniswapV2)
    }
    
    async fn is_uniswap_v2(&self, _address: &str) -> Result<bool> {
        // Check if contract has token0() and token1() methods
        // These are standard for V2 pools
        Ok(true) // For now, default to V2
    }
    
    async fn is_uniswap_v3(&self, _address: &str) -> Result<bool> {
        // Check if contract has liquidity() and slot0() methods
        // These are specific to V3
        Ok(false)
    }
    
    async fn is_curve(&self, _address: &str) -> Result<bool> {
        // Check if contract has coins() method
        // This is specific to Curve
        Ok(false)
    }
    
    /// Create a pool instance of the right type
    pub async fn create_pool(&self, address: &str, dex_name: &str) -> Result<Box<dyn DexPool>> {
        // Create DEX-specific instances based on the identified DEX
        match dex_name {
            "quickswap" => {
                Ok(Box::new(quickswap::QuickSwapPool::new(
                    address.to_string(),
                    self.rpc_url.clone(),
                )))
            }
            "sushiswap" => {
                Ok(Box::new(sushiswap::SushiSwapPool::new(
                    address.to_string(),
                    self.rpc_url.clone(),
                )))
            }
            "uniswapv3" => {
                // For now, use generic UniswapV2 for V3 (will be improved)
                Ok(Box::new(uniswap_v2::UniswapV2Pool::new(
                    address.to_string(),
                    dex_name.to_string(),
                    self.rpc_url.clone(),
                )))
            }
            _ => {
                // Unknown or generic DEX - use generic UniswapV2 implementation
                Ok(Box::new(uniswap_v2::UniswapV2Pool::new(
                    address.to_string(),
                    dex_name.to_string(),
                    self.rpc_url.clone(),
                )))
            }
        }
    }
}