use alphapulse_flash_arbitrage::{OpportunityDetector, RelayConsumer, SignalOutput};
use alphapulse_state_market::PoolStateManager;
use anyhow::Result;
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with simple format
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Flash Arbitrage Service...");

    // Create shared components
    let pool_manager = Arc::new(PoolStateManager::new());
    info!("✅ Pool state manager initialized");

    // Create opportunity detector with pool manager and default config
    let detector = Arc::new(OpportunityDetector::new(
        pool_manager.clone(),
        Default::default(), // Use default detector configuration
    ));
    info!("✅ Opportunity detector initialized");

    // Create signal output component
    let signal_output = Arc::new(SignalOutput::new(
        "/tmp/alphapulse/signals.sock".to_string(),
    ));
    info!("✅ Signal output configured for Signal Relay");

    // Create relay consumer with all components
    let mut consumer = RelayConsumer::new(
        "/tmp/alphapulse/market_data.sock".to_string(),
        pool_manager,
        detector,
        signal_output,
    );

    info!("✅ Flash Arbitrage Service initialized successfully");
    info!("📡 Listening for pool events on Market Data Relay");
    info!("📊 Analyzing ALL swaps for arbitrage opportunities");
    info!("🎯 Sending signals to Signal Relay → Dashboard");

    // Start consuming and analyzing pool events
    if let Err(e) = consumer.start().await {
        error!("Flash arbitrage service error: {}", e);
        return Err(e);
    }

    Ok(())
}
