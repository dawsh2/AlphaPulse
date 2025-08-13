// Test cross-process atomic operations on macOS ARM64
// This test checks if the issue is with MAP_SHARED flags or fundamental RMW restrictions

use std::fs::OpenOptions;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use memmap2::{Mmap, MmapMut, MmapOptions};

#[repr(C, align(128))]
struct TestAtomic {
    value: AtomicU64,
    _padding: [u8; 120],
}

fn test_writer_process() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Writer: Creating shared memory with test atomic");
    
    let path = "/tmp/cross_process_atomic_test";
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)?;
    
    file.set_len(128)?;
    
    let mut mmap = unsafe {
        MmapOptions::new()
            .len(128)
            .map_mut(&file)?
    };
    
    // Initialize the atomic
    let test_atomic = mmap.as_mut_ptr() as *mut TestAtomic;
    unsafe {
        (*test_atomic).value.store(0, Ordering::SeqCst);
    }
    
    println!("✅ Writer: Initialized atomic at {:p}", unsafe { &(*test_atomic).value });
    
    // Try some operations from the writer process
    unsafe {
        println!("🔧 Writer: Testing load operation...");
        let val = (*test_atomic).value.load(Ordering::SeqCst);
        println!("✅ Writer: load() successful, value={}", val);
        
        println!("🔧 Writer: Testing store operation...");
        (*test_atomic).value.store(42, Ordering::SeqCst);
        println!("✅ Writer: store() successful");
        
        println!("🔧 Writer: Testing fetch_add operation...");
        let old_val = (*test_atomic).value.fetch_add(1, Ordering::SeqCst);
        println!("✅ Writer: fetch_add() successful, old_value={}", old_val);
        
        println!("🔧 Writer: Testing fetch_or operation...");
        let old_val = (*test_atomic).value.fetch_or(0x100, Ordering::SeqCst);
        println!("✅ Writer: fetch_or() successful, old_value=0x{:x}", old_val);
    }
    
    println!("🔧 Writer: Waiting for reader to test cross-process operations...");
    std::thread::sleep(Duration::from_secs(10));
    
    // Check if reader modified the value
    unsafe {
        let final_val = (*test_atomic).value.load(Ordering::SeqCst);
        println!("🔧 Writer: Final value after reader: 0x{:x}", final_val);
    }
    
    Ok(())
}

fn test_reader_process() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Reader: Opening existing shared memory");
    
    let path = "/tmp/cross_process_atomic_test";
    let file = OpenOptions::new()
        .read(true)
        .write(true) // Note: we need write access for RMW operations
        .open(path)?;
    
    let mmap = unsafe {
        MmapOptions::new()
            .len(128)
            .map_mut(&file)?
    };
    
    let test_atomic = mmap.as_ptr() as *const TestAtomic;
    
    unsafe {
        println!("🔍 Reader: atomic at {:p}", &(*test_atomic).value);
        
        // Give writer time to initialize
        std::thread::sleep(Duration::from_secs(2));
        
        println!("🔍 Reader: Testing cross-process load...");
        let val = (*test_atomic).value.load(Ordering::SeqCst);
        println!("✅ Reader: Cross-process load() successful, value=0x{:x}", val);
        
        println!("🔍 Reader: Testing cross-process fetch_or...");
        let old_val = (*test_atomic).value.fetch_or(0x200, Ordering::SeqCst);
        println!("✅ Reader: Cross-process fetch_or() successful, old_value=0x{:x}", old_val);
        
        println!("🔍 Reader: Testing cross-process fetch_add...");
        let old_val = (*test_atomic).value.fetch_add(10, Ordering::SeqCst);
        println!("✅ Reader: Cross-process fetch_add() successful, old_value=0x{:x}", old_val);
        
        let final_val = (*test_atomic).value.load(Ordering::SeqCst);
        println!("🔍 Reader: Final value: 0x{:x}", final_val);
    }
    
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} [writer|reader]", args[0]);
        std::process::exit(1);
    }
    
    match args[1].as_str() {
        "writer" => {
            if let Err(e) = test_writer_process() {
                eprintln!("❌ Writer failed: {}", e);
                std::process::exit(1);
            }
        }
        "reader" => {
            if let Err(e) = test_reader_process() {
                eprintln!("❌ Reader failed: {}", e);
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("Unknown mode: {}", args[1]);
            std::process::exit(1);
        }
    }
}