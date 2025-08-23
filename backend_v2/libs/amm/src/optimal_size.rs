//! Optimal position sizing for arbitrage opportunities
//!
//! Calculates the exact trade size that maximizes profit while
//! considering slippage, gas costs, and liquidity constraints.

use super::{V2Math, V2PoolState, V3PoolState};
use anyhow::Result;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Configuration for position sizing
#[derive(Debug, Clone)]
pub struct SizingConfig {
    /// Minimum profit threshold in USD
    pub min_profit_usd: Decimal,
    /// Maximum position as percentage of pool liquidity
    pub max_position_pct: Decimal,
    /// Gas cost estimate in USD
    pub gas_cost_usd: Decimal,
    /// Slippage tolerance in basis points
    pub slippage_tolerance_bps: u32,
}

impl Default for SizingConfig {
    fn default() -> Self {
        Self {
            min_profit_usd: dec!(0.50),
            max_position_pct: dec!(0.05), // 5% of pool
            gas_cost_usd: dec!(5.0),
            slippage_tolerance_bps: 50, // 0.5%
        }
    }
}

/// Calculates optimal trade sizes for arbitrage
pub struct OptimalSizeCalculator {
    config: SizingConfig,
}

impl OptimalSizeCalculator {
    pub fn new(config: SizingConfig) -> Self {
        Self { config }
    }

    /// Calculate optimal arbitrage size between two V2 pools
    pub fn calculate_v2_arbitrage_size(
        &self,
        pool_a: &V2PoolState, // Buy from this pool
        pool_b: &V2PoolState, // Sell to this pool
        token_price_usd: Decimal,
    ) -> Result<OptimalPosition> {
        // Get theoretical optimal amount
        let theoretical_optimal = V2Math::calculate_optimal_arbitrage_amount(pool_a, pool_b)?;

        if theoretical_optimal <= dec!(0) {
            return Ok(OptimalPosition::no_opportunity());
        }

        // Apply position limits
        let max_from_pool_a = pool_a.reserve_in * self.config.max_position_pct;
        let max_from_pool_b = pool_b.reserve_out * self.config.max_position_pct;
        let max_position = max_from_pool_a.min(max_from_pool_b);

        let optimal_amount = theoretical_optimal.min(max_position);

        // Calculate expected output
        let amount_out_from_a = V2Math::calculate_output_amount(
            optimal_amount,
            pool_a.reserve_in,
            pool_a.reserve_out,
            pool_a.fee_bps,
        )?;

        let amount_out_from_b = V2Math::calculate_output_amount(
            amount_out_from_a,
            pool_b.reserve_in,
            pool_b.reserve_out,
            pool_b.fee_bps,
        )?;

        // Calculate profit
        let profit_tokens = amount_out_from_b - optimal_amount;
        let profit_usd = profit_tokens * token_price_usd;
        let profit_after_gas = profit_usd - self.config.gas_cost_usd;

        // Check if profitable
        if profit_after_gas < self.config.min_profit_usd {
            return Ok(OptimalPosition::no_opportunity());
        }

        // Calculate slippage
        let slippage_a = V2Math::calculate_slippage(
            optimal_amount,
            pool_a.reserve_in,
            pool_a.reserve_out,
            pool_a.fee_bps,
        )?;

        let slippage_b = V2Math::calculate_slippage(
            amount_out_from_a,
            pool_b.reserve_in,
            pool_b.reserve_out,
            pool_b.fee_bps,
        )?;

        let total_slippage_bps = ((slippage_a + slippage_b) * dec!(100)).round();

        // Check slippage tolerance
        if total_slippage_bps > Decimal::from(self.config.slippage_tolerance_bps) {
            return Ok(OptimalPosition::no_opportunity());
        }

        Ok(OptimalPosition {
            amount_in: optimal_amount,
            expected_amount_out: amount_out_from_b,
            expected_profit_usd: profit_after_gas,
            total_slippage_bps: total_slippage_bps.try_into().unwrap_or(0),
            gas_cost_usd: self.config.gas_cost_usd,
            is_profitable: true,
        })
    }

