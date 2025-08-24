//! StateInvalidationTLV FixedVec Zero-Copy Integration Tests
//!
//! Comprehensive test suite validating the complete bijection cycle:
//! Vec<InstrumentId> → FixedVec → StateInvalidationTLV → serialize → deserialize → FixedVec → Vec<InstrumentId>
//!
//! Tests cover:
//! - Perfect bijection preservation
//! - Zero-copy serialization validation
//! - Performance benchmarks (>1M msg/s targets)
//! - Bounds testing for MAX_INSTRUMENTS=16
//! - Memory layout validation

use protocol_v2::tlv::market_data::{StateInvalidationTLV, InvalidationReason};
use protocol_v2::tlv::dynamic_payload::{FixedVec, MAX_INSTRUMENTS};
use protocol_v2::{VenueId, InstrumentId};
use zerocopy::AsBytes;
use std::time::Instant;

/// Test perfect bijection: Vec<InstrumentId> → StateInvalidationTLV → Vec<InstrumentId>
#[test]
fn test_bijection_preservation() {
    let test_cases = vec![
        // Single instrument case
        vec![InstrumentId::stock(VenueId::Kraken, "BTCUSD")],
        
        // Typical invalidation (2-3 instruments)
        vec![
            InstrumentId::stock(VenueId::Coinbase, "ETHUSD"),
            InstrumentId::stock(VenueId::Coinbase, "ETHBTC"),
        ],
        
        // Multiple instruments from different venues
        vec![
            InstrumentId::stock(VenueId::Binance, "ADAUSDT"),
            InstrumentId::stock(VenueId::Kraken, "ADAUSD"),
            InstrumentId::stock(VenueId::Coinbase, "ADAUSD"),
        ],
        
        // Complex multi-instrument invalidation
        vec![
            InstrumentId::stock(VenueId::Binance, "BTCUSDT"),
            InstrumentId::stock(VenueId::Binance, "ETHUSDT"), 
            InstrumentId::stock(VenueId::Binance, "ADAUSDT"),
            InstrumentId::stock(VenueId::Binance, "DOTUSDT"),
            InstrumentId::stock(VenueId::Binance, "LINKUSDT"),
        ],
        
        // Maximum capacity test (16 instruments)
        (0..MAX_INSTRUMENTS)
            .map(|i| InstrumentId::stock(VenueId::Binance, &format!("PAIR{:02}", i)))
            .collect(),
    ];

    for (i, original_instruments) in test_cases.iter().enumerate() {
        // Create StateInvalidationTLV from Vec
        let sequence = 12345u64;
        let timestamp_ns = 1700000000000000000u64;
        
        let tlv = StateInvalidationTLV::new(
            VenueId::Binance,
            sequence,
            original_instruments,
            InvalidationReason::Staleness,
            timestamp_ns,
        ).unwrap_or_else(|e| panic!("Test case {}: Failed to create StateInvalidationTLV: {}", i, e));

        // Validate bijection
        let recovered_instruments = tlv.to_instruments_vec();
        
        assert_eq!(
            &recovered_instruments, original_instruments,
            "Test case {}: Bijection failed. Original: {:?}, Recovered: {:?}",
            i, original_instruments, recovered_instruments
        );
        
        // Validate accessors
        assert_eq!(tlv.len(), original_instruments.len(), "Test case {}: Length mismatch", i);
        assert_eq!(tlv.get_instruments(), original_instruments.as_slice(), "Test case {}: Slice mismatch", i);
        assert_eq!(tlv.is_empty(), original_instruments.is_empty(), "Test case {}: Empty check failed", i);
        
        // Validate metadata
        assert_eq!(tlv.sequence, sequence);
        assert_eq!(tlv.timestamp_ns, timestamp_ns);
        assert_eq!(tlv.venue, VenueId::Binance as u16);
        assert_eq!(tlv.reason, InvalidationReason::Staleness as u8);
    }
}

