//! Uniswap V2 AMM math with exact calculations
//!
//! Preserves full precision using Decimal type for accurate slippage
//! and optimal position sizing calculations.

use anyhow::{bail, Result};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Pool reserves and fee structure for V2 AMMs
#[derive(Debug, Clone)]
pub struct V2PoolState {
    pub reserve_in: Decimal,
    pub reserve_out: Decimal,
    pub fee_bps: u32, // Fee in basis points (30 = 0.3%)
}

/// V2 AMM math functions with zero precision loss
pub struct V2Math;

impl V2Math {
    /// Calculate exact output amount for Uniswap V2 using x*y=k formula
    ///
    /// # Arguments
    /// * `amount_in` - Input token amount (in token decimals)
    /// * `reserve_in` - Input token reserve (in token decimals)  
    /// * `reserve_out` - Output token reserve (in token decimals)
    /// * `fee_bps` - Fee in basis points (30 = 0.3%)
    ///
    /// # Returns
    /// Exact output amount after fees and slippage
    pub fn calculate_output_amount(
        amount_in: Decimal,
        reserve_in: Decimal,
        reserve_out: Decimal,
        fee_bps: u32,
    ) -> Result<Decimal> {
        // Validate inputs
        if amount_in <= dec!(0) {
            bail!("Input amount must be positive");
        }
        if reserve_in <= dec!(0) || reserve_out <= dec!(0) {
            bail!("Reserves must be positive");
        }

        // Apply fee: amount_in_after_fee = amount_in * (10000 - fee_bps) / 10000
        let fee_multiplier = Decimal::from(10000 - fee_bps) / dec!(10000);
        let amount_in_after_fee = amount_in * fee_multiplier;

        // x*y=k formula: output = (amount_in_after_fee * reserve_out) / (reserve_in + amount_in_after_fee)
        let numerator = amount_in_after_fee * reserve_out;
        let denominator = reserve_in + amount_in_after_fee;

        if denominator <= dec!(0) {
            bail!("Invalid calculation: denominator would be zero");
        }

        Ok(numerator / denominator)
    }

    /// Calculate required input amount for desired output (reverse calculation)
    pub fn calculate_input_amount(
        amount_out: Decimal,
        reserve_in: Decimal,
        reserve_out: Decimal,
        fee_bps: u32,
    ) -> Result<Decimal> {
        // Validate inputs
        if amount_out <= dec!(0) {
            bail!("Output amount must be positive");
        }
        if amount_out >= reserve_out {
            bail!("Insufficient liquidity: output exceeds reserves");
        }

        let numerator = reserve_in * amount_out * dec!(10000);
        let denominator = (reserve_out - amount_out) * Decimal::from(10000 - fee_bps);

        if denominator <= dec!(0) {
            bail!("Invalid calculation: denominator would be zero");
        }

        // Add 1 to round up (ensures sufficient input)
        Ok((numerator / denominator) + dec!(1))
    }

    /// Calculate optimal arbitrage amount using closed-form solution
    ///
    /// This finds the exact trade size that maximizes profit between two pools.
    /// Derivation: Set d(profit)/d(amount) = 0 and solve for amount.
    pub fn calculate_optimal_arbitrage_amount(
        pool_a: &V2PoolState, // Buy from this pool
        pool_b: &V2PoolState, // Sell to this pool
    ) -> Result<Decimal> {
        // Convert fees to multipliers (e.g., 30 bps = 0.997)
        let fee_a = Decimal::from(10000 - pool_a.fee_bps) / dec!(10000);
        let fee_b = Decimal::from(10000 - pool_b.fee_bps) / dec!(10000);

        // For arbitrage: Buy from pool_a, sell to pool_b
        // Optimal amount formula (closed-form solution):
        // x* = sqrt(r_a_in * r_a_out * r_b_in * r_b_out * fee_a * fee_b) - r_a_in * fee_a
        //      ---------------------------------------------------------------------
        //                                    fee_a

        let sqrt_input = pool_a.reserve_in
            * pool_a.reserve_out
            * pool_b.reserve_out
            * pool_b.reserve_in
            * fee_a
            * fee_b;

        // Check if arbitrage is possible
        if sqrt_input <= dec!(0) {
            return Ok(dec!(0)); // No profitable arbitrage
        }

        // Calculate square root
        let sqrt_value = Self::decimal_sqrt(sqrt_input)?;

        let optimal_amount = (sqrt_value - pool_a.reserve_in * fee_a) / fee_a;

        // Sanity checks
        if optimal_amount <= dec!(0) {
            return Ok(dec!(0)); // No profitable arbitrage
        }

        // Cap at reasonable percentage of pool liquidity (10%)
        let max_amount = pool_a.reserve_in.min(pool_b.reserve_out) * dec!(0.1);

        Ok(optimal_amount.min(max_amount))
    }

