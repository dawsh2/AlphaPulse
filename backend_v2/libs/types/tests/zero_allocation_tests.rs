//! Zero-Allocation Testing Infrastructure for Hot Path
//!
//! Automated detection of allocation regressions in critical performance paths.
//! These tests ensure that hot path operations maintain zero-allocation guarantees.

use torq_types::protocol::tlv::market_data::TradeTLV;
use torq_types::protocol::tlv::{
    build_message_direct, with_hot_path_buffer, with_signal_buffer, TrueZeroCopyBuilder,
};
use torq_codec::protocol::{InstrumentId, RelayDomain, SourceType, TLVType, VenueId};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Allocation tracking wrapper for the system allocator
struct AllocationTracker;

/// Global counter for allocations during tests
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static BYTES_ALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for AllocationTracker {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Only track in test mode
        #[cfg(test)]
        {
            ALLOCATIONS.fetch_add(1, Ordering::SeqCst);
            BYTES_ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
        }
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        #[cfg(test)]
        {
            DEALLOCATIONS.fetch_add(1, Ordering::SeqCst);
        }
        System.dealloc(ptr, layout)
    }
}

#[cfg(test)]
#[global_allocator]
static GLOBAL: AllocationTracker = AllocationTracker;

/// Reset allocation counters for a new test
fn reset_allocation_tracking() {
    ALLOCATIONS.store(0, Ordering::SeqCst);
    DEALLOCATIONS.store(0, Ordering::SeqCst);
    BYTES_ALLOCATED.store(0, Ordering::SeqCst);
}

/// Get current allocation count
fn get_allocation_count() -> usize {
    ALLOCATIONS.load(Ordering::SeqCst)
}

/// Get current bytes allocated
fn get_bytes_allocated() -> usize {
    BYTES_ALLOCATED.load(Ordering::SeqCst)
}

/// Assert that no allocations occurred in the given closure
macro_rules! assert_zero_allocations {
    ($name:expr, $block:block) => {{
        reset_allocation_tracking();
        let start_allocs = get_allocation_count();

        $block

        let end_allocs = get_allocation_count();
        let allocations = end_allocs - start_allocs;

        assert_eq!(
            allocations, 0,
            "{}: Expected zero allocations, but {} allocations occurred ({} bytes)",
            $name,
            allocations,
            get_bytes_allocated()
        );
    }};
}

