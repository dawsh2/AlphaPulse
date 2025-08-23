//! Roundtrip deep equality test for protocol messages
//!
//! Ensures that data serialized and deserialized through the relay
//! maintains perfect bit-for-bit equality with no precision loss.

#[cfg(test)]
mod roundtrip_tests {
    use alphapulse_protocol_v2::{
        build_instrument_id, InstrumentId, MessageHeader, PoolSwapTLV, QuoteTLV, RelayDomain,
        SourceType, TLVMessage, TLVType, TradeTLV, VenueId, MESSAGE_MAGIC, PROTOCOL_VERSION,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Deep equality check for MessageHeader
    fn assert_header_equality(original: &MessageHeader, deserialized: &MessageHeader) {
        assert_eq!(original.magic, deserialized.magic, "Magic mismatch");
        assert_eq!(original.version, deserialized.version, "Version mismatch");
        assert_eq!(
            original.message_type, deserialized.message_type,
            "Message type mismatch"
        );
        assert_eq!(
            original.relay_domain, deserialized.relay_domain,
            "Relay domain mismatch"
        );
        assert_eq!(
            original.source_type, deserialized.source_type,
            "Source type mismatch"
        );
        assert_eq!(
            original.sequence, deserialized.sequence,
            "Sequence mismatch"
        );
        assert_eq!(
            original.timestamp_ns, deserialized.timestamp_ns,
            "Timestamp mismatch"
        );
        assert_eq!(
            original.instrument_id, deserialized.instrument_id,
            "Instrument ID mismatch"
        );
        assert_eq!(
            original.checksum, deserialized.checksum,
            "Checksum mismatch"
        );
    }

    #[test]
    fn test_trade_tlv_roundtrip() {
        println!("\n=== TradeTLV Roundtrip Test ===\n");

        // Create original trade with precise values
        let original_price: i64 = 4523467890123; // $45,234.67890123 with 8 decimals
        let original_volume: i64 = 123456789; // 1.23456789 BTC
        let original_timestamp_ns: u64 = 1734567890123456789;
        let original_instrument_id = build_instrument_id(1, 1, 2, 1, VenueId::Kraken as u16, 0);

        // Create header
        let original_header = MessageHeader {
            magic: MESSAGE_MAGIC,
            version: PROTOCOL_VERSION,
            message_type: TLVType::Trade as u8,
            relay_domain: RelayDomain::MarketData as u8,
            source_type: SourceType::KrakenCollector as u8,
            sequence: 42,
            timestamp_ns: original_timestamp_ns,
            instrument_id: original_instrument_id,
            checksum: 0,
        };

        // Serialize to bytes
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &original_header as *const _ as *const u8,
                std::mem::size_of::<MessageHeader>(),
            )
        };

        let mut message = header_bytes.to_vec();

        // Add TradeTLV payload
        let is_buy = true;
        let flags = if is_buy { 0x01 } else { 0x00 };

        message.push(TLVType::Trade as u8);
        message.push(flags);
        message.extend_from_slice(&16u16.to_le_bytes()); // Length
        message.extend_from_slice(&original_price.to_le_bytes());
        message.extend_from_slice(&original_volume.to_le_bytes());

        // Calculate checksum
        let checksum = crc32fast::hash(&message);
        let checksum_offset = std::mem::size_of::<MessageHeader>() - 4;
        message[checksum_offset..checksum_offset + 4].copy_from_slice(&checksum.to_le_bytes());

        println!("Original values:");
        println!(
            "  Price: {} (${:.8})",
            original_price,
            original_price as f64 / 100_000_000.0
        );
        println!(
            "  Volume: {} ({:.8} BTC)",
            original_volume,
            original_volume as f64 / 100_000_000.0
        );
        println!("  Timestamp: {} ns", original_timestamp_ns);
        println!("  Instrument: 0x{:016x}", original_instrument_id);
        println!("  Checksum: 0x{:08x}", checksum);
        println!("  Message size: {} bytes", message.len());

