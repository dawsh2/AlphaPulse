use protocol_v2::tlv::DemoDeFiArbitrageTLV;
use protocol_v2::{VenueId, TLVMessageBuilder, RelayDomain, MessageSource};
use tokio::net::UnixStream;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Creating demo arbitrage opportunity signal...");

    // Create a demo arbitrage TLV
    let arbitrage_tlv = DemoDeFiArbitrageTLV {
        strategy_id: 21, // Flash Arbitrage
        signal_id: 123456789,
        confidence: 95,
        chain_id: 137, // Polygon
        expected_profit_q: 1000000000000, // $10 profit (8 decimal places)
        required_capital_q: 50000000000000, // $500 capital
        estimated_gas_cost_q: 500000000, // $5 gas
        token_in: 0x2791bca1f2de4661ed88a30c99a7a9449aa84174, // USDC
        token_out: 0x7ceb23fd6bc0add59e62ac25578270cff1b9f619, // WETH
        optimal_amount_q: 1000000, // 1 USDC (6 decimals)
        slippage_tolerance: 50, // 0.5%
        max_gas_price_gwei: 30,
        valid_until: (chrono::Utc::now().timestamp() as u32) + 60, // Valid for 1 minute
        priority: 5,
        timestamp_ns: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos() as u64,
        venue_a: VenueId::UniswapV2,
        venue_b: VenueId::UniswapV3,
        pool_a: [1u8; 20],
        pool_b: [2u8; 20],
        _padding: [0u8; 20],
    };

    // Create TLV message
    let mut builder = TLVMessageBuilder::new(RelayDomain::Signal, MessageSource::Strategy21);
    builder.add_extended_tlv(255, &arbitrage_tlv)?;
    let message_bytes = builder.finalize()?;

    println!("📨 Sending demo signal to SignalRelay...");

    // Connect to signal relay
    let mut stream = UnixStream::connect("/tmp/alphapulse/signals.sock").await?;
    stream.write_all(&message_bytes).await?;

    println!("✅ Demo arbitrage signal sent! Check dashboard...");
    
    Ok(())
}
