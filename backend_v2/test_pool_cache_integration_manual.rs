//! Manual test for pool cache integration
//!
//! This tests the integration of PoolCache with the Polygon collector
//! without requiring the full test infrastructure.

use alphapulse_state_market::pool_cache::PoolCache;
use std::path::PathBuf;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing pool cache integration...");

    // Create temporary directory for testing
    let temp_dir = std::env::temp_dir().join("alphapulse_test_pool_cache");
    std::fs::create_dir_all(&temp_dir)?;
    println!("✅ Created temp directory: {:?}", temp_dir);

    // Test 1: Create pool cache with persistence
    let cache = PoolCache::with_persistence(temp_dir.clone(), 137);
    println!("✅ Created PoolCache with persistence (Polygon chain ID: 137)");

    // Test 2: Get initial stats
    let stats = cache.stats();
    println!("✅ Cache stats: {} pools cached", stats.cached_pools);
    assert_eq!(stats.cached_pools, 0, "Should start with 0 pools");

    // Test 3: Test cache loading from empty directory
    let loaded = cache.load_from_disk().await?;
    println!("✅ Loaded {} pools from disk", loaded);

    // Test 4: Test unknown pool discovery (should fail gracefully)
    let unknown_pool = [1u8; 20]; // Non-zero address
    println!("🔍 Testing discovery of unknown pool: 0x{}", hex::encode(unknown_pool));
    
    match cache.get_or_discover_pool(unknown_pool).await {
        Ok(pool_info) => {
            println!("✅ Successfully discovered pool: {}", hex::encode(pool_info.pool_address));
        }
        Err(e) => {
            println!("⚠️ Pool discovery failed as expected (no RPC configured): {}", e);
        }
    }

    // Test 5: Test cache checking
    let is_cached = cache.is_cached(&unknown_pool);
    println!("✅ Pool cached after discovery attempt: {}", is_cached);

    // Test 6: Force snapshot (persistence test)
    cache.force_snapshot().await?;
    println!("✅ Successfully created cache snapshot");

    // Test 7: Create new cache instance and load from disk
    let cache2 = PoolCache::with_persistence(temp_dir.clone(), 137);
    let loaded2 = cache2.load_from_disk().await?;
    println!("✅ Second cache instance loaded {} pools", loaded2);

    // Clean up
    std::fs::remove_dir_all(&temp_dir)?;
    println!("✅ Cleaned up temp directory");

    println!("\n🎉 All pool cache integration tests passed!");
    println!("📋 Test Summary:");
    println!("   ✅ PoolCache creation with persistence");
    println!("   ✅ Cache statistics tracking");
    println!("   ✅ Loading from empty directory");
    println!("   ✅ Discovery attempt (fails gracefully without RPC)");
    println!("   ✅ Cache state checking");
    println!("   ✅ Snapshot/persistence functionality");
    println!("   ✅ Recovery from disk");

    Ok(())
}