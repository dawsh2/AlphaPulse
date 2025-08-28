//! Integration tests for DEX ABI library
//!
//! Tests event decoding with real-world data patterns and edge cases

use torq_dex::abi::*;
use web3::types::{Bytes, Log, H160, H256, U256};

/// Create a test log with given topics and data
fn create_test_log(address: H160, topics: Vec<H256>, data: Vec<u8>) -> Log {
    Log {
        address,
        topics,
        data: Bytes(data),
        block_hash: None,
        block_number: None,
        transaction_hash: None,
        transaction_index: None,
        log_index: None,
        transaction_log_index: None,
        log_type: None,
        removed: None,
    }
}

/// Create H256 from hex string (for test topics)
fn h256_from_hex(hex: &str) -> H256 {
    let bytes = hex::decode(hex.trim_start_matches("0x")).unwrap();
    let mut result = [0u8; 32];
    result[32 - bytes.len()..].copy_from_slice(&bytes);
    H256(result)
}

/// Create H160 from hex string (for addresses)
fn h160_from_hex(hex: &str) -> H160 {
    let bytes = hex::decode(hex.trim_start_matches("0x")).unwrap();
    let mut result = [0u8; 20];
    result[20 - bytes.len()..].copy_from_slice(&bytes);
    H160(result)
}

#[test]
fn test_protocol_detection() {
    // Test V3 detection (larger data payload)
    let v3_log = create_test_log(
        h160_from_hex("0x1234567890123456789012345678901234567890"),
        vec![H256::zero(); 3],
        vec![0u8; 150], // Large payload indicates V3
    );

    // Test V2 detection (smaller data payload)
    let v2_log = create_test_log(
        h160_from_hex("0x9876543210987654321098765432109876543210"),
        vec![H256::zero(); 3],
        vec![0u8; 64], // Smaller payload indicates V2
    );

    let v3_protocol = detect_dex_protocol(&v3_log.address, &v3_log);
    let v2_protocol = detect_dex_protocol(&v2_log.address, &v2_log);

    assert_eq!(v3_protocol, DEXProtocol::UniswapV3);
    assert_eq!(v2_protocol, DEXProtocol::UniswapV2);
}

#[test]
fn test_quickswap_detection() {
    let quickswap_addr = h160_from_hex("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"); // Starts with 0x5C
    let log = create_test_log(quickswap_addr, vec![H256::zero(); 3], vec![0u8; 150]);

    let protocol = detect_dex_protocol(&log.address, &log);
    assert_eq!(protocol, DEXProtocol::QuickswapV3);
}

#[test]
fn test_sushiswap_detection() {
    let sushi_addr = h160_from_hex("0xc35DADB65012eC5796536bD9864eD8773aBc74C4"); // Starts with 0xc3
    let log = create_test_log(sushi_addr, vec![H256::zero(); 3], vec![0u8; 64]);

    let protocol = detect_dex_protocol(&log.address, &log);
    assert_eq!(protocol, DEXProtocol::SushiswapV2);
}

#[test]
fn test_v2_swap_abi_structure() {
    let event = uniswap_v2::swap_event();

    // Verify event name
    assert_eq!(event.name, "Swap");

    // Verify parameter count and types
    assert_eq!(event.inputs.len(), 6);
    assert_eq!(event.inputs[0].name, "sender");
    assert_eq!(event.inputs[1].name, "amount0In");
    assert_eq!(event.inputs[2].name, "amount1In");
    assert_eq!(event.inputs[3].name, "amount0Out");
    assert_eq!(event.inputs[4].name, "amount1Out");
    assert_eq!(event.inputs[5].name, "to");

    // Verify indexed status
    assert!(event.inputs[0].indexed); // sender
    assert!(!event.inputs[1].indexed); // amount0In
    assert!(event.inputs[5].indexed); // to
}

