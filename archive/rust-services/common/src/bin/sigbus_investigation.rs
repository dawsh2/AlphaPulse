// Systematic SIGBUS investigation - isolate the exact cause
use std::fs::{File, OpenOptions};
use std::ptr;
use memmap2::{MmapOptions, Mmap};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio;

fn main() {
    println!("🔬 SIGBUS Root Cause Investigation");
    println!("===================================\n");
    
    // Test 1: Can we mmap at all in async?
    test_1_basic_mmap();
    
    // Test 2: Can we read from mmap in async without atomics?
    test_2_simple_read();
    
    // Test 3: Can we read atomics from mmap in async?
    test_3_atomic_read();
    
    // Test 4: Can we do volatile reads in async?
    test_4_volatile_read();
    
    // Test 5: What about with spawn_blocking?
    test_5_spawn_blocking();
    
    // Test 6: Can we pass mmap across thread boundaries?
    test_6_thread_boundary();
    
    // Test 7: What if we create mmap inside async context?
    test_7_mmap_in_async();
    
    println!("\n✅ Investigation complete!");
}

fn test_1_basic_mmap() {
    println!("TEST 1: Basic mmap in async context");
    println!("-------------------------------------");
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        println!("  Creating mmap...");
        let file = File::open("/tmp/alphapulse_shm/trades").unwrap();
        let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
        println!("  ✅ Mmap created: {} bytes at {:p}", mmap.len(), mmap.as_ptr());
        
        println!("  Spawning async task...");
        let handle = tokio::spawn(async move {
            println!("    Inside async task");
            println!("    Mmap pointer: {:p}", mmap.as_ptr());
            println!("    Mmap length: {}", mmap.len());
            println!("    ✅ Can access mmap properties");
        });
        
        match handle.await {
            Ok(_) => println!("  ✅ Async task completed successfully"),
            Err(e) => println!("  ❌ Async task panicked: {:?}", e),
        }
    });
    println!();
}

fn test_2_simple_read() {
    println!("TEST 2: Simple read from mmap in async");
    println!("---------------------------------------");
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let file = File::open("/tmp/alphapulse_shm/trades").unwrap();
        let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
        
        let handle = tokio::spawn(async move {
            println!("    Attempting to read first 4 bytes...");
            let ptr = mmap.as_ptr() as *const u32;
            let value = unsafe { *ptr };  // Direct dereference
            println!("    ✅ Read value: {}", value);
        });
        
        match handle.await {
            Ok(_) => println!("  ✅ Simple read succeeded"),
            Err(e) => println!("  ❌ Simple read failed: {:?}", e),
        }
    });
    println!();
}

fn test_3_atomic_read() {
    println!("TEST 3: Atomic read from mmap in async");
    println!("---------------------------------------");
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let file = File::open("/tmp/alphapulse_shm/trades").unwrap();
        let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
        
        let handle = tokio::spawn(async move {
            println!("    Attempting atomic read at offset 8...");
            let ptr = unsafe { mmap.as_ptr().add(8) } as *const AtomicU64;
            let atomic_ref = unsafe { &*ptr };
            let value = atomic_ref.load(Ordering::Acquire);
            println!("    ✅ Atomic read value: {}", value);
        });
        
        match handle.await {
            Ok(_) => println!("  ✅ Atomic read succeeded"),
            Err(e) => println!("  ❌ Atomic read failed: {:?}", e),
        }
    });
    println!();
}

fn test_4_volatile_read() {
    println!("TEST 4: Volatile read from mmap in async");
    println!("-----------------------------------------");
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let file = File::open("/tmp/alphapulse_shm/trades").unwrap();
        let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
        
        let handle = tokio::spawn(async move {
            println!("    Attempting volatile read...");
            let ptr = mmap.as_ptr() as *const u32;
            let value = unsafe { ptr::read_volatile(ptr) };
            println!("    ✅ Volatile read value: {}", value);
        });
        
        match handle.await {
            Ok(_) => println!("  ✅ Volatile read succeeded"),
            Err(e) => println!("  ❌ Volatile read failed: {:?}", e),
        }
    });
    println!();
}

fn test_5_spawn_blocking() {
    println!("TEST 5: Read with spawn_blocking");
    println!("---------------------------------");
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let file = File::open("/tmp/alphapulse_shm/trades").unwrap();
        let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
        
        let result = tokio::task::spawn_blocking(move || {
            println!("    Inside blocking task");
            let ptr = mmap.as_ptr() as *const u32;
            let value = unsafe { ptr::read_volatile(ptr) };
            println!("    Read value: {}", value);
            value
        }).await;
        
        match result {
            Ok(v) => println!("  ✅ spawn_blocking succeeded: {}", v),
            Err(e) => println!("  ❌ spawn_blocking failed: {:?}", e),
        }
    });
    println!();
}

fn test_6_thread_boundary() {
    println!("TEST 6: Mmap across thread boundary");
    println!("------------------------------------");
    
    let file = File::open("/tmp/alphapulse_shm/trades").unwrap();
    let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
    
    // Try to move mmap to another thread
    let handle = std::thread::spawn(move || {
        println!("    In new thread");
        let ptr = mmap.as_ptr() as *const u32;
        let value = unsafe { *ptr };
        println!("    Read value: {}", value);
        value
    });
    
    match handle.join() {
        Ok(v) => println!("  ✅ Thread read succeeded: {}", v),
        Err(_) => println!("  ❌ Thread read failed"),
    }
    println!();
}

fn test_7_mmap_in_async() {
    println!("TEST 7: Create mmap inside async context");
    println!("-----------------------------------------");
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let handle = tokio::spawn(async {
            println!("    Creating mmap inside async task...");
            let file = File::open("/tmp/alphapulse_shm/trades").unwrap();
            let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
            
            println!("    Reading from mmap created in async...");
            let ptr = mmap.as_ptr() as *const u32;
            let value = unsafe { ptr::read_volatile(ptr) };
            println!("    ✅ Read value: {}", value);
        });
        
        match handle.await {
            Ok(_) => println!("  ✅ Async-created mmap succeeded"),
            Err(e) => println!("  ❌ Async-created mmap failed: {:?}", e),
        }
    });
    println!();
}