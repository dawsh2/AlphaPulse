// AlphaPulse API Server - HTTP interface compatible with Python repository pattern
use alphapulse_api_server::{
    handlers::{health, trades, metrics as metrics_handler, candles, delta_stats},
    state::AppState,
    realtime_websocket_discovery::{initialize_discovery_websocket, discovery_websocket_handler},
    tokio_websocket::{initialize_tokio_websocket, tokio_websocket_handler},
    redis_websocket::{initialize_redis_websocket, redis_websocket_handler},
};
use axum::{
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, error};
use dotenvy::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv().ok();
    
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "alphapulse_api_server=info,axum=info".to_string())
        )
        .init();
    
    info!("🚀 Starting AlphaPulse API Server v0.1.0");
    
    // Initialize metrics exporter
    let recorder = metrics_exporter_prometheus::PrometheusBuilder::new()
        .build_recorder();
    metrics::set_global_recorder(recorder)
        .expect("Failed to install Prometheus metrics recorder");
    
    // Create application state
    let app_state = AppState::new().await?;
    
    // Initialize WebSocket server based on configuration
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());
    
    // Use Redis WebSocket for true cross-process event-driven streaming
    initialize_redis_websocket(redis_url).await?;
    info!("🚀 Redis WebSocket server initialized (event-driven with XREAD BLOCK)");
    
    // Build the router with Redis WebSocket handler
    let app = Router::new()
        // Health check
        .route("/health", get(health::health_check))
        
        // Trade data endpoints (compatible with MarketDataRepository)
        .route("/trades/:symbol", get(trades::get_trades))
        .route("/trades/:symbol/recent", get(trades::get_recent_trades))
        .route("/ohlcv/:symbol", get(trades::get_ohlcv))
        .route("/summary", get(trades::get_data_summary))
        .route("/symbols/:exchange", get(trades::get_symbols))
        
        // Candle/chart data endpoints (compatible with frontend)
        .route("/api/market-data/:symbol/candles", get(candles::get_candles))
        .route("/api/market-data/batch", axum::routing::post(candles::get_candles_batch))
        .route("/api/market-data/save", axum::routing::post(candles::save_market_data))
        
        // Metrics endpoint for Prometheus
        .route("/metrics", get(metrics_handler::prometheus_metrics))
        
        // Delta statistics endpoints (new in v1.0)
        .route("/api/v1/delta-stats/:exchange/:symbol", get(delta_stats::get_delta_stats))
        .route("/api/v1/delta-stats/summary", get(delta_stats::get_delta_summary))
        .route("/api/v1/exchanges", get(delta_stats::get_exchanges))
        .route("/api/v1/system/health", get(delta_stats::get_system_health))
        .route("/api/v1/arbitrage/opportunities", get(delta_stats::get_arbitrage_opportunities))
        
        // Redis WebSocket endpoints (event-driven with XREAD BLOCK)
        .route("/ws", get(redis_websocket_handler))
        .route("/realtime", get(redis_websocket_handler))
        
        // Add state
        .with_state(app_state)
        
        // Add middleware
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());
    
    // Get port from environment
    let port = std::env::var("API_PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .unwrap_or(3001);
    
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    
    info!("🌐 API Server listening on {}", addr);
    info!("📊 Prometheus metrics available at http://{}:{}/metrics", addr.ip(), addr.port());
    info!("🏥 Health check available at http://{}:{}/health", addr.ip(), addr.port());
    
    // Start the server
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .await
        .map_err(|e| {
            error!("Server error: {}", e);
            e
        })?;
    
    Ok(())
}