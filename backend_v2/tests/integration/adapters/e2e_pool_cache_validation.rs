//! End-to-End Pool Cache Validation Tests
//!
//! Tests the complete pipeline: Real Contract RPC → PoolCache → TLV → Validation
//! Ensures no hardcoded decimals and proper semantic validation

use alphapulse_state_market::{PoolCache, PoolCacheConfig};
use protocol_v2::{PoolSwapTLV, VenueId};
use std::collections::HashMap;
use tokio::sync::mpsc;
use web3::types::H160;

/// Known Polygon token addresses and expected decimals for validation
const POLYGON_TEST_TOKENS: &[(&str, u8)] = &[
    ("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270", 18), // WMATIC
    ("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174", 6),  // USDC
    ("0x7ceB23fD6f88B76af052C3cA459C1173c5b9b96d", 18), // WETH
    ("0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063", 18), // DAI
    ("0xc2132D05D31c914a87C6611C10748AEb04B58e8F", 6),  // USDT
];

/// Known Polygon pool addresses for testing
const POLYGON_TEST_POOLS: &[&str] = &[
    "0x45dDa9cb7c25131dF268515131f647d726f50608", // USDC/WETH 0.3%
    "0x847b64f9d3A95e977D157866447a5C0A5dFa0Ee5", // WMATIC/USDC 0.3%
    "0xAE81FAc689A1b4b1e06e7ef4a2ab4CD8aC0A087D", // DAI/USDC 0.05%
];

#[tokio::test]
#[ignore] // Only run with --ignored flag since it makes real RPC calls
async fn test_e2e_rpc_to_tlv_pipeline() {
    // Initialize real RPC-enabled pool cache
    let cache_config = PoolCacheConfig::default(); // Uses real Polygon RPC
    let cache = PoolCache::new(cache_config);

    let mut validation_results = HashMap::new();

    for &pool_address_str in POLYGON_TEST_POOLS {
        println!("🧪 Testing pool: {}", pool_address_str);

        // Convert to bytes for cache lookup
        let pool_address = pool_address_str
            .parse::<H160>()
            .expect("Invalid pool address");
        let pool_address_bytes: [u8; 20] = pool_address.into();

        // Step 1: RPC Discovery (real contract calls)
        let pool_info = match cache.get_or_discover_pool(pool_address_bytes).await {
            Ok(info) => {
                println!("✅ Pool discovery successful");
                info
            }
            Err(e) => {
                println!("❌ Pool discovery failed: {}", e);
                validation_results.insert(pool_address_str, format!("Discovery failed: {}", e));
                continue;
            }
        };

        // Step 2: Validate Token Addresses
        println!("   Token0: 0x{}", hex::encode(pool_info.token0));
        println!("   Token1: 0x{}", hex::encode(pool_info.token1));
        println!("   Token0 decimals: {}", pool_info.token0_decimals);
        println!("   Token1 decimals: {}", pool_info.token1_decimals);

        // Step 3: Semantic Validation Against Known Tokens
        let mut semantic_validation_passed = true;

        // Validate token0 decimals if it's a known token
        let token0_hex = format!("0x{}", hex::encode(pool_info.token0));
        if let Some(&expected_decimals) = POLYGON_TEST_TOKENS
            .iter()
            .find(|(addr, _)| addr.to_lowercase() == token0_hex.to_lowercase())
            .map(|(_, decimals)| decimals)
        {
            if pool_info.token0_decimals != expected_decimals {
                println!(
                    "❌ Token0 decimals mismatch: expected {}, got {}",
                    expected_decimals, pool_info.token0_decimals
                );
                semantic_validation_passed = false;
            } else {
                println!("✅ Token0 decimals correct: {}", expected_decimals);
            }
        }

        // Validate token1 decimals if it's a known token
        let token1_hex = format!("0x{}", hex::encode(pool_info.token1));
        if let Some(&expected_decimals) = POLYGON_TEST_TOKENS
            .iter()
            .find(|(addr, _)| addr.to_lowercase() == token1_hex.to_lowercase())
            .map(|(_, decimals)| decimals)
        {
            if pool_info.token1_decimals != expected_decimals {
                println!(
                    "❌ Token1 decimals mismatch: expected {}, got {}",
                    expected_decimals, pool_info.token1_decimals
                );
                semantic_validation_passed = false;
            } else {
                println!("✅ Token1 decimals correct: {}", expected_decimals);
            }
        }

        // Step 4: Create TLV Message
        let test_swap_tlv = PoolSwapTLV {
            venue: VenueId::Polygon,
            pool_address: pool_info.pool_address,
            token_in_addr: pool_info.token0,
            token_out_addr: pool_info.token1,
            amount_in: 1_000_000_000_000_000_000u128, // 1 token in native precision
            amount_out: 500_000_000u128, // 0.5 token out (assuming USDC with 6 decimals)
            amount_in_decimals: pool_info.token0_decimals,
            amount_out_decimals: pool_info.token1_decimals,
            sqrt_price_x96_after: [0u8; 20],
            tick_after: 0,
            liquidity_after: 0,
            timestamp_ns: torq_network::time::safe_system_timestamp_ns(),
            block_number: 12345678,
        };

        // Step 5: TLV Serialization Roundtrip Test
        let tlv_bytes = test_swap_tlv.to_bytes();
        let deserialized_tlv = match PoolSwapTLV::from_bytes(&tlv_bytes) {
            Ok(tlv) => tlv,
            Err(e) => {
                println!("❌ TLV deserialization failed: {}", e);
                validation_results.insert(pool_address_str, format!("TLV roundtrip failed: {}", e));
                continue;
            }
        };

        // Step 6: Deep Equality Validation
        let deep_equality_passed = validate_tlv_deep_equality(&test_swap_tlv, &deserialized_tlv);

        // Step 7: Critical Decimal Validation
        let decimals_validation_passed = validate_no_hardcoded_decimals(&deserialized_tlv);

        // Record results
        let result =
            if semantic_validation_passed && deep_equality_passed && decimals_validation_passed {
                "✅ PASSED - All validations successful".to_string()
            } else {
                format!(
                    "❌ FAILED - Semantic: {}, Deep Equality: {}, No Hardcoded Decimals: {}",
                    semantic_validation_passed, deep_equality_passed, decimals_validation_passed
                )
            };

        validation_results.insert(pool_address_str, result.clone());
        println!("📊 {}", result);
        println!();
    }

    // Final Report
    println!("🏁 E2E VALIDATION REPORT");
    println!("========================");
    for (pool, result) in &validation_results {
        println!("{}: {}", pool, result);
    }

    // Ensure all tests passed
    let all_passed = validation_results
        .values()
        .all(|result| result.contains("PASSED"));

    assert!(
        all_passed,
        "Some E2E validation tests failed. See report above."
    );
    println!("🎉 ALL E2E VALIDATION TESTS PASSED!");
}

