//! TLV Parsing Robustness Tests
//! 
//! Tests TLV parsing with real-world edge cases including:
//! - Actual exchange message formats that may be malformed
//! - Network fragmentation scenarios
//! - Corrupted data from transmission errors
//! - Oversized orderbook snapshots from volatile markets

mod common;

use alphapulse_protocol_v2::{
    MessageHeader, parse_header, parse_tlv_extensions,
    tlv::{TLVMessageBuilder, ParseError, parse_tlv_extensions_for_relay},
    RelayDomain, SourceType, TLVType,
};
use common::*;

#[test]
fn test_fragmented_network_packet() {
    // Real scenario: TCP packet fragmentation mid-TLV
    let full_msg = create_market_data_message(SourceType::BinanceCollector);
    
    // Simulate receiving message in fragments (common with large orderbooks)
    let fragment_sizes = [32, 8, 8, 4]; // Header, then partial TLVs
    let mut received = Vec::new();
    let mut offset = 0;
    
    for size in fragment_sizes {
        let end = (offset + size).min(full_msg.len());
        received.extend_from_slice(&full_msg[offset..end]);
        offset = end;
        
        // Try parsing at each fragment stage
        if received.len() >= MessageHeader::SIZE {
            // Header parsing might require the full message for checksum validation
            // So we only test that with complete messages
            if received.len() >= full_msg.len() {
                let header_result = parse_header(&received);
                assert!(header_result.is_ok(), "Full message should parse");
                
                let tlv_result = parse_tlv_extensions(&received[MessageHeader::SIZE..]);
                assert!(tlv_result.is_ok(), "TLVs should parse");
            } else if received.len() > MessageHeader::SIZE {
                // Partial TLV data should fail to parse
                let tlv_result = parse_tlv_extensions(&received[MessageHeader::SIZE..]);
                assert!(tlv_result.is_err(), "Partial TLV should not parse");
            }
        }
    }
}

#[test]
fn test_binance_orderbook_overflow() {
    // Real case: Binance can send massive orderbook snapshots during volatility
    // Create a realistic L2 snapshot with 1000 levels (40KB payload)
    let mut orderbook_payload = Vec::with_capacity(40_000);
    
    // Add realistic price levels (BTC/USDT around $45,000)
    for i in 0..1000 {
        let price: i64 = 4500000000000 + (i * 100000000); // $45,000 + $1 increments
        let volume: i64 = 10000000 + (i * 1000000); // 0.1 + 0.01 BTC increments
        orderbook_payload.extend_from_slice(&price.to_le_bytes());
        orderbook_payload.extend_from_slice(&volume.to_le_bytes());
    }
    
    // This should trigger extended TLV (Type 255) automatically
    let msg = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .add_tlv_bytes(TLVType::L2Snapshot, orderbook_payload.clone())
        .build();
    
    // Verify extended TLV was used (Type 255)
    assert!(msg[MessageHeader::SIZE] == 255, "Should use extended TLV for large payload");
    
    // Parse and verify
    let tlv_data = &msg[MessageHeader::SIZE..];
    let tlvs = parse_tlv_extensions(tlv_data).unwrap();
    assert_eq!(tlvs.len(), 1);
    
    // Relay parser should also handle it
    let relay_tlvs = parse_tlv_extensions_for_relay(tlv_data).unwrap();
    assert_eq!(relay_tlvs.len(), 1);
}

#[test]
fn test_kraken_malformed_array_format() {
    // Kraken sends arrays like [price, volume, timestamp]
    // Sometimes they send malformed data with wrong array lengths
    
    // Create a Trade TLV with wrong size (Kraken sends 32 bytes instead of expected 24)
    let kraken_trade_payload = vec![
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, // instrument_id
        0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, // price
        0x00, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x00, // volume
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, // extra kraken data
    ];
    
    let msg = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::KrakenCollector)
        .add_tlv_bytes(TLVType::Trade, kraken_trade_payload)
        .build();
    
    let tlv_data = &msg[MessageHeader::SIZE..];
    
    // Strict parser may enforce size validation
    let strict_result = parse_tlv_extensions(tlv_data);
    
    // Relay parser should be more lenient for performance
    let relay_result = parse_tlv_extensions_for_relay(tlv_data);
    assert!(relay_result.is_ok(), "Relay parser should handle size mismatches");
}