    /// Calculate price impact of a trade
    pub fn calculate_price_impact(
        amount_in: Decimal,
        reserve_in: Decimal,
        reserve_out: Decimal,
    ) -> Result<Decimal> {
        if amount_in <= dec!(0) || reserve_in <= dec!(0) || reserve_out <= dec!(0) {
            bail!("Invalid inputs for price impact calculation");
        }

        // Current price (before trade)
        let price_before = reserve_out / reserve_in;

        // Price after trade
        let new_reserve_in = reserve_in + amount_in;
        let new_reserve_out = reserve_out
            - Self::calculate_output_amount(
                amount_in,
                reserve_in,
                reserve_out,
                0, // No fee for impact calculation
            )?;
        let price_after = new_reserve_out / new_reserve_in;

        // Price impact as percentage
        let impact = (price_before - price_after).abs() / price_before * dec!(100);

        Ok(impact)
    }

    /// Calculate slippage for a given trade size
    /// Returns the difference between ideal and actual exchange rates
    pub fn calculate_slippage(
        amount_in: Decimal,
        reserve_in: Decimal,
        reserve_out: Decimal,
        fee_bps: u32,
    ) -> Result<Decimal> {
        // Ideal rate (infinite liquidity)
        let ideal_rate = reserve_out / reserve_in;
        let ideal_output = amount_in * ideal_rate;

        // Actual output with slippage
        let actual_output =
            Self::calculate_output_amount(amount_in, reserve_in, reserve_out, fee_bps)?;

        // Slippage as percentage
        let slippage = (ideal_output - actual_output) / ideal_output * dec!(100);

        Ok(slippage)
    }

    /// Calculate square root of a Decimal using Newton's method
    /// Maintains precision for large numbers
    fn decimal_sqrt(value: Decimal) -> Result<Decimal> {
        if value < dec!(0) {
            bail!("Cannot calculate square root of negative number");
        }
        if value == dec!(0) {
            return Ok(dec!(0));
        }

        // Initial guess
        let mut x = value;
        let mut last_x = dec!(0);
        let epsilon = dec!(0.0000000001); // Precision threshold

        // Newton's method: x_new = (x + value/x) / 2
        let max_iterations = 100;
        for _ in 0..max_iterations {
            let next_x = (x + value / x) / dec!(2);

            // Check convergence
            if (next_x - last_x).abs() < epsilon {
                return Ok(next_x);
            }

            last_x = x;
            x = next_x;
        }

        // Return best approximation if not fully converged
        Ok(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v2_output_calculation() {
        // Test case: 100 tokens in, 1000:2000 reserves, 0.3% fee
        let output =
            V2Math::calculate_output_amount(dec!(100), dec!(1000), dec!(2000), 30).unwrap();

        // Expected: ~181.32 tokens out
        assert!((output - dec!(181.32)).abs() < dec!(0.01));
    }

    #[test]
    fn test_optimal_arbitrage_amount() {
        let pool_a = V2PoolState {
            reserve_in: dec!(10000),
            reserve_out: dec!(20000),
            fee_bps: 30,
        };

        let pool_b = V2PoolState {
            reserve_in: dec!(19000),  // Note: reserves are flipped
            reserve_out: dec!(10500), // Pool B has better price
            fee_bps: 30,
        };

        let optimal = V2Math::calculate_optimal_arbitrage_amount(&pool_a, &pool_b).unwrap();

        // Should find non-zero optimal amount
        assert!(optimal > dec!(0));
        // Should be capped at 10% of liquidity
        assert!(optimal <= dec!(1000));
    }

    #[test]
    fn test_price_impact() {
        let impact = V2Math::calculate_price_impact(dec!(100), dec!(1000), dec!(2000)).unwrap();

        // Large trade should have noticeable impact
        assert!(impact > dec!(0));
        assert!(impact < dec!(20)); // But not extreme for 10% of reserves
    }

    #[test]
    fn test_sqrt_accuracy() {
        let result = V2Math::decimal_sqrt(dec!(100)).unwrap();
        assert!((result - dec!(10)).abs() < dec!(0.0001));

        let result = V2Math::decimal_sqrt(dec!(2)).unwrap();
        assert!((result - dec!(1.41421356)).abs() < dec!(0.0001));
    }
}
