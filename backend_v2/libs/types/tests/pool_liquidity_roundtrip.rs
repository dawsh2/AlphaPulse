//! PoolLiquidityTLV Roundtrip Validation Test
//!
//! Tests the complete serialization/deserialization cycle for pool liquidity data
//! with realistic DEX pool configurations and token reserves.

use alphapulse_types::protocol::{PoolInstrumentId, PoolLiquidityTLV, TLVType, VenueId};

/// Test PoolLiquidityTLV roundtrip validation with realistic DEX data
#[tokio::test]
async fn test_pool_liquidity_tlv_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing PoolLiquidityTLV roundtrip validation");

    // Realistic Polygon token addresses (first 8 bytes)
    let usdc_polygon = 0x2791bca1f2de4661u64; // USDC on Polygon
    let weth_polygon = 0x7ceb23fd6c244eb4u64; // WETH on Polygon
    let wmatic = 0x0d500b1d8e8ef31eu64; // WMATIC

    // Test Case 1: Two-token pool (USDC/WETH)
    println!("📊 Testing two-token pool liquidity...");
    let pool_id_2token = PoolInstrumentId::from_pair(VenueId::Polygon, usdc_polygon, weth_polygon);

    let liquidity_2token = PoolLiquidityTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id_2token.clone(),
        reserves: vec![
            1500000000000000u64 as i64, // 15M USDC (6 decimals = 15,000,000 * 10^8 for 8-decimal format)
            5000000000000000u64 as i64, // 50 WETH (18 decimals = 50 * 10^8 for 8-decimal format)
        ],
        total_supply: 27386127875461u64 as i64, // LP token supply in 8-decimal format
        fee_rate: 30,                           // 0.3% fee (30 basis points)
        timestamp_ns: 1755754000000000000u64,   // Current timestamp in nanoseconds
    };

    // Perform roundtrip validation
    test_liquidity_roundtrip(&liquidity_2token, "Two-token USDC/WETH pool").await?;

    // Test Case 2: Three-token pool (USDC/WETH/WMATIC triangular)
    println!("📊 Testing three-token triangular pool liquidity...");
    let pool_id_3token =
        PoolInstrumentId::from_triple(VenueId::Polygon, usdc_polygon, weth_polygon, wmatic);

    let liquidity_3token = PoolLiquidityTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id_3token.clone(),
        reserves: vec![
            800000000000000u64 as i64,   // 8M USDC
            2500000000000000u64 as i64,  // 25 WETH
            12000000000000000u64 as i64, // 120K WMATIC
        ],
        total_supply: 35000000000000u64 as i64, // LP token supply
        fee_rate: 50,                           // 0.5% fee for triangular pool
        timestamp_ns: 1755754100000000000u64,
    };

    test_liquidity_roundtrip(&liquidity_3token, "Three-token triangular pool").await?;

    // Test Case 3: Edge case - single token (shouldn't happen but test robustness)
    println!("📊 Testing edge case: minimal reserves...");
    let pool_id_edge = PoolInstrumentId::from_pair(VenueId::Polygon, usdc_polygon, weth_polygon);

    let liquidity_edge = PoolLiquidityTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id_edge.clone(),
        reserves: vec![
            1u64 as i64, // 1 wei equivalent in 8-decimal format
            1u64 as i64, // 1 wei equivalent in 8-decimal format
        ],
        total_supply: 0i64, // Empty pool
        fee_rate: 100,      // 1% fee
        timestamp_ns: 1755754200000000000u64,
    };

    test_liquidity_roundtrip(&liquidity_edge, "Edge case minimal reserves").await?;

    // Test Case 4: Large pool with many tokens (stress test)
    println!("📊 Testing large multi-token pool...");
    let tokens: Vec<u64> = (0..8).map(|i| 0x1000000000000000u64 + i).collect();
    let pool_id_large = PoolInstrumentId::new(VenueId::Polygon, &tokens);

    let large_reserves: Vec<i64> = (0..8).map(|i| (1000 + i * 500) * 100000000i64).collect(); // Varying reserves

    let liquidity_large = PoolLiquidityTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id_large.clone(),
        reserves: large_reserves,
        total_supply: 50000000000000i64,
        fee_rate: 25, // 0.25% fee
        timestamp_ns: 1755754300000000000u64,
    };

    test_liquidity_roundtrip(&liquidity_large, "Large 8-token pool").await?;

    println!("✅ All PoolLiquidityTLV roundtrip tests passed!");
    Ok(())
}

