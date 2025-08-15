// Test to verify numerical accuracy between JSON input -> binary protocol -> output

use alphapulse_protocol::*;
use serde_json::json;

#[test]
fn test_trade_message_price_accuracy() {
    // Test various price scenarios
    let test_cases = vec![
        // (json_price, expected_fixed_point, description)
        (0.00000001, 1u64, "Very small price (1 satoshi)"),
        (0.001, 100000u64, "Small price"),
        (1.0, 100000000u64, "Unity price"),
        (100.0, 10000000000u64, "Medium price"),
        (65000.0, 6500000000000u64, "Bitcoin-like price"),
        (0.9986, 99860000u64, "Stablecoin price"),
        (2171279.172451, 217127917245100u64, "Large DEX price"),
    ];

    for (json_price, expected_fp, description) in test_cases {
        // Convert JSON price to fixed-point (8 decimals)
        let fixed_point = (json_price * 1e8) as u64;
        assert_eq!(fixed_point, expected_fp, "Failed for: {}", description);
        
        // Create trade message
        let trade = TradeMessage::new(
            1234567890000000000,
            fixed_point,
            100000000, // 1.0 volume
            12345678,
            TradeSide::Unknown,
        );
        
        // Convert back to f64
        let decoded_price = trade.price_f64();
        let diff = (decoded_price - json_price).abs();
        
        // Allow for small floating point errors (< 0.0000001%)
        let tolerance = json_price * 1e-9;
        assert!(
            diff < tolerance.max(1e-9),
            "Price conversion failed for {}: JSON={}, decoded={}, diff={}",
            description, json_price, decoded_price, diff
        );
    }
}

#[test]
fn test_volume_accuracy() {
    let test_cases = vec![
        (0.0, 0u64, "Zero volume"),
        (0.001, 100000u64, "Small volume"),
        (100.0, 10000000000u64, "Medium volume"),
        (1000000.0, 100000000000000u64, "Large volume"),
    ];

    for (json_volume, expected_fp, description) in test_cases {
        let fixed_point = (json_volume * 1e8) as u64;
        assert_eq!(fixed_point, expected_fp, "Failed for: {}", description);
        
        let trade = TradeMessage::new(
            1234567890000000000,
            100000000, // 1.0 price
            fixed_point,
            12345678,
            TradeSide::Unknown,
        );
        
        let decoded_volume = trade.volume_f64();
        let diff = (decoded_volume - json_volume).abs();
        let tolerance = json_volume * 1e-9;
        
        assert!(
            diff < tolerance.max(1e-9),
            "Volume conversion failed for {}: JSON={}, decoded={}, diff={}",
            description, json_volume, decoded_volume, diff
        );
    }
}

#[test]
fn test_symbol_hash_consistency() {
    // Test that symbol hashing is consistent
    let test_symbols = vec![
        ("quickswap", "WMATIC", "USDC"),
        ("sushiswap", "WETH", "USDC"),
        ("coinbase", "BTC", "USD"),
        ("quickswap", "LINK", "USDC"),
        ("sushiswap", "AAVE", "USDC"),
    ];
    
    for (exchange, base, quote) in test_symbols {
        let descriptor1 = SymbolDescriptor::spot(exchange, base, quote);
        let descriptor2 = SymbolDescriptor::spot(exchange, base, quote);
        
        assert_eq!(
            descriptor1.hash(),
            descriptor2.hash(),
            "Hash mismatch for {}:{}-{}",
            exchange, base, quote
        );
        
        // Verify hash is non-zero
        assert_ne!(descriptor1.hash(), 0, "Hash is zero for {}:{}-{}", exchange, base, quote);
    }
}

#[test]
fn test_extreme_values() {
    // Test handling of extreme values that might cause overflow
    let test_cases = vec![
        // Maximum safe JavaScript integer (2^53 - 1)
        (9007199254740991.0f64, "Max safe JS integer"),
        // Very small but non-zero
        (1e-8, "Minimum representable value"),
        // Large DEX prices seen in production
        (997493747671.9919, "Large DAI/USDC price"),
        (258011226939.4053, "Large WETH/WBTC price"),
    ];
    
    for (value, description) in test_cases {
        // Check if we can safely convert to fixed-point
        let fp = (value * 1e8) as u64;
        
        // Create a trade message
        let trade = TradeMessage::new(
            1234567890000000000,
            fp,
            100000000,
            12345678,
            TradeSide::Unknown,
        );
        
        let decoded = trade.price_f64();
        let relative_error = ((decoded - value) / value).abs();
        
        // For large values, allow up to 0.01% relative error due to floating point
        assert!(
            relative_error < 0.0001,
            "Failed for {}: value={}, decoded={}, error={}%",
            description, value, decoded, relative_error * 100.0
        );
    }
}

#[test]
fn test_json_to_binary_roundtrip() {
    // Simulate the full pipeline: JSON -> Binary Protocol -> JSON
    let json_trade = json!({
        "symbol": "quickswap:WETH-USDC",
        "price": 4605.23,
        "volume": 1500.75,
        "timestamp": 1234567890000u64,
        "side": "unknown"
    });
    
    // Extract values
    let price = json_trade["price"].as_f64().unwrap();
    let volume = json_trade["volume"].as_f64().unwrap();
    let timestamp_ms = json_trade["timestamp"].as_u64().unwrap();
    
    // Convert to protocol message
    let descriptor = SymbolDescriptor::spot("quickswap", "WETH", "USDC");
    let trade_msg = TradeMessage::new(
        timestamp_ms * 1_000_000, // Convert to nanoseconds
        (price * 1e8) as u64,
        (volume * 1e8) as u64,
        descriptor.hash(),
        TradeSide::Unknown,
    );
    
    // Convert back to JSON-like values
    let output_json = json!({
        "symbol_hash": trade_msg.symbol_hash(),
        "price": trade_msg.price_f64(),
        "volume": trade_msg.volume_f64(),
        "timestamp": trade_msg.timestamp_ns() / 1_000_000,
    });
    
    // Verify accuracy
    assert!((output_json["price"].as_f64().unwrap() - price).abs() < 0.00001);
    assert!((output_json["volume"].as_f64().unwrap() - volume).abs() < 0.00001);
    assert_eq!(output_json["timestamp"].as_u64().unwrap(), timestamp_ms);
}

fn main() {
    println!("Running numerical accuracy tests...");
    test_trade_message_price_accuracy();
    test_volume_accuracy();
    test_symbol_hash_consistency();
    test_extreme_values();
    test_json_to_binary_roundtrip();
    println!("✅ All tests passed!");
}