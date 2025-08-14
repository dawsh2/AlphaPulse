// Test producer for TokioTransport
use alphapulse_common::{
    tokio_transport::init_global_transport,
    Trade,
};
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info".to_string())
        )
        .init();
    
    info!("🚀 Starting test producer for TokioTransport");
    
    // Initialize global transport
    let transport = init_global_transport(10000);
    info!("✅ Global transport initialized with capacity 10000");
    
    // Generate mock trades
    let symbols = vec!["BTC-USD", "ETH-USD", "SOL-USD"];
    let exchanges = vec!["coinbase", "kraken", "binance"];
    let mut trade_count = 0;
    
    loop {
        for symbol in &symbols {
            for exchange in &exchanges {
                let trade = Trade {
                    timestamp: chrono::Utc::now().timestamp() as f64,
                    symbol: symbol.to_string(),
                    exchange: exchange.to_string(),
                    price: match *symbol {
                        "BTC-USD" => 50000.0 + (rand::random::<f64>() * 1000.0),
                        "ETH-USD" => 3000.0 + (rand::random::<f64>() * 100.0),
                        "SOL-USD" => 100.0 + (rand::random::<f64>() * 10.0),
                        _ => 100.0,
                    },
                    volume: rand::random::<f64>() * 10.0,
                    side: Some(if rand::random::<bool>() { "buy" } else { "sell" }.to_string()),
                    trade_id: Some(format!("test_{}_{}", exchange, trade_count)),
                };
                
                transport.write(trade.clone()).await?;
                trade_count += 1;
                
                if trade_count % 100 == 0 {
                    info!("📊 Written {} trades", trade_count);
                    let metrics = transport.metrics();
                    info!("  Queue size: {}", metrics.queue_size.load(std::sync::atomic::Ordering::Relaxed));
                }
            }
        }
        
        // Vary the rate to simulate real market conditions
        let sleep_ms = (rand::random::<f64>() * 100.0) as u64;
        tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
    }
}