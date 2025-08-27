//! Kraken Signal Strategy Main Entry Point

use alphapulse_strategies::kraken_signals::{KrakenSignalStrategy, StrategyConfig};
use alphapulse_strategies::common::config::{resolve_config_path, load_config_file};
use alphapulse_strategies::common::logging::init_strategy_logging;
use anyhow::{Context, Result};
use tokio::signal;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize standardized logging
    init_strategy_logging("kraken_signals_service")?;

    info!("Starting AlphaPulse Kraken Signal Strategy");

    // Load configuration
    let config = load_config()
        .context("Failed to load Kraken strategy configuration")?;

    info!(
        "Configuration loaded: monitoring {} instruments",
        config.target_instruments.len()
    );

    // Create and start strategy
    let mut strategy = KrakenSignalStrategy::new(config);

    // Start strategy in background
    let strategy_handle = tokio::spawn(async move {
        strategy.start().await
            .context("Kraken signal strategy execution failed")
            .unwrap_or_else(|e| {
                error!("Strategy failed: {:?}", e);
            })
    });

    info!("Kraken Signal Strategy running. Press Ctrl+C to stop.");

    // Wait for shutdown signal
    signal::ctrl_c().await
        .context("Failed to listen for shutdown signal")?;

    info!("Shutting down Kraken Signal Strategy");
    strategy_handle.abort();

    Ok(())
}

fn load_config() -> Result<StrategyConfig> {
    let config_path = resolve_config_path(
        "KRAKEN_STRATEGY_CONFIG_PATH", 
        "configs/kraken_strategy.toml"
    );

    load_config_file(&config_path, StrategyConfig::default())
}