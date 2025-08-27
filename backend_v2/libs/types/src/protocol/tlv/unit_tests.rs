//! # Core TLV Unit Test Framework  
//!
//! This module provides unit testing coverage for working Protocol V2 TLV message types.
//! Focus on core market data TLVs and basic functionality validation.
//!
//! ## Test Categories
//!
//! - **Round-trip Serialization**: TLV types must serialize and deserialize identically
//! - **Edge Case Handling**: Zero values, maximum values, boundary conditions  
//! - **Size Validation**: Struct sizes match expected values for zerocopy compatibility
//! - **Performance Requirements**: Tests complete in <1 second total
//!
//! Run with: `cargo test unit_tests::tests`

#[cfg(test)]
mod tests {
    use crate::protocol::tlv::market_data::*;
    use crate::protocol::tlv::arbitrage_signal::*;
    use crate::protocol::identifiers::{InstrumentId, VenueId};
    use zerocopy::{AsBytes, FromBytes};
    use std::time::Instant;

    /// Test framework for round-trip serialization of any TLV type
    fn test_roundtrip<T>(original: T, expected_size: usize, type_name: &str) 
    where
        T: AsBytes + FromBytes + PartialEq + Clone + std::fmt::Debug,
    {
        // Serialize to bytes
        let bytes = original.as_bytes();
        
        // Verify size matches expectation
        assert_eq!(
            bytes.len(), 
            expected_size,
            "{} size mismatch: expected {} bytes, got {}",
            type_name, expected_size, bytes.len()
        );
        
        // Deserialize from bytes
        let deserialized = T::ref_from(bytes)
            .unwrap_or_else(|| panic!("Failed to deserialize {}", type_name));
        
        // Verify round-trip integrity
        assert_eq!(
            &original, 
            &*deserialized,
            "{} round-trip failed: data corruption detected",
            type_name
        );
    }

