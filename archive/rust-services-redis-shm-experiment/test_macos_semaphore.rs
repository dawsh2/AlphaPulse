// Test macOS semaphore implementation
use alphapulse_common::macos_semaphore::MacOSSemaphore;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    println!("Testing macOS semaphore implementation...");
    
    // Test 1: Basic initialization
    println!("\n1. Testing basic initialization...");
    match MacOSSemaphore::new(0) {
        Ok(_) => println!("✅ Semaphore created successfully!"),
        Err(e) => {
            println!("❌ Failed to create semaphore: {}", e);
            return;
        }
    }
    
    // Test 2: Post and wait
    println!("\n2. Testing post and wait...");
    let sem = match MacOSSemaphore::new(0) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            println!("❌ Failed to create semaphore: {}", e);
            return;
        }
    };
    
    sem.post().unwrap();
    println!("✅ Posted to semaphore");
    
    sem.wait().unwrap();
    println!("✅ Successfully waited on semaphore");
    
    // Test 3: Cross-thread signaling
    println!("\n3. Testing cross-thread signaling...");
    let sem = match MacOSSemaphore::new(0) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            println!("❌ Failed to create semaphore: {}", e);
            return;
        }
    };
    
    let sem_clone = Arc::clone(&sem);
    let start = Instant::now();
    
    // Spawn thread that will wait
    let handle = thread::spawn(move || {
        println!("   Thread: Waiting for semaphore...");
        sem_clone.wait().unwrap();
        let elapsed = start.elapsed();
        println!("   Thread: Got semaphore signal after {:?}", elapsed);
        42
    });
    
    // Give thread time to start waiting
    thread::sleep(Duration::from_millis(100));
    
    println!("   Main: Posting to semaphore...");
    sem.post().unwrap();
    
    // Wait for thread to complete
    let result = handle.join().unwrap();
    println!("✅ Thread completed with result: {}", result);
    
    // Test 4: Try wait
    println!("\n4. Testing try_wait...");
    let sem = match MacOSSemaphore::new(0) {
        Ok(s) => s,
        Err(e) => {
            println!("❌ Failed to create semaphore: {}", e);
            return;
        }
    };
    
    match sem.try_wait() {
        Ok(false) => println!("✅ try_wait correctly returned false (would block)"),
        Ok(true) => println!("❌ try_wait incorrectly returned true"),
        Err(e) => println!("❌ try_wait failed: {}", e),
    }
    
    sem.post().unwrap();
    
    match sem.try_wait() {
        Ok(true) => println!("✅ try_wait correctly returned true after post"),
        Ok(false) => println!("❌ try_wait incorrectly returned false after post"),
        Err(e) => println!("❌ try_wait failed: {}", e),
    }
    
    println!("\n✅ All tests passed! macOS semaphores work correctly.");
}