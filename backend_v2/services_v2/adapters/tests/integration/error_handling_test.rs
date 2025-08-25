//! Error handling and recovery tests for pool cache integration
//!
//! Tests graceful degradation and recovery scenarios when RPC is unavailable,
//! cache files are corrupted, or other error conditions occur.

use alphapulse_state_market::pool_cache::{PoolCache, PoolCacheError};
use std::sync::Arc;
use std::fs::{File, OpenOptions};
use std::io::Write;
use tempfile::TempDir;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_rpc_unavailable_graceful_degradation() {
    // Test behavior when RPC endpoint is not configured/available
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = Arc::new(PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137));

    let unknown_pools = [
        [0x01; 20],
        [0x02; 20],
        [0x03; 20],
        [0x00; 20], // Zero address (should be rejected)
        [0xFF; 20], // Invalid address
    ];

    // All discovery attempts should fail gracefully
    for (i, pool_addr) in unknown_pools.iter().enumerate() {
        let result = cache.get_or_discover_pool(*pool_addr).await;

        // Should fail but not panic
        assert!(result.is_err(), "Pool {} should fail discovery without RPC", i);

        // Pool should not be cached after failed discovery
        assert!(!cache.is_cached(pool_addr), "Pool {} should not be cached after failed discovery", i);

        // Error should be meaningful
        if let Err(error) = result {
            match error {
                PoolCacheError::Discovery(msg) => {
                    assert!(
                        msg.contains("RPC") ||
                        msg.contains("network") ||
                        msg.contains("connection") ||
                        msg.contains("pool") ||
                        msg.contains("token"),
                        "Error message should be meaningful: {}", msg
                    );
                },
                PoolCacheError::Other(_) => {
                    // Other errors are also acceptable for unavailable RPC
                },
            }
        }
    }

    // Cache should still be functional after errors
    let stats = cache.stats();
    assert_eq!(stats.cached_pools, 0, "No pools should be cached");
    assert!(stats.cache_misses >= unknown_pools.len(), "Should have recorded cache misses");

    // Basic operations should still work
    let _ = cache.load_from_disk().await;
    let _ = cache.force_snapshot().await;
}

#[tokio::test]
async fn test_corrupted_cache_file_recovery() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache_file_path = temp_dir.path().join("pool_cache.tlv");

    // Create a corrupted cache file
    {
        let mut corrupted_file = File::create(&cache_file_path).expect("Failed to create file");
        corrupted_file.write_all(b"INVALID_CACHE_DATA_CORRUPTED").expect("Failed to write corrupted data");
    }

    // Create cache and attempt to load corrupted file
    let cache = Arc::new(PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137));

    // Loading should handle corruption gracefully
    let load_result = cache.load_from_disk().await;
    match load_result {
        Ok(count) => {
            // If it succeeds, should load 0 pools (treating file as empty)
            assert_eq!(count, 0, "Corrupted file should be treated as empty");
        },
        Err(_) => {
            // If it fails, error should be handled gracefully
            println!("Corrupted cache file handling: expected error occurred");
        }
    }

    // Cache should still be functional after corruption
    let stats = cache.stats();
    assert_eq!(stats.cached_pools, 0, "Cache should be empty after corruption");

    // Basic operations should work
    let snapshot_result = cache.force_snapshot().await;
    assert!(snapshot_result.is_ok(), "Should be able to create new snapshot after corruption");

    // Verify new file was created
    assert!(cache_file_path.exists(), "Cache file should exist after snapshot");
}

#[tokio::test]
async fn test_disk_space_exhaustion_handling() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = Arc::new(PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137));

    // Test that cache operations handle disk space issues gracefully
    // Note: We can't actually exhaust disk space in tests, so we test the error paths

    // Perform some operations
    let _ = cache.load_from_disk().await;

    // Test snapshot creation - should handle errors gracefully
    let snapshot_result = cache.force_snapshot().await;

    // This should succeed in test environment
    if snapshot_result.is_ok() {
        println!("Snapshot created successfully");
    } else {
        println!("Snapshot failed (expected in some test environments)");
        // Failure should be handled gracefully - no panic
    }

    // Cache should remain functional
    let stats = cache.stats();
    println!("Cache stats after potential disk error: {} pools", stats.cached_pools);
}

#[tokio::test]
async fn test_concurrent_discovery_of_same_pool() {
    // Test that concurrent discovery of the same unknown pool is handled correctly
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = Arc::new(PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137));

    let test_pool = [0x42; 20];
    let num_concurrent_requests = 10;

    // Spawn multiple tasks trying to discover the same pool
    let handles: Vec<_> = (0..num_concurrent_requests).map(|i| {
        let cache = cache.clone();
        tokio::spawn(async move {
            let result = cache.get_or_discover_pool(test_pool).await;
            (i, result.is_err())
        })
    }).collect();

    // All should fail (no RPC configured) but should handle concurrency correctly
    let mut all_failed = true;
    for handle in handles {
        let (task_id, failed) = handle.await.expect("Task should complete");
        all_failed &= failed;
        println!("Task {}: discovery failed = {}", task_id, failed);
    }

    assert!(all_failed, "All discovery attempts should fail without RPC");

    // Pool should not be cached after failed concurrent discovery
    assert!(!cache.is_cached(&test_pool), "Pool should not be cached after failed discovery");

    // Cache statistics should be consistent
    let stats = cache.stats();
    assert!(stats.cache_misses > 0, "Should have recorded cache misses");
    assert_eq!(stats.cached_pools, 0, "No pools should be cached");
}

