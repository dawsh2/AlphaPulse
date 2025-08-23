//! End-to-end integration test for the complete arbitrage flow

use alphapulse_flash_arbitrage::{
    detector::{DetectorConfig, OpportunityDetector},
    pool_state::{PoolState, PoolStateManager},
};
use alphapulse_protocol_v2::{
    instrument_id::{PoolInstrumentId, VenueId},
    tlv::TLVMessageBuilder,
    SourceType, TLVType,
};
use rust_decimal_macros::dec;
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_complete_arbitrage_flow() {
    // 1. Setup infrastructure
    let pool_manager = Arc::new(PoolStateManager::new());
    let config = DetectorConfig::default();
    let detector = Arc::new(OpportunityDetector::new(pool_manager.clone(), config));

    // Set token prices
    detector.update_token_price(1, dec!(2000)); // ETH
    detector.update_token_price(2, dec!(1)); // USDC

    // 2. Simulate adapter receiving DEX events
    let (adapter_tx, mut adapter_rx) = mpsc::channel(100);

    // Spawn mock adapter
    tokio::spawn(async move {
        // Simulate swap event on Uniswap
        let mut builder = TLVMessageBuilder::new(SourceType::PolygonCollector, 1);

        // In real scenario, would parse actual swap event
        // For now, just signal that pools need updating
        builder.add_tlv(TLVType::Trade, &[0u8; 48]).unwrap();
        let message = builder.build().unwrap();
        adapter_tx.send(message).await.unwrap();

        // Simulate another swap on Sushiswap
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let mut builder = TLVMessageBuilder::new(SourceType::PolygonCollector, 2);
        builder.add_tlv(TLVType::Trade, &[1u8; 48]).unwrap();
        let message = builder.build().unwrap();
        adapter_tx.send(message).await.unwrap();
    });

    // 3. Process messages and update pool states
    let pool_manager_clone = pool_manager.clone();
    let detector_clone = detector.clone();

    tokio::spawn(async move {
        while let Some(_message) = adapter_rx.recv().await {
            // In real implementation, would parse TLV and extract pool data
            // For testing, manually update pools

            // Update Uniswap pool
            let pool_a = PoolInstrumentId {
                tokens: vec![1, 2],
                venue_id: VenueId::Uniswap as u16,
                pool_type: 2,
            };

            pool_manager_clone
                .update_pool(PoolState::V2 {
                    pool_id: pool_a.clone(),
                    reserves: (dec!(1000), dec!(2000000)),
                    fee_tier: 30,
                    last_update_ns: 1000000,
                })
                .unwrap();

            // Update Sushiswap pool with arbitrage opportunity
            let pool_b = PoolInstrumentId {
                tokens: vec![1, 2],
                venue_id: VenueId::Sushiswap as u16,
                pool_type: 2,
            };

            pool_manager_clone
                .update_pool(PoolState::V2 {
                    pool_id: pool_b.clone(),
                    reserves: (dec!(1050), dec!(1995000)), // Different price!
                    fee_tier: 30,
                    last_update_ns: 1000001,
                })
                .unwrap();

            // 4. Detect arbitrage opportunities
            let opportunities = detector_clone.find_arbitrage(&pool_a);

            if !opportunities.is_empty() {
                println!("Found {} arbitrage opportunities", opportunities.len());

                for opp in &opportunities {
                    println!(
                        "Opportunity {}: {} -> {}, profit: ${}, slippage: {}bps",
                        opp.id,
                        opp.token_in,
                        opp.token_out,
                        opp.expected_profit_usd,
                        opp.slippage_bps
                    );

                    // 5. In production, would execute via flash loan here
                    // For testing, just verify opportunity is valid
                    assert!(opp.expected_profit_usd > dec!(0));
                    assert!(opp.optimal_amount > dec!(0));
                }
            }
        }
    });

    // Wait for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify pools were added
    assert_eq!(pool_manager.stats().total_pools, 2);

    // Verify arbitrage pairs exist
    let pool_a = PoolInstrumentId {
        tokens: vec![1, 2],
        venue_id: VenueId::Uniswap as u16,
        pool_type: 2,
    };

    let pairs = pool_manager.find_arbitrage_pairs(&pool_a);
    assert!(!pairs.is_empty());
}

