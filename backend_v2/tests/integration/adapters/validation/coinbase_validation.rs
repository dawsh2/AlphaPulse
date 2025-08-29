//! Complete Four-Step Validation Pipeline for Coinbase Adapter
//!
//! This test implements the MANDATORY validation pipeline that EVERY data type
//! must complete before production use:
//!
//! 1. Raw Data Parsing Validation - Ensure provider data is parsed correctly
//! 2. TLV Serialization Validation - Ensure semantic mapping is correct
//! 3. TLV Deserialization Validation - Ensure no corruption in binary format
//! 4. Deep Equality Validation - Ensure perfect roundtrip with zero data loss
//!
//! Uses ONLY real data captured from Coinbase WebSocket streams.

use adapter_service::input::collectors::coinbase::CoinbaseMatchEvent;
use adapter_service::AdapterError;
use protocol_v2::{TradeTLV, VenueId};
use serde_json::Value;
use std::fs;
use zerocopy::AsBytes;

/// STEP 1: Validate raw data parsing AND semantic correctness
pub fn validate_coinbase_raw_parsing(
    raw_json: &Value,
    parsed: &CoinbaseMatchEvent,
) -> Result<(), String> {
    // 1. All required fields extracted correctly
    if parsed.event_type.is_empty() {
        return Err("Missing event_type field".to_string());
    }

    if parsed.product_id.is_empty() {
        return Err("Missing product_id field".to_string());
    }

    if parsed.price.is_empty() {
        return Err("Missing price field".to_string());
    }

    if parsed.size.is_empty() {
        return Err("Missing size field".to_string());
    }

    // 2. Data types match Coinbase specification exactly
    if parsed.event_type != "match" && parsed.event_type != "last_match" {
        return Err(format!("Invalid event type: {}", parsed.event_type));
    }

    if parsed.side != "buy" && parsed.side != "sell" {
        return Err(format!("Invalid side: {}", parsed.side));
    }

    // 3. SEMANTIC CORRECTNESS: Verify parsing preserves exact meaning
    // Compare parsed struct fields directly with original JSON values
    if let Some(original_trade_id) = raw_json["trade_id"].as_u64() {
        if parsed.trade_id != original_trade_id {
            return Err("Trade ID semantic corruption detected".to_string());
        }
    }

    if let Some(original_price) = raw_json["price"].as_str() {
        if parsed.price != original_price {
            return Err("Price semantic corruption detected".to_string());
        }
    }

    if let Some(original_size) = raw_json["size"].as_str() {
        if parsed.size != original_size {
            return Err("Size semantic corruption detected".to_string());
        }
    }

    if let Some(original_side) = raw_json["side"].as_str() {
        if parsed.side != original_side {
            return Err("Side semantic corruption detected".to_string());
        }
    }

    if let Some(original_product_id) = raw_json["product_id"].as_str() {
        if parsed.product_id != original_product_id {
            return Err("Product ID semantic corruption detected".to_string());
        }
    }

    if let Some(original_sequence) = raw_json["sequence"].as_u64() {
        if parsed.sequence != original_sequence {
            return Err("Sequence semantic corruption detected".to_string());
        }
    }

    if let Some(original_time) = raw_json["time"].as_str() {
        if parsed.time != original_time {
            return Err("Timestamp semantic corruption detected".to_string());
        }
    }

    // 4. Precision preservation - ensure string decimals can be parsed without loss
    use rust_decimal::Decimal;
    use std::str::FromStr;

    let price_decimal = Decimal::from_str(&parsed.price)
        .map_err(|_| format!("Price precision lost during parsing: {}", parsed.price))?;

    let size_decimal = Decimal::from_str(&parsed.size)
        .map_err(|_| format!("Size precision lost during parsing: {}", parsed.size))?;

    // 5. Basic data integrity (not business logic constraints)
    if price_decimal <= Decimal::ZERO {
        return Err("Price cannot be zero or negative".to_string());
    }

    if size_decimal <= Decimal::ZERO {
        return Err("Size cannot be zero or negative".to_string());
    }

    // 6. Timestamp format validation (semantic correctness)
    chrono::DateTime::parse_from_rfc3339(&parsed.time)
        .map_err(|_| format!("Invalid timestamp format: {}", parsed.time))?;

    Ok(())
}

