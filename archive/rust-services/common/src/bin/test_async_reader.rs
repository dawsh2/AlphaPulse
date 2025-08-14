// Test async shared memory reader to reproduce API server crash
use alphapulse_common::shared_memory::{SharedMemoryReader, OrderBookDeltaReader};
use tokio;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing async shared memory reader (reproducing API server pattern)...");
    
    // Test 1: Create reader and spawn async task
    println!("\n📊 Creating reader and spawning async task...");
    
    if let Ok(reader) = OrderBookDeltaReader::open("/tmp/alphapulse_shm/orderbook_deltas", 1) {
        println!("✅ Created OrderBookDeltaReader");
        
        // Spawn async task like the API server does
        let handle = tokio::spawn(async move {
            println!("  📊 Inside spawned task");
            
            let mut reader = reader;
            
            // Try to read immediately (this is where API server crashes)
            println!("  🎯 Attempting first read...");
            let deltas = reader.read_deltas();
            println!("  ✅ First read successful! Read {} deltas", deltas.len());
            
            // Read in a loop
            for i in 0..5 {
                tokio::time::sleep(Duration::from_millis(100)).await;
                let deltas = reader.read_deltas();
                println!("  Read {}: {} deltas", i + 1, deltas.len());
            }
            
            println!("  ✅ Task completed successfully");
        });
        
        // Wait for task to complete
        match handle.await {
            Ok(()) => println!("✅ Async task completed without error"),
            Err(e) => println!("❌ Async task panicked: {:?}", e),
        }
    } else {
        println!("❌ Failed to create reader");
    }
    
    // Test 2: Also test trades reader
    println!("\n📊 Testing SharedMemoryReader in async task...");
    
    if let Ok(reader) = SharedMemoryReader::open("/tmp/alphapulse_shm/trades", 0) {
        println!("✅ Created SharedMemoryReader");
        
        let handle = tokio::spawn(async move {
            println!("  📊 Inside spawned task for trades");
            
            let mut reader = reader;
            
            println!("  🎯 Attempting first trade read...");
            let trades = reader.read_trades();
            println!("  ✅ First read successful! Read {} trades", trades.len());
            
            println!("  ✅ Trade reader task completed successfully");
        });
        
        match handle.await {
            Ok(()) => println!("✅ Trade reader task completed without error"),
            Err(e) => println!("❌ Trade reader task panicked: {:?}", e),
        }
    }
    
    println!("\n✅ All async tests completed!");
    Ok(())
}