#[test]
fn test_coinbase_string_decimal_edge_cases() {
    // Coinbase sends string decimals that can have edge cases:
    // "0.00000001", "1e-8", "123456789.123456789"
    
    // Simulate pre-parsed values with maximum precision
    let btc_satoshi_price: i64 = 1; // 0.00000001 BTC = 1 satoshi
    let large_price: i64 = 12345678912345678; // Maximum precision
    let negative_spread: i64 = -100000000; // -1.0 (shouldn't happen but test robustness)
    
    // Create quote with extreme values
    let mut quote_payload = Vec::with_capacity(32);
    quote_payload.extend_from_slice(&1u64.to_le_bytes()); // instrument_id
    quote_payload.extend_from_slice(&btc_satoshi_price.to_le_bytes()); // bid
    quote_payload.extend_from_slice(&large_price.to_le_bytes()); // ask
    quote_payload.extend_from_slice(&negative_spread.to_le_bytes()); // spread
    
    let msg = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::CoinbaseCollector)
        .add_tlv_bytes(TLVType::Quote, quote_payload)
        .build();
    
    let tlv_data = &msg[MessageHeader::SIZE..];
    let tlvs = parse_tlv_extensions(tlv_data).unwrap();
    assert_eq!(tlvs.len(), 1);
}

#[test]
fn test_polygon_dex_wei_precision() {
    // Polygon DEX pools use 18-decimal Wei values
    // Test extreme values that could overflow if not handled properly
    
    let max_wei: u64 = u64::MAX; // 18,446,744,073,709,551,615 wei
    let pool_tvl: u64 = 1_000_000_000_000_000_000; // 1 ETH in wei
    
    let mut pool_payload = Vec::new();
    pool_payload.extend_from_slice(&max_wei.to_le_bytes()); // token0_reserve
    pool_payload.extend_from_slice(&pool_tvl.to_le_bytes()); // token1_reserve
    pool_payload.extend_from_slice(&3000u32.to_le_bytes()); // fee_bps (0.3%)
    
    let msg = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::PolygonCollector)
        .add_tlv_bytes(TLVType::OrderBook, pool_payload)
        .build();
    
    let tlv_data = &msg[MessageHeader::SIZE..];
    let tlvs = parse_tlv_extensions(tlv_data).unwrap();
    assert_eq!(tlvs.len(), 1);
}

#[test]
fn test_multiple_tlv_parsing_with_corruption() {
    // Real scenario: Message with multiple TLVs where middle one is corrupted
    let trade_payload = vec![0x01; 24];
    let corrupt_payload = vec![0xFF; 16]; // Will claim wrong length
    let quote_payload = vec![0x02; 32];
    
    // Build message manually to inject corruption
    let mut msg = Vec::new();
    
    // Add header (simplified)
    let header = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .build();
    msg.extend_from_slice(&header[..MessageHeader::SIZE]);
    
    // Add first TLV (valid)
    msg.push(TLVType::Trade as u8);
    msg.push(24);
    msg.extend_from_slice(&trade_payload);
    
    // Add corrupted TLV (length says 255 but only 16 bytes follow)
    msg.push(TLVType::Quote as u8);
    msg.push(255); // Wrong length!
    msg.extend_from_slice(&corrupt_payload);
    
    // Try to add third TLV (unreachable due to corruption)
    msg.push(TLVType::OrderBook as u8);
    msg.push(32);
    msg.extend_from_slice(&quote_payload);
    
    let tlv_data = &msg[MessageHeader::SIZE..];
    
    // Should fail due to truncation
    match parse_tlv_extensions(tlv_data) {
        Err(ParseError::TruncatedTLV { offset }) => {
            assert!(offset > 0, "Should detect truncation after first valid TLV");
        }
        _ => panic!("Expected TruncatedTLV error"),
    }
}

#[test]
fn test_zero_length_tlv() {
    // Some system TLVs like Heartbeat can have zero payload
    let msg = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::MarketDataRelay)
        .add_tlv_bytes(TLVType::Heartbeat, vec![])
        .build();
    
    let tlv_data = &msg[MessageHeader::SIZE..];
    
    // Both parsers should handle zero-length TLVs
    let strict_tlvs = parse_tlv_extensions(tlv_data).unwrap();
    assert_eq!(strict_tlvs.len(), 1);
    
    let relay_tlvs = parse_tlv_extensions_for_relay(tlv_data).unwrap();
    assert_eq!(relay_tlvs.len(), 1);
    assert_eq!(relay_tlvs[0].tlv_length, 0);
}

