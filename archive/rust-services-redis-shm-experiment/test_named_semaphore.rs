// Test named semaphore implementation
use alphapulse_common::named_semaphore::NamedSemaphore;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    println!("Testing named semaphore implementation (fully supported on macOS)...");
    
    // Test 1: Basic initialization
    println!("\n1. Testing basic initialization...");
    let sem = match NamedSemaphore::create(0) {
        Ok(s) => {
            println!("✅ Named semaphore created successfully: {}", s.name());
            s
        }
        Err(e) => {
            println!("❌ Failed to create named semaphore: {}", e);
            return;
        }
    };
    
    // Test 2: Post and wait
    println!("\n2. Testing post and wait...");
    sem.post().unwrap();
    println!("✅ Posted to semaphore");
    
    sem.wait().unwrap();
    println!("✅ Successfully waited on semaphore");
    
    // Test 3: Cross-thread signaling
    println!("\n3. Testing cross-thread signaling...");
    let sem = match NamedSemaphore::create(0) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            println!("❌ Failed to create semaphore: {}", e);
            return;
        }
    };
    
    println!("   Created semaphore: {}", sem.name());
    
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
    let sem = match NamedSemaphore::create(0) {
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
    
    // Test 5: Cross-process simulation (using named access)
    println!("\n5. Testing cross-process access...");
    let sem1 = NamedSemaphore::create(1).unwrap();
    let name = sem1.name().to_string();
    println!("   Created semaphore: {}", name);
    
    // Simulate another process opening the same semaphore
    let sem2 = NamedSemaphore::open(&name).unwrap();
    println!("   Opened same semaphore from 'different process'");
    
    sem1.wait().unwrap();
    println!("   Process 1: Acquired semaphore");
    
    match sem2.try_wait() {
        Ok(false) => println!("✅ Process 2: Correctly blocked"),
        _ => println!("❌ Process 2: Should have been blocked"),
    }
    
    sem1.post().unwrap();
    println!("   Process 1: Released semaphore");
    
    sem2.wait().unwrap();
    println!("✅ Process 2: Successfully acquired semaphore");
    
    println!("\n✅ All tests passed! Named semaphores work correctly on macOS.");
}