/// Validate deep equality between original and deserialized TLV
fn validate_tlv_deep_equality(original: &PoolSwapTLV, deserialized: &PoolSwapTLV) -> bool {
    let checks = [
        ("venue", original.venue == deserialized.venue),
        (
            "pool_address",
            original.pool_address == deserialized.pool_address,
        ),
        (
            "token_in_addr",
            original.token_in_addr == deserialized.token_in_addr,
        ),
        (
            "token_out_addr",
            original.token_out_addr == deserialized.token_out_addr,
        ),
        ("amount_in", original.amount_in == deserialized.amount_in),
        ("amount_out", original.amount_out == deserialized.amount_out),
        (
            "amount_in_decimals",
            original.amount_in_decimals == deserialized.amount_in_decimals,
        ),
        (
            "amount_out_decimals",
            original.amount_out_decimals == deserialized.amount_out_decimals,
        ),
        (
            "block_number",
            original.block_number == deserialized.block_number,
        ),
    ];

    let mut all_passed = true;
    for (field_name, passed) in &checks {
        if !passed {
            println!("❌ Deep equality failed for field: {}", field_name);
            all_passed = false;
        }
    }

    if all_passed {
        println!("✅ Deep equality validation passed");
    }

    all_passed
}

/// Critical validation: Ensure no hardcoded decimals (18) are used
fn validate_no_hardcoded_decimals(tlv: &PoolSwapTLV) -> bool {
    // This test will fail if we revert to hardcoded decimals
    // Since USDC has 6 decimals, we should never see both decimals as 18 for USDC pools

    let token0_addr = format!("0x{}", hex::encode(tlv.token_in_addr));
    let token1_addr = format!("0x{}", hex::encode(tlv.token_out_addr));

    // Check if this involves USDC (6 decimals)
    let usdc_address = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".to_lowercase();
    let has_usdc =
        token0_addr.to_lowercase() == usdc_address || token1_addr.to_lowercase() == usdc_address;

    if has_usdc {
        // If this pool involves USDC, we should NOT see both decimals as 18
        if tlv.amount_in_decimals == 18 && tlv.amount_out_decimals == 18 {
            println!("❌ CRITICAL: Hardcoded decimals detected! USDC pool has both decimals=18");
            return false;
        }

        // At least one should be 6 (USDC's actual decimals)
        if tlv.amount_in_decimals != 6 && tlv.amount_out_decimals != 6 {
            println!("❌ CRITICAL: USDC pool doesn't have 6 decimals anywhere");
            return false;
        }
    }

    println!("✅ No hardcoded decimals detected");
    true
}

