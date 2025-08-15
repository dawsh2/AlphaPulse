//! Data Validation Module
//! 
//! Validates input data ranges and formats to ensure data integrity and prevent
//! corruption before conversion to internal formats.

use thiserror::Error;
use crate::conversion::{fixed_point_to_f64, FIXED_POINT_DECIMALS};

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Price out of range: {price} for asset type {asset_type}")]
    PriceOutOfRange { price: f64, asset_type: String },
    #[error("Volume out of range: {volume} for symbol {symbol}")]
    VolumeOutOfRange { volume: f64, symbol: String },
    #[error("Timestamp out of range: {timestamp}")]
    TimestampOutOfRange { timestamp: u64 },
    #[error("Invalid precision: {value} has more than {max_decimals} decimal places")]
    InvalidPrecision { value: String, max_decimals: u32 },
    #[error("Extreme value detected: {value} for field {field}")]
    ExtremeValue { value: f64, field: String },
    #[error("Stale data: timestamp {timestamp} is too old")]
    StaleData { timestamp: u64 },
}

/// Asset types for validation
#[derive(Debug, Clone, PartialEq)]
pub enum AssetType {
    /// Cryptocurrency pairs (e.g., BTC-USD, ETH-USDC)
    Crypto,
    /// Traditional stocks (e.g., AAPL, GOOGL)
    Stock,
    /// Foreign exchange pairs (e.g., EUR-USD)
    Forex,
    /// Commodities (e.g., Gold, Oil)
    Commodity,
    /// Derivatives (futures, options)
    Derivative,
    /// Stablecoins (USDC, USDT, DAI)
    Stablecoin,
}

/// Price range configuration for different asset types
pub struct PriceRanges {
    pub min: f64,
    pub max: f64,
    pub typical_min: f64,
    pub typical_max: f64,
}

impl AssetType {
    /// Get reasonable price ranges for the asset type
    pub fn price_ranges(&self) -> PriceRanges {
        match self {
            AssetType::Crypto => PriceRanges {
                min: 0.00000001,        // Minimum satoshi-like precision
                max: 10_000_000.0,      // $10M per token (extreme but possible)
                typical_min: 0.0001,    // $0.0001 (small altcoins)
                typical_max: 500_000.0, // $500K (BTC at extreme)
            },
            AssetType::Stock => PriceRanges {
                min: 0.01,              // Penny stocks
                max: 1_000_000.0,       // $1M per share (Berkshire Hathaway)
                typical_min: 1.0,       // $1 typical minimum
                typical_max: 5_000.0,   // $5K typical maximum
            },
            AssetType::Forex => PriceRanges {
                min: 0.0001,            // Very weak currencies
                max: 1000.0,            // Strong currencies vs weak
                typical_min: 0.01,      // Typical range
                typical_max: 100.0,     // Typical range
            },
            AssetType::Commodity => PriceRanges {
                min: 0.01,              // Cheap commodities
                max: 100_000.0,         // Expensive metals per oz
                typical_min: 1.0,       // Typical range
                typical_max: 10_000.0,  // Typical range
            },
            AssetType::Derivative => PriceRanges {
                min: 0.01,              // Options can be very cheap
                max: 1_000_000.0,       // Futures can be expensive
                typical_min: 1.0,       // Typical range
                typical_max: 100_000.0, // Typical range
            },
            AssetType::Stablecoin => PriceRanges {
                min: 0.90,              // Depeg protection
                max: 1.10,              // Depeg protection
                typical_min: 0.995,     // Normal range
                typical_max: 1.005,     // Normal range
            },
        }
    }
}

