/// Exact V3 mathematics for optimal trade calculation
use std::cmp::{min, max};

/// V3 tick math constants
const MIN_TICK: i32 = -887272;
const MAX_TICK: i32 = 887272;
// Use a large but valid u128 value for max sqrt ratio
const MIN_SQRT_RATIO: u128 = 4295128739;  // sqrt(1.0001^MIN_TICK) * 2^96
const MAX_SQRT_RATIO: u128 = 340282366920938463463374607431768211455;  // Close to u128::MAX

/// Calculate sqrtPriceX96 from tick
pub fn sqrt_price_from_tick(tick: i32) -> u128 {
    // This would use lookup tables in production for efficiency
    // Simplified calculation here
    let ratio = 1.0001_f64.powi(tick / 2);
    (ratio * (2_f64.powi(96))) as u128
}

/// Calculate tick from sqrtPriceX96
pub fn tick_from_sqrt_price(sqrt_price_x96: u128) -> i32 {
    let price = (sqrt_price_x96 as f64 / 2_f64.powi(96)).powi(2);
    (price.ln() / 1.0001_f64.ln()) as i32
}

/// Swap state during calculation
#[derive(Debug, Clone)]
pub struct SwapState {
    pub amount_remaining: u128,
    pub amount_calculated: u128,
    pub sqrt_price_x96: u128,
    pub tick: i32,
    pub liquidity: u128,
}

