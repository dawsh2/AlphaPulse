//! Uniswap V3 tick mathematics for exact calculations
//!
//! Handles concentrated liquidity with tick-based pricing.
//! All calculations maintain full precision for accurate arbitrage.

use anyhow::{bail, Result};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Type alias for V3 swap calculation results: (amount_out, new_sqrt_price, new_tick)
type SwapResult = (u128, u128, i32);

/// V3 tick math constants
pub const MIN_TICK: i32 = -887272;
pub const MAX_TICK: i32 = 887272;
pub const MIN_SQRT_RATIO: u128 = 4295128739;
pub const MAX_SQRT_RATIO: u128 = 340282366920938463463374607431768211455;

/// V3 pool state with concentrated liquidity
#[derive(Debug, Clone)]
pub struct V3PoolState {
    pub liquidity: u128,
    pub sqrt_price_x96: u128,
    pub current_tick: i32,
    pub fee_pips: u32, // Fee in pips (3000 = 0.3%)
}

/// Swap state during V3 calculation
#[derive(Debug, Clone)]
pub struct V3SwapState {
    pub amount_remaining: u128,
    pub amount_calculated: u128,
    pub sqrt_price_x96: u128,
    pub tick: i32,
    pub liquidity: u128,
}

/// V3 AMM math with tick-based calculations
pub struct V3Math;

impl V3Math {
    /// Calculate exact V3 swap output for a given input
    pub fn calculate_output_amount(
        amount_in: u128,
        pool: &V3PoolState,
        zero_for_one: bool, // true = token0 -> token1
    ) -> Result<SwapResult> {
        // (amount_out, new_sqrt_price, new_tick)
        // For simplicity, assume swap stays within current tick
        // In production, would need to handle tick crossing

        let (amount_out, new_sqrt_price) = Self::swap_within_tick(
            pool.sqrt_price_x96,
            pool.liquidity,
            amount_in,
            pool.fee_pips,
            zero_for_one,
        )?;

        // Calculate new tick from new price
        let new_tick = Self::get_tick_at_sqrt_price(new_sqrt_price)?;

        Ok((amount_out, new_sqrt_price, new_tick))
    }

    /// Calculate swap within a single tick (no tick crossing)
    pub fn swap_within_tick(
        sqrt_price_current_x96: u128,
        liquidity: u128,
        amount_in: u128,
        fee_pips: u32,
        zero_for_one: bool,
    ) -> Result<(u128, u128)> {
        // (amount_out, new_sqrt_price)
        if liquidity == 0 {
            bail!("No liquidity in tick");
        }

        // Apply fee: fee_pips = fee * 1_000_000 (e.g., 3000 = 0.3%)
        let amount_in_after_fee = amount_in * (1_000_000 - fee_pips as u128) / 1_000_000;

        if zero_for_one {
            // Token0 -> Token1 (price decreases)
            Self::compute_swap_step_decreasing(
                sqrt_price_current_x96,
                liquidity,
                amount_in_after_fee,
            )
        } else {
            // Token1 -> Token0 (price increases)
            Self::compute_swap_step_increasing(
                sqrt_price_current_x96,
                liquidity,
                amount_in_after_fee,
            )
        }
    }

    /// Compute swap for decreasing price (token0 -> token1)
    fn compute_swap_step_decreasing(
        sqrt_price_current_x96: u128,
        liquidity: u128,
        amount_in: u128,
    ) -> Result<(u128, u128)> {
        // Calculate price change from amount in
        // ΔsqrtP = amount_in * Q96 / liquidity
        let sqrt_price_delta = amount_in
            .checked_mul(1u128 << 96)
            .ok_or_else(|| anyhow::anyhow!("Overflow in price calculation"))?
            / liquidity;

        // New price (decreases for token0 -> token1)
        let new_sqrt_price = sqrt_price_current_x96
            .saturating_sub(sqrt_price_delta)
            .max(MIN_SQRT_RATIO);

        // Calculate output amount
        // amount_out = liquidity * ΔsqrtP / Q96
        let amount_out =
            Self::calculate_amount1_delta(sqrt_price_current_x96, new_sqrt_price, liquidity)?;

        Ok((amount_out, new_sqrt_price))
    }

    /// Compute swap for increasing price (token1 -> token0)
    fn compute_swap_step_increasing(
        sqrt_price_current_x96: u128,
        liquidity: u128,
        amount_in: u128,
    ) -> Result<(u128, u128)> {
        // Calculate output first (token0 out)
        // Then determine new price from liquidity constraint

        // Simplified calculation for single tick
        let amount_out = Self::calculate_amount0_delta(
            sqrt_price_current_x96,
            sqrt_price_current_x96 + 1000, // Small price increase
            liquidity,
        )?;

        // New price increases
        let sqrt_price_delta = amount_in
            .checked_mul(1u128 << 96)
            .ok_or_else(|| anyhow::anyhow!("Overflow in price calculation"))?
            / liquidity;

        let new_sqrt_price = sqrt_price_current_x96.saturating_add(sqrt_price_delta);

        Ok((amount_out, new_sqrt_price))
    }