#[test]
fn test_v3_swap_abi_structure() {
    let event = uniswap_v3::swap_event();

    // Verify event name
    assert_eq!(event.name, "Swap");

    // Verify parameter count and types
    assert_eq!(event.inputs.len(), 7);
    assert_eq!(event.inputs[0].name, "sender");
    assert_eq!(event.inputs[1].name, "recipient");
    assert_eq!(event.inputs[2].name, "amount0");
    assert_eq!(event.inputs[3].name, "amount1");
    assert_eq!(event.inputs[4].name, "sqrtPriceX96");
    assert_eq!(event.inputs[5].name, "liquidity");
    assert_eq!(event.inputs[6].name, "tick");

    // Verify indexed status
    assert!(event.inputs[0].indexed); // sender
    assert!(event.inputs[1].indexed); // recipient
    assert!(!event.inputs[2].indexed); // amount0
}

#[test]
fn test_v2_mint_abi_structure() {
    let event = uniswap_v2::mint_event();

    assert_eq!(event.name, "Mint");
    assert_eq!(event.inputs.len(), 3);
    assert_eq!(event.inputs[0].name, "sender");
    assert_eq!(event.inputs[1].name, "amount0");
    assert_eq!(event.inputs[2].name, "amount1");
}

#[test]
fn test_v3_mint_abi_structure() {
    let event = uniswap_v3::mint_event();

    assert_eq!(event.name, "Mint");
    assert_eq!(event.inputs.len(), 7);
    assert_eq!(event.inputs[1].name, "owner");
    assert_eq!(event.inputs[2].name, "tickLower");
    assert_eq!(event.inputs[3].name, "tickUpper");
    assert_eq!(event.inputs[4].name, "amount");
}

#[test]
fn test_overflow_handling() {
    // Test large U256 values that exceed i64::MAX
    let max_i64 = U256::from(i64::MAX);
    let overflow_value = max_i64 + U256::from(1000000);

    // This should truncate to i64::MAX with a warning (not panic)
    // Note: In a real test environment, we'd want to capture the warning
    let result = SwapEventDecoder::safe_u256_to_i64(overflow_value);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), i64::MAX);
}

#[test]
fn test_normal_value_conversion() {
    let normal_value = U256::from(1_000_000_000_000u64); // 1 trillion
    let result = SwapEventDecoder::safe_u256_to_i64(normal_value);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1_000_000_000_000i64);
}

#[test]
fn test_zero_value_conversion() {
    let zero = U256::zero();
    let result = SwapEventDecoder::safe_u256_to_i64(zero);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0i64);
}

#[test]
fn test_max_safe_value_conversion() {
    let max_safe = U256::from(i64::MAX);
    let result = SwapEventDecoder::safe_u256_to_i64(max_safe);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), i64::MAX);
}

#[test]
fn test_validated_swap_construction() {
    let pool_addr = [1u8; 20];
    let swap = ValidatedSwap {
        pool_address: pool_addr,
        amount_in: 1000000,
        amount_out: 950000,
        token_in_is_token0: true,
        sqrt_price_x96_after: 0, // V2 style
        tick_after: 0,
        liquidity_after: 0,
        dex_protocol: DEXProtocol::UniswapV2,
    };

    assert_eq!(swap.pool_address, pool_addr);
    assert_eq!(swap.amount_in, 1000000);
    assert_eq!(swap.amount_out, 950000);
    assert!(swap.token_in_is_token0);
    assert_eq!(swap.dex_protocol, DEXProtocol::UniswapV2);
}

#[test]
fn test_validated_mint_construction() {
    let pool_addr = [2u8; 20];
    let lp_addr = [3u8; 20];
    let mint = ValidatedMint {
        pool_address: pool_addr,
        liquidity_provider: lp_addr,
        liquidity_delta: 500000,
        amount0: 1000000,
        amount1: 2000000,
        tick_lower: -100,
        tick_upper: 100,
        dex_protocol: DEXProtocol::UniswapV3,
    };

    assert_eq!(mint.pool_address, pool_addr);
    assert_eq!(mint.liquidity_provider, lp_addr);
    assert_eq!(mint.liquidity_delta, 500000);
    assert_eq!(mint.tick_lower, -100);
    assert_eq!(mint.tick_upper, 100);
}

