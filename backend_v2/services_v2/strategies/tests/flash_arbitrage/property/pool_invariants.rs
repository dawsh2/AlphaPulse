//! Property-based tests for pool invariants

use alphapulse_strategies::flash_arbitrage::{
    math::V3Math,
    pool_state::{PoolState, PoolStateManager},
};
use proptest::prelude::*;
use protocol_v2::instrument_id::{PoolInstrumentId, VenueId};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;

// Property: Pool manager should maintain unique pools by hash
proptest! {
    #[test]
    fn pool_uniqueness_by_hash(
        token_a in 1u64..1000u64,
        token_b in 1u64..1000u64,
        venue_id in 1u16..10u16,
    ) {
        let manager = PoolStateManager::new();

        let pool_id = PoolInstrumentId {
            tokens: vec![token_a, token_b],
            venue_id,
            pool_type: 2,
        };

        // Add pool multiple times with different states
        for i in 0..5 {
            let state = PoolState::V2 {
                pool_id: pool_id.clone(),
                reserves: (dec!(1000) + Decimal::from(i), dec!(2000)),
                fee_tier: 30,
                last_update_ns: 1000000 + i as u64,
            };

            manager.update_pool(state).unwrap();
        }

        // Should only have one pool
        prop_assert_eq!(manager.stats().total_pools, 1);

        // Should have latest update
        let pool = manager.get_pool_by_id(&pool_id).unwrap();
        prop_assert_eq!(pool.last_update_ns(), 1000004);
    }
}

// Property: Token indexing should be bidirectional
proptest! {
    #[test]
    fn token_index_bidirectional(
        tokens in prop::collection::vec(1u64..100u64, 2..4),
        venue_id in 1u16..10u16,
    ) {
        let manager = PoolStateManager::new();

        let pool_id = PoolInstrumentId {
            tokens: tokens.clone(),
            venue_id,
            pool_type: 2,
        };

        let state = PoolState::V2 {
            pool_id: pool_id.clone(),
            reserves: (dec!(1000), dec!(2000)),
            fee_tier: 30,
            last_update_ns: 1000000,
        };

        manager.update_pool(state).unwrap();

        // Every token in the pool should find the pool
        for token in &tokens {
            let pools = manager.find_pools_with_token(*token);
            prop_assert!(pools.iter().any(|p| {
                p.pool_id().fast_hash() == pool_id.fast_hash()
            }));
        }
    }
}

// Property: V3 tick should be bounded
proptest! {
    #[test]
    fn v3_tick_bounds(
        liquidity in 1u128..u128::MAX/2,
        sqrt_price in V3Math::MIN_SQRT_RATIO..V3Math::MAX_SQRT_RATIO,
        amount_in in 1u128..1000000u128,
    ) {
        let pool = alphapulse_strategies::flash_arbitrage::math::V3PoolState {
            liquidity,
            sqrt_price_x96: sqrt_price,
            current_tick: 0,
            fee_pips: 3000,
        };

        if let Ok((_, _, new_tick)) = V3Math::calculate_output_amount(
            amount_in,
            &pool,
            true,
        ) {
            prop_assert!(new_tick >= V3Math::MIN_TICK);
            prop_assert!(new_tick <= V3Math::MAX_TICK);
        }
    }
}

// Property: Arbitrage pairs should be symmetric
proptest! {
    #[test]
    fn arbitrage_pairs_symmetric(
        token_a in 1u64..100u64,
        token_b in 1u64..100u64,
        venue_1 in 1u16..5u16,
        venue_2 in 6u16..10u16,
    ) {
        prop_assume!(token_a != token_b);
        prop_assume!(venue_1 != venue_2);

        let manager = Arc::new(PoolStateManager::new());

        // Add two pools with same tokens
        let pool_1 = PoolInstrumentId {
            tokens: vec![token_a, token_b],
            venue_id: venue_1,
            pool_type: 2,
        };

        let pool_2 = PoolInstrumentId {
            tokens: vec![token_a, token_b],
            venue_id: venue_2,
            pool_type: 2,
        };

        manager.update_pool(PoolState::V2 {
            pool_id: pool_1.clone(),
            reserves: (dec!(1000), dec!(2000)),
            fee_tier: 30,
            last_update_ns: 1000000,
        }).unwrap();

        manager.update_pool(PoolState::V2 {
            pool_id: pool_2.clone(),
            reserves: (dec!(1100), dec!(1900)),
            fee_tier: 30,
            last_update_ns: 1000001,
        }).unwrap();

        // Find pairs from pool_1's perspective
        let pairs_1 = manager.find_arbitrage_pairs(&pool_1);

        // Find pairs from pool_2's perspective
        let pairs_2 = manager.find_arbitrage_pairs(&pool_2);

        // Both should find each other
        prop_assert_eq!(pairs_1.len(), 1);
        prop_assert_eq!(pairs_2.len(), 1);
        prop_assert_eq!(pairs_1[0].pool_b, pool_2.fast_hash());
        prop_assert_eq!(pairs_2[0].pool_b, pool_1.fast_hash());
    }
}

// Property: Stale cleanup should preserve fresh pools
proptest! {
    #[test]
    fn stale_cleanup_preserves_fresh(
        num_fresh in 1usize..10usize,
        num_stale in 1usize..10usize,
    ) {
        let manager = PoolStateManager::new();
        let current_time = 1700000000000000000u64;

        // Add fresh pools
        for i in 0..num_fresh {
            let pool_id = PoolInstrumentId {
                tokens: vec![i as u64, (i + 1) as u64],
                venue_id: 1,
                pool_type: 2,
            };

            manager.update_pool(PoolState::V2 {
                pool_id,
                reserves: (dec!(1000), dec!(2000)),
                fee_tier: 30,
                last_update_ns: current_time,
            }).unwrap();
        }

        // Add stale pools
        for i in 0..num_stale {
            let pool_id = PoolInstrumentId {
                tokens: vec![(100 + i) as u64, (101 + i) as u64],
                venue_id: 2,
                pool_type: 2,
            };

            manager.update_pool(PoolState::V2 {
                pool_id,
                reserves: (dec!(1000), dec!(2000)),
                fee_tier: 30,
                last_update_ns: current_time - 7200_000_000_000, // 2 hours old
            }).unwrap();
        }

        // Clean up pools older than 1 hour
        let removed = manager.cleanup_stale_pools(3600_000_000_000);

        prop_assert_eq!(removed, num_stale);
        prop_assert_eq!(manager.stats().total_pools, num_fresh);
    }
}
