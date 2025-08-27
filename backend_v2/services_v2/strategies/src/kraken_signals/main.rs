//! Kraken Signal Strategy Main Entry Point

use alphapulse_kraken_signals::{KrakenSignalStrategy, StrategyConfig};
use anyhow::Result;
use std::path::Path;
use tokio::signal;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("alphapulse_kraken_signals=info".parse()?)
                .add_directive("info".parse()?),
        )
        .init();

    info!("Starting AlphaPulse Kraken Signal Strategy");

    // Load configuration
    let config = load_config().unwrap_or_else(|e| {
        error!("Failed to load config: {}, using defaults", e);
        StrategyConfig::default()
    });

    info!(
        "Configuration loaded: monitoring {} instruments",
        config.target_instruments.len()
    );

    // Create and start strategy
    let mut strategy = KrakenSignalStrategy::new(config);

    // Start strategy in background
    let strategy_handle = tokio::spawn(async move {
        if let Err(e) = strategy.start().await {
            error!("Kraken signal strategy failed: {}", e);
        }
    });

    info!("Kraken Signal Strategy running. Press Ctrl+C to stop.");

    // Wait for shutdown signal
    signal::ctrl_c().await?;

    info!("Shutting down Kraken Signal Strategy");
    strategy_handle.abort();

    Ok(())
}

fn load_config() -> Result<StrategyConfig> {
    let config_path = std::env::var("KRAKEN_STRATEGY_CONFIG_PATH")
        .unwrap_or_else(|_| "kraken_strategy_config.toml".to_string());

    if Path::new(&config_path).exists() {
        let config_str = std::fs::read_to_string(&config_path)?;
        let config: StrategyConfig = toml::from_str(&config_str)?;
        Ok(config)
    } else {
        info!("Config file {} not found, using defaults", config_path);
        Ok(StrategyConfig::default())
    }
}
