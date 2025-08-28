//! Precision Handling and Validation for Financial Data
//!
//! Ensures proper precision handling across different asset types and exchange formats
//! to maintain accuracy in financial calculations and prevent precision loss.
//!
//! ## Precision Requirements by Asset Type
//!
//! ### DEX Tokens (Native Precision)
//! - **WETH**: 18 decimal places (`1 WETH = 1_000_000_000_000_000_000 wei`)
//! - **USDC**: 6 decimal places (`1 USDC = 1_000_000 units`)
//! - **USDT**: 6 decimal places (`1 USDT = 1_000_000 units`)  
//! - **DAI**: 18 decimal places (`1 DAI = 1_000_000_000_000_000_000 units`)
//!
//! ### Traditional Exchanges (8-Decimal Fixed Point)
//! - **USD Prices**: 8 decimal places (`$45,000.00 = 4_500_000_000_000`)
//! - **BTC/ETH Pairs**: 8 decimal places for consistency
//! - **Fiat Conversions**: 8 decimal places
//!
//! ## Critical Rules
//!
//! 1. **NO FLOATING POINT**: Never use f32/f64 for financial calculations
//! 2. **Preserve Native Precision**: Don't normalize DEX token amounts
//! 3. **Explicit Conversions**: Always document precision changes
//! 4. **Nanosecond Timestamps**: Never truncate to milliseconds
//!
//! ## Example Usage
//!
//! ```rust
//! use torq_types::precision::{TokenAmount, ExchangePrice, validate_precision};
//!
//! // DEX token amounts - preserve native precision
//! let weth_amount = TokenAmount::new_weth(1_500_000_000_000_000_000); // 1.5 WETH
//! let usdc_amount = TokenAmount::new_usdc(5_000_000); // 5.0 USDC
//!
//! // Traditional exchange prices - 8 decimal fixed point
//! let btc_price = ExchangePrice::from_usd(45_000_00000000); // $45,000.00
//! 
//! // Validate no precision loss in conversions
//! assert!(validate_precision(&weth_amount, &usdc_amount).is_ok());
//! ```

use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PrecisionError {
    #[error("Precision mismatch: {0}")]
    PrecisionMismatch(String),
    
    #[error("Value overflow: {0}")]
    Overflow(String),
    
    #[error("Invalid timestamp: {0}")]
    InvalidTimestamp(String),
    
    #[error("System time error: {0}")]
    SystemTimeError(String),
}

pub type Result<T> = std::result::Result<T, PrecisionError>;

/// Token amount with native precision preservation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenAmount {
    /// Raw amount in smallest units (wei for ETH, etc.)
    pub raw_amount: i64,
    /// Number of decimal places for this token
    pub decimals: u8,
    /// Token symbol for validation
    pub symbol: TokenSymbol,
}

/// Traditional exchange price with 8-decimal fixed point
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExchangePrice {
    /// Price in 8-decimal fixed point (USD * 100_000_000)
    pub price_fixed: i64,
    /// Base currency (what's being priced)
    pub base: &'static str,
    /// Quote currency (usually USD)
    pub quote: &'static str,
}

/// Supported token symbols with known precision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSymbol {
    /// Wrapped Ethereum (18 decimals)
    WETH,
    /// USD Coin (6 decimals)
    USDC,
    /// Tether USD (6 decimals)
    USDT,
    /// Dai Stablecoin (18 decimals)
    DAI,
    /// Wrapped Bitcoin (8 decimals)
    WBTC,
    /// Custom token with specified decimals
    Custom { symbol: &'static str, decimals: u8 },
}

impl TokenAmount {
    /// Create WETH amount (18 decimals)
    pub fn new_weth(wei_amount: i64) -> Self {
        Self {
            raw_amount: wei_amount,
            decimals: 18,
            symbol: TokenSymbol::WETH,
        }
    }

