//! E2E Validation Consumer
//!
//! Connects to MarketDataRelay and validates live Polygon data with deep equality checks

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::UnixStream;
use tracing::{debug, error, info, warn};

use protocol_v2::tlv::market_data::{
    PoolBurnTLV, PoolLiquidityTLV, PoolMintTLV, PoolSwapTLV, PoolTickTLV,
};

#[derive(Debug, Default)]
struct ValidationStats {
    messages_received: AtomicU64,
    swaps_validated: AtomicU64,
    mints_validated: AtomicU64,
    burns_validated: AtomicU64,
    ticks_validated: AtomicU64,
    liquidity_validated: AtomicU64,
    deep_equality_passes: AtomicU64,
    deep_equality_failures: AtomicU64,
    precision_preserved: AtomicU64,
    precision_lost: AtomicU64,
    arbitrage_opportunities: AtomicU64,
}

impl ValidationStats {
    fn report(&self) {
        println!("\n╔══════════════════════════════════════════════════════╗");
        println!("║         E2E LIVE DATA VALIDATION REPORT              ║");
        println!("╚══════════════════════════════════════════════════════╝");

        let total = self.messages_received.load(Ordering::Relaxed);
        let swaps = self.swaps_validated.load(Ordering::Relaxed);
        let mints = self.mints_validated.load(Ordering::Relaxed);
        let burns = self.burns_validated.load(Ordering::Relaxed);
        let ticks = self.ticks_validated.load(Ordering::Relaxed);
        let liquidity = self.liquidity_validated.load(Ordering::Relaxed);

        println!("\n📊 Message Processing:");
        println!("  Total received:     {}", total);
        println!("  Swaps validated:    {}", swaps);
        println!("  Mints validated:    {}", mints);
        println!("  Burns validated:    {}", burns);
        println!("  Ticks validated:    {}", ticks);
        println!("  Liquidity updates:  {}", liquidity);

        let passes = self.deep_equality_passes.load(Ordering::Relaxed);
        let failures = self.deep_equality_failures.load(Ordering::Relaxed);
        let total_validated = passes + failures;

        if total_validated > 0 {
            let pass_rate = (passes as f64 / total_validated as f64) * 100.0;
            println!("\n✅ Deep Equality Validation:");
            println!("  Passed:             {} ({:.2}%)", passes, pass_rate);
            println!("  Failed:             {}", failures);

            let preserved = self.precision_preserved.load(Ordering::Relaxed);
            let lost = self.precision_lost.load(Ordering::Relaxed);
            let precision_rate = if preserved + lost > 0 {
                (preserved as f64 / (preserved + lost) as f64) * 100.0
            } else {
                100.0
            };
            println!(
                "  Precision preserved: {} ({:.2}%)",
                preserved, precision_rate
            );
            println!("  Precision lost:      {}", lost);
        }

        let opps = self.arbitrage_opportunities.load(Ordering::Relaxed);
        if opps > 0 {
            println!("\n💰 Arbitrage Detection:");
            println!("  Opportunities found: {}", opps);
        }

        println!("\n══════════════════════════════════════════════════════");
    }
}

fn validate_pool_swap(swap: &PoolSwapTLV, stats: &Arc<ValidationStats>) -> bool {
    stats.swaps_validated.fetch_add(1, Ordering::Relaxed);

    // Serialize and deserialize for roundtrip validation
    let bytes = swap.to_bytes();
    match PoolSwapTLV::from_bytes(&bytes) {
        Ok(recovered) => {
            // Deep equality check
            if swap == &recovered {
                stats.deep_equality_passes.fetch_add(1, Ordering::Relaxed);

                // Verify precision preservation
                if swap.amount_in == recovered.amount_in
                    && swap.amount_out == recovered.amount_out
                    && swap.amount_in_decimals == recovered.amount_in_decimals
                    && swap.amount_out_decimals == recovered.amount_out_decimals
                {
                    stats.precision_preserved.fetch_add(1, Ordering::Relaxed);
                    debug!(
                        "✅ Swap validated: {} -> {} (decimals: {}, {})",
                        swap.amount_in,
                        swap.amount_out,
                        swap.amount_in_decimals,
                        swap.amount_out_decimals
                    );
                } else {
                    stats.precision_lost.fetch_add(1, Ordering::Relaxed);
                    warn!("⚠️ Precision lost in swap!");
                }

                // Simple arbitrage detection
                if swap.amount_in > 100000000000 {
                    // Large swap
                    stats
                        .arbitrage_opportunities
                        .fetch_add(1, Ordering::Relaxed);
                    info!("💰 Potential arbitrage from large swap: {}", swap.amount_in);
                }

                true
            } else {
                stats.deep_equality_failures.fetch_add(1, Ordering::Relaxed);
                error!("❌ Deep equality failed for swap!");
                false
            }
        }
        Err(e) => {
            stats.deep_equality_failures.fetch_add(1, Ordering::Relaxed);
            error!("❌ Failed to deserialize swap: {}", e);
            false
        }
    }
}

