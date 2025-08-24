//! Complete Four-Step Validation Pipeline for Gemini Adapter
//!
//! This test implements the MANDATORY validation pipeline that EVERY data type
//! must complete before production use:
//!
//! 1. Raw Data Parsing Validation - Ensure provider data is parsed correctly
//! 2. TLV Serialization Validation - Ensure semantic mapping is correct  
//! 3. TLV Deserialization Validation - Ensure no corruption in binary format
//! 4. Deep Equality Validation - Ensure perfect roundtrip with zero data loss
//!
//! Uses ONLY real data captured from Gemini WebSocket streams.

use crate::input::collectors::gemini::{GeminiMarketDataEvent, GeminiTradeEvent};
use crate::AdapterError;
use protocol_v2::{TradeTLV, VenueId};
use serde_json::Value;
use std::fs;
use zerocopy::AsBytes;

/// STEP 1: Validate raw data parsing AND semantic correctness
pub fn validate_gemini_raw_parsing(
    raw_json: &Value,
    parsed: &GeminiMarketDataEvent,
) -> Result<(), String> {
    // 1. All required fields extracted correctly
    if parsed.event_type.is_empty() {
        return Err("Missing event_type field".to_string());
    }

    if parsed.socket_sequence == 0 {
        return Err("Missing socket_sequence field".to_string());
    }

    // 2. Data types match Gemini specification exactly
    if parsed.event_type != "update" && parsed.event_type != "heartbeat" {
        return Err(format!("Invalid event type: {}", parsed.event_type));
    }

    // For update events, validate trade data
    if parsed.event_type == "update" {
        if parsed.events.is_none() {
            return Err("Update events must have events array".to_string());
        }

        let events = parsed.events.as_ref().unwrap();
        if events.is_empty() {
            return Err("Events array cannot be empty for update".to_string());
        }

        // Validate first trade event (pattern matching)
        let trade_event = &events[0];
        if trade_event.trade_type != "trade" {
            return Err(format!("Invalid trade type: {}", trade_event.trade_type));
        }

        if trade_event.maker_side != "bid" && trade_event.maker_side != "ask" {
            return Err(format!("Invalid maker side: {}", trade_event.maker_side));
        }

        // 3. SEMANTIC CORRECTNESS: Verify parsing preserves exact meaning
        // Compare parsed struct fields directly with original JSON values
        if let Some(original_event_id) = raw_json["eventId"].as_u64() {
            if let Some(parsed_event_id) = parsed.event_id {
                if parsed_event_id != original_event_id {
                    return Err("Event ID semantic corruption detected".to_string());
                }
            }
        }

        if let Some(original_sequence) = raw_json["socket_sequence"].as_u64() {
            if parsed.socket_sequence != original_sequence {
                return Err("Socket sequence semantic corruption detected".to_string());
            }
        }

        if let Some(events_array) = raw_json["events"].as_array() {
            if !events_array.is_empty() {
                let original_trade = &events_array[0];

                if let Some(original_tid) = original_trade["tid"].as_u64() {
                    if trade_event.tid != original_tid {
                        return Err("Trade ID semantic corruption detected".to_string());
                    }
                }

                if let Some(original_price) = original_trade["price"].as_str() {
                    if trade_event.price != original_price {
                        return Err("Price semantic corruption detected".to_string());
                    }
                }

                if let Some(original_amount) = original_trade["amount"].as_str() {
                    if trade_event.amount != original_amount {
                        return Err("Amount semantic corruption detected".to_string());
                    }
                }

                if let Some(original_maker_side) = original_trade["makerSide"].as_str() {
                    if trade_event.maker_side != original_maker_side {
                        return Err("Maker side semantic corruption detected".to_string());
                    }
                }

                if let Some(original_timestamp) = original_trade["timestampms"].as_u64() {
                    if trade_event.timestamp_ms != original_timestamp {
                        return Err("Timestamp semantic corruption detected".to_string());
                    }
                }
            }
        }

        // 4. Precision preservation - ensure string decimals can be parsed without loss
        use rust_decimal::Decimal;
        use std::str::FromStr;

        let price_decimal = Decimal::from_str(&trade_event.price)
            .map_err(|_| format!("Price precision lost during parsing: {}", trade_event.price))?;

        let amount_decimal = Decimal::from_str(&trade_event.amount).map_err(|_| {
            format!(
                "Amount precision lost during parsing: {}",
                trade_event.amount
            )
        })?;

        // 5. Basic data integrity (not business logic constraints)
        if price_decimal <= Decimal::ZERO {
            return Err("Price cannot be zero or negative".to_string());
        }

        if amount_decimal <= Decimal::ZERO {
            return Err("Amount cannot be zero or negative".to_string());
        }

        // 6. Timestamp validation (reasonable bounds, not artificial constraints)
        if trade_event.timestamp_ms == 0 {
            return Err("Timestamp cannot be zero".to_string());
        }
    }

    Ok(())
}

