use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod uniswap_v2;
pub mod uniswap_v3;
// pub mod curve;

/// Common trait for all DEX pool types
#[async_trait]
pub trait DexPool: Send + Sync {
    /// Get the DEX name (quickswap, sushiswap, etc.)
    fn dex_name(&self) -> &str;
    
    /// Get the pool address
    fn address(&self) -> &str;
    
    /// Get pool fee in basis points
    fn fee(&self) -> u32 {
        30 // Default 0.3% for V2 pools
    }
    
    /// Query token addresses from the pool contract
    async fn get_tokens(&self) -> Result<(String, String)>;
    
    /// Parse a swap event from this pool type
    fn parse_swap_event(&self, data: &str) -> Result<SwapEvent>;
    
    /// Parse pool events (Mint/Burn/Collect/Sync)
    fn parse_pool_event(&self, event_signature: &str, data: &str, topics: &[String]) -> Result<PoolEvent>;
    
    /// Calculate price from a swap event
    fn calculate_price(&self, swap: &SwapEvent, token0_decimals: u8, token1_decimals: u8) -> Price;
    
    /// Calculate liquidity impact from a pool event
    fn calculate_liquidity_impact(&self, event: &PoolEvent, token0_price: f64, token1_price: f64) -> f64 {
        let core = event.core();
        // Default implementation - can be overridden by specific pool types
        match event.event_type() {
            PoolUpdateType::Mint => {
                // Positive impact - liquidity added
                event.core().token0_symbol.parse::<f64>().unwrap_or(0.0) * token0_price +
                event.core().token1_symbol.parse::<f64>().unwrap_or(0.0) * token1_price
            },
            PoolUpdateType::Burn => {
                // Negative impact - liquidity removed
                -(event.core().token0_symbol.parse::<f64>().unwrap_or(0.0) * token0_price +
                  event.core().token1_symbol.parse::<f64>().unwrap_or(0.0) * token1_price)
            },
            _ => 0.0 // Other events don't directly affect liquidity
        }
    }
    
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

// Import event types from protocol crate
pub use alphapulse_protocol::{
    SwapEvent, SwapEventCore, SwapEventTrait,
    UniswapV2SwapEvent, UniswapV3SwapEvent, CurveSwapEvent,
    PoolEvent, PoolEventCore, PoolEventTrait, 
    UniswapV2PoolEvent, UniswapV3PoolEvent, CurvePoolEvent, BalancerPoolEvent,
    PoolUpdateType, ProtocolType
};

/// Calculated price from a swap
#[derive(Debug, Clone)]
pub struct Price {
    pub token0_symbol: String,
    pub token1_symbol: String,
    pub price: f64,  // Price of token0 in terms of token1
    pub volume: f64,  // Volume in USD equivalent
    pub timestamp: u64,
}

/// V3-specific price calculation using sqrtPriceX96
/// Returns (price_of_token0_in_token1_terms, volume_in_token1_terms)
pub fn calculate_v3_price_and_volume(swap: &SwapEvent, token0_decimals: u8, token1_decimals: u8) -> (f64, f64) {
    match swap {
        SwapEvent::UniswapV3(v3) => {
            // Convert sqrtPriceX96 to human-readable price
            let sqrt_price = v3.sqrt_price_x96 as f64 / (2_f64.powi(96));
            let raw_price = sqrt_price * sqrt_price;
            
            // Adjust for decimal differences between tokens
            // This gives us the price in terms of token1 per token0
            let decimal_adjustment = 10_f64.powi((token0_decimals as i32) - (token1_decimals as i32));
            let price = raw_price * decimal_adjustment;
            
            // Calculate volume from the swap amounts (amounts are in raw wei)
            let core = &v3.core;
            let decimals0_factor = 10_f64.powi(token0_decimals as i32);
            let decimals1_factor = 10_f64.powi(token1_decimals as i32);
            
            let amount0_in = (core.amount0_in as f64) / decimals0_factor;
            let amount1_in = (core.amount1_in as f64) / decimals1_factor;
            let amount0_out = (core.amount0_out as f64) / decimals0_factor;
            let amount1_out = (core.amount1_out as f64) / decimals1_factor;
            
            let volume = if amount0_in > 0.000001 && amount1_out > 0.000001 {
                // Swapping token0 -> token1
                amount0_in * price
            } else if amount1_in > 0.000001 && amount0_out > 0.000001 {
                // Swapping token1 -> token0
                amount1_in
            } else {
                0.0
            };
            
            (price, volume)
        }
        _ => {
            // Fallback to standard calculation for non-V3 swaps
            calculate_standard_price_and_volume(swap)
        }
    }
}

/// Standard price and volume calculation for UniswapV2-style swaps with raw amounts
/// Returns (price_of_token0_in_token1_terms, volume_in_token1_terms)
pub fn calculate_standard_price_and_volume(swap: &SwapEvent) -> (f64, f64) {
    // Work with raw amounts directly - no decimal adjustment here
    // The caller should pass token decimals if needed
    let core = swap.core();
    let amount0_in = core.amount0_in as f64;
    let amount1_in = core.amount1_in as f64;
    let amount0_out = core.amount0_out as f64;
    let amount1_out = core.amount1_out as f64;
    
    // Minimum amount in wei (about 0.000001 tokens for 18 decimals)
    const MIN_AMOUNT_WEI: f64 = 1_000_000_000_000.0; // 10^12 wei
    
    if amount0_in > MIN_AMOUNT_WEI && amount1_out > MIN_AMOUNT_WEI {
        // Swapping token0 -> token1
        // Price = how much token1 we get per token0
        let price = amount1_out / amount0_in;
        // Volume = amount of token0 being traded, valued in token1 terms
        let volume = amount0_in * price;
        (price, volume)
    } else if amount1_in > MIN_AMOUNT_WEI && amount0_out > MIN_AMOUNT_WEI {
        // Swapping token1 -> token0
        // Price = how much token1 we need per token0 (inverse of swap ratio)
        let price = amount1_in / amount0_out;
        // Volume = amount of token0 being traded, valued in token1 terms  
        let volume = amount0_out * price;
        (price, volume)
    } else {
        // Complex multi-directional swap or invalid data
        // This includes dust trades where amounts round to 0 after decimal adjustment
        tracing::debug!("Invalid swap amounts for price calculation: in0={}, out0={}, in1={}, out1={}", 
               amount0_in, amount0_out, amount1_in, amount1_out);
        (0.0, 0.0)
    }
}

/// Event signatures for different pool types - these are public Ethereum event hashes, not secrets
// nosec: Public Ethereum event signatures

// Swap event signatures
pub const UNISWAP_V2_SWAP_SIGNATURE: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
pub const UNISWAP_V3_SWAP_SIGNATURE: &str = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";
pub const CURVE_TOKEN_EXCHANGE_SIGNATURE: &str = "0x8b3e96f2b889fa771c53c981b40daf005f63f637f1869f707052d15a3dd97140";

// V2 Pool event signatures
pub const UNISWAP_V2_MINT_SIGNATURE: &str = "0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26394f4c03821c4f";
pub const UNISWAP_V2_BURN_SIGNATURE: &str = "0xdccd412f0b1252819cb1fd330b93224ca42612892bb3f4f789976e6d8136129a";
pub const UNISWAP_V2_SYNC_SIGNATURE: &str = "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1";

// V3 Pool event signatures
pub const UNISWAP_V3_MINT_SIGNATURE: &str = "0x7a53080ba414158be7ec69b987b5fb7d07dee101babe276914f785c42da22a01b";
pub const UNISWAP_V3_BURN_SIGNATURE: &str = "0x0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c";
pub const UNISWAP_V3_COLLECT_SIGNATURE: &str = "0x40d0efd1a53d60ecbf40971b9daf7dc90178c3aadc7aab1765632738fa8b8f01";

// Curve pool event signatures (future use)
pub const CURVE_ADD_LIQUIDITY_SIGNATURE: &str = "0x26f55a85081d24974e85c6c00045d0f0453991e95873f52bff0d21af4079a768";
pub const CURVE_REMOVE_LIQUIDITY_SIGNATURE: &str = "0x9878ca375e106f2a43c3b599fc624568131c4c9a4ba66a14563715763be9d59d";

// Balancer pool event signatures (future use)  
pub const BALANCER_POOL_BALANCE_CHANGED_SIGNATURE: &str = "0xe5ce249087ce04f05a957192435400fd97868dba0e6a4b4c049abf8af80dae78";

use std::collections::HashMap;
use std::sync::LazyLock;

/// HOT PATH: Static lookup table for O(1) event signature identification
static POOL_EVENT_SIGNATURES: LazyLock<HashMap<&'static str, (PoolUpdateType, EventBasedPoolType)>> = LazyLock::new(|| {
    let mut map = HashMap::with_capacity(16);
    map.insert(UNISWAP_V2_MINT_SIGNATURE, (PoolUpdateType::Mint, EventBasedPoolType::UniswapV2Style));
    map.insert(UNISWAP_V2_BURN_SIGNATURE, (PoolUpdateType::Burn, EventBasedPoolType::UniswapV2Style));
    map.insert(UNISWAP_V2_SYNC_SIGNATURE, (PoolUpdateType::Sync, EventBasedPoolType::UniswapV2Style));
    map.insert(UNISWAP_V3_MINT_SIGNATURE, (PoolUpdateType::Mint, EventBasedPoolType::UniswapV3Style));
    map.insert(UNISWAP_V3_BURN_SIGNATURE, (PoolUpdateType::Burn, EventBasedPoolType::UniswapV3Style));
    map.insert(UNISWAP_V3_COLLECT_SIGNATURE, (PoolUpdateType::Collect, EventBasedPoolType::UniswapV3Style));
    map.insert(CURVE_ADD_LIQUIDITY_SIGNATURE, (PoolUpdateType::Mint, EventBasedPoolType::CurveStyle));
    map.insert(CURVE_REMOVE_LIQUIDITY_SIGNATURE, (PoolUpdateType::Burn, EventBasedPoolType::CurveStyle));
    map.insert(BALANCER_POOL_BALANCE_CHANGED_SIGNATURE, (PoolUpdateType::Mint, EventBasedPoolType::BalancerStyle));
    map
});

