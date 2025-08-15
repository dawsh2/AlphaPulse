//! Data Conversion Module
//! 
//! Handles conversion from external data formats (JSON strings, various number representations)
//! to internal fixed-point integer representations without precision loss.

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::str::FromStr;
use thiserror::Error;
use crate::{SymbolDescriptor, TradeSide};

/// Number of decimal places used in fixed-point representation
pub const FIXED_POINT_DECIMALS: u32 = 8;

/// Multiplier for converting to fixed-point (10^8)
const FIXED_POINT_MULTIPLIER: i64 = 100_000_000;

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("Invalid decimal format: {0}")]
    InvalidDecimal(String),
    #[error("Numeric overflow: value {0} too large for fixed-point representation")]
    Overflow(String),
    #[error("Invalid timestamp format: {0}")]
    InvalidTimestamp(String),
    #[error("Invalid side format: {0}")]
    InvalidSide(String),
    #[error("Empty or null value")]
    EmptyValue,
    #[error("Negative value not allowed: {0}")]
    NegativeValue(String),
}

/// Exchange-specific timestamp formats
#[derive(Debug, Clone, Copy)]
pub enum TimestampFormat {
    /// Unix timestamp in seconds
    UnixSeconds,
    /// Unix timestamp in milliseconds
    UnixMilliseconds,
    /// Unix timestamp in microseconds  
    UnixMicroseconds,
    /// Unix timestamp in nanoseconds
    UnixNanoseconds,
    /// ISO 8601 string format
    Iso8601,
}

/// Convert a decimal string to fixed-point integer (8 decimal places) without precision loss
/// 
/// # Examples
/// ```
/// use alphapulse_protocol::conversion::parse_price_to_fixed_point;
/// 
/// // Exact conversion with no precision loss
/// assert_eq!(parse_price_to_fixed_point("4605.23").unwrap(), 460523000000);
/// assert_eq!(parse_price_to_fixed_point("0.00000001").unwrap(), 1);
/// assert_eq!(parse_price_to_fixed_point("1.0").unwrap(), 100000000);
/// ```
pub fn parse_price_to_fixed_point(price_str: &str) -> Result<i64, ConversionError> {
    if price_str.trim().is_empty() {
        return Err(ConversionError::EmptyValue);
    }

    // Parse as Decimal to maintain exact precision
    let decimal = Decimal::from_str(price_str.trim())
        .map_err(|_| ConversionError::InvalidDecimal(price_str.to_string()))?;

    // Check for negative values
    if decimal.is_sign_negative() {
        return Err(ConversionError::NegativeValue(price_str.to_string()));
    }

    // Convert to fixed-point (8 decimals) as integer
    let multiplier = Decimal::from(FIXED_POINT_MULTIPLIER);
    let fixed_point_decimal = decimal * multiplier;
    
    // Convert to i64, checking for overflow
    fixed_point_decimal.to_i64()
        .ok_or_else(|| ConversionError::Overflow(price_str.to_string()))
}

/// Convert a volume string to fixed-point integer, handling exchange-specific decimal places
/// 
/// Most exchanges send volume as decimal strings. This function preserves exact precision
/// by converting through Decimal arithmetic.
pub fn parse_volume_to_fixed_point(volume_str: &str) -> Result<i64, ConversionError> {
    // Volume uses same fixed-point conversion as price
    parse_price_to_fixed_point(volume_str)
}

/// Convert various timestamp formats to nanoseconds since Unix epoch
pub fn parse_timestamp_to_ns(timestamp_str: &str, format: TimestampFormat) -> Result<u64, ConversionError> {
    if timestamp_str.trim().is_empty() {
        return Err(ConversionError::EmptyValue);
    }

    match format {
        TimestampFormat::UnixSeconds => {
            let seconds = timestamp_str.parse::<f64>()
                .map_err(|_| ConversionError::InvalidTimestamp(timestamp_str.to_string()))?;
            Ok((seconds * 1_000_000_000.0) as u64)
        },
        TimestampFormat::UnixMilliseconds => {
            let millis = timestamp_str.parse::<u64>()
                .map_err(|_| ConversionError::InvalidTimestamp(timestamp_str.to_string()))?;
            Ok(millis * 1_000_000)
        },
        TimestampFormat::UnixMicroseconds => {
            let micros = timestamp_str.parse::<u64>()
                .map_err(|_| ConversionError::InvalidTimestamp(timestamp_str.to_string()))?;
            Ok(micros * 1_000)
        },
        TimestampFormat::UnixNanoseconds => {
            timestamp_str.parse::<u64>()
                .map_err(|_| ConversionError::InvalidTimestamp(timestamp_str.to_string()))
        },
        TimestampFormat::Iso8601 => {
            // For ISO 8601, we'd need a datetime parsing library
            // For now, return an error since most exchanges use Unix timestamps
            Err(ConversionError::InvalidTimestamp("ISO8601 not yet supported".to_string()))
        }
    }
}