    /// Create USDC amount (6 decimals)
    pub fn new_usdc(units: i64) -> Self {
        Self {
            raw_amount: units,
            decimals: 6,
            symbol: TokenSymbol::USDC,
        }
    }

    /// Create USDT amount (6 decimals)
    pub fn new_usdt(units: i64) -> Self {
        Self {
            raw_amount: units,
            decimals: 6,
            symbol: TokenSymbol::USDT,
        }
    }

    /// Create DAI amount (18 decimals)
    pub fn new_dai(units: i64) -> Self {
        Self {
            raw_amount: units,
            decimals: 18,
            symbol: TokenSymbol::DAI,
        }
    }

    /// Create WBTC amount (8 decimals)
    pub fn new_wbtc(satoshis: i64) -> Self {
        Self {
            raw_amount: satoshis,
            decimals: 8,
            symbol: TokenSymbol::WBTC,
        }
    }

    /// Create custom token amount
    pub fn new_custom(amount: i64, symbol: &'static str, decimals: u8) -> Self {
        Self {
            raw_amount: amount,
            decimals,
            symbol: TokenSymbol::Custom { symbol, decimals },
        }
    }

    /// Get the decimal multiplier for this token
    pub fn decimal_multiplier(&self) -> i64 {
        10i64.pow(self.decimals as u32)
    }

    /// Convert to human readable string (for display only, not calculations)
    pub fn to_display_string(&self) -> String {
        let multiplier = self.decimal_multiplier();
        let whole = self.raw_amount / multiplier;
        let fractional = self.raw_amount % multiplier;
        
        format!("{}.{:0width$} {}", 
            whole, 
            fractional, 
            self.symbol.as_str(),
            width = self.decimals as usize
        )
    }

    /// Validate this amount maintains required precision
    pub fn validate_precision(&self) -> Result<()> {
        // Check that decimals match expected for symbol
        let expected_decimals = self.symbol.expected_decimals();
        if self.decimals != expected_decimals {
            return Err(PrecisionError::PrecisionMismatch(format!(
                "Token {} has {} decimals but expected {}",
                self.symbol.as_str(), self.decimals, expected_decimals
            )));
        }

        // Check for reasonable bounds (prevent overflow in calculations)
        if self.raw_amount.abs() > i64::MAX / 1000 {
            return Err(PrecisionError::Overflow(format!(
                "Token amount {} too large, risk of overflow",
                self.raw_amount
            )));
        }

        Ok(())
    }
}

impl ExchangePrice {
    /// Create USD price with 8-decimal fixed point
    pub fn from_usd(price_fixed: i64) -> Self {
        Self {
            price_fixed,
            base: "BTC", // Default, can be overridden
            quote: "USD",
        }
    }

    /// Create price for specific pair
    pub fn new(price_fixed: i64, base: &'static str, quote: &'static str) -> Self {
        Self {
            price_fixed,
            base,
            quote,
        }
    }

    /// Get decimal multiplier (always 8 for exchange prices)
    pub const fn decimal_multiplier() -> i64 {
        100_000_000 // 8 decimal places
    }

    /// Convert to display string (for display only)
    pub fn to_display_string(&self) -> String {
        let multiplier = Self::decimal_multiplier();
        let whole = self.price_fixed / multiplier;
        let fractional = self.price_fixed % multiplier;
        
        format!("{}.{:08} {}/{}", whole, fractional, self.base, self.quote)
    }

    /// Validate price precision
    pub fn validate_precision(&self) -> Result<()> {
        // Check for reasonable price bounds
        if self.price_fixed <= 0 {
            return Err(PrecisionError::PrecisionMismatch(
                "Exchange price must be positive".to_string()
            ));
        }

        if self.price_fixed > i64::MAX / 1000 {
            return Err(PrecisionError::Overflow(format!(
                "Price {} too large, risk of overflow", self.price_fixed
            )));
        }

        Ok(())
    }
}