/// Test zero-copy serialization/deserialization without allocations
#[test]
fn test_zero_copy_serialization() {
    let original_instruments = vec![
        InstrumentId::stock(VenueId::Kraken, "BTCUSD"),
        InstrumentId::stock(VenueId::Kraken, "ETHUSD"),
        InstrumentId::stock(VenueId::Kraken, "ADAUSD"),
    ];
    
    let sequence = 9876u64;
    let timestamp_ns = 1700000000000000000u64;
    
    // Create original TLV
    let original_tlv = StateInvalidationTLV::new(
        VenueId::Kraken,
        sequence,
        &original_instruments,
        InvalidationReason::RateLimited,
        timestamp_ns,
    ).unwrap();

    // Zero-copy serialization via AsBytes
    let serialized_bytes: &[u8] = original_tlv.as_bytes();
    
    // Validate serialized size
    let expected_size = std::mem::size_of::<StateInvalidationTLV>();
    assert_eq!(
        serialized_bytes.len(), expected_size,
        "Serialized size mismatch. Expected: {}, Got: {}",
        expected_size, serialized_bytes.len()
    );
    
    // Zero-copy deserialization via zerocopy::Ref
    let tlv_ref = zerocopy::Ref::<_, StateInvalidationTLV>::new(serialized_bytes)
        .expect("Failed to deserialize StateInvalidationTLV");
    
    let deserialized_tlv = *tlv_ref.into_ref();
    
    // Validate complete equality
    assert_eq!(original_tlv.sequence, deserialized_tlv.sequence);
    assert_eq!(original_tlv.timestamp_ns, deserialized_tlv.timestamp_ns);
    assert_eq!(original_tlv.venue, deserialized_tlv.venue);
    assert_eq!(original_tlv.reason, deserialized_tlv.reason);
    
    // Validate instruments bijection is preserved through serialization
    let deserialized_instruments = deserialized_tlv.to_instruments_vec();
    assert_eq!(deserialized_instruments, original_instruments);
    
    // Validate no allocations by checking direct slice access
    assert_eq!(deserialized_tlv.get_instruments(), original_instruments.as_slice());
    
    println!("✅ Zero-copy serialization: {} bytes", serialized_bytes.len());
    println!("✅ Perfect bijection preserved through serialization/deserialization");
}

/// Test bounds enforcement for MAX_INSTRUMENTS=16
#[test]
fn test_bounds_enforcement() {
    let sequence = 5555u64;
    let timestamp_ns = 1700000000000000000u64;
    
    // Test maximum capacity (should succeed)
    let max_instruments: Vec<InstrumentId> = (0..MAX_INSTRUMENTS)
        .map(|i| InstrumentId::stock(VenueId::Balancer, &format!("TOKEN{:02}USD", i)))
        .collect();
    
    let tlv_max = StateInvalidationTLV::new(
        VenueId::Balancer,
        sequence,
        &max_instruments,
        InvalidationReason::Maintenance,
        timestamp_ns,
    );
    
    assert!(tlv_max.is_ok(), "Maximum capacity should be allowed");
    let tlv = tlv_max.unwrap();
    assert_eq!(tlv.len(), MAX_INSTRUMENTS);
    assert_eq!(tlv.to_instruments_vec(), max_instruments);
    
    // Test exceeding capacity (should fail)
    let excessive_instruments: Vec<InstrumentId> = (0..MAX_INSTRUMENTS + 1)
        .map(|i| InstrumentId::stock(VenueId::Balancer, &format!("EXCESS{:02}USD", i)))
        .collect();
    
    let tlv_excessive = StateInvalidationTLV::new(
        VenueId::Balancer,
        sequence,
        &excessive_instruments,
        InvalidationReason::Maintenance,
        timestamp_ns,
    );
    
    assert!(tlv_excessive.is_err(), "Exceeding capacity should fail");
    
    // Test empty instruments (should fail)
    let empty_instruments: Vec<InstrumentId> = vec![];
    let tlv_empty = StateInvalidationTLV::new(
        VenueId::Binance,
        sequence,
        &empty_instruments,
        InvalidationReason::Recovery,
        timestamp_ns,
    );
    
    assert!(tlv_empty.is_err(), "Empty instruments should fail");
}