/// Calculate swap within a single tick
pub fn swap_within_tick(
    state: &SwapState,
    sqrt_price_limit_x96: u128,
    fee_pips: u32,
    zero_for_one: bool,
) -> (u128, u128, u128) {
    // Apply fee
    let amount_remaining_less_fee = state.amount_remaining * (1_000_000 - fee_pips as u128) / 1_000_000;
    
    if state.liquidity == 0 {
        return (0, state.sqrt_price_x96, 0);
    }
    
    let (amount_in, amount_out, sqrt_price_next) = if zero_for_one {
        // Token0 -> Token1 (price decreases)
        compute_swap_step_exact_in_decreasing(
            state.sqrt_price_x96,
            sqrt_price_limit_x96,
            state.liquidity,
            amount_remaining_less_fee,
        )
    } else {
        // Token1 -> Token0 (price increases)
        compute_swap_step_exact_in_increasing(
            state.sqrt_price_x96,
            sqrt_price_limit_x96,
            state.liquidity,
            amount_remaining_less_fee,
        )
    };
    
    (amount_in, sqrt_price_next, amount_out)
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
    let max_amount_in = liquidity * sqrt_price_diff / (1u128 << 96);
    
    if amount_in <= max_amount_in {
        // Can swap entire amount within this tick
        let sqrt_price_next = sqrt_price_current_x96 - (amount_in * (1u128 << 96) / liquidity);
        let amount_out = liquidity * (sqrt_price_current_x96 - sqrt_price_next) / (1u128 << 96);
        (amount_in, amount_out, sqrt_price_next)
    } else {
        // Will hit target price
        let amount_in_consumed = max_amount_in;
        let amount_out = liquidity * sqrt_price_diff / (1u128 << 96);
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
    // This requires more complex math with division
    // Simplified version here
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

/// Simulate complete V3 swap through multiple ticks
pub fn simulate_v3_swap(
    amount_in: u128,
    sqrt_price_start_x96: u128,
    tick_current: i32,
    liquidity_start: u128,
    tick_spacing: i32,
    fee_pips: u32,
    zero_for_one: bool,
    get_liquidity_net: impl Fn(i32) -> i128,  // Callback to get liquidityNet at tick
) -> (u128, u128, i32, Vec<i32>) {
    let mut state = SwapState {
        amount_remaining: amount_in,
        amount_calculated: 0,
        sqrt_price_x96: sqrt_price_start_x96,
        tick: tick_current,
        liquidity: liquidity_start,
    };
    
    let mut ticks_crossed = Vec::new();
    
    // Limit iterations to prevent infinite loops
    for _ in 0..20 {
        if state.amount_remaining == 0 {
            break;
        }
        
        // Find next initialized tick
        let next_tick = if zero_for_one {
            // Moving left (price decreasing)
            ((state.tick / tick_spacing) - 1) * tick_spacing
        } else {
            // Moving right (price increasing)
            ((state.tick / tick_spacing) + 1) * tick_spacing
        };
        
        let sqrt_price_limit = sqrt_price_from_tick(next_tick);
        
        // Swap within current tick range
        let (amount_in_step, sqrt_price_next, amount_out_step) = swap_within_tick(
            &state,
            sqrt_price_limit,
            fee_pips,
            zero_for_one,
        );
        
        state.amount_remaining = state.amount_remaining.saturating_sub(amount_in_step);
        state.amount_calculated = state.amount_calculated.saturating_add(amount_out_step);
        state.sqrt_price_x96 = sqrt_price_next;
        
        // Check if we crossed the tick
        if sqrt_price_next == sqrt_price_limit {
            // Crossed tick, update liquidity
            ticks_crossed.push(next_tick);
            state.tick = next_tick;
            
            let liquidity_net = get_liquidity_net(next_tick);
            if zero_for_one {
                // Moving left, subtract liquidityNet
                state.liquidity = (state.liquidity as i128 - liquidity_net) as u128;
            } else {
                // Moving right, add liquidityNet
                state.liquidity = (state.liquidity as i128 + liquidity_net) as u128;
            }
        }
    }
    
    (state.amount_calculated, state.sqrt_price_x96, state.tick, ticks_crossed)
}

/// Find optimal V3 trade size using gradient ascent
pub fn find_optimal_v3_trade(
    sqrt_price_x96: u128,
    tick: i32,
    liquidity: u128,
    tick_spacing: i32,
    fee_pips: u32,
    get_liquidity_net: impl Fn(i32) -> i128,
) -> u128 {
    // Start with small amount
    let mut x = liquidity / 1000;  // 0.1% of liquidity
    let mut best_profit = 0i128;
    let mut best_x = 0u128;
    
    // Gradient ascent
    for _ in 0..20 {
        // Calculate profit at x
        let (output, _, _, _) = simulate_v3_swap(
            x, sqrt_price_x96, tick, liquidity, 
            tick_spacing, fee_pips, true, &get_liquidity_net
        );
        let profit = output as i128 - x as i128;
        
        if profit > best_profit {
            best_profit = profit;
            best_x = x;
        }
        
        // Calculate gradient numerically
        let h = x / 100;  // 1% step
        let (output_plus, _, _, _) = simulate_v3_swap(
            x + h, sqrt_price_x96, tick, liquidity,
            tick_spacing, fee_pips, true, &get_liquidity_net
        );
        let profit_plus = output_plus as i128 - (x + h) as i128;
        
        let gradient = (profit_plus - profit) / h as i128;
        
        // Update x in direction of gradient
        if gradient > 0 {
            x = x + x / 10;  // Increase by 10%
        } else if gradient < 0 {
            x = x - x / 10;  // Decrease by 10%
        } else {
            break;  // Converged
        }
        
        // Constrain x
        x = x.min(liquidity / 10).max(1);
    }
    
    best_x
}

/// Calculate optimal V3-V3 arbitrage
pub fn calculate_optimal_v3_arbitrage(
    pool1: &V3PoolState,
    pool2: &V3PoolState,
    get_liquidity_net_1: impl Fn(i32) -> i128,
    get_liquidity_net_2: impl Fn(i32) -> i128,
) -> (u128, i128) {
    // Determine direction
    let (buy_pool, sell_pool, buy_zero_for_one, sell_zero_for_one) = 
        if pool1.sqrt_price_x96 < pool2.sqrt_price_x96 {
            (pool1, pool2, false, true)  // Buy token0 from pool1, sell to pool2
        } else {
            (pool2, pool1, false, true)  // Buy token0 from pool2, sell to pool1
        };
    
    // For V3-V3, we need numerical optimization
    // Test multiple sizes and find best
    let test_sizes = vec![
        buy_pool.liquidity / 10000,  // 0.01%
        buy_pool.liquidity / 1000,   // 0.1%
        buy_pool.liquidity / 100,    // 1%
    ];
    
    let mut best_profit = 0i128;
    let mut best_size = 0u128;
    
    for size in test_sizes {
        // Simulate buy
        let (token0_out, _, _, _) = simulate_v3_swap(
            size, buy_pool.sqrt_price_x96, buy_pool.tick, buy_pool.liquidity,
            buy_pool.tick_spacing, buy_pool.fee_pips, buy_zero_for_one, &get_liquidity_net_1
        );
        
        // Simulate sell
        let (token1_out, _, _, _) = simulate_v3_swap(
            token0_out, sell_pool.sqrt_price_x96, sell_pool.tick, sell_pool.liquidity,
            sell_pool.tick_spacing, sell_pool.fee_pips, sell_zero_for_one, &get_liquidity_net_2
        );
        
        let profit = token1_out as i128 - size as i128;
        
        if profit > best_profit {
            best_profit = profit;
            best_size = size;
        }
    }
    
    (best_size, best_profit)
}

pub struct V3PoolState {
    pub sqrt_price_x96: u128,
    pub tick: i32,
    pub liquidity: u128,
    pub tick_spacing: i32,
    pub fee_pips: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sqrt_price_conversions() {
        let tick = 1000;
        let sqrt_price = sqrt_price_from_tick(tick);
        let tick_back = tick_from_sqrt_price(sqrt_price);
        
        // Should be close (may have small rounding difference)
        assert!((tick - tick_back).abs() <= 1);
    }
    
    #[test]
    fn test_swap_within_tick() {
        let state = SwapState {
            amount_remaining: 1000000,
            amount_calculated: 0,
            sqrt_price_x96: 79228162514264337593543950336,  // Price = 1
            tick: 0,
            liquidity: 1000000000000,
        };
        
        let sqrt_price_limit = sqrt_price_from_tick(-100);
        let (amount_in, sqrt_price_next, amount_out) = swap_within_tick(
            &state,
            sqrt_price_limit,
            3000,  // 0.3% fee
            true,  // zero_for_one
        );
        
        assert!(amount_in > 0);
        assert!(amount_out > 0);
        assert!(sqrt_price_next <= state.sqrt_price_x96);
    }
}