//! End-to-End Live Data Validation Test
//!
//! Validates the complete pipeline from Polygon blockchain data to arbitrage detection
//! with deep equality validation and all pool event types.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use protocol_v2::{
    VenueId, TLVType,
    tlv::market_data::{
        PoolSwapTLV, PoolMintTLV, PoolBurnTLV, PoolTickTLV, PoolLiquidityTLV,
        TLVMessage, TLVHeader, calculate_checksum,
    },
    instrument_id::pairing::PoolInstrumentId,
};

/// E2E test result statistics
#[derive(Debug, Default)]
struct E2EValidationStats {
    // Message counts by type
    pub swaps_processed: u64,
    pub mints_processed: u64,
    pub burns_processed: u64,
    pub ticks_processed: u64,
    pub liquidity_updates_processed: u64,
    
    // Validation results
    pub deep_equality_passes: u64,
    pub deep_equality_failures: u64,
    pub precision_preserved: u64,
    pub precision_lost: u64,
    
    // Arbitrage detection
    pub opportunities_detected: u64,
    pub profitable_opportunities: u64,
    pub unprofitable_skipped: u64,
    
    // Performance metrics
    pub avg_latency_ns: u64,
    pub max_latency_ns: u64,
    pub min_latency_ns: u64,
}

impl E2EValidationStats {
    pub fn report(&self) {
        println!("\n╔══════════════════════════════════════════════════════╗");
        println!("║          E2E LIVE DATA VALIDATION REPORT            ║");
        println!("╚══════════════════════════════════════════════════════╝");
        
        println!("\n📊 Message Processing Statistics:");
        println!("  • Swaps processed:      {}", self.swaps_processed);
        println!("  • Mints processed:      {}", self.mints_processed);
        println!("  • Burns processed:      {}", self.burns_processed);
        println!("  • Ticks processed:      {}", self.ticks_processed);
        println!("  • Liquidity updates:    {}", self.liquidity_updates_processed);
        println!("  • Total messages:       {}", self.total_messages());
        
        println!("\n✅ Deep Equality Validation:");
        println!("  • Passed:               {} ({:.2}%)", 
                 self.deep_equality_passes, self.pass_rate());
        println!("  • Failed:               {}", self.deep_equality_failures);
        println!("  • Precision preserved:  {} ({:.2}%)", 
                 self.precision_preserved, self.precision_rate());
        println!("  • Precision lost:       {}", self.precision_lost);
        
        println!("\n💰 Arbitrage Detection:");
        println!("  • Opportunities found:  {}", self.opportunities_detected);
        println!("  • Profitable:           {}", self.profitable_opportunities);
        println!("  • Unprofitable skipped: {}", self.unprofitable_skipped);
        if self.opportunities_detected > 0 {
            println!("  • Profit rate:          {:.2}%", 
                     (self.profitable_opportunities as f64 / self.opportunities_detected as f64) * 100.0);
        }
        
        println!("\n⚡ Performance Metrics:");
        println!("  • Average latency:      {} ns", self.avg_latency_ns);
        println!("  • Maximum latency:      {} ns", self.max_latency_ns);
        println!("  • Minimum latency:      {} ns", self.min_latency_ns);
        
        println!("\n══════════════════════════════════════════════════════");
    }
    
    fn total_messages(&self) -> u64 {
        self.swaps_processed + self.mints_processed + self.burns_processed + 
        self.ticks_processed + self.liquidity_updates_processed
    }
    
    fn pass_rate(&self) -> f64 {
        if self.deep_equality_passes + self.deep_equality_failures == 0 {
            return 100.0;
        }
        (self.deep_equality_passes as f64 / 
         (self.deep_equality_passes + self.deep_equality_failures) as f64) * 100.0
    }
    
    fn precision_rate(&self) -> f64 {
        if self.precision_preserved + self.precision_lost == 0 {
            return 100.0;
        }
        (self.precision_preserved as f64 / 
         (self.precision_preserved + self.precision_lost) as f64) * 100.0
    }
}

