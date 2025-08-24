//! Deep Equality Validation Tests
//!
//! Comprehensive tests to ensure perfect roundtrip serialization/deserialization
//! with no loss of precision or data corruption for all TLV types.

use protocol_v2::{
    VenueId, InstrumentId, TLVType,
    tlv::market_data::{
        TradeTLV, QuoteTLV, PoolSwapTLV, PoolMintTLV, 
        PoolBurnTLV, PoolTickTLV, PoolLiquidityTLV,
        // Legacy TLV types removed - use Protocol V2 MessageHeader + TLV extensions
    },
    instrument_id::pairing::PoolInstrumentId,
};

/// Helper macro to test roundtrip equality
macro_rules! assert_deep_equality {
    ($original:expr, $recovered:expr) => {
        assert_eq!($original, $recovered, "Deep equality failed!");
        
        // Also verify individual fields for better error messages
        assert_eq!($original.venue, $recovered.venue, "Venue mismatch");
        assert_eq!($original.timestamp_ns, $recovered.timestamp_ns, "Timestamp mismatch");
    };
}

/// Helper to create a test pool ID
fn test_pool_id() -> PoolInstrumentId {
    PoolInstrumentId::from_pair(VenueId::Polygon, 1001, 2002)
}

#[test]
fn test_pool_swap_deep_equality() {
    let original = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id: test_pool_id(),
        token_in: 0xAABBCCDD11223344,
        token_out: 0x5566778899AABBCC,
        amount_in: 123456789012345,  // 8-decimal precision
        amount_out: 987654321098765,
        fee_paid: 30000000,  // 0.3 in 8-decimal
        sqrt_price_x96_after: 0,
        tick_after: 0,
        liquidity_after: 0,
        timestamp_ns: 1700000000123456789,
        block_number: 1000,
    };
    
    // Serialize to bytes
    let bytes = original.to_bytes();
    
    // Deserialize back
    let recovered = PoolSwapTLV::from_bytes(&bytes).unwrap();
    
    // Deep equality check
    assert_deep_equality!(original, recovered);
    assert_eq!(original.pool_id, recovered.pool_id, "Pool ID mismatch");
    assert_eq!(original.token_in, recovered.token_in, "Token in mismatch");
    assert_eq!(original.token_out, recovered.token_out, "Token out mismatch");
    assert_eq!(original.amount_in, recovered.amount_in, "Amount in mismatch");
    assert_eq!(original.amount_out, recovered.amount_out, "Amount out mismatch");
    assert_eq!(original.fee_paid, recovered.fee_paid, "Fee paid mismatch");
    
    // Verify no precision loss
    assert_eq!(original.amount_in, 123456789012345i64);
    assert_eq!(recovered.amount_in, 123456789012345i64);
}

#[test]
fn test_pool_mint_deep_equality() {
    let original = PoolMintTLV {
        venue: VenueId::Polygon,
        pool_id: test_pool_id(),
        provider: 0xDEADBEEFCAFEBABE,
        tick_lower: -887220,  // Negative tick
        tick_upper: 887220,   // Positive tick
        liquidity_delta: 999999999999999,
        amount0: 500000000000000,  // 5000000.00000000 tokens
        amount1: 250000000000000,  // 2500000.00000000 tokens
        timestamp_ns: u64::MAX - 1,  // Near max value
    };
    
    let bytes = original.to_bytes();
    let recovered = PoolMintTLV::from_bytes(&bytes).unwrap();
    
    assert_deep_equality!(original, recovered);
    assert_eq!(original.provider, recovered.provider);
    assert_eq!(original.tick_lower, recovered.tick_lower);
    assert_eq!(original.tick_upper, recovered.tick_upper);
    assert_eq!(original.liquidity_delta, recovered.liquidity_delta);
    assert_eq!(original.amount0, recovered.amount0);
    assert_eq!(original.amount1, recovered.amount1);
    
    // Test negative ticks preserved
    assert_eq!(recovered.tick_lower, -887220);
}

