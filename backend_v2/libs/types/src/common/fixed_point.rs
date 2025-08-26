//! Fixed-point arithmetic types for precise financial calculations
//!
//! This module provides type-safe fixed-point arithmetic to prevent precision loss
//! in financial calculations. All types use integer storage with compile-time
//! decimal scaling to ensure exact representation and arithmetic.
//!
//! ## Design Principles
//!
//! - **No Precision Loss**: All values stored as scaled integers
//! - **Overflow Protection**: Checked arithmetic with clear error handling
//! - **Type Safety**: Distinct types prevent mixing incompatible scales
//! - **Performance**: Direct integer operations after validation
//! - **Transparency**: Clear conversion boundaries between floating-point and fixed-point

use crate::common::errors::FixedPointError;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Sub};

/// Fixed-point USD value with 8 decimal places precision
///
/// Represents USD amounts as scaled integers to avoid floating-point precision loss.
/// Scale factor: 100,000,000 (10^8)
///
/// Examples:
/// - $1.00 = UsdFixedPoint8(100_000_000)
/// - $0.01 = UsdFixedPoint8(1_000_000)
/// - $1000.12345678 = UsdFixedPoint8(100_012_345_678)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UsdFixedPoint8(pub i64);

impl UsdFixedPoint8 {
    /// Scale factor for 8 decimal places
    pub const SCALE: i64 = 100_000_000;

    /// Maximum representable value (prevents overflow)
    pub const MAX: Self = Self(i64::MAX);

    /// Minimum representable value
    pub const MIN: Self = Self(i64::MIN);

    /// Zero dollars
    pub const ZERO: Self = Self(0);

    /// One cent ($0.01)
    pub const ONE_CENT: Self = Self(1_000_000);

    /// One dollar ($1.00)
    pub const ONE_DOLLAR: Self = Self(100_000_000);

    /// Create from a decimal string with exact parsing
    ///
    /// This is the PRIMARY method for creating UsdFixedPoint8 from external data.
    /// Use this for parsing JSON, configuration files, user input, etc.
    ///
    /// # Examples
    /// ```
    /// use alphapulse_types::UsdFixedPoint8;
    ///
    /// let price = UsdFixedPoint8::from_decimal_str("123.456789").unwrap();
    /// assert_eq!(price.to_f64(), 123.456789);
    /// ```
    pub fn from_decimal_str(s: &str) -> Result<Self, FixedPointError> {
        use std::str::FromStr;

        let decimal = Decimal::from_str(s).map_err(|_| FixedPointError::InvalidDecimal {
            input: s.to_string(),
        })?;

        // Scale to 8 decimal places
        let scaled = decimal * Decimal::from(Self::SCALE);

        // Convert to i64 with bounds checking
        if let Some(value) = scaled.to_i64() {
            Ok(Self(value))
        } else {
            let float_val = decimal.to_f64().unwrap_or(f64::NAN);
            if float_val > 0.0 {
                Err(FixedPointError::Overflow { value: float_val })
            } else {
                Err(FixedPointError::Underflow { value: float_val })
            }
        }
    }

    /// CONVENIENCE method: Create from f64 with safety checks
    ///
    /// Use this for AMM math boundary conversions where floating-point
    /// calculations have already been performed. This method validates
    /// that the f64 can be safely converted to fixed-point.
    ///
    /// # Safety Notes
    /// - Validates finite values only (rejects NaN, infinity)
    /// - Checks for overflow/underflow
    /// - Rounds to nearest representable value
    ///
    /// # Examples
    /// ```
    /// use alphapulse_types::UsdFixedPoint8;
    ///
    /// let price = UsdFixedPoint8::try_from_f64(42.123456).unwrap();
    /// ```
    pub fn try_from_f64(value: f64) -> Result<Self, FixedPointError> {
        if !value.is_finite() {
            return Err(FixedPointError::NotFinite { value });
        }

        let scaled = value * Self::SCALE as f64;

        // Check for overflow/underflow
        if scaled > i64::MAX as f64 {
            return Err(FixedPointError::Overflow { value });
        }
        if scaled < i64::MIN as f64 {
            return Err(FixedPointError::Underflow { value });
        }

        Ok(Self(scaled.round() as i64))
    }