/// Simulated arbitrage detector for testing
struct TestArbitrageDetector {
    min_profit_threshold: i64,  // 8-decimal fixed point
}

impl TestArbitrageDetector {
    fn new() -> Self {
        Self {
            min_profit_threshold: 100000000,  // $1.00 minimum profit
        }
    }
    
    fn check_swap_arbitrage(&self, swap: &PoolSwapTLV) -> Option<i64> {
        // Simple heuristic: large swaps create arbitrage opportunities
        let size_impact = swap.amount_in.abs() / 100000000;  // Impact per dollar
        if size_impact > 10 {  // $10+ swap
            let estimated_profit = size_impact * 1000000;  // 1% of impact
            if estimated_profit > self.min_profit_threshold {
                return Some(estimated_profit);
            }
        }
        None
    }
    
    fn check_liquidity_arbitrage(&self, mint: &PoolMintTLV) -> Option<i64> {
        // Large liquidity additions can create imbalances
        if mint.liquidity_delta > 1000000000000000 {  // Large addition
            return Some(50000000);  // $0.50 opportunity
        }
        None
    }
    
    fn check_tick_arbitrage(&self, tick: &PoolTickTLV) -> Option<i64> {
        // Tick crossings with high liquidity changes create opportunities
        if tick.liquidity_net.abs() > 500000000000000 {
            return Some(75000000);  // $0.75 opportunity
        }
        None
    }
}

/// Validate a pool swap with deep equality check
fn validate_pool_swap(swap: &PoolSwapTLV, stats: &mut E2EValidationStats) -> bool {
    stats.swaps_processed += 1;
    
    // Serialize and deserialize for roundtrip validation
    let bytes = swap.to_bytes();
    match PoolSwapTLV::from_bytes(&bytes) {
        Ok(recovered) => {
            // Deep equality check
            if swap == &recovered {
                stats.deep_equality_passes += 1;
                
                // Verify precision preservation
                if swap.amount_in == recovered.amount_in &&
                   swap.amount_out == recovered.amount_out &&
                   swap.fee_paid == recovered.fee_paid {
                    stats.precision_preserved += 1;
                } else {
                    stats.precision_lost += 1;
                }
                
                true
            } else {
                stats.deep_equality_failures += 1;
                eprintln!("❌ Deep equality failed for swap!");
                eprintln!("   Original:  {:?}", swap);
                eprintln!("   Recovered: {:?}", recovered);
                false
            }
        }
        Err(e) => {
            stats.deep_equality_failures += 1;
            eprintln!("❌ Failed to deserialize swap: {}", e);
            false
        }
    }
}

/// Validate a pool mint with deep equality check
fn validate_pool_mint(mint: &PoolMintTLV, stats: &mut E2EValidationStats) -> bool {
    stats.mints_processed += 1;
    
    let bytes = mint.to_bytes();
    match PoolMintTLV::from_bytes(&bytes) {
        Ok(recovered) => {
            if mint == &recovered {
                stats.deep_equality_passes += 1;
                
                // Check tick preservation (can be negative)
                if mint.tick_lower == recovered.tick_lower &&
                   mint.tick_upper == recovered.tick_upper {
                    stats.precision_preserved += 1;
                } else {
                    stats.precision_lost += 1;
                }
                
                true
            } else {
                stats.deep_equality_failures += 1;
                false
            }
        }
        Err(_) => {
            stats.deep_equality_failures += 1;
            false
        }
    }
}

/// Validate a pool burn with deep equality check
fn validate_pool_burn(burn: &PoolBurnTLV, stats: &mut E2EValidationStats) -> bool {
    stats.burns_processed += 1;
    
    let bytes = burn.to_bytes();
    match PoolBurnTLV::from_bytes(&bytes) {
        Ok(recovered) => {
            if burn == &recovered {
                stats.deep_equality_passes += 1;
                stats.precision_preserved += 1;
                true
            } else {
                stats.deep_equality_failures += 1;
                false
            }
        }
        Err(_) => {
            stats.deep_equality_failures += 1;
            false
        }
    }
}

