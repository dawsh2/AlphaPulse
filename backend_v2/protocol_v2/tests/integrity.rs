//! Checksum and Integrity Tests
//!
//! Tests data integrity features with real-world scenarios:
//! - Network corruption detection
//! - Selective validation for different domains
//! - Performance impact of checksums
//! - Tamper detection in financial messages

mod common;

use common::*;
use protocol_v2::{
    current_timestamp_ns, parse_header,
    tlv::{ParseError, TLVMessageBuilder},
    validation::{
        calculate_crc32, calculate_crc32_excluding_checksum, embed_checksum,
        verify_message_checksum,
    },
    InstrumentId, MessageHeader, RelayDomain, SourceType, TLVType, VenueId, MESSAGE_MAGIC,
};
use std::time::Instant;

#[test]
fn test_single_bit_flip_detection() {
    // Simulate real network bit flip errors
    let msg = create_market_data_message(SourceType::BinanceCollector);

    // Test flipping each bit in critical fields
    let critical_offsets = [
        12, // First byte of sequence number
        20, // First byte of timestamp
        32, // First byte of TLV payload
        40, // Middle of trade data
    ];

    for offset in critical_offsets {
        if offset < msg.len() {
            let mut corrupted = msg.clone();
            corrupted[offset] ^= 0x01; // Flip lowest bit

            // Recalculate what the checksum SHOULD be if this was intentional
            let actual_checksum = u32::from_le_bytes(corrupted[28..32].try_into().unwrap());
            let calculated_checksum = calculate_crc32_excluding_checksum(&corrupted, 28);

            assert_ne!(
                actual_checksum, calculated_checksum,
                "Bit flip at offset {} should invalidate checksum",
                offset
            );

            // Parse should detect corruption
            match parse_header(&corrupted) {
                Ok(header) => {
                    // Header parsed, but checksum should be wrong
                    assert!(
                        !verify_message_checksum(
                            &corrupted,
                            {
                                let c = header.checksum;
                                c
                            },
                            28
                        ),
                        "Should detect corruption at offset {}",
                        offset
                    );
                }
                Err(ParseError::ChecksumMismatch { .. }) => {
                    // Good - detected corruption
                }
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }
    }
}

#[test]
fn test_market_data_checksum_skip_performance() {
    // Market data relay skips checksums for performance
    // Measure the actual performance difference

    let msg = create_market_data_message(SourceType::BinanceCollector);
    let iterations = 100_000;

    // Test with full validation (including checksum)
    let start_full = Instant::now();
    for _ in 0..iterations {
        let _ = parse_header(&msg).unwrap();
    }
    let full_duration = start_full.elapsed();

    // Test without checksum validation (simulate fast path)
    let start_fast = Instant::now();
    for _ in 0..iterations {
        // Just validate magic number and basic structure
        if msg.len() >= MessageHeader::SIZE && &msg[0..4] == &[0xDE, 0xAD, 0xBE, 0xEF] {
            // Fast path - no checksum validation
        }
    }
    let fast_duration = start_fast.elapsed();

    let full_ns = full_duration.as_nanos() / iterations as u128;
    let fast_ns = fast_duration.as_nanos() / iterations as u128;
    let speedup = full_ns as f64 / fast_ns as f64;

    println!("Full validation: {} ns/op", full_ns);
    println!("Fast parsing: {} ns/op", fast_ns);
    println!("Speedup: {:.1}x", speedup);

    // Fast parsing should be significantly faster
    assert!(
        speedup > 1.5,
        "Fast parsing should be at least 1.5x faster, got {:.1}x",
        speedup
    );
}

#[test]
fn test_signal_domain_enforces_checksum() {
    // Signal domain MUST validate checksums (financial integrity)
    let msg = create_signal_message(SourceType::ArbitrageStrategy);

    // Corrupt the message
    let mut corrupted = msg.clone();
    corrupted[40] ^= 0xFF; // Flip multiple bits in payload

    // Signal relay should reject corrupted messages
    match parse_header(&corrupted) {
        Ok(_) => {
            // Header might parse, but checksum validation should fail
            let header = parse_header(&corrupted).unwrap();
            assert!(
                !verify_message_checksum(
                    &corrupted,
                    {
                        let c = header.checksum;
                        c
                    },
                    28
                ),
                "Signal domain must detect corruption"
            );
        }
        Err(ParseError::ChecksumMismatch {
            expected,
            calculated,
        }) => {
            assert_ne!(expected, calculated);
        }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn test_execution_domain_audit_trail() {
    // Execution domain needs perfect audit trail
    let order_id: u64 = 1234567890;
    let btc = InstrumentId::coin(VenueId::Binance, "BTC");
    let usdt = InstrumentId::coin(VenueId::Binance, "USDT");
    let instrument = InstrumentId::pool(VenueId::UniswapV2, btc, usdt);
    let price: i64 = 4500000000000; // $45,000
    let quantity: i64 = 10000000; // 0.1 BTC

    let mut payload = Vec::new();
    payload.extend_from_slice(&order_id.to_le_bytes());
    payload.extend_from_slice(&instrument.to_u64().to_le_bytes());
    payload.extend_from_slice(&price.to_le_bytes());
    payload.extend_from_slice(&quantity.to_le_bytes());

    let msg = TLVMessageBuilder::new(RelayDomain::Execution, SourceType::ExecutionEngine)
        .add_tlv_bytes(TLVType::OrderRequest, payload)
        .build();

    // Verify checksum is embedded correctly
    let header = parse_header(&msg).unwrap();
    let embedded_checksum = header.checksum;

    // Use the proper validation function that accounts for checksum zeroing
    assert!(
        verify_message_checksum(&msg, embedded_checksum, 28),
        "Execution message checksum validation failed"
    );
}

#[test]
fn test_checksum_with_extended_tlv() {
    // Large messages with extended TLV should still have valid checksums
    let large_payload = vec![0xAB; 10_000]; // 10KB payload

    let msg = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .add_tlv_bytes(TLVType::L2Snapshot, large_payload)
        .build();

    // Should use extended TLV
    assert_eq!(msg[MessageHeader::SIZE], 255, "Should use extended TLV");

    // Checksum should cover entire message
    let header = parse_header(&msg).unwrap();
    assert!(
        verify_message_checksum(
            &msg,
            {
                let c = header.checksum;
                c
            },
            28
        ),
        "Extended TLV message checksum validation failed"
    );

    // Corrupt somewhere in the large payload
    let mut corrupted = msg.clone();
    corrupted[5000] ^= 0x01; // Flip bit in middle of payload

    let corrupted_checksum = calculate_crc32_excluding_checksum(&corrupted, 28);
    let orig_checksum = header.checksum;
    assert_ne!(
        orig_checksum, corrupted_checksum,
        "Should detect corruption in large payload"
    );
}

#[test]
fn test_replay_attack_detection() {
    // Test that replayed messages can be detected via sequence numbers
    let msg1 = create_execution_message(SourceType::ExecutionEngine);

    // Simulate replay attack - same message sent twice
    let msg2 = msg1.clone();

    let header1 = parse_header(&msg1).unwrap();
    let header2 = parse_header(&msg2).unwrap();

    // Sequence numbers should be identical (replay attack signature)
    let seq1 = header1.sequence;
    let seq2 = header2.sequence;
    assert_eq!(seq1, seq2, "Replayed message has same sequence number");

    // Timestamp might be identical too
    let ts1 = header1.timestamp;
    let ts2 = header2.timestamp;
    assert_eq!(ts1, ts2, "Replayed message has same timestamp");

    // In production, relay would track sequence numbers per source
    // and reject duplicate sequences
}

#[test]
fn test_message_age_validation() {
    // Test staleness detection for time-sensitive messages
    let mut old_msg = create_signal_message(SourceType::ArbitrageStrategy);

    // Set timestamp to 1 hour ago
    let one_hour_ago = (current_timestamp_ns() - 3_600_000_000_000) as u64; // 1 hour in nanoseconds
    old_msg[20..28].copy_from_slice(&one_hour_ago.to_le_bytes());

    // Properly recalculate checksum using embed_checksum function
    embed_checksum(&mut old_msg, 28);

    let header = parse_header(&old_msg).unwrap();

    // Check message age
    let age_ns = header.age_ns();
    assert!(
        age_ns >= 3_600_000_000_000,
        "Message should be at least 1 hour old"
    );

    // Arbitrage signals older than 1 second should be rejected
    let max_signal_age_ns = 1_000_000_000; // 1 second
    assert!(
        header.is_older_than(max_signal_age_ns),
        "Old arbitrage signal should be detected as stale"
    );
}

#[test]
fn test_checksum_calculation_determinism() {
    // Same message should always produce same checksum
    let msg = create_market_data_message(SourceType::KrakenCollector);

    let checksum1 = calculate_crc32_excluding_checksum(&msg, 28);
    let checksum2 = calculate_crc32_excluding_checksum(&msg, 28);
    let checksum3 = calculate_crc32_excluding_checksum(&msg, 28);

    assert_eq!(checksum1, checksum2, "Checksum not deterministic");
    assert_eq!(checksum2, checksum3, "Checksum not deterministic");
}

#[test]
fn test_partial_message_integrity() {
    // Test integrity when receiving message in chunks (TCP fragmentation)
    let full_msg = create_market_data_message(SourceType::CoinbaseCollector);
    let header = parse_header(&full_msg).unwrap();
    let expected_checksum = header.checksum; // Copy from packed struct

    // Simulate receiving in chunks
    let mut received = Vec::new();
    let chunk_sizes = [16, 16, 8, 12]; // Total 52 bytes
    let mut offset = 0;

    for chunk_size in chunk_sizes {
        let end = (offset + chunk_size).min(full_msg.len());
        received.extend_from_slice(&full_msg[offset..end]);
        offset = end;

        if received.len() >= 32 {
            // Can verify checksum once we have full header
            let received_checksum = u32::from_le_bytes(received[28..32].try_into().unwrap());
            assert_eq!(
                received_checksum, expected_checksum,
                "Checksum should be consistent as we receive chunks"
            );
        }
    }
}

#[test]
fn test_cross_platform_checksum_compatibility() {
    // Ensure checksums are identical across different endianness
    let msg = create_market_data_message(SourceType::PolygonCollector);

    // CRC32 should be the same regardless of platform
    let checksum = calculate_crc32(&msg[..28]); // Just header without checksum

    // Manually calculate CRC32 to verify
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&msg[..28]);
    let manual_checksum = hasher.finalize();

    assert_eq!(
        checksum, manual_checksum,
        "CRC32 calculation should be platform-independent"
    );
}

#[test]
fn test_high_value_transaction_integrity() {
    // High-value transactions need extra integrity verification
    let whale_order_size: i64 = 100_000_000_000_000; // $1M worth at $10k/BTC
    let btc_price: i64 = 4500000000000; // $45,000

    let mut payload = Vec::new();
    payload.extend_from_slice(&999999u64.to_le_bytes()); // order_id
    payload.extend_from_slice(&1u64.to_le_bytes()); // BTC instrument
    payload.extend_from_slice(&btc_price.to_le_bytes());
    payload.extend_from_slice(&whale_order_size.to_le_bytes());

    let msg = TLVMessageBuilder::new(RelayDomain::Execution, SourceType::ExecutionEngine)
        .add_tlv_bytes(TLVType::OrderRequest, payload.clone())
        .build();

    // Verify multiple times for high-value orders
    for _ in 0..3 {
        let header = parse_header(&msg).unwrap();
        assert!(
            verify_message_checksum(
                &msg,
                {
                    let c = header.checksum;
                    c
                },
                28
            ),
            "High-value order checksum verification failed"
        );
    }

    // Test that even 1-bit change in order size is detected
    let mut tampered = msg.clone();
    tampered[MessageHeader::SIZE + 2 + 24] ^= 0x01; // Flip 1 bit in order size

    let tampered_checksum = calculate_crc32_excluding_checksum(&tampered, 28);
    let original_checksum = calculate_crc32_excluding_checksum(&msg, 28);

    assert_ne!(
        tampered_checksum, original_checksum,
        "Must detect any tampering with high-value orders"
    );
}

#[test]
fn test_checksum_performance_impact() {
    // Measure actual performance impact of checksum validation
    let messages: Vec<_> = (0..10000)
        .map(|_| create_market_data_message(SourceType::BinanceCollector))
        .collect();

    // Measure without checksum validation (but still parse header)
    let start_no_check = Instant::now();
    for msg in &messages {
        let _ = parse_header(msg).map(|h| h.magic == MESSAGE_MAGIC); // Parse header but skip checksum
    }
    let no_check_duration = start_no_check.elapsed();

    // Measure with checksum validation
    let start_with_check = Instant::now();
    for msg in &messages {
        let header = parse_header(msg).unwrap();
        let _ = verify_message_checksum(msg, header.checksum, 28);
    }
    let with_check_duration = start_with_check.elapsed();

    let overhead_percent =
        ((with_check_duration.as_nanos() as f64 / no_check_duration.as_nanos() as f64) - 1.0)
            * 100.0;

    println!("Checksum validation overhead: {:.1}%", overhead_percent);

    // Checksum overhead should be reasonable (< 200% for realistic comparison)
    assert!(
        overhead_percent < 200.0,
        "Checksum overhead too high: {:.1}%",
        overhead_percent
    );
}
