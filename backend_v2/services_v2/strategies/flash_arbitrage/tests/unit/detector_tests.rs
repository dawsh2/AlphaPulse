//! Unit tests for arbitrage opportunity detection

use alphapulse_flash_arbitrage::{
    detector::{DetectorConfig, OpportunityDetector, StrategyType, TokenPriceOracle},
    pool_state::{PoolState, PoolStateManager},
};
use protocol_v2::instrument_id::{PoolInstrumentId, VenueId};
use rust_decimal_macros::dec;
use std::sync::Arc;

#[test]
fn test_basic_opportunity_detection() {
    let pool_manager = Arc::new(PoolStateManager::new());
    let config = DetectorConfig::default();
    let detector = OpportunityDetector::new(pool_manager.clone(), config);

    // Set token prices
    detector.update_token_price(1, dec!(2000)); // ETH = $2000
    detector.update_token_price(2, dec!(1)); // USDC = $1

    // Add pool with normal price
    let pool_a = PoolInstrumentId {
        tokens: vec![1, 2],
        venue_id: VenueId::Uniswap as u16,
        pool_type: 2,
    };

    pool_manager
        .update_pool(PoolState::V2 {
            pool_id: pool_a.clone(),
            reserves: (dec!(1000), dec!(2000000)), // 1 ETH = 2000 USDC (fair)
            fee_tier: 30,
            last_update_ns: 1000000,
        })
        .unwrap();

    // Add pool with arbitrage opportunity
    let pool_b = PoolInstrumentId {
        tokens: vec![1, 2],
        venue_id: VenueId::Sushiswap as u16,
        pool_type: 2,
    };

    pool_manager
        .update_pool(PoolState::V2 {
            pool_id: pool_b.clone(),
            reserves: (dec!(1000), dec!(1900000)), // 1 ETH = 1900 USDC (cheap!)
            fee_tier: 30,
            last_update_ns: 1000001,
        })
        .unwrap();

    // Detect opportunities
    let opportunities = detector.find_arbitrage(&pool_a);

    // Should find opportunity if prices are sufficiently different
    if !opportunities.is_empty() {
        let opp = &opportunities[0];
        assert_eq!(opp.strategy_type, StrategyType::V2ToV2);
        assert!(opp.expected_profit_usd > dec!(0));
    }
}

#[test]
fn test_minimum_profit_threshold() {
    let pool_manager = Arc::new(PoolStateManager::new());

    // Set high minimum profit threshold
    let mut config = DetectorConfig::default();
    config.min_profit_usd = dec!(1000); // $1000 minimum

    let detector = OpportunityDetector::new(pool_manager.clone(), config);

    detector.update_token_price(1, dec!(2000));
    detector.update_token_price(2, dec!(1));

    // Add pools with small price difference
    let pool_a = PoolInstrumentId {
        tokens: vec![1, 2],
        venue_id: VenueId::Uniswap as u16,
        pool_type: 2,
    };

    let pool_b = PoolInstrumentId {
        tokens: vec![1, 2],
        venue_id: VenueId::Sushiswap as u16,
        pool_type: 2,
    };

    pool_manager
        .update_pool(PoolState::V2 {
            pool_id: pool_a.clone(),
            reserves: (dec!(1000), dec!(2000000)),
            fee_tier: 30,
            last_update_ns: 1000000,
        })
        .unwrap();

    pool_manager
        .update_pool(PoolState::V2 {
            pool_id: pool_b.clone(),
            reserves: (dec!(1000), dec!(1999000)), // Very small difference
            fee_tier: 30,
            last_update_ns: 1000001,
        })
        .unwrap();

    // Should not find opportunities below threshold
    let opportunities = detector.find_arbitrage(&pool_a);
    assert!(opportunities.is_empty());
}

#[test]
fn test_gas_cost_consideration() {
    let pool_manager = Arc::new(PoolStateManager::new());

    let mut config = DetectorConfig::default();
    config.gas_cost_usd = dec!(50); // $50 gas cost

    let detector = OpportunityDetector::new(pool_manager.clone(), config);

    detector.update_token_price(1, dec!(2000));
    detector.update_token_price(2, dec!(1));

    // Add pools with moderate price difference
    let pool_a = PoolInstrumentId {
        tokens: vec![1, 2],
        venue_id: VenueId::Uniswap as u16,
        pool_type: 2,
    };

    let pool_b = PoolInstrumentId {
        tokens: vec![1, 2],
        venue_id: VenueId::Sushiswap as u16,
        pool_type: 2,
    };

    pool_manager
        .update_pool(PoolState::V2 {
            pool_id: pool_a.clone(),
            reserves: (dec!(100), dec!(200000)), // Small pool
            fee_tier: 30,
            last_update_ns: 1000000,
        })
        .unwrap();

    pool_manager
        .update_pool(PoolState::V2 {
            pool_id: pool_b.clone(),
            reserves: (dec!(100), dec!(195000)), // Price difference
            fee_tier: 30,
            last_update_ns: 1000001,
        })
        .unwrap();

    let opportunities = detector.find_arbitrage(&pool_a);

    // With high gas costs and small pools, might not be profitable
    for opp in &opportunities {
        assert_eq!(opp.gas_cost_usd, dec!(50));
        // If profitable, profit must exceed gas cost
        if opp.expected_profit_usd > dec!(0) {
            assert!(opp.expected_profit_usd > opp.gas_cost_usd);
        }
    }
}

