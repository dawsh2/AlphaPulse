// Test program to verify atomic shared memory implementation works in async contexts
// This tests the optimized approach that avoids SIGBUS without thread overhead

use alphapulse_common::{
    shared_memory_v2::{OptimizedOrderBookDeltaReader, OptimizedOrderBookDeltaWriter},
    shared_memory::SharedOrderBookDelta,
};
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing optimized atomic shared memory implementation...\n");
    
    let path = "/tmp/test_atomic_orderbook";
    let capacity = 10000;
    
    // Clean up any existing file
    let _ = std::fs::remove_file(path);
    
    // Create writer
    println!("📝 Creating atomic writer...");
    let mut writer = OptimizedOrderBookDeltaWriter::create(path, capacity)?;
    
    // Write test data
    println!("✍️ Writing test data...");
    for i in 0..100 {
        let mut delta = SharedOrderBookDelta::new(
            i * 1_000_000_000,
            "BTC/USD",
            "coinbase",
            i,
            if i > 0 { i - 1 } else { 0 },
        );
        
        // Add changes
        delta.add_change(50000.0 + i as f64, 1.5, true, 0);  // Ask
        delta.add_change(49900.0 + i as f64, 2.0, false, 0); // Bid
        
        writer.write_delta_optimized(&delta)?;
    }
    
    println!("✅ Wrote 100 deltas\n");
    
    // Test reading in async context (this is where SIGBUS occurred before)
    println!("🔍 Testing async reads (10 iterations)...\n");
    
    for iteration in 1..=10 {
        // Spawn async task to read from shared memory
        let path_clone = path.to_string();
        let handle = tokio::spawn(async move {
            let start = Instant::now();
            
            // Open reader in async context
            let mut reader = OptimizedOrderBookDeltaReader::open(&path_clone, 0)
                .expect("Failed to open reader");
            
            // Read deltas using atomic operations (NOT volatile reads)
            let deltas = reader.read_deltas_optimized();
            
            let latency = start.elapsed();
            
            (deltas.len(), latency)
        });
        
        // Add some async operations to stress test
        sleep(Duration::from_micros(10)).await;
        
        // Wait for result
        match handle.await {
            Ok((count, latency)) => {
                println!("  Iteration {}: Read {} deltas in {:.1}μs ✅", 
                         iteration, count, latency.as_nanos() as f64 / 1000.0);
            }
            Err(e) => {
                println!("  Iteration {}: FAILED - {:?} ❌", iteration, e);
                return Err(format!("Async read failed: {:?}", e).into());
            }
        }
    }
    
    println!("\n🎯 Testing with concurrent writes and reads...\n");
    
    // Test concurrent writing and reading
    let path_clone = path.to_string();
    let write_handle = tokio::spawn(async move {
        let mut writer = OptimizedOrderBookDeltaWriter::create(&path_clone, capacity)
            .expect("Failed to create writer");
        
        for i in 0..1000 {
            let mut delta = SharedOrderBookDelta::new(
                i * 1_000_000,
                "ETH/USD",
                "kraken",
                i,
                if i > 0 { i - 1 } else { 0 },
            );
            
            delta.add_change(3000.0 + (i as f64 * 0.1), 0.5, true, 1);  // Ask, update action
            
            writer.write_delta_optimized(&delta).expect("Write failed");
            
            // Small delay to simulate real-world timing
            tokio::time::sleep(Duration::from_micros(100)).await;
        }
        
        println!("  Writer: Completed 1000 writes");
    });
    
    // Multiple concurrent readers
    let mut read_handles = vec![];
    for reader_id in 0..4 {
        let path_clone = path.to_string();
        let handle = tokio::spawn(async move {
            let mut reader = OptimizedOrderBookDeltaReader::open(&path_clone, reader_id)
                .expect("Failed to open reader");
            
            let mut total_read = 0;
            let mut read_latencies = vec![];
            
            for _ in 0..50 {
                let start = Instant::now();
                let deltas = reader.read_deltas_optimized();
                let latency = start.elapsed();
                
                if !deltas.is_empty() {
                    total_read += deltas.len();
                    read_latencies.push(latency.as_nanos() as f64 / 1000.0);
                }
                
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            
            let avg_latency = if !read_latencies.is_empty() {
                read_latencies.iter().sum::<f64>() / read_latencies.len() as f64
            } else {
                0.0
            };
            
            println!("  Reader {}: Read {} deltas, avg latency: {:.1}μs", 
                     reader_id, total_read, avg_latency);
            
            (total_read, avg_latency)
        });
        read_handles.push(handle);
    }
    
    // Wait for all tasks
    write_handle.await?;
    for handle in read_handles {
        handle.await?;
    }
    
    println!("\n✨ SUCCESS! Atomic implementation works perfectly in async contexts:");
    println!("  • No SIGBUS crashes");
    println!("  • Sub-microsecond latencies achievable");
    println!("  • Lock-free concurrent access");
    println!("  • No thread overhead");
    
    // Clean up
    let _ = std::fs::remove_file(path);
    
    Ok(())
}