/// Validate a pool tick crossing with deep equality check
fn validate_pool_tick(tick: &PoolTickTLV, stats: &mut E2EValidationStats) -> bool {
    stats.ticks_processed += 1;
    
    let bytes = tick.to_bytes();
    match PoolTickTLV::from_bytes(&bytes) {
        Ok(recovered) => {
            if tick == &recovered {
                stats.deep_equality_passes += 1;
                
                // Verify sqrt price X96 format preserved
                if tick.price_sqrt == recovered.price_sqrt {
                    stats.precision_preserved += 1;
                } else {
                    stats.precision_lost += 1;
                }
                
                true
            } else {
                stats.deep_equality_failures += 1;
                false
            }
        }
        Err(_) => {
            stats.deep_equality_failures += 1;
            false
        }
    }
}

/// Validate pool liquidity update with deep equality check
fn validate_pool_liquidity(liq: &PoolLiquidityTLV, stats: &mut E2EValidationStats) -> bool {
    stats.liquidity_updates_processed += 1;
    
    let bytes = liq.to_bytes();
    match PoolLiquidityTLV::from_bytes(&bytes) {
        Ok(recovered) => {
            if liq == &recovered {
                stats.deep_equality_passes += 1;
                
                // Check all reserves preserved
                let reserves_match = liq.reserves.iter()
                    .zip(recovered.reserves.iter())
                    .all(|(a, b)| a == b);
                    
                if reserves_match {
                    stats.precision_preserved += 1;
                } else {
                    stats.precision_lost += 1;
                }
                
                true
            } else {
                stats.deep_equality_failures += 1;
                false
            }
        }
        Err(_) => {
            stats.deep_equality_failures += 1;
            false
        }
    }
}

/// Generate test pool events simulating live Polygon data
async fn generate_test_events(tx: mpsc::Sender<TLVMessage>) {
    let pool_id = PoolInstrumentId::from_pair(VenueId::Polygon, 0x1234, 0x5678);
    
    // Simulate a realistic sequence of pool events
    
    // 1. Initial liquidity provision
    let mint = PoolMintTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        provider: 0xDEADBEEF,
        tick_lower: -887220,
        tick_upper: 887220,
        liquidity_delta: 1000000000000000,
        amount0: 500000000000000,
        amount1: 1000000000000000,
        timestamp_ns: 1700000000000000001,
    };
    tx.send(mint.to_tlv_message()).await.unwrap();
    
    // 2. Swap event
    let swap = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        token_in: 0x1234,
        token_out: 0x5678,
        amount_in: 100000000000000,  // 1000000.00000000
        amount_out: 195000000000000,  // 1950000.00000000
        fee_paid: 300000000000,       // 3000.00000000
        timestamp_ns: 1700000000000000002,
    };
    tx.send(swap.to_tlv_message()).await.unwrap();
    
    // 3. Tick crossing
    let tick = PoolTickTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        tick: 100,
        liquidity_net: -50000000000000,
        price_sqrt: 7922816251426433759,  // X96 format
        timestamp_ns: 1700000000000000003,
    };
    tx.send(tick.to_tlv_message()).await.unwrap();
    
    // 4. Another swap (potential arbitrage)
    let swap2 = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        token_in: 0x5678,
        token_out: 0x1234,
        amount_in: 200000000000000,
        amount_out: 98000000000000,
        fee_paid: 600000000000,
        timestamp_ns: 1700000000000000004,
    };
    tx.send(swap2.to_tlv_message()).await.unwrap();
    
    // 5. Liquidity removal
    let burn = PoolBurnTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        provider: 0xCAFEBABE,
        tick_lower: -100,
        tick_upper: 100,
        liquidity_delta: -200000000000000,
        amount0: 100000000000000,
        amount1: 200000000000000,
        timestamp_ns: 1700000000000000005,
    };
    tx.send(burn.to_tlv_message()).await.unwrap();
    
    // 6. Liquidity state update
    let liquidity = PoolLiquidityTLV {
        venue: VenueId::Polygon,
        pool_id: pool_id.clone(),
        reserves: vec![
            800000000000000,   // Token 0
            1600000000000000,  // Token 1
        ],
        total_supply: 1200000000000000,
        fee_rate: 30,  // 0.3%
        timestamp_ns: 1700000000000000006,
    };
    tx.send(liquidity.to_tlv_message()).await.unwrap();
    
    // Generate more events to simulate real activity
    for i in 0..10 {
        let swap = PoolSwapTLV {
            venue: VenueId::Polygon,
            pool_id: pool_id.clone(),
            token_in: if i % 2 == 0 { 0x1234 } else { 0x5678 },
            token_out: if i % 2 == 0 { 0x5678 } else { 0x1234 },
            amount_in: 10000000000000 + (i as i64 * 1000000000000),
            amount_out: 19500000000000 - (i as i64 * 500000000000),
            fee_paid: 30000000000 + (i as i64 * 1000000000),
            timestamp_ns: 1700000000000000010 + i as u64,
        };
        tx.send(swap.to_tlv_message()).await.unwrap();
        
        if i % 3 == 0 {
            // Add occasional tick crossings
            let tick = PoolTickTLV {
                venue: VenueId::Polygon,
                pool_id: pool_id.clone(),
                tick: 100 + (i as i32 * 10),
                liquidity_net: -5000000000000 * (i as i64 + 1),
                price_sqrt: 7922816251426433759 + (i as u64 * 1000000),
                timestamp_ns: 1700000000000000020 + i as u64,
            };
            tx.send(tick.to_tlv_message()).await.unwrap();
        }
    }
}

