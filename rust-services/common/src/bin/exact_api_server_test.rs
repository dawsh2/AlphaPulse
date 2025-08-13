// Exact replication of what API server does
use alphapulse_common::shared_memory::{OrderBookDeltaReader, SharedMemoryReader};
use tokio;
use std::time::Duration;

#[tokio::main]
async fn main() {
    println!("🔬 Exact API Server Pattern Test");
    println!("=================================\n");
    
    // This is EXACTLY what the API server does
    
    // Test 1: Coinbase orderbook deltas
    println!("TEST 1: Coinbase orderbook deltas (reader_id=1)");
    println!("------------------------------------------------");
    if let Ok(reader) = OrderBookDeltaReader::open("/tmp/alphapulse_shm/orderbook_deltas", 1) {
        println!("✅ Created reader outside async");
        
        // Now spawn exactly like API server
        let handle = tokio::spawn(async move {
            let mut reader = reader;
            println!("  📊 Inside spawned task (like run_coinbase_delta_reader)");
            
            // Add a small delay like we tested before
            tokio::time::sleep(Duration::from_millis(10)).await;
            
            // This is the EXACT line that crashes
            println!("  🎯 Calling read_deltas()...");
            let deltas = reader.read_deltas();
            println!("  ✅ Got {} deltas", deltas.len());
        });
        
        match handle.await {
            Ok(_) => println!("✅ Task completed"),
            Err(e) => println!("❌ Task crashed: {:?}", e),
        }
    } else {
        println!("❌ Failed to create reader");
    }
    
    // Test 2: Trades
    println!("\nTEST 2: Trades (reader_id=0)");
    println!("-----------------------------");
    if let Ok(reader) = SharedMemoryReader::open("/tmp/alphapulse_shm/trades", 0) {
        println!("✅ Created reader outside async");
        
        let handle = tokio::spawn(async move {
            let mut reader = reader;
            println!("  📊 Inside spawned task (like run_trade_reader)");
            
            tokio::time::sleep(Duration::from_millis(10)).await;
            
            println!("  🎯 Calling read_trades()...");
            let trades = reader.read_trades();
            println!("  ✅ Got {} trades", trades.len());
        });
        
        match handle.await {
            Ok(_) => println!("✅ Task completed"),
            Err(e) => println!("❌ Task crashed: {:?}", e),
        }
    } else {
        println!("❌ Failed to create reader");
    }
    
    println!("\n✅ Test complete!");
}