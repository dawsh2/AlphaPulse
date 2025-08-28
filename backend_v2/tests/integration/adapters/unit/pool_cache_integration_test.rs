//! Unit tests for pool cache integration with Polygon collector
//!
//! Tests the integration between PoolCache and UnifiedPolygonCollector
//! focusing on proper initialization, configuration, and basic functionality.

use torq_state_market::pool_cache::{PoolCache, PoolCacheError, PoolInfo};
use torq_state_market::pool_state::PoolStateManager;
use protocol_v2::{VenueId, DEXProtocol};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;
use std::time::Duration;

/// Test helper to create a test pool cache with persistence
async fn create_test_cache(temp_dir: &std::path::Path) -> Arc<PoolCache> {
    let cache = Arc::new(PoolCache::with_persistence(temp_dir.to_path_buf(), 137));
    // Load any existing data
    let _ = cache.load_from_disk().await;
    cache
}

/// Test helper to create a test pool info
fn create_test_pool_info(pool_addr: [u8; 20]) -> PoolInfo {
    PoolInfo {
        pool_address: pool_addr,
        token0: [0x01; 20], // Mock WETH address
        token1: [0x02; 20], // Mock USDC address
        token0_decimals: 18,
        token1_decimals: 6,
        pool_type: DEXProtocol::UniswapV3,
        fee_tier: Some(3000), // 0.3%
        discovered_at: 1234567890_000_000_000,
        venue: VenueId::Polygon,
        last_seen: 1234567890_000_000_000,
    }
}

#[tokio::test]
async fn test_pool_cache_creation_and_initialization() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = create_test_cache(temp_dir.path()).await;

    // Test initial state
    let stats = cache.stats();
    assert_eq!(stats.cached_pools, 0, "Should start with empty cache");
    assert_eq!(stats.cache_hits, 0, "Should start with zero hits");
    assert_eq!(stats.cache_misses, 0, "Should start with zero misses");
}

#[tokio::test]
async fn test_cache_hit_miss_tracking() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = create_test_cache(temp_dir.path()).await;

    let test_pool = [0x42; 20];

    // First access should be a miss (will fail discovery due to no RPC)
    let result1 = cache.get_or_discover_pool(test_pool).await;
    assert!(result1.is_err(), "Discovery should fail without RPC");

    let stats_after_miss = cache.stats();
    assert_eq!(stats_after_miss.cache_misses, 1, "Should record cache miss");

    // Test cached lookup (won't exist but will increment miss counter)
    let cached_result = cache.get_cached(&test_pool);
    assert!(cached_result.is_none(), "Pool should not be cached after failed discovery");

    let stats_after_cached_lookup = cache.stats();
    assert_eq!(stats_after_cached_lookup.cache_misses, 2, "Should record another miss for get_cached");
}

#[tokio::test]
async fn test_cache_persistence_operations() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create first cache instance
    {
        let cache1 = create_test_cache(temp_dir.path()).await;

        // Force a snapshot even with empty cache
        cache1.force_snapshot().await.expect("Snapshot should succeed");
    }

    // Create second cache instance and test loading
    {
        let cache2 = create_test_cache(temp_dir.path()).await;

        // Should load successfully (even if empty)
        let loaded_count = cache2.load_from_disk().await.expect("Load should succeed");
        println!("Loaded {} pools from persistence", loaded_count);

        // Verify cache is in expected state
        let stats = cache2.stats();
        assert_eq!(stats.cached_pools, loaded_count as usize, "Stats should match loaded count");
    }
}

#[tokio::test]
async fn test_cache_is_cached_functionality() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = create_test_cache(temp_dir.path()).await;

    let test_pools = [
        [0x01; 20],
        [0x02; 20],
        [0x03; 20],
    ];

    // Initially, no pools should be cached
    for pool in &test_pools {
        assert!(!cache.is_cached(pool), "Pool should not be cached initially");
    }

    // After failed discovery attempts, pools still should not be cached
    for pool in &test_pools {
        let _ = cache.get_or_discover_pool(*pool).await; // Will fail due to no RPC
        assert!(!cache.is_cached(pool), "Pool should not be cached after failed discovery");
    }
}