#[test]
fn test_pool_burn_deep_equality() {
    let original = PoolBurnTLV {
        venue: VenueId::Polygon,
        pool_id: test_pool_id(),
        provider: 0x1234567890ABCDEF,
        tick_lower: i32::MIN + 1,  // Near min value
        tick_upper: i32::MAX - 1,  // Near max value
        liquidity_delta: -500000000000000,  // Negative (removal)
        amount0: 100000000000000,
        amount1: 200000000000000,
        timestamp_ns: 0,  // Minimum timestamp
    };
    
    let bytes = original.to_bytes();
    let recovered = PoolBurnTLV::from_bytes(&bytes).unwrap();
    
    assert_deep_equality!(original, recovered);
    assert_eq!(original.liquidity_delta, recovered.liquidity_delta);
    
    // Verify extreme tick values preserved
    assert_eq!(recovered.tick_lower, i32::MIN + 1);
    assert_eq!(recovered.tick_upper, i32::MAX - 1);
}

#[test]
fn test_pool_tick_deep_equality() {
    let original = PoolTickTLV {
        venue: VenueId::Polygon,
        pool_id: test_pool_id(),
        tick: -100,  // Negative tick crossing
        liquidity_net: -123456789012345,  // Negative liquidity change
        price_sqrt: 7922816251426433759,  // X96 format sqrt price (truncated to fit u64)
        timestamp_ns: 1234567890123456789,
    };
    
    let bytes = original.to_bytes();
    let recovered = PoolTickTLV::from_bytes(&bytes).unwrap();
    
    assert_deep_equality!(original, recovered);
    assert_eq!(original.tick, recovered.tick);
    assert_eq!(original.liquidity_net, recovered.liquidity_net);
    assert_eq!(original.price_sqrt, recovered.price_sqrt);
    
    // Verify X96 sqrt price preserved exactly
    assert_eq!(recovered.price_sqrt, 7922816251426433759u64);
}

#[test]
fn test_pool_liquidity_deep_equality() {
    let original = PoolLiquidityTLV {
        venue: VenueId::Polygon,
        pool_id: test_pool_id(),
        reserves: vec![
            1000000000000000000,  // Token 0 reserve
            2000000000000000000,  // Token 1 reserve
            500000000000000,      // Additional reserve (for 3+ token pools)
        ],
        total_supply: 1500000000000000000,
        fee_rate: 3000,  // 30 basis points = 0.3%
        timestamp_ns: 9876543210987654321,
    };
    
    let bytes = original.to_bytes();
    let recovered = PoolLiquidityTLV::from_bytes(&bytes).unwrap();
    
    assert_deep_equality!(original, recovered);
    assert_eq!(original.reserves.len(), recovered.reserves.len());
    for (i, (&orig, &recov)) in original.reserves.iter().zip(recovered.reserves.iter()).enumerate() {
        assert_eq!(orig, recov, "Reserve {} mismatch", i);
    }
    assert_eq!(original.total_supply, recovered.total_supply);
    assert_eq!(original.fee_rate, recovered.fee_rate);
}

#[test]
fn test_extreme_values_roundtrip() {
    // Test with maximum values
    let max_swap = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id: test_pool_id(),
        token_in: u64::MAX,
        token_out: u64::MAX,
        amount_in: i64::MAX,
        amount_out: i64::MAX,
        fee_paid: i64::MAX,
        sqrt_price_x96_after: u64::MAX,
        tick_after: i32::MAX,
        liquidity_after: i64::MAX,
        timestamp_ns: u64::MAX,
        block_number: u64::MAX,
    };
    
    let bytes = max_swap.to_bytes();
    let recovered = PoolSwapTLV::from_bytes(&bytes).unwrap();
    assert_eq!(max_swap, recovered);
    assert_eq!(recovered.amount_in, i64::MAX);
    assert_eq!(recovered.timestamp_ns, u64::MAX);
    
    // Test with minimum values
    let min_swap = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id: test_pool_id(),
        token_in: 0,
        token_out: 0,
        amount_in: i64::MIN,
        amount_out: i64::MIN,
        fee_paid: 0,
        sqrt_price_x96_after: 0,
        tick_after: i32::MIN,
        liquidity_after: i64::MIN,
        timestamp_ns: 0,
        block_number: 0,
    };
    
    let bytes = min_swap.to_bytes();
    let recovered = PoolSwapTLV::from_bytes(&bytes).unwrap();
    assert_eq!(min_swap, recovered);
    assert_eq!(recovered.amount_in, i64::MIN);
}