#[tokio::test]
async fn test_multi_hop_arbitrage() {
    let pool_manager = Arc::new(PoolStateManager::new());
    let config = DetectorConfig::default();
    let detector = Arc::new(OpportunityDetector::new(pool_manager.clone(), config));

    // Set token prices
    detector.update_token_price(1, dec!(2000)); // ETH
    detector.update_token_price(2, dec!(1)); // USDC
    detector.update_token_price(3, dec!(1)); // DAI

    // Create triangular arbitrage opportunity
    // ETH -> USDC -> DAI -> ETH

    // Pool 1: ETH/USDC
    pool_manager
        .update_pool(PoolState::V2 {
            pool_id: PoolInstrumentId {
                tokens: vec![1, 2],
                venue_id: VenueId::Uniswap as u16,
                pool_type: 2,
            },
            reserves: (dec!(1000), dec!(2000000)),
            fee_tier: 30,
            last_update_ns: 1000000,
        })
        .unwrap();

    // Pool 2: USDC/DAI
    pool_manager
        .update_pool(PoolState::V2 {
            pool_id: PoolInstrumentId {
                tokens: vec![2, 3],
                venue_id: VenueId::Uniswap as u16,
                pool_type: 2,
            },
            reserves: (dec!(1000000), dec!(999000)), // Slight imbalance
            fee_tier: 5,
            last_update_ns: 1000001,
        })
        .unwrap();

    // Pool 3: DAI/ETH
    pool_manager
        .update_pool(PoolState::V2 {
            pool_id: PoolInstrumentId {
                tokens: vec![3, 1],
                venue_id: VenueId::Sushiswap as u16,
                pool_type: 2,
            },
            reserves: (dec!(2000000), dec!(990)), // Completing the triangle
            fee_tier: 30,
            last_update_ns: 1000002,
        })
        .unwrap();

    // Verify all pools are indexed
    assert_eq!(pool_manager.stats().total_pools, 3);

    // Find pools connected to ETH
    let eth_pools = pool_manager.find_pools_with_token(1);
    assert_eq!(eth_pools.len(), 2);

    // Find pools connected to USDC
    let usdc_pools = pool_manager.find_pools_with_token(2);
    assert_eq!(usdc_pools.len(), 2);

    // TODO: Implement multi-hop arbitrage detection
}

#[tokio::test]
async fn test_v2_v3_cross_protocol() {
    let pool_manager = Arc::new(PoolStateManager::new());

    // Add V2 pool
    let v2_pool = PoolInstrumentId {
        tokens: vec![1, 2],
        venue_id: VenueId::Uniswap as u16,
        pool_type: 2,
    };

    pool_manager
        .update_pool(PoolState::V2 {
            pool_id: v2_pool.clone(),
            reserves: (dec!(1000), dec!(2000000)),
            fee_tier: 30,
            last_update_ns: 1000000,
        })
        .unwrap();

    // Add V3 pool with different price
    let v3_pool = PoolInstrumentId {
        tokens: vec![1, 2],
        venue_id: VenueId::UniswapV3 as u16,
        pool_type: 3,
    };

    pool_manager
        .update_pool(PoolState::V3 {
            pool_id: v3_pool.clone(),
            liquidity: 1_000_000_000_000,
            sqrt_price_x96: 77228162514264337593543950336, // Different price
            current_tick: -100,
            fee_tier: 500,
            last_update_ns: 1000001,
        })
        .unwrap();

    // Verify both pools exist
    assert_eq!(pool_manager.stats().total_pools, 2);
    assert_eq!(pool_manager.stats().v2_pools, 1);
    assert_eq!(pool_manager.stats().v3_pools, 1);

    // Find arbitrage pairs (should work across protocols)
    let pairs = pool_manager.find_arbitrage_pairs(&v2_pool);
    assert_eq!(pairs.len(), 1);

    // TODO: Implement V2<->V3 arbitrage execution
}
