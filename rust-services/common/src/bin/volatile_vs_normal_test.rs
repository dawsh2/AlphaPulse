// Test volatile vs non-volatile reads in async context
use alphapulse_common::shared_memory::{OrderBookDeltaWriter, SharedOrderBookDelta};
use std::fs::File;
use memmap2::MmapOptions;
use std::ptr;
use tokio;

#[tokio::main]
async fn main() {
    println!("🔬 Volatile vs Non-Volatile Read Test");
    println!("======================================\n");
    
    // First, create a test file with data
    let test_file = "/tmp/test_volatile_reads";
    let _ = std::fs::remove_file(test_file);
    
    if let Ok(mut writer) = OrderBookDeltaWriter::create(test_file, 100) {
        for i in 0..5 {
            let mut delta = SharedOrderBookDelta::new(i, "TEST", "test", i, 0);
            delta.add_change(100.0 + i as f64, 1.0, false, 1);
            writer.write_delta(&delta).unwrap();
        }
        println!("✅ Created test file with 5 deltas");
    }
    
    // Test 1: Non-volatile read of SharedOrderBookDelta (256 bytes)
    test_non_volatile_read(test_file).await;
    
    // Test 2: Volatile read of SharedOrderBookDelta (256 bytes)
    test_volatile_read(test_file).await;
    
    // Test 3: Read in smaller chunks
    test_chunked_read(test_file).await;
    
    // Clean up
    let _ = std::fs::remove_file(test_file);
    println!("\n✅ Test complete!");
}

async fn test_non_volatile_read(path: &str) {
    println!("\nTEST 1: Non-volatile read of 256-byte struct");
    println!("----------------------------------------------");
    
    let path = path.to_string();
    let handle = tokio::spawn(async move {
        let file = File::open(&path).unwrap();
        let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
        
        // Skip header and read first delta
        let header_size = std::mem::size_of::<alphapulse_common::shared_memory::RingBufferHeader>();
        let delta_ptr = unsafe { mmap.as_ptr().add(header_size) } as *const SharedOrderBookDelta;
        
        println!("  🎯 Attempting NON-volatile read...");
        let delta = unsafe { *delta_ptr };  // Regular dereference, not volatile
        println!("  ✅ Non-volatile read succeeded: timestamp={}", delta.timestamp_ns);
    });
    
    match handle.await {
        Ok(_) => println!("  ✅ Non-volatile read completed"),
        Err(e) => println!("  ❌ Non-volatile read crashed: {:?}", e),
    }
}

async fn test_volatile_read(path: &str) {
    println!("\nTEST 2: Volatile read of 256-byte struct");
    println!("-----------------------------------------");
    
    let path = path.to_string();
    let handle = tokio::spawn(async move {
        let file = File::open(&path).unwrap();
        let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
        
        // Skip header and read first delta
        let header_size = std::mem::size_of::<alphapulse_common::shared_memory::RingBufferHeader>();
        let delta_ptr = unsafe { mmap.as_ptr().add(header_size) } as *const SharedOrderBookDelta;
        
        println!("  🎯 Attempting VOLATILE read...");
        let delta = unsafe { ptr::read_volatile(delta_ptr) };  // Volatile read
        println!("  ✅ Volatile read succeeded: timestamp={}", delta.timestamp_ns);
    });
    
    match handle.await {
        Ok(_) => println!("  ✅ Volatile read completed"),
        Err(e) => println!("  ❌ Volatile read crashed: {:?}", e),
    }
}

async fn test_chunked_read(path: &str) {
    println!("\nTEST 3: Read struct in smaller chunks");
    println!("---------------------------------------");
    
    let path = path.to_string();
    let handle = tokio::spawn(async move {
        let file = File::open(&path).unwrap();
        let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
        
        // Skip header
        let header_size = std::mem::size_of::<alphapulse_common::shared_memory::RingBufferHeader>();
        let delta_ptr = unsafe { mmap.as_ptr().add(header_size) };
        
        println!("  🎯 Reading timestamp (8 bytes)...");
        let timestamp = unsafe { ptr::read_volatile(delta_ptr as *const u64) };
        println!("  ✅ Timestamp: {}", timestamp);
        
        println!("  🎯 Reading full struct in 64-byte chunks...");
        let mut buffer = [0u8; 256];
        for i in 0..4 {
            let chunk_ptr = unsafe { delta_ptr.add(i * 64) };
            let chunk = unsafe { ptr::read_volatile(chunk_ptr as *const [u8; 64]) };
            buffer[i*64..(i+1)*64].copy_from_slice(&chunk);
            println!("  ✅ Read chunk {}", i);
        }
        
        // Interpret buffer as SharedOrderBookDelta
        let delta = unsafe { ptr::read(&buffer as *const _ as *const SharedOrderBookDelta) };
        println!("  ✅ Reconstructed delta: timestamp={}", delta.timestamp_ns);
    });
    
    match handle.await {
        Ok(_) => println!("  ✅ Chunked read completed"),
        Err(e) => println!("  ❌ Chunked read crashed: {:?}", e),
    }
}