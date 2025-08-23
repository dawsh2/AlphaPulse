//! Precision Boundary Test
//!
//! Tests extreme values that could cause precision loss or overflow:
//! - Maximum/minimum i64 values
//! - Borderline precision cases
//! - Edge cases that might cause floating point errors
//! - Real-world extreme market conditions

use alphapulse_protocol_v2::{VenueId, TradeTLV, PoolSwapTLV, PoolInstrumentId, InstrumentId};

/// Test extreme precision and boundary conditions
#[tokio::test] 
async fn test_extreme_precision_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Testing extreme precision boundary conditions");
    
    let test_cases = vec![
        ("Maximum i64 value", i64::MAX),
        ("Minimum i64 value", i64::MIN),
        ("Zero value", 0i64),
        ("One wei equivalent", 1i64),
        ("Maximum Bitcoin supply (21M * 1e8)", 2100000000000000i64),
        ("Maximum Ethereum supply (120M * 1e8)", 12000000000000000i64),
        ("Largest realistic price ($1M * 1e8)", 100000000000000i64),
        ("Smallest realistic price (1 satoshi)", 1i64),
        ("Near overflow boundary", i64::MAX - 1000),
        ("Precision edge case", 999999999i64), // 9.99999999 in 8-decimal
    ];
    
    for (description, value) in test_cases {
        test_precision_roundtrip(description, value).await?;
    }
    
    // Test precision with real market scenarios
    test_real_market_scenarios().await?;
    
    println!("✅ All precision boundary tests passed!");
    Ok(())
}

/// Test a specific precision value through full roundtrip
async fn test_precision_roundtrip(description: &str, value: i64) -> Result<(), Box<dyn std::error::Error>> {
    println!("  🔍 Testing {}: {}", description, value);
    
    // Test with TradeTLV
    let trade = TradeTLV {
        venue: VenueId::Binance,
        instrument_id: InstrumentId::from_u64(0x1234567890ABCDEF),
        price: value,
        volume: if value == 0 { 1 } else { value.saturating_abs().min(1000000000) }, // Reasonable volume
        side: 0,
        timestamp_ns: 1700000000000000000,
    };
    
    let bytes = trade.to_bytes();
    let recovered = TradeTLV::from_bytes(&bytes)?;
    
    if trade != recovered {
        return Err(format!("TradeTLV precision lost for {}: {} != {}", 
                          description, trade.price, recovered.price).into());
    }
    
    // Specifically verify the critical value
    assert_eq!(trade.price, recovered.price, "Price precision lost for {}", description);
    assert_eq!(trade.volume, recovered.volume, "Volume precision lost for {}", description);
    
    // Test with PoolSwapTLV for variety
    let pool_id = PoolInstrumentId::from_pair(VenueId::Polygon, 0x1000, 0x2000);
    let swap = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id,
        token_in: 1,
        token_out: 2,
        amount_in: value,
        amount_out: if value == 0 { 1 } else { value.saturating_abs().min(1000000000) },
        fee_paid: if value == 0 { 0 } else { value.saturating_abs().min(100000) }, // Small fee
        sqrt_price_x96_after: 0,  // V2 pool
        tick_after: 0,
        liquidity_after: 0,
        timestamp_ns: 1700000000000000000,
        block_number: 1000,
    };
    
    let swap_bytes = swap.to_bytes();
    let recovered_swap = PoolSwapTLV::from_bytes(&swap_bytes)?;
    
    if swap != recovered_swap {
        return Err(format!("PoolSwapTLV precision lost for {}", description).into());
    }
    
    assert_eq!(swap.amount_in, recovered_swap.amount_in, "Swap amount_in precision lost for {}", description);
    
    println!("    ✅ {} preserved perfectly", description);
    Ok(())
}

/// Test real market scenarios with actual extreme values
async fn test_real_market_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    println!("  📊 Testing real market extreme scenarios");
    
    let scenarios = vec![
        ("GameStop squeeze ($483.00)", 48300000000i64), // $483 * 1e8
        ("Bitcoin ATH ($69,420.69)", 6942069000000i64), // $69,420.69 * 1e8  
        ("Micro penny stock ($0.0001)", 10000i64), // $0.0001 * 1e8
        ("Hyperinflation token (1 trillion supply)", 100000000000000000i64),
        ("DeFi flash loan (100M USDC)", 10000000000000000i64), // 100M * 1e8
        ("Minimal DEX swap (1 wei)", 1i64),
        ("Large institutional trade ($100M)", 10000000000000000i64),
        ("Stablecoin depeg ($0.9950)", 99500000i64), // $0.9950 * 1e8
    ];
    
    for (scenario, price) in scenarios {
        // Create realistic trade volume based on price
        let volume = match price {
            p if p > 1000000000000i64 => 100000000i64, // 1.0 token for expensive assets
            p if p > 100000000i64 => 1000000000i64, // 10.0 tokens for normal assets  
            _ => 10000000000i64, // 100.0 tokens for cheap assets
        };
        
        let trade = TradeTLV {
            venue: VenueId::Coinbase,
            instrument_id: InstrumentId::from_u64(0x1234567890ABCDEF + price as u64),
            price,
            volume,
            side: 0,
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos() as u64,
        };
        
        let bytes = trade.to_bytes();
        let recovered = TradeTLV::from_bytes(&bytes)?;
        
        assert_eq!(trade, recovered, "Real market scenario failed: {}", scenario);
        
        // Convert back to human readable to verify
        let human_price = recovered.price as f64 / 1e8;
        println!("    ✅ {} - Price: ${:.8}, Volume: {:.8}", 
                 scenario, human_price, recovered.volume as f64 / 1e8);
    }
    
    Ok(())
}

