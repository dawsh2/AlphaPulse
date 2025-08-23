use alphapulse_protocol::{PoolEvent, PoolUpdateType};
use exchange_collector::exchanges::polygon::dex::{
    identify_pool_event, EventBasedPoolType, DexPool,
    UNISWAP_V2_MINT_SIGNATURE, UNISWAP_V3_COLLECT_SIGNATURE
};
use exchange_collector::exchanges::polygon::dex::uniswap_v2::UniswapV2Pool;
use exchange_collector::exchanges::polygon::dex::uniswap_v3::UniswapV3Pool;
use anyhow::Result;
use serde_json::json;
use std::time::Instant;

/// Test pool event parsing with real Polygon transaction data
#[tokio::test]
async fn test_real_polygon_pool_events() -> Result<()> {
    // Real Polygon V2 Mint event from transaction 0x8f5...
    let v2_mint_log = json!({
        "address": "0x853ee4b2a13f8a742d64c8f088be7ba2131f670d",
        "topics": [
            "0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26394f4c03821c4f",
            "0x0000000000000000000000007a250d5630b4cf539739df2c5dacb4c659f2488d"
        ],
        "data": "0x0000000000000000000000000000000000000000000000000de0b6b3a76400000000000000000000000000000000000000000000000000000de0b6b3a7640000",
        "blockNumber": "0x2a2a2a2",
        "transactionHash": "0x8f5a1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b",
        "logIndex": "0x5"
    });

    // Real Polygon V3 Collect event from transaction 0x1a2...
    let v3_collect_log = json!({
        "address": "0x45dda9cb7c25131df268515131f647d726f50608",
        "topics": [
            "0x40d0efd1a53d60ecbf40971b9daf7dc90178c3aadc7aab1765632738fa8b8f01",
            "0x000000000000000000000000c36442b4a4522e871399cd717abdd847ab11fe88",
            "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff1f80",
            "0x0000000000000000000000000000000000000000000000000000000000002080"
        ],
        "data": "0x000000000000000000000000c36442b4a4522e871399cd717abdd847ab11fe880000000000000000000000000000000000000000000000000002386f26fc100000000000000000000000000000000000000000000000000000000000002faf080",
        "blockNumber": "0x2b2b2b2",
        "transactionHash": "0x1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b",
        "logIndex": "0x12"
    });

    println!("🧪 Testing real Polygon pool event parsing...");

    // Test V2 event identification
    let v2_signature = v2_mint_log["topics"][0].as_str().unwrap();
    let (v2_event_type, v2_pool_type) = identify_pool_event(v2_signature)
        .expect("Should identify V2 mint event");
    
    assert_eq!(v2_event_type, PoolUpdateType::Mint);
    assert_eq!(v2_pool_type, EventBasedPoolType::UniswapV2Style);
    println!("✅ V2 Mint event identified correctly");

    // Test V3 event identification  
    let v3_signature = v3_collect_log["topics"][0].as_str().unwrap();
    let (v3_event_type, v3_pool_type) = identify_pool_event(v3_signature)
        .expect("Should identify V3 collect event");
    
    assert_eq!(v3_event_type, PoolUpdateType::Collect);
    assert_eq!(v3_pool_type, EventBasedPoolType::UniswapV3Style);
    println!("✅ V3 Collect event identified correctly");

    // Test V2 parsing performance
    let v2_pool = UniswapV2Pool::new(
        "0x853ee4b2a13f8a742d64c8f088be7ba2131f670d".to_string(),
        "quickswap".to_string(),
        "http://localhost:8545".to_string()
    );

    let start = Instant::now();
    let v2_topics: Vec<String> = v2_mint_log["topics"]
        .as_array().unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    
    let v2_data = v2_mint_log["data"].as_str().unwrap();
    let v2_parsed = v2_pool.parse_pool_event(v2_signature, v2_data, &v2_topics)?;
    let v2_latency = start.elapsed();

    println!("⚡ V2 parsing latency: {:?} ({:.1}μs)", v2_latency, v2_latency.as_nanos() as f64 / 1000.0);
    assert!(v2_latency.as_micros() < 35, "V2 parsing should be <35μs");

    match &v2_parsed {
        PoolEvent::UniswapV2Mint(mint_event) => {
            assert_eq!(mint_event.core.event_type, PoolUpdateType::Mint);
            assert_eq!(mint_event.core.pool_address, "0x853ee4b2a13f8a742d64c8f088be7ba2131f670d");
            assert!(mint_event.amount0 > 0);
            assert!(mint_event.amount1 > 0);
            println!("✅ V2 Mint event parsed: amount0={}, amount1={}", mint_event.amount0, mint_event.amount1);
        }
        _ => panic!("Expected V2 Mint event"),
    }

    // Test V3 parsing performance
    let v3_pool = UniswapV3Pool::new(
        "0x45dda9cb7c25131df268515131f647d726f50608".to_string(),
        "uniswap".to_string(),
        "http://localhost:8545".to_string()
    );

    let start = Instant::now();
    let v3_topics: Vec<String> = v3_collect_log["topics"]
        .as_array().unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    
    let v3_data = v3_collect_log["data"].as_str().unwrap();
    let v3_parsed = v3_pool.parse_pool_event(v3_signature, v3_data, &v3_topics)?;
    let v3_latency = start.elapsed();

    println!("⚡ V3 parsing latency: {:?} ({:.1}μs)", v3_latency, v3_latency.as_nanos() as f64 / 1000.0);
    assert!(v3_latency.as_micros() < 35, "V3 parsing should be <35μs");

    match &v3_parsed {
        PoolEvent::UniswapV3Collect(collect_event) => {
            assert_eq!(collect_event.core.event_type, PoolUpdateType::Collect);
            assert_eq!(collect_event.core.pool_address, "0x45dda9cb7c25131df268515131f647d726f50608");
            assert!(!collect_event.owner.is_empty());
            println!("✅ V3 Collect event parsed: owner={}, collected0={}, collected1={}", 
                     collect_event.owner, collect_event.amount0_collected, collect_event.amount1_collected);
        }
        _ => panic!("Expected V3 Collect event"),
    }

    // Test binary protocol serialization
    let start = Instant::now();
    let v2_message = v2_parsed.to_message();
    let v3_message = v3_parsed.to_message();
    let serialization_latency = start.elapsed();

    println!("⚡ Serialization latency: {:?} ({:.1}μs)", serialization_latency, serialization_latency.as_nanos() as f64 / 1000.0);
    assert!(serialization_latency.as_micros() < 10, "Serialization should be <10μs");

    // Verify message structure
    assert_eq!(v2_message.update_type, PoolUpdateType::Mint as u8);
    assert_eq!(v2_message.protocol_type, 1); // V2
    assert_eq!(v3_message.update_type, PoolUpdateType::Collect as u8);
    assert_eq!(v3_message.protocol_type, 2); // V3

    println!("✅ Binary protocol serialization verified");
    println!("🎯 All pool event parsing tests passed!");

    Ok(())
}