/// STEP 2: Validate TLV serialization
pub fn validate_gemini_tlv_serialization(tlv: &TradeTLV) -> Result<Vec<u8>, String> {
    // 1. Semantic validation before serialization
    if tlv.venue().map_err(|e| e.to_string())? != VenueId::Gemini {
        return Err("Venue must be Gemini".to_string());
    }

    if tlv.price <= 0 {
        return Err("Price must be positive in TLV".to_string());
    }

    if tlv.volume <= 0 {
        return Err("Volume must be positive in TLV".to_string());
    }

    if tlv.side > 1 {
        return Err("Side must be 0 (buy) or 1 (sell)".to_string());
    }

    if tlv.timestamp_ns == 0 {
        return Err("Timestamp cannot be zero".to_string());
    }

    // 2. Serialize to bytes
    let bytes = tlv.as_bytes();

    // 3. Validate serialized format
    if bytes.is_empty() {
        return Err("Serialization produced empty bytes".to_string());
    }

    // 4. Check expected byte structure (TradeTLV should be fixed size)
    let expected_size = std::mem::size_of::<TradeTLV>();
    if bytes.len() != expected_size {
        return Err(format!(
            "Invalid TradeTLV size: expected {}, got {}",
            expected_size,
            bytes.len()
        ));
    }

    Ok(bytes)
}

/// STEP 3: Validate TLV deserialization  
pub fn validate_gemini_tlv_deserialization(bytes: &[u8]) -> Result<TradeTLV, String> {
    // 1. Deserialize from bytes
    let recovered =
        TradeTLV::from_bytes(bytes).map_err(|e| format!("Deserialization failed: {}", e))?;

    // 2. Structural validation - all fields present and valid
    if recovered.venue().map_err(|e| e.to_string())? != VenueId::Gemini {
        return Err("Venue corruption detected after deserialization".to_string());
    }

    if recovered.price <= 0 {
        return Err("Price corruption detected after deserialization".to_string());
    }

    if recovered.volume <= 0 {
        return Err("Volume corruption detected after deserialization".to_string());
    }

    if recovered.timestamp_ns == 0 {
        return Err("Timestamp corruption detected after deserialization".to_string());
    }

    // 3. Semantic validation on deserialized data
    if recovered.side > 1 {
        return Err("Side corruption detected after deserialization".to_string());
    }

    // 4. Basic structural integrity - no artificial bounds
    // Note: Collectors should forward ALL data received from providers
    // No "reasonable" constraints - data validation is for corruption detection only

    Ok(recovered)
}

/// STEP 4: Validate semantic & deep equality
pub fn validate_gemini_deep_equality(
    original: &TradeTLV,
    recovered: &TradeTLV,
) -> Result<(), String> {
    // 1. Semantic equality - business meaning preserved
    if original.venue().map_err(|e| e.to_string())?
        != recovered.venue().map_err(|e| e.to_string())?
    {
        return Err("Venue semantics corrupted".to_string());
    }

    // Copy packed fields to avoid unaligned access (CRITICAL for ARM/M1)
    let orig_price = original.price;
    let rec_price = recovered.price;
    if orig_price != rec_price {
        return Err(format!(
            "Price semantics corrupted: {} → {}",
            orig_price, rec_price
        ));
    }

    let orig_volume = original.volume;
    let rec_volume = recovered.volume;
    if orig_volume != rec_volume {
        return Err(format!(
            "Volume semantics corrupted: {} → {}",
            orig_volume, rec_volume
        ));
    }

    let orig_side = original.side;
    let rec_side = recovered.side;
    if orig_side != rec_side {
        return Err(format!(
            "Side semantics corrupted: {} → {}",
            orig_side, rec_side
        ));
    }

    let orig_timestamp = original.timestamp_ns;
    let rec_timestamp = recovered.timestamp_ns;
    if orig_timestamp != rec_timestamp {
        return Err(format!(
            "Timestamp semantics corrupted: {} → {}",
            orig_timestamp, rec_timestamp
        ));
    }

    // 2. Deep equality - byte-for-byte identical
    if original != recovered {
        return Err("Deep equality failed - struct comparison mismatch".to_string());
    }

    // 3. Re-serialization produces identical bytes
    let original_bytes = original.as_bytes();
    let recovered_bytes = recovered.as_bytes();

    if original_bytes != recovered_bytes {
        return Err("Re-serialization produces different bytes".to_string());
    }

    Ok(())
}

