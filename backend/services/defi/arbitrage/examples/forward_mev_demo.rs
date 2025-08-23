use std::time::{SystemTime, UNIX_EPOCH};

// Copy the essential structures for demo
use arbitrage::mev_protection::{ForwardLookingMevProtection, MevDecision, MarketRegime};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Forward-Looking MEV Protection Demo\n");

    // Initialize the system
    let mut mev_protection = ForwardLookingMevProtection::new();
    
    // Simulate some block updates to populate market signals
    println!("📊 Updating from recent blocks...");
    for block in 12345670..12345675 {
        mev_protection.update_from_block(block).await?;
    }

    let market_signals = &mev_protection.live_signals;
    println!("Current market conditions:");
    println!("  Gas: {:.1} gwei", market_signals.current_gas_gwei);
    println!("  Regime: {:?}", market_signals.market_regime);
    println!("  MEV density: {:.1} txs/block", market_signals.recent_mev_density);
    println!("  Block fullness: {:.1}%", market_signals.block_fullness * 100.0);
    println!();

    // Test various arbitrage scenarios
    let scenarios = vec![
        ("Small profit, simple path", 8.0, 2, 150),
        ("Medium profit, medium complexity", 25.0, 3, 120),
        ("Large profit, complex path", 75.0, 6, 180),
        ("Huge profit, simple path", 200.0, 2, 100),
        ("Tiny profit, complex path", 5.0, 4, 200),
    ];

    println!("🎯 Testing MEV protection decisions:\n");
    for (description, profit_usd, path_complexity, execution_speed_ms) in scenarios {
        let start = SystemTime::now();
        let tx_timestamp_ns = start.duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
        
        let decision = mev_protection.should_use_protection(
            profit_usd,
            path_complexity,
            execution_speed_ms,
            tx_timestamp_ns,
        );

        let decision_time = start.elapsed().unwrap();
        
        println!("Scenario: {}", description);
        println!("  Profit: ${:.0}, Complexity: {} hops, Speed: {}ms", 
                 profit_usd, path_complexity, execution_speed_ms);
        println!("  Decision: {} (threat: {:.3})", 
                 if decision.use_protection { "🛡️  PROTECT" } else { "⚡ PUBLIC" },
                 decision.threat_score);
        println!("  Advantages: break_even={:.3}, speed={:.3}, complexity={:.3}",
                 decision.break_even_advantage, decision.speed_advantage, decision.complexity_advantage);
        println!("  Decision time: {:.2}ms", decision_time.as_nanos() as f64 / 1_000_000.0);
        println!("  Reasoning: {}", decision.reasoning);
        println!();
    }

    // Demonstrate regime-specific behavior
    println!("🌊 Testing different market regimes:\n");
    
    // Simulate high gas scenario
    println!("High Gas Scenario (80 gwei):");
    mev_protection.live_signals.current_gas_gwei = 80.0;
    mev_protection.live_signals.market_regime = MarketRegime::HighGas;
    
    let decision = mev_protection.should_use_protection(30.0, 3, 150, 
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64);
    println!("  $30 profit → {} (threat: {:.3})", 
             if decision.use_protection { "PROTECT" } else { "PUBLIC" }, decision.threat_score);

    // Simulate low gas scenario  
    println!("Low Gas Scenario (15 gwei):");
    mev_protection.live_signals.current_gas_gwei = 15.0;
    mev_protection.live_signals.market_regime = MarketRegime::LowGas;
    
    let decision = mev_protection.should_use_protection(30.0, 3, 150,
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64);
    println!("  $30 profit → {} (threat: {:.3})", 
             if decision.use_protection { "PROTECT" } else { "PUBLIC" }, decision.threat_score);

    // Simulate volatile gas scenario
    println!("Volatile Gas Scenario:");
    mev_protection.live_signals.current_gas_gwei = 45.0;
    mev_protection.live_signals.market_regime = MarketRegime::Volatile;
    mev_protection.live_signals.recent_mev_density = 5.0; // High MEV competition
    
    let decision = mev_protection.should_use_protection(30.0, 3, 150,
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64);
    println!("  $30 profit → {} (threat: {:.3})", 
             if decision.use_protection { "PROTECT" } else { "PUBLIC" }, decision.threat_score);
    println!();

    // Demonstrate Huff advantage
    println!("⚡ Huff Efficiency Advantage Demo:\n");
    let break_even_huff = mev_protection.calculate_our_break_even();
    let break_even_mev = mev_protection.calculate_current_mev_break_even();
    
    println!("Break-even comparison:");
    println!("  Our break-even (Huff): ${:.2}", break_even_huff);
    println!("  MEV bot break-even: ${:.2}", break_even_mev);
    println!("  Our advantage: {:.1}x more efficient", break_even_mev / break_even_huff);
    println!();

    // Performance benchmark
    println!("⚡ Performance Benchmark:\n");
    let mut total_time = std::time::Duration::new(0, 0);
    let iterations = 1000;
    
    for _ in 0..iterations {
        let start = SystemTime::now();
        let _ = mev_protection.should_use_protection(50.0, 3, 150,
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64);
        total_time += start.elapsed().unwrap();
    }
    
    let avg_time_us = total_time.as_nanos() as f64 / iterations as f64 / 1000.0;
    println!("Average decision time: {:.1}μs ({} iterations)", avg_time_us, iterations);
    println!("Target: <1000μs (1ms) ✅");
    
    if avg_time_us < 1000.0 {
        println!("🎉 Performance requirement MET!");
    } else {
        println!("❌ Performance requirement FAILED!");
    }

    Ok(())
}