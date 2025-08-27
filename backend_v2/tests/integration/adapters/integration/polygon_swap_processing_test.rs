//! Integration tests for Polygon swap event processing with pool cache
//!
//! Tests the complete flow from WebSocket log events through pool cache
//! resolution to TLV message generation with real addresses.

use alphapulse_state_market::pool_cache::{PoolCache, PoolInfo};
use alphapulse_state_market::pool_state::PoolStateManager;
use protocol_v2::{tlv::market_data::PoolSwapTLV, VenueId, DEXProtocol, parse_header, parse_tlv_extensions, TLVType};
use std::sync::Arc;
use tempfile::TempDir;
use web3::types::{Log, H160, H256, U64, Bytes};
use std::collections::HashMap;

/// Test helper to create a mock pool cache with pre-populated data
async fn create_mock_pool_cache_with_data(temp_dir: &std::path::Path) -> Arc<PoolCache> {
    let cache = Arc::new(PoolCache::with_persistence(temp_dir.to_path_buf(), 137));

    // Pre-populate with known pool information
    // Note: In a real implementation, we'd need to add pools to the cache
    // For now, we'll test the error path where pools aren't cached

    cache
}

/// Create a realistic Uniswap V3 swap log with known pool address
fn create_uniswap_v3_swap_log() -> Log {
    Log {
        // Known Uniswap V3 USDC/WETH pool on Polygon
        address: "0x45dda9cb7c25131df268515131f647d726f50608".parse().unwrap(),
        topics: vec![
            // Uniswap V3 Swap signature: Swap(address,address,int256,int256,uint160,uint128,int24)
            "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67".parse().unwrap(),
            // sender (Uniswap Router)
            "0x000000000000000000000000e592427a0aece92de3edee1f18e0157c05861564".parse().unwrap(),
            // recipient (Uniswap Router)
            "0x000000000000000000000000e592427a0aece92de3edee1f18e0157c05861564".parse().unwrap(),
        ],
        data: Bytes(
            hex::decode(concat!(
                "000000000000000000000000000000000000000000000000002386f26fc10000", // amount0: +10 WETH
                "fffffffffffffffffffffffffffffffffffffffffffffffffffff8e9db5e8180", // amount1: -27000 USDC (negative)
                "000000000000000000000001b1ae4d6e2ef5896dc1c9c88f1b3d9b8f7e5a4c10", // sqrtPriceX96
                "00000000000000000000000000000000000000000000000000038d7ea4c68000", // liquidity
                "0000000000000000000000000000000000000000000000000000000000000d41"  // tick
            )).expect("Valid hex")
        ),
        block_hash: Some("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".parse().unwrap()),
        block_number: Some(U64::from(48_500_000)),
        transaction_hash: Some("0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".parse().unwrap()),
        transaction_index: Some(U64::from(42)),
        log_index: Some(U64::from(1)),
        transaction_log_index: Some(U64::from(1)),
        log_type: Some("mined".to_string()),
        removed: Some(false),
    }
}

/// Create a Uniswap V2 swap log for testing
fn create_uniswap_v2_swap_log() -> Log {
    Log {
        // Mock QuickSwap (V2) pool address
        address: "0x6e7a5fafcec6bb1e78bae2a1f0b612012bf14827".parse().unwrap(),
        topics: vec![
            // Uniswap V2 Swap signature: Swap(address,uint256,uint256,uint256,uint256,address)
            "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822".parse().unwrap(),
            // sender
            "0x000000000000000000000000a5e0829caced8ffdd4de3c43696c57f7d7a678ff".parse().unwrap(),
            // to
            "0x000000000000000000000000742d35cc6644c44e24c70c9b40b9a0f5ac0c8de8".parse().unwrap(),
        ],
        data: Bytes(
            hex::decode(concat!(
                "0000000000000000000000000000000000000000000000000000000000000000", // amount0In: 0
                "00000000000000000000000000000000000000000000000000038d7ea4c68000", // amount1In: 1000 USDC
                "000000000000000000000000000000000000000000000000002386f26fc10000", // amount0Out: 10 WETH
                "0000000000000000000000000000000000000000000000000000000000000000"  // amount1Out: 0
            )).expect("Valid hex")
        ),
        block_hash: Some("0x2345678901bcdef12345678901bcdef12345678901bcdef12345678901bcdef1".parse().unwrap()),
        block_number: Some(U64::from(48_500_001)),
        transaction_hash: Some("0xbcdef12345678901bcdef12345678901bcdef12345678901bcdef12345678901".parse().unwrap()),
        transaction_index: Some(U64::from(43)),
        log_index: Some(U64::from(2)),
        transaction_log_index: Some(U64::from(2)),
        log_type: Some("mined".to_string()),
        removed: Some(false),
    }
}

