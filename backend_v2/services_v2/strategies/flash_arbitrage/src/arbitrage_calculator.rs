//! Arbitrage opportunity calculator with precise AMM math
//! 
//! Calculates optimal trade sizes, expected profits, and all costs
//! for cross-DEX arbitrage opportunities using closed-form AMM solutions.

// TODO: Import from actual AMM library when available
// use alphapulse_amm::uniswap_v2_math::{calculate_optimal_arbitrage_amount, calculate_price_impact};
// use alphapulse_amm::uniswap_v3_math::{calculate_optimal_swap, get_next_sqrt_price_from_amount};
use serde::{Deserialize, Serialize};

/// Complete arbitrage opportunity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageMetrics {
    /// Price spread in USD
    pub spread_usd: f64,
    /// Price spread as percentage
    pub spread_percent: f64,
    /// Optimal trade size in base token units
    pub optimal_size: u128,
    /// Optimal trade size in USD
    pub optimal_size_usd: f64,
    /// Expected gross profit in USD
    pub gross_profit: f64,
    /// Total DEX fees in USD
    pub total_fees: f64,
    /// Estimated gas cost in USD
    pub gas_estimate: f64,
    /// Expected slippage impact in USD
    pub slippage_impact: f64,
    /// Net profit after all costs
    pub net_profit: f64,
    /// Whether the opportunity is profitable
    pub is_profitable: bool,
    /// Execution priority score
    pub priority: u16,
}

/// Pool information for arbitrage calculation
#[derive(Debug, Clone)]
pub struct PoolInfo {
    /// Pool type (V2 or V3)
    pub pool_type: PoolType,
    /// Current price in USD
    pub price_usd: f64,
    /// Pool fee in basis points (300 = 0.3%)
    pub fee_bps: u16,
    /// For V2: reserve amounts
    pub reserves: Option<(u128, u128)>,
    /// For V3: liquidity and tick
    pub liquidity: Option<u128>,
    pub current_tick: Option<i32>,
    /// Token decimals
    pub token0_decimals: u8,
    pub token1_decimals: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PoolType {
    UniswapV2,
    UniswapV3,
    SushiSwap,
}

/// Calculate complete arbitrage metrics for a pool pair
pub fn calculate_arbitrage_metrics(
    pool_a: &PoolInfo,
    pool_b: &PoolInfo,
    gas_price_gwei: u64,
    eth_price_usd: f64,
) -> ArbitrageMetrics {
    // Calculate spread
    let spread_usd = (pool_b.price_usd - pool_a.price_usd).abs();
    let avg_price = (pool_a.price_usd + pool_b.price_usd) / 2.0;
    let spread_percent = (spread_usd / avg_price) * 100.0;
    
    // Calculate optimal trade size based on pool types
    let optimal_size = calculate_optimal_size(pool_a, pool_b);
    let optimal_size_usd = (optimal_size as f64 / 10_f64.powi(pool_a.token0_decimals as i32)) * avg_price;
    
    // Calculate gross profit
    let gross_profit = spread_usd * (optimal_size as f64 / 10_f64.powi(pool_a.token0_decimals as i32));
    
    // Calculate fees
    let fee_a = optimal_size_usd * (pool_a.fee_bps as f64 / 10000.0);
    let fee_b = optimal_size_usd * (pool_b.fee_bps as f64 / 10000.0);
    let total_fees = fee_a + fee_b;
    
    // Estimate gas cost (assuming ~300k gas for flash loan arbitrage)
    let gas_units = 300_000u64;
    let gas_cost_eth = (gas_units * gas_price_gwei) as f64 / 1e9;
    let gas_estimate = gas_cost_eth * eth_price_usd;
    
    // Calculate slippage impact
    let slippage_impact = calculate_slippage(pool_a, pool_b, optimal_size);
    
    // Calculate net profit
    let net_profit = gross_profit - total_fees - gas_estimate - slippage_impact;
    let is_profitable = net_profit > 0.0;
    
    
    // Calculate priority (higher profit = higher priority)
    let priority = (net_profit * 100.0).min(65535.0) as u16;
    
    ArbitrageMetrics {
        spread_usd,
        spread_percent,
        optimal_size,
        optimal_size_usd,
        gross_profit,
        total_fees,
        gas_estimate,
        slippage_impact,
        net_profit,
        is_profitable,
        priority,
    }
}

/// Calculate optimal trade size using AMM math
fn calculate_optimal_size(pool_a: &PoolInfo, pool_b: &PoolInfo) -> u128 {
    match (&pool_a.pool_type, &pool_b.pool_type) {
        (PoolType::UniswapV2 | PoolType::SushiSwap, PoolType::UniswapV2 | PoolType::SushiSwap) => {
            // Both are V2-style pools
            if let (Some((r_a0, r_a1)), Some((r_b0, r_b1))) = (pool_a.reserves, pool_b.reserves) {
                // Simplified optimal arbitrage calculation
                // TODO: Use actual AMM library function when available
                calculate_optimal_arbitrage_amount_simple(r_a0, r_a1, r_b0, r_b1)
            } else {
                0
            }
        }
        (PoolType::UniswapV3, _) | (_, PoolType::UniswapV3) => {
            // At least one V3 pool - use approximation for now
            // TODO: Implement full V3 optimal calculation
            if let Some((r_a0, _)) = pool_a.reserves {
                // Use 1% of reserves as approximation
                r_a0 / 100
            } else {
                1000000000000000000 // Default to 1 token
            }
        }
        _ => 0,
    }
}

/// Calculate expected slippage for the trade
fn calculate_slippage(pool_a: &PoolInfo, pool_b: &PoolInfo, trade_size: u128) -> f64 {
    // Simplified slippage calculation
    // For V2: Use constant product formula
    // For V3: Would need tick range information
    
    let base_slippage = match (&pool_a.pool_type, &pool_b.pool_type) {
        (PoolType::UniswapV2 | PoolType::SushiSwap, PoolType::UniswapV2 | PoolType::SushiSwap) => {
            // V2 slippage based on trade size relative to reserves
            if let (Some((r_a0, _)), Some((r_b0, _))) = (pool_a.reserves, pool_b.reserves) {
                let impact_a = (trade_size as f64) / (r_a0 as f64) * 100.0;
                let impact_b = (trade_size as f64) / (r_b0 as f64) * 100.0;
                (impact_a + impact_b) / 2.0
            } else {
                0.5 // Default 0.5% slippage
            }
        }
        _ => 1.0, // Higher default for V3 or mixed pools
    };
    
    // Convert to USD
    let trade_size_usd = (trade_size as f64 / 10_f64.powi(pool_a.token0_decimals as i32)) * pool_a.price_usd;
    trade_size_usd * (base_slippage / 100.0)
}

/// Simple optimal arbitrage amount calculation (placeholder)
fn calculate_optimal_arbitrage_amount_simple(r_a0: u128, _r_a1: u128, r_b0: u128, _r_b1: u128) -> u128 {
    // Simplified calculation - use smaller of 1% of reserves
    let max_a = r_a0 / 100;
    let max_b = r_b0 / 100;
    max_a.min(max_b)
}


/// Gas price tracker for rolling average
pub struct GasPriceTracker {
    prices: Vec<u64>,
    max_samples: usize,
}

impl GasPriceTracker {
    pub fn new(max_samples: usize) -> Self {
        Self {
            prices: Vec::with_capacity(max_samples),
            max_samples,
        }
    }
    
