//! # Core TLV Tests - Standalone Unit Testing
//!
//! This module provides essential unit tests for TLV types without external dependencies.
//! Focuses on the working TLV types: TradeTLV, QuoteTLV, PoolSwapTLV, and ArbitrageSignalTLV.
//!
//! Run with: `cargo test core_tests`

#[cfg(test)]
mod tests {
    use crate::protocol::identifiers::{InstrumentId, VenueId};
    use crate::protocol::tlv::arbitrage_signal::{ArbitrageSignalTLV, ARBITRAGE_SIGNAL_TLV_SIZE};
    use crate::protocol::tlv::market_data::{PoolSwapTLV, QuoteTLV, TradeTLV};
    use std::time::Instant;
    use zerocopy::{AsBytes, FromBytes};

    /// Test framework for round-trip serialization
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
            type_name,
            expected_size,
            bytes.len()
        );

        // Deserialize from bytes
        let deserialized =
            T::ref_from(bytes).unwrap_or_else(|| panic!("Failed to deserialize {}", type_name));

        // Verify round-trip integrity
        assert_eq!(
            &original, &*deserialized,
            "{} round-trip failed: data corruption detected",
            type_name
        );
    }

    #[test]
    fn test_trade_tlv_roundtrip_serialization() {
        let instrument_id = InstrumentId {
            venue: VenueId::Polygon as u16,
            asset_type: 1,
            reserved: 0,
            asset_id: 12345,
        };

        let trade = TradeTLV::new(
            VenueId::Polygon,
            instrument_id,
            100000000i64,           // price: 1.00000000 in 8-decimal fixed point
            50000000000i64,         // volume: 500.00000000
            0u8,                    // side: buy
            1234567890000000000u64, // timestamp_ns
        );

        test_roundtrip(trade, 40, "TradeTLV");
    }

    #[test]
    fn test_quote_tlv_roundtrip_serialization() {
        let instrument_id = InstrumentId {
            venue: VenueId::Binance as u16,
            asset_type: 1,
            reserved: 0,
            asset_id: 98765,
        };

        let quote = QuoteTLV::new(
            VenueId::Binance,
            instrument_id,
            99900000i64,            // bid_price: 0.999
            1000000i64,             // bid_size: 0.01
            100100000i64,           // ask_price: 1.001
            2000000i64,             // ask_size: 0.02
            1234567890000000000u64, // timestamp_ns
        );

        test_roundtrip(quote, 56, "QuoteTLV");
    }

    #[test]
    fn test_pool_swap_tlv_roundtrip_serialization() {
        let swap = PoolSwapTLV::new(
            [0x42u8; 20], // pool_address
            [0x43u8; 20], // token_in
            [0x44u8; 20], // token_out
            VenueId::Polygon,
            1000u128,               // amount_in
            900u128,                // amount_out
            5000u128,               // liquidity_after
            1234567890000000000u64, // timestamp_ns
            12345u64,               // block_number
            100i32,                 // tick_after
            18u8,                   // amount_in_decimals
            6u8,                    // amount_out_decimals
            12345u128,              // sqrt_price_x96_after
        );

        test_roundtrip(swap, 208, "PoolSwapTLV");
    }

    #[test]
    fn test_arbitrage_signal_tlv_roundtrip() {
        let signal = ArbitrageSignalTLV::new(
            [0x11u8; 20],           // source_pool
            [0x22u8; 20],           // target_pool
            300u16,                 // source_venue (UniswapV2)
            301u16,                 // target_venue (UniswapV3)
            [0x33u8; 20],           // token_in
            [0x44u8; 20],           // token_out
            10.0f64,                // expected_profit_usd
            1000.0f64,              // required_capital_usd
            150u16,                 // spread_bps (1.5%)
            5.0f64,                 // dex_fees_usd
            2.0f64,                 // gas_cost_usd
            1.0f64,                 // slippage_usd
            1640995200000000000u64, // timestamp_ns
        );

        test_roundtrip(signal, ARBITRAGE_SIGNAL_TLV_SIZE, "ArbitrageSignalTLV");
    }

    #[test]
    fn test_edge_cases_zero_values() {
        // Test with zero values to catch initialization issues
        let swap = PoolSwapTLV::new(
            [0u8; 20], // zero address
            [0u8; 20], // zero token_in
            [0u8; 20], // zero token_out
            VenueId::Ethereum,
            0u128, // zero amount_in
            0u128, // zero amount_out
            0u128, // zero liquidity
            0u64,  // zero timestamp
            0u64,  // zero block
            0i32,  // zero tick
            0u8,   // zero decimals
            0u8,   // zero decimals
            0u128, // zero sqrt_price
        );

        test_roundtrip(swap, 208, "PoolSwapTLV (zero values)");

        // Test trade with zero price (edge case)
        let trade = TradeTLV::new(
            VenueId::Ethereum,
            InstrumentId {
                venue: 1,
                asset_type: 1,
                reserved: 0,
                asset_id: 1,
            },
            0i64, // zero price
            0i64, // zero volume
            0u8,  // buy side
            0u64, // zero timestamp
        );

        test_roundtrip(trade, 40, "TradeTLV (zero values)");
    }

    #[test]
    fn test_edge_cases_max_values() {
        // Test with maximum values to catch overflow issues
        let swap = PoolSwapTLV::new(
            [0xFFu8; 20], // max address
            [0xFFu8; 20], // max token addresses
            [0xFFu8; 20],
            VenueId::Polygon,
            u128::MAX, // max amounts
            u128::MAX,
            u128::MAX, // max liquidity
            u64::MAX,  // max timestamp
            u64::MAX,  // max block
            i32::MAX,  // max tick
            u8::MAX,   // max decimals
            u8::MAX,
            u128::MAX, // max sqrt_price
        );

        test_roundtrip(swap, 208, "PoolSwapTLV (max values)");

        // Test trade with maximum values
        let trade = TradeTLV::new(
            VenueId::Ethereum,
            InstrumentId {
                venue: u16::MAX,
                asset_type: 255,
                reserved: 255,
                asset_id: u64::MAX,
            },
            i64::MAX, // max price
            i64::MAX, // max volume
            u8::MAX,  // max side
            u64::MAX, // max timestamp
        );

        test_roundtrip(trade, 40, "TradeTLV (max values)");
    }

    #[test]
    fn test_struct_sizes_and_alignment() {
        use std::mem::{align_of, size_of};

        // Verify expected sizes for zerocopy compatibility
        assert_eq!(size_of::<TradeTLV>(), 40, "TradeTLV size");
        assert_eq!(size_of::<QuoteTLV>(), 56, "QuoteTLV size");
        assert_eq!(size_of::<PoolSwapTLV>(), 208, "PoolSwapTLV size");
        assert_eq!(
            size_of::<ArbitrageSignalTLV>(),
            ARBITRAGE_SIGNAL_TLV_SIZE,
            "ArbitrageSignalTLV size"
        );

        // Verify proper alignment for zerocopy
        assert_eq!(
            size_of::<TradeTLV>() % align_of::<TradeTLV>(),
            0,
            "TradeTLV alignment"
        );
        assert_eq!(
            size_of::<QuoteTLV>() % align_of::<QuoteTLV>(),
            0,
            "QuoteTLV alignment"
        );
        assert_eq!(
            size_of::<PoolSwapTLV>() % align_of::<PoolSwapTLV>(),
            0,
            "PoolSwapTLV alignment"
        );

        println!("✅ All TLV struct sizes and alignments validated");
    }

    #[test]
    fn test_zerocopy_traits_all_types() {
        // Verify all TLV types support zerocopy traits correctly
        use zerocopy::{AsBytes, FromBytes, FromZeroes};

        // Test TradeTLV
        let trade = TradeTLV::new(
            VenueId::Polygon,
            InstrumentId {
                venue: 1,
                asset_type: 1,
                reserved: 0,
                asset_id: 12345,
            },
            100000000i64,
            50000000000i64,
            0u8,
            1234567890000000000u64,
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
            [0x42u8; 20],
            [0x43u8; 20],
            [0x44u8; 20],
            VenueId::Polygon,
            1000u128,
            900u128,
            5000u128,
            1234567890u64,
            12345u64,
            100i32,
            18u8,
            6u8,
            12345u128,
        );

        let _: &[u8] = swap.as_bytes();
        let swap_bytes = swap.as_bytes();
        let _swap_ref = PoolSwapTLV::ref_from(swap_bytes).unwrap();
        let _zero_swap = PoolSwapTLV::new_zeroed();

        println!("✅ All TLV types support zerocopy traits (AsBytes, FromBytes, FromZeroes)");
    }

    #[test]
    fn test_performance_requirement_under_1_second() {
        // This test must complete in <1 second as per TEST-001 requirements
        let start = Instant::now();

        // Run comprehensive operations on multiple TLV types
        for i in 0..1000 {
            // Create TradeTLV
            let trade = TradeTLV::new(
                VenueId::Polygon,
                InstrumentId {
                    venue: 1,
                    asset_type: 1,
                    reserved: 0,
                    asset_id: i,
                },
                (i * 1000) as i64,
                (i * 2000) as i64,
                (i % 2) as u8,
                1234567890000000000u64 + i,
            );

            // Create PoolSwapTLV
            let swap = PoolSwapTLV::new(
                [i as u8; 20],
                [(i + 1) as u8; 20],
                [(i + 2) as u8; 20],
                VenueId::Ethereum,
                i as u128,
                (i + 100) as u128,
                (i + 1000) as u128,
                1234567890u64 + i,
                i,
                i as i32,
                18u8,
                6u8,
                i as u128,
            );

            // Create ArbitrageSignalTLV
            let signal = ArbitrageSignalTLV::new(
                [i as u8; 20],
                [(i + 1) as u8; 20],
                300u16,
                301u16,
                [i as u8; 20],
                [(i + 2) as u8; 20],
                (i as f64) * 0.01,
                1000.0,
                150u16,
                5.0,
                2.0,
                1.0,
                1640995200000000000u64 + i,
            );

            // Serialize and deserialize (round-trip test)
            let trade_bytes = trade.as_bytes();
            let trade_recovered = TradeTLV::ref_from(trade_bytes).unwrap();
            assert_eq!(trade, *trade_recovered);

            let swap_bytes = swap.as_bytes();
            let swap_recovered = PoolSwapTLV::ref_from(swap_bytes).unwrap();
            assert_eq!(swap, *swap_recovered);

            let signal_bytes = signal.as_bytes();
            let signal_recovered = ArbitrageSignalTLV::ref_from(signal_bytes).unwrap();
            assert_eq!(signal, *signal_recovered);
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
    fn test_field_boundary_values() {
        // Test boundary values for different numeric types

        // u8 boundaries (used for side, decimals)
        for side in [0u8, 1u8, 127u8, 255u8] {
            let trade = TradeTLV::new(
                VenueId::Ethereum,
                InstrumentId {
                    venue: 1,
                    asset_type: 1,
                    reserved: 0,
                    asset_id: 123,
                },
                100000000i64,
                50000000000i64,
                side,
                1234567890000000000u64,
            );
            test_roundtrip(trade, 40, &format!("TradeTLV side={}", side));
        }

        // i32 boundaries (used for tick values)
        for tick in [i32::MIN, -1000i32, 0i32, 1000i32, i32::MAX] {
            let swap = PoolSwapTLV::new(
                [0x42u8; 20],
                [0x43u8; 20],
                [0x44u8; 20],
                VenueId::Polygon,
                1000u128,
                900u128,
                5000u128,
                1234567890u64,
                12345u64,
                tick,
                18u8,
                6u8,
                12345u128,
            );
            test_roundtrip(swap, 208, &format!("PoolSwapTLV tick={}", tick));
        }

        // u16 boundaries (used in ArbitrageSignalTLV)
        for spread in [0u16, 1u16, 100u16, 10000u16, u16::MAX] {
            let signal = ArbitrageSignalTLV::new(
                [0x11u8; 20],
                [0x22u8; 20],
                300u16,
                301u16,
                [0x33u8; 20],
                [0x44u8; 20],
                10.0f64,
                1000.0f64,
                spread,
                5.0f64,
                2.0f64,
                1.0f64,
                1640995200000000000u64,
            );
            test_roundtrip(
                signal,
                ARBITRAGE_SIGNAL_TLV_SIZE,
                &format!("ArbitrageSignalTLV spread_bps={}", spread),
            );
        }

        println!("✅ All boundary value tests passed");
    }

    #[test]
    fn test_padding_bytes_initialization() {
        // Verify padding bytes are properly zeroed in packed structs
        let trade = TradeTLV::new(
            VenueId::Ethereum,
            InstrumentId {
                venue: 1,
                asset_type: 1,
                reserved: 0,
                asset_id: 123,
            },
            100000000i64,
            50000000000i64,
            1u8,
            1234567890000000000u64,
        );

        // TradeTLV should have proper padding
        let bytes = trade.as_bytes();
        assert_eq!(bytes.len(), 40, "TradeTLV should be exactly 40 bytes");

        let swap = PoolSwapTLV::new(
            [0x42u8; 20],
            [0x43u8; 20],
            [0x44u8; 20],
            VenueId::Polygon,
            1000u128,
            900u128,
            5000u128,
            1234567890u64,
            12345u64,
            100i32,
            18u8,
            6u8,
            12345u128,
        );

        // PoolSwapTLV has explicit padding field that should be zeroed
        assert_eq!(swap._padding, [0u8; 8], "PoolSwapTLV padding not zeroed");

        println!("✅ Padding bytes properly initialized");
    }
}
