//! Comprehensive Coinbase Roundtrip Validation Test
//!
//! Verifies complete data integrity through the full pipeline:
//! 1. Coinbase JSON → CoinbaseMatchEvent (semantic preservation)
//! 2. CoinbaseMatchEvent → TradeTLV (semantic mapping)
//! 3. TradeTLV → Binary bytes (serialization)
//! 4. Binary bytes → TradeTLV (deserialization)
//! 5. Deep equality between original and recovered TLV
//! 6. Semantic equality with original Coinbase data

use torq_adapter_service::input::collectors::coinbase::CoinbaseMatchEvent;
use protocol_v2::{InstrumentId, TradeTLV, VenueId};
use serde_json::Value;
use std::convert::TryFrom;
use zerocopy::{AsBytes, FromBytes};

#[test]
fn test_coinbase_complete_roundtrip_with_semantic_preservation() {
    // Step 1: Original Coinbase JSON data
    let original_json = r#"{
        "type": "match",
        "trade_id": 865127782,
        "maker_order_id": "5f4bb11b-f065-4025-ad53-2091b10ad2cf",
        "taker_order_id": "66715b57-0167-4ae9-8b2b-75a064a923f4",
        "side": "buy",
        "size": "0.00004147",
        "price": "116827.85",
        "product_id": "BTC-USD",
        "sequence": 110614077300,
        "time": "2025-08-22T20:11:30.012637Z"
    }"#;

    let json_value: Value = serde_json::from_str(original_json).unwrap();

    // Step 2: Parse to CoinbaseMatchEvent
    let parsed_event: CoinbaseMatchEvent = serde_json::from_value(json_value.clone()).unwrap();

    // Verify semantic preservation from JSON to struct
    assert_eq!(parsed_event.trade_id, 865127782);
    assert_eq!(parsed_event.side, "buy");
    assert_eq!(parsed_event.price, "116827.85");
    assert_eq!(parsed_event.size, "0.00004147");
    assert_eq!(parsed_event.product_id, "BTC-USD");

    // Step 3: Convert to TradeTLV
    let original_tlv = TradeTLV::try_from(parsed_event.clone()).unwrap();

    // Verify semantic mapping to TLV
    assert_eq!(original_tlv.venue().unwrap(), VenueId::Coinbase);

    // Copy packed fields to avoid unaligned access
    let tlv_price = original_tlv.price;
    let tlv_volume = original_tlv.volume;
    let tlv_side = original_tlv.side;

    // Verify correct fixed-point conversion (8 decimals)
    assert_eq!(tlv_price, 11682785000000); // 116827.85 * 1e8
    assert_eq!(tlv_volume, 4147); // 0.00004147 * 1e8
    assert_eq!(tlv_side, 0); // buy = 0

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

    // Step 7: Re-serialize and verify produces identical bytes
    let re_serialized = recovered_tlv.as_bytes();
    assert_eq!(
        binary_bytes, re_serialized,
        "Re-serialization produced different bytes!"
    );

    // Step 8: Verify SEMANTIC EQUALITY with original Coinbase data
    // The TLV should preserve the economic meaning of the original trade

    // Price semantic check (original: "116827.85" USD)
    let recovered_price_usd = (recovered_tlv.price as f64) / 100_000_000.0;
    assert!(
        (recovered_price_usd - 116827.85).abs() < 0.000001,
        "Price semantic corrupted: {} != 116827.85",
        recovered_price_usd
    );

    // Volume semantic check (original: "0.00004147" BTC)
    let recovered_volume_btc = (recovered_tlv.volume as f64) / 100_000_000.0;
    assert!(
        (recovered_volume_btc - 0.00004147).abs() < 0.000000001,
        "Volume semantic corrupted: {} != 0.00004147",
        recovered_volume_btc
    );

    // Side semantic check (original: "buy")
    assert_eq!(recovered_tlv.side, 0, "Buy side should map to 0");

    // Instrument semantic check (original: "BTC-USD")
    let instrument = recovered_tlv.instrument_id();
    let instrument_venue = instrument.venue;
    assert_eq!(instrument_venue, VenueId::Coinbase as u16);

    let recovered_price_display = recovered_tlv.price;
    println!("✅ Complete roundtrip validation successful!");
    println!("   Original JSON price: {}", json_value["price"]);
    println!("   TLV fixed-point price: {}", recovered_price_display);
    println!("   Recovered USD price: {:.2}", recovered_price_usd);
    println!("   Deep equality: PASSED");
    println!("   Semantic preservation: PASSED");
}

