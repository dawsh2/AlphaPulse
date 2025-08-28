//! Dashboard WebSocket server entry point

use torq_dashboard_websocket::{DashboardConfig, DashboardServer};
use clap::Parser;
use std::path::PathBuf;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Bind address
    #[arg(long, default_value = "127.0.0.1")]
    bind_address: String,

    /// Port
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// Market data relay path
    #[arg(long, default_value = "/tmp/torq/market_data.sock")]
    market_data_relay: String,

    /// Signal relay path
    #[arg(long, default_value = "/tmp/torq/signals.sock")]
    signal_relay: String,

    /// Execution relay path
    #[arg(long, default_value = "/tmp/torq/execution.sock")]
    execution_relay: String,

    /// Maximum connections
    #[arg(long, default_value_t = 1000)]
    max_connections: usize,

    /// Enable CORS
    #[arg(long)]
    enable_cors: bool,

    /// Heartbeat interval in seconds
    #[arg(long, default_value_t = 30)]
    heartbeat_interval: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "torq_dashboard_websocket=info,warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    info!("Starting Torq Dashboard WebSocket Server");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = if let Some(config_path) = args.config {
        load_config_from_file(&config_path).await?
    } else {
        // Use command line arguments
        DashboardConfig {
            bind_address: args.bind_address,
            port: args.port,
            market_data_relay_path: args.market_data_relay,
            signal_relay_path: args.signal_relay,
            execution_relay_path: args.execution_relay,
            max_connections: args.max_connections,
            client_buffer_size: 1000,
            enable_cors: args.enable_cors,
            heartbeat_interval_secs: args.heartbeat_interval,
        }
    };

    info!("Configuration loaded: {:?}", config);

    // Create and start dashboard server
    let server = DashboardServer::new(config);

    // Handle shutdown gracefully
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        info!("Shutdown signal received");
    };

    tokio::select! {
        result = server.start() => {
            if let Err(e) = result {
                error!("Dashboard server error: {}", e);
                return Err(e.into());
            }
        }
        _ = shutdown_signal => {
            info!("Shutting down dashboard server");
        }
    }

    Ok(())
}

async fn load_config_from_file(
    path: &PathBuf,
) -> Result<DashboardConfig, Box<dyn std::error::Error>> {
    let contents = tokio::fs::read_to_string(path).await?;

    if path.extension().and_then(|s| s.to_str()) == Some("json") {
        Ok(serde_json::from_str(&contents)?)
    } else {
        // Default to TOML
        Ok(toml::from_str(&contents)?)
    }
}