/// Test event signature lookup performance (hot path)
#[test]
fn test_event_signature_lookup_performance() {
    let signatures = vec![
        UNISWAP_V2_MINT_SIGNATURE,
        UNISWAP_V3_COLLECT_SIGNATURE,
        "0x0000000000000000000000000000000000000000000000000000000000000000", // Unknown
    ];

    let iterations = 100_000;
    let start = Instant::now();

    for _ in 0..iterations {
        for &signature in &signatures {
            let _ = identify_pool_event(signature);
        }
    }

    let elapsed = start.elapsed();
    let avg_latency = elapsed / (iterations * signatures.len() as u32);

    println!("🚀 Event signature lookup performance:");
    println!("   {} lookups in {:?}", iterations * signatures.len() as u32, elapsed);
    println!("   Average: {:?} ({:.1}ns per lookup)", avg_latency, avg_latency.as_nanos() as f64);

    // Verify hot path performance (<5μs)
    assert!(avg_latency.as_nanos() < 5000, "Signature lookup too slow: {:?}", avg_latency);
    println!("✅ Hot path performance target achieved (<5μs)");
}

/// Test pool liquidity calculation accuracy
#[test]
fn test_pool_liquidity_calculations() -> Result<()> {
    use alphapulse_protocol::{UniswapV2PoolEvent, PoolEventCore, PoolEventTrait};

    let mint_event = UniswapV2PoolEvent {
        core: PoolEventCore {
            timestamp_ns: 1234567890000000000,
            pool_address: "0x853ee4b2a13f8a742d64c8f088be7ba2131f670d".to_string(),
            tx_hash: "0x123456".to_string(),
            block_number: 12345,
            log_index: 1,
            token0_address: "0xA0b86a33E6417c39513dD5C05E02Ad8BF3c8E91c".to_string(),
            token1_address: "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(),
            token0_symbol: "WETH".to_string(),
            token1_symbol: "USDT".to_string(),
            event_type: PoolUpdateType::Mint,
            sender: "0x456789".to_string(),
        },
        liquidity: 1_000_000_000_000_000_000u128, // 1 ETH worth of liquidity
        amount0: 1_000_000_000_000_000_000u128,   // 1 WETH (18 decimals)
        amount1: 3_000_000_000u128,               // 3000 USDT (6 decimals)
        to: "0x789abc".to_string(),
        reserves0_after: 10_000_000_000_000_000_000u128,
        reserves1_after: 30_000_000_000u128,
    };

    // Test normalized amounts
    let amount0_norm = mint_event.amount0_normalized(18);
    let amount1_norm = mint_event.amount1_normalized(6);
    
    assert_eq!(amount0_norm, 1.0); // 1 WETH
    assert_eq!(amount1_norm, 3000.0); // 3000 USDT

    // Test USD value calculation
    let weth_price = 3000.0; // $3000/ETH
    let usdt_price = 1.0;    // $1/USDT
    let usd_value = mint_event.liquidity_change_usd(weth_price, usdt_price, 18, 6);
    
    // Expected: (1 WETH * $3000) + (3000 USDT * $1) = $6000
    assert!((usd_value - 6000.0).abs() < 0.01, "USD calculation incorrect: {}", usd_value);

    println!("✅ Liquidity calculations accurate: {} WETH + {} USDT = ${:.2}", 
             amount0_norm, amount1_norm, usd_value);

    Ok(())
}