/// Parse trade side from string representation
pub fn parse_trade_side(side_str: &str) -> Result<TradeSide, ConversionError> {
    match side_str.to_lowercase().as_str() {
        "buy" | "bid" | "b" => Ok(TradeSide::Buy),
        "sell" | "ask" | "s" => Ok(TradeSide::Sell),
        _ => Err(ConversionError::InvalidSide(side_str.to_string())),
    }
}

/// Convert fixed-point integer back to f64 for display purposes
/// 
/// This should only be used for final display/JSON output to preserve
/// the exact fixed-point precision throughout the pipeline.
pub fn fixed_point_to_f64(fixed_point: i64) -> f64 {
    fixed_point as f64 / FIXED_POINT_MULTIPLIER as f64
}

/// Convert fixed-point integer to Decimal for exact arithmetic
/// 
/// Useful when you need exact decimal arithmetic on the stored values.
pub fn fixed_point_to_decimal(fixed_point: i64) -> Decimal {
    Decimal::from(fixed_point) / Decimal::from(FIXED_POINT_MULTIPLIER)
}

/// Normalize symbol string to canonical format
/// 
/// Converts exchange-specific symbol formats to our standard format.
/// Examples:
/// - "BTC-USD" → "BTC-USD" (already canonical)
/// - "BTCUSD" → "BTC-USD" (add separator)
/// - "btc_usd" → "BTC-USD" (normalize case and separator)
pub fn normalize_symbol(symbol: &str, exchange: &str) -> Result<String, ConversionError> {
    let normalized = symbol.trim().to_uppercase();
    
    if normalized.is_empty() {
        return Err(ConversionError::EmptyValue);
    }

    // Exchange-specific normalization rules
    let canonical = match exchange.to_lowercase().as_str() {
        "coinbase" => {
            // Coinbase uses "BTC-USD" format (already canonical)
            normalized
        },
        "kraken" => {
            // Kraken might use different formats, normalize to our standard
            normalized.replace('/', "-").replace('_', "-")
        },
        "binance" => {
            // Binance uses "BTCUSDT" format, need to split
            if normalized.len() >= 6 && !normalized.contains('-') {
                // Simple heuristic: assume last 3-4 chars are quote currency
                if normalized.ends_with("USDT") {
                    let base = &normalized[..normalized.len()-4];
                    format!("{}-USDT", base)
                } else if normalized.ends_with("USD") || normalized.ends_with("BTC") || normalized.ends_with("ETH") {
                    let base = &normalized[..normalized.len()-3];
                    let quote = &normalized[normalized.len()-3..];
                    format!("{}-{}", base, quote)
                } else {
                    normalized
                }
            } else {
                normalized
            }
        },
        _ => {
            // Default: just normalize separators
            normalized.replace('/', "-").replace('_', "-")
        }
    };

    Ok(canonical)
}

