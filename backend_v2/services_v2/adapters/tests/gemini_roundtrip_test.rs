//! Comprehensive Gemini Roundtrip Validation Test
//!
//! Verifies complete data integrity through the full pipeline:
//! 1. Gemini JSON → GeminiTradeEvent (semantic preservation)
//! 2. GeminiTradeEvent → TradeTLV (semantic mapping)
//! 3. TradeTLV → Binary bytes (serialization)
//! 4. Binary bytes → TradeTLV (deserialization)
//! 5. Deep equality between original and recovered TLV
//! 6. Semantic equality with original Gemini data

use alphapulse_adapter_service::input::collectors::gemini::{
    GeminiMarketDataEvent, GeminiTradeEvent,
};
use alphapulse_protocol_v2::{InstrumentId, TradeTLV, VenueId};
use serde_json::Value;
use std::convert::TryFrom;
use zerocopy::{AsBytes, FromBytes};

#[test]
fn test_gemini_complete_roundtrip_with_semantic_preservation() {
    // Step 1: Original Gemini JSON data (based on their API format)
    let original_json = r#"{
        "type": "update",
        "eventId": 123456789,
        "socket_sequence": 1,
        "events": [
            {
                "type": "trade",
                "tid": 987654321,
                "price": "45123.50",
                "amount": "0.12345678",
                "makerSide": "bid",
                "timestampms": 1693234567890
            }
        ]
    }"#;

    let json_value: Value = serde_json::from_str(original_json).unwrap();

    // Step 2: Parse to GeminiMarketDataEvent
    let market_event: GeminiMarketDataEvent = serde_json::from_value(json_value.clone()).unwrap();
    assert_eq!(market_event.event_type, "update");
    assert_eq!(market_event.event_id, Some(123456789));
    assert!(market_event.events.is_some());

    let trade_event = &market_event.events.unwrap()[0];

    // Verify semantic preservation from JSON to struct
    assert_eq!(trade_event.tid, 987654321);
    assert_eq!(trade_event.price, "45123.50");
    assert_eq!(trade_event.amount, "0.12345678");
    assert_eq!(trade_event.maker_side, "bid");
    assert_eq!(trade_event.timestamp_ms, 1693234567890);

    // Step 3: Convert to TradeTLV (with symbol context)
    let original_tlv = TradeTLV::try_from((trade_event, "btcusd")).unwrap();

    // Verify semantic mapping to TLV
    assert_eq!(original_tlv.venue().unwrap(), VenueId::Gemini);

    // Copy packed fields to avoid unaligned access (ARM/M1 safety)
    let tlv_price = original_tlv.price;
    let tlv_volume = original_tlv.volume;
    let tlv_side = original_tlv.side;
    let tlv_timestamp = original_tlv.timestamp_ns;

    // Verify correct fixed-point conversion (8 decimals for CEX)
    assert_eq!(tlv_price, 4512350000000); // 45123.50 * 1e8
    assert_eq!(tlv_volume, 12345678); // 0.12345678 * 1e8
    assert_eq!(tlv_side, 1); // maker was bidding, taker sold (market sell)
    assert_eq!(tlv_timestamp, 1693234567890000000); // ms * 1_000_000

    // Step 4: Serialize to binary
    let binary_bytes = original_tlv.as_bytes();
    assert!(!binary_bytes.is_empty());
    assert_eq!(binary_bytes.len(), std::mem::size_of::<TradeTLV>());

    // Step 5: Deserialize from binary
    let recovered_tlv = TradeTLV::from_bytes(binary_bytes).unwrap();

    // Step 6: Verify DEEP EQUALITY (byte-for-byte identical)
    assert_eq!(original_tlv, recovered_tlv, "Deep equality failed!");

    // Verify all fields match exactly (copy to avoid packed field issues)
    let orig_venue = original_tlv.venue_id;
    let rec_venue = recovered_tlv.venue_id;
    assert_eq!(orig_venue, rec_venue);

    let orig_asset_type = original_tlv.asset_type;
    let rec_asset_type = recovered_tlv.asset_type;
    assert_eq!(orig_asset_type, rec_asset_type);

    let orig_asset_id = original_tlv.asset_id;
    let rec_asset_id = recovered_tlv.asset_id;
    assert_eq!(orig_asset_id, rec_asset_id);

    let orig_price = original_tlv.price;
    let rec_price = recovered_tlv.price;
    assert_eq!(orig_price, rec_price);

    let orig_volume = original_tlv.volume;
    let rec_volume = recovered_tlv.volume;
    assert_eq!(orig_volume, rec_volume);

    let orig_side = original_tlv.side;
    let rec_side = recovered_tlv.side;
    assert_eq!(orig_side, rec_side);

    let orig_timestamp = original_tlv.timestamp_ns;
    let rec_timestamp = recovered_tlv.timestamp_ns;
    assert_eq!(orig_timestamp, rec_timestamp);

    // Step 7: Verify InstrumentId construction correctness
    let instrument = recovered_tlv.instrument_id();
    let instrument_venue = instrument.venue;
    assert_eq!(instrument_venue, VenueId::Gemini as u16);

    // Step 8: Verify semantic correctness of final result
    let recovered_price_display = recovered_tlv.price as f64 / 100_000_000.0;
    let recovered_volume_display = recovered_tlv.volume as f64 / 100_000_000.0;

    assert!((recovered_price_display - 45123.50).abs() < 0.01);
    assert!((recovered_volume_display - 0.12345678).abs() < 0.00000001);

    println!("✅ Gemini roundtrip validation PASSED");
    println!(
        "   Original: {} {} @ {} (maker: {})",
        trade_event.amount, "btcusd", trade_event.price, trade_event.maker_side
    );
    println!(
        "   TLV: price={}, volume={}, side={}",
        recovered_price_display, recovered_volume_display, recovered_tlv.side
    );
}