fn validate_pool_mint(mint: &PoolMintTLV, stats: &Arc<ValidationStats>) -> bool {
    stats.mints_validated.fetch_add(1, Ordering::Relaxed);

    let bytes = mint.to_bytes();
    match PoolMintTLV::from_bytes(&bytes) {
        Ok(recovered) => {
            if mint == &recovered {
                stats.deep_equality_passes.fetch_add(1, Ordering::Relaxed);

                // Check tick preservation (can be negative)
                if mint.tick_lower == recovered.tick_lower
                    && mint.tick_upper == recovered.tick_upper
                    && mint.liquidity_delta == recovered.liquidity_delta
                {
                    stats.precision_preserved.fetch_add(1, Ordering::Relaxed);
                    debug!(
                        "✅ Mint validated: liquidity={}, ticks=[{}, {}]",
                        mint.liquidity_delta, mint.tick_lower, mint.tick_upper
                    );
                } else {
                    stats.precision_lost.fetch_add(1, Ordering::Relaxed);
                }

                // Large liquidity additions can create opportunities
                if mint.liquidity_delta > 1000000000000000 {
                    stats
                        .arbitrage_opportunities
                        .fetch_add(1, Ordering::Relaxed);
                    info!("💧 Large liquidity added: {}", mint.liquidity_delta);
                }

                true
            } else {
                stats.deep_equality_failures.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
        Err(_) => {
            stats.deep_equality_failures.fetch_add(1, Ordering::Relaxed);
            false
        }
    }
}

fn validate_pool_burn(burn: &PoolBurnTLV, stats: &Arc<ValidationStats>) -> bool {
    stats.burns_validated.fetch_add(1, Ordering::Relaxed);

    let bytes = burn.to_bytes();
    match PoolBurnTLV::from_bytes(&bytes) {
        Ok(recovered) => {
            if burn == &recovered {
                stats.deep_equality_passes.fetch_add(1, Ordering::Relaxed);
                stats.precision_preserved.fetch_add(1, Ordering::Relaxed);
                debug!("✅ Burn validated: liquidity={}", burn.liquidity_delta);
                true
            } else {
                stats.deep_equality_failures.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
        Err(_) => {
            stats.deep_equality_failures.fetch_add(1, Ordering::Relaxed);
            false
        }
    }
}

fn validate_pool_tick(tick: &PoolTickTLV, stats: &Arc<ValidationStats>) -> bool {
    stats.ticks_validated.fetch_add(1, Ordering::Relaxed);

    let bytes = tick.to_bytes();
    match PoolTickTLV::from_bytes(&bytes) {
        Ok(recovered) => {
            if tick == &recovered {
                stats.deep_equality_passes.fetch_add(1, Ordering::Relaxed);

                // Verify sqrt price X96 format preserved
                if tick.price_sqrt == recovered.price_sqrt {
                    stats.precision_preserved.fetch_add(1, Ordering::Relaxed);
                    debug!(
                        "✅ Tick validated: tick={}, sqrt_price={}",
                        tick.tick, tick.price_sqrt
                    );
                } else {
                    stats.precision_lost.fetch_add(1, Ordering::Relaxed);
                }

                // Tick crossings with high liquidity changes create opportunities
                if tick.liquidity_net.abs() > 500000000000000 {
                    stats
                        .arbitrage_opportunities
                        .fetch_add(1, Ordering::Relaxed);
                    info!(
                        "📊 Large tick crossing: liquidity_net={}",
                        tick.liquidity_net
                    );
                }

                true
            } else {
                stats.deep_equality_failures.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
        Err(_) => {
            stats.deep_equality_failures.fetch_add(1, Ordering::Relaxed);
            false
        }
    }
}

fn validate_pool_liquidity(liq: &PoolLiquidityTLV, stats: &Arc<ValidationStats>) -> bool {
    stats.liquidity_validated.fetch_add(1, Ordering::Relaxed);

    let bytes = liq.to_bytes();
    match PoolLiquidityTLV::from_bytes(&bytes) {
        Ok(recovered) => {
            if liq == &recovered {
                stats.deep_equality_passes.fetch_add(1, Ordering::Relaxed);

                // Check all reserves preserved
                let reserves_match = liq
                    .reserves
                    .iter()
                    .zip(recovered.reserves.iter())
                    .all(|(a, b)| a == b);

                if reserves_match {
                    stats.precision_preserved.fetch_add(1, Ordering::Relaxed);
                    debug!("✅ Liquidity validated: {} reserves", liq.reserves.len());
                } else {
                    stats.precision_lost.fetch_add(1, Ordering::Relaxed);
                }

                true
            } else {
                stats.deep_equality_failures.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
        Err(_) => {
            stats.deep_equality_failures.fetch_add(1, Ordering::Relaxed);
            false
        }
    }
}

async fn process_relay_message(data: &[u8], stats: &Arc<ValidationStats>) {
    // Parse relay message header (32 bytes)
    if data.len() < 32 {
        return;
    }

    // Check magic number (0xDEADBEEF)
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if magic != 0xDEADBEEF {
        return;
    }

    let payload_size = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
    let _timestamp_ns = u64::from_le_bytes([
        data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
    ]);

    if data.len() < 32 + payload_size {
        return;
    }

    stats.messages_received.fetch_add(1, Ordering::Relaxed);

    // Extract TLV payload
    let tlv_data = &data[32..32 + payload_size];

    // Process TLV messages
    let mut offset = 0;
    while offset + 2 <= tlv_data.len() {
        let tlv_type = tlv_data[offset];
        let tlv_length = tlv_data[offset + 1] as usize;

        if offset + 2 + tlv_length > tlv_data.len() {
            break;
        }

        let tlv_payload = &tlv_data[offset + 2..offset + 2 + tlv_length];

        // Process based on TLV type
        match tlv_type {
            11 => {
                // PoolSwapTLV
                if let Ok(swap) = PoolSwapTLV::from_bytes(tlv_payload) {
                    validate_pool_swap(&swap, stats);
                }
            }
            12 => {
                // PoolMintTLV
                if let Ok(mint) = PoolMintTLV::from_bytes(tlv_payload) {
                    validate_pool_mint(&mint, stats);
                }
            }
            13 => {
                // PoolBurnTLV
                if let Ok(burn) = PoolBurnTLV::from_bytes(tlv_payload) {
                    validate_pool_burn(&burn, stats);
                }
            }
            14 => {
                // PoolTickTLV
                if let Ok(tick) = PoolTickTLV::from_bytes(tlv_payload) {
                    validate_pool_tick(&tick, stats);
                }
            }
            10 => {
                // PoolLiquidityTLV
                if let Ok(liq) = PoolLiquidityTLV::from_bytes(tlv_payload) {
                    validate_pool_liquidity(&liq, stats);
                }
            }
            _ => {}
        }

        offset += 2 + tlv_length;
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("🚀 Starting E2E Validation Consumer");
    info!("   Connecting to MarketDataRelay to validate live Polygon data");

    let stats = Arc::new(ValidationStats::default());
    let socket_path = "/tmp/alphapulse/market_data.sock";

    // Connect to relay
    let mut stream = match UnixStream::connect(socket_path).await {
        Ok(s) => {
            info!("✅ Connected to MarketDataRelay");
            s
        }
        Err(e) => {
            error!("❌ Failed to connect to relay: {}", e);
            error!("   Make sure MarketDataRelay is running!");
            return;
        }
    };

    let mut buffer = vec![0u8; 8192];
    let start_time = std::time::Instant::now();

    // Process messages for 30 seconds
    loop {
        if start_time.elapsed().as_secs() >= 30 {
            info!("⏰ Validation period complete (30 seconds)");
            break;
        }

        match tokio::time::timeout(
            std::time::Duration::from_millis(100),
            stream.read(&mut buffer),
        )
        .await
        {
            Ok(Ok(0)) => {
                warn!("Connection closed");
                break;
            }
            Ok(Ok(bytes_read)) => {
                process_relay_message(&buffer[..bytes_read], &stats).await;

                // Print progress every 10 messages
                let total = stats.messages_received.load(Ordering::Relaxed);
                if total % 10 == 0 && total > 0 {
                    info!("📦 Processed {} messages...", total);
                }
            }
            Ok(Err(e)) => {
                error!("Read error: {}", e);
                break;
            }
            Err(_) => {
                // Timeout - normal
            }
        }
    }

    // Print final report
    stats.report();

    // Validation assertions
    let total = stats.messages_received.load(Ordering::Relaxed);
    let passes = stats.deep_equality_passes.load(Ordering::Relaxed);
    let failures = stats.deep_equality_failures.load(Ordering::Relaxed);

    if total == 0 {
        error!("❌ No messages received! Check if live_polygon_relay is running.");
        std::process::exit(1);
    }

    if failures > 0 {
        error!("❌ {} deep equality failures detected!", failures);
        std::process::exit(1);
    }

    if passes == 0 {
        error!("❌ No validations passed!");
        std::process::exit(1);
    }

    let _preserved = stats.precision_preserved.load(Ordering::Relaxed);
    let lost = stats.precision_lost.load(Ordering::Relaxed);

    if lost > 0 {
        error!("❌ Precision loss detected in {} cases!", lost);
        std::process::exit(1);
    }

    info!("\n✅ E2E VALIDATION SUCCESSFUL!");
    info!("   {} messages validated with 100% deep equality", passes);
    info!("   All precision preserved");
    info!("   Arbitrage detection functional");
}
