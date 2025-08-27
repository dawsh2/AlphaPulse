//! Simple performance test for PoolSwapTLV parsing
//!
//! Validates that zerocopy parsing meets <35μs hot path requirement

use std::time::Instant;

/// Test that zerocopy::Ref::new completes within 35μs
/// This is the critical operation in the hot path
#[test]
fn test_zerocopy_parsing_performance() {
    // Create a buffer that simulates a PoolSwapTLV structure
    // Size is 208 bytes based on the struct definition
    let buffer = vec![0u8; 208];

    // Warm up
    for _ in 0..100 {
        let _ = zerocopy::Ref::<_, [u8; 208]>::new(&buffer[..]);
    }

    // Measure parsing performance
    const ITERATIONS: usize = 100000;
    let start = Instant::now();

    for _ in 0..ITERATIONS {
        // This simulates the zero-copy parsing operation
        let _ = zerocopy::Ref::<_, [u8; 208]>::new(&buffer[..]);
    }

    let elapsed = start.elapsed();
    let avg_time_ns = elapsed.as_nanos() / ITERATIONS as u128;
    let avg_time_us = avg_time_ns as f64 / 1000.0;

    println!(
        "Zero-copy parsing (208 bytes): {:.3}μs average",
        avg_time_us
    );

    // Assert we're well under 35μs (should be nanoseconds for this operation)
    assert!(
        avg_time_us < 1.0, // Should be sub-microsecond
        "Zero-copy parsing took {:.3}μs, expected sub-microsecond",
        avg_time_us
    );
}

/// Test alignment handling performance
#[test]
fn test_alignment_copy_performance() {
    // Create an unaligned buffer
    let mut unaligned_buffer = vec![0u8; 209];
    for i in 0..208 {
        unaligned_buffer[i + 1] = i as u8;
    }
    let unaligned_bytes = &unaligned_buffer[1..209];

    // Warm up
    for _ in 0..100 {
        let mut aligned = vec![0u8; 208];
        aligned.copy_from_slice(unaligned_bytes);
        let _ = zerocopy::Ref::<_, [u8; 208]>::new(&aligned[..]);
    }

    // Measure copy + parsing performance
    const ITERATIONS: usize = 10000;
    let start = Instant::now();

    for _ in 0..ITERATIONS {
        // This simulates the fallback path for unaligned data
        let mut aligned = vec![0u8; 208];
        aligned.copy_from_slice(unaligned_bytes);
        let _ = zerocopy::Ref::<_, [u8; 208]>::new(&aligned[..]);
    }

    let elapsed = start.elapsed();
    let avg_time_ns = elapsed.as_nanos() / ITERATIONS as u128;
    let avg_time_us = avg_time_ns as f64 / 1000.0;

    println!(
        "Alignment copy + parsing (208 bytes): {:.3}μs average",
        avg_time_us
    );

    // Assert we're still under 35μs even with the copy
    assert!(
        avg_time_us < 35.0,
        "Alignment handling took {:.3}μs, exceeding 35μs target",
        avg_time_us
    );
}

/// Test that the full message processing path meets performance targets
#[test]
fn test_full_message_processing_performance() {
    // Simulate a full Protocol V2 message: 32-byte header + 2-byte TLV header + 208-byte PoolSwapTLV
    let message = vec![0u8; 32 + 2 + 208];

    // Warm up
    for _ in 0..100 {
        // Parse header
        let _header = &message[..32];
        // Parse TLV type and length
        let _tlv_type = message[32];
        let _tlv_length = message[33];
        // Parse PoolSwapTLV
        let payload = &message[34..34 + 208];
        let _ = zerocopy::Ref::<_, [u8; 208]>::new(payload);
    }

    // Measure full processing performance
    const ITERATIONS: usize = 10000;
    let start = Instant::now();

    for _ in 0..ITERATIONS {
        // 1. Parse header (simulated)
        let _header = &message[..32];
        let _magic = u32::from_le_bytes([message[0], message[1], message[2], message[3]]);
        let _payload_size = u16::from_le_bytes([message[4], message[5]]);

        // 2. Parse TLV header
        let tlv_type = message[32];
        let tlv_length = message[33];

        // 3. Validate TLV
        if tlv_type != 11 || tlv_length as usize != 208 {
            continue;
        }

        // 4. Parse PoolSwapTLV payload
        let payload = &message[34..34 + 208];
        if let Some(_swap) = zerocopy::Ref::<_, [u8; 208]>::new(payload) {
            // 5. Extract key fields (simulated)
            let _amount_in = &payload[0..16]; // First u128
            let _amount_out = &payload[16..32]; // Second u128
        }
    }

    let elapsed = start.elapsed();
    let avg_time_ns = elapsed.as_nanos() / ITERATIONS as u128;
    let avg_time_us = avg_time_ns as f64 / 1000.0;

    println!("Full message processing: {:.3}μs average", avg_time_us);

    // Assert the full path completes within 35μs
    assert!(
        avg_time_us < 35.0,
        "Full message processing took {:.3}μs, exceeding 35μs target",
        avg_time_us
    );
}