/// Determine asset type from symbol
pub fn classify_asset_type(symbol: &str) -> AssetType {
    let symbol_upper = symbol.to_uppercase();
    
    // Parse base asset from symbol (before the first - or :)
    let base_asset = if let Some(pos) = symbol_upper.find('-') {
        &symbol_upper[..pos]
    } else if let Some(pos) = symbol_upper.find(':') {
        // For "exchange:pair" format, get the base from pair
        let pair_part = &symbol_upper[pos+1..];
        if let Some(dash_pos) = pair_part.find('-') {
            &pair_part[..dash_pos]
        } else {
            pair_part
        }
    } else {
        &symbol_upper
    };
    
    // Classify based on base asset primarily
    if base_asset == "USDC" || base_asset == "USDT" || base_asset == "DAI" || 
       base_asset == "FRAX" || base_asset == "LUSD" || base_asset == "BUSD" {
        return AssetType::Stablecoin;
    }
    
    // Check for crypto base assets
    if base_asset == "BTC" || base_asset == "ETH" || base_asset == "WETH" || 
       base_asset == "WBTC" || base_asset == "MATIC" || base_asset == "WMATIC" ||
       base_asset == "LINK" || base_asset == "AAVE" || base_asset == "UNI" ||
       base_asset == "SUSHI" {
        return AssetType::Crypto;
    }
    
    // Check for forex pairs (both assets are fiat currencies)
    if (base_asset == "EUR" || base_asset == "GBP" || base_asset == "JPY" ||
        base_asset == "CHF" || base_asset == "CAD") && symbol_upper.contains("USD") {
        return AssetType::Forex;
    }
    
    // Special case: stablecoin pairs (both are stablecoins)
    if symbol_upper.contains("USDC-USDT") || symbol_upper.contains("USDT-USDC") ||
       symbol_upper.contains("DAI-USDC") || symbol_upper.contains("USDC-DAI") {
        return AssetType::Stablecoin;
    }
    
    // Default to crypto for most cases in our system
    AssetType::Crypto
}

/// Validate price range for specific asset type
pub fn validate_price_range(price_fp: i64, symbol: &str) -> Result<(), ValidationError> {
    let price = fixed_point_to_f64(price_fp);
    let asset_type = classify_asset_type(symbol);
    let ranges = asset_type.price_ranges();
    
    // Check absolute bounds
    if price < ranges.min || price > ranges.max {
        return Err(ValidationError::PriceOutOfRange { 
            price, 
            asset_type: format!("{:?}", asset_type) 
        });
    }
    
    // For stablecoins, be extra strict
    if asset_type == AssetType::Stablecoin {
        if price < 0.50 || price > 2.0 {
            return Err(ValidationError::ExtremeValue { 
                value: price, 
                field: "stablecoin_price".to_string() 
            });
        }
    }
    
    // Check for suspiciously extreme values within bounds
    if price > ranges.typical_max * 10.0 {
        // Log warning but don't fail - markets can be extreme
        eprintln!("Warning: Extreme price detected: {} for {}", price, symbol);
    }
    
    Ok(())
}

/// Validate volume range 
pub fn validate_volume_range(volume_fp: i64, symbol: &str) -> Result<(), ValidationError> {
    let volume = fixed_point_to_f64(volume_fp);
    
    // Volume must be non-negative
    if volume < 0.0 {
        return Err(ValidationError::VolumeOutOfRange { volume, symbol: symbol.to_string() });
    }
    
    // Check for reasonable volume limits
    // These are very generous bounds to catch obvious errors
    let max_reasonable_volume = match classify_asset_type(symbol) {
        AssetType::Crypto => 1_000_000_000.0,      // $1B volume in single trade
        AssetType::Stock => 100_000_000.0,         // $100M stock trade
        AssetType::Forex => 10_000_000_000.0,      // $10B forex trade
        AssetType::Commodity => 1_000_000_000.0,   // $1B commodity trade
        AssetType::Derivative => 1_000_000_000.0,  // $1B derivative trade
        AssetType::Stablecoin => 1_000_000_000.0,  // $1B stablecoin trade
    };
    
    if volume > max_reasonable_volume {
        return Err(ValidationError::VolumeOutOfRange { volume, symbol: symbol.to_string() });
    }
    
    Ok(())
}

