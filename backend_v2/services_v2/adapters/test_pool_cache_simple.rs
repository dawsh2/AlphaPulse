//! Simple pool cache integration test
//!
//! This test validates the basic pool cache functionality without
//! relying on the broken TLV structures in the codebase.

use alphapulse_state_market::pool_cache::PoolCache;
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_pool_cache_basic_functionality() {
    // Create temporary directory for cache persistence
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create pool cache with persistence
    let cache = Arc::new(PoolCache::with_persistence(
        temp_dir.path().to_path_buf(),
        137,
    ));

    // Test basic cache operations
    let stats = cache.stats();
    println!("Initial cache stats: {} pools", stats.cached_pools);
    assert_eq!(stats.cached_pools, 0);

    // Test cache loading (should be empty initially)
    let loaded_count = cache.load_from_disk().await.expect("Load should succeed");
    assert_eq!(loaded_count, 0);

    // Test unknown pool lookup (should fail due to no RPC, but not panic)
    let test_pool = [0x42u8; 20];
    let discovery_result = cache.get_or_discover_pool(test_pool).await;
    assert!(
        discovery_result.is_err(),
        "Discovery should fail without RPC configured"
    );

    // Test that pool is not cached after failed discovery
    assert!(!cache.is_cached(&test_pool));

    // Test snapshot creation
    let snapshot_result = cache.force_snapshot().await;
    assert!(snapshot_result.is_ok(), "Snapshot should succeed");

    // Verify cache file was created
    let cache_file = temp_dir.path().join("pool_cache.tlv");
    assert!(
        cache_file.exists(),
        "Cache file should exist after snapshot"
    );

    println!("✅ Pool cache basic functionality test passed");
}

#[tokio::test]
async fn test_pool_cache_concurrent_access() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = Arc::new(PoolCache::with_persistence(
        temp_dir.path().to_path_buf(),
        137,
    ));

    // Test concurrent cache operations
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let cache = cache.clone();
            tokio::spawn(async move {
                let test_pool = [i as u8; 20];
                let _ = cache.is_cached(&test_pool);
                let _ = cache.get_cached(&test_pool);
                i
            })
        })
        .collect();

    // All operations should complete without panic
    for handle in handles {
        let task_id = handle.await.expect("Task should complete");
        println!("Concurrent task {} completed", task_id);
    }

    println!("✅ Pool cache concurrent access test passed");
}

fn main() {
    println!("This is a test file - run with `cargo test --test test_pool_cache_simple`");
}