#[test]
fn test_coinbase_roundtrip_edge_cases() {
    // Test very small values
    let small_trade = CoinbaseMatchEvent {
        event_type: "match".to_string(),
        trade_id: 1,
        maker_order_id: "m1".to_string(),
        taker_order_id: "t1".to_string(),
        side: "sell".to_string(),
        size: "0.00000001".to_string(), // Minimum Bitcoin unit (1 satoshi)
        price: "0.01".to_string(),      // Very low price
        product_id: "BTC-USD".to_string(),
        sequence: 1,
        time: "2025-08-22T20:11:30.012637Z".to_string(),
    };

    let tlv = TradeTLV::try_from(small_trade).unwrap();
    let bytes = tlv.as_bytes();
    let recovered = TradeTLV::from_bytes(bytes).unwrap();

    assert_eq!(tlv, recovered, "Small value roundtrip failed");
    let tlv_volume = tlv.volume;
    let tlv_price = tlv.price;
    assert_eq!(tlv_volume, 1); // 0.00000001 * 1e8 = 1
    assert_eq!(tlv_price, 1000000); // 0.01 * 1e8

    // Test large values
    let large_trade = CoinbaseMatchEvent {
        event_type: "match".to_string(),
        trade_id: 999999999,
        maker_order_id: "m2".to_string(),
        taker_order_id: "t2".to_string(),
        side: "buy".to_string(),
        size: "1000000.0".to_string(),  // Large volume
        price: "999999.99".to_string(), // High price
        product_id: "BTC-USD".to_string(),
        sequence: 999999999,
        time: "2025-08-22T20:11:30.012637Z".to_string(),
    };

    let tlv = TradeTLV::try_from(large_trade).unwrap();
    let bytes = tlv.as_bytes();
    let recovered = TradeTLV::from_bytes(bytes).unwrap();

    assert_eq!(tlv, recovered, "Large value roundtrip failed");
    let tlv_volume = tlv.volume;
    let tlv_price = tlv.price;
    assert_eq!(tlv_volume, 100000000000000); // 1000000 * 1e8
    assert_eq!(tlv_price, 99999999000000); // 999999.99 * 1e8
}

#[test]
fn test_coinbase_semantic_preservation_all_products() {
    let products = vec![
        ("BTC-USD", "Bitcoin"),
        ("ETH-USD", "Ethereum"),
        ("SOL-USD", "Solana"),
    ];

    for (product_id, name) in products {
        let event = CoinbaseMatchEvent {
            event_type: "match".to_string(),
            trade_id: 12345,
            maker_order_id: "maker".to_string(),
            taker_order_id: "taker".to_string(),
            side: "buy".to_string(),
            size: "1.0".to_string(),
            price: "100.0".to_string(),
            product_id: product_id.to_string(),
            sequence: 12345,
            time: "2025-08-22T20:11:30.012637Z".to_string(),
        };

        let tlv = TradeTLV::try_from(event.clone()).unwrap();
        let bytes = tlv.as_bytes();
        let recovered = TradeTLV::from_bytes(bytes).unwrap();

        // Deep equality
        assert_eq!(tlv, recovered, "{} deep equality failed", name);

        // Semantic preservation
        assert_eq!(recovered.venue().unwrap(), VenueId::Coinbase);
        let rec_price = recovered.price;
        let rec_volume = recovered.volume;
        assert_eq!(rec_price, 10000000000); // 100.0 * 1e8
        assert_eq!(rec_volume, 100000000); // 1.0 * 1e8

        println!("✅ {} ({}) roundtrip successful", name, product_id);
    }
}