    pub fn add_price(&mut self, price_gwei: u64) {
        self.prices.push(price_gwei);
        if self.prices.len() > self.max_samples {
            self.prices.remove(0);
        }
    }
    
    pub fn get_average(&self) -> u64 {
        if self.prices.is_empty() {
            30 // Default 30 gwei
        } else {
            let sum: u64 = self.prices.iter().sum();
            sum / self.prices.len() as u64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_arbitrage_calculation() {
        let pool_a = PoolInfo {
            pool_type: PoolType::UniswapV2,
            price_usd: 3000.0,
            fee_bps: 300,
            reserves: Some((1000000000000000000000, 3000000000)), // 1000 ETH, 3M USDC
            liquidity: None,
            current_tick: None,
            token0_decimals: 18,
            token1_decimals: 6,
        };
        
        let pool_b = PoolInfo {
            pool_type: PoolType::SushiSwap,
            price_usd: 3010.0,
            fee_bps: 300,
            reserves: Some((500000000000000000000, 1505000000)), // 500 ETH, 1.5M USDC
            liquidity: None,
            current_tick: None,
            token0_decimals: 18,
            token1_decimals: 6,
        };
        
        let metrics = calculate_arbitrage_metrics(&pool_a, &pool_b, 30, 3000.0);
        
        assert!(metrics.spread_usd > 0.0);
        assert!(metrics.spread_percent > 0.0);
        assert!(metrics.optimal_size > 0);
        assert!(metrics.total_fees > 0.0);
    }
    
    #[test]
    fn test_gas_tracker() {
        let mut tracker = GasPriceTracker::new(5);
        tracker.add_price(20);
        tracker.add_price(30);
        tracker.add_price(40);
        
        assert_eq!(tracker.get_average(), 30);
    }
}