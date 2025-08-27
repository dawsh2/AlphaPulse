//! Unified relay binary - single executable configured per domain
//!
//! Usage:
//!   relay --config config/market_data.toml
//!   relay --config config/signal.toml
//!   relay --config config/execution.toml

use alphapulse_relays::{relay::Relay, RelayConfig};
use anyhow::Result;
use clap::Parser;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(name = "relay")]
#[command(about = "AlphaPulse message relay service")]
#[command(version)]
struct Args {
    /// Path to configuration file
    #[arg(short, long)]
    config: String,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Enable JSON logging format
    #[arg(long)]
    json_logs: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    init_logging(&args)?;

    info!("Starting AlphaPulse Relay");
    info!("Configuration: {}", args.config);

    // Load configuration
    let config = RelayConfig::from_file(&args.config).map_err(|e| {
        error!("Failed to load configuration: {}", e);
        e
    })?;

    info!(
        "Loaded configuration for {} relay (domain {})",
        config.relay.name, config.relay.domain
    );

    // Log relay mode
    match config.relay.domain {
        1 => info!("Mode: PERFORMANCE (no checksum validation)"),
        2 => info!("Mode: RELIABILITY (checksum validation enabled)"),
        3 => info!("Mode: SECURITY (full validation + audit logging)"),
        _ => info!("Mode: CUSTOM"),
    }

    // Create and start relay
    let mut relay = Relay::new(config).await?;

    // Set up signal handlers
    let shutdown = setup_signal_handlers();

    // Start relay in background
    let _relay_handle = tokio::spawn(async move {
        if let Err(e) = relay.start().await {
            error!("Relay failed: {}", e);
            std::process::exit(1);
        }
    });

    // Wait for shutdown signal
    shutdown.await;
    info!("Received shutdown signal");

    // TODO: Implement graceful shutdown
    // relay.stop().await?;

    Ok(())
}

fn init_logging(args: &Args) -> Result<()> {
    let log_level = match args.log_level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };

    if args.json_logs {
        // JSON logging not available in current version
        tracing_subscriber::fmt().with_max_level(log_level).init();
    } else {
        tracing_subscriber::fmt().with_max_level(log_level).init();
    }

    Ok(())
}

async fn setup_signal_handlers() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
}