#[test]
fn test_hot_path_buffer_zero_allocations() {
    // Warm up the thread-local buffer first
    let _ = with_hot_path_buffer(|buffer| {
        buffer[0] = 0xFF;
        Ok(((), 1))
    });

    // Now test that subsequent uses have zero allocations
    assert_zero_allocations!("hot_path_buffer_reuse", {
        let result = with_hot_path_buffer(|buffer| {
            // Simulate writing message data
            for i in 0..100 {
                buffer[i] = (i % 256) as u8;
            }
            Ok((42, 100))
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    });
}

#[test]
fn test_signal_buffer_zero_allocations() {
    // Warm up the thread-local buffer
    let _ = with_signal_buffer(|buffer| {
        buffer[0] = 0xFF;
        Ok(((), 1))
    });

    // Test zero allocations on reuse
    assert_zero_allocations!("signal_buffer_reuse", {
        let result = with_signal_buffer(|buffer| {
            for i in 0..50 {
                buffer[i] = (i % 256) as u8;
            }
            Ok((123, 50))
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 123);
    });
}

#[test]
fn test_zero_copy_message_building_no_allocations() {
    // Create test data
    let instrument_id = InstrumentId {
        venue: VenueId::Binance as u16,
        asset_type: 1,
        reserved: 0,
        asset_id: 12345,
    };
    let trade = TradeTLV::from_instrument(
        VenueId::Binance,
        instrument_id,
        100_000_000, // price: $1.00
        50_000_000,  // volume: 0.50
        0,           // side: buy
        1234567890,  // timestamp
    );

    // Warm up the buffer
    let _ = with_hot_path_buffer(|buffer| {
        let builder =
            TrueZeroCopyBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector);
        builder
            .build_into_buffer(buffer, TLVType::Trade, &trade)
            .map(|size| (size, size))
    });

    // Test zero allocations for message construction into buffer
    // Note: When using build_with_hot_path_buffer() that returns Vec<u8>,
    // one allocation is expected and necessary for thread-safe channel communication
    assert_zero_allocations!("zero_copy_message_build_into_buffer", {
        let result = with_hot_path_buffer(|buffer| {
            let builder =
                TrueZeroCopyBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
                    .with_sequence(12345);

            let size = builder.build_into_buffer(buffer, TLVType::Trade, &trade)?;

            Ok((size, size))
        });

        assert!(result.is_ok());
        assert!(result.unwrap() > 32); // At least header size
    });
}

#[test]
fn test_multiple_message_builds_zero_allocations() {
    let instrument_id = InstrumentId {
        venue: VenueId::Kraken as u16,
        asset_type: 1,
        reserved: 0,
        asset_id: 67890,
    };
    let trade = TradeTLV::from_instrument(
        VenueId::Kraken,
        instrument_id,
        200_000_000,
        75_000_000,
        1,
        1234567891,
    );

    // Warm up
    let _ = with_hot_path_buffer(|buffer| {
        TrueZeroCopyBuilder::new(RelayDomain::MarketData, SourceType::KrakenCollector)
            .build_into_buffer(buffer, TLVType::Trade, &trade)
            .map(|size| (size, size))
    });

    // Test that building 100 messages causes zero allocations
    assert_zero_allocations!("multiple_message_builds", {
        for i in 0..100 {
            let result = with_hot_path_buffer(|buffer| {
                let builder =
                    TrueZeroCopyBuilder::new(RelayDomain::MarketData, SourceType::KrakenCollector)
                        .with_sequence(i);

                let size = builder.build_into_buffer(buffer, TLVType::Trade, &trade)?;

                // Simulate sending the message
                std::hint::black_box(&buffer[..size]);

                Ok((size, size))
            });

            assert!(result.is_ok());
        }
    });
}

#[test]
fn test_hot_path_performance_target() {
    let instrument_id = InstrumentId {
        venue: VenueId::Polygon as u16,
        asset_type: 1,
        reserved: 0,
        asset_id: 11111,
    };
    let trade = TradeTLV::from_instrument(
        VenueId::Polygon,
        instrument_id,
        150_000_000,
        25_000_000,
        0,
        1234567892,
    );

    // Warm up the buffer and JIT
    for _ in 0..1000 {
        let _ = with_hot_path_buffer(|buffer| {
            TrueZeroCopyBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
                .build_into_buffer(buffer, TLVType::Trade, &trade)
                .map(|size| (size, size))
        });
    }

    // Measure performance
    let iterations = 100_000;
    let start = std::time::Instant::now();

    reset_allocation_tracking();

    for i in 0..iterations {
        let _ = with_hot_path_buffer(|buffer| {
            let builder =
                TrueZeroCopyBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
                    .with_sequence(i);

            let size = builder.build_into_buffer(buffer, TLVType::Trade, &trade)?;

            std::hint::black_box(size);
            Ok(((), size))
        })
        .unwrap();
    }

    let duration = start.elapsed();
    let allocations = get_allocation_count();

    // Calculate performance metrics
    let ns_per_op = duration.as_nanos() as f64 / iterations as f64;
    let allocs_per_op = allocations as f64 / iterations as f64;

    println!("Hot Path Performance:");
    println!("  Time per operation: {:.2} ns", ns_per_op);
    println!("  Allocations per operation: {:.4}", allocs_per_op);
    println!("  Total allocations: {}", allocations);
    println!("  Total bytes allocated: {}", get_bytes_allocated());

    // Assert performance targets
    assert!(
        ns_per_op < 100.0,
        "Performance target not met: {:.2} ns/op (target: <100ns)",
        ns_per_op
    );
    assert_eq!(
        allocations, 0,
        "Hot path should have zero allocations after warmup, but had {}",
        allocations
    );
}

#[test]
fn test_allocation_regression_detection() {
    // This test intentionally triggers an allocation to verify our detection works
    reset_allocation_tracking();

    // This should trigger an allocation and be caught
    let allocation_detected = {
        let start = get_allocation_count();
        let _vec = vec![1, 2, 3]; // Intentional allocation
        let end = get_allocation_count();
        end > start
    };

    assert!(
        allocation_detected,
        "Allocation tracking is not working properly"
    );
}

/// Test that build_with_hot_path_buffer has exactly one allocation (for Vec return)
#[test]
fn test_build_with_hot_path_buffer_single_allocation() {
    let instrument_id = InstrumentId {
        venue: VenueId::Binance as u16,
        asset_type: 1,
        reserved: 0,
        asset_id: 55555,
    };
    let trade = TradeTLV::from_instrument(
        VenueId::Binance,
        instrument_id,
        300_000_000,
        150_000_000,
        1,
        1234567894,
    );

    // Warm up
    let _ = build_message_direct(
        RelayDomain::MarketData,
        SourceType::BinanceCollector,
        TLVType::Trade,
        &trade,
    );

    // Test: Expect exactly ONE allocation (for the Vec<u8> return value)
    reset_allocation_tracking();

    // For sequence number, we need to use TrueZeroCopyBuilder directly
    let message = with_hot_path_buffer(|buffer| {
        let builder =
            TrueZeroCopyBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
                .with_sequence(9999);

        let size = builder.build_into_buffer(buffer, TLVType::Trade, &trade)?;
        let result = buffer[..size].to_vec();
        Ok((result, size))
    })
    .unwrap();

    let allocations = get_allocation_count();

    // This is the expected and correct behavior!
    assert_eq!(
        allocations, 1,
        "Expected exactly 1 allocation for Vec<u8> return, got {}",
        allocations
    );

    // Verify the message is valid
    assert!(message.len() > 32);
}

/// Benchmark test to establish baseline performance
#[test]
#[ignore] // Run with --ignored flag for benchmarking
fn bench_hot_path_throughput() {
    let instrument_id = InstrumentId {
        venue: VenueId::Binance as u16,
        asset_type: 1,
        reserved: 0,
        asset_id: 99999,
    };
    let trade = TradeTLV::from_instrument(
        VenueId::Binance,
        instrument_id,
        500_000_000,
        100_000_000,
        0,
        1234567893,
    );

    // Warm up
    for _ in 0..10_000 {
        let _ = with_hot_path_buffer(|buffer| {
            TrueZeroCopyBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
                .build_into_buffer(buffer, TLVType::Trade, &trade)
                .map(|size| (size, size))
        });
    }

    // Benchmark
    let iterations = 1_000_000;
    let start = std::time::Instant::now();

    for i in 0..iterations {
        let _ = with_hot_path_buffer(|buffer| {
            let builder =
                TrueZeroCopyBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
                    .with_sequence(i);

            let size = builder.build_into_buffer(buffer, TLVType::Trade, &trade)?;

            // Simulate network send
            std::hint::black_box(&buffer[..size]);

            Ok(((), size))
        })
        .unwrap();
    }

    let duration = start.elapsed();
    let messages_per_second = iterations as f64 / duration.as_secs_f64();

    println!("Hot Path Throughput Benchmark:");
    println!("  Messages processed: {}", iterations);
    println!("  Total time: {:?}", duration);
    println!("  Throughput: {:.0} messages/second", messages_per_second);
    println!(
        "  Latency: {:.2} ns/message",
        duration.as_nanos() as f64 / iterations as f64
    );

    // Assert we meet the >1M messages/second target
    assert!(
        messages_per_second > 1_000_000.0,
        "Throughput target not met: {:.0} msg/s (target: >1M msg/s)",
        messages_per_second
    );
}
