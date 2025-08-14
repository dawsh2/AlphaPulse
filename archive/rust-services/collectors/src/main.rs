// AlphaPulse Rust Collectors - Main Application
use alphapulse_collectors::{
    coinbase::CoinbaseCollector,
    kraken::KrakenCollector,
    binance_us::BinanceUSCollector,
    redis_writer::RedisStreamsWriter,
    orderbook_writer::OrderBookWriter,
    collector_trait::{CollectorManager, MarketDataCollector},
};
use alphapulse_common::{CollectorConfig, MetricsCollector};
use std::sync::Arc;
use tokio::sync::mpsc;
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
                .unwrap_or_else(|_| "alphapulse_collectors=info,alphapulse_common=info".to_string())
        )
        .init();
    
    info!("🚀 Starting AlphaPulse Rust Collectors v0.1.0");
    
    // Load configuration
    let config = load_config();
    info!("Configuration: {:#?}", config);
    
    // Initialize metrics
    let metrics = Arc::new(MetricsCollector::new());
    
    // Create collector manager
    let mut manager = CollectorManager::new(config.buffer_size);
    
    // Get the orderbook sender before adding collectors
    let orderbook_sender = manager.get_orderbook_sender();
    
    // Create collectors based on exchange configuration
    let symbols = config.symbols.clone();
    
    // Add Coinbase collector with Redis streams support
    let coinbase_collector = CoinbaseCollector::new(
        symbols.iter().map(|s| s.replace("/", "-")).collect() // Coinbase uses BTC-USD format
    )
    .with_orderbook_sender(orderbook_sender.clone())
    .with_redis_streams()
    .await
    .expect("Failed to connect to Redis for streaming");
    
    manager.add_collector(Arc::new(coinbase_collector) as Arc<dyn MarketDataCollector>);
    
    // Add Kraken collector  
    let kraken_collector = Arc::new(KrakenCollector::new(
        symbols.clone() // Kraken uses BTC/USD format
    )) as Arc<dyn MarketDataCollector>;
    manager.add_collector(kraken_collector);
    
    // Add Binance.US collector for USDT pairs
    let usdt_symbols = vec!["BTC/USDT".to_string(), "ETH/USDT".to_string()];
    let binance_us_collector = Arc::new(BinanceUSCollector::new(
        usdt_symbols
    )) as Arc<dyn MarketDataCollector>;
    manager.add_collector(binance_us_collector);
    
    // Get the receivers before moving to Arc
    let trade_receiver = manager.get_trade_receiver();
    let orderbook_receiver = manager.get_orderbook_receiver();
    
    // Start Redis writer for trades
    let redis_writer = RedisStreamsWriter::new(
        config.redis_url.clone(),
        config.buffer_size,
        config.batch_timeout_ms,
    );
    
    // Start OrderBook writer
    let orderbook_writer = OrderBookWriter::new(
        config.redis_url.clone(),
        config.buffer_size,
        config.batch_timeout_ms,
    );
    
    // Move manager to Arc after getting receivers
    let manager = Arc::new(manager);
    
    let writer_task = tokio::spawn(async move {
        if let Err(e) = redis_writer.start(trade_receiver).await {
            error!("Redis writer failed: {}", e);
        }
    });
    
    let orderbook_task = tokio::spawn(async move {
        if let Err(e) = orderbook_writer.start(orderbook_receiver).await {
            error!("OrderBook writer failed: {}", e);
        }
    });
    
    // Start all collectors
    manager.start_all().await?;
    
    // Health monitoring task
    let health_manager = manager.clone();
    let health_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let health_status = health_manager.health_status();
            
            info!("Health Status:");
            for (exchange, healthy) in health_status {
                let status = if healthy { "✓ HEALTHY" } else { "✗ UNHEALTHY" };
                info!("  {} {}", exchange, status);
            }
        }
    });
    
    // Metrics reporting task
    let metrics_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            metrics.record_uptime();
            
            // Record system metrics
            if let Ok(memory) = get_memory_usage() {
                metrics.record_memory_usage(memory);
            }
            
            info!("📊 Metrics updated");
        }
    });
    
    info!("🎯 All collectors started successfully");
    info!("📡 Collecting data from Coinbase and Kraken");
    info!("📤 Writing to Redis Streams: {}", config.redis_url);
    
    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    info!("🛑 Received shutdown signal");
    
    // Graceful shutdown
    writer_task.abort();
    orderbook_task.abort();
    health_task.abort();
    metrics_task.abort();
    
    info!("✅ AlphaPulse Collectors shutdown complete");
    Ok(())
}

fn load_config() -> CollectorConfig {
    CollectorConfig {
        exchange: "multi".to_string(),
        symbols: vec![
            "BTC/USD".to_string(),
            "ETH/USD".to_string(),
            "BTC/USDT".to_string(),
            "ETH/USDT".to_string(),
        ],
        redis_url: std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        api_port: std::env::var("API_PORT")
            .unwrap_or_else(|_| "3001".to_string())
            .parse()
            .unwrap_or(3001),
        buffer_size: std::env::var("BUFFER_SIZE")
            .unwrap_or_else(|_| "1000".to_string())
            .parse()
            .unwrap_or(1000),
        batch_timeout_ms: std::env::var("BATCH_TIMEOUT_MS")
            .unwrap_or_else(|_| "100".to_string())
            .parse()
            .unwrap_or(100),
    }
}

fn get_memory_usage() -> Result<u64, Box<dyn std::error::Error>> {
    // Simple memory usage estimation
    // In production, you might want to use a more sophisticated approach
    Ok(0) // Placeholder
}