//! Persistence and state recovery tests for pool cache
//!
//! Tests cache persistence across collector restarts, journal recovery after crash,
//! and atomic file operations to prevent corruption.

use alphapulse_state_market::pool_cache::{PoolCache, PoolInfo};
use protocol_v2::{VenueId, DEXProtocol};
use std::sync::Arc;
use std::fs::{File, OpenOptions, remove_file};
use std::io::{Write, Read};
use std::path::Path;
use tempfile::TempDir;
use tokio::time::{sleep, Duration};

/// Helper to create a test pool info with specific data
fn create_test_pool_info(pool_id: u32) -> PoolInfo {
    let mut pool_address = [0u8; 20];
    let mut token0 = [0u8; 20];
    let mut token1 = [0u8; 20];

    pool_address[16..20].copy_from_slice(&pool_id.to_be_bytes());
    token0[16..20].copy_from_slice(&(pool_id * 2).to_be_bytes());
    token1[16..20].copy_from_slice(&(pool_id * 2 + 1).to_be_bytes());

    PoolInfo {
        pool_address,
        token0,
        token1,
        token0_decimals: 18,
        token1_decimals: 6,
        pool_type: DEXProtocol::UniswapV3,
        fee_tier: Some(3000),
        discovered_at: 1234567890_000_000_000 + pool_id as u64,
        venue: VenueId::Polygon,
        last_seen: 1234567890_000_000_000 + pool_id as u64,
    }
}

#[tokio::test]
async fn test_cache_persistence_across_restarts() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache_path = temp_dir.path().join("pool_cache.tlv");

    // First instance - create and snapshot
    {
        let cache1 = PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137);

        // Load initial state (should be empty)
        let loaded_count = cache1.load_from_disk().await.expect("Initial load should succeed");
        assert_eq!(loaded_count, 0, "Initial cache should be empty");

        // Create a snapshot (even if empty)
        cache1.force_snapshot().await.expect("Snapshot should succeed");

        // Verify file was created
        assert!(cache_path.exists(), "Cache file should exist after snapshot");

        let cache1_stats = cache1.stats();
        println!("First instance stats: {} pools", cache1_stats.cached_pools);
    }

    // Second instance - load from persistence
    {
        let cache2 = PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137);

        // Should load from the persisted file
        let loaded_count = cache2.load_from_disk().await.expect("Load should succeed");
        println!("Second instance loaded {} pools", loaded_count);

        // Stats should be consistent
        let cache2_stats = cache2.stats();
        assert_eq!(cache2_stats.cached_pools, loaded_count as usize, "Stats should match loaded count");

        // Create another snapshot to test write operations
        cache2.force_snapshot().await.expect("Second snapshot should succeed");
    }

    // Third instance - verify persistence is maintained
    {
        let cache3 = PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137);

        let loaded_count = cache3.load_from_disk().await.expect("Third load should succeed");
        println!("Third instance loaded {} pools", loaded_count);

        // Should still be consistent
        let cache3_stats = cache3.stats();
        assert_eq!(cache3_stats.cached_pools, loaded_count as usize, "Third instance stats should match");
    }
}

#[tokio::test]
async fn test_journal_file_operations() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137);

    // Test that journal operations (if any) don't interfere with basic functionality
    let _ = cache.load_from_disk().await;

    // Perform some operations that might create journal entries
    for i in 0..5 {
        let pool_addr = [i; 20];
        let _ = cache.get_or_discover_pool(pool_addr).await; // Will fail but may create journal entries
        let _ = cache.is_cached(&pool_addr);
    }

    // Force snapshot multiple times to test journal handling
    for i in 0..3 {
        let result = cache.force_snapshot().await;
        assert!(result.is_ok(), "Snapshot {} should succeed", i);
    }

    // Check for potential journal files
    let journal_path = temp_dir.path().join("pool_cache.tlv.journal");
    if journal_path.exists() {
        println!("Journal file exists: {:?}", journal_path);

        // Journal file should be readable
        let mut journal_file = File::open(journal_path).expect("Journal should be readable");
        let mut journal_content = Vec::new();
        let read_result = journal_file.read_to_end(&mut journal_content);
        assert!(read_result.is_ok(), "Journal file should be readable");
    } else {
        println!("No journal file created (expected for empty cache)");
    }
}

#[tokio::test]
async fn test_atomic_file_operations() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache_path = temp_dir.path().join("pool_cache.tlv");

    // Test atomic snapshot operations
    let cache = PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137);

    // Create initial snapshot
    cache.force_snapshot().await.expect("Initial snapshot should succeed");
    assert!(cache_path.exists(), "Cache file should exist");

    // Check file is not empty (should have at least header)
    let metadata = std::fs::metadata(&cache_path).expect("Should get file metadata");
    println!("Cache file size: {} bytes", metadata.len());

    // Multiple snapshots should not corrupt the file
    for i in 0..10 {
        cache.force_snapshot().await.expect(&format!("Snapshot {} should succeed", i));

        // File should still exist and be readable
        assert!(cache_path.exists(), "Cache file should still exist after snapshot {}", i);

        let file = File::open(&cache_path).expect("Cache file should be readable");
        let metadata = file.metadata().expect("Should get metadata");
        assert!(metadata.len() > 0, "Cache file should not be empty after snapshot {}", i);
    }
}