#[test]
fn test_gemini_symbol_normalization() {
    let event = GeminiTradeEvent {
        trade_type: "trade".to_string(),
        tid: 1,
        price: "1".to_string(),
        amount: "1".to_string(),
        maker_side: "bid".to_string(),
        timestamp_ms: 1,
    };

    // Test known symbol mappings
    assert_eq!(event.normalized_symbol("btcusd"), "BTC/USD");
    assert_eq!(event.normalized_symbol("ethusd"), "ETH/USD");
    assert_eq!(event.normalized_symbol("maticusd"), "MATIC/USD");
    assert_eq!(event.normalized_symbol("solusd"), "SOL/USD");

    // Test fallback for unknown symbols
    assert_eq!(event.normalized_symbol("newcoinusd"), "NEWCOIN/USD");
}

#[test]
fn test_gemini_trade_side_conversion() {
    // Test bid (maker was bidding, taker sold)
    let bid_event = GeminiTradeEvent {
        trade_type: "trade".to_string(),
        tid: 1,
        price: "1".to_string(),
        amount: "1".to_string(),
        maker_side: "bid".to_string(),
        timestamp_ms: 1,
    };
    assert_eq!(bid_event.trade_side(), 1); // Market sell

    // Test ask (maker was asking, taker bought)
    let ask_event = GeminiTradeEvent {
        trade_type: "trade".to_string(),
        tid: 1,
        price: "1".to_string(),
        amount: "1".to_string(),
        maker_side: "ask".to_string(),
        timestamp_ms: 1,
    };
    assert_eq!(ask_event.trade_side(), 0); // Market buy
}