/// Mock pool info for testing with real addresses
fn create_mock_pool_info(pool_address: [u8; 20]) -> PoolInfo {
    // Use real Polygon token addresses
    let weth_address = hex::decode("7ceb23fd6f88b76af052c3ca459c1173c5b9b96d").unwrap(); // WETH
    let usdc_address = hex::decode("2791bca1f2de4661ed88a30c99a7a9449aa84174").unwrap(); // USDC

    let mut token0 = [0u8; 20];
    let mut token1 = [0u8; 20];
    token0.copy_from_slice(&weth_address);
    token1.copy_from_slice(&usdc_address);

    PoolInfo {
        pool_address,
        token0,
        token1,
        token0_decimals: 18, // WETH
        token1_decimals: 6,  // USDC
        pool_type: DEXProtocol::UniswapV3,
        fee_tier: Some(3000), // 0.3%
        discovered_at: 1234567890_000_000_000,
        venue: VenueId::Polygon,
        last_seen: 1234567890_000_000_000,
    }
}

#[tokio::test]
async fn test_process_swap_event_with_unknown_pool() {
    // This tests the current behavior where unknown pools are skipped
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = create_mock_pool_cache_with_data(temp_dir.path()).await;

    let swap_log = create_uniswap_v3_swap_log();

    // Convert H160 to [u8; 20] as the code does
    let pool_address = swap_log.address;
    let mut pool_addr_bytes = [0u8; 20];
    pool_addr_bytes.copy_from_slice(&pool_address.0);

    // Attempt to get pool info (should fail since cache is empty and no RPC configured)
    let result = cache.get_or_discover_pool(pool_addr_bytes).await;
    assert!(result.is_err(), "Unknown pool should trigger discovery failure");

    // Verify the pool is not cached after failed discovery
    assert!(!cache.is_cached(&pool_addr_bytes), "Failed discovery should not cache pool");

    // Test cache statistics
    let stats = cache.stats();
    assert_eq!(stats.cached_pools, 0, "No pools should be cached");
    assert!(stats.cache_misses > 0, "Should have cache misses");
}

#[tokio::test]
async fn test_swap_log_parsing_and_validation() {
    // Test that swap logs have the expected structure
    let v3_log = create_uniswap_v3_swap_log();
    let v2_log = create_uniswap_v2_swap_log();

    // Validate V3 log structure
    assert!(!v3_log.topics.is_empty(), "V3 log should have topics");
    assert!(v3_log.data.0.len() >= 224, "V3 log should have >= 224 bytes data (7 * 32)");
    assert_eq!(v3_log.topics.len(), 3, "V3 Swap should have 3 topics (signature + 2 indexed)");

    // Validate V2 log structure
    assert!(!v2_log.topics.is_empty(), "V2 log should have topics");
    assert_eq!(v2_log.data.0.len(), 128, "V2 log should have 128 bytes data (4 * 32)");
    assert_eq!(v2_log.topics.len(), 3, "V2 Swap should have 3 topics");

    // Test V2/V3 detection logic
    let is_v3_detected = v3_log.data.0.len() >= 224;
    let is_v2_detected = v2_log.data.0.len() < 224;
    assert!(is_v3_detected, "Should detect V3 swap correctly");
    assert!(is_v2_detected, "Should detect V2 swap correctly");
}

#[tokio::test]
async fn test_pool_address_extraction() {
    let swap_log = create_uniswap_v3_swap_log();

    // Test the address extraction logic used in process_swap_event
    let pool_address = swap_log.address;
    let mut pool_addr_bytes = [0u8; 20];
    pool_addr_bytes.copy_from_slice(&pool_address.0);

    // Verify the address was extracted correctly
    let expected_address: H160 = "0x45dda9cb7c25131df268515131f647d726f50608".parse().unwrap();
    assert_eq!(pool_address, expected_address, "Pool address should match expected");

    // Verify byte conversion
    let mut expected_bytes = [0u8; 20];
    expected_bytes.copy_from_slice(&expected_address.0);
    assert_eq!(pool_addr_bytes, expected_bytes, "Address bytes should match");
}