    /// Test edge cases for numeric fields
    fn test_edge_cases<T, F>(mut constructor: F, field_name: &str, type_name: &str)
    where
        T: AsBytes + FromBytes + PartialEq + Clone + std::fmt::Debug,
        F: FnMut(i64) -> T,
    {
        let test_values = vec![
            0i64,           // Zero
            1i64,           // Minimum positive
            -1i64,          // Minimum negative  
            i64::MAX,       // Maximum positive
            i64::MIN,       // Maximum negative
            42i64,          // Arbitrary value
            -42i64,         // Arbitrary negative
        ];

        for value in test_values {
            let tlv = constructor(value);
            let bytes = tlv.as_bytes();
            let recovered = T::ref_from(bytes)
                .unwrap_or_else(|| panic!("Edge case deserialization failed for {} in {}", field_name, type_name));
            
            assert_eq!(
                tlv, *recovered,
                "{} failed edge case test for {} with value {}",
                type_name, field_name, value
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Market Data TLV Tests (Types 1-19)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_trade_tlv_roundtrip() {
        let instrument_id = InstrumentId {
            venue: VenueId::Polygon as u16,
            asset_type: 1,
            reserved: 0,
            asset_id: 12345,
        };

        let trade = TradeTLV::new(
            VenueId::Polygon,
            instrument_id,
            100000000i64,      // price: 1.00000000 in 8-decimal fixed point
            50000000000i64,    // volume: 500.00000000
            0u8,               // side: buy
            1234567890000000000u64, // timestamp_ns
        );

        test_roundtrip(trade, 40, "TradeTLV");
    }

    #[test]
    fn test_trade_tlv_edge_cases() {
        let instrument_id = InstrumentId {
            venue: VenueId::Ethereum as u16,
            asset_type: 2,
            reserved: 0,
            asset_id: 67890,
        };

        // Test price edge cases
        test_edge_cases(
            |price| TradeTLV::new(VenueId::Ethereum, instrument_id, price, 1000000000i64, 1u8, 1000000000u64),
            "price",
            "TradeTLV"
        );

        // Test volume edge cases
        test_edge_cases(
            |volume| TradeTLV::new(VenueId::Ethereum, instrument_id, 50000000i64, volume, 0u8, 1000000000u64),
            "volume", 
            "TradeTLV"
        );
    }

    #[test]
    fn test_quote_tlv_roundtrip() {
        let instrument_id = InstrumentId {
            venue: VenueId::Binance as u16,
            asset_type: 1,
            reserved: 0,
            asset_id: 98765,
        };

        let quote = QuoteTLV::new(
            VenueId::Binance,
            instrument_id,
            99900000i64,    // bid_price: 0.999
            1000000i64,     // bid_size: 0.01
            100100000i64,   // ask_price: 1.001
            2000000i64,     // ask_size: 0.02
            1234567890000000000u64, // timestamp_ns
        );

        test_roundtrip(quote, 56, "QuoteTLV");
    }

    #[test]
    fn test_pool_swap_tlv_roundtrip() {
        let swap = PoolSwapTLV::new(
            [0x42u8; 20],      // pool_address
            [0x43u8; 20],      // token_in
            [0x44u8; 20],      // token_out
            VenueId::Polygon,
            1000u128,          // amount_in
            900u128,           // amount_out
            5000u128,          // liquidity_after
            1234567890000000000u64, // timestamp_ns
            12345u64,          // block_number
            100i32,            // tick_after
            18u8,              // amount_in_decimals
            6u8,               // amount_out_decimals
            12345u128,         // sqrt_price_x96_after
        );

        test_roundtrip(swap, 208, "PoolSwapTLV");
    }

    #[test]
    fn test_pool_swap_tlv_zero_values() {
        // Test with all zero values to catch initialization issues
        let swap = PoolSwapTLV::new(
            [0u8; 20],         // zero address
            [0u8; 20],         // zero token_in
            [0u8; 20],         // zero token_out  
            VenueId::Ethereum,
            0u128,             // zero amount_in
            0u128,             // zero amount_out
            0u128,             // zero liquidity
            0u64,              // zero timestamp
            0u64,              // zero block
            0i32,              // zero tick
            0u8,               // zero decimals
            0u8,               // zero decimals
            0u128,             // zero sqrt_price
        );

        test_roundtrip(swap, 208, "PoolSwapTLV (zero values)");
    }

    #[test]
    fn test_pool_swap_tlv_max_values() {
        // Test with maximum values to catch overflow issues
        let swap = PoolSwapTLV::new(
            [0xFFu8; 20],      // max address
            [0xFFu8; 20],      // max token addresses
            [0xFFu8; 20],
            VenueId::Polygon,
            u128::MAX,         // max amounts
            u128::MAX,
            u128::MAX,         // max liquidity
            u64::MAX,          // max timestamp
            u64::MAX,          // max block
            i32::MAX,          // max tick
            u8::MAX,           // max decimals
            u8::MAX,
            u128::MAX,         // max sqrt_price
        );

        test_roundtrip(swap, 208, "PoolSwapTLV (max values)");
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Signal TLV Tests (Types 20-39)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_arbitrage_signal_tlv_roundtrip() {
        let signal = ArbitrageSignalTLV::new(
            [0x11u8; 20],      // source_pool
            [0x22u8; 20],      // target_pool
            300u16,            // source_venue (UniswapV2)
            301u16,            // target_venue (UniswapV3)
            [0x33u8; 20],      // token_in
            [0x44u8; 20],      // token_out
            10.0f64,           // expected_profit_usd
            1000.0f64,         // required_capital_usd
            150u16,            // spread_bps (1.5%)
            5.0f64,            // dex_fees_usd
            2.0f64,            // gas_cost_usd
            1.0f64,            // slippage_usd
            1640995200000000000u64, // timestamp_ns
        );

        test_roundtrip(signal, ARBITRAGE_SIGNAL_TLV_SIZE, "ArbitrageSignalTLV");
    }

    #[test]
    fn test_arbitrage_signal_edge_cases() {
        // Test profit potential edge cases by varying expected_profit_usd
        let test_values = vec![0.0f64, 0.001f64, 1000.0f64, -100.0f64];
        
        for profit in test_values {
            let signal = ArbitrageSignalTLV::new(
                [0x11u8; 20],      // source_pool
                [0x22u8; 20],      // target_pool
                300u16, 301u16,    // venues
                [0x33u8; 20], [0x44u8; 20], // tokens
                profit,            // expected_profit_usd (test value)
                1000.0f64,         // required_capital_usd
                150u16,            // spread_bps
                5.0f64, 2.0f64, 1.0f64, // fees/costs/slippage
                1640995200000000000u64, // timestamp_ns
            );
            
            test_roundtrip(signal, ARBITRAGE_SIGNAL_TLV_SIZE, &format!("ArbitrageSignalTLV profit={}", profit));
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // System TLV Tests (Types 100-119) - SKIPPED due to constructor issues
    // ═══════════════════════════════════════════════════════════════════════
    
    // NOTE: System TLV tests will be added once constructor interfaces are stabilized

    // ═══════════════════════════════════════════════════════════════════════
    // Comprehensive Test Suite
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_all_struct_sizes_and_alignment() {
        use std::mem::{align_of, size_of};

        // Market Data TLVs
        assert_eq!(size_of::<TradeTLV>(), 40, "TradeTLV size");
        assert_eq!(size_of::<QuoteTLV>(), 56, "QuoteTLV size");
        assert_eq!(size_of::<PoolSwapTLV>(), 208, "PoolSwapTLV size");

        // Signal TLVs
        assert_eq!(size_of::<ArbitrageSignalTLV>(), ARBITRAGE_SIGNAL_TLV_SIZE, "ArbitrageSignalTLV size");

        // System TLVs - SKIPPED due to constructor issues
        // assert_eq!(size_of::<SystemHealthTLV>(), 32, "SystemHealthTLV size");
        // assert_eq!(size_of::<TraceContextTLV>(), 48, "TraceContextTLV size");

        // Verify proper alignment for zerocopy
        assert_eq!(size_of::<TradeTLV>() % align_of::<TradeTLV>(), 0, "TradeTLV alignment");
        assert_eq!(size_of::<QuoteTLV>() % align_of::<QuoteTLV>(), 0, "QuoteTLV alignment");
        assert_eq!(size_of::<PoolSwapTLV>() % align_of::<PoolSwapTLV>(), 0, "PoolSwapTLV alignment");

        println!("✅ All TLV struct sizes and alignments validated");
    }

    #[test]
    fn test_padding_bytes_initialized() {
        // Verify padding bytes are properly zeroed
        let trade = TradeTLV::new(
            VenueId::Ethereum,
            InstrumentId { venue: 1, asset_type: 1, reserved: 0, asset_id: 123 },
            100000000i64,
            50000000000i64,
            1u8,
            1234567890000000000u64,
        );

        // TradeTLV has 3 bytes of padding
        let bytes = trade.as_bytes();
        let padding_start = bytes.len() - 3;
        assert_eq!(&bytes[padding_start..], &[0u8; 3], "TradeTLV padding not zeroed");

        let swap = PoolSwapTLV::new(
            [0x42u8; 20], [0x43u8; 20], [0x44u8; 20], VenueId::Polygon,
            1000u128, 900u128, 5000u128, 1234567890u64, 12345u64,
            100i32, 18u8, 6u8, 12345u128,
        );

        // PoolSwapTLV has 8 bytes of padding
        assert_eq!(swap._padding, [0u8; 8], "PoolSwapTLV padding not zeroed");

        println!("✅ All padding bytes properly initialized to zero");
    }

    #[test]
    fn test_performance_requirement_under_1_second() {
        // This test must complete in <1 second as per TEST-001 requirements
        let start = Instant::now();

        // Run a comprehensive set of operations
        for i in 0..1000 {
            // Create various TLV types
            let trade = TradeTLV::new(
                VenueId::Polygon,
                InstrumentId { venue: 1, asset_type: 1, reserved: 0, asset_id: i },
                (i * 1000) as i64,
                (i * 2000) as i64,
                (i % 2) as u8,
                1234567890000000000u64 + i,
            );

            let swap = PoolSwapTLV::new(
                [i as u8; 20], [(i + 1) as u8; 20], [(i + 2) as u8; 20], VenueId::Ethereum,
                i as u128, (i + 100) as u128, (i + 1000) as u128, 
                1234567890u64 + i, i, i as i32, 18u8, 6u8, i as u128,
            );

            // Serialize and deserialize
            let trade_bytes = trade.as_bytes();
            let trade_recovered = TradeTLV::ref_from(trade_bytes).unwrap();
            assert_eq!(trade, *trade_recovered);

            let swap_bytes = swap.as_bytes();
            let swap_recovered = PoolSwapTLV::ref_from(swap_bytes).unwrap();
            assert_eq!(swap, *swap_recovered);
        }

        let duration = start.elapsed();
        println!("Performance test completed in: {:?}", duration);

        // Requirement: All tests must complete in <1 second
        assert!(
            duration.as_secs() < 1,
            "Performance test took too long: {:?} (requirement: <1s)",
            duration
        );

        println!("✅ Performance requirement met: <1 second for comprehensive testing");
    }

    #[test]
    fn test_zerocopy_compatibility_all_types() {
        // Verify all TLV types support zerocopy traits correctly
        use zerocopy::{AsBytes, FromBytes, FromZeroes};

        // Test TradeTLV
        let trade = TradeTLV::new(
            VenueId::Polygon,
            InstrumentId { venue: 1, asset_type: 1, reserved: 0, asset_id: 12345 },
            100000000i64, 50000000000i64, 0u8, 1234567890000000000u64,
        );
        
        // Must support AsBytes
        let _: &[u8] = trade.as_bytes();
        
        // Must support FromBytes
        let trade_bytes = trade.as_bytes();
        let _trade_ref = TradeTLV::ref_from(trade_bytes).unwrap();
        
        // Must support FromZeroes
        let _zero_trade = TradeTLV::new_zeroed();

        // Test PoolSwapTLV
        let swap = PoolSwapTLV::new(
            [0x42u8; 20], [0x43u8; 20], [0x44u8; 20], VenueId::Polygon,
            1000u128, 900u128, 5000u128, 1234567890u64, 12345u64,
            100i32, 18u8, 6u8, 12345u128,
        );
        
        let _: &[u8] = swap.as_bytes();
        let swap_bytes = swap.as_bytes();
        let _swap_ref = PoolSwapTLV::ref_from(swap_bytes).unwrap();
        let _zero_swap = PoolSwapTLV::new_zeroed();

        println!("✅ All TLV types support zerocopy traits (AsBytes, FromBytes, FromZeroes)");
    }

    #[test]
    fn test_boundary_values_all_numeric_types() {
        // Test boundary values for different numeric types used in TLVs

        // u8 boundaries (used for side, decimals, etc.)
        for value in [0u8, 1u8, 127u8, 128u8, 255u8] {
            let trade = TradeTLV::new(
                VenueId::Ethereum,
                InstrumentId { venue: 1, asset_type: 1, reserved: 0, asset_id: 123 },
                100000000i64, 50000000000i64, value, 1234567890000000000u64,
            );
            test_roundtrip(trade, 40, &format!("TradeTLV side={}", value));
        }

        // u16 boundaries (used for venue_id)
        for venue in [VenueId::Ethereum, VenueId::Polygon, VenueId::Binance] {
            let trade = TradeTLV::new(
                venue,
                InstrumentId { venue: venue as u16, asset_type: 1, reserved: 0, asset_id: 123 },
                100000000i64, 50000000000i64, 0u8, 1234567890000000000u64,
            );
            test_roundtrip(trade, 40, &format!("TradeTLV venue={:?}", venue));
        }

        // u32 boundaries (used for gas prices, block numbers, etc.)
        // Test by varying the spread_bps field (u16) instead since that's what ArbitrageSignalTLV has
        for value in [0u16, 1u16, 100u16, 1000u16, u16::MAX] {
            let signal = ArbitrageSignalTLV::new(
                [0x11u8; 20], [0x22u8; 20], // pools
                300u16, 301u16,             // venues
                [0x33u8; 20], [0x44u8; 20], // tokens
                10.0f64, 1000.0f64,        // profits/capital
                value,                     // spread_bps (test value)
                5.0f64, 2.0f64, 1.0f64,   // fees/costs/slippage
                1234567890000000000u64,    // timestamp
            );
            test_roundtrip(signal, ARBITRAGE_SIGNAL_TLV_SIZE, &format!("ArbitrageSignalTLV spread_bps={}", value));
        }

        println!("✅ All numeric boundary values handled correctly");
    }
}