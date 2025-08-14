// Simple test data producer to feed the dashboard
use alphapulse_common::{
    shared_memory::{SharedMemoryWriter, SharedTrade},
    shared_memory_registry::{SharedMemoryRegistry, FeedMetadata, FeedType},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::thread;

fn main() {
    println!("🚀 Starting test data producer for dashboard...");
    
    // Create shared memory writer
    let path = "./shm/test_trades";
    let capacity = 10000;
    
    println!("Creating shared memory writer at {}...", path);
    let mut writer = SharedMemoryWriter::create(path, capacity).expect("Failed to create writer");
    
    // Register the feed so the API server can discover it
    let mut registry = SharedMemoryRegistry::new().expect("Failed to create registry");
    let metadata = FeedMetadata {
        feed_id: "test_trades".to_string(),
        feed_type: FeedType::Trades,
        path: path.into(),
        exchange: "test".to_string(),
        symbol: Some("BTC-USD".to_string()),
        created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        last_heartbeat: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        capacity,
        producer_pid: std::process::id(),
    };
    
    registry.register_feed(metadata).expect("Failed to register feed");
    println!("✅ Feed registered with discovery service");
    
    // Generate fake trades
    let mut trade_id = 0u64;
    let symbols = vec!["BTC-USD", "ETH-USD"];
    let exchanges = vec!["coinbase", "kraken", "binance"];
    
    loop {
        for symbol in &symbols {
            for exchange in &exchanges {
                trade_id += 1;
                
                // Create trade
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;
                
                let price = if symbol.starts_with("BTC") {
                    50000.0 + (trade_id as f64 % 1000.0) - 500.0
                } else {
                    3000.0 + (trade_id as f64 % 100.0) - 50.0
                };
                
                let volume = 0.01 + (trade_id as f64 % 10.0) * 0.1;
                let side = trade_id % 2 == 0;
                
                let trade = SharedTrade::new(
                    timestamp,
                    symbol,
                    exchange,
                    price,
                    volume,
                    side,
                    &format!("trade_{}", trade_id),
                );
                
                // Write trade
                match writer.write_trade(&trade) {
                    Ok(_) => {
                        if trade_id % 100 == 0 {
                            println!("📊 Wrote {} trades (latest: {} {} @ ${:.2})", 
                                     trade_id, exchange, symbol, price);
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to write trade: {}", e);
                    }
                }
            }
        }
        
        // Control rate - about 60 trades/sec (6 per iteration, 10 iterations/sec)
        thread::sleep(Duration::from_millis(100));
    }
}