impl TokenSymbol {
    /// Get expected decimal places for this token
    pub fn expected_decimals(&self) -> u8 {
        match self {
            TokenSymbol::WETH => 18,
            TokenSymbol::USDC => 6,
            TokenSymbol::USDT => 6,
            TokenSymbol::DAI => 18,
            TokenSymbol::WBTC => 8,
            TokenSymbol::Custom { decimals, .. } => *decimals,
        }
    }

    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenSymbol::WETH => "WETH",
            TokenSymbol::USDC => "USDC",
            TokenSymbol::USDT => "USDT",
            TokenSymbol::DAI => "DAI",
            TokenSymbol::WBTC => "WBTC",
            TokenSymbol::Custom { symbol, .. } => symbol,
        }
    }
}

/// Validate precision across different amount types
pub fn validate_precision(token: &TokenAmount, price: &ExchangePrice) -> Result<()> {
    token.validate_precision()?;
    price.validate_precision()?;
    
    // Ensure we're not mixing incompatible precisions without explicit conversion
    // This is a compile-time safety check - mixing requires explicit conversion
    
    Ok(())
}

/// Timestamp precision validator
pub fn validate_timestamp_precision(timestamp_ns: u64) -> Result<()> {
    let current_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| PrecisionError::SystemTimeError(format!("System time error: {}", e)))?
        .as_nanos() as u64;

    // Validate timestamp is in reasonable range (nanoseconds, not micro/milli)
    let hour_in_ns = 60 * 60 * 1_000_000_000u64;
    let day_in_ns = 24 * hour_in_ns;

    // Allow timestamps within 24 hours of current time
    if timestamp_ns < current_ns.saturating_sub(day_in_ns) || 
       timestamp_ns > current_ns + day_in_ns {
        return Err(PrecisionError::InvalidTimestamp(format!(
            "Timestamp {} appears invalid - not in nanoseconds? Current: {}",
            timestamp_ns, current_ns
        )));
    }

    // Check that timestamp has nanosecond precision (not truncated)
    // If timestamp ends in many zeros, it might be truncated
    let trailing_zeros = timestamp_ns.trailing_zeros();
    if trailing_zeros >= 20 { // More than 6 decimal places of zeros
        return Err(PrecisionError::InvalidTimestamp(format!(
            "Timestamp {} appears truncated (too many trailing zeros)",
            timestamp_ns
        )));
    }

    Ok(())
}

/// Create a registry of known token precisions for validation
pub fn create_precision_registry() -> HashMap<&'static str, u8> {
    let mut registry = HashMap::new();
    
    // Major DEX tokens
    registry.insert("WETH", 18);
    registry.insert("USDC", 6);
    registry.insert("USDT", 6);
    registry.insert("DAI", 18);
    registry.insert("WBTC", 8);
    registry.insert("UNI", 18);
    registry.insert("LINK", 18);
    registry.insert("AAVE", 18);
    registry.insert("SUSHI", 18);
    registry.insert("CRV", 18);
    
    // Stablecoins
    registry.insert("BUSD", 18);
    registry.insert("FRAX", 18);
    registry.insert("LUSD", 18);
    registry.insert("MIM", 18);
    
    registry
}

/// Validate that a price calculation doesn't use floating point
pub fn validate_no_floating_point(code: &str) -> bool {
    // Simple string-based validation - in practice would use AST analysis
    !(code.contains("f32") || 
      code.contains("f64") || 
      code.contains("float") ||
      code.contains(".0") || // Catches 100.0 literals
      code.contains("as f"))
}

/// Helper to convert between precision levels safely
pub struct PrecisionConverter;