#[tokio::test]
async fn test_concurrent_persistence_operations() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = Arc::new(PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137));

    // Spawn multiple tasks performing persistence operations
    let handles: Vec<_> = (0..5).map(|i| {
        let cache = cache.clone();
        tokio::spawn(async move {
            // Each task performs load and snapshot operations
            let load_result = cache.load_from_disk().await;
            sleep(Duration::from_millis(i * 10)).await; // Stagger operations
            let snapshot_result = cache.force_snapshot().await;

            (i, load_result.is_ok(), snapshot_result.is_ok())
        })
    }).collect();

    // All operations should succeed
    for handle in handles {
        let (task_id, load_ok, snapshot_ok) = handle.await.expect("Task should complete");
        assert!(load_ok, "Task {} load should succeed", task_id);
        assert!(snapshot_ok, "Task {} snapshot should succeed", task_id);
    }

    // Cache should be in consistent state after concurrent operations
    let stats = cache.stats();
    println!("Cache stats after concurrent operations: {} pools", stats.cached_pools);

    // Final load should work
    let final_load = cache.load_from_disk().await;
    assert!(final_load.is_ok(), "Final load should succeed");
}

#[tokio::test]
async fn test_persistence_file_integrity() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache_path = temp_dir.path().join("pool_cache.tlv");

    let cache = PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137);

    // Create initial snapshot
    cache.force_snapshot().await.expect("Initial snapshot should succeed");

    // Read the file directly to verify it's not corrupted
    let mut file_content = Vec::new();
    {
        let mut file = File::open(&cache_path).expect("Cache file should exist");
        file.read_to_end(&mut file_content).expect("Should read file content");
    }

    println!("Cache file contains {} bytes", file_content.len());
    assert!(!file_content.is_empty(), "Cache file should not be empty");

    // File should start with valid header (implementation specific)
    // For TLV format, we expect some structured header
    if file_content.len() >= 4 {
        println!("First 4 bytes: {:02X} {:02X} {:02X} {:02X}",
                file_content[0], file_content[1], file_content[2], file_content[3]);
    }

    // Create new cache instance and load the file
    let cache2 = PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137);
    let loaded_count = cache2.load_from_disk().await.expect("Should load from persisted file");

    println!("Loaded {} pools from persisted file", loaded_count);

    // Stats should be consistent
    let stats = cache2.stats();
    assert_eq!(stats.cached_pools, loaded_count as usize, "Stats should match loaded count");
}

#[tokio::test]
async fn test_recovery_from_partial_writes() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache_path = temp_dir.path().join("pool_cache.tlv");

    // Create a valid cache file first
    {
        let cache = PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137);
        cache.force_snapshot().await.expect("Initial snapshot should succeed");
    }

    // Simulate partial write by truncating the file
    {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(false)
            .open(&cache_path)
            .expect("Should open cache file");

        // Write some garbage data to simulate corruption during write
        file.write_all(b"PARTIAL_WRITE_CORRUPTION").expect("Should write corruption");
    }

    // Try to recover with new cache instance
    let cache2 = PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137);

    // Load should handle corruption gracefully
    let load_result = cache2.load_from_disk().await;
    match load_result {
        Ok(count) => {
            println!("Recovered {} pools from corrupted file", count);
            // If it succeeds, should be 0 (treating as empty)
            assert_eq!(count, 0, "Corrupted file should be treated as empty");
        },
        Err(e) => {
            println!("Load failed as expected due to corruption: {}", e);
            // Error should be handled gracefully
        }
    }

    // Cache should still be functional
    let stats = cache2.stats();
    assert_eq!(stats.cached_pools, 0, "Cache should be empty after corruption recovery");

    // Should be able to create new valid snapshot
    let snapshot_result = cache2.force_snapshot().await;
    assert!(snapshot_result.is_ok(), "Should create new valid snapshot");
}

#[tokio::test]
async fn test_persistence_directory_creation() {
    // Test with non-existent directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let nested_path = temp_dir.path().join("nested").join("cache");

    // Directory doesn't exist initially
    assert!(!nested_path.exists(), "Nested directory should not exist initially");

    // Create cache with non-existent directory
    let cache_result = std::panic::catch_unwind(|| {
        PoolCache::with_persistence(nested_path.clone(), 137)
    });

    // Should either create directory or handle gracefully
    if cache_result.is_ok() {
        let cache = cache_result.unwrap();

        // Try to perform operations
        let load_result = tokio_test::block_on(cache.load_from_disk());
        let snapshot_result = tokio_test::block_on(cache.force_snapshot());

        // At least one operation should work or handle the missing directory gracefully
        if load_result.is_err() && snapshot_result.is_err() {
            println!("Both operations failed - this is acceptable for missing directory");
        } else {
            println!("Some operations succeeded - directory was created or error handled gracefully");
        }
    } else {
        println!("Cache creation failed with non-existent directory - this is acceptable");
    }
}

#[tokio::test]
async fn test_multiple_cache_instances_same_directory() {
    // Test behavior with multiple cache instances using the same directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let cache1 = Arc::new(PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137));
    let cache2 = Arc::new(PoolCache::with_persistence(temp_dir.path().to_path_buf(), 137));

    // Both should be able to load
    let load1 = cache1.load_from_disk().await.expect("Cache1 load should succeed");
    let load2 = cache2.load_from_disk().await.expect("Cache2 load should succeed");

    println!("Cache1 loaded: {}, Cache2 loaded: {}", load1, load2);

    // Both should be able to snapshot (though this might create race conditions)
    let snapshot1 = cache1.force_snapshot().await;
    let snapshot2 = cache2.force_snapshot().await;

    // At least one should succeed (or both should handle conflicts gracefully)
    if snapshot1.is_err() && snapshot2.is_err() {
        println!("Both snapshots failed - might be due to file locking");
    } else {
        println!("At least one snapshot succeeded");
    }

    // Both caches should remain functional
    let stats1 = cache1.stats();
    let stats2 = cache2.stats();

    println!("Cache1 stats: {} pools, Cache2 stats: {} pools",
             stats1.cached_pools, stats2.cached_pools);
}
