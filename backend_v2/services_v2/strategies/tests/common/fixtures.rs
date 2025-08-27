//! Common test fixtures for strategy testing

use alphapulse_types::InstrumentId;
use rust_decimal::Decimal;

/// Common test instrument IDs
pub const TEST_BTC_USD_INSTRUMENT: &str = "BTC-USD";
pub const TEST_ETH_USD_INSTRUMENT: &str = "ETH-USD";
pub const TEST_WETH_USDC_POOL: &str = "0xa0b86a33e6ba26b1c3c39a4cc8a0b86a33e6ba26";

/// Standard test prices for various assets
pub struct TestPrices;

impl TestPrices {
    pub const BTC_USD: f64 = 45_000.0;
    pub const ETH_USD: f64 = 2_800.0;
    pub const WETH_PRICE: f64 = 2_800.0;
    pub const USDC_PRICE: f64 = 1.0;
}

/// Create test timestamp in nanoseconds
pub fn test_timestamp_ns() -> u64 {
    1700000000000000000 // 2023-11-14 22:13:20 UTC
}

/// Create test timestamp with offset
pub fn test_timestamp_ns_offset(offset_seconds: i64) -> u64 {
    let base = 1700000000000000000u64;
    if offset_seconds >= 0 {
        base + (offset_seconds as u64 * 1_000_000_000)
    } else {
        base - ((-offset_seconds) as u64 * 1_000_000_000)
    }
}

/// Generate a sequence of realistic price movements for testing
pub fn generate_price_series(base_price: f64, count: usize, volatility: f64) -> Vec<f64> {
    let mut prices = Vec::with_capacity(count);
    let mut current_price = base_price;
    
    for i in 0..count {
        // Simple price walk with some trend and randomness simulation
        let trend = (i as f64 * 0.001) - 0.05; // Slight downward trend
        let noise = (i as f64 * 1.618) % 2.0 - 1.0; // Pseudo-random between -1 and 1
        
        current_price *= 1.0 + (trend + noise * volatility);
        prices.push(current_price);
    }
    
    prices
}

/// Create test pool state for arbitrage testing
pub fn create_test_pool_state(
    reserve_0: u128,
    reserve_1: u128,
    fee_tier: u32,
) -> Result<(), String> {
    // This would create a pool state for testing
    // Implementation depends on actual PoolState structure
    Ok(())
}

/// Standard test amounts in wei
pub struct TestAmounts;

impl TestAmounts {
    pub const ONE_ETH_WEI: u128 = 1_000_000_000_000_000_000; // 1 ETH in wei
    pub const THOUSAND_USDC: u128 = 1_000_000_000; // 1000 USDC (6 decimals)
    pub const MIN_ARBITRAGE_PROFIT: u128 = 10_000_000_000_000_000; // 0.01 ETH minimum
}

/// Create test configuration values
pub struct TestConfig;

impl TestConfig {
    pub fn default_min_profit_bps() -> u32 { 50 } // 0.5%
    pub fn default_max_slippage_bps() -> u32 { 10 } // 0.1%
    pub fn default_gas_limit() -> u64 { 500_000 }
    pub fn default_confidence_threshold() -> u8 { 70 }
}