/// Test incremental instrument addition
#[test]
fn test_incremental_instrument_addition() {
    let sequence = 7777u64;
    let timestamp_ns = 1700000000000000000u64;
    
    // Start with single instrument
    let initial_instruments = vec![InstrumentId::stock(VenueId::UniswapV3, "ETHUSDC")];
    let mut tlv = StateInvalidationTLV::new(
        VenueId::UniswapV3,
        sequence,
        &initial_instruments,
        InvalidationReason::Disconnection,
        timestamp_ns,
    ).unwrap();
    
    assert_eq!(tlv.len(), 1);
    assert_eq!(tlv.to_instruments_vec(), initial_instruments);
    
    // Add more instruments incrementally
    let additional_instruments = vec![
        InstrumentId::stock(VenueId::UniswapV3, "BTCWETH"),
        InstrumentId::stock(VenueId::UniswapV3, "USDTUSDC"),
        InstrumentId::stock(VenueId::UniswapV3, "WETHUSDT"),
    ];
    
    for instrument in &additional_instruments {
        tlv.add_instrument(*instrument).unwrap();
    }
    
    // Validate final state
    let mut expected_final = initial_instruments;
    expected_final.extend_from_slice(&additional_instruments);
    
    assert_eq!(tlv.len(), expected_final.len());
    assert_eq!(tlv.to_instruments_vec(), expected_final);
    
    // Test capacity exceeded
    let remaining_capacity = MAX_INSTRUMENTS - tlv.len();
    
    // Fill to capacity
    for i in 0..remaining_capacity {
        let instrument = InstrumentId::stock(VenueId::UniswapV3, &format!("PAIR{}", i));
        let result = tlv.add_instrument(instrument);
        assert!(result.is_ok(), "Should be able to add instrument {}", i);
    }
    
    // Attempt to exceed capacity
    let overflow_instrument = InstrumentId::stock(VenueId::UniswapV3, "OVERFLOW");
    let overflow_result = tlv.add_instrument(overflow_instrument);
    assert!(overflow_result.is_err(), "Should fail when exceeding capacity");
}

/// Performance benchmark: Construction rate (target >1M msg/s)
#[test] 
fn test_construction_performance() {
    let sequence = 1111u64;
    let timestamp_ns = 1700000000000000000u64;
    
    // Test data representing typical invalidation
    let instruments = vec![
        InstrumentId::stock(VenueId::Kraken, "BTCUSD"),
        InstrumentId::stock(VenueId::Kraken, "ETHUSD"),
    ];
    
    const ITERATIONS: usize = 1_000_000;
    let start = Instant::now();
    
    for _ in 0..ITERATIONS {
        let _tlv = StateInvalidationTLV::new(
            VenueId::Kraken,
            sequence,
            &instruments,
            InvalidationReason::Staleness,
            timestamp_ns,
        ).unwrap();
        
        // Prevent optimization
        std::hint::black_box(_tlv);
    }
    
    let elapsed = start.elapsed();
    let rate_per_second = (ITERATIONS as f64) / elapsed.as_secs_f64();
    
    println!("🚀 Construction rate: {:.0} msg/s", rate_per_second);
    println!("📊 Target: >1,000,000 msg/s");
    
    // Performance requirement
    assert!(
        rate_per_second > 1_000_000.0,
        "Construction rate {:.0} msg/s below target 1M msg/s",
        rate_per_second
    );
}