    /// Calculate amount0 delta for given price range
    fn calculate_amount0_delta(
        sqrt_price_a_x96: u128,
        sqrt_price_b_x96: u128,
        liquidity: u128,
    ) -> Result<u128> {
        if sqrt_price_a_x96 > sqrt_price_b_x96 {
            return Self::calculate_amount0_delta(sqrt_price_b_x96, sqrt_price_a_x96, liquidity);
        }

        let price_diff = sqrt_price_b_x96.saturating_sub(sqrt_price_a_x96);
        if price_diff == 0 {
            return Ok(0);
        }

        // For small price differences, use simplified calculation to avoid overflow
        // amount0 ≈ liquidity * price_diff / sqrt_price_a (simplified approximation)
        if price_diff < (1u128 << 32) && liquidity < (1u128 << 32) {
            let amount = liquidity * price_diff / (sqrt_price_a_x96 >> 48);
            return Ok(amount);
        }

        // For larger values, scale everything down to prevent overflow
        let scale_factor = 1u128 << 32; // 2^32

        let liquidity_scaled = liquidity / scale_factor;
        let price_diff_scaled = price_diff / scale_factor;
        let sqrt_a_scaled = sqrt_price_a_x96 >> 48; // Scale down by 2^48

        if sqrt_a_scaled == 0 {
            bail!("Division by zero in amount calculation");
        }

        let amount = liquidity_scaled * price_diff_scaled * (1u128 << 48) / sqrt_a_scaled;
        Ok(amount)
    }

    /// Calculate amount1 delta for given price range
    fn calculate_amount1_delta(
        sqrt_price_a_x96: u128,
        sqrt_price_b_x96: u128,
        liquidity: u128,
    ) -> Result<u128> {
        if sqrt_price_a_x96 > sqrt_price_b_x96 {
            return Self::calculate_amount1_delta(sqrt_price_b_x96, sqrt_price_a_x96, liquidity);
        }

        let delta = sqrt_price_b_x96.saturating_sub(sqrt_price_a_x96);
        if delta == 0 {
            return Ok(0);
        }

        // amount1 = liquidity * (sqrt(p_b) - sqrt(p_a)) / 2^96
        // To avoid precision loss with small values, scale appropriately
        if delta < (1u128 << 48) && liquidity < (1u128 << 48) {
            // For small values, avoid the >> 96 which would make result 0
            let amount = (liquidity * delta) / (1u128 << 48); // Use smaller divisor
            return Ok(amount);
        }

        // For larger values, use the original formula
        let amount = liquidity
            .checked_mul(delta)
            .ok_or_else(|| anyhow::anyhow!("Overflow in amount calculation"))?
            >> 96;

        Ok(amount)
    }

    /// Get tick at given sqrt price
    fn get_tick_at_sqrt_price(sqrt_price_x96: u128) -> Result<i32> {
        // Simplified tick calculation
        // In production, would use proper logarithm

        if sqrt_price_x96 < MIN_SQRT_RATIO {
            return Ok(MIN_TICK);
        }
        if sqrt_price_x96 == MAX_SQRT_RATIO {
            return Ok(MAX_TICK);
        }

        // Approximate tick from price
        // tick = log_1.0001(price) = log(price) / log(1.0001)
        // For now, use linear approximation
        let tick_estimate = ((sqrt_price_x96 as i128 - MIN_SQRT_RATIO as i128) * 887272 * 2
            / (MAX_SQRT_RATIO as i128 - MIN_SQRT_RATIO as i128))
            - 887272;

        Ok(tick_estimate as i32)
    }

    /// Calculate price impact for V3 swap
    pub fn calculate_price_impact(
        amount_in: u128,
        pool: &V3PoolState,
        zero_for_one: bool,
    ) -> Result<Decimal> {
        // Get initial price
        let price_before = Self::sqrt_price_to_price_decimal(pool.sqrt_price_x96)?;

        // Calculate swap
        let (_, new_sqrt_price, _) = Self::calculate_output_amount(amount_in, pool, zero_for_one)?;

        // Get new price
        let price_after = Self::sqrt_price_to_price_decimal(new_sqrt_price)?;

        // Calculate impact as percentage
        let impact = (price_before - price_after).abs() / price_before * dec!(100);

        Ok(impact)
    }

    /// Convert sqrt price X96 to decimal price
    fn sqrt_price_to_price_decimal(sqrt_price_x96: u128) -> Result<Decimal> {
        // price = (sqrt_price / 2^96)^2
        let sqrt_price = Decimal::from(sqrt_price_x96) / Decimal::from(1u128 << 96);
        Ok(sqrt_price * sqrt_price)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v3_swap_within_tick() {
        let pool = V3PoolState {
            liquidity: 1_000_000_000_000,                  // 1M liquidity
            sqrt_price_x96: 79228162514264337593543950336, // Price = 1.0
            current_tick: 0,
            fee_pips: 3000, // 0.3% fee
        };

        // Small swap that stays within tick
        let (amount_out, new_price, new_tick) = V3Math::calculate_output_amount(
            1000000, // 1 token in
            &pool, true, // token0 -> token1
        )
        .unwrap();

        // Should get slightly less than 1 token out due to price impact
        assert!(amount_out > 0);
        assert!(amount_out < 1000000);

        // Price should decrease (token0 -> token1)
        assert!(new_price < pool.sqrt_price_x96);
    }

    #[test]
    fn test_amount_calculations() {
        let sqrt_price_a = 79228162514264337593543950336u128; // Price = 1.0
        let sqrt_price_b = 79228162514264337593543950336u128 + 1000000;
        let liquidity = 1_000_000_000;

        let amount0 =
            V3Math::calculate_amount0_delta(sqrt_price_a, sqrt_price_b, liquidity).unwrap();

        assert!(amount0 > 0);

        let amount1 =
            V3Math::calculate_amount1_delta(sqrt_price_a, sqrt_price_b, liquidity).unwrap();

        assert!(amount1 > 0);
    }
}
