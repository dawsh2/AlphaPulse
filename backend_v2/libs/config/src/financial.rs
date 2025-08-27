//! Financial constants and precision handling
//!
//! This module defines precision multipliers and constants for handling
//! financial calculations with appropriate precision per asset type.

/// Fixed-point precision for USD prices (8 decimal places)
/// Multiply by this to convert USD to fixed-point representation
pub const USD_FIXED_POINT_MULTIPLIER: i64 = 100_000_000; // 10^8

/// Fixed-point precision for USDC (6 decimal places)  
/// Native USDC precision on most chains
pub const USDC_PRECISION_MULTIPLIER: i64 = 1_000_000; // 10^6

/// Fixed-point precision for WETH (18 decimal places)
/// Native ETH/WETH precision
pub const WETH_PRECISION_MULTIPLIER: i128 = 1_000_000_000_000_000_000; // 10^18

/// Fixed-point precision for WMATIC (18 decimal places)
/// Native MATIC precision  
pub const WMATIC_PRECISION_MULTIPLIER: i128 = 1_000_000_000_000_000_000; // 10^18

/// Convert USD float to 8-decimal fixed-point
pub fn usd_to_fixed_point(usd_value: f64) -> i64 {
    (usd_value * USD_FIXED_POINT_MULTIPLIER as f64).round() as i64
}

/// Convert 8-decimal fixed-point to USD float
pub fn fixed_point_to_usd(fixed_point: i64) -> f64 {
    fixed_point as f64 / USD_FIXED_POINT_MULTIPLIER as f64
}

/// Default price values for common scenarios
pub mod defaults {
    use super::*;
    
    /// Default MATIC price in USD fixed-point ($0.33)
    pub const DEFAULT_MATIC_PRICE_USD: i64 = 33_000_000; // $0.33 in 8-decimal fixed-point
    
    /// Default gas fee in USD fixed-point
    pub const DEFAULT_GAS_FEE_USD: i64 = 250_000_000; // $2.50
    
    /// Default DEX fee in USD fixed-point  
    pub const DEFAULT_DEX_FEE_USD: i64 = 300_000_000; // $3.00
    
    /// Default slippage cost in USD fixed-point
    pub const DEFAULT_SLIPPAGE_COST_USD: i64 = 100_000_000; // $1.00
}

/// Gas price constants
pub mod gas {
    /// Typical flash arbitrage gas usage
    pub const FLASH_ARBITRAGE_GAS_UNITS: u64 = 300_000;
    
    /// Gas price cache duration (seconds)
    pub const GAS_PRICE_CACHE_DURATION_SECS: u64 = 300;
    
    /// Default gas price (30 gwei in wei)
    pub const DEFAULT_GAS_PRICE_WEI: u64 = 30_000_000_000;
}