// Investigation: Why do SharedMemoryReader/OrderBookDeltaReader crash in async?
use alphapulse_common::shared_memory::{SharedMemoryReader, OrderBookDeltaReader};
use tokio;
use std::time::Duration;

#[tokio::main]
async fn main() {
    println!("🔍 Reader Implementation Investigation");
    println!("======================================\n");
    
    // Test 1: Create reader outside, use inside async
    test_1_reader_outside_async().await;
    
    // Test 2: Create reader inside async
    test_2_reader_inside_async().await;
    
    // Test 3: Read immediately vs after delay
    test_3_timing_issue().await;
    
    // Test 4: Multiple reads
    test_4_multiple_reads().await;
    
    // Test 5: Check what read_deltas actually does
    test_5_debug_read_deltas().await;
    
    println!("\n✅ Investigation complete!");
}

async fn test_1_reader_outside_async() {
    println!("TEST 1: Create reader outside, use inside async");
    println!("------------------------------------------------");
    
    // Create reader outside async context
    if let Ok(reader) = OrderBookDeltaReader::open("/tmp/alphapulse_shm/orderbook_deltas", 10) {
        println!("  ✅ Reader created outside async");
        
        let handle = tokio::spawn(async move {
            println!("    Inside async task");
            let mut reader = reader;
            
            println!("    🎯 Calling get_lag()...");
            let lag = reader.get_lag();
            println!("    ✅ get_lag() returned: {}", lag);
            
            println!("    🎯 Calling read_deltas()...");
            let deltas = reader.read_deltas();  // THIS is where it crashes
            println!("    ✅ read_deltas() returned {} items", deltas.len());
        });
        
        match handle.await {
            Ok(_) => println!("  ✅ Task succeeded"),
            Err(e) => println!("  ❌ Task panicked: {:?}", e),
        }
    }
    println!();
}

async fn test_2_reader_inside_async() {
    println!("TEST 2: Create reader inside async");
    println!("-----------------------------------");
    
    let handle = tokio::spawn(async {
        println!("    Creating reader inside async task...");
        
        match OrderBookDeltaReader::open("/tmp/alphapulse_shm/orderbook_deltas", 11) {
            Ok(mut reader) => {
                println!("    ✅ Reader created inside async");
                
                println!("    🎯 Calling read_deltas()...");
                let deltas = reader.read_deltas();
                println!("    ✅ read_deltas() returned {} items", deltas.len());
            }
            Err(e) => {
                println!("    ❌ Failed to create reader: {:?}", e);
            }
        }
    });
    
    match handle.await {
        Ok(_) => println!("  ✅ Task succeeded"),
        Err(e) => println!("  ❌ Task panicked: {:?}", e),
    }
    println!();
}

async fn test_3_timing_issue() {
    println!("TEST 3: Read immediately vs after delay");
    println!("----------------------------------------");
    
    // Test immediate read
    if let Ok(reader) = OrderBookDeltaReader::open("/tmp/alphapulse_shm/orderbook_deltas", 12) {
        let handle = tokio::spawn(async move {
            let mut reader = reader;
            println!("    Reading immediately...");
            let deltas = reader.read_deltas();
            println!("    ✅ Immediate read: {} deltas", deltas.len());
        });
        
        match handle.await {
            Ok(_) => println!("  ✅ Immediate read succeeded"),
            Err(e) => println!("  ❌ Immediate read failed: {:?}", e),
        }
    }
    
    // Test delayed read
    if let Ok(reader) = OrderBookDeltaReader::open("/tmp/alphapulse_shm/orderbook_deltas", 13) {
        let handle = tokio::spawn(async move {
            let mut reader = reader;
            println!("    Waiting 100ms...");
            tokio::time::sleep(Duration::from_millis(100)).await;
            println!("    Reading after delay...");
            let deltas = reader.read_deltas();
            println!("    ✅ Delayed read: {} deltas", deltas.len());
        });
        
        match handle.await {
            Ok(_) => println!("  ✅ Delayed read succeeded"),
            Err(e) => println!("  ❌ Delayed read failed: {:?}", e),
        }
    }
    println!();
}

async fn test_4_multiple_reads() {
    println!("TEST 4: Multiple reads in sequence");
    println!("-----------------------------------");
    
    if let Ok(reader) = OrderBookDeltaReader::open("/tmp/alphapulse_shm/orderbook_deltas", 14) {
        let handle = tokio::spawn(async move {
            let mut reader = reader;
            
            for i in 0..3 {
                println!("    Read attempt {}...", i + 1);
                let deltas = reader.read_deltas();
                println!("    ✅ Read {}: {} deltas", i + 1, deltas.len());
                
                if i < 2 {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        });
        
        match handle.await {
            Ok(_) => println!("  ✅ Multiple reads succeeded"),
            Err(e) => println!("  ❌ Multiple reads failed: {:?}", e),
        }
    }
    println!();
}

async fn test_5_debug_read_deltas() {
    println!("TEST 5: Debug what read_deltas() actually does");
    println!("-----------------------------------------------");
    
    // Let's manually replicate what read_deltas does
    if let Ok(reader) = OrderBookDeltaReader::open("/tmp/alphapulse_shm/orderbook_deltas", 15) {
        let handle = tokio::spawn(async move {
            println!("    Reader created, examining internal state...");
            
            // The reader has: mmap, header, data_start, capacity, reader_id, last_sequence
            // read_deltas() does:
            // 1. Reads header.write_sequence atomically
            // 2. Loops from last_sequence to write_sequence
            // 3. For each, calculates index and reads volatile from data_start + offset
            
            println!("    Simulating read_deltas() operations...");
            
            // This is approximately what happens inside read_deltas()
            // We can't access private fields, but we know the issue is in there
            let mut reader = reader;
            
            println!("    🎯 Attempting actual read_deltas()...");
            let deltas = reader.read_deltas();
            println!("    ✅ Got {} deltas", deltas.len());
        });
        
        match handle.await {
            Ok(_) => println!("  ✅ Debug read succeeded"),
            Err(e) => println!("  ❌ Debug read failed: {:?}", e),
        }
    }
    println!();
}