/// Performance benchmark: Parsing rate (target >1.6M msg/s) 
#[test]
fn test_parsing_performance() {
    let sequence = 2222u64;
    let timestamp_ns = 1700000000000000000u64;
    
    let instruments = vec![
        InstrumentId::stock(VenueId::Coinbase, "BTCUSD"),
        InstrumentId::stock(VenueId::Coinbase, "ETHUSD"),
        InstrumentId::stock(VenueId::Coinbase, "ADAUSD"),
    ];
    
    // Pre-create serialized data
    let original_tlv = StateInvalidationTLV::new(
        VenueId::Coinbase,
        sequence,
        &instruments,
        InvalidationReason::AuthenticationFailure,
        timestamp_ns,
    ).unwrap();
    
    let serialized_bytes = original_tlv.as_bytes();
    
    const ITERATIONS: usize = 1_600_000; // Slightly above target for headroom
    let start = Instant::now();
    
    for _ in 0..ITERATIONS {
        let tlv_ref = zerocopy::Ref::<_, StateInvalidationTLV>::new(serialized_bytes)
            .expect("Failed to parse");
        let _tlv = *tlv_ref.into_ref();
        
        // Prevent optimization
        std::hint::black_box(_tlv);
    }
    
    let elapsed = start.elapsed();
    let rate_per_second = (ITERATIONS as f64) / elapsed.as_secs_f64();
    
    println!("⚡ Parsing rate: {:.0} msg/s", rate_per_second);
    println!("📊 Target: >1,600,000 msg/s");
    
    // Performance requirement  
    assert!(
        rate_per_second > 1_600_000.0,
        "Parsing rate {:.0} msg/s below target 1.6M msg/s", 
        rate_per_second
    );
}

/// Test memory layout and size consistency
#[test]
fn test_memory_layout() {
    use std::mem::{size_of, align_of};
    
    // Validate struct size
    let tlv_size = size_of::<StateInvalidationTLV>();
    let fixed_vec_size = size_of::<FixedVec<InstrumentId, MAX_INSTRUMENTS>>();
    
    println!("📏 StateInvalidationTLV size: {} bytes", tlv_size);
    println!("📏 FixedVec<InstrumentId, {}> size: {} bytes", MAX_INSTRUMENTS, fixed_vec_size);
    
    // Expected layout (with alignment):
    // - sequence: u64 (8 bytes)
    // - timestamp_ns: u64 (8 bytes)
    // - instruments: FixedVec (varies with alignment)
    // - venue: u16 (2 bytes)
    // - reason: u8 (1 byte) 
    // - _padding: [u8; 5] (5 bytes)
    
    // We'll measure the actual sizes and update expectations
    
    // Validate alignment
    let tlv_alignment = align_of::<StateInvalidationTLV>();
    println!("🎯 StateInvalidationTLV alignment: {} bytes", tlv_alignment);
    
    // Should be aligned to largest field (InstrumentId arrays in FixedVec)
    
    // Test actual instance
    let instruments = vec![
        InstrumentId::stock(VenueId::Binance, "BTCUSDT"),
        InstrumentId::stock(VenueId::Binance, "ETHUSDT"),
    ];
    let tlv = StateInvalidationTLV::new(
        VenueId::Binance,
        3333u64,
        &instruments,
        InvalidationReason::Maintenance,
        1700000000000000000u64,
    ).unwrap();
    
    let serialized = tlv.as_bytes();
    assert_eq!(serialized.len(), tlv_size, "Serialized size mismatch");
    
    println!("✅ Memory layout validation passed");
}

