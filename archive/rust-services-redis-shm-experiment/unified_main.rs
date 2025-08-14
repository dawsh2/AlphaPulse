// AlphaPulse Unified Service - Single process for maximum performance
// Collectors + API Server + WebSocket all share TokioTransport

use alphapulse_collectors::{
    coinbase::CoinbaseCollector,
    collector_trait::MarketDataCollector,
};
use alphapulse_api_server::{
    handlers::{health, metrics as metrics_handler},
    state::AppState,
    redis_websocket::{initialize_redis_websocket, redis_websocket_handler},
};
use axum::{routing::get, Router};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info".to_string())
        )
        .init();
    
    info!("🚀 Starting AlphaPulse Unified Service");
    
    // Initialize Redis WebSocket server
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());
    
    initialize_redis_websocket(redis_url.clone()).await?;
    info!("✅ Redis WebSocket server initialized");
    
    // Create channel for trades (for compatibility)
    let (tx, mut _rx) = mpsc::channel(1000);
    
    // Start Coinbase collector in background
    let symbols = vec!["BTC-USD".to_string(), "ETH-USD".to_string()];
    let collector = CoinbaseCollector::new(symbols)
        .with_redis_streams()
        .await?;
    
    let collector_arc = Arc::new(collector) as Arc<dyn MarketDataCollector>;
    let collector_clone = collector_arc.clone();
    
    tokio::spawn(async move {
        info!("📊 Starting Coinbase collector with Redis streams");
        if let Err(e) = collector_clone.start(tx).await {
            tracing::error!("Collector error: {}", e);
        }
    });
    
    info!("🎯 Trades will be written to Redis stream 'trades:stream'");
    info!("📡 API server will read with XREAD BLOCK (event-driven)");
    
    // Create application state
    let app_state = AppState::new().await?;
    
    // Build the API server router
    let app = Router::new()
        .route("/health", get(health::health_check))
        .route("/metrics", get(metrics_handler::prometheus_metrics))
        .route("/ws", get(redis_websocket_handler))
        .route("/realtime", get(redis_websocket_handler))
        .with_state(app_state)
        .layer(CorsLayer::permissive());
    
    // Start the server
    let port = std::env::var("API_PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .unwrap_or(3001);
    
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("🌐 API Server listening on {}", addr);
    info!("📊 WebSocket available at ws://{}/ws", addr);
    info!("🏥 Health check at http://{}/health", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}