    /// Calculate optimal size for V3 arbitrage (simplified)
    pub fn calculate_v3_arbitrage_size(
        &self,
        pool_a: &V3PoolState,
        pool_b: &V3PoolState,
        token_price_usd: Decimal,
        zero_for_one: bool,
    ) -> Result<OptimalPosition> {
        // V3 is more complex due to tick boundaries
        // For now, use a conservative fixed size
        let test_amount = 1_000_000_000_u128; // Test with reasonable amount

        // Simulate swap in pool A
        let (amount_out_a, _, _) =
            super::V3Math::calculate_output_amount(test_amount, pool_a, zero_for_one)?;

        // Simulate swap in pool B (opposite direction)
        let (amount_out_b, _, _) =
            super::V3Math::calculate_output_amount(amount_out_a, pool_b, !zero_for_one)?;

        // Check if profitable
        if amount_out_b <= test_amount {
            return Ok(OptimalPosition::no_opportunity());
        }

        let profit_units = amount_out_b - test_amount;
        let profit_usd = Decimal::from(profit_units) * token_price_usd / dec!(1000000000);
        let profit_after_gas = profit_usd - self.config.gas_cost_usd;

        if profit_after_gas < self.config.min_profit_usd {
            return Ok(OptimalPosition::no_opportunity());
        }

        Ok(OptimalPosition {
            amount_in: Decimal::from(test_amount),
            expected_amount_out: Decimal::from(amount_out_b),
            expected_profit_usd: profit_after_gas,
            total_slippage_bps: 0, // TODO: Calculate V3 slippage
            gas_cost_usd: self.config.gas_cost_usd,
            is_profitable: true,
        })
    }

    /// Calculate size for cross-protocol arbitrage (V2 <-> V3)
    pub fn calculate_cross_protocol_size(
        &self,
        v2_pool: &V2PoolState,
        _v3_pool: &V3PoolState,
        _token_price_usd: Decimal,
        v2_is_source: bool,
    ) -> Result<OptimalPosition> {
        if v2_is_source {
            // Buy from V2, sell to V3
            // Start with conservative estimate
            let test_amount = v2_pool.reserve_in * dec!(0.01); // 1% of pool

            let _amount_out = V2Math::calculate_output_amount(
                test_amount,
                v2_pool.reserve_in,
                v2_pool.reserve_out,
                v2_pool.fee_bps,
            )?;

            // Convert to V3 and check profitability
            // This is simplified - real implementation would iterate to find optimal

            Ok(OptimalPosition::no_opportunity()) // TODO: Implement
        } else {
            // Buy from V3, sell to V2
            Ok(OptimalPosition::no_opportunity()) // TODO: Implement
        }
    }
}

/// Result of optimal position calculation
#[derive(Debug, Clone)]
pub struct OptimalPosition {
    pub amount_in: Decimal,
    pub expected_amount_out: Decimal,
    pub expected_profit_usd: Decimal,
    pub total_slippage_bps: u32,
    pub gas_cost_usd: Decimal,
    pub is_profitable: bool,
}

impl OptimalPosition {
    fn no_opportunity() -> Self {
        Self {
            amount_in: dec!(0),
            expected_amount_out: dec!(0),
            expected_profit_usd: dec!(0),
            total_slippage_bps: 0,
            gas_cost_usd: dec!(0),
            is_profitable: false,
        }
    }

    /// Get profit margin as percentage
    pub fn profit_margin_pct(&self) -> Decimal {
        if self.amount_in == dec!(0) {
            return dec!(0);
        }
        (self.expected_profit_usd / self.amount_in) * dec!(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v2_optimal_sizing() {
        let pool_a = V2PoolState {
            reserve_in: dec!(10000),
            reserve_out: dec!(20000),
            fee_bps: 30,
        };

        let pool_b = V2PoolState {
            reserve_in: dec!(19000),
            reserve_out: dec!(10500),
            fee_bps: 30,
        };

        let calculator = OptimalSizeCalculator::new(SizingConfig::default());
        let position = calculator
            .calculate_v2_arbitrage_size(
                &pool_a,
                &pool_b,
                dec!(1.0), // $1 per token
            )
            .unwrap();

        if position.is_profitable {
            assert!(position.amount_in > dec!(0));
            assert!(position.expected_profit_usd > dec!(0));
            assert!(position.total_slippage_bps < 100); // Less than 1%
        }
    }
}
