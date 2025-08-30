//! Simple relay starter that uses default configs
//! Starts market data and signal relays for testing

use torq_relays::{relay::Relay, RelayConfig};
use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Start signal relay
    let signal_config = RelayConfig::signal_defaults();
    info!(
        "Starting signal relay: {}",
        signal_config.transport.path.as_ref().unwrap()
    );

    let mut signal_relay = Relay::new(signal_config).await?;

    // Start signal relay in background
    tokio::spawn(async move {
        if let Err(e) = signal_relay.start().await {
            eprintln!("Signal relay failed: {}", e);
        }
    });

    // Start market data relay
    let market_config = RelayConfig::market_data_defaults();
    info!(
        "Starting market data relay: {}",
        market_config.transport.path.as_ref().unwrap()
    );

    let mut market_relay = Relay::new(market_config).await?;

    info!("🚀 Both relays started successfully");

    // Run market data relay (blocking)
    market_relay.start().await?;

    Ok(())
}
