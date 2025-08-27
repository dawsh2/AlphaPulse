//! Development relay CLI for testing and debugging
//!
//! Usage:
//!   relay_dev market_data --log-level debug
//!   relay_dev signal --metrics-interval 5
//!   relay_dev execution --socket /tmp/test.sock

use alphapulse_relays::{relay::Relay, RelayConfig};
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::time::Duration;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(name = "relay_dev")]
#[command(about = "AlphaPulse relay development and testing tool")]
#[command(version)]
struct Args {
    /// Relay type to run
    #[command(subcommand)]
    relay_type: RelayType,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "debug", global = true)]
    log_level: String,

    /// Override socket path
    #[arg(short, long, global = true)]
    socket: Option<String>,

    /// Show performance metrics every N seconds
    #[arg(long, global = true)]
    metrics_interval: Option<u64>,

    /// Enable message content logging (verbose)
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum RelayType {
    /// Run market data relay (domain 1, no checksum)
    MarketData {
        /// Custom configuration file
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Run signal relay (domain 2, checksum enabled)
    Signal {
        /// Custom configuration file
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Run execution relay (domain 3, full validation)
    Execution {
        /// Custom configuration file
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Run all relays for testing
    All,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize debug logging
    init_debug_logging(&args)?;

    info!("🚀 Starting Relay Development Tool");

    match args.relay_type {
        RelayType::MarketData { ref config } => {
            run_relay("market_data", config.clone(), &args).await?;
        }
        RelayType::Signal { ref config } => {
            run_relay("signal", config.clone(), &args).await?;
        }
        RelayType::Execution { ref config } => {
            run_relay("execution", config.clone(), &args).await?;
        }
        RelayType::All => {
            run_all_relays(&args).await?;
        }
    }

    Ok(())
}

async fn run_relay(relay_name: &str, custom_config: Option<String>, args: &Args) -> Result<()> {
    info!("Starting {} relay in development mode", relay_name);

    // Load configuration
    let mut config = if let Some(config_path) = custom_config {
        info!("Using custom config: {}", config_path);
        RelayConfig::from_file(config_path)?
    } else {
        // Use default configs
        match relay_name {
            "market_data" => RelayConfig::market_data_defaults(),
            "signal" => RelayConfig::signal_defaults(),
            "execution" => RelayConfig::execution_defaults(),
            _ => return Err(anyhow::anyhow!("Unknown relay type")),
        }
    };

    // Override socket if provided
    if let Some(socket) = &args.socket {
        info!("Overriding socket path: {}", socket);
        config.transport.path = Some(socket.clone());
    }

    // Log configuration
    info!("Configuration loaded:");
    info!("  Domain: {}", config.relay.domain);
    info!("  Name: {}", config.relay.name);
    info!("  Checksum: {}", config.validation.checksum);
    info!("  Audit: {}", config.validation.audit);
    info!("  Topics: {:?}", config.topics.available);

    // Create relay
    let mut relay = Relay::new(config).await?;

    // Start metrics reporting if requested
    if let Some(interval) = args.metrics_interval {
        spawn_metrics_reporter(interval);
    }

    // Start relay
    info!("Relay starting...");
    if let Err(e) = relay.start().await {
        error!("Relay failed: {}", e);
        return Err(e.into());
    }

    Ok(())
}

async fn run_all_relays(_args: &Args) -> Result<()> {
    info!("Starting ALL relays for integration testing");

    // Create all three relays with dev paths
    let mut market_config = RelayConfig::market_data_defaults();
    market_config.transport.path = Some("/tmp/alphapulse_dev/market_data.sock".to_string());

    let mut signal_config = RelayConfig::signal_defaults();
    signal_config.transport.path = Some("/tmp/alphapulse_dev/signals.sock".to_string());

    let mut execution_config = RelayConfig::execution_defaults();
    execution_config.transport.path = Some("/tmp/alphapulse_dev/execution.sock".to_string());

    // Create socket directories
    std::fs::create_dir_all("/tmp/alphapulse_dev")?;

    // Start all relays
    let market_relay = Relay::new(market_config).await?;
    let signal_relay = Relay::new(signal_config).await?;
    let execution_relay = Relay::new(execution_config).await?;

    // Run in parallel
    tokio::select! {
        result = tokio::spawn(async move {
            let mut relay = market_relay;
            relay.start().await
        }) => {
            if let Err(e) = result? {
                error!("Market data relay failed: {}", e);
            }
        }
        result = tokio::spawn(async move {
            let mut relay = signal_relay;
            relay.start().await
        }) => {
            if let Err(e) = result? {
                error!("Signal relay failed: {}", e);
            }
        }
        result = tokio::spawn(async move {
            let mut relay = execution_relay;
            relay.start().await
        }) => {
            if let Err(e) = result? {
                error!("Execution relay failed: {}", e);
            }
        }
    }

    Ok(())
}

fn init_debug_logging(args: &Args) -> Result<()> {
    let log_level = match args.log_level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::DEBUG,
    };

    // Use pretty formatting for development
    tracing_subscriber::fmt()
        .pretty()
        .with_max_level(log_level)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    Ok(())
}

fn spawn_metrics_reporter(interval_secs: u64) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        loop {
            interval.tick().await;
            info!("📊 Performance Metrics:");
            info!("  Messages/sec: TODO");
            info!("  Latency p50: TODO");
            info!("  Latency p99: TODO");
            info!("  Active connections: TODO");
            info!("  Memory usage: TODO");
        }
    });
}
