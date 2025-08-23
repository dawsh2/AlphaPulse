//! Market Data Relay Server Binary
//! 
//! Ultra-high performance relay for TLV types 1-19 with NO checksum validation.
//! Target: >1M messages/second
//! 
//! Usage:
//!   cargo run --bin market_data_relay [socket_path]

use clap::Parser;
use alphapulse_protocol_v2::relay::market_data_relay::MarketDataRelay;
use tokio::signal;
use tracing::{info, error, Level};
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "market_data_relay")]
#[command(about = "Ultra-high performance market data relay server")]
struct Args {
    /// Unix socket path for the relay
    #[arg(short, long, default_value = "/tmp/alphapulse_market_data.sock")]
    socket: String,
    
    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
    
    /// Enable performance metrics reporting
    #[arg(long)]
    metrics: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Initialize logging
    let log_level = match args.log_level.as_str() {
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };
    
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();
    
    info!("🚀 Starting AlphaPulse Market Data Relay");
    info!("Socket: {}", args.socket);
    info!("Performance Mode: CHECKSUM VALIDATION DISABLED");
    info!("Target: >1M messages/second");
    
    // Create and start the relay
    let mut relay = MarketDataRelay::new(&args.socket);
    
    // Start performance monitoring if requested
    if args.metrics {
        tokio::spawn(async move {
            performance_monitoring_task().await;
        });
    }
    
    // Handle shutdown gracefully
    let shutdown_signal = async {
        signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
        info!("Shutdown signal received, stopping market data relay...");
    };
    
    // Run the relay with graceful shutdown
    tokio::select! {
        result = relay.start() => {
            if let Err(e) = result {
                error!("Market data relay error: {}", e);
                return Err(e.into());
            }
        }
        _ = shutdown_signal => {
            info!("Market data relay shutting down gracefully");
        }
    }
    
    info!("Market data relay stopped");
    Ok(())
}

/// Background task for performance monitoring and reporting
async fn performance_monitoring_task() {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
    
    loop {
        interval.tick().await;
        
        // Note: In a real implementation, we'd need to pass relay stats through a channel
        // For now, just log that monitoring is active
        info!("📊 Market Data Relay performance monitoring active");
        
        // Example of performance warnings that would be implemented:
        // - Current throughput below 500K msg/s target
        // - No active consumers connected
        // - Memory usage growing unexpectedly
        // - Socket errors or connection issues
    }
}