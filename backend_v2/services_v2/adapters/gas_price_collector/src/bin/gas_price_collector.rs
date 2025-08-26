//! Gas price collector binary - Phase 1 implementation
//! 
//! Streams gas prices via WebSocket to avoid RPC rate limiting

use alphapulse_gas_price_collector::{GasPriceCollector, GasPriceCollectorConfig};
use anyhow::Result;
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();
    
    info!("🚀 Starting AlphaPulse Gas Price Collector (Phase 1)");
    
    // Load configuration (could be from file/env in production)
    let config = GasPriceCollectorConfig {
        ws_endpoint: std::env::var("POLYGON_WS_ENDPOINT")
            .unwrap_or_else(|_| "wss://polygon-mainnet.g.alchemy.com/v2/demo".to_string()),
        relay_socket_path: std::env::var("RELAY_SOCKET_PATH")
            .unwrap_or_else(|_| "/tmp/market_data_relay.sock".to_string()),
        network_id: 137, // Polygon
        priority_fee_gwei: std::env::var("PRIORITY_FEE_GWEI")
            .unwrap_or_else(|_| "2".to_string())
            .parse()
            .unwrap_or(2),
    };
    
    info!("⚡ Configuration:");
    info!("  WebSocket: {}", config.ws_endpoint);
    info!("  Relay Socket: {}", config.relay_socket_path);
    info!("  Network ID: {}", config.network_id);
    info!("  Priority Fee: {} gwei", config.priority_fee_gwei);
    
    // Create and start collector
    let collector = Arc::new(GasPriceCollector::new(config).await?);
    
    info!("✅ Gas price collector initialized");
    info!("📡 Starting WebSocket gas price streaming...");
    
    // Start streaming (runs forever)
    match collector.start_streaming().await {
        Ok(_) => info!("Gas price streaming completed"),
        Err(e) => {
            error!("❌ Gas price streaming failed: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}