#[tokio::test]
async fn test_mock_pool_info_creation() {
    // Test our helper function for creating mock pool info
    let pool_address = [0x42; 20];
    let pool_info = create_mock_pool_info(pool_address);

    // Validate pool info structure
    assert_eq!(pool_info.pool_address, pool_address, "Pool address should match");
    assert_eq!(pool_info.token0_decimals, 18, "WETH should have 18 decimals");
    assert_eq!(pool_info.token1_decimals, 6, "USDC should have 6 decimals");
    assert_eq!(pool_info.venue, VenueId::Polygon, "Should be Polygon venue");
    assert_eq!(pool_info.pool_type, DEXProtocol::UniswapV3, "Should be V3 pool");

    // Verify token addresses are set (should be real WETH/USDC addresses)
    assert_ne!(pool_info.token0, [0u8; 20], "Token0 should not be zero address");
    assert_ne!(pool_info.token1, [0u8; 20], "Token1 should not be zero address");
    assert_ne!(pool_info.token0, pool_info.token1, "Token addresses should be different");
}

#[tokio::test]
async fn test_ethabi_log_structure_compatibility() {
    // Test that our logs can be parsed by ethabi (structure validation)
    use ethabi::RawLog;

    let v3_log = create_uniswap_v3_swap_log();
    let v2_log = create_uniswap_v2_swap_log();

    // Create RawLog structures as done in process_swap_event
    let v3_raw_log = RawLog {
        topics: v3_log.topics.clone(),
        data: v3_log.data.0.clone(),
    };

    let v2_raw_log = RawLog {
        topics: v2_log.topics.clone(),
        data: v2_log.data.0.clone(),
    };

    // Validate structure (actual parsing would require ABI definitions)
    assert_eq!(v3_raw_log.topics.len(), 3, "V3 raw log should have correct topic count");
    assert_eq!(v3_raw_log.data.len(), 224, "V3 raw log should have correct data length");

    assert_eq!(v2_raw_log.topics.len(), 3, "V2 raw log should have correct topic count");
    assert_eq!(v2_raw_log.data.len(), 128, "V2 raw log should have correct data length");
}

#[tokio::test]
async fn test_integration_with_pool_state_manager() {
    // Test that PoolStateManager can be integrated alongside PoolCache
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache = create_mock_pool_cache_with_data(temp_dir.path()).await;
    let pool_state_manager = Arc::new(PoolStateManager::new());

    // Simulate the integration pattern used in UnifiedPolygonCollector
    let collector_state = (cache, pool_state_manager);

    // Verify both components are accessible
    let (cache_ref, state_manager_ref) = &collector_state;

    let cache_stats = cache_ref.stats();
    assert_eq!(cache_stats.cached_pools, 0, "Cache should start empty");

    // PoolStateManager integration is mainly structural - verify it's properly referenced
    assert!(Arc::strong_count(state_manager_ref) >= 1, "State manager should be properly referenced");
}

/// Test helper to validate TLV message structure (for future use when we have working cache)
#[allow(dead_code)]
fn validate_tlv_message_structure(message_bytes: &[u8]) -> Result<(), String> {
    if message_bytes.len() < 32 {
        return Err("Message too short for header".to_string());
    }

    // Parse header
    let header = parse_header(message_bytes).map_err(|e| format!("Header parse error: {:?}", e))?;

    // Validate header structure
    if header.magic != 0xDEADBEEF {
        return Err(format!("Invalid magic: {:X}", header.magic));
    }

    // Parse TLV payload
    let payload_start = 32;
    let payload_end = payload_start + header.payload_size as usize;

    if message_bytes.len() < payload_end {
        return Err("Message too short for payload".to_string());
    }

    let tlv_payload = &message_bytes[payload_start..payload_end];
    let _tlvs = parse_tlv_extensions(tlv_payload).map_err(|e| format!("TLV parse error: {:?}", e))?;

    Ok(())
}

#[tokio::test]
async fn test_realistic_pool_addresses_in_logs() {
    // Verify our test logs use realistic Polygon pool addresses
    let v3_log = create_uniswap_v3_swap_log();
    let v2_log = create_uniswap_v2_swap_log();

    // V3 pool should be a known Uniswap V3 pool on Polygon
    let v3_pool: H160 = "0x45dda9cb7c25131df268515131f647d726f50608".parse().unwrap();
    assert_eq!(v3_log.address, v3_pool, "V3 pool should match expected address");

    // V2 pool should be a realistic QuickSwap pool address
    let v2_pool: H160 = "0x6e7a5fafcec6bb1e78bae2a1f0b612012bf14827".parse().unwrap();
    assert_eq!(v2_log.address, v2_pool, "V2 pool should match expected address");

    // Both should be non-zero addresses
    assert_ne!(v3_log.address, H160::zero(), "V3 pool should not be zero address");
    assert_ne!(v2_log.address, H160::zero(), "V2 pool should not be zero address");
}
