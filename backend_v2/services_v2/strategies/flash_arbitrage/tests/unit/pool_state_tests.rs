//! Unit tests for pool state management

use alphapulse_flash_arbitrage::pool_state::{PoolState, PoolStateManager};
use alphapulse_protocol_v2::instrument_id::{PoolInstrumentId, VenueId};
use rust_decimal_macros::dec;
use std::sync::Arc;

#[test]
fn test_pool_manager_basic_operations() {
    let manager = PoolStateManager::new();

    // Create test pool
    let pool_id = PoolInstrumentId {
        tokens: vec![1, 2], // WETH, USDC
        venue_id: VenueId::Uniswap as u16,
        pool_type: 2, // V2
    };

    let state = PoolState::V2 {
        pool_id: pool_id.clone(),
        reserves: (dec!(1000), dec!(2000000)),
        fee_tier: 30,
        last_update_ns: 1000000,
    };

    // Add pool
    manager.update_pool(state.clone()).unwrap();

    // Retrieve by ID
    let retrieved = manager.get_pool_by_id(&pool_id).unwrap();
    assert_eq!(retrieved.pool_id().fast_hash(), pool_id.fast_hash());

    // Retrieve by hash
    let hash = pool_id.fast_hash();
    let retrieved_by_hash = manager.get_pool(hash).unwrap();
    assert_eq!(retrieved_by_hash.pool_id().fast_hash(), hash);

    // Check stats
    let stats = manager.stats();
    assert_eq!(stats.total_pools, 1);
    assert_eq!(stats.v2_pools, 1);
    assert_eq!(stats.v3_pools, 0);
}

#[test]
fn test_token_indexing() {
    let manager = PoolStateManager::new();

    // Add multiple pools with overlapping tokens
    let pools = vec![
        (vec![1, 2], VenueId::Uniswap),   // WETH/USDC on Uniswap
        (vec![1, 2], VenueId::Sushiswap), // WETH/USDC on Sushiswap
        (vec![1, 3], VenueId::Uniswap),   // WETH/DAI on Uniswap
        (vec![2, 3], VenueId::Uniswap),   // USDC/DAI on Uniswap
    ];

    for (tokens, venue) in pools {
        let pool_id = PoolInstrumentId {
            tokens,
            venue_id: venue as u16,
            pool_type: 2,
        };

        let state = PoolState::V2 {
            pool_id,
            reserves: (dec!(1000), dec!(1000)),
            fee_tier: 30,
            last_update_ns: 1000000,
        };

        manager.update_pool(state).unwrap();
    }

    // Find all pools with WETH (token 1)
    let weth_pools = manager.find_pools_with_token(1);
    assert_eq!(weth_pools.len(), 3); // 3 pools contain WETH

    // Find all pools with USDC (token 2)
    let usdc_pools = manager.find_pools_with_token(2);
    assert_eq!(usdc_pools.len(), 3); // 3 pools contain USDC

    // Find all pools with DAI (token 3)
    let dai_pools = manager.find_pools_with_token(3);
    assert_eq!(dai_pools.len(), 2); // 2 pools contain DAI
}

#[test]
fn test_pair_indexing() {
    let manager = PoolStateManager::new();

    // Add pools for same pair on different venues
    let venues = vec![VenueId::Uniswap, VenueId::Sushiswap, VenueId::QuickSwap];

    for venue in venues {
        let pool_id = PoolInstrumentId {
            tokens: vec![1, 2], // Same pair
            venue_id: venue as u16,
            pool_type: 2,
        };

        let state = PoolState::V2 {
            pool_id,
            reserves: (dec!(1000), dec!(2000)),
            fee_tier: 30,
            last_update_ns: 1000000,
        };

        manager.update_pool(state).unwrap();
    }

    // Find all pools for WETH/USDC pair
    let pair_pools = manager.find_pools_for_pair(1, 2);
    assert_eq!(pair_pools.len(), 3);

    // Order shouldn't matter
    let pair_pools_reversed = manager.find_pools_for_pair(2, 1);
    assert_eq!(pair_pools_reversed.len(), 3);
}