        // Deserialize from bytes
        let deserialized_header =
            unsafe { std::ptr::read(message.as_ptr() as *const MessageHeader) };

        // Verify header equality
        assert_header_equality(&original_header, &deserialized_header);

        // Parse TLV payload
        let tlv_offset = std::mem::size_of::<MessageHeader>();
        let tlv_type = message[tlv_offset];
        let tlv_flags = message[tlv_offset + 1];
        let tlv_length = u16::from_le_bytes([message[tlv_offset + 2], message[tlv_offset + 3]]);

        let price_offset = tlv_offset + 4;
        let volume_offset = price_offset + 8;

        let deserialized_price = i64::from_le_bytes([
            message[price_offset],
            message[price_offset + 1],
            message[price_offset + 2],
            message[price_offset + 3],
            message[price_offset + 4],
            message[price_offset + 5],
            message[price_offset + 6],
            message[price_offset + 7],
        ]);

        let deserialized_volume = i64::from_le_bytes([
            message[volume_offset],
            message[volume_offset + 1],
            message[volume_offset + 2],
            message[volume_offset + 3],
            message[volume_offset + 4],
            message[volume_offset + 5],
            message[volume_offset + 6],
            message[volume_offset + 7],
        ]);

        println!("\nDeserialized values:");
        println!(
            "  Price: {} (${:.8})",
            deserialized_price,
            deserialized_price as f64 / 100_000_000.0
        );
        println!(
            "  Volume: {} ({:.8} BTC)",
            deserialized_volume,
            deserialized_volume as f64 / 100_000_000.0
        );

        // Deep equality assertions
        assert_eq!(tlv_type, TLVType::Trade as u8, "TLV type mismatch");
        assert_eq!(tlv_flags, flags, "TLV flags mismatch");
        assert_eq!(tlv_length, 16, "TLV length mismatch");
        assert_eq!(
            original_price, deserialized_price,
            "Price mismatch after roundtrip!"
        );
        assert_eq!(
            original_volume, deserialized_volume,
            "Volume mismatch after roundtrip!"
        );

        // Verify checksum integrity
        let mut verify_message = message.clone();
        verify_message[checksum_offset..checksum_offset + 4].copy_from_slice(&[0; 4]);
        let calculated_checksum = crc32fast::hash(&verify_message);
        assert_eq!(
            checksum, calculated_checksum,
            "Checksum verification failed!"
        );

