//! Polygon Adapter Binary
//!
//! Standalone binary for the Polygon DEX adapter plugin.

use torq_polygon_adapter::{PolygonAdapter, PolygonConfig};
use adapter_service::{Adapter, SafeAdapter};
use clap::Parser;
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "polygon_adapter")]
#[command(about = "Polygon DEX Adapter for Torq Protocol V2")]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    let level = if args.debug {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    
    tracing_subscriber::fmt()
        .with_max_level(level)
        .init();

    info!("🚀 Starting Polygon DEX Adapter");

    // Load configuration
    let config = PolygonConfig::from_file(&args.config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

    info!("📋 Configuration loaded from: {:?}", args.config);

    // Create adapter
    let adapter = PolygonAdapter::new(config)?;

    info!("Adapter created with ID: {}", adapter.identifier());

    // Start adapter
    match adapter.start().await {
        Ok(()) => {
            info!("✅ Polygon adapter started successfully");
            
            // Keep running until interrupted
            tokio::signal::ctrl_c().await?;
            info!("📡 Received shutdown signal");
            
            // Stop adapter
            adapter.stop().await?;
            info!("✅ Polygon adapter stopped gracefully");
        }
        Err(e) => {
            error!("🔥 Failed to start Polygon adapter: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}