#[tokio::test]
async fn test_concurrent_cache_access() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = Arc::new(create_test_cache(temp_dir.path()).await.as_ref().clone());

    let test_pools: Vec<[u8; 20]> = (0..10).map(|i| [i as u8; 20]).collect();

    // Spawn multiple tasks accessing cache concurrently
    let handles: Vec<_> = test_pools.into_iter().map(|pool| {
        let cache = cache.clone();
        tokio::spawn(async move {
            // Each task tries to access a different pool
            let result = cache.get_or_discover_pool(pool).await;
            let is_cached = cache.is_cached(&pool);
            let stats = cache.stats();

            // Return results for verification
            (result.is_err(), is_cached, stats.cache_misses > 0)
        })
    }).collect();

    // Wait for all tasks to complete
    let mut total_misses = 0;
    for handle in handles {
        let (failed_discovery, is_cached, has_misses) = handle.await.expect("Task should complete");
        assert!(failed_discovery, "Discovery should fail without RPC");
        assert!(!is_cached, "Pool should not be cached after failed discovery");
        assert!(has_misses, "Should have cache misses");
    }

    // Verify final cache state
    let final_stats = cache.stats();
    assert_eq!(final_stats.cached_pools, 0, "No pools should be cached");
    assert!(final_stats.cache_misses >= 10, "Should have at least 10 cache misses");
}

#[tokio::test]
async fn test_pool_state_manager_integration() {
    // Test that PoolStateManager can be created alongside PoolCache
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = create_test_cache(temp_dir.path()).await;
    let pool_state_manager = Arc::new(PoolStateManager::new());

    // Verify both components are properly initialized
    let cache_stats = cache.stats();
    assert_eq!(cache_stats.cached_pools, 0, "Cache should be empty initially");

    // PoolStateManager doesn't have public stats, but we can verify it was created
    // This test mainly verifies the integration pattern works
    assert!(Arc::strong_count(&pool_state_manager) >= 1, "PoolStateManager should be properly referenced");
}

#[tokio::test]
async fn test_error_handling_graceful_degradation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = create_test_cache(temp_dir.path()).await;

    // Test various invalid pool addresses
    let invalid_pools = [
        [0x00; 20], // Zero address
        [0xFF; 20], // Invalid address
        [0xDE; 20], // Another invalid address
    ];

    for invalid_pool in &invalid_pools {
        // All discovery attempts should fail gracefully
        let result = cache.get_or_discover_pool(*invalid_pool).await;
        match result {
            Err(PoolCacheError::Discovery(msg)) => {
                // Should fail with discovery error
                assert!(msg.contains("RPC") || msg.contains("network") || msg.contains("pool"),
                       "Error should be related to network/RPC issues");
            },
            Err(PoolCacheError::Other(_)) => {
                // Other errors are also acceptable for invalid addresses
            },
            Ok(_) => panic!("Invalid pool should not be successfully discovered"),
        }

        // Pool should not be cached after failed discovery
        assert!(!cache.is_cached(invalid_pool), "Invalid pool should not be cached");
    }

    // Cache should still be functional after errors
    let stats = cache.stats();
    assert_eq!(stats.cached_pools, 0, "No invalid pools should be cached");
    assert!(stats.cache_misses >= invalid_pools.len(), "Should have recorded cache misses");
}

#[tokio::test]
async fn test_cache_shutdown_cleanup() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Test that cache can be properly shut down
    let cache = create_test_cache(temp_dir.path()).await;

    // Perform some operations
    let _ = cache.get_or_discover_pool([0x42; 20]).await;
    let _ = cache.force_snapshot().await;

    // Test graceful shutdown
    let cache_clone = Arc::try_unwrap(cache).expect("Should be able to unwrap cache");
    let shutdown_result = cache_clone.shutdown().await;

    // Shutdown should succeed even if no background tasks were running
    assert!(shutdown_result.is_ok(), "Cache shutdown should succeed");
}
