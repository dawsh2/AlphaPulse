/// V3 Pool validation to prevent issues with extreme/unusable pools
use ethers::types::{Address, U256};

/// Uniswap V3 constants
const MIN_TICK: i32 = -887272;
const MAX_TICK: i32 = 887272;
const MIN_SQRT_RATIO: u128 = 4295128739;  // sqrt(1.0001^MIN_TICK) * 2^96
const MAX_SQRT_RATIO: u128 = 1461446703485210103287273052203988822378723970342;  // sqrt(1.0001^MAX_TICK) * 2^96

pub struct V3PoolValidator;

impl V3PoolValidator {
    /// Validate a V3 pool is usable for trading
    pub fn validate_pool(
        sqrt_price_x96: u128,
        tick: i32,
        liquidity: u128,
        token0_balance: U256,
        token1_balance: U256,
        token0_decimals: u8,
        token1_decimals: u8,
    ) -> Result<PoolHealth, PoolValidationError> {
        // 1. Check if liquidity exists at current tick
        if liquidity == 0 {
            return Ok(PoolHealth::NoLiquidity);
        }
        
        // 2. Check if price is at extreme boundaries
        // Don't use hardcoded numbers - use the actual V3 constants
        if sqrt_price_x96 <= MIN_SQRT_RATIO {
            return Ok(PoolHealth::PriceAtMinimum);
        }
        if sqrt_price_x96 >= MAX_SQRT_RATIO {
            return Ok(PoolHealth::PriceAtMaximum);
        }
        
        // 3. Check if tick is near boundaries (within 1% of max)
        let tick_threshold = (MAX_TICK as f64 * 0.99) as i32;
        if tick < -tick_threshold || tick > tick_threshold {
            return Ok(PoolHealth::NearBoundary);
        }
        
        // 4. Validate price consistency
        // The sqrt_price should match the tick (within rounding)
        let expected_sqrt_price = Self::sqrt_price_from_tick(tick);
        let price_deviation = ((sqrt_price_x96 as i128 - expected_sqrt_price as i128).abs() as f64) 
            / expected_sqrt_price as f64;
        
        if price_deviation > 0.001 {  // More than 0.1% deviation
            return Err(PoolValidationError::InconsistentPriceAndTick);
        }
        
        // 5. Check if pool has reasonable token balances
        if token0_balance.is_zero() && token1_balance.is_zero() {
            return Ok(PoolHealth::EmptyPool);
        }
        
        // 6. Check price sanity against token balances
        // This is tricky because liquidity can be concentrated away from current price
        // So we just check for extreme disconnection
        let price_from_sqrt = Self::price_from_sqrt_x96(sqrt_price_x96, token0_decimals, token1_decimals);
        
        if token0_balance > U256::zero() && token1_balance > U256::zero() {
            // Calculate implied price from balances
            let balance0_f64 = Self::u256_to_f64_with_decimals(token0_balance, token0_decimals);
            let balance1_f64 = Self::u256_to_f64_with_decimals(token1_balance, token1_decimals);
            let implied_price = balance1_f64 / balance0_f64;
            
            // Check if tick price is wildly different from balance ratio
            // Allow up to 100x difference (liquidity can be very concentrated)
            let price_ratio = price_from_sqrt / implied_price;
            if price_ratio < 0.01 || price_ratio > 100.0 {
                return Ok(PoolHealth::SuspiciousPrice {
                    tick_price: price_from_sqrt,
                    implied_price,
                });
            }
        }
        
        // 7. Additional checks for specific token pairs
        // For stablecoin pairs, price should be near 1
        if Self::is_stablecoin_pair(token0_balance, token1_balance) {
            if price_from_sqrt < 0.9 || price_from_sqrt > 1.1 {
                return Ok(PoolHealth::UnusualStablecoinPrice(price_from_sqrt));
            }
        }
        
        Ok(PoolHealth::Healthy)
    }
    
    /// Calculate sqrtPriceX96 from tick
    fn sqrt_price_from_tick(tick: i32) -> u128 {
        // This would use the actual Uniswap V3 math library
        // Simplified version here
        let sqrt_ratio = 1.0001_f64.powf(tick as f64 / 2.0);
        (sqrt_ratio * 2_f64.powi(96)) as u128
    }
    