/// Create a SymbolDescriptor from exchange symbol string
pub fn parse_symbol_descriptor(symbol: &str, exchange: &str) -> Result<SymbolDescriptor, ConversionError> {
    let canonical = normalize_symbol(symbol, exchange)?;
    
    SymbolDescriptor::parse(&format!("{}:{}", exchange, canonical))
        .ok_or_else(|| ConversionError::InvalidDecimal(format!("Invalid symbol format: {}", symbol)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_conversion_precision() {
        // Test cases that should maintain exact precision
        let test_cases = vec![
            ("4605.23", 460523000000i64),
            ("0.00000001", 1i64),
            ("1.0", 100000000i64),
            ("65000.0", 6500000000000i64),
            ("0.9998", 99980000i64),
            ("123.45678900", 12345678900i64), // Exact 8 decimals
        ];

        for (input, expected) in test_cases {
            let result = parse_price_to_fixed_point(input).unwrap();
            assert_eq!(result, expected, "Failed for input: {}", input);
            
            // Test round-trip conversion
            let recovered = fixed_point_to_f64(result);
            let original: f64 = input.parse().unwrap();
            let diff = (recovered - original).abs();
            assert!(diff < 1e-8, "Round-trip precision loss for {}: {} vs {}", input, original, recovered);
        }
    }

    #[test]
    fn test_precision_vs_float() {
        // Compare our exact conversion vs f64 conversion
        let test_price = "4605.23";
        
        // Our exact method
        let exact_fp = parse_price_to_fixed_point(test_price).unwrap();
        let exact_recovered = fixed_point_to_f64(exact_fp);
        
        // f64 method (broken)
        let float_parsed: f64 = test_price.parse().unwrap();
        let float_fp = (float_parsed * 100000000.0) as i64;
        let float_recovered = float_fp as f64 / 100000000.0;
        
        // Our method should be more accurate
        let original: f64 = test_price.parse().unwrap();
        let exact_error = (exact_recovered - original).abs();
        let float_error = (float_recovered - original).abs();
        
        println!("Original: {}", original);
        println!("Exact method: {} (error: {})", exact_recovered, exact_error);
        println!("Float method: {} (error: {})", float_recovered, float_error);
        
        // Our method should have zero or minimal error
        assert!(exact_error <= float_error, "Exact method should be at least as accurate");
    }

    #[test]
    fn test_edge_cases() {
        // Test edge cases and error conditions
        assert!(parse_price_to_fixed_point("").is_err());
        assert!(parse_price_to_fixed_point("   ").is_err());
        assert!(parse_price_to_fixed_point("abc").is_err());
        assert!(parse_price_to_fixed_point("-100").is_err()); // Negative price
        
        // Test very small values
        assert_eq!(parse_price_to_fixed_point("0.00000001").unwrap(), 1);
        assert_eq!(parse_price_to_fixed_point("0.000000001").unwrap(), 0); // Rounds down
        
        // Test very large values (should not overflow)
        let large_price = "1000000.0"; // 1M should be fine
        assert!(parse_price_to_fixed_point(large_price).is_ok());
    }

    #[test]
    fn test_timestamp_conversion() {
        // Test different timestamp formats
        assert_eq!(parse_timestamp_to_ns("1609459200", TimestampFormat::UnixSeconds).unwrap(), 1609459200000000000);
        assert_eq!(parse_timestamp_to_ns("1609459200000", TimestampFormat::UnixMilliseconds).unwrap(), 1609459200000000000);
        assert_eq!(parse_timestamp_to_ns("1609459200000000", TimestampFormat::UnixMicroseconds).unwrap(), 1609459200000000000);
        assert_eq!(parse_timestamp_to_ns("1609459200000000000", TimestampFormat::UnixNanoseconds).unwrap(), 1609459200000000000);
    }

    #[test]
    fn test_trade_side_parsing() {
        assert_eq!(parse_trade_side("buy").unwrap(), TradeSide::Buy);
        assert_eq!(parse_trade_side("BUY").unwrap(), TradeSide::Buy);
        assert_eq!(parse_trade_side("bid").unwrap(), TradeSide::Buy);
        assert_eq!(parse_trade_side("sell").unwrap(), TradeSide::Sell);
        assert_eq!(parse_trade_side("SELL").unwrap(), TradeSide::Sell);
        assert_eq!(parse_trade_side("ask").unwrap(), TradeSide::Sell);
        
        assert!(parse_trade_side("invalid").is_err());
    }

    #[test]
    fn test_symbol_normalization() {
        // Test Coinbase (already canonical)
        assert_eq!(normalize_symbol("BTC-USD", "coinbase").unwrap(), "BTC-USD");
        
        // Test Binance (no separator)
        assert_eq!(normalize_symbol("BTCUSDT", "binance").unwrap(), "BTC-USDT");
        assert_eq!(normalize_symbol("ETHUSDT", "binance").unwrap(), "ETH-USDT");
        assert_eq!(normalize_symbol("BTCUSD", "binance").unwrap(), "BTC-USD");
        
        // Test Kraken (various separators)
        assert_eq!(normalize_symbol("BTC/USD", "kraken").unwrap(), "BTC-USD");
        assert_eq!(normalize_symbol("BTC_USD", "kraken").unwrap(), "BTC-USD");
        
        // Test case normalization
        assert_eq!(normalize_symbol("btc-usd", "coinbase").unwrap(), "BTC-USD");
    }

    #[test]
    fn test_decimal_precision() {
        // Test with Decimal for exact arithmetic
        let price_fp = parse_price_to_fixed_point("4605.23").unwrap();
        let decimal = fixed_point_to_decimal(price_fp);
        
        // Should be exactly 4605.23 in decimal
        assert_eq!(decimal.to_string(), "4605.23");
        
        // Test arithmetic with exact precision
        let doubled = decimal * Decimal::from(2);
        assert_eq!(doubled.to_string(), "9210.46");
    }
}