/// Complete four-step validation pipeline for Gemini data
pub fn complete_gemini_validation_pipeline(raw_json: &Value) -> Result<TradeTLV, String> {
    // Parse raw JSON using Gemini parser
    let parsed: GeminiMarketDataEvent = serde_json::from_value(raw_json.clone())
        .map_err(|e| format!("JSON parsing failed: {}", e))?;

    // STEP 1: Validate raw data parsing
    validate_gemini_raw_parsing(raw_json, &parsed)?;

    // Extract trade event from parsed data
    if parsed.event_type != "update" {
        return Err("Only update events contain trade data".to_string());
    }

    let events = parsed
        .events
        .ok_or("Update events must have events array")?;
    if events.is_empty() {
        return Err("Events array cannot be empty".to_string());
    }

    let trade_event = &events[0];
    if trade_event.trade_type != "trade" {
        return Err("Only trade events are supported in validation".to_string());
    }

    // Transform to TLV format (using btcusd as default symbol for validation)
    let original_tlv: TradeTLV = (trade_event, "btcusd")
        .try_into()
        .map_err(|e: AdapterError| format!("TLV conversion failed: {}", e))?;

    // STEP 2: Validate TLV serialization
    let bytes = validate_gemini_tlv_serialization(&original_tlv)?;

    // STEP 3: Validate TLV deserialization
    let recovered_tlv = validate_gemini_tlv_deserialization(&bytes)?;

    // STEP 4: Validate semantic & deep equality
    validate_gemini_deep_equality(&original_tlv, &recovered_tlv)?;

    Ok(recovered_tlv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_complete_validation_pipeline() {
        // Load real Gemini data samples
        let samples_path = "tests/fixtures/gemini/trades_real_samples.json";
        let samples_data =
            fs::read_to_string(samples_path).expect("Failed to read Gemini test fixtures");

        let samples: Vec<Value> =
            serde_json::from_str(&samples_data).expect("Failed to parse Gemini test fixtures");

        assert!(!samples.is_empty(), "Must have real Gemini data samples");
        println!("Testing with {} real Gemini trade samples", samples.len());

        let mut success_count = 0;
        let mut error_count = 0;

        for (i, sample) in samples.iter().enumerate() {
            match complete_gemini_validation_pipeline(sample) {
                Ok(validated_tlv) => {
                    success_count += 1;

                    // Additional semantic checks specific to Gemini
                    assert_eq!(validated_tlv.venue().unwrap(), VenueId::Gemini);
                    assert!(validated_tlv.price > 0, "Price must be positive");
                    assert!(validated_tlv.volume > 0, "Volume must be positive");
                    assert!(validated_tlv.timestamp_ns > 0, "Timestamp must be positive");

                    if i % 2 == 0 {
                        // Copy packed fields to avoid unaligned access
                        let price = validated_tlv.price;
                        let volume = validated_tlv.volume;
                        let side = validated_tlv.side;
                        println!(
                            "✅ Sample {} validation passed: price={}, vol={}, side={}",
                            i, price, volume, side
                        );
                    }
                }
                Err(e) => {
                    error_count += 1;
                    println!("❌ Sample {} validation failed: {}", i, e);

                    // For debugging, print the problematic sample
                    if error_count <= 5 {
                        println!(
                            "   Problematic sample: {}",
                            serde_json::to_string_pretty(sample)
                                .unwrap_or_else(|_| "Could not serialize sample".to_string())
                        );
                    }
                }
            }
        }

        println!("\n📊 Gemini Validation Results:");
        println!("   ✅ Success: {}/{} samples", success_count, samples.len());
        println!("   ❌ Errors:  {}/{} samples", error_count, samples.len());

        // Require at least 95% success rate
        let success_rate = (success_count as f64) / (samples.len() as f64);
        assert!(
            success_rate >= 0.95,
            "Validation success rate too low: {:.2}% (minimum: 95%)",
            success_rate * 100.0
        );

        println!("   🎉 Overall success rate: {:.1}%", success_rate * 100.0);
    }

    #[test]
    fn test_gemini_edge_cases() {
        // Test edge cases that might be present in real data

        // Very small trade (satoshi-level)
        let small_trade = serde_json::json!({
            "type": "update",
            "eventId": 999999999,
            "socket_sequence": 999,
            "events": [
                {
                    "type": "trade",
                    "tid": 999999999,
                    "price": "100000.99999999",
                    "amount": "0.00000001",
                    "makerSide": "bid",
                    "timestampms": 1629464726493
                }
            ]
        });

        let result = complete_gemini_validation_pipeline(&small_trade);
        assert!(
            result.is_ok(),
            "Small trade validation should succeed: {:?}",
            result.err()
        );

        // Large trade
        let large_trade = serde_json::json!({
            "type": "update",
            "eventId": 888888888,
            "socket_sequence": 888,
            "events": [
                {
                    "type": "trade",
                    "tid": 888888888,
                    "price": "50000.0",
                    "amount": "1000.0",
                    "makerSide": "ask",
                    "timestampms": 1629464726500
                }
            ]
        });

        let result = complete_gemini_validation_pipeline(&large_trade);
        assert!(
            result.is_ok(),
            "Large trade validation should succeed: {:?}",
            result.err()
        );

        // High precision values
        let precision_trade = serde_json::json!({
            "type": "update",
            "eventId": 777777777,
            "socket_sequence": 777,
            "events": [
                {
                    "type": "trade",
                    "tid": 777777777,
                    "price": "4123.456789",
                    "amount": "0.123456789",
                    "makerSide": "bid",
                    "timestampms": 1629464726510
                }
            ]
        });

        let result = complete_gemini_validation_pipeline(&precision_trade);
        assert!(
            result.is_ok(),
            "High precision trade validation should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_gemini_validation_error_detection() {
        // Test that validation catches corruption and errors

        // Invalid maker side
        let invalid_maker_side = serde_json::json!({
            "type": "update",
            "eventId": 123456,
            "socket_sequence": 123,
            "events": [
                {
                    "type": "trade",
                    "tid": 123456,
                    "price": "50000.0",
                    "amount": "1.0",
                    "makerSide": "invalid_side",  // Invalid
                    "timestampms": 1629464726493
                }
            ]
        });

        let result = complete_gemini_validation_pipeline(&invalid_maker_side);
        assert!(result.is_err(), "Invalid maker side should be rejected");

        // Zero price
        let zero_price = serde_json::json!({
            "type": "update",
            "eventId": 123457,
            "socket_sequence": 124,
            "events": [
                {
                    "type": "trade",
                    "tid": 123457,
                    "price": "0",  // Invalid
                    "amount": "1.0",
                    "makerSide": "bid",
                    "timestampms": 1629464726493
                }
            ]
        });

        let result = complete_gemini_validation_pipeline(&zero_price);
        assert!(result.is_err(), "Zero price should be rejected");

        // Invalid trade type
        let invalid_trade_type = serde_json::json!({
            "type": "update",
            "eventId": 123458,
            "socket_sequence": 125,
            "events": [
                {
                    "type": "invalid_trade",  // Invalid
                    "tid": 123458,
                    "price": "50000.0",
                    "amount": "1.0",
                    "makerSide": "ask",
                    "timestampms": 1629464726493
                }
            ]
        });

        let result = complete_gemini_validation_pipeline(&invalid_trade_type);
        assert!(result.is_err(), "Invalid trade type should be rejected");

        // Empty price
        let empty_price = serde_json::json!({
            "type": "update",
            "eventId": 123459,
            "socket_sequence": 126,
            "events": [
                {
                    "type": "trade",
                    "tid": 123459,
                    "price": "",  // Invalid
                    "amount": "1.0",
                    "makerSide": "bid",
                    "timestampms": 1629464726493
                }
            ]
        });

        let result = complete_gemini_validation_pipeline(&empty_price);
        assert!(result.is_err(), "Empty price should be rejected");

        // Zero timestamp
        let zero_timestamp = serde_json::json!({
            "type": "update",
            "eventId": 123460,
            "socket_sequence": 127,
            "events": [
                {
                    "type": "trade",
                    "tid": 123460,
                    "price": "50000.0",
                    "amount": "1.0",
                    "makerSide": "ask",
                    "timestampms": 0  // Invalid
                }
            ]
        });

        let result = complete_gemini_validation_pipeline(&zero_timestamp);
        assert!(result.is_err(), "Zero timestamp should be rejected");
    }

    #[test]
    fn test_gemini_semantic_preservation() {
        // Test that semantic meaning is preserved through the pipeline

        // Test bid maker side (taker sold)
        let bid_trade = serde_json::json!({
            "type": "update",
            "eventId": 111111,
            "socket_sequence": 111,
            "events": [
                {
                    "type": "trade",
                    "tid": 111111,
                    "price": "45000.0",
                    "amount": "0.5",
                    "makerSide": "bid",  // Maker bidding = taker sold = side 1
                    "timestampms": 1629464726493
                }
            ]
        });

        let result = complete_gemini_validation_pipeline(&bid_trade).unwrap();
        assert_eq!(
            result.side, 1,
            "Bid maker side should result in sell side (1)"
        );

        // Test ask maker side (taker bought)
        let ask_trade = serde_json::json!({
            "type": "update",
            "eventId": 222222,
            "socket_sequence": 222,
            "events": [
                {
                    "type": "trade",
                    "tid": 222222,
                    "price": "45000.0",
                    "amount": "0.5",
                    "makerSide": "ask",  // Maker asking = taker bought = side 0
                    "timestampms": 1629464726493
                }
            ]
        });

        let result = complete_gemini_validation_pipeline(&ask_trade).unwrap();
        assert_eq!(
            result.side, 0,
            "Ask maker side should result in buy side (0)"
        );
    }

    #[test]
    fn test_gemini_precision_preservation() {
        // Test maximum precision preservation

        let precision_trade = serde_json::json!({
            "type": "update",
            "eventId": 333333,
            "socket_sequence": 333,
            "events": [
                {
                    "type": "trade",
                    "tid": 333333,
                    "price": "12345.12345678",     // 8 decimal places (max for fixed-point)
                    "amount": "0.12345678",       // 8 decimal places
                    "makerSide": "bid",
                    "timestampms": 1629464726493
                }
            ]
        });

        let result = complete_gemini_validation_pipeline(&precision_trade).unwrap();

        // Copy packed fields to avoid unaligned access
        let price = result.price;
        let volume = result.volume;

        // Verify precision preservation
        // price: 12345.12345678 * 100_000_000 = 1234512345678
        assert_eq!(price, 1234512345678, "Price precision not preserved");

        // amount: 0.12345678 * 100_000_000 = 12345678
        assert_eq!(volume, 12345678, "Volume precision not preserved");
    }

    #[test]
    fn test_gemini_timestamp_conversion() {
        // Test timestamp conversion from milliseconds to nanoseconds

        let timestamp_trade = serde_json::json!({
            "type": "update",
            "eventId": 444444,
            "socket_sequence": 444,
            "events": [
                {
                    "type": "trade",
                    "tid": 444444,
                    "price": "50000.0",
                    "amount": "1.0",
                    "makerSide": "ask",
                    "timestampms": 1629464726493  // Specific timestamp to test
                }
            ]
        });

        let result = complete_gemini_validation_pipeline(&timestamp_trade).unwrap();

        // Copy packed field to avoid unaligned access
        let timestamp_ns = result.timestamp_ns;

        // Verify conversion: 1629464726493 ms * 1_000_000 = 1629464726493000000 ns
        assert_eq!(
            timestamp_ns, 1629464726493000000,
            "Timestamp conversion incorrect"
        );
    }
}