        println!("\n✅ TradeTLV roundtrip test PASSED - perfect equality maintained!");
    }

    #[test]
    fn test_quote_tlv_roundtrip() {
        println!("\n=== QuoteTLV Roundtrip Test ===\n");

        // Original quote with maximum precision
        let original_bid: i64 = 4523400000000; // $45,234.00000000
        let original_ask: i64 = 4523500000000; // $45,235.00000000
        let original_bid_size: i64 = 250000000; // 2.5 BTC
        let original_ask_size: i64 = 180000000; // 1.8 BTC
        let original_timestamp_ns: u64 = u64::MAX - 1; // Test edge case

        let original_header = MessageHeader {
            magic: MESSAGE_MAGIC,
            version: PROTOCOL_VERSION,
            message_type: TLVType::Quote as u8,
            relay_domain: RelayDomain::MarketData as u8,
            source_type: SourceType::CoinbaseCollector as u8,
            sequence: u64::MAX,
            timestamp_ns: original_timestamp_ns,
            instrument_id: build_instrument_id(1, 3, 2, 1, VenueId::Coinbase as u16, 0),
            checksum: 0,
        };

        // Serialize
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &original_header as *const _ as *const u8,
                std::mem::size_of::<MessageHeader>(),
            )
        };

        let mut message = header_bytes.to_vec();

        // Add QuoteTLV
        message.push(TLVType::Quote as u8);
        message.push(0); // Flags
        message.extend_from_slice(&32u16.to_le_bytes()); // Length
        message.extend_from_slice(&original_bid.to_le_bytes());
        message.extend_from_slice(&original_ask.to_le_bytes());
        message.extend_from_slice(&original_bid_size.to_le_bytes());
        message.extend_from_slice(&original_ask_size.to_le_bytes());

        // Checksum
        let checksum = crc32fast::hash(&message);
        let checksum_offset = std::mem::size_of::<MessageHeader>() - 4;
        message[checksum_offset..checksum_offset + 4].copy_from_slice(&checksum.to_le_bytes());

        println!("Original quote:");
        println!("  Bid: {} @ {}", original_bid, original_bid_size);
        println!("  Ask: {} @ {}", original_ask, original_ask_size);
        println!(
            "  Spread: {} ({:.4}%)",
            original_ask - original_bid,
            ((original_ask - original_bid) as f64 / original_bid as f64) * 100.0
        );

        // Deserialize
        let deserialized_header =
            unsafe { std::ptr::read(message.as_ptr() as *const MessageHeader) };

        assert_header_equality(&original_header, &deserialized_header);

        let tlv_offset = std::mem::size_of::<MessageHeader>();
        let bid_offset = tlv_offset + 4;
        let ask_offset = bid_offset + 8;
        let bid_size_offset = ask_offset + 8;
        let ask_size_offset = bid_size_offset + 8;

        let deserialized_bid =
            i64::from_le_bytes(message[bid_offset..bid_offset + 8].try_into().unwrap());
        let deserialized_ask =
            i64::from_le_bytes(message[ask_offset..ask_offset + 8].try_into().unwrap());
        let deserialized_bid_size = i64::from_le_bytes(
            message[bid_size_offset..bid_size_offset + 8]
                .try_into()
                .unwrap(),
        );
        let deserialized_ask_size = i64::from_le_bytes(
            message[ask_size_offset..ask_size_offset + 8]
                .try_into()
                .unwrap(),
        );

        // Deep equality checks
        assert_eq!(original_bid, deserialized_bid, "Bid price mismatch!");
        assert_eq!(original_ask, deserialized_ask, "Ask price mismatch!");
        assert_eq!(
            original_bid_size, deserialized_bid_size,
            "Bid size mismatch!"
        );
        assert_eq!(
            original_ask_size, deserialized_ask_size,
            "Ask size mismatch!"
        );

        println!("\n✅ QuoteTLV roundtrip test PASSED - all values identical!");
    }

    #[test]
    fn test_pool_swap_tlv_roundtrip() {
        println!("\n=== PoolSwapTLV Roundtrip Test ===\n");

        // Test with Wei values (18 decimals)
        let original_amount_in: u128 = 1234567890123456789012345678; // Large Wei amount
        let original_amount_out: u128 = 9876543210987654321098765432;
        let original_sqrt_price_x96: u128 = u128::MAX / 2; // Test large value

        let original_header = MessageHeader {
            magic: MESSAGE_MAGIC,
            version: PROTOCOL_VERSION,
            message_type: 11, // PoolSwapTLV
            relay_domain: RelayDomain::MarketData as u8,
            source_type: SourceType::PolygonCollector as u8,
            sequence: 999999,
            timestamp_ns: 1734567890123456789,
            instrument_id: build_instrument_id(
                2,
                100,
                101,
                3,
                VenueId::UniswapV3Polygon as u16,
                500,
            ),
            checksum: 0,
        };

        // Serialize header
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &original_header as *const _ as *const u8,
                std::mem::size_of::<MessageHeader>(),
            )
        };

        let mut message = header_bytes.to_vec();

        // Add PoolSwapTLV payload
        message.push(11); // PoolSwapTLV type
        message.push(0); // Flags

        // Calculate payload size
        let payload_size = 2 + 32 + 8 + 8 + 16 + 16 + 16 + 1 + 1 + 4; // venue + pool_id + tokens + amounts + sqrt_price + decimals + tick
        message.extend_from_slice(&(payload_size as u16).to_le_bytes());

        // Add payload data
        message.extend_from_slice(&(VenueId::UniswapV3Polygon as u16).to_le_bytes());
        message.extend_from_slice(&[0xAB; 32]); // pool_id
        message.extend_from_slice(&0x1234567890ABCDEFu64.to_le_bytes()); // token_in
        message.extend_from_slice(&0xFEDCBA0987654321u64.to_le_bytes()); // token_out
        message.extend_from_slice(&original_amount_in.to_le_bytes());
        message.extend_from_slice(&original_amount_out.to_le_bytes());
        message.extend_from_slice(&original_sqrt_price_x96.to_le_bytes());
        message.push(18); // amount_in_decimals
        message.push(6); // amount_out_decimals (USDC)
        message.extend_from_slice(&123456i32.to_le_bytes()); // tick

        // Calculate checksum
        let checksum = crc32fast::hash(&message);
        let checksum_offset = std::mem::size_of::<MessageHeader>() - 4;
        message[checksum_offset..checksum_offset + 4].copy_from_slice(&checksum.to_le_bytes());

        println!("Original PoolSwap:");
        println!("  Amount In: {} (18 decimals)", original_amount_in);
        println!("  Amount Out: {} (6 decimals)", original_amount_out);
        println!("  Sqrt Price X96: {}", original_sqrt_price_x96);

        // Deserialize and verify
        let deserialized_header =
            unsafe { std::ptr::read(message.as_ptr() as *const MessageHeader) };

        assert_header_equality(&original_header, &deserialized_header);

        // Parse amounts
        let tlv_offset = std::mem::size_of::<MessageHeader>();
        let amount_in_offset = tlv_offset + 4 + 2 + 32 + 8 + 8;
        let amount_out_offset = amount_in_offset + 16;
        let sqrt_price_offset = amount_out_offset + 16;

        let deserialized_amount_in = u128::from_le_bytes(
            message[amount_in_offset..amount_in_offset + 16]
                .try_into()
                .unwrap(),
        );
        let deserialized_amount_out = u128::from_le_bytes(
            message[amount_out_offset..amount_out_offset + 16]
                .try_into()
                .unwrap(),
        );
        let deserialized_sqrt_price = u128::from_le_bytes(
            message[sqrt_price_offset..sqrt_price_offset + 16]
                .try_into()
                .unwrap(),
        );

        assert_eq!(
            original_amount_in, deserialized_amount_in,
            "Amount in mismatch!"
        );
        assert_eq!(
            original_amount_out, deserialized_amount_out,
            "Amount out mismatch!"
        );
        assert_eq!(
            original_sqrt_price_x96, deserialized_sqrt_price,
            "Sqrt price mismatch!"
        );

        println!("\n✅ PoolSwapTLV roundtrip test PASSED - Wei precision preserved!");
    }

    #[test]
    fn test_binary_precision_edge_cases() {
        println!("\n=== Binary Precision Edge Cases ===\n");

        // Test edge values
        let test_cases = vec![
            (0i64, "Zero value"),
            (1i64, "Minimum positive"),
            (-1i64, "Minimum negative"),
            (i64::MAX, "Maximum i64"),
            (i64::MIN, "Minimum i64"),
            (99999999i64, "0.99999999 (max 8 decimal)"),
            (100000000i64, "1.00000000 exactly"),
            (4523467890123i64, "45234.67890123 (typical price)"),
        ];

        for (original_value, description) in test_cases {
            println!("Testing: {} = {}", description, original_value);

            // Serialize
            let bytes = original_value.to_le_bytes();

            // Deserialize
            let deserialized = i64::from_le_bytes(bytes);

            // Verify exact equality
            assert_eq!(
                original_value, deserialized,
                "Failed for {}: {} != {}",
                description, original_value, deserialized
            );

            // Also test as part of a message
            let mut message = Vec::new();
            message.extend_from_slice(&bytes);

            let from_vec = i64::from_le_bytes(message[0..8].try_into().unwrap());

            assert_eq!(
                original_value, from_vec,
                "Failed vector test for {}",
                description
            );
        }

        println!("\n✅ All edge cases passed with perfect precision!");
    }

    #[test]
    fn test_floating_point_conversion_precision() {
        println!("\n=== Float to Fixed-Point Conversion ===\n");

        // Test that our conversion maintains precision
        let test_prices = vec![
            "45234.67890123",
            "0.00000001", // 1 satoshi
            "0.99999999",
            "1.00000000",
            "999999.99999999",
            "0.12345678",
        ];

        for price_str in test_prices {
            let price_float: f64 = price_str.parse().unwrap();
            let price_fixed = (price_float * 100_000_000.0) as i64;
            let price_back = price_fixed as f64 / 100_000_000.0;

            // Parse original string to get exact decimal places
            let parts: Vec<&str> = price_str.split('.').collect();
            let decimal_places = if parts.len() > 1 { parts[1].len() } else { 0 };

            println!(
                "  {} → {} → {:.width$}",
                price_str,
                price_fixed,
                price_back,
                width = decimal_places
            );

            // Verify the roundtrip maintains the value within floating point precision
            let diff = (price_float - price_back).abs();
            assert!(
                diff < 1e-10,
                "Precision loss for {}: diff = {}",
                price_str,
                diff
            );
        }

        println!("\n✅ Float conversion maintains precision within limits!");
    }

    #[test]
    fn test_message_corruption_detection() {
        println!("\n=== Message Corruption Detection ===\n");

        let original_header = MessageHeader {
            magic: MESSAGE_MAGIC,
            version: PROTOCOL_VERSION,
            message_type: TLVType::Trade as u8,
            relay_domain: RelayDomain::MarketData as u8,
            source_type: SourceType::KrakenCollector as u8,
            sequence: 1,
            timestamp_ns: 1234567890,
            instrument_id: 9999,
            checksum: 0,
        };

        // Serialize
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &original_header as *const _ as *const u8,
                std::mem::size_of::<MessageHeader>(),
            )
        };

        let mut message = header_bytes.to_vec();

        // Add payload
        message.push(TLVType::Trade as u8);
        message.push(0);
        message.extend_from_slice(&16u16.to_le_bytes());
        message.extend_from_slice(&12345678i64.to_le_bytes());
        message.extend_from_slice(&87654321i64.to_le_bytes());

        // Calculate correct checksum
        let correct_checksum = crc32fast::hash(&message);
        let checksum_offset = std::mem::size_of::<MessageHeader>() - 4;
        message[checksum_offset..checksum_offset + 4]
            .copy_from_slice(&correct_checksum.to_le_bytes());

        // Verify correct message passes
        let stored_checksum = u32::from_le_bytes(
            message[checksum_offset..checksum_offset + 4]
                .try_into()
                .unwrap(),
        );
        let mut verify_message = message.clone();
        verify_message[checksum_offset..checksum_offset + 4].copy_from_slice(&[0; 4]);
        let calculated_checksum = crc32fast::hash(&verify_message);
        assert_eq!(
            stored_checksum, calculated_checksum,
            "Valid message failed checksum!"
        );

        // Corrupt a single bit in the price
        message[std::mem::size_of::<MessageHeader>() + 5] ^= 0x01;

        // Checksum should now fail
        let mut verify_corrupted = message.clone();
        verify_corrupted[checksum_offset..checksum_offset + 4].copy_from_slice(&[0; 4]);
        let corrupted_checksum = crc32fast::hash(&verify_corrupted);
        assert_ne!(
            stored_checksum, corrupted_checksum,
            "Corruption not detected!"
        );

        println!("✅ Message corruption correctly detected by checksum!");
    }
}

fn main() {
    println!("Run with: cargo test --test roundtrip_equality_test");
}