/// Validate timestamp is reasonable (not too old, not in future)
pub fn validate_timestamp(timestamp_ns: u64, _exchange: &str) -> Result<(), ValidationError> {
    let now_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    // Allow up to 1 hour in the future (for clock skew)
    let max_future_ns = now_ns + (3600 * 1_000_000_000);
    
    // Allow up to 24 hours in the past (for replay/backfill)
    let max_past_ns = 24 * 3600 * 1_000_000_000;
    let min_timestamp_ns = now_ns.saturating_sub(max_past_ns);
    
    if timestamp_ns > max_future_ns {
        return Err(ValidationError::TimestampOutOfRange { timestamp: timestamp_ns });
    }
    
    if timestamp_ns < min_timestamp_ns {
        return Err(ValidationError::StaleData { timestamp: timestamp_ns });
    }
    
    Ok(())
}

/// Validate that a decimal string doesn't have too many decimal places
pub fn validate_decimal_precision(value_str: &str) -> Result<(), ValidationError> {
    if let Some(decimal_pos) = value_str.find('.') {
        let decimal_places = value_str.len() - decimal_pos - 1;
        if decimal_places > FIXED_POINT_DECIMALS as usize {
            return Err(ValidationError::InvalidPrecision { 
                value: value_str.to_string(), 
                max_decimals: FIXED_POINT_DECIMALS 
            });
        }
    }
    Ok(())
}

/// Comprehensive validation for trade data
pub fn validate_trade_data(
    symbol: &str,
    price_fp: i64,
    volume_fp: i64,
    timestamp_ns: u64,
    exchange: &str,
) -> Result<(), ValidationError> {
    validate_price_range(price_fp, symbol)?;
    validate_volume_range(volume_fp, symbol)?;
    validate_timestamp(timestamp_ns, exchange)?;
    Ok(())
}