#[test]
fn test_unknown_tlv_type_handling() {
    // Future TLV types should not break existing parsers
    let future_tlv_type = 199u8; // Undefined type
    
    let mut msg = Vec::new();
    let header_msg = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .build();
    msg.extend_from_slice(&header_msg[..MessageHeader::SIZE]);
    
    // Add unknown TLV type
    msg.push(future_tlv_type);
    msg.push(8); // 8 byte payload
    msg.extend_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
    
    let tlv_data = &msg[MessageHeader::SIZE..];
    
    // Parser should handle unknown types gracefully
    let result = parse_tlv_extensions(tlv_data);
    match result {
        Ok(tlvs) => {
            // May accept unknown type
            assert_eq!(tlvs.len(), 1);
        }
        Err(ParseError::UnknownTLVType(t)) => {
            assert_eq!(t, future_tlv_type);
        }
        _ => panic!("Unexpected error for unknown TLV type"),
    }
    
    // Relay parser should definitely accept it for forward compatibility
    let relay_result = parse_tlv_extensions_for_relay(tlv_data);
    assert!(relay_result.is_ok(), "Relay should handle unknown TLV types");
}

#[test]
fn test_rapid_message_sequence() {
    // Test parsing many messages rapidly (like during market open)
    let mut messages = Vec::new();
    
    // Generate 1000 messages with increasing sequence numbers
    for i in 0..1000 {
        let mut msg = create_market_data_message(SourceType::BinanceCollector);
        
        // Update sequence number
        let seq_bytes = (i as u64).to_le_bytes();
        msg[12..20].copy_from_slice(&seq_bytes);
        
        // Recalculate checksum
        let checksum = alphapulse_protocol_v2::validation::calculate_crc32_excluding_checksum(&msg, 28);
        msg[28..32].copy_from_slice(&checksum.to_be_bytes());
        
        messages.push(msg);
    }
    
    // Parse all messages
    let start = std::time::Instant::now();
    for msg in &messages {
        let header = parse_header(msg).unwrap();
        let tlv_data = &msg[MessageHeader::SIZE..];
        let _tlvs = parse_tlv_extensions_for_relay(tlv_data).unwrap();
        
        // Verify sequence number
        let sequence = header.sequence;
        assert!(sequence < 1000);
    }
    let elapsed = start.elapsed();
    
    let throughput = 1000.0 / elapsed.as_secs_f64();
    println!("Parsed {} messages in {:?} ({:.0} msg/s)", 
             messages.len(), elapsed, throughput);
}