#[test]
fn test_arbitrage_pair_detection() {
    let manager = PoolStateManager::new();

    // Add two pools with same tokens but different prices
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

    manager
        .update_pool(PoolState::V2 {
            pool_id: pool_a.clone(),
            reserves: (dec!(1000), dec!(2000000)), // 1 ETH = 2000 USDC
            fee_tier: 30,
            last_update_ns: 1000000,
        })
        .unwrap();

    manager
        .update_pool(PoolState::V2 {
            pool_id: pool_b.clone(),
            reserves: (dec!(1100), dec!(2090000)), // 1 ETH = 1900 USDC (arbitrage!)
            fee_tier: 30,
            last_update_ns: 1000001,
        })
        .unwrap();

    // Find arbitrage pairs
    let pairs = manager.find_arbitrage_pairs(&pool_a);
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].pool_b, pool_b.fast_hash());
    assert_eq!(pairs[0].shared_tokens, vec![1, 2]);
}

#[test]
fn test_v3_pool_state() {
    let manager = PoolStateManager::new();

    let pool_id = PoolInstrumentId {
        tokens: vec![1, 2],
        venue_id: VenueId::UniswapV3 as u16,
        pool_type: 3, // V3
    };

    let state = PoolState::V3 {
        pool_id: pool_id.clone(),
        liquidity: 1_000_000_000_000,
        sqrt_price_x96: 79228162514264337593543950336,
        current_tick: 0,
        fee_tier: 500, // 0.05%
        last_update_ns: 1000000,
    };

    manager.update_pool(state).unwrap();

    // Retrieve and verify it's V3
    let retrieved = manager.get_pool_by_id(&pool_id).unwrap();
    assert!(retrieved.as_v3_pool().is_some());
    assert!(retrieved.as_v2_pool().is_none());

    // Check stats
    let stats = manager.stats();
    assert_eq!(stats.v3_pools, 1);
    assert_eq!(stats.v2_pools, 0);
}

#[test]
fn test_stale_pool_cleanup() {
    let manager = PoolStateManager::new();

    // Add pools with different timestamps
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    // Fresh pool
    let fresh_pool = PoolInstrumentId {
        tokens: vec![1, 2],
        venue_id: VenueId::Uniswap as u16,
        pool_type: 2,
    };

    manager
        .update_pool(PoolState::V2 {
            pool_id: fresh_pool.clone(),
            reserves: (dec!(1000), dec!(2000)),
            fee_tier: 30,
            last_update_ns: current_time,
        })
        .unwrap();

    // Stale pool (1 hour old)
    let stale_pool = PoolInstrumentId {
        tokens: vec![3, 4],
        venue_id: VenueId::Sushiswap as u16,
        pool_type: 2,
    };

    manager
        .update_pool(PoolState::V2 {
            pool_id: stale_pool.clone(),
            reserves: (dec!(500), dec!(1000)),
            fee_tier: 30,
            last_update_ns: current_time - 3_600_000_000_000, // 1 hour ago
        })
        .unwrap();

    // Should have 2 pools
    assert_eq!(manager.stats().total_pools, 2);

    // Clean up pools older than 30 minutes
    let removed = manager.cleanup_stale_pools(1_800_000_000_000);
    assert_eq!(removed, 1);

    // Should only have fresh pool
    assert_eq!(manager.stats().total_pools, 1);
    assert!(manager.get_pool_by_id(&fresh_pool).is_some());
    assert!(manager.get_pool_by_id(&stale_pool).is_none());
}

#[test]
fn test_concurrent_updates() {
    use std::sync::Arc;
    use std::thread;

    let manager = Arc::new(PoolStateManager::new());
    let mut handles = vec![];

    // Spawn multiple threads updating different pools
    for i in 0..10 {
        let manager_clone = manager.clone();
        let handle = thread::spawn(move || {
            let pool_id = PoolInstrumentId {
                tokens: vec![i, i + 1],
                venue_id: VenueId::Uniswap as u16,
                pool_type: 2,
            };

            for j in 0..100 {
                let state = PoolState::V2 {
                    pool_id: pool_id.clone(),
                    reserves: (dec!(1000) + dec!(j), dec!(2000) + dec!(j)),
                    fee_tier: 30,
                    last_update_ns: 1000000 + j as u64,
                };

                manager_clone.update_pool(state).unwrap();
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Should have 10 pools
    assert_eq!(manager.stats().total_pools, 10);

    // Each pool should have latest update (j=99)
    for i in 0..10 {
        let pool_id = PoolInstrumentId {
            tokens: vec![i, i + 1],
            venue_id: VenueId::Uniswap as u16,
            pool_type: 2,
        };

        let pool = manager.get_pool_by_id(&pool_id).unwrap();
        assert_eq!(pool.last_update_ns(), 1000099);
    }
}
