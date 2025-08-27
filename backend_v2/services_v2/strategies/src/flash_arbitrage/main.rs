//! Flash Arbitrage Strategy Entry Point

use alphapulse_flash_arbitrage::{StrategyConfig, StrategyEngine};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = load_config()?;

    // Create and run strategy engine
    let mut engine = StrategyEngine::new(config).await?;
    engine.run().await?;

    Ok(())
}

fn load_config() -> Result<StrategyConfig> {
    use alphapulse_flash_arbitrage::executor::ExecutorConfig;
    use alphapulse_flash_arbitrage::DetectorConfig;

    // Load configuration from environment or use defaults
    let market_data_relay_path = std::env::var("MARKET_DATA_RELAY_PATH")
        .unwrap_or_else(|_| "/tmp/alphapulse/market_data.sock".to_string());

    let signal_relay_path = std::env::var("SIGNAL_RELAY_PATH")
        .unwrap_or_else(|_| "/tmp/alphapulse/signals.sock".to_string());

    Ok(StrategyConfig {
        detector: DetectorConfig::default(), // Use default detector config for now
        executor: ExecutorConfig {
            private_key: std::env::var("EXECUTOR_PRIVATE_KEY").unwrap_or_else(|_| {
                "0x0000000000000000000000000000000000000000000000000000000000000000".to_string()
            }),
            rpc_url: std::env::var("POLYGON_RPC_URL")
                .unwrap_or_else(|_| "https://polygon-rpc.com".to_string()),
            backup_rpc_urls: vec![
                "https://rpc-mainnet.matic.network".to_string(),
                "https://rpc.ankr.com/polygon".to_string(),
            ],
            flash_loan_contract: "0x0000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
            use_flashbots: false, // No Flashbots on Polygon
            flashbots_relay_url: "https://relay.flashbots.net".to_string(),
            max_gas_price_gwei: 100,
            tx_timeout_secs: 30,
            max_slippage_bps: 300, // 3% max slippage
        },
        market_data_relay_path,
        signal_relay_path,
        consumer_id: 1001, // Flash arbitrage strategy ID
    })
}