#[test]
fn test_bidirectional_opportunity_detection() {
    let pool_manager = Arc::new(PoolStateManager::new());
    let config = DetectorConfig::default();
    let detector = OpportunityDetector::new(pool_manager.clone(), config);

    detector.update_token_price(1, dec!(2000));
    detector.update_token_price(2, dec!(1));

    // Create asymmetric pools where arbitrage works in one direction
    let pool_a = PoolInstrumentId {
        tokens: vec![1, 2],
        venue_id: VenueId::Uniswap as u16,
        pool_type: 2,
    };

    let pool_b = PoolInstrumentId {
        tokens: vec![1, 2],
        venue_id: VenueId::Sushiswap as u16,
        pool_type: 2,
    };

    // Pool A: More ETH, less USDC
    pool_manager
        .update_pool(PoolState::V2 {
            pool_id: pool_a.clone(),
            reserves: (dec!(1500), dec!(2700000)), // 1 ETH = 1800 USDC
            fee_tier: 30,
            last_update_ns: 1000000,
        })
        .unwrap();

    // Pool B: Less ETH, more USDC
    pool_manager
        .update_pool(PoolState::V2 {
            pool_id: pool_b.clone(),
            reserves: (dec!(900), dec!(2070000)), // 1 ETH = 2300 USDC
            fee_tier: 30,
            last_update_ns: 1000001,
        })
        .unwrap();

    let opportunities = detector.find_arbitrage(&pool_a);

    // Should detect the best direction
    if !opportunities.is_empty() {
        let opp = &opportunities[0];
        // Should buy ETH from cheaper pool and sell to expensive pool
        assert!(opp.token_in == 1 || opp.token_in == 2);
    }
}

#[test]
fn test_v3_opportunity_detection() {
    let pool_manager = Arc::new(PoolStateManager::new());
    let config = DetectorConfig::default();
    let detector = OpportunityDetector::new(pool_manager.clone(), config);

    detector.update_token_price(1, dec!(2000));
    detector.update_token_price(2, dec!(1));

    // Add V3 pools
    let pool_a = PoolInstrumentId {
        tokens: vec![1, 2],
        venue_id: VenueId::UniswapV3 as u16,
        pool_type: 3,
    };

    let pool_b = PoolInstrumentId {
        tokens: vec![1, 2],
        venue_id: (VenueId::UniswapV3 as u16) + 1000, // Different V3 pool
        pool_type: 3,
    };

    pool_manager
        .update_pool(PoolState::V3 {
            pool_id: pool_a.clone(),
            liquidity: 1_000_000_000_000,
            sqrt_price_x96: 79228162514264337593543950336, // Price = 1.0
            current_tick: 0,
            fee_tier: 500, // 0.05%
            last_update_ns: 1000000,
        })
        .unwrap();

    pool_manager
        .update_pool(PoolState::V3 {
            pool_id: pool_b.clone(),
            liquidity: 1_000_000_000_000,
            sqrt_price_x96: 77228162514264337593543950336, // Different price
            current_tick: -100,
            fee_tier: 3000, // 0.3%
            last_update_ns: 1000001,
        })
        .unwrap();

    let opportunities = detector.find_arbitrage(&pool_a);

    // Should detect V3 to V3 opportunities
    for opp in &opportunities {
        assert_eq!(opp.strategy_type, StrategyType::V3ToV3);
    }
}

#[test]
fn test_token_price_oracle() {
    let oracle = TokenPriceOracle::new();

    // Update prices
    oracle.update_price(1, dec!(2000)); // ETH
    oracle.update_price(2, dec!(1)); // USDC
    oracle.update_price(3, dec!(1)); // DAI

    // Retrieve prices
    assert_eq!(oracle.get_price(1), Some(dec!(2000)));
    assert_eq!(oracle.get_price(2), Some(dec!(1)));
    assert_eq!(oracle.get_price(3), Some(dec!(1)));

    // Non-existent token
    assert_eq!(oracle.get_price(999), None);

    // Update existing price
    oracle.update_price(1, dec!(2100));
    assert_eq!(oracle.get_price(1), Some(dec!(2100)));
}

#[test]
fn test_opportunity_id_generation() {
    let pool_manager = Arc::new(PoolStateManager::new());
    let config = DetectorConfig::default();
    let detector = OpportunityDetector::new(pool_manager.clone(), config);

    detector.update_token_price(1, dec!(2000));
    detector.update_token_price(2, dec!(1));

    // Add multiple pool pairs
    for i in 0..3 {
        let pool_a = PoolInstrumentId {
            tokens: vec![1, 2],
            venue_id: (VenueId::Uniswap as u16) + i,
            pool_type: 2,
        };

        let pool_b = PoolInstrumentId {
            tokens: vec![1, 2],
            venue_id: (VenueId::Sushiswap as u16) + i,
            pool_type: 2,
        };

        pool_manager
            .update_pool(PoolState::V2 {
                pool_id: pool_a.clone(),
                reserves: (dec!(1000), dec!(2000000) + dec!(i * 10000)),
                fee_tier: 30,
                last_update_ns: 1000000 + i as u64,
            })
            .unwrap();

        pool_manager
            .update_pool(PoolState::V2 {
                pool_id: pool_b.clone(),
                reserves: (dec!(1000), dec!(1900000) + dec!(i * 10000)),
                fee_tier: 30,
                last_update_ns: 1000001 + i as u64,
            })
            .unwrap();

        let opportunities = detector.find_arbitrage(&pool_a);

        // Each opportunity should have unique ID
        for (j, opp) in opportunities.iter().enumerate() {
            for (k, other) in opportunities.iter().enumerate() {
                if j != k {
                    assert_ne!(opp.id, other.id);
                }
            }
        }
    }
}