#[test]
fn test_gemini_precision_preservation() {
    // Test high precision values
    let high_precision_event = GeminiTradeEvent {
        trade_type: "trade".to_string(),
        tid: 1,
        price: "123456.78901234".to_string(),
        amount: "0.12345678".to_string(),
        maker_side: "ask".to_string(),
        timestamp_ms: 1693234567890,
    };

    // Verify precise fixed-point conversion
    let price_fp = high_precision_event.price_fixed_point().unwrap();
    assert_eq!(price_fp, 12345678901234); // Truncated to 8 decimals

    let amount_fp = high_precision_event.amount_fixed_point().unwrap();
    assert_eq!(amount_fp, 12345678); // 0.12345678 * 1e8
}

#[test]
fn test_gemini_edge_cases() {
    // Test zero values (should fail validation)
    let zero_price_event = GeminiTradeEvent {
        trade_type: "trade".to_string(),
        tid: 1,
        price: "0".to_string(),
        amount: "1".to_string(),
        maker_side: "bid".to_string(),
        timestamp_ms: 1,
    };
    assert!(zero_price_event.validate().is_err());

    // Test negative values (should fail parsing)
    let negative_amount_event = GeminiTradeEvent {
        trade_type: "trade".to_string(),
        tid: 1,
        price: "1".to_string(),
        amount: "-1".to_string(),
        maker_side: "bid".to_string(),
        timestamp_ms: 1,
    };
    assert!(negative_amount_event.validate().is_err());

    // Test invalid timestamp
    let zero_timestamp_event = GeminiTradeEvent {
        trade_type: "trade".to_string(),
        tid: 1,
        price: "1".to_string(),
        amount: "1".to_string(),
        maker_side: "bid".to_string(),
        timestamp_ms: 0,
    };
    assert!(zero_timestamp_event.validate().is_err());
}

#[test]
fn test_multiple_symbol_support() {
    use alphapulse_adapter_service::GeminiCollector;
    use tokio::sync::mpsc;

    let (tx, _rx) = mpsc::channel(100);
    let collector = GeminiCollector::new(
        vec![
            "btcusd".to_string(),
            "ethusd".to_string(),
            "maticusd".to_string(),
        ],
        tx,
    );

    let tracked = collector.tracked_instruments();
    assert_eq!(tracked.len(), 3);

    // Verify InstrumentId construction for all symbols
    for instrument in tracked {
        assert_eq!(instrument.venue, VenueId::Gemini as u16);
        // Each should have proper asset type for crypto coins
    }
}

/// Real-world data test with actual Gemini API format
/// This test uses real JSON structure from Gemini's documentation
#[test]
fn test_gemini_real_world_format() {
    let real_gemini_json = r#"{
        "type": "update",
        "eventId": 36902233362,
        "socket_sequence": 661,
        "events": [
            {
                "type": "trade",
                "tid": 36902233362,
                "price": "23570.44",
                "amount": "0.0009",
                "makerSide": "ask",
                "timestampms": 1629464726493
            }
        ]
    }"#;

    // Parse and convert following full pipeline
    let json_value: Value = serde_json::from_str(real_gemini_json).unwrap();
    let market_event: GeminiMarketDataEvent = serde_json::from_value(json_value).unwrap();
    let trade_event = &market_event.events.unwrap()[0];

    // Convert to TLV
    let tlv = TradeTLV::try_from((trade_event, "btcusd")).unwrap();

    // Verify all conversions are correct
    assert_eq!(tlv.venue().unwrap(), VenueId::Gemini);

    // Copy packed fields for safety
    let price = tlv.price;
    let volume = tlv.volume;
    let side = tlv.side;
    let timestamp = tlv.timestamp_ns;

    // Verify conversions
    assert_eq!(price, 2357044000000); // 23570.44 * 1e8
    assert_eq!(volume, 90000); // 0.0009 * 1e8
    assert_eq!(side, 0); // Maker was asking, taker bought (market buy)
    assert_eq!(timestamp, 1629464726493000000); // ms to ns

    // Verify roundtrip integrity
    let binary = tlv.as_bytes();
    let recovered = TradeTLV::from_bytes(binary).unwrap();
    assert_eq!(tlv, recovered);

    println!("✅ Real-world Gemini format validation PASSED");
}
