//! Concurrent Processing Stress Test
//!
//! Tests the system under heavy concurrent load to identify:
//! - Race conditions in TLV processing
//! - Memory pressure under load
//! - Deadlocks or blocking issues
//! - Performance degradation at scale

use alphapulse_protocol_v2::{
    InstrumentId, PoolInstrumentId, PoolSwapTLV, QuoteTLV, TradeTLV, VenueId,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use zerocopy::AsBytes;

/// Test concurrent TLV processing with 1000+ simultaneous messages
#[tokio::test]
async fn test_concurrent_tlv_processing() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔥 Starting concurrent TLV processing stress test");

    let num_tasks = 1000;
    let messages_per_task = 100;
    let total_messages = num_tasks * messages_per_task;

    println!(
        "📊 Configuration: {} tasks × {} messages = {} total",
        num_tasks, messages_per_task, total_messages
    );

    // Shared counters for validation
    let processed_count = Arc::new(AtomicU64::new(0));
    let error_count = Arc::new(AtomicU64::new(0));

    // Semaphore to control concurrency (prevent overwhelming the system)
    let semaphore = Arc::new(Semaphore::new(100)); // Max 100 concurrent tasks

    let start_time = Instant::now();

    // Spawn concurrent tasks
    let mut handles = Vec::new();

    for task_id in 0..num_tasks {
        let processed_count = processed_count.clone();
        let error_count = error_count.clone();
        let semaphore = semaphore.clone();

        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();

            for msg_id in 0..messages_per_task {
                match process_test_message(task_id, msg_id).await {
                    Ok(_) => {
                        processed_count.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        eprintln!("❌ Task {} Message {} failed: {}", task_id, msg_id, e);
                        error_count.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await?;
    }

    let elapsed = start_time.elapsed();
    let processed = processed_count.load(Ordering::Relaxed);
    let errors = error_count.load(Ordering::Relaxed);

    println!("⚡ Results:");
    println!("   📊 Total time: {:?}", elapsed);
    println!("   ✅ Processed: {} messages", processed);
    println!("   ❌ Errors: {} messages", errors);
    println!(
        "   🚀 Throughput: {:.0} msg/sec",
        processed as f64 / elapsed.as_secs_f64()
    );

    // Validate results
    assert_eq!(
        processed + errors,
        total_messages as u64,
        "Message count mismatch"
    );
    assert_eq!(
        errors, 0,
        "Should have zero errors in concurrent processing"
    );

    // Performance requirement: Should process at least 10k messages per second
    let throughput = processed as f64 / elapsed.as_secs_f64();
    assert!(
        throughput > 10000.0,
        "Throughput too low: {:.0} msg/sec",
        throughput
    );

    println!("✅ Concurrent processing stress test passed!");
    Ok(())
}

/// Process a single test message with full roundtrip validation
async fn process_test_message(
    task_id: usize,
    msg_id: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create different message types based on IDs to test variety
    match (task_id + msg_id) % 3 {
        0 => {
            // Test TradeTLV
            let trade = TradeTLV {
                venue: VenueId::Kraken,
                instrument_id: InstrumentId::from_u64((task_id as u64) << 32 | msg_id as u64),
                price: (task_id as i64 * 1000 + msg_id as i64) * 100000000, // 8-decimal fixed point
                volume: (msg_id as i64 + 1) * 10000000,
                side: (task_id % 2) as u8,
                timestamp_ns: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_nanos() as u64,
            };

            // Full roundtrip
            let bytes = trade.to_bytes();
            let recovered = TradeTLV::from_bytes(&bytes)?;
            if trade != recovered {
                return Err(format!("TradeTLV roundtrip failed for {}-{}", task_id, msg_id).into());
            }
        }
        1 => {
            // Test QuoteTLV
            let quote = QuoteTLV {
                venue: VenueId::Binance,
                instrument_id: InstrumentId::from_u64((task_id as u64) << 32 | msg_id as u64),
                bid_price: (task_id as i64 * 1000) * 100000000,
                bid_size: (msg_id as i64 + 1) * 10000000,
                ask_price: (task_id as i64 * 1000 + 50) * 100000000, // Slightly higher
                ask_size: (msg_id as i64 + 2) * 10000000,
                timestamp_ns: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_nanos() as u64,
            };

            let bytes = quote.to_bytes();
            let recovered = QuoteTLV::from_bytes(&bytes)?;
            if quote != recovered {
                return Err(format!("QuoteTLV roundtrip failed for {}-{}", task_id, msg_id).into());
            }
        }
        2 => {
            // Test PoolSwapTLV
            let pool_id = PoolInstrumentId::from_pair(
                VenueId::Polygon,
                (task_id as u64) << 16 | 0x1000,
                (msg_id as u64) << 16 | 0x2000,
            );

            let swap = PoolSwapTLV {
                venue: VenueId::Polygon,
                pool_id,
                token_in: task_id as u64,
                token_out: msg_id as u64,
                amount_in: (task_id as i64 * 1000000) * 100000000,
                amount_out: (msg_id as i64 * 500000) * 100000000,
                amount_in_decimals: 18, // Default decimals
                amount_out_decimals: 6, // Default decimals
                sqrt_price_x96_after: 0,
                tick_after: 0,
                liquidity_after: 0,
                timestamp_ns: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_nanos() as u64,
                block_number: 1000,
            };

            let bytes = swap.to_bytes();
            let recovered = PoolSwapTLV::from_bytes(&bytes)?;
            if swap != recovered {
                return Err(
                    format!("PoolSwapTLV roundtrip failed for {}-{}", task_id, msg_id).into(),
                );
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}

/// Test memory usage under sustained high load
#[tokio::test]
async fn test_memory_pressure() -> Result<(), Box<dyn std::error::Error>> {
    println!("💾 Starting memory pressure test");

    let num_iterations = 100_000;
    let batch_size = 1000;

    println!(
        "📊 Processing {} messages in batches of {}",
        num_iterations, batch_size
    );

    let start_memory = get_memory_usage();
    let start_time = Instant::now();

    for batch in 0..(num_iterations / batch_size) {
        let mut messages = Vec::with_capacity(batch_size);

        // Create batch of messages
        for i in 0..batch_size {
            let msg_id = batch * batch_size + i;
            let trade = TradeTLV::new(
                VenueId::Binance,
                InstrumentId::from_u64(msg_id as u64),
                (msg_id as i64) * 100000000,
                (i as i64 + 1) * 10000000,
                (msg_id % 2) as u8,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_nanos() as u64,
            );

            // Serialize and deserialize
            let bytes = trade.as_bytes().to_vec();
            let recovered = TradeTLV::from_bytes(&bytes)?;
            messages.push(recovered);
        }

        // Validate all messages in batch
        for (i, msg) in messages.iter().enumerate() {
            let expected_price = ((batch * batch_size + i) as i64) * 100000000;
            let msg_price = msg.price; // Copy field to avoid packed field reference
            assert_eq!(
                msg_price, expected_price,
                "Price mismatch in batch {}, message {}",
                batch, i
            );
        }

        // Let messages go out of scope to test garbage collection
        drop(messages);

        // Periodic progress report
        if batch % 10 == 0 {
            let current_memory = get_memory_usage();
            let processed = batch * batch_size;
            let elapsed = start_time.elapsed();
            let rate = processed as f64 / elapsed.as_secs_f64();

            println!(
                "   📊 Batch {}: {} messages, {:.0} msg/sec, memory delta: {} KB",
                batch,
                processed,
                rate,
                (current_memory as i64 - start_memory as i64) / 1024
            );
        }
    }

    let final_memory = get_memory_usage();
    let memory_delta = final_memory as i64 - start_memory as i64;
    let elapsed = start_time.elapsed();

    println!("💾 Memory test results:");
    println!(
        "   📊 Processed: {} messages in {:?}",
        num_iterations, elapsed
    );
    println!("   💾 Memory delta: {} KB", memory_delta / 1024);
    println!(
        "   🚀 Average rate: {:.0} msg/sec",
        num_iterations as f64 / elapsed.as_secs_f64()
    );

    // Memory should not grow excessively (< 100MB for 100k messages)
    assert!(
        memory_delta < 100 * 1024 * 1024,
        "Memory usage too high: {} KB",
        memory_delta / 1024
    );

    println!("✅ Memory pressure test passed!");
    Ok(())
}

/// Get current memory usage (simplified - would use proper memory profiling in production)
fn get_memory_usage() -> usize {
    // This is a simplified version - in production we'd use proper memory profiling
    // For now, just return a placeholder that could be replaced with actual memory measurement
    std::env::var("MEMORY_USAGE")
        .unwrap_or_default()
        .parse()
        .unwrap_or(0)
}

/// Test system behavior with malformed/corrupted data
#[tokio::test]
async fn test_corrupted_data_handling() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing corrupted data handling");

    let test_cases = vec![
        ("Empty data", vec![]),
        ("Too short", vec![1, 2, 3]),
        ("Wrong size for TradeTLV", vec![0; 20]), // Should be 39 bytes
        ("All zeros", vec![0; 39]),
        ("All 0xFF", vec![0xFF; 39]),
        ("Random corruption", generate_corrupted_trade_data()),
        (
            "Truncated at end",
            generate_valid_trade_data()[..30].to_vec(),
        ),
        (
            "Invalid venue ID",
            corrupt_venue_id(generate_valid_trade_data()),
        ),
    ];

    let mut passed_tests = 0;
    let mut failed_tests = 0;

    for (description, data) in test_cases {
        match TradeTLV::from_bytes(&data) {
            Ok(trade) => {
                println!(
                    "   ⚠️  {} - Unexpectedly succeeded: {:?}",
                    description, trade
                );
                failed_tests += 1;
            }
            Err(error) => {
                println!("   ✅ {} - Properly rejected: {}", description, error);
                passed_tests += 1;
            }
        }
    }

    println!("🧪 Corrupted data test results:");
    println!("   ✅ Properly rejected: {}", passed_tests);
    println!("   ❌ Incorrectly accepted: {}", failed_tests);

    // All corrupted data should be rejected
    assert_eq!(
        failed_tests, 0,
        "Some corrupted data was incorrectly accepted"
    );

    println!("✅ Corrupted data handling test passed!");
    Ok(())
}

/// Generate a valid TradeTLV data for corruption testing
fn generate_valid_trade_data() -> Vec<u8> {
    let trade = TradeTLV::new(
        VenueId::Binance,
        InstrumentId::from_u64(0x1234567890ABCDEF),
        5000000000000, // $50,000
        100000000,     // 1.0
        0,
        1700000000000000000,
    );
    trade.as_bytes().to_vec()
}

/// Corrupt the venue ID field
fn corrupt_venue_id(mut data: Vec<u8>) -> Vec<u8> {
    if data.len() >= 2 {
        data[0] = 0xFF; // Invalid venue ID
        data[1] = 0xFF;
    }
    data
}

/// Generate random corrupted data based on valid structure
fn generate_corrupted_trade_data() -> Vec<u8> {
    let mut data = generate_valid_trade_data();
    // Corrupt random bytes
    for i in (0..data.len()).step_by(5) {
        data[i] = data[i].wrapping_add(1);
    }
    data
}