#[tokio::test]
async fn test_invalid_pool_addresses_rejection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = Arc::new(PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137));

    let invalid_addresses = [
        [0x00; 20], // Zero address
        [0xFF; 20], // All 0xFF (unlikely to be valid)
    ];

    // Test invalid address handling
    for (i, invalid_addr) in invalid_addresses.iter().enumerate() {
        let result = cache.get_or_discover_pool(*invalid_addr).await;

        // Should fail appropriately
        assert!(result.is_err(), "Invalid address {} should be rejected", i);

        // Should not be cached
        assert!(!cache.is_cached(invalid_addr), "Invalid address {} should not be cached", i);

        // Test get_cached also handles invalid addresses
        let cached_result = cache.get_cached(invalid_addr);
        assert!(cached_result.is_none(), "get_cached should return None for invalid address {}", i);
    }

    // Valid-looking addresses should attempt discovery (and fail due to no RPC)
    let valid_looking_addresses = [
        [0x01; 20],
        [0xAB; 20],
        [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC],
    ];

    for (i, valid_addr) in valid_looking_addresses.iter().enumerate() {
        let result = cache.get_or_discover_pool(*valid_addr).await;

        // Should attempt discovery and fail (due to no RPC)
        assert!(result.is_err(), "Valid-looking address {} should attempt discovery", i);

        // Should not be cached after failed discovery
        assert!(!cache.is_cached(valid_addr), "Valid-looking address {} should not be cached after failed discovery", i);
    }
}

#[tokio::test]
async fn test_cache_state_consistency_after_errors() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = Arc::new(PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137));

    // Perform various operations that will fail
    let test_pools = [
        [0x01; 20],
        [0x02; 20],
        [0x00; 20], // Invalid
        [0x03; 20],
        [0xFF; 20], // Invalid
    ];

    let mut expected_misses = 0;

    for pool in &test_pools {
        // Attempt discovery
        let discovery_result = cache.get_or_discover_pool(*pool).await;
        assert!(discovery_result.is_err(), "Discovery should fail");
        expected_misses += 1;

        // Check cached status
        let cached_result = cache.get_cached(pool);
        assert!(cached_result.is_none(), "Should not be cached");
        expected_misses += 1;

        // Check is_cached
        let is_cached = cache.is_cached(pool);
        assert!(!is_cached, "Should not be cached");
    }

    // Verify cache state is consistent
    let stats = cache.stats();
    assert_eq!(stats.cached_pools, 0, "No pools should be cached");
    assert!(stats.cache_misses >= expected_misses, "Should have recorded all cache misses");
    assert_eq!(stats.cache_hits, 0, "Should have no cache hits");

    // Test persistence operations still work
    let load_result = cache.load_from_disk().await;
    assert!(load_result.is_ok(), "Load should succeed");

    let snapshot_result = cache.force_snapshot().await;
    assert!(snapshot_result.is_ok(), "Snapshot should succeed");

    // Stats should remain consistent after persistence operations
    let final_stats = cache.stats();
    assert_eq!(final_stats.cached_pools, 0, "No pools should be cached after persistence ops");
}

#[tokio::test]
async fn test_cache_recovery_after_temporary_failures() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = Arc::new(PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137));

    // Simulate temporary failure scenario
    let test_pool = [0x42; 20];

    // First attempt - should fail due to no RPC
    let result1 = cache.get_or_discover_pool(test_pool).await;
    assert!(result1.is_err(), "First discovery should fail");

    // Immediate retry - should also fail
    let result2 = cache.get_or_discover_pool(test_pool).await;
    assert!(result2.is_err(), "Immediate retry should also fail");

    // Wait a bit and retry - should still fail but handle gracefully
    sleep(Duration::from_millis(10)).await;
    let result3 = cache.get_or_discover_pool(test_pool).await;
    assert!(result3.is_err(), "Delayed retry should fail");

    // Cache should remain in consistent state
    assert!(!cache.is_cached(&test_pool), "Pool should not be cached");

    let stats = cache.stats();
    assert!(stats.cache_misses >= 3, "Should have recorded all attempts");
    assert_eq!(stats.cached_pools, 0, "No pools should be cached");

    // Other operations should work normally
    let other_pool = [0x43; 20];
    let other_result = cache.get_cached(&other_pool);
    assert!(other_result.is_none(), "Other pool should not be cached");
}

#[tokio::test]
async fn test_shutdown_graceful_handling_after_errors() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Test shutdown after various error conditions
    let cache = PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137);

    // Cause some errors
    let _ = cache.get_or_discover_pool([0x00; 20]).await; // Invalid pool
    let _ = cache.get_or_discover_pool([0x01; 20]).await; // Discovery failure

    // Shutdown should work gracefully even after errors
    let shutdown_result = cache.shutdown().await;
    assert!(shutdown_result.is_ok(), "Shutdown should succeed even after errors");
}