    /// Convert to f64 for display or interfacing with floating-point systems
    ///
    /// # Warning
    /// Only use for display, logging, or interfacing with external systems
    /// that require floating-point. Never use for financial calculations.
    pub fn to_f64(self) -> f64 {
        self.0 as f64 / Self::SCALE as f64
    }

    /// Get the raw scaled integer value
    pub fn raw_value(self) -> i64 {
        self.0
    }

    /// Create from raw scaled integer (advanced usage)
    pub fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    /// Create from cents value (compile-time constant for performance)
    #[inline]
    pub const fn from_cents(cents: i64) -> Self {
        Self(cents * 1_000_000) // Compile-time constant multiplication
    }

    /// Create from dollars (compile-time constant for performance)
    #[inline]
    pub const fn from_dollars(dollars: i64) -> Self {
        Self(dollars * Self::SCALE) // Compile-time constant
    }

    // CHECKED ARITHMETIC - For critical calculations where overflow must be handled

    /// Checked addition - returns None on overflow
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    /// Checked subtraction - returns None on underflow
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    /// Checked multiplication by integer quantity
    pub fn checked_mul_quantity(self, qty: i64) -> Option<Self> {
        self.0.checked_mul(qty).map(Self)
    }

    /// Checked division by integer quantity
    pub fn checked_div_quantity(self, qty: i64) -> Option<Self> {
        if qty == 0 {
            return None;
        }
        self.0.checked_div(qty).map(Self)
    }

    // SATURATING ARITHMETIC - For analytics/display where overflow should be clamped

    /// Saturating addition - clamps to max on overflow
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    /// Saturating subtraction - clamps to min on underflow
    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    /// Absolute value
    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }
}

/// Display implementation for convenient logging
impl fmt::Display for UsdFixedPoint8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "${:.8}", self.to_f64())
    }
}

/// Panicking arithmetic via traits - "should never fail" operations
/// Use for fee calculations with constants and other scenarios where
/// overflow is mathematically impossible
impl Add for UsdFixedPoint8 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0) // Will panic on overflow - use for "safe" operations only
    }
}

impl Sub for UsdFixedPoint8 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0) // Will panic on underflow - use for "safe" operations only
    }
}

/// Fixed-point percentage value with 4 decimal places precision
///
/// Represents percentages as scaled integers to avoid floating-point precision loss.
/// Scale factor: 10,000 (10^4)
///
/// Examples:
/// - 12.34% = PercentageFixedPoint4(123400)
/// - 0.01% = PercentageFixedPoint4(100)
/// - 100.00% = PercentageFixedPoint4(1_000_000)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PercentageFixedPoint4(pub i32);

impl PercentageFixedPoint4 {
    /// Scale factor for 4 decimal places
    pub const SCALE: i32 = 10_000;

    /// Maximum representable percentage
    pub const MAX: Self = Self(i32::MAX);

    /// Minimum representable percentage
    pub const MIN: Self = Self(i32::MIN);

    /// Zero percent
    pub const ZERO: Self = Self(0);

    /// One percent (1.00%)
    pub const ONE_PERCENT: Self = Self(10_000);

    /// One basis point (0.01%)
    pub const ONE_BASIS_POINT: Self = Self(100);

    /// Create from a decimal string with exact parsing
    pub fn from_decimal_str(s: &str) -> Result<Self, FixedPointError> {
        use std::str::FromStr;

        let decimal = Decimal::from_str(s).map_err(|_| FixedPointError::InvalidDecimal {
            input: s.to_string(),
        })?;

        // Scale to 4 decimal places
        let scaled = decimal * Decimal::from(Self::SCALE);

        // Convert to i32 with bounds checking
        if let Some(value) = scaled.to_i32() {
            Ok(Self(value))
        } else {
            let float_val = decimal.to_f64().unwrap_or(f64::NAN);
            if float_val > 0.0 {
                Err(FixedPointError::Overflow { value: float_val })
            } else {
                Err(FixedPointError::Underflow { value: float_val })
            }
        }
    }