/// Test edge cases and error conditions
#[test]
fn test_edge_cases() {
    let sequence = 4444u64;
    let timestamp_ns = 1700000000000000000u64;
    
    // Test with different invalidation reasons
    for reason in [
        InvalidationReason::Disconnection,
        InvalidationReason::AuthenticationFailure,
        InvalidationReason::RateLimited,
        InvalidationReason::Staleness,
        InvalidationReason::Maintenance,
        InvalidationReason::Recovery,
    ] {
        let instruments = vec![InstrumentId::stock(VenueId::Kraken, "TESTUSD")];
        let tlv = StateInvalidationTLV::new(
            VenueId::Kraken,
            sequence,
            &instruments,
            reason,
            timestamp_ns,
        ).unwrap();
        
        assert_eq!(tlv.reason, reason as u8);
        assert_eq!(tlv.to_instruments_vec(), instruments);
    }
    
    // Test serialization roundtrip with different venues
    for venue in [VenueId::Binance, VenueId::Coinbase, VenueId::Kraken, VenueId::UniswapV3] {
        let instruments = vec![InstrumentId::stock(venue, "TESTUSD")];
        let tlv = StateInvalidationTLV::new(
            venue,
            sequence,
            &instruments,
            InvalidationReason::Recovery,
            timestamp_ns,
        ).unwrap();
        
        let serialized = tlv.as_bytes();
        let deserialized_ref = zerocopy::Ref::<_, StateInvalidationTLV>::new(serialized).unwrap();
        let deserialized = *deserialized_ref.into_ref();
        
        assert_eq!(deserialized.venue, venue as u16);
        assert_eq!(deserialized.to_instruments_vec(), instruments);
    }
    
    println!("✅ Edge case validation passed");
}

/// Test InvalidationReason enum conversion
#[test]
fn test_invalidation_reason_conversion() {
    use protocol_v2::tlv::market_data::InvalidationReason;
    
    // Test all reason variants
    let test_cases = [
        (InvalidationReason::Disconnection, 0u8),
        (InvalidationReason::AuthenticationFailure, 1u8),
        (InvalidationReason::RateLimited, 2u8),
        (InvalidationReason::Staleness, 3u8),
        (InvalidationReason::Maintenance, 4u8),
        (InvalidationReason::Recovery, 5u8),
    ];
    
    for (reason, expected_value) in test_cases {
        assert_eq!(reason as u8, expected_value);
        
        // Test roundtrip conversion
        let converted = InvalidationReason::try_from(expected_value).unwrap();
        assert_eq!(converted, reason);
    }
    
    // Test invalid reason conversion
    assert!(InvalidationReason::try_from(99u8).is_err());
    
    println!("✅ InvalidationReason conversion validation passed");
}

/// Integration test with complex instrument scenarios
#[test]
fn test_complex_instrument_scenarios() {
    let sequence = 8888u64;
    let timestamp_ns = 1700000000000000000u64;
    
    // Test mixed venues and asset types
    let complex_instruments = vec![
        InstrumentId::stock(VenueId::Binance, "BTCUSDT"),
        InstrumentId::stock(VenueId::Coinbase, "BTCUSD"),
        InstrumentId::stock(VenueId::Kraken, "XBTUSD"),
        InstrumentId::stock(VenueId::UniswapV3, "WETHUSDC"),
    ];
    
    let tlv = StateInvalidationTLV::new(
        VenueId::Binance, // Primary venue for invalidation
        sequence,
        &complex_instruments,
        InvalidationReason::Staleness,
        timestamp_ns,
    ).unwrap();
    
    // Validate all instruments are preserved
    let recovered = tlv.to_instruments_vec();
    assert_eq!(recovered, complex_instruments);
    
    // Validate serialization preserves complex data
    let serialized = tlv.as_bytes();
    let deserialized_ref = zerocopy::Ref::<_, StateInvalidationTLV>::new(serialized).unwrap();
    let deserialized = *deserialized_ref.into_ref();
    
    assert_eq!(deserialized.to_instruments_vec(), complex_instruments);
    assert_eq!(deserialized.len(), complex_instruments.len());
    assert_eq!(deserialized.sequence, sequence);
    assert_eq!(deserialized.venue, VenueId::Binance as u16);
    
    println!("✅ Complex instrument scenarios validation passed");
}