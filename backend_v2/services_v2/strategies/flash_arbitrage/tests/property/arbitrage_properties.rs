//! Property-based tests for arbitrage logic

use alphapulse_flash_arbitrage::math::{V2Math, V2PoolState};
use proptest::prelude::*;
use rust_decimal::Decimal;

// Property: Output calculation should always be less than reserves
proptest! {
    #[test]
    fn output_less_than_reserves(
        amount_in in 1u64..1000000u64,
        reserve_in in 1000u64..100000000u64,
        reserve_out in 1000u64..100000000u64,
        fee_bps in 1u32..1000u32,
    ) {
        let amount_in_dec = Decimal::from(amount_in);
        let reserve_in_dec = Decimal::from(reserve_in);
        let reserve_out_dec = Decimal::from(reserve_out);

        if let Ok(output) = V2Math::calculate_output_amount(
            amount_in_dec,
            reserve_in_dec,
            reserve_out_dec,
            fee_bps,
        ) {
            prop_assert!(output < reserve_out_dec);
            prop_assert!(output > Decimal::ZERO);
        }
    }
}

// Property: x*y=k invariant should hold after swap
proptest! {
    #[test]
    fn constant_product_invariant(
        amount_in in 1u64..10000u64,
        reserve_in in 10000u64..1000000u64,
        reserve_out in 10000u64..1000000u64,
    ) {
        let amount_in_dec = Decimal::from(amount_in);
        let reserve_in_dec = Decimal::from(reserve_in);
        let reserve_out_dec = Decimal::from(reserve_out);

        let k_before = reserve_in_dec * reserve_out_dec;

        if let Ok(amount_out) = V2Math::calculate_output_amount(
            amount_in_dec,
            reserve_in_dec,
            reserve_out_dec,
            0, // No fee for pure invariant test
        ) {
            let new_reserve_in = reserve_in_dec + amount_in_dec;
            let new_reserve_out = reserve_out_dec - amount_out;
            let k_after = new_reserve_in * new_reserve_out;

            // k should be preserved (within rounding)
            let diff = (k_after - k_before).abs();
            let tolerance = k_before * Decimal::new(1, 10); // 0.0000000001
            prop_assert!(diff < tolerance);
        }
    }
}

// Property: Optimal arbitrage amount should be non-negative
proptest! {
    #[test]
    fn optimal_arbitrage_non_negative(
        reserve_a_in in 100u64..100000u64,
        reserve_a_out in 100u64..100000u64,
        reserve_b_in in 100u64..100000u64,
        reserve_b_out in 100u64..100000u64,
        fee_bps in 1u32..100u32,
    ) {
        let pool_a = V2PoolState {
            reserve_in: Decimal::from(reserve_a_in),
            reserve_out: Decimal::from(reserve_a_out),
            fee_bps,
        };

        let pool_b = V2PoolState {
            reserve_in: Decimal::from(reserve_b_in),
            reserve_out: Decimal::from(reserve_b_out),
            fee_bps,
        };

        if let Ok(optimal) = V2Math::calculate_optimal_arbitrage_amount(&pool_a, &pool_b) {
            prop_assert!(optimal >= Decimal::ZERO);
        }
    }
}

// Property: Price impact should increase with trade size
proptest! {
    #[test]
    fn price_impact_monotonic(
        reserve_in in 10000u64..1000000u64,
        reserve_out in 10000u64..1000000u64,
    ) {
        let reserve_in_dec = Decimal::from(reserve_in);
        let reserve_out_dec = Decimal::from(reserve_out);

        let small_trade = Decimal::from(10u64);
        let large_trade = Decimal::from(1000u64);

        if let (Ok(small_impact), Ok(large_impact)) = (
            V2Math::calculate_price_impact(small_trade, reserve_in_dec, reserve_out_dec),
            V2Math::calculate_price_impact(large_trade, reserve_in_dec, reserve_out_dec),
        ) {
            prop_assert!(large_impact >= small_impact);
        }
    }
}

// Property: Round-trip trades should result in loss due to fees
proptest! {
    #[test]
    fn round_trip_loss(
        amount in 100u64..10000u64,
        reserve_a in 10000u64..1000000u64,
        reserve_b in 10000u64..1000000u64,
        fee_bps in 1u32..100u32,
    ) {
        let amount_dec = Decimal::from(amount);
        let reserve_a_dec = Decimal::from(reserve_a);
        let reserve_b_dec = Decimal::from(reserve_b);

        // Trade A -> B
        if let Ok(amount_b) = V2Math::calculate_output_amount(
            amount_dec,
            reserve_a_dec,
            reserve_b_dec,
            fee_bps,
        ) {
            // Update reserves
            let new_reserve_a = reserve_a_dec + amount_dec;
            let new_reserve_b = reserve_b_dec - amount_b;

            // Trade B -> A (reverse)
            if let Ok(amount_a_final) = V2Math::calculate_output_amount(
                amount_b,
                new_reserve_b,
                new_reserve_a,
                fee_bps,
            ) {
                // Should get back less than we started with
                prop_assert!(amount_a_final < amount_dec);
            }
        }
    }
}

// Property: Slippage should be bounded by theoretical maximum
proptest! {
    #[test]
    fn slippage_bounded(
        amount_in in 1u64..100000u64,
        reserve_in in 10000u64..10000000u64,
        reserve_out in 10000u64..10000000u64,
        fee_bps in 1u32..1000u32,
    ) {
        let amount_in_dec = Decimal::from(amount_in);
        let reserve_in_dec = Decimal::from(reserve_in);
        let reserve_out_dec = Decimal::from(reserve_out);

        if let Ok(slippage) = V2Math::calculate_slippage(
            amount_in_dec,
            reserve_in_dec,
            reserve_out_dec,
            fee_bps,
        ) {
            // Slippage should be between 0 and 100%
            prop_assert!(slippage >= Decimal::ZERO);
            prop_assert!(slippage <= Decimal::from(100));

            // For small trades relative to pool, slippage should be small
            let trade_ratio = amount_in_dec / reserve_in_dec;
            if trade_ratio < Decimal::new(1, 2) { // Less than 0.01 (1%)
                prop_assert!(slippage < Decimal::from(2)); // Less than 2%
            }
        }
    }
}
