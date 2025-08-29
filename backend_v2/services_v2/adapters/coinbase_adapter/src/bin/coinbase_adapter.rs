//! Coinbase adapter service binary

use adapter_service::{Adapter, SafeAdapter};
use torq_coinbase_adapter::CoinbasePluginAdapter;
use anyhow::Result;
use tokio::signal;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("torq_coinbase_adapter=debug".parse()?)
                .add_directive("torq_adapters=debug".parse()?),
        )
        .init();

    info!("Starting Coinbase adapter service");

    // Create adapter instance
    let mut adapter = CoinbasePluginAdapter::new();

    // Initialize the adapter
    adapter.initialize().await?;

    // Start the adapter
    adapter.start().await?;

    info!("Coinbase adapter running, press Ctrl+C to stop");

    // Wait for shutdown signal
    signal::ctrl_c().await?;

    info!("Shutdown signal received");

    // Stop the adapter
    adapter.stop().await?;

    info!("Coinbase adapter stopped");

    Ok(())
}