#[test]
fn test_malicious_size_attack() {
    // Test defense against malicious size values that could cause DoS
    
    // Try to claim 4GB payload (u32::MAX)
    let mut malicious_msg = Vec::new();
    let header_msg = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::BinanceCollector)
        .build();
    malicious_msg.extend_from_slice(&header_msg[..MessageHeader::SIZE]);
    
    // Extended TLV claiming huge size
    malicious_msg.push(255); // Extended TLV marker
    malicious_msg.push(0);   // Reserved
    malicious_msg.push(TLVType::OrderBook as u8);
    malicious_msg.extend_from_slice(&u16::MAX.to_le_bytes()); // Max u16 size
    
    let tlv_data = &malicious_msg[MessageHeader::SIZE..];
    
    // Parser should reject without allocating huge memory
    match parse_tlv_extensions(tlv_data) {
        Err(ParseError::TruncatedTLV { .. }) => {
            // Good - detected the lie about size
        }
        Err(ParseError::PayloadTooLarge { size }) => {
            assert!(size > 1024 * 1024); // Should detect excessive size
        }
        Ok(_) => panic!("Should not parse malicious size"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn test_cross_domain_tlv_contamination() {
    // Ensure TLVs from wrong domain are caught
    
    // Try to put Signal TLV in Market Data message
    let signal_payload = vec![0x01; 16];
    
    // This should be caught somewhere in the pipeline
    let msg = TLVMessageBuilder::new(RelayDomain::MarketData, SourceType::Dashboard)
        .add_tlv_bytes(TLVType::SignalIdentity, signal_payload)
        .build();
    
    let tlv_data = &msg[MessageHeader::SIZE..];
    let relay_tlvs = parse_tlv_extensions_for_relay(tlv_data).unwrap();
    
    // Verify domain validation catches it
    assert!(!relay_tlvs[0].is_valid_for_domain(RelayDomain::MarketData),
            "Should detect wrong domain TLV");
    assert!(relay_tlvs[0].is_valid_for_domain(RelayDomain::Signal),
            "Should validate for correct domain");
}

#[test]
fn test_real_arbitrage_opportunity_message() {
    // Test a real arbitrage signal with all required TLVs
    use alphapulse_protocol_v2::InstrumentId;
    
    // Real arb opportunity: WETH/USDC price difference
    let opportunity_id: u64 = 0x1234567890ABCDEF;
    let weth = InstrumentId::coin(alphapulse_protocol_v2::VenueId::Ethereum, "WETH");
    let usdc = InstrumentId::coin(alphapulse_protocol_v2::VenueId::Ethereum, "USDC");
    let weth_usdc = InstrumentId::pool(alphapulse_protocol_v2::VenueId::UniswapV2, weth, usdc);
    
    // Build complete arbitrage signal message
    let mut msg = TLVMessageBuilder::new(RelayDomain::Signal, SourceType::ArbitrageStrategy);
    
    // SignalIdentity (TLV 20)
    let mut signal_id_payload = Vec::new();
    signal_id_payload.extend_from_slice(&opportunity_id.to_le_bytes());
    signal_id_payload.extend_from_slice(&1u64.to_le_bytes()); // version
    msg = msg.add_tlv_bytes(TLVType::SignalIdentity, signal_id_payload);
    
    // AssetCorrelation (TLV 21) - WETH/USDC spread
    let mut correlation_payload = Vec::new();
    correlation_payload.extend_from_slice(&weth_usdc.to_u64().to_le_bytes());
    correlation_payload.extend_from_slice(&500i64.to_le_bytes()); // 0.5% spread
    correlation_payload.extend_from_slice(&950000000i64.to_le_bytes()); // 95% confidence
    msg = msg.add_tlv_bytes(TLVType::AssetCorrelation, correlation_payload);
    
    // Economics (TLV 22) - Profit calculation
    let mut economics_payload = Vec::new();
    economics_payload.extend_from_slice(&50000000000i64.to_le_bytes()); // $500 expected profit
    economics_payload.extend_from_slice(&10000000000i64.to_le_bytes()); // $100 gas cost
    economics_payload.extend_from_slice(&100000000000i64.to_le_bytes()); // $1000 capital required
    economics_payload.extend_from_slice(&4000i32.to_le_bytes()); // 40% APR
    msg = msg.add_tlv_bytes(TLVType::Economics, economics_payload);
    
    let complete_msg = msg.build();
    
    // Parse and validate the complete arbitrage signal
    let header = parse_header(&complete_msg).unwrap();
    assert_eq!(header.get_relay_domain().unwrap(), RelayDomain::Signal);
    
    let tlv_data = &complete_msg[MessageHeader::SIZE..];
    let tlvs = parse_tlv_extensions(tlv_data).unwrap();
    assert_eq!(tlvs.len(), 3, "Should have 3 TLVs for complete arb signal");
}

#[test]
fn test_high_frequency_trade_stream() {
    // Simulate HFT stream with microsecond-apart trades
    let mut last_timestamp = 0u64;
    
    for i in 0..100 {
        let msg = create_market_data_message(SourceType::BinanceCollector);
        let header = parse_header(&msg).unwrap();
        
        let timestamp = header.timestamp;
        if i > 0 {
            let delta_ns = timestamp - last_timestamp;
            // Should be at least some nanoseconds apart
            assert!(delta_ns > 0, "Timestamps must be strictly increasing");
            
            // In real HFT, trades can be microseconds apart
            if delta_ns < 1000 {
                println!("Sub-microsecond trade detected: {} ns apart", delta_ns);
            }
        }
        last_timestamp = timestamp;
        
        // Brief pause to simulate real timing
        std::thread::sleep(std::time::Duration::from_micros(10));
    }
}