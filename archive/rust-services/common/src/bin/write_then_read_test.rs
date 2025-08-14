// Test: Write data then read in async - does newly written data cause SIGBUS?
use alphapulse_common::shared_memory::{OrderBookDeltaWriter, OrderBookDeltaReader, SharedOrderBookDelta};
use tokio;
use std::time::Duration;

#[tokio::main]
async fn main() {
    println!("🔬 Write-Then-Read SIGBUS Test");
    println!("==============================\n");
    
    // Test 1: Read from existing shared memory (with collector data)
    test_read_existing().await;
    
    // Test 2: Create new file, write, then read
    test_write_then_read().await;
    
    // Test 3: Write in one thread, read in async
    test_concurrent_write_read().await;
    
    println!("\n✅ Test complete!");
}

async fn test_read_existing() {
    println!("TEST 1: Read from existing shared memory with collector data");
    println!("-------------------------------------------------------------");
    
    // Try to read data that collectors have written
    if let Ok(reader) = OrderBookDeltaReader::open("/tmp/alphapulse_shm/orderbook_deltas", 20) {
        println!("  Reader created for existing file");
        
        let handle = tokio::spawn(async move {
            let mut reader = reader;
            println!("    Reading existing data...");
            let deltas = reader.read_deltas();
            println!("    ✅ Read {} deltas from collector data", deltas.len());
            
            // Try reading again after a delay (collectors might write more)
            tokio::time::sleep(Duration::from_millis(100)).await;
            println!("    Reading after delay...");
            let deltas2 = reader.read_deltas();
            println!("    ✅ Read {} more deltas", deltas2.len());
        });
        
        match handle.await {
            Ok(_) => println!("  ✅ Reading existing data succeeded"),
            Err(e) => println!("  ❌ Reading existing data failed: {:?}", e),
        }
    }
    println!();
}

async fn test_write_then_read() {
    println!("TEST 2: Write to new file, then read");
    println!("-------------------------------------");
    
    let test_file = "/tmp/test_orderbook_deltas";
    
    // Clean up any existing file
    let _ = std::fs::remove_file(test_file);
    
    // Create writer and write some data
    if let Ok(mut writer) = OrderBookDeltaWriter::create(test_file, 1000) {
        println!("  Created writer for new file");
        
        // Write some test deltas
        for i in 0..5 {
            let mut delta = SharedOrderBookDelta::new(
                i as u64,
                "TEST-USD",
                "test",
                i as u64,
                0,
            );
            delta.add_change(100.0 + i as f64, 1.0, false, 1);
            delta.add_change(101.0 + i as f64, 1.0, true, 1);
            
            writer.write_delta(&delta).unwrap();
            println!("  Wrote delta {}", i);
        }
        
        println!("  ✅ Wrote 5 test deltas");
    }
    
    // Now try to read in async
    if let Ok(reader) = OrderBookDeltaReader::open(test_file, 21) {
        let handle = tokio::spawn(async move {
            let mut reader = reader;
            println!("    Reading freshly written data...");
            let deltas = reader.read_deltas();
            println!("    ✅ Read {} deltas", deltas.len());
            
            if !deltas.is_empty() {
                let first = &deltas[0];
                println!("    First delta: timestamp={}, symbol={}", 
                    first.timestamp_ns, first.symbol_str());
            }
        });
        
        match handle.await {
            Ok(_) => println!("  ✅ Reading freshly written data succeeded"),
            Err(e) => println!("  ❌ Reading freshly written data failed: {:?}", e),
        }
    }
    
    // Clean up
    let _ = std::fs::remove_file(test_file);
    println!();
}

async fn test_concurrent_write_read() {
    println!("TEST 3: Write in background, read in async");
    println!("-------------------------------------------");
    
    let test_file = "/tmp/test_concurrent_deltas";
    let _ = std::fs::remove_file(test_file);
    
    // Create the file first
    if let Ok(mut writer) = OrderBookDeltaWriter::create(test_file, 1000) {
        // Write one delta to initialize
        let delta = SharedOrderBookDelta::new(0, "INIT", "test", 0, 0);
        writer.write_delta(&delta).unwrap();
        println!("  Initialized file");
    }
    
    // Start a thread that continuously writes
    let writer_handle = std::thread::spawn(move || {
        if let Ok(mut writer) = OrderBookDeltaWriter::create(test_file, 1000) {
            for i in 0..10 {
                let mut delta = SharedOrderBookDelta::new(
                    i as u64,
                    "LIVE-USD",
                    "test",
                    i as u64,
                    0,
                );
                delta.add_change(100.0 + i as f64, 1.0, false, 1);
                writer.write_delta(&delta).unwrap();
                
                std::thread::sleep(Duration::from_millis(50));
            }
            println!("    Writer thread: Wrote 10 deltas");
        }
    });
    
    // Try to read while writing is happening
    tokio::time::sleep(Duration::from_millis(100)).await; // Let writer start
    
    if let Ok(reader) = OrderBookDeltaReader::open(test_file, 22) {
        let handle = tokio::spawn(async move {
            let mut reader = reader;
            
            for attempt in 0..3 {
                println!("    Read attempt {}...", attempt + 1);
                let deltas = reader.read_deltas();
                println!("    ✅ Read {} deltas", deltas.len());
                
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
        
        match handle.await {
            Ok(_) => println!("  ✅ Concurrent read succeeded"),
            Err(e) => println!("  ❌ Concurrent read failed: {:?}", e),
        }
    }
    
    // Wait for writer to finish
    writer_handle.join().unwrap();
    
    // Clean up
    let _ = std::fs::remove_file(test_file);
    println!();
}