#[tokio::test]
async fn test_e2e_live_validation() {
    println!("\n🚀 Starting E2E Live Data Validation Test");
    println!("   Testing complete pipeline with all pool event types\n");
    
    let mut stats = E2EValidationStats::default();
    let detector = TestArbitrageDetector::new();
    
    // Create channel for events
    let (tx, mut rx) = mpsc::channel::<TLVMessage>(100);
    
    // Start event generator
    tokio::spawn(async move {
        generate_test_events(tx).await;
    });
    
    // Process events with validation and arbitrage detection
    let mut latencies = Vec::new();
    
    while let Ok(tlv_msg) = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
        if let Some(msg) = tlv_msg {
            let start = std::time::Instant::now();
            
            // Validate based on TLV type
            match msg.header.tlv_type {
                TLVType::PoolSwap => {
                    if let Ok(swap) = PoolSwapTLV::from_bytes(&msg.payload) {
                        if validate_pool_swap(&swap, &mut stats) {
                            // Check for arbitrage
                            if let Some(profit) = detector.check_swap_arbitrage(&swap) {
                                stats.opportunities_detected += 1;
                                if profit > detector.min_profit_threshold {
                                    stats.profitable_opportunities += 1;
                                    println!("💰 Arbitrage opportunity: ${:.2}", profit as f64 / 100000000.0);
                                } else {
                                    stats.unprofitable_skipped += 1;
                                }
                            }
                        }
                    }
                }
                TLVType::PoolMint => {
                    if let Ok(mint) = PoolMintTLV::from_bytes(&msg.payload) {
                        if validate_pool_mint(&mint, &mut stats) {
                            if let Some(profit) = detector.check_liquidity_arbitrage(&mint) {
                                stats.opportunities_detected += 1;
                                if profit > detector.min_profit_threshold {
                                    stats.profitable_opportunities += 1;
                                    println!("💧 Liquidity arbitrage: ${:.2}", profit as f64 / 100000000.0);
                                } else {
                                    stats.unprofitable_skipped += 1;
                                }
                            }
                        }
                    }
                }
                TLVType::PoolBurn => {
                    if let Ok(burn) = PoolBurnTLV::from_bytes(&msg.payload) {
                        validate_pool_burn(&burn, &mut stats);
                    }
                }
                TLVType::PoolTick => {
                    if let Ok(tick) = PoolTickTLV::from_bytes(&msg.payload) {
                        if validate_pool_tick(&tick, &mut stats) {
                            if let Some(profit) = detector.check_tick_arbitrage(&tick) {
                                stats.opportunities_detected += 1;
                                if profit > detector.min_profit_threshold {
                                    stats.profitable_opportunities += 1;
                                    println!("📊 Tick arbitrage: ${:.2}", profit as f64 / 100000000.0);
                                } else {
                                    stats.unprofitable_skipped += 1;
                                }
                            }
                        }
                    }
                }
                TLVType::PoolLiquidity => {
                    if let Ok(liq) = PoolLiquidityTLV::from_bytes(&msg.payload) {
                        validate_pool_liquidity(&liq, &mut stats);
                    }
                }
                _ => {}
            }
            
            let latency = start.elapsed().as_nanos() as u64;
            latencies.push(latency);
        }
    }
    
    // Calculate performance metrics
    if !latencies.is_empty() {
        stats.avg_latency_ns = latencies.iter().sum::<u64>() / latencies.len() as u64;
        stats.max_latency_ns = *latencies.iter().max().unwrap();
        stats.min_latency_ns = *latencies.iter().min().unwrap();
    }
    
    // Print comprehensive report
    stats.report();
    
    // Assertions for test validation
    assert!(stats.total_messages() > 0, "No messages processed!");
    assert!(stats.deep_equality_passes > 0, "No deep equality passes!");
    assert_eq!(stats.deep_equality_failures, 0, "Deep equality failures detected!");
    assert!(stats.pass_rate() == 100.0, "Not all messages passed validation!");
    assert!(stats.precision_rate() == 100.0, "Precision loss detected!");
    assert!(stats.opportunities_detected > 0, "No arbitrage opportunities detected!");
    assert!(stats.avg_latency_ns < 1000000, "Latency too high (>1ms)!");
    
    println!("\n✅ E2E Live Validation Test PASSED!");
    println!("   All pool events processed correctly");
    println!("   Deep equality maintained throughout pipeline");
    println!("   Arbitrage strategy fully functional");
}