/// STEP 2: Validate TLV serialization
pub fn validate_coinbase_tlv_serialization(tlv: &TradeTLV) -> Result<Vec<u8>, String> {
    // 1. Semantic validation before serialization
    if tlv.venue().map_err(|e| e.to_string())? != VenueId::Coinbase {
        return Err("Venue must be Coinbase".to_string());
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
pub fn validate_coinbase_tlv_deserialization(bytes: &[u8]) -> Result<TradeTLV, String> {
    // 1. Deserialize from bytes
    let recovered =
        TradeTLV::from_bytes(bytes).map_err(|e| format!("Deserialization failed: {}", e))?;

    // 2. Structural validation - all fields present and valid
    if recovered.venue().map_err(|e| e.to_string())? != VenueId::Coinbase {
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
pub fn validate_coinbase_deep_equality(
    original: &TradeTLV,
    recovered: &TradeTLV,
) -> Result<(), String> {
    // 1. Semantic equality - business meaning preserved
    if original.venue().map_err(|e| e.to_string())?
        != recovered.venue().map_err(|e| e.to_string())?
    {
        return Err("Venue semantics corrupted".to_string());
    }

    // Copy packed fields to avoid unaligned access
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

    // 3. Hash comparison - compare serialized bytes instead of struct hash
    // (TradeTLV doesn't implement Hash trait, use byte comparison instead)

    // 4. Re-serialization produces identical bytes
    let original_bytes = original.as_bytes();
    let recovered_bytes = recovered.as_bytes();

    if original_bytes != recovered_bytes {
        return Err("Re-serialization produces different bytes".to_string());
    }

    Ok(())
}

/// Complete four-step validation pipeline for Coinbase data
pub fn complete_coinbase_validation_pipeline(raw_json: &Value) -> Result<TradeTLV, String> {
    // Parse raw JSON using Coinbase parser
    let parsed: CoinbaseMatchEvent = serde_json::from_value(raw_json.clone())
        .map_err(|e| format!("JSON parsing failed: {}", e))?;

    // STEP 1: Validate raw data parsing
    validate_coinbase_raw_parsing(raw_json, &parsed)?;

    // Transform to TLV format
    let original_tlv: TradeTLV = parsed
        .try_into()
        .map_err(|e: AdapterError| format!("TLV conversion failed: {}", e))?;

    // STEP 2: Validate TLV serialization
    let bytes = validate_coinbase_tlv_serialization(&original_tlv)?;

    // STEP 3: Validate TLV deserialization
    let recovered_tlv = validate_coinbase_tlv_deserialization(&bytes)?;

    // STEP 4: Validate semantic & deep equality
    validate_coinbase_deep_equality(&original_tlv, &recovered_tlv)?;

    Ok(recovered_tlv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coinbase_complete_validation_pipeline() {
        // Load real Coinbase data samples
        let samples_path = "tests/fixtures/coinbase/trades_raw.json";
        let samples_data =
            fs::read_to_string(samples_path).expect("Failed to read Coinbase test fixtures");

        let samples: Vec<Value> =
            serde_json::from_str(&samples_data).expect("Failed to parse Coinbase test fixtures");

        assert!(!samples.is_empty(), "Must have real Coinbase data samples");
        println!("Testing with {} real Coinbase trade samples", samples.len());

        let mut success_count = 0;
        let mut error_count = 0;

        for (i, sample) in samples.iter().enumerate() {
            match complete_coinbase_validation_pipeline(sample) {
                Ok(validated_tlv) => {
                    success_count += 1;

                    // Additional semantic checks specific to Coinbase
                    assert_eq!(validated_tlv.venue().unwrap(), VenueId::Coinbase);
                    assert!(validated_tlv.price > 0, "Price must be positive");
                    assert!(validated_tlv.volume > 0, "Volume must be positive");
                    assert!(validated_tlv.timestamp_ns > 0, "Timestamp must be positive");

                    if i % 50 == 0 {
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

        println!("\n📊 Coinbase Validation Results:");
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
    fn test_coinbase_edge_cases() {
        // Test edge cases that might be present in real data

        // Very small trade
        let small_trade = serde_json::json!({
            "type": "match",
            "trade_id": 999999999,
            "maker_order_id": "small-maker",
            "taker_order_id": "small-taker",
            "side": "buy",
            "size": "0.00000001",  // Minimum Bitcoin unit
            "price": "100000.99999999",  // High precision price
            "product_id": "BTC-USD",
            "sequence": 999999999,
            "time": "2025-08-22T20:11:30.012637Z"
        });

        let result = complete_coinbase_validation_pipeline(&small_trade);
        assert!(
            result.is_ok(),
            "Small trade validation should succeed: {:?}",
            result.err()
        );

        // Large trade
        let large_trade = serde_json::json!({
            "type": "match",
            "trade_id": 888888888,
            "maker_order_id": "large-maker",
            "taker_order_id": "large-taker",
            "side": "sell",
            "size": "1000.0",  // Large size
            "price": "50000.0",
            "product_id": "BTC-USD",
            "sequence": 888888888,
            "time": "2025-08-22T20:11:30.012637Z"
        });

        let result = complete_coinbase_validation_pipeline(&large_trade);
        assert!(
            result.is_ok(),
            "Large trade validation should succeed: {:?}",
            result.err()
        );

        // Different product
        let eth_trade = serde_json::json!({
            "type": "match",
            "trade_id": 777777777,
            "maker_order_id": "eth-maker",
            "taker_order_id": "eth-taker",
            "side": "buy",
            "size": "5.5",
            "price": "4000.25",
            "product_id": "ETH-USD",
            "sequence": 777777777,
            "time": "2025-08-22T20:11:30.012637Z"
        });

        let result = complete_coinbase_validation_pipeline(&eth_trade);
        assert!(
            result.is_ok(),
            "ETH trade validation should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_coinbase_validation_error_detection() {
        // Test that validation catches corruption and errors

        // Invalid side
        let invalid_side = serde_json::json!({
            "type": "match",
            "trade_id": 123456,
            "maker_order_id": "maker",
            "taker_order_id": "taker",
            "side": "invalid_side",  // Invalid
            "size": "1.0",
            "price": "50000.0",
            "product_id": "BTC-USD",
            "sequence": 123456,
            "time": "2025-08-22T20:11:30.012637Z"
        });

        let result = complete_coinbase_validation_pipeline(&invalid_side);
        assert!(result.is_err(), "Invalid side should be rejected");

        // Zero price
        let zero_price = serde_json::json!({
            "type": "match",
            "trade_id": 123457,
            "maker_order_id": "maker",
            "taker_order_id": "taker",
            "side": "buy",
            "size": "1.0",
            "price": "0",  // Invalid
            "product_id": "BTC-USD",
            "sequence": 123457,
            "time": "2025-08-22T20:11:30.012637Z"
        });

        let result = complete_coinbase_validation_pipeline(&zero_price);
        assert!(result.is_err(), "Zero price should be rejected");

        // Invalid timestamp
        let invalid_time = serde_json::json!({
            "type": "match",
            "trade_id": 123458,
            "maker_order_id": "maker",
            "taker_order_id": "taker",
            "side": "sell",
            "size": "1.0",
            "price": "50000.0",
            "product_id": "BTC-USD",
            "sequence": 123458,
            "time": "invalid-timestamp"  // Invalid
        });

        let result = complete_coinbase_validation_pipeline(&invalid_time);
        assert!(result.is_err(), "Invalid timestamp should be rejected");
    }
}
