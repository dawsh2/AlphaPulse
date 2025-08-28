//! Live Token Address Validation Integration Test
//!
//! Tests our TokenAddressValidator against real Polygon blockchain
//! using public RPC endpoints to verify our parsing and validation logic.

use torq_adapter_service::{
    input::collectors::{
        pool_cache_manager::{PoolCacheManager, PoolInfo},
        polygon_dex::abi_events::{SwapEventDecoder, DEXProtocol},
    },
};
use protocol_v2::VenueId;
use web3::{Web3, transports::Http};
use std::path::PathBuf;
use std::sync::Arc;

// Import our actual TokenAddressValidator from tests
include!("../../../tests/validation/token_address_validator.rs");

#[tokio::test]
#[ignore] // Run with: cargo test live_token_validation -- --ignored --nocapture
async fn test_token_address_validator_with_live_rpc() {
    println!("\n=== Testing TokenAddressValidator with Live Polygon RPC ===\n");

    // Try public RPC endpoints
    let rpc_endpoints = vec![
        "https://polygon-rpc.com",
        "https://rpc.ankr.com/polygon",
        "https://polygon-mainnet.public.blastapi.io",
    ];

    let mut validator = None;
    for endpoint in &rpc_endpoints {
        println!("Trying RPC endpoint: {}", endpoint);

        // Create cache directory for test
        let cache_dir = PathBuf::from("/tmp/torq_test_cache");
        std::fs::create_dir_all(&cache_dir).ok();

        match TokenAddressValidator::new(endpoint, &cache_dir, 137).await {
            Ok(v) => {
                println!("✓ Connected to {}", endpoint);
                validator = Some(v);
                break;
            }
            Err(e) => {
                println!("✗ Failed to connect: {}", e);
            }
        }
    }

    let validator = validator.expect("No working RPC endpoint found");

    // Test with real Polygon swap event from our fixtures
    let log = crate::fixtures::polygon::uniswap_v3_swap_real();

    println!("\nValidating real Uniswap V3 swap event...");
    println!("Pool address: {:?}", log.address);

    // Run our token address validation
    let result = validator.validate_token_addresses(&log, DEXProtocol::UniswapV3).await;

    match result {
        Ok(validated_event) => {
            println!("\n✅ Token address validation PASSED!");

            let pool_info = &validated_event.pool_info;
            println!("\nValidated pool information:");
            println!("  Pool: 0x{}", hex::encode(pool_info.pool_address.as_bytes()));
            println!("  Token0: 0x{}", hex::encode(pool_info.token0.as_bytes()));
            println!("  Token1: 0x{}", hex::encode(pool_info.token1.as_bytes()));
            println!("  Token0 decimals: {}", pool_info.token0_decimals);
            println!("  Token1 decimals: {}", pool_info.token1_decimals);
            println!("  Pool type: {:?}", pool_info.pool_type);
            println!("  Fee tier: {:?}", pool_info.fee_tier);

            // Verify decimals are correct
            assert_eq!(pool_info.token0_decimals, 6, "USDC should have 6 decimals");
            assert_eq!(pool_info.token1_decimals, 18, "WETH should have 18 decimals");

            // Verify swap data was parsed correctly
            let swap_data = &validated_event.validated_data;
            println!("\nValidated swap data:");
            println!("  Amount0: {}", swap_data.amount0);
            println!("  Amount1: {}", swap_data.amount1);
            println!("  Tick: {}", swap_data.tick);

            // Semantic validation - one amount in, one out
            assert!(
                (swap_data.amount0 > 0 && swap_data.amount1 < 0) ||
                (swap_data.amount0 < 0 && swap_data.amount1 > 0),
                "Swap should have one positive and one negative amount"
            );

            println!("\n✅ All validations passed!");
            println!("✅ Token addresses verified against blockchain");
            println!("✅ Token decimals match on-chain values");
            println!("✅ Pool configuration validated");
            println!("✅ Swap semantics are correct");
        }
        Err(e) => {
            panic!("❌ Token address validation failed: {}", e);
        }
    }
}

#[tokio::test]
#[ignore] // Run with: cargo test test_validate_known_tokens -- --ignored --nocapture
async fn test_validate_known_polygon_tokens() {
    println!("\n=== Validating Known Polygon Tokens ===\n");

    // Create validator
    let cache_dir = PathBuf::from("/tmp/torq_test_cache");
    std::fs::create_dir_all(&cache_dir).ok();

    let validator = TokenAddressValidator::new(
        "https://polygon-rpc.com",
        &cache_dir,
        137
    ).await.expect("Failed to create validator");

    // Test known token decimal queries
    let known_tokens = vec![
        ("WMATIC", "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270", 18u8),
        ("USDC", "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174", 6u8),
        ("USDT", "0xc2132D05D31c914a87C6611C10748AEb04B58e8F", 6u8),
        ("WETH", "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619", 18u8),
    ];

    for (name, address_str, expected_decimals) in known_tokens {
        let address = address_str.parse().expect("Invalid address");

        println!("Validating {} token...", name);
        let decimals = validator.query_token_decimals(address).await
            .expect(&format!("Failed to query {} decimals", name));

        if decimals == expected_decimals {
            println!("  ✅ {}: {} decimals (correct)", name, decimals);
        } else {
            panic!("  ❌ {}: {} decimals (expected {})", name, decimals, expected_decimals);
        }
    }

    println!("\n✅ All known token validations passed!");
}

#[tokio::test]
#[ignore] // Run with: cargo test test_pool_factory_validation -- --ignored --nocapture
async fn test_pool_factory_validation() {
    println!("\n=== Testing Pool Factory Validation ===\n");

    let cache_dir = PathBuf::from("/tmp/torq_test_cache");
    std::fs::create_dir_all(&cache_dir).ok();

    let validator = TokenAddressValidator::new(
        "https://polygon-rpc.com",
        &cache_dir,
        137
    ).await.expect("Failed to create validator");

    // Test pool from our fixtures
    let pool_address = "0x45dda9cb7c25131df268515131f647d726f50608".parse().unwrap();

    println!("Validating Uniswap V3 pool: {:?}", pool_address);

    // Query pool info directly
    let pool_info = validator.query_pool_info_from_chain(pool_address).await
        .expect("Failed to query pool info");

    println!("\nPool validated successfully:");
    println!("  Token0: {:?} ({} decimals)", pool_info.token0, pool_info.token0_decimals);
    println!("  Token1: {:?} ({} decimals)", pool_info.token1, pool_info.token1_decimals);
    println!("  Pool type: {:?}", pool_info.pool_type);
    println!("  Fee tier: {:?}", pool_info.fee_tier);

    // Validate against factory
    validator.validate_pool_factory(&pool_info).await
        .expect("Pool factory validation failed");

    println!("\n✅ Pool is valid and from known factory!");

    // Test caching
    validator.pool_cache.upsert(pool_info.clone()).await
        .expect("Failed to cache pool info");

    let cached = validator.pool_cache.get(&pool_address).await
        .expect("Pool should be cached");

    assert_eq!(cached.token0_decimals, pool_info.token0_decimals);
    assert_eq!(cached.token1_decimals, pool_info.token1_decimals);

    println!("✅ Pool info successfully cached and retrieved!");
}