/// Test a single PoolLiquidityTLV roundtrip
async fn test_liquidity_roundtrip(
    original: &PoolLiquidityTLV,
    description: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("  🔄 Testing {}", description);

    // Step 1: Serialize to bytes
    let serialized = original.to_bytes();
    println!("    📦 Serialized to {} bytes", serialized.len());

    // Step 2: Deserialize back
    let recovered = PoolLiquidityTLV::from_bytes(&serialized)
        .map_err(|e| format!("Deserialization failed: {}", e))?;

    // Step 3: Deep equality check
    if original != &recovered {
        return Err(format!(
            "Roundtrip failed for {}!\nOriginal: {:?}\nRecovered: {:?}",
            description, original, recovered
        )
        .into());
    }

    // Step 4: Verify pool bijection works
    let original_tokens = original.pool_id.get_tokens();
    let recovered_tokens = recovered.pool_id.get_tokens();

    if original_tokens != recovered_tokens {
        return Err(format!(
            "Pool bijection failed for {}!\nOriginal tokens: {:?}\nRecovered tokens: {:?}",
            description, original_tokens, recovered_tokens
        )
        .into());
    }

    // Step 5: Verify reserve counts match
    if original.reserves.len() != recovered.reserves.len() {
        return Err(format!(
            "Reserve count mismatch for {}! Original: {}, Recovered: {}",
            description,
            original.reserves.len(),
            recovered.reserves.len()
        )
        .into());
    }

    // Step 6: Verify individual reserves
    for (i, (orig, recv)) in original
        .reserves
        .iter()
        .zip(recovered.reserves.iter())
        .enumerate()
    {
        if orig != recv {
            return Err(format!(
                "Reserve {} mismatch for {}! Original: {}, Recovered: {}",
                i, description, orig, recv
            )
            .into());
        }
    }

    println!(
        "    ✅ Perfect roundtrip: {} tokens, {} reserves, {} bytes",
        recovered_tokens.len(),
        recovered.reserves.len(),
        serialized.len()
    );

    Ok(())
}

/// Test TLV message roundtrip (with header and checksum)
#[tokio::test]
async fn test_pool_liquidity_tlv_message_roundtrip() {
    println!("🔍 Testing PoolLiquidityTLV message roundtrip (with TLV header)");

    let usdc = 0x2791bca1f2de4661u64;
    let weth = 0x7ceb23fd6c244eb4u64;
    let pool_id = PoolInstrumentId::from_pair(VenueId::Polygon, usdc, weth);

    let original_liquidity = PoolLiquidityTLV {
        venue: VenueId::Polygon,
        pool_id,
        reserves: vec![1000000000000000i64, 500000000000000i64],
        total_supply: 1500000000000000i64,
        fee_rate: 30,
        timestamp_ns: 1755754400000000000u64,
    };

    // Convert to TLV message (with header and checksum)
    let tlv_message = original_liquidity.to_tlv_message();

    println!(
        "📦 TLV Message: {} byte header + {} byte payload",
        std::mem::size_of_val(&tlv_message.header),
        tlv_message.payload.len()
    );

    // Verify header
    assert_eq!(tlv_message.header.magic, 0xDEADBEEF);
    assert_eq!(tlv_message.header.tlv_type, TLVType::PoolLiquidity);

    // Deserialize from payload
    let recovered_liquidity = PoolLiquidityTLV::from_bytes(&tlv_message.payload)
        .expect("Failed to deserialize TLV message payload");

    // Verify perfect equality
    assert_eq!(original_liquidity, recovered_liquidity);

    println!("✅ TLV message roundtrip successful with header validation");
}

/// Benchmark serialization performance
#[tokio::test]
async fn test_pool_liquidity_performance() {
    println!("⚡ Testing PoolLiquidityTLV serialization performance");

    let usdc = 0x2791bca1f2de4661u64;
    let weth = 0x7ceb23fd6c244eb4u64;
    let pool_id = PoolInstrumentId::from_pair(VenueId::Polygon, usdc, weth);

    let liquidity = PoolLiquidityTLV {
        venue: VenueId::Polygon,
        pool_id,
        reserves: vec![1000000000000000i64, 500000000000000i64],
        total_supply: 1500000000000000i64,
        fee_rate: 30,
        timestamp_ns: 1755754500000000000u64,
    };

    let iterations = 10000;
    let start = std::time::Instant::now();

    for _ in 0..iterations {
        let serialized = liquidity.to_bytes();
        let _recovered = PoolLiquidityTLV::from_bytes(&serialized).unwrap();
    }

    let elapsed = start.elapsed();
    let per_op = elapsed / iterations;

    println!("📊 Performance: {} iterations in {:?}", iterations, elapsed);
    println!(
        "⚡ Average: {:?} per roundtrip ({:.0} ops/sec)",
        per_op,
        1.0 / per_op.as_secs_f64()
    );

    // Should be very fast (< 10μs per operation for production use)
    assert!(
        per_op.as_nanos() < 10000,
        "Serialization too slow: {:?}",
        per_op
    );

    println!("✅ Performance test passed - fast enough for production");
}