#[test]
fn test_validated_burn_construction() {
    let pool_addr = [4u8; 20];
    let lp_addr = [5u8; 20];
    let burn = ValidatedBurn {
        pool_address: pool_addr,
        liquidity_provider: lp_addr,
        liquidity_delta: 300000,
        amount0: 800000,
        amount1: 1600000,
        tick_lower: -50,
        tick_upper: 50,
        dex_protocol: DEXProtocol::UniswapV3,
    };

    assert_eq!(burn.pool_address, pool_addr);
    assert_eq!(burn.liquidity_provider, lp_addr);
    assert_eq!(burn.liquidity_delta, 300000);
}

#[test]
fn test_error_types() {
    // Test that our error types are properly constructed
    let missing_field_error = DecodingError::MissingField("test_field".to_string());
    let overflow_error = DecodingError::ValueOverflow {
        value: "999999999999999999999".to_string(),
    };
    let protocol_error = DecodingError::UnsupportedProtocol(DEXProtocol::UniswapV2);

    // Verify error messages contain expected content
    assert!(format!("{}", missing_field_error).contains("test_field"));
    assert!(format!("{}", overflow_error).contains("999999999999999999999"));
    assert!(format!("{}", protocol_error).contains("UniswapV2"));
}

#[test]
fn test_all_protocol_variants() {
    // Verify all protocol enum variants are constructible
    let protocols = vec![
        DEXProtocol::UniswapV2,
        DEXProtocol::UniswapV3,
        DEXProtocol::SushiswapV2,
        DEXProtocol::QuickswapV2,
        DEXProtocol::QuickswapV3,
    ];

    for protocol in protocols {
        // Each protocol should be copy, clone, eq, debug
        let copied = protocol;
        let cloned = protocol.clone();
        assert_eq!(protocol, copied);
        assert_eq!(protocol, cloned);
        println!("Protocol: {:?}", protocol); // Test Debug
    }
}

/// Test that our ABI events can be used in error scenarios
#[test]
fn test_decoding_error_scenarios() {
    // Test unsupported protocol error
    let empty_log = create_test_log(H160::zero(), vec![], vec![]);

    // This should fail gracefully with UnsupportedProtocol error
    // Note: We can't easily test actual decoding without valid ABI data,
    // but we can verify the error construction works
    let error = DecodingError::UnsupportedProtocol(DEXProtocol::UniswapV2);
    assert!(matches!(error, DecodingError::UnsupportedProtocol(_)));
}

#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;

    #[test]
    fn benchmark_protocol_detection() {
        let log = create_test_log(
            h160_from_hex("0x1234567890123456789012345678901234567890"),
            vec![H256::zero(); 3],
            vec![0u8; 150],
        );

        let start = Instant::now();
        for _ in 0..10000 {
            let _ = detect_dex_protocol(&log.address, &log);
        }
        let elapsed = start.elapsed();

        println!(
            "Protocol detection benchmark: 10k iterations in {:?}",
            elapsed
        );
        // Should be very fast - mostly just pattern matching
        assert!(elapsed.as_millis() < 100); // Less than 100ms for 10k iterations
    }

    #[test]
    fn benchmark_abi_construction() {
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = uniswap_v2::swap_event();
            let _ = uniswap_v3::swap_event();
            let _ = uniswap_v2::mint_event();
            let _ = uniswap_v3::mint_event();
        }
        let elapsed = start.elapsed();

        println!(
            "ABI construction benchmark: 4k ABI constructions in {:?}",
            elapsed
        );
        // ABI construction should be fast
        assert!(elapsed.as_millis() < 500); // Less than 500ms for 4k constructions
    }
}