/// Check for potential data corruption patterns
pub fn detect_corruption_patterns(
    symbol: &str,
    price_fp: i64,
    volume_fp: i64,
) -> Vec<String> {
    let mut warnings = Vec::new();
    
    let price = fixed_point_to_f64(price_fp);
    let volume = fixed_point_to_f64(volume_fp);
    
    // Check for prices that look like they have decimal place errors
    if price > 1_000_000.0 {
        // Could be a decimal place error (e.g., USDC with 18 decimals instead of 6)
        warnings.push(format!("Suspiciously high price: {} - possible decimal misconfiguration", price));
    }
    
    if price < 0.000001 && !symbol.to_uppercase().contains("SHIB") {
        // Very small price for non-meme coins
        warnings.push(format!("Suspiciously low price: {} - possible decimal error", price));
    }
    
    // Check for round numbers that might indicate test data
    if price == 1.0 || price == 100.0 || price == 1000.0 {
        warnings.push(format!("Round number price: {} - might be test data", price));
    }
    
    // Check volume patterns
    if volume == 0.0 {
        warnings.push("Zero volume trade".to_string());
    }
    
    if volume == price {
        warnings.push("Volume equals price - possible data confusion".to_string());
    }
    
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversion::parse_price_to_fixed_point;

    #[test]
    fn test_asset_classification() {
        assert_eq!(classify_asset_type("BTC-USD"), AssetType::Crypto);        // Base: BTC (crypto)
        assert_eq!(classify_asset_type("ETH-USDC"), AssetType::Crypto);       // Base: ETH (crypto)
        assert_eq!(classify_asset_type("USDC-USDT"), AssetType::Stablecoin);  // Both stablecoins
        assert_eq!(classify_asset_type("quickswap:USDC-USDT"), AssetType::Stablecoin); // Both stablecoins
        assert_eq!(classify_asset_type("EUR-USD"), AssetType::Forex);         // Base: EUR (fiat)
        assert_eq!(classify_asset_type("quickswap:ETH-USDC"), AssetType::Crypto); // Base: ETH
        assert_eq!(classify_asset_type("USDC-USD"), AssetType::Stablecoin);   // Base: USDC
    }

    #[test]
    fn test_price_validation() {
        // Valid crypto price
        let price_fp = parse_price_to_fixed_point("4605.23").unwrap();
        assert!(validate_price_range(price_fp, "ETH-USD").is_ok());
        
        // Valid stablecoin price
        let stable_price_fp = parse_price_to_fixed_point("0.9998").unwrap();
        assert!(validate_price_range(stable_price_fp, "USDC-USDT").is_ok());
        
        // Invalid stablecoin price (too far from $1)
        let bad_stable_fp = parse_price_to_fixed_point("5.0").unwrap();
        assert!(validate_price_range(bad_stable_fp, "USDC-USDT").is_err());
    }

    #[test]
    fn test_volume_validation() {
        // Valid volume
        let volume_fp = parse_price_to_fixed_point("1.5").unwrap();
        assert!(validate_volume_range(volume_fp, "ETH-USD").is_ok());
        
        // Zero volume (valid)
        let zero_volume_fp = parse_price_to_fixed_point("0.0").unwrap();
        assert!(validate_volume_range(zero_volume_fp, "ETH-USD").is_ok());
        
        // Extremely large volume (invalid)
        let huge_volume_fp = parse_price_to_fixed_point("10000000000.0").unwrap(); // $10B
        assert!(validate_volume_range(huge_volume_fp, "ETH-USD").is_err());
    }

    #[test]
    fn test_timestamp_validation() {
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        // Current timestamp should be valid
        assert!(validate_timestamp(now_ns, "coinbase").is_ok());
        
        // Timestamp 1 hour ago should be valid
        let hour_ago = now_ns - (3600 * 1_000_000_000);
        assert!(validate_timestamp(hour_ago, "coinbase").is_ok());
        
        // Timestamp 2 days ago should be invalid
        let two_days_ago = now_ns - (2 * 24 * 3600 * 1_000_000_000);
        assert!(validate_timestamp(two_days_ago, "coinbase").is_err());
        
        // Future timestamp (within 1 hour) should be valid
        let future = now_ns + (1800 * 1_000_000_000); // 30 minutes
        assert!(validate_timestamp(future, "coinbase").is_ok());
        
        // Far future timestamp should be invalid
        let far_future = now_ns + (2 * 3600 * 1_000_000_000); // 2 hours
        assert!(validate_timestamp(far_future, "coinbase").is_err());
    }

    #[test]
    fn test_decimal_precision_validation() {
        // Valid precision (8 decimals or less)
        assert!(validate_decimal_precision("4605.23").is_ok());
        assert!(validate_decimal_precision("4605.12345678").is_ok());
        
        // Invalid precision (more than 8 decimals)
        assert!(validate_decimal_precision("4605.123456789").is_err());
        
        // Integer values should be valid
        assert!(validate_decimal_precision("4605").is_ok());
    }

    #[test]
    fn test_corruption_detection() {
        let price_fp = parse_price_to_fixed_point("10000000.0").unwrap(); // Very high price to trigger warning
        let volume_fp = parse_price_to_fixed_point("1.5").unwrap();
        
        let warnings = detect_corruption_patterns("ETH-USD", price_fp, volume_fp);
        assert!(!warnings.is_empty());
        assert!(warnings[0].contains("Suspiciously high price"));
    }

    #[test]
    fn test_comprehensive_validation() {
        let price_fp = parse_price_to_fixed_point("4605.23").unwrap();
        let volume_fp = parse_price_to_fixed_point("1.5").unwrap();
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        // Should pass all validations
        assert!(validate_trade_data("ETH-USD", price_fp, volume_fp, now_ns, "coinbase").is_ok());
    }
}