    /// Calculate human-readable price from sqrtPriceX96
    fn price_from_sqrt_x96(sqrt_price_x96: u128, decimals0: u8, decimals1: u8) -> f64 {
        let sqrt_price = sqrt_price_x96 as f64 / 2_f64.powi(96);
        let price = sqrt_price * sqrt_price;
        // Adjust for decimal difference
        price * 10_f64.powi((decimals0 - decimals1) as i32)
    }
    
    /// Convert U256 to f64 with decimals
    fn u256_to_f64_with_decimals(value: U256, decimals: u8) -> f64 {
        // Simplified - would need proper handling for large values
        let value_str = value.to_string();
        let value_f64 = value_str.parse::<f64>().unwrap_or(f64::MAX);
        value_f64 / 10_f64.powi(decimals as i32)
    }
    
    /// Check if this is a stablecoin pair (USDC/USDT/DAI etc)
    fn is_stablecoin_pair(_token0: U256, _token1: U256) -> bool {
        // Would check token addresses against known stablecoins
        false  // Simplified
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PoolHealth {
    Healthy,
    NoLiquidity,
    EmptyPool,
    PriceAtMinimum,
    PriceAtMaximum,
    NearBoundary,
    SuspiciousPrice {
        tick_price: f64,
        implied_price: f64,
    },
    UnusualStablecoinPrice(f64),
}

#[derive(Debug, Clone)]
pub enum PoolValidationError {
    InconsistentPriceAndTick,
    InvalidSqrtPrice,
}

impl PoolHealth {
    /// Should we use this pool for arbitrage?
    pub fn is_usable(&self) -> bool {
        matches!(self, PoolHealth::Healthy)
    }
    
    /// Get a human-readable description
    pub fn description(&self) -> String {
        match self {
            PoolHealth::Healthy => "Pool is healthy and usable".to_string(),
            PoolHealth::NoLiquidity => "No liquidity at current price".to_string(),
            PoolHealth::EmptyPool => "Pool has no token balances".to_string(),
            PoolHealth::PriceAtMinimum => "Price at minimum tick boundary (essentially 0)".to_string(),
            PoolHealth::PriceAtMaximum => "Price at maximum tick boundary (essentially ∞)".to_string(),
            PoolHealth::NearBoundary => "Price near tick boundary, limited liquidity".to_string(),
            PoolHealth::SuspiciousPrice { tick_price, implied_price } => {
                format!("Price mismatch: tick says {:.6} but balances imply {:.6}", tick_price, implied_price)
            },
            PoolHealth::UnusualStablecoinPrice(price) => {
                format!("Unusual price {:.4} for stablecoin pair", price)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extreme_tick_detection() {
        // The pool from the example
        let result = V3PoolValidator::validate_pool(
            39440026246147520296348,  // sqrtPriceX96
            -290276,                   // tick (extremely negative)
            23616285014461648866,      // liquidity
            U256::from(5256097u64) * U256::exp10(18),  // WMATIC balance
            U256::from(741631u64) * U256::exp10(6),    // USDT balance
            18,  // WMATIC decimals
            6,   // USDT decimals
        );
        
        // Should detect this as near boundary or suspicious
        match result {
            Ok(health) => assert!(!health.is_usable()),
            Err(_) => assert!(true),  // Error is also acceptable
        }
    }
    
    #[test]
    fn test_healthy_pool() {
        // A normal pool around $0.40 per WMATIC
        let sqrt_price = (0.4_f64.sqrt() * 2_f64.powi(96)) as u128;
        let tick = 0;  // Near middle
        
        let result = V3PoolValidator::validate_pool(
            sqrt_price,
            tick,
            1000000000000,  // Some liquidity
            U256::from(1000u64) * U256::exp10(18),  // 1000 WMATIC
            U256::from(400u64) * U256::exp10(6),    // 400 USDT
            18,
            6,
        );
        
        assert!(matches!(result, Ok(PoolHealth::Healthy)));
    }
}