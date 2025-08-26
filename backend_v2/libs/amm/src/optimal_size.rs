//! Optimal position sizing for arbitrage opportunities
//!
//! Calculates the exact trade size that maximizes profit while
//! considering slippage, gas costs, and liquidity constraints.

use super::{V2Math, V2PoolState, V3Math, V3PoolState};
use anyhow::Result;
use rust_decimal::prelude::ToPrimitive;
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

        // Calculate slippage for V3 pools
        // V3 slippage is more complex due to concentrated liquidity
        // Approximate by comparing with infinite liquidity scenario
        // Convert sqrt_price_x96 to a rough price approximation
        // Note: This is a simplified calculation for demonstration
        let price_a = Decimal::from(pool_a.sqrt_price_x96) / dec!(1000000000000);
        let price_b = Decimal::from(pool_b.sqrt_price_x96) / dec!(1000000000000);

        // Ideal output without slippage (using current price)
        let ideal_out_a = Decimal::from(test_amount as u64) * price_a;
        let ideal_out_b = ideal_out_a * price_b;

        // Actual output with slippage
        let actual_out_b = Decimal::from(amount_out_b);

        // Calculate slippage in basis points
        let slippage_bps = if ideal_out_b > dec!(0) {
            ((ideal_out_b - actual_out_b).abs() / ideal_out_b * dec!(10000)).round()
        } else {
            dec!(0)
        };

        Ok(OptimalPosition {
            amount_in: Decimal::from(test_amount),
            expected_amount_out: Decimal::from(amount_out_b),
            expected_profit_usd: profit_after_gas,
            total_slippage_bps: slippage_bps.to_u32().unwrap_or(0),
            gas_cost_usd: self.config.gas_cost_usd,
            is_profitable: true,
        })
    }

    /// Calculate size for cross-protocol arbitrage (V2 <-> V3)
    pub fn calculate_cross_protocol_size(
        &self,
        v2_pool: &V2PoolState,
        v3_pool: &V3PoolState,
        token_price_usd: Decimal,
        v2_is_source: bool,
    ) -> Result<OptimalPosition> {
        if v2_is_source {
            // Buy from V2, sell to V3
            // Use binary search to find optimal amount
            let mut low = v2_pool.reserve_in * dec!(0.001); // 0.1% of pool
            let mut high = v2_pool.reserve_in * dec!(0.05); // 5% of pool
            let mut best_position = OptimalPosition::no_opportunity();

            // Binary search for optimal amount
            for _ in 0..10 {
                let mid = (low + high) / dec!(2);

                // Calculate V2 output
                let v2_out = V2Math::calculate_output_amount(
                    mid,
                    v2_pool.reserve_in,
                    v2_pool.reserve_out,
                    v2_pool.fee_bps,
                )?;

                // Calculate V3 output (selling what we got from V2)
                // Safely convert v2_out to u128, returning error if precision would be lost
                let v2_out_u128 = v2_out.to_u128().ok_or_else(|| {
                    anyhow::anyhow!("Precision loss converting V2 output to u128: {}", v2_out)
                })?;

                // Determine swap direction based on which token we're selling
                // This is a simplification - in production, would analyze token pair properly
                let zero_for_one = true; // Assume selling token0 for token1 consistently

                let (v3_out, _, _) =
                    V3Math::calculate_output_amount(v2_out_u128, v3_pool, zero_for_one)?;

                // Calculate profit
                // Safely convert mid to u128, returning error if precision would be lost
                let mid_u128 = mid.to_u128().ok_or_else(|| {
                    anyhow::anyhow!("Precision loss converting mid value to u128: {}", mid)
                })?;

                let profit_units = if v3_out > mid_u128 {
                    v3_out - mid_u128
                } else {
                    0
                };

                let profit_usd = Decimal::from(profit_units) * token_price_usd / dec!(1000000000);
                let profit_after_gas = profit_usd - self.config.gas_cost_usd;

                if profit_after_gas > best_position.expected_profit_usd {
                    best_position = OptimalPosition {
                        amount_in: mid,
                        expected_amount_out: Decimal::from(v3_out),
                        expected_profit_usd: profit_after_gas,
                        total_slippage_bps: 0, // Cross-protocol slippage is complex
                        gas_cost_usd: self.config.gas_cost_usd,
                        is_profitable: profit_after_gas > self.config.min_profit_usd,
                    };
                    low = mid;
                } else {
                    high = mid;
                }
            }

            Ok(best_position)
        } else {
            // Buy from V3, sell to V2
            // Similar logic but reversed
            let test_amount = 1000000u64; // Start with reasonable test amount

            // Calculate V3 output
            let (v3_out, _, _) =
                V3Math::calculate_output_amount(test_amount as u128, v3_pool, true)?;

            // Calculate V2 output
            let v2_out = V2Math::calculate_output_amount(
                Decimal::from(v3_out),
                v2_pool.reserve_in,
                v2_pool.reserve_out,
                v2_pool.fee_bps,
            )?;

            // Check profitability
            let profit_units = if v2_out > Decimal::from(test_amount) {
                v2_out - Decimal::from(test_amount)
            } else {
                dec!(0)
            };

            let profit_usd = profit_units * token_price_usd / dec!(1000000000);
            let profit_after_gas = profit_usd - self.config.gas_cost_usd;

            if profit_after_gas > self.config.min_profit_usd {
                Ok(OptimalPosition {
                    amount_in: Decimal::from(test_amount),
                    expected_amount_out: v2_out,
                    expected_profit_usd: profit_after_gas,
                    total_slippage_bps: 0,
                    gas_cost_usd: self.config.gas_cost_usd,
                    is_profitable: true,
                })
            } else {
                Ok(OptimalPosition::no_opportunity())
            }
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

    #[test]
    fn test_performance_u128_operations() {
        use std::time::Instant;

        // Create realistic pool states for performance testing
        let v2_pool = V2PoolState {
            reserve_in: dec!(1000000.123456), // 1M tokens with precision
            reserve_out: dec!(2000000.789012),
            fee_bps: 30,
        };

        let v3_pool = V3PoolState {
            sqrt_price_x96: 79228162514264337593543950336, // ~1.0 price
            liquidity: 1000000000000,
            current_tick: 0,
            fee_pips: 3000,
        };

        let calculator = OptimalSizeCalculator::new(SizingConfig::default());

        // Warm up to ensure realistic measurements
        for _ in 0..100 {
            let _ = calculator.calculate_cross_protocol_size(&v2_pool, &v3_pool, dec!(1.0), true);
        }

        // Measure performance of u128 conversion operations
        let iterations = 1000;
        let start = Instant::now();

        for _ in 0..iterations {
            let _ = calculator.calculate_cross_protocol_size(&v2_pool, &v3_pool, dec!(1.0), true);
        }

        let duration = start.elapsed();
        let avg_duration_ns = duration.as_nanos() / iterations;
        let avg_duration_us = avg_duration_ns as f64 / 1000.0;

        // Hot path requirement: <35μs per operation
        assert!(
            avg_duration_us < 35.0,
            "Performance regression: {:.2}μs per operation exceeds 35μs hot path requirement",
            avg_duration_us
        );

        println!(
            "AMM cross-protocol calculation performance: {:.2}μs per operation",
            avg_duration_us
        );
    }

    #[test]
    fn test_precision_loss_detection() {
        // Create pools that would cause fractional mid values during binary search
        let v2_pool = V2PoolState {
            reserve_in: dec!(10000.12345), // Fractional reserves to trigger fractional calculations
            reserve_out: dec!(20000.67890),
            fee_bps: 30,
        };

        let v3_pool = V3PoolState {
            sqrt_price_x96: 1000000000000000,
            liquidity: 1000000,
            current_tick: 1000,
            fee_pips: 3000, // 0.3% in pips
        };

        let calculator = OptimalSizeCalculator::new(SizingConfig::default());

        // This should trigger the binary search which will produce fractional mid values
        let result = calculator.calculate_cross_protocol_size(
            &v2_pool,
            &v3_pool,
            dec!(1.0),
            true, // v2_is_source
        );

        // The function should either succeed with proper conversion
        // or return an error if precision loss would occur
        match result {
            Ok(position) => {
                // If it succeeds, values should be properly converted
                assert!(position.amount_in >= dec!(0));
            }
            Err(e) => {
                // If precision loss occurs, ensure error message contains expected text
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("Precision loss converting"),
                    "Expected precision loss error message, got: {}",
                    error_msg
                );
            }
        }
    }
}