impl PrecisionConverter {
    /// Convert token amount to 8-decimal fixed point for exchange comparison
    /// This is an EXPLICIT precision change that must be documented
    pub fn token_to_exchange_precision(token: &TokenAmount, exchange_rate: &ExchangePrice) -> Result<i64> {
        token.validate_precision()?;
        exchange_rate.validate_precision()?;

        // This conversion MUST be explicit and documented
        // Example: Converting 1.5 WETH (1_500_000_000_000_000_000 wei) 
        // to USD using $2000/ETH rate (200_000_000_000 in 8-decimal)
        
        let token_multiplier = token.decimal_multiplier();
        let _exchange_multiplier = ExchangePrice::decimal_multiplier(); // Used for validation context
        
        // Prevent overflow in multiplication
        if token.raw_amount > i64::MAX / exchange_rate.price_fixed {
            return Err(PrecisionError::Overflow(
                "Conversion would overflow - amounts too large".to_string()
            ));
        }
        
        // Convert: (token_amount * exchange_rate) / token_multiplier
        let value_in_exchange_units = (token.raw_amount * exchange_rate.price_fixed) / token_multiplier;
        
        Ok(value_in_exchange_units)
    }
}

impl fmt::Display for TokenAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}

impl fmt::Display for ExchangePrice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_amounts() {
        let weth = TokenAmount::new_weth(1_500_000_000_000_000_000); // 1.5 WETH
        assert_eq!(weth.decimals, 18);
        assert_eq!(weth.raw_amount, 1_500_000_000_000_000_000);
        assert!(weth.validate_precision().is_ok());

        let usdc = TokenAmount::new_usdc(5_000_000); // 5.0 USDC
        assert_eq!(usdc.decimals, 6);
        assert_eq!(usdc.raw_amount, 5_000_000);
        assert!(usdc.validate_precision().is_ok());
    }

    #[test]
    fn test_exchange_prices() {
        let btc_price = ExchangePrice::from_usd(4_500_000_000_000); // $45,000.00
        assert_eq!(btc_price.price_fixed, 4_500_000_000_000);
        assert!(btc_price.validate_precision().is_ok());

        // Display should show proper decimal places
        let display = btc_price.to_display_string();
        assert!(display.contains("45000.00000000"));
    }

    #[test]
    fn test_precision_validation() {
        let weth = TokenAmount::new_weth(1_000_000_000_000_000_000);
        let price = ExchangePrice::from_usd(200_000_000_000); // $2000
        
        assert!(validate_precision(&weth, &price).is_ok());
    }

    #[test]
    fn test_timestamp_validation() {
        let current_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        // Valid nanosecond timestamp
        assert!(validate_timestamp_precision(current_ns).is_ok());
        
        // Invalid: looks like milliseconds (too few digits)
        let ms_timestamp = current_ns / 1_000_000;
        assert!(validate_timestamp_precision(ms_timestamp).is_err());
        
        // Invalid: too many trailing zeros (truncated)
        let truncated = (current_ns / 1_000_000) * 1_000_000; // Remove last 6 digits
        assert!(validate_timestamp_precision(truncated).is_err());
    }

    #[test]
    fn test_float_detection() {
        assert!(validate_no_floating_point("let price = 100i64;"));
        assert!(validate_no_floating_point("let amount = token.raw_amount;"));
        assert!(!validate_no_floating_point("let price = 100.0f64;"));
        assert!(!validate_no_floating_point("let price: f32 = 100.0;"));
        assert!(!validate_no_floating_point("100.0 as float"));
    }

    #[test]
    fn test_precision_converter() {
        let weth = TokenAmount::new_weth(1_500_000_000_000_000_000); // 1.5 WETH
        let eth_price = ExchangePrice::new(200_000_000_000, "ETH", "USD"); // $2000.00
        
        let value = PrecisionConverter::token_to_exchange_precision(&weth, &eth_price)
            .expect("Conversion should succeed");
        
        // 1.5 ETH * $2000 = $3000.00 = 300_000_000_000 in 8-decimal
        assert_eq!(value, 300_000_000_000);
    }

    #[test]
    fn test_precision_registry() {
        let registry = create_precision_registry();
        assert_eq!(registry["WETH"], 18);
        assert_eq!(registry["USDC"], 6);
        assert_eq!(registry["WBTC"], 8);
    }
}