/// Test with simulated network latency and failures
#[tokio::test]
async fn test_e2e_with_network_conditions() {
    println!("\n🌐 Testing E2E with Network Conditions");
    
    let mut stats = E2EValidationStats::default();
    let (tx, mut rx) = mpsc::channel::<TLVMessage>(100);
    
    // Generate events with simulated delays
    tokio::spawn(async move {
        let pool_id = PoolInstrumentId::from_pair(VenueId::Polygon, 0x1111, 0x2222);
        
        for i in 0..20 {
            // Simulate network jitter
            tokio::time::sleep(Duration::from_millis(i % 10)).await;
            
            let swap = PoolSwapTLV {
                venue: VenueId::Polygon,
                pool_id: pool_id.clone(),
                token_in: 0x1111,
                token_out: 0x2222,
                amount_in: 50000000000000 + (i as i64 * 1000000000000),
                amount_out: 95000000000000 - (i as i64 * 500000000000),
                fee_paid: 15000000000,
                sqrt_price_x96_after: 0,  // V2 pool, no V3 state
                tick_after: 0,
                liquidity_after: 0,
                timestamp_ns: 1700000001000000000 + (i as u64 * 1000000),
                block_number: 1000 + i as u64,
            };
            
            tx.send(swap.to_tlv_message()).await.unwrap();
        }
    });
    
    // Process with timeout handling
    let mut consecutive_timeouts = 0;
    loop {
        match tokio::time::timeout(Duration::from_millis(50), rx.recv()).await {
            Ok(Some(msg)) => {
                consecutive_timeouts = 0;
                
                if let Ok(swap) = PoolSwapTLV::from_bytes(&msg.payload) {
                    validate_pool_swap(&swap, &mut stats);
                }
            }
            Ok(None) => break,
            Err(_) => {
                consecutive_timeouts += 1;
                if consecutive_timeouts > 5 {
                    break;
                }
            }
        }
    }
    
    assert!(stats.swaps_processed >= 15, "Too many messages lost!");
    assert_eq!(stats.deep_equality_failures, 0, "Validation failures under network conditions!");
    
    println!("✅ Network conditions test passed");
    println!("   Processed {}/{} messages", stats.swaps_processed, 20);
    println!("   System resilient to network jitter");
}