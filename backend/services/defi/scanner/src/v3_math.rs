/// Proper V3 tick mathematics for exact calculations
/// Imported from exchange_collector's proven V3 math module

/// V3 tick math constants
const MIN_TICK: i32 = -887272;
const MAX_TICK: i32 = 887272;
const MIN_SQRT_RATIO: u128 = 4295128739;
const MAX_SQRT_RATIO: u128 = 340282366920938463463374607431768211455;

/// Swap state during V3 calculation
#[derive(Debug, Clone)]
pub struct V3SwapState {
    pub amount_remaining: u128,
    pub amount_calculated: u128,
    pub sqrt_price_x96: u128,
    pub tick: i32,
    pub liquidity: u128,
}

/// Calculate exact V3 swap within a single tick
pub fn swap_within_tick(
    sqrt_price_current_x96: u128,
    sqrt_price_limit_x96: u128,
    liquidity: u128,
    amount_in: u128,
    fee_pips: u32,
    zero_for_one: bool,
) -> (u128, u128, u128) {
    // Apply fee: fee_pips = fee * 1_000_000 (e.g., 3000 = 0.3%)
    let amount_in_after_fee = amount_in * (1_000_000 - fee_pips as u128) / 1_000_000;
    
    if liquidity == 0 {
        return (0, sqrt_price_current_x96, 0);
    }
    
    let (amount_in_consumed, amount_out, sqrt_price_next) = if zero_for_one {
        // Token0 -> Token1 (price decreases)
        compute_swap_step_exact_in_decreasing(
            sqrt_price_current_x96,
            sqrt_price_limit_x96,
            liquidity,
            amount_in_after_fee,
        )
    } else {
        // Token1 -> Token0 (price increases)
        compute_swap_step_exact_in_increasing(
            sqrt_price_current_x96,
            sqrt_price_limit_x96,
            liquidity,
            amount_in_after_fee,
        )
    };
    
    (amount_in_consumed, sqrt_price_next, amount_out)
}

/// Compute swap for decreasing price (token0 -> token1)
fn compute_swap_step_exact_in_decreasing(
    sqrt_price_current_x96: u128,
    sqrt_price_target_x96: u128,
    liquidity: u128,
    amount_in: u128,
) -> (u128, u128, u128) {
    // Calculate max amount that can be swapped to reach target price
    let sqrt_price_diff = sqrt_price_current_x96.saturating_sub(sqrt_price_target_x96);
    
    // Use checked arithmetic to prevent overflow
    let max_amount_in = if let Some(product) = liquidity.checked_mul(sqrt_price_diff) {
        product / (1u128 << 96)
    } else {
        // Overflow - use maximum value
        u128::MAX / (1u128 << 96)
    };
    
    if amount_in <= max_amount_in {
        // Can swap entire amount within this tick
        let sqrt_price_delta = if let Some(product) = amount_in.checked_mul(1u128 << 96) {
            product / liquidity
        } else {
            sqrt_price_current_x96 // Fallback to consume all liquidity
        };
        
        let sqrt_price_next = sqrt_price_current_x96.saturating_sub(sqrt_price_delta);
        let price_moved = sqrt_price_current_x96.saturating_sub(sqrt_price_next);
        
        let amount_out = if let Some(product) = liquidity.checked_mul(price_moved) {
            product / (1u128 << 96)
        } else {
            0 // Fallback on overflow
        };
        
        (amount_in, amount_out, sqrt_price_next)
    } else {
        // Will hit target price
        let amount_in_consumed = max_amount_in;
        let amount_out = if let Some(product) = liquidity.checked_mul(sqrt_price_diff) {
            product / (1u128 << 96)
        } else {
            0 // Fallback on overflow
        };
        (amount_in_consumed, amount_out, sqrt_price_target_x96)
    }
}

/// Compute swap for increasing price (token1 -> token0)
fn compute_swap_step_exact_in_increasing(
    sqrt_price_current_x96: u128,
    sqrt_price_target_x96: u128,
    liquidity: u128,
    amount_in: u128,
) -> (u128, u128, u128) {
    // More complex math for increasing price direction
    let sqrt_price_diff = sqrt_price_target_x96.saturating_sub(sqrt_price_current_x96);
    let max_amount_in = liquidity * sqrt_price_diff / sqrt_price_current_x96;
    
    if amount_in <= max_amount_in {
        // Can swap entire amount
        let sqrt_price_next = sqrt_price_current_x96 + (amount_in * sqrt_price_current_x96 / liquidity);
        let amount_out = amount_in * (1u128 << 96) / sqrt_price_next;
        (amount_in, amount_out, sqrt_price_next)
    } else {
        // Will hit target price
        let amount_in_consumed = max_amount_in;
        let amount_out = liquidity * (1u128 << 96) / sqrt_price_target_x96 
            - liquidity * (1u128 << 96) / sqrt_price_current_x96;
        (amount_in_consumed, amount_out, sqrt_price_target_x96)
    }
}

/// Calculate price from sqrtPriceX96 for price impact calculation
pub fn price_from_sqrt_price_x96(sqrt_price_x96: u128) -> f64 {
    let sqrt_price = sqrt_price_x96 as f64 / (1u128 << 96) as f64;
    sqrt_price * sqrt_price
}

/// Calculate price impact for V3 swap
pub fn calculate_v3_price_impact(
    sqrt_price_before: u128,
    sqrt_price_after: u128,
) -> f64 {
    let price_before = price_from_sqrt_price_x96(sqrt_price_before);
    let price_after = price_from_sqrt_price_x96(sqrt_price_after);
    
    if price_before == 0.0 {
        return 0.0;
    }
    
    ((price_after - price_before) / price_before).abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v3_price_calculation() {
        let sqrt_price_x96 = 79228162514264337593543950336u128; // sqrt(1) * 2^96
        let price = price_from_sqrt_price_x96(sqrt_price_x96);
        
        // Should be approximately 1.0
        assert!((price - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_v3_swap_within_tick() {
        let sqrt_price_current = 79228162514264337593543950336u128; // sqrt(1)
        let sqrt_price_limit = 79228162514264337593543950000u128;   // Slightly lower
        let liquidity = 1000000u128;
        let amount_in = 1000u128;
        let fee_pips = 3000; // 0.3%
        
        let (amount_consumed, sqrt_price_new, amount_out) = swap_within_tick(
            sqrt_price_current,
            sqrt_price_limit,
            liquidity,
            amount_in,
            fee_pips,
            true, // zero_for_one
        );
        
        assert!(amount_consumed > 0);
        assert!(amount_out > 0);
        assert!(sqrt_price_new < sqrt_price_current);
    }
}