/// HOT PATH: Determine pool event type and protocol from event signature (<5μs)
pub fn identify_pool_event(signature: &str) -> Option<(PoolUpdateType, EventBasedPoolType)> {
    POOL_EVENT_SIGNATURES.get(signature).copied()
}

/// Pool type based on event signature
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventBasedPoolType {
    UniswapV2Style,  // Includes QuickSwap, SushiSwap, etc.
    UniswapV3Style,  // V3 pools with tick-based liquidity
    CurveStyle,      // Curve stable pools
    BalancerStyle,   // Balancer weighted/stable pools
}

/// Factory for creating pools based on event signatures instead of contract inspection
pub struct PoolFactory {
    rpc_url: String,
    shared_client: Option<Arc<reqwest::Client>>,
}

impl PoolFactory {
    pub fn new(rpc_url: String) -> Self {
        Self { 
            rpc_url,
            shared_client: None,
        }
    }
    
    /// Create PoolFactory with shared HTTP client to prevent connection leaks
    pub fn new_with_client(rpc_url: String, client: Arc<reqwest::Client>) -> Self {
        Self { 
            rpc_url,
            shared_client: Some(client),
        }
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
                // Use shared client if available, otherwise pool creates its own
                if let Some(shared_client) = &self.shared_client {
                    Ok(Box::new(uniswap_v2::UniswapV2Pool::new_with_client(
                        address.to_string(),
                        "uniswapv2-style".to_string(),
                        self.rpc_url.clone(),
                        shared_client.clone(),
                    )))
                } else {
                    Ok(Box::new(uniswap_v2::UniswapV2Pool::new(
                        address.to_string(),
                        "uniswapv2-style".to_string(),
                        self.rpc_url.clone(),
                    )))
                }
            }
            EventBasedPoolType::UniswapV3Style => {
                // Use shared client if available, otherwise pool creates its own
                if let Some(shared_client) = &self.shared_client {
                    Ok(Box::new(uniswap_v3::UniswapV3Pool::new_with_client(
                        address.to_string(),
                        "uniswapv3".to_string(),
                        self.rpc_url.clone(),
                        shared_client.clone(),
                    )))
                } else {
                    Ok(Box::new(uniswap_v3::UniswapV3Pool::new(
                        address.to_string(),
                        "uniswapv3".to_string(),
                        self.rpc_url.clone(),
                    )))
                }
            }
            EventBasedPoolType::CurveStyle => {
                // Future: implement Curve pool
                Err(anyhow::anyhow!("Curve pools not yet implemented"))
            }
            EventBasedPoolType::BalancerStyle => {
                // Future: implement Balancer pool
                Err(anyhow::anyhow!("Balancer pools not yet implemented"))
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
                // QuickSwap uses UniswapV2 format, no need for separate implementation
                Ok(Box::new(uniswap_v2::UniswapV2Pool::new(
                    address.to_string(),
                    "quickswap".to_string(),  // Pass DEX name for proper labeling
                    self.rpc_url.clone(),
                )))
            }
            "sushiswap" => {
                // SushiSwap uses UniswapV2 format, no need for separate implementation
                Ok(Box::new(uniswap_v2::UniswapV2Pool::new(
                    address.to_string(),
                    "sushiswap".to_string(),  // Pass DEX name for proper labeling
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