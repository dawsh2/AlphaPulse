// AlphaPulse API Server - HTTP interface compatible with Python repository pattern
use alphapulse_api_server::{
    handlers::{health, trades, metrics as metrics_handler},
    state::AppState,
    websocket,
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
    
    // Build the router
    let app = Router::new()
        // Health check
        .route("/health", get(health::health_check))
        
        // Trade data endpoints (compatible with MarketDataRepository)
        .route("/trades/:symbol", get(trades::get_trades))
        .route("/trades/:symbol/recent", get(trades::get_recent_trades))
        .route("/ohlcv/:symbol", get(trades::get_ohlcv))
        .route("/summary", get(trades::get_data_summary))
        .route("/symbols/:exchange", get(trades::get_symbols))
        
        // Metrics endpoint for Prometheus
        .route("/metrics", get(metrics_handler::prometheus_metrics))
        
        // WebSocket endpoint for real-time data
        .route("/ws", get(websocket::websocket_handler))
        
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