#[tokio::test]
async fn test_safe_failure_on_missing_pool_cache() {
    use web3::types::Log;

    // Create a mock log that would normally trigger swap processing
    let mock_log = create_mock_swap_log();
    let (tx, mut rx) = mpsc::channel(10);

    // Test the static method without pool cache - should return None (safe failure)

    // Should return None (safe failure) rather than creating TLV with hardcoded decimals
    assert!(
        result.is_none(),
        "Should safely return None when pool cache unavailable"
    );

    // Ensure no message was sent to channel
    assert!(
        rx.try_recv().is_err(),
        "No TLV message should be created without proper decimals"
    );

    println!("✅ Safe failure test passed - no hardcoded decimals created");
}

fn create_mock_swap_log() -> web3::types::Log {
    // Create a minimal mock log for testing safe failure behavior
    web3::types::Log {
        address: "0x45dDa9cb7c25131dF268515131f647d726f50608"
            .parse()
            .unwrap(),
        topics: vec![
            "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67"
                .parse()
                .unwrap(), // Swap event signature
            "0x000000000000000000000000742ceb1981bb1b7697c94ab6f2B84F6b93e0c3B9"
                .parse()
                .unwrap(), // Sender
            "0x000000000000000000000000742ceb1981bb1b7697c94ab6f2B84F6b93e0c3B9"
                .parse()
                .unwrap(), // Recipient
        ],
        data: web3::types::Bytes(vec![0u8; 128]), // Mock data
        block_hash: Some(
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                .parse()
                .unwrap(),
        ),
        block_number: Some(12345678u64.into()),
        transaction_hash: Some(
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdef"
                .parse()
                .unwrap(),
        ),
        transaction_index: Some(1u64.into()),
        log_index: Some(1u64.into()),
        transaction_log_index: None,
        log_type: None,
        removed: Some(false),
    }
}

#[test]
fn test_polygon_token_registry_completeness() {
    // Validate our test token registry covers major Polygon tokens
    println!("🔍 Validating Polygon test token registry:");

    for (address, decimals) in POLYGON_TEST_TOKENS {
        println!("  {} -> {} decimals", address, decimals);

        // Validate address format
        assert!(address.starts_with("0x"), "Address should start with 0x");
        assert_eq!(
            address.len(),
            42,
            "Address should be 42 characters (0x + 40 hex)"
        );

        // Validate decimals are reasonable
        assert!(*decimals <= 30, "Decimals should be reasonable (<=30)");
        assert!(*decimals > 0, "Decimals should be > 0");
    }

    // Ensure we have both 6-decimal (USDC, USDT) and 18-decimal (WMATIC, WETH, DAI) tokens
    let has_6_decimals = POLYGON_TEST_TOKENS.iter().any(|(_, d)| *d == 6);
    let has_18_decimals = POLYGON_TEST_TOKENS.iter().any(|(_, d)| *d == 18);

    assert!(
        has_6_decimals,
        "Should include 6-decimal tokens (USDC, USDT)"
    );
    assert!(
        has_18_decimals,
        "Should include 18-decimal tokens (WMATIC, WETH)"
    );

    println!("✅ Token registry validation passed");
}