    /// CONVENIENCE method: Create from f64 with safety checks
    pub fn try_from_f64(value: f64) -> Result<Self, FixedPointError> {
        if !value.is_finite() {
            return Err(FixedPointError::NotFinite { value });
        }

        let scaled = value * Self::SCALE as f64;

        // Check for overflow/underflow
        if scaled > i32::MAX as f64 {
            return Err(FixedPointError::Overflow { value });
        }
        if scaled < i32::MIN as f64 {
            return Err(FixedPointError::Underflow { value });
        }

        Ok(Self(scaled.round() as i32))
    }

    /// Convert to f64 for display purposes only
    pub fn to_f64(self) -> f64 {
        self.0 as f64 / Self::SCALE as f64
    }

    /// Get the raw scaled integer value
    pub fn raw_value(self) -> i32 {
        self.0
    }

    /// Create from raw scaled integer (advanced usage)
    pub fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    // CHECKED ARITHMETIC

    /// Checked addition - returns None on overflow
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    /// Checked subtraction - returns None on underflow
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    // SATURATING ARITHMETIC

    /// Saturating addition - clamps to max on overflow
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    /// Saturating subtraction - clamps to min on underflow
    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    /// Absolute value
    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }
}

/// Display implementation for convenient logging
impl fmt::Display for PercentageFixedPoint4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4}%", self.to_f64())
    }
}

/// Panicking arithmetic via traits
impl Add for PercentageFixedPoint4 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0) // Will panic on overflow
    }
}

impl Sub for PercentageFixedPoint4 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0) // Will panic on underflow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usd_fixed_point_creation() {
        // Test decimal string parsing
        let price = UsdFixedPoint8::from_decimal_str("123.45678901").unwrap();
        assert_eq!(price.raw_value(), 12345678901);

        // Test f64 conversion
        let price2 = UsdFixedPoint8::try_from_f64(123.45678901).unwrap();
        // Note: f64 precision may cause slight differences
        assert!((price2.to_f64() - 123.45678901).abs() < 1e-7);
    }

    #[test]
    fn test_usd_fixed_point_constants() {
        assert_eq!(UsdFixedPoint8::ZERO.to_f64(), 0.0);
        assert_eq!(UsdFixedPoint8::ONE_CENT.to_f64(), 0.01);
        assert_eq!(UsdFixedPoint8::ONE_DOLLAR.to_f64(), 1.0);
    }

    #[test]
    fn test_usd_checked_arithmetic() {
        let a = UsdFixedPoint8::ONE_DOLLAR;
        let b = UsdFixedPoint8::ONE_CENT;

        let sum = a.checked_add(b).unwrap();
        assert_eq!(sum.to_f64(), 1.01);

        let diff = a.checked_sub(b).unwrap();
        assert_eq!(diff.to_f64(), 0.99);
    }

    #[test]
    fn test_percentage_fixed_point_creation() {
        let pct = PercentageFixedPoint4::from_decimal_str("12.3456").unwrap();
        assert_eq!(pct.raw_value(), 123456);

        let pct2 = PercentageFixedPoint4::try_from_f64(12.3456).unwrap();
        assert_eq!(pct2.to_f64(), 12.3456);
    }

    #[test]
    fn test_percentage_constants() {
        assert_eq!(PercentageFixedPoint4::ZERO.to_f64(), 0.0);
        assert_eq!(PercentageFixedPoint4::ONE_PERCENT.to_f64(), 1.0);
        assert_eq!(PercentageFixedPoint4::ONE_BASIS_POINT.to_f64(), 0.01);
    }

    #[test]
    fn test_error_handling() {
        // Test invalid decimal
        assert!(UsdFixedPoint8::from_decimal_str("not_a_number").is_err());

        // Test non-finite f64
        assert!(UsdFixedPoint8::try_from_f64(f64::NAN).is_err());
        assert!(UsdFixedPoint8::try_from_f64(f64::INFINITY).is_err());
    }

    #[test]
    fn test_display_formatting() {
        let usd = UsdFixedPoint8::from_decimal_str("123.456789").unwrap();
        let display = format!("{}", usd);
        assert!(display.starts_with("$123.45678"));

        let pct = PercentageFixedPoint4::from_decimal_str("12.34").unwrap();
        let display = format!("{}", pct);
        assert_eq!(display, "12.3400%");
    }
}
