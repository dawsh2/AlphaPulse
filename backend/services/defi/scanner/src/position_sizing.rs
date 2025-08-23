use crate::{ArbitrageOpportunity, PoolInfo, amm_math::{AmmMath, PoolReserves, V3PoolState}};
use anyhow::Result;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tracing::{debug, warn};

/// Position sizing service based on liquidity analysis and slippage constraints
pub struct PositionSizer {
    config: PositionSizingConfig,
}

#[derive(Debug, Clone)]
pub struct PositionSizingConfig {
    pub max_price_impact: Decimal,      // Maximum slippage tolerance (e.g., 2%)
    pub max_flash_amount_usd: Decimal,  // Global position limit
    pub liquidity_safety_factor: Decimal, // Use only X% of available liquidity
    pub min_confidence_threshold: f64,   // Skip low-confidence opportunities
}

impl Default for PositionSizingConfig {
    fn default() -> Self {
        Self {
            max_price_impact: dec!(0.02),        // 2% max price impact
            max_flash_amount_usd: dec!(10000),   // $10K max position
            liquidity_safety_factor: dec!(0.1),  // Use max 10% of pool liquidity
            min_confidence_threshold: 0.8,       // 80% confidence minimum
        }
    }
}

impl PositionSizer {
    pub fn new(config: PositionSizingConfig) -> Self {
        Self { config }
    }

    /// Calculate optimal position size based on liquidity constraints and slippage
    pub fn calculate_optimal_size(&self, opportunity: &ArbitrageOpportunity) -> Result<Decimal> {
        debug!("Calculating optimal position size for opportunity {}", opportunity.id);

        // Get pool information from opportunity
        let buy_pool = self.extract_pool_info_from_opportunity(opportunity, true)?;
        let sell_pool = self.extract_pool_info_from_opportunity(opportunity, false)?;

        // Calculate maximum trade size for each pool based on slippage constraints
        let max_buy_size = self.calculate_max_size_for_pool(&buy_pool)?;
        let max_sell_size = self.calculate_max_size_for_pool(&sell_pool)?;

        // Use the most restrictive constraint
        let liquidity_constraint = max_buy_size.min(max_sell_size);

        // Apply global position limit
        let position_limit = self.config.max_flash_amount_usd;

        // Use existing AMM math to find optimal arbitrage amount
        let optimal_amount = if buy_pool.pool_type == "v2" && sell_pool.pool_type == "v2" {
            self.calculate_optimal_v2_arbitrage(&buy_pool, &sell_pool)?
        } else {
            // For V3 or mixed pools, use conservative estimate
            liquidity_constraint.min(position_limit) * dec!(0.5)
        };

        // Apply all constraints
        let final_size = optimal_amount
            .min(liquidity_constraint)
            .min(position_limit);

        debug!("Position sizing result: optimal={}, liquidity_constraint={}, position_limit={}, final={}",
               optimal_amount, liquidity_constraint, position_limit, final_size);

        Ok(final_size)
    }

    /// Calculate maximum trade size for a pool to stay within slippage tolerance
    fn calculate_max_size_for_pool(&self, pool: &PoolInfo) -> Result<Decimal> {
        match pool.pool_type.as_str() {
            "v2" => {
                // Use existing AMM math for V2 pools
                let max_price_impact_bps = (self.config.max_price_impact * dec!(10000)).to_u32().unwrap_or(200);
                
                AmmMath::calculate_max_trade_size(
                    pool.reserve_in,
                    pool.reserve_out,
                    max_price_impact_bps,
                    pool.fee_bps
                )
            },
            "v3" => {
                // For V3 pools, use liquidity-based sizing
                let active_liquidity = pool.v3_liquidity.unwrap_or_default();
                
                // Simplified V3 sizing: use % of active liquidity
                let max_size = active_liquidity * self.config.liquidity_safety_factor;
                Ok(max_size)
            },
            _ => {
                warn!("Unknown pool type: {}, using conservative sizing", pool.pool_type);
                Ok(dec!(1000)) // Conservative fallback
            }
        }
    }

    /// Calculate optimal V2 arbitrage amount using closed-form solution
    fn calculate_optimal_v2_arbitrage(&self, buy_pool: &PoolInfo, sell_pool: &PoolInfo) -> Result<Decimal> {
        // Use existing AMM math optimal arbitrage calculation
        AmmMath::calculate_optimal_v2_arbitrage(
            buy_pool.reserve_in,
            buy_pool.reserve_out,
            buy_pool.fee_bps,
            sell_pool.reserve_in,
            sell_pool.reserve_out,
            sell_pool.fee_bps,
        )
    }

    /// Extract pool information from opportunity
    fn extract_pool_info_from_opportunity(&self, opportunity: &ArbitrageOpportunity, is_buy_pool: bool) -> Result<PoolInfo> {
        // This is a simplified implementation - in production, would get real pool data
        // from the opportunity's buy_pool/sell_pool fields
        
        let pool_address = if is_buy_pool {
            &opportunity.buy_pool
        } else {
            &opportunity.sell_pool
        };

        // For now, create a mock pool info - would be replaced with real pool queries
        Ok(PoolInfo {
            address: pool_address.clone(),
            exchange: if is_buy_pool { opportunity.buy_exchange.clone() } else { opportunity.sell_exchange.clone() },
            token0: opportunity.token_in.clone(),
            token1: opportunity.token_out.clone(),
            reserve_in: dec!(1000000),  // Would fetch from chain
            reserve_out: dec!(500000),  // Would fetch from chain
            fee_bps: 30,               // Would get from pool
            pool_type: "v2".to_string(), // Would detect from contract
            v3_liquidity: None,
        })
    }

