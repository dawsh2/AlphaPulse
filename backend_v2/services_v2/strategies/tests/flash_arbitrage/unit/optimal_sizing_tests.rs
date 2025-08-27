//! Unit tests for optimal position sizing

use alphapulse_strategies::flash_arbitrage::math::{
    optimal_size::{OptimalPosition, OptimalSizeCalculator, SizingConfig},
    V2PoolState, V3PoolState,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[test]
fn test_v2_optimal_sizing_with_profit() {
    let config = SizingConfig {
        min_profit_usd: dec!(1),
        max_position_pct: dec!(0.10), // 10% of pool
        gas_cost_usd: dec!(5),
        slippage_tolerance_bps: 100, // 1%
    };

    let calculator = OptimalSizeCalculator::new(config);

    // Create profitable arbitrage scenario
    let pool_a = V2PoolState {
        reserve_in: dec!(1000),
        reserve_out: dec!(2000000), // 1 ETH = 2000 USDC
        fee_bps: 30,
    };

    let pool_b = V2PoolState {
        reserve_in: dec!(1950000), // Flipped: USDC in, ETH out
        reserve_out: dec!(1000),   // 1 ETH = 1950 USDC (cheaper)
        fee_bps: 30,
    };

    let position = calculator
        .calculate_v2_arbitrage_size(
            &pool_a,
            &pool_b,
            dec!(2000), // ETH price $2000
        )
        .unwrap();

    if position.is_profitable {
        // Should have positive profit after gas
        assert!(position.expected_profit_usd > dec!(0));
        assert!(position.expected_profit_usd > position.gas_cost_usd);

        // Should respect position limits
        assert!(position.amount_in <= pool_a.reserve_in * dec!(0.10));

        // Slippage should be reasonable
        assert!(position.total_slippage_bps <= 100);
    }
}

#[test]
fn test_position_limit_enforcement() {
    let config = SizingConfig {
        min_profit_usd: dec!(0.01),
        max_position_pct: dec!(0.01), // Only 1% of pool
        gas_cost_usd: dec!(0),
        slippage_tolerance_bps: 1000,
    };

    let calculator = OptimalSizeCalculator::new(config);

    let pool_a = V2PoolState {
        reserve_in: dec!(10000), // Large pool
        reserve_out: dec!(20000000),
        fee_bps: 30,
    };

    let pool_b = V2PoolState {
        reserve_in: dec!(19000000),
        reserve_out: dec!(10000),
        fee_bps: 30,
    };

    let position = calculator
        .calculate_v2_arbitrage_size(&pool_a, &pool_b, dec!(2000))
        .unwrap();

    if position.is_profitable {
        // Should not exceed 1% of pool
        assert!(position.amount_in <= dec!(100)); // 1% of 10000
    }
}

#[test]
fn test_gas_cost_impact() {
    // Test with very high gas cost
    let high_gas_config = SizingConfig {
        min_profit_usd: dec!(1),
        max_position_pct: dec!(0.10),
        gas_cost_usd: dec!(1000), // Very high gas
        slippage_tolerance_bps: 100,
    };

    let calculator = OptimalSizeCalculator::new(high_gas_config);

    let pool_a = V2PoolState {
        reserve_in: dec!(100), // Small pools
        reserve_out: dec!(200000),
        fee_bps: 30,
    };

    let pool_b = V2PoolState {
        reserve_in: dec!(195000),
        reserve_out: dec!(100),
        fee_bps: 30,
    };

    let position = calculator
        .calculate_v2_arbitrage_size(&pool_a, &pool_b, dec!(2000))
        .unwrap();

    // With high gas and small pools, should not be profitable
    assert!(!position.is_profitable);
}

#[test]
fn test_slippage_tolerance() {
    let config = SizingConfig {
        min_profit_usd: dec!(0),
        max_position_pct: dec!(0.50), // Allow large trades
        gas_cost_usd: dec!(0),
        slippage_tolerance_bps: 50, // 0.5% max slippage
    };

    let calculator = OptimalSizeCalculator::new(config);

    // Small pools where large trades cause high slippage
    let pool_a = V2PoolState {
        reserve_in: dec!(10),
        reserve_out: dec!(20000),
        fee_bps: 30,
    };

    let pool_b = V2PoolState {
        reserve_in: dec!(19500),
        reserve_out: dec!(10),
        fee_bps: 30,
    };

    let position = calculator
        .calculate_v2_arbitrage_size(&pool_a, &pool_b, dec!(2000))
        .unwrap();

    // Should limit position to keep slippage under tolerance
    if position.is_profitable {
        assert!(position.total_slippage_bps <= 50);
        // Position should be small relative to pool
        assert!(position.amount_in < pool_a.reserve_in * dec!(0.1));
    }
}

#[test]
fn test_v3_sizing_simplified() {
    let config = SizingConfig::default();
    let calculator = OptimalSizeCalculator::new(config);

    let pool_a = V3PoolState {
        liquidity: 10_000_000_000_000,
        sqrt_price_x96: 79228162514264337593543950336,
        current_tick: 0,
        fee_pips: 500,
    };

    let pool_b = V3PoolState {
        liquidity: 10_000_000_000_000,
        sqrt_price_x96: 77228162514264337593543950336, // Different price
        current_tick: -100,
        fee_pips: 3000,
    };

    let position = calculator
        .calculate_v3_arbitrage_size(&pool_a, &pool_b, dec!(2000), true)
        .unwrap();

    // V3 sizing is simplified for now
    if position.is_profitable {
        assert!(position.expected_profit_usd > position.gas_cost_usd);
    }
}

#[test]
fn test_no_opportunity_scenarios() {
    let config = SizingConfig::default();
    let calculator = OptimalSizeCalculator::new(config);

    // Identical pools - no arbitrage
    let pool_a = V2PoolState {
        reserve_in: dec!(1000),
        reserve_out: dec!(2000000),
        fee_bps: 30,
    };

    let pool_b = V2PoolState {
        reserve_in: dec!(2000000),
        reserve_out: dec!(1000),
        fee_bps: 30,
    };

    let position = calculator
        .calculate_v2_arbitrage_size(&pool_a, &pool_b, dec!(2000))
        .unwrap();

    // Perfect equilibrium should not be profitable after fees
    assert!(!position.is_profitable);
}

#[test]
fn test_profit_margin_calculation() {
    let position = OptimalPosition {
        amount_in: dec!(100),
        expected_amount_out: dec!(110),
        expected_profit_usd: dec!(200), // $200 profit on $100 * $2000 = $200k trade
        total_slippage_bps: 50,
        gas_cost_usd: dec!(5),
        is_profitable: true,
    };

    let margin = position.profit_margin_pct();
    assert_eq!(margin, dec!(2)); // 200/100 = 2%

    // Test with no trade
    let no_trade = OptimalPosition {
        amount_in: dec!(0),
        expected_amount_out: dec!(0),
        expected_profit_usd: dec!(0),
        total_slippage_bps: 0,
        gas_cost_usd: dec!(0),
        is_profitable: false,
    };

    assert_eq!(no_trade.profit_margin_pct(), dec!(0));
}

#[test]
fn test_cross_protocol_placeholder() {
    let config = SizingConfig::default();
    let calculator = OptimalSizeCalculator::new(config);

    let v2_pool = V2PoolState {
        reserve_in: dec!(1000),
        reserve_out: dec!(2000000),
        fee_bps: 30,
    };

    let v3_pool = V3PoolState {
        liquidity: 1_000_000_000_000,
        sqrt_price_x96: 79228162514264337593543950336,
        current_tick: 0,
        fee_pips: 500,
    };

    // V2 as source
    let position = calculator
        .calculate_cross_protocol_size(&v2_pool, &v3_pool, dec!(2000), true)
        .unwrap();

    // Currently returns no_opportunity (TODO)
    assert!(!position.is_profitable);

    // V3 as source
    let position = calculator
        .calculate_cross_protocol_size(&v2_pool, &v3_pool, dec!(2000), false)
        .unwrap();

    assert!(!position.is_profitable);
}