/// Test arithmetic operations that might cause overflow
#[tokio::test]
async fn test_arithmetic_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧮 Testing arithmetic boundary conditions");
    
    // Test operations that might overflow in real trading scenarios
    let large_price = 1000000000000i64; // $10,000 * 1e8
    let large_volume = 100000000000i64; // 1,000 tokens * 1e8
    
    // This multiplication would overflow if done naively
    // But our system should handle it properly
    let trade = TradeTLV {
        venue: VenueId::Binance,
        instrument_id: InstrumentId::from_u64(0x123456789ABCDEF0),
        price: large_price,
        volume: large_volume,
        side: 1,
        timestamp_ns: 1700000000000000000,
    };
    
    // Verify no precision loss in storage/retrieval
    let bytes = trade.to_bytes();
    let recovered = TradeTLV::from_bytes(&bytes)?;
    
    assert_eq!(trade.price, recovered.price, "Large price lost precision");
    assert_eq!(trade.volume, recovered.volume, "Large volume lost precision");
    
    // Test fee calculations that might be problematic
    let fee_basis_points = 30; // 0.3%
    let calculated_fee = (large_price.saturating_mul(large_volume) / 10000 / 100000000).min(i64::MAX);
    
    println!("  🧮 Large trade calculation test:");
    println!("    💰 Price: ${:.2}", large_price as f64 / 1e8);
    println!("    📊 Volume: {:.2}", large_volume as f64 / 1e8);
    println!("    💸 Calculated fee: ${:.2}", calculated_fee as f64 / 1e8);
    
    // Test with pool swap that has large amounts
    let pool_id = PoolInstrumentId::from_pair(VenueId::Polygon, 0x1000, 0x2000);
    let large_swap = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id,
        token_in: 1,
        token_out: 2,
        amount_in: large_volume,
        amount_out: large_volume.saturating_mul(large_price) / 1000000000000i64, // Price conversion
        fee_paid: calculated_fee,
        sqrt_price_x96_after: 0,  // V2 pool
        tick_after: 0,
        liquidity_after: 0,
        timestamp_ns: 1700000000000000000,
        block_number: 1000,
    };
    
    let swap_bytes = large_swap.to_bytes();
    let recovered_swap = PoolSwapTLV::from_bytes(&swap_bytes)?;
    
    assert_eq!(large_swap, recovered_swap, "Large swap lost precision");
    
    println!("✅ Arithmetic boundary tests passed!");
    Ok(())
}

/// Test timestamp boundary conditions
#[tokio::test]
async fn test_timestamp_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    println!("⏰ Testing timestamp boundary conditions");
    
    let timestamp_cases = vec![
        ("Unix epoch start", 0u64),
        ("Year 2000", 946684800000000000u64), // 2000-01-01 in nanoseconds
        ("Current time", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos() as u64),
        ("Year 2050", 2524608000000000000u64), // 2050-01-01 in nanoseconds
        ("Maximum u64", u64::MAX),
    ];
    
    for (description, timestamp) in timestamp_cases {
        let trade = TradeTLV {
            venue: VenueId::Kraken,
            instrument_id: InstrumentId::from_u64(0x1234567890ABCDEF),
            price: 5000000000000i64, // $50,000
            volume: 100000000i64,    // 1.0
            side: 0,
            timestamp_ns: timestamp,
        };
        
        let bytes = trade.to_bytes();
        let recovered = TradeTLV::from_bytes(&bytes)?;
        
        assert_eq!(trade.timestamp_ns, recovered.timestamp_ns, 
                   "Timestamp precision lost for {}", description);
        
        println!("  ✅ {} - Timestamp: {} ns", description, recovered.timestamp_ns);
    }
    
    println!("✅ Timestamp boundary tests passed!");
    Ok(())
}