    /// Calculate position size for multi-hop arbitrage
    pub fn calculate_multihop_position(&self, path: &[PoolReserves], gas_cost_usd: Decimal) -> Result<Decimal> {
        // Use existing AMM math for multi-hop optimal sizing
        AmmMath::calculate_optimal_multihop_arbitrage(path, gas_cost_usd)
    }

    /// Validate that position size respects confidence constraints
    pub fn validate_position_confidence(&self, opportunity: &ArbitrageOpportunity, position_size: Decimal) -> bool {
        // Skip low-confidence opportunities regardless of size
        if opportunity.confidence_score < self.config.min_confidence_threshold {
            debug!("Skipping low confidence opportunity: {:.2} < {:.2}", 
                   opportunity.confidence_score, self.config.min_confidence_threshold);
            return false;
        }

        // Larger positions require higher confidence
        let size_factor = position_size / self.config.max_flash_amount_usd;
        let required_confidence = if size_factor > dec!(0.5) {
            0.9 // Require 90% confidence for large positions
        } else {
            self.config.min_confidence_threshold
        };

        opportunity.confidence_score >= required_confidence
    }

    /// Get effective liquidity for position sizing
    pub fn get_effective_liquidity(&self, pool: &PoolInfo) -> Decimal {
        match pool.pool_type.as_str() {
            "v2" => {
                // V2: Use geometric mean of reserves as effective liquidity
                let product = pool.reserve_in * pool.reserve_out;
                // Since we can't use sqrt directly, use approximation
                let sqrt_approx = if product > dec!(0) {
                    // Use Newton's method for sqrt (simplified)
                    let mut x = product / dec!(2);
                    for _ in 0..5 {
                        x = (x + product / x) / dec!(2);
                    }
                    x
                } else {
                    dec!(0)
                };
                sqrt_approx * self.config.liquidity_safety_factor
            },
            "v3" => {
                // V3: Use active liquidity in current tick
                pool.v3_liquidity.unwrap_or_default() * self.config.liquidity_safety_factor
            },
            _ => {
                // Unknown pool type - use minimum of reserves
                pool.reserve_in.min(pool.reserve_out) * self.config.liquidity_safety_factor
            }
        }
    }

    /// Calculate risk-adjusted position size
    pub fn calculate_risk_adjusted_size(&self, opportunity: &ArbitrageOpportunity, base_size: Decimal) -> Decimal {
        let mut adjusted_size = base_size;

        // Reduce size for low confidence
        if opportunity.confidence_score < 0.9 {
            let confidence_factor = Decimal::from_f64_retain(opportunity.confidence_score).unwrap_or(dec!(0.8));
            adjusted_size = adjusted_size * confidence_factor;
        }

        // Reduce size for high gas costs relative to profit
        let gas_to_profit_ratio = opportunity.gas_cost_estimate / opportunity.profit_usd.max(dec!(0.01));
        if gas_to_profit_ratio > dec!(0.3) {
            adjusted_size = adjusted_size * dec!(0.7);
        }

        // Ensure minimum viable size
        adjusted_size.max(dec!(1)) // Minimum $1 trade
    }
}

/// Extended pool information for position sizing
#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub address: String,
    pub exchange: String,
    pub token0: String,
    pub token1: String,
    pub reserve_in: Decimal,
    pub reserve_out: Decimal,
    pub fee_bps: u32,
    pub pool_type: String, // "v2", "v3", "curve"
    pub v3_liquidity: Option<Decimal>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_sizing_config() {
        let config = PositionSizingConfig::default();
        assert_eq!(config.max_price_impact, dec!(0.02));
        assert_eq!(config.max_flash_amount_usd, dec!(10000));
    }

    #[test]
    fn test_effective_liquidity_v2() {
        let sizer = PositionSizer::new(PositionSizingConfig::default());
        
        let pool = PoolInfo {
            address: "0x123".to_string(),
            exchange: "uniswap".to_string(),
            token0: "USDC".to_string(),
            token1: "WETH".to_string(),
            reserve_in: dec!(1000000),  // 1M USDC
            reserve_out: dec!(400),     // 400 WETH
            fee_bps: 30,
            pool_type: "v2".to_string(),
            v3_liquidity: None,
        };

        let liquidity = sizer.get_effective_liquidity(&pool);
        
        // Should be roughly sqrt(1M * 400) * 0.1 = ~20,000 * 0.1 = ~2,000
        assert!(liquidity > dec!(1000));
        assert!(liquidity < dec!(5000));
    }

    #[test]
    fn test_confidence_validation() {
        let sizer = PositionSizer::new(PositionSizingConfig::default());
        
        let high_confidence_opp = ArbitrageOpportunity {
            id: "test".to_string(),
            token_in: "USDC".to_string(),
            token_out: "WETH".to_string(),
            amount_in: dec!(1000),
            amount_out: dec!(1020),
            profit_usd: dec!(20),
            profit_percentage: dec!(0.02),
            buy_exchange: "uniswap".to_string(),
            sell_exchange: "sushiswap".to_string(),
            buy_pool: "0x123".to_string(),
            sell_pool: "0x456".to_string(),
            gas_cost_estimate: dec!(1),
            net_profit_usd: dec!(19),
            timestamp: 0,
            block_number: 0,
            confidence_score: 0.95,
        };

        assert!(sizer.validate_position_confidence(&high_confidence_opp, dec!(1000)));

        let low_confidence_opp = ArbitrageOpportunity {
            confidence_score: 0.5,
            ..high_confidence_opp
        };

        assert!(!sizer.validate_position_confidence(&low_confidence_opp, dec!(1000)));
    }
}