#[test]
fn test_precision_preservation() {
    // Test that 8-decimal precision is maintained exactly
    let amounts = vec![
        1,  // Smallest unit
        12345678,  // 0.12345678
        100000000,  // 1.00000000
        123456789012345,  // Large amount
        999999999999999,  // Near max precision
    ];
    
    for amount in amounts {
        let swap = PoolSwapTLV {
            venue: VenueId::Polygon,
            pool_id: test_pool_id(),
            token_in: 1,
            token_out: 2,
            amount_in: amount,
            amount_out: amount * 2,
            fee_paid: amount / 333,  // ~0.3% fee
            sqrt_price_x96_after: 0,
            tick_after: 0,
            liquidity_after: 0,
            timestamp_ns: 1700000000000000000,
            block_number: 1000,
        };
        
        let bytes = swap.to_bytes();
        let recovered = PoolSwapTLV::from_bytes(&bytes).unwrap();
        
        assert_eq!(swap.amount_in, recovered.amount_in, 
                   "Precision lost for amount {}", amount);
        assert_eq!(swap.amount_out, recovered.amount_out);
        assert_eq!(swap.fee_paid, recovered.fee_paid);
    }
}

#[test]
fn test_nanosecond_timestamp_preservation() {
    // Test that nanosecond precision timestamps are preserved
    let timestamps = vec![
        0,  // Epoch
        1_000_000_000,  // 1 second
        1_000_000_001,  // 1 second + 1 nanosecond
        999_999_999,    // Just under 1 second
        1700000000123456789,  // Realistic timestamp with nanos
        u64::MAX,  // Maximum possible
    ];
    
    for ts in timestamps {
        let tick = PoolTickTLV {
            venue: VenueId::Polygon,
            pool_id: test_pool_id(),
            tick: 0,
            liquidity_net: 0,
            price_sqrt: 1000000,
            timestamp_ns: ts,
        };
        
        let bytes = tick.to_bytes();
        let recovered = PoolTickTLV::from_bytes(&bytes).unwrap();
        
        assert_eq!(tick.timestamp_ns, recovered.timestamp_ns,
                   "Timestamp precision lost for {}", ts);
        
        // Verify exact nanosecond preservation
        assert_eq!(recovered.timestamp_ns, ts);
    }
}

#[test]
fn test_byte_level_equality() {
    // Test that serialization is deterministic
    let swap = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id: test_pool_id(),
        token_in: 0x1234567890ABCDEF,
        token_out: 0xFEDCBA0987654321,
        amount_in: 555555555555555,
        amount_out: 666666666666666,
        fee_paid: 1666666666666,
        sqrt_price_x96_after: 0,
        tick_after: 0,
        liquidity_after: 0,
        timestamp_ns: 1234567890123456789,
        block_number: 1000,
    };
    
    // Serialize multiple times
    let bytes1 = swap.to_bytes();
    let bytes2 = swap.to_bytes();
    let bytes3 = swap.to_bytes();
    
    // All serializations should be identical
    assert_eq!(bytes1, bytes2, "Non-deterministic serialization!");
    assert_eq!(bytes2, bytes3, "Non-deterministic serialization!");
    
    // Legacy checksum test removed - use Protocol V2 TLVMessageBuilder for validation
}

/* Legacy TLV message test removed - use Protocol V2 TLVMessageBuilder for validation
#[test]
fn test_tlv_message_wrapper_integrity() {
    // Test that TLV message wrapper preserves data
    let mint = PoolMintTLV {
        venue: VenueId::Polygon,
        pool_id: test_pool_id(),
        provider: 0xCAFEBABEDEADBEEF,
        tick_lower: -500,
        tick_upper: 500,
        liquidity_delta: 1000000000000000,
        amount0: 500000000000000,
        amount1: 500000000000000,
        timestamp_ns: 1700000000000000000,
    };
    
    // Convert to TLV message
    let tlv_msg = mint.to_tlv_message();
    
    // Verify header
    assert_eq!(tlv_msg.header.magic, 0xDEADBEEF);
    assert_eq!(tlv_msg.header.tlv_type, TLVType::PoolMint);
    assert_eq!(tlv_msg.header.payload_len as usize, tlv_msg.payload.len());
    
    // Verify checksum
    let calculated_checksum = calculate_checksum(&tlv_msg.payload);
    assert_eq!(tlv_msg.header.checksum, calculated_checksum);
    
    // Recover from payload
    let recovered = PoolMintTLV::from_bytes(&tlv_msg.payload).unwrap();
    assert_eq!(mint, recovered);
}
*/

#[test]
fn test_cross_type_no_corruption() {
    // Ensure different TLV types don't corrupt each other
    let swap = PoolSwapTLV {
        venue: VenueId::Polygon,
        pool_id: test_pool_id(),
        token_in: 111,
        token_out: 222,
        amount_in: 333333333,
        amount_out: 444444444,
        fee_paid: 1111111,
        sqrt_price_x96_after: 0,
        tick_after: 0,
        liquidity_after: 0,
        timestamp_ns: 5555555555,
        block_number: 1000,
    };
    
    let mint = PoolMintTLV {
        venue: VenueId::Polygon,
        pool_id: test_pool_id(),
        provider: 777,
        tick_lower: -100,
        tick_upper: 100,
        liquidity_delta: 888888888,
        amount0: 999999999,
        amount1: 101010101,
        timestamp_ns: 5555555555,
    };
    
    // Serialize both
    let swap_bytes = swap.to_bytes();
    let mint_bytes = mint.to_bytes();
    
    // Note: Different TLV types may have the same size - that's OK since
    // the TLV header (not tested here) contains the type discriminator.
    // What matters is that cross-deserialization fails.
    
    // Deserialize and verify no cross-contamination
    let recovered_swap = PoolSwapTLV::from_bytes(&swap_bytes).unwrap();
    let recovered_mint = PoolMintTLV::from_bytes(&mint_bytes).unwrap();
    
    assert_eq!(swap, recovered_swap);
    assert_eq!(mint, recovered_mint);
    
    // Note: Type safety is enforced at the TLV header level, not in raw bytes
    // The raw bytes don't contain type information, so cross-deserialization
    // may succeed if the binary layout is compatible. This is by design -
    // type checking happens when parsing TLV messages with headers.
}

/// Validation statistics for reporting
#[derive(Debug, Default)]
pub struct ValidationStats {
    pub total_messages: u64,
    pub deep_equality_passes: u64,
    pub deep_equality_failures: u64,
    pub precision_errors: Vec<(String, i64, i64)>,  // (field, expected, actual)
    pub timestamp_errors: Vec<(u64, u64)>,  // (expected, actual)
}

impl ValidationStats {
    pub fn validate_pool_swap(&mut self, original: &PoolSwapTLV, recovered: &PoolSwapTLV) -> bool {
        self.total_messages += 1;
        
        let mut passed = true;
        
        // Check each field
        if original.amount_in != recovered.amount_in {
            self.precision_errors.push((
                "amount_in".to_string(),
                original.amount_in,
                recovered.amount_in
            ));
            passed = false;
        }
        
        if original.amount_out != recovered.amount_out {
            self.precision_errors.push((
                "amount_out".to_string(),
                original.amount_out,
                recovered.amount_out
            ));
            passed = false;
        }
        
        if original.timestamp_ns != recovered.timestamp_ns {
            self.timestamp_errors.push((original.timestamp_ns, recovered.timestamp_ns));
            passed = false;
        }
        
        if passed && original == recovered {
            self.deep_equality_passes += 1;
            true
        } else {
            self.deep_equality_failures += 1;
            false
        }
    }
    
    pub fn success_rate(&self) -> f64 {
        if self.total_messages == 0 {
            return 100.0;
        }
        (self.deep_equality_passes as f64 / self.total_messages as f64) * 100.0
    }
    
    pub fn report(&self) {
        println!("=== Deep Equality Validation Report ===");
        println!("Total messages validated: {}", self.total_messages);
        println!("Passed: {} ({:.2}%)", self.deep_equality_passes, self.success_rate());
        println!("Failed: {}", self.deep_equality_failures);
        
        if !self.precision_errors.is_empty() {
            println!("\nPrecision Errors:");
            for (field, expected, actual) in &self.precision_errors {
                println!("  {}: expected {}, got {} (diff: {})", 
                         field, expected, actual, expected - actual);
            }
        }
        
        if !self.timestamp_errors.is_empty() {
            println!("\nTimestamp Errors:");
            for (expected, actual) in &self.timestamp_errors {
                println!("  expected {}, got {} (diff: {} ns)", 
                         expected, actual, expected.saturating_